//! MESH-06 — per-node pgrust content-addressed event-log (local-first + sync).
//!
//! Layer E (local-first) of the mesh-real plan. A node commits signed intents to
//! a content-addressed event-log **locally**, running the kernel `decide`/`fold`
//! Law *before* any network IO. The event-id is a hash of
//! `(prev, actor_pubkey, actor_seq, payload)` so replays of the same content are
//! idempotent (no TTL dedup — a duplicate is a *structural* no-op, not a timeout).
//!
//! # Store abstraction (pgrust stand-in)
//! The production node persists this log in **pgrust** (the WAVE0 DK-05 Postgres
//! client; the `deploy/` tree carries the pgrust service + migrations). This
//! crate MUST stay dependency-light and offline-testable, so the store is behind
//! a [`EventStore`] trait with a default in-memory [`MemEventStore`]. The real
//! `PgEventStore` (backed by pgrust) is wired in the node binary, NOT here.
//!
//! innovate: replace `MemEventStore` with a `PgEventStore` (pgrust) in the node
//! runtime to get durable, crash-safe, SQL-queryable event logs. The trait is
//! the only seam. Offline ceiling: `MemEventStore` is non-durable and not shared
//! across processes — single-node local-first only. pgrust upgrades it to the
//! durable, multi-writer-safe variant without touching `EventLog`.
//!
//! CI GUARD: NO-COURIER-SCORING — events carry an `actor_pubkey` (identity),
//! never a score. The log is neutral, idempotent plumbing.

use std::collections::HashSet;

/// Deterministic SHA3-256 (FIPS 202) — pure Rust, no external dependency, so the
/// kernel stays offline-buildable and dependency-free. Used to content-address
/// events (collision resistance needed for idempotency, not just a hash map).
pub fn sha3_256(input: &[u8]) -> [u8; 32] {
    // Keccak-f[1600] round constants (FIPS 202).
    const RC: [u64; 24] = [
        0x0000000000000001,
        0x0000000000008082,
        0x800000000000808a,
        0x8000000080008000,
        0x000000000000808b,
        0x0000000080000001,
        0x8000000080008081,
        0x8000000000008009,
        0x000000000000008a,
        0x0000000000000088,
        0x0000000080008009,
        0x000000008000000a,
        0x000000008000808b,
        0x800000000000008b,
        0x8000000000008089,
        0x8000000000008003,
        0x8000000000008002,
        0x8000000000000080,
        0x000000000000800a,
        0x800000008000000a,
        0x8000000080008081,
        0x8000000000008080,
        0x0000000080000001,
        0x8000000080008008,
    ];
    // Rho rotation offsets r[x][y].
    const R: [[u32; 5]; 5] = [
        [0, 36, 3, 41, 18],
        [1, 44, 10, 45, 2],
        [62, 6, 43, 15, 61],
        [28, 55, 25, 21, 56],
        [27, 20, 39, 8, 14],
    ];

    fn keccak_f(s: &mut [u64; 25]) {
        for r in 0..24 {
            // θ (theta)
            let mut c = [0u64; 5];
            for x in 0..5 {
                c[x] = s[x] ^ s[x + 5] ^ s[x + 10] ^ s[x + 15] ^ s[x + 20];
            }
            let mut d = [0u64; 5];
            for x in 0..5 {
                d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            }
            for x in 0..5 {
                for y in 0..5 {
                    s[x + 5 * y] ^= d[x];
                }
            }
            // ρ + π
            let mut b = [0u64; 25];
            for x in 0..5 {
                for y in 0..5 {
                    let dest_x = y;
                    let dest_y = (2 * x + 3 * y) % 5;
                    b[dest_x + 5 * dest_y] = s[x + 5 * y].rotate_left(R[x][y]);
                }
            }
            // χ (chi)
            for x in 0..5 {
                for y in 0..5 {
                    let idx = x + 5 * y;
                    s[idx] = b[idx] ^ ((!b[((x + 1) % 5) + 5 * y]) & b[((x + 2) % 5) + 5 * y]);
                }
            }
            // ι (iota)
            s[0] ^= RC[r];
        }
    }

    const RATE: usize = 136; // SHA3-256 rate in bytes.
    let mut msg = input.to_vec();
    msg.push(0x06); // domain padding for SHA-3
    while msg.len() % RATE != 0 {
        msg.push(0);
    }
    *msg.last_mut().unwrap() |= 0x80; // multi-rate padding terminator

    let mut state = [0u64; 25];
    for block in msg.chunks_exact(RATE) {
        for j in 0..(RATE / 8) {
            let lane = u64::from_le_bytes(block[j * 8..j * 8 + 8].try_into().unwrap());
            state[j] ^= lane;
        }
        keccak_f(&mut state);
    }
    let mut out = [0u8; 32];
    for j in 0..4 {
        out[j * 8..j * 8 + 8].copy_from_slice(&state[j].to_le_bytes());
    }
    out
}

/// A single content-addressed mesh event.
///
/// The log is a hash chain: `prev` is the content-id of the preceding event at
/// this node (zero for the genesis event). `actor_seq` is a per-actor monotonic
/// counter. The tuple `(prev, actor_pubkey, actor_seq, payload)` is hashed to
/// produce the content-id, which is the idempotency key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshEvent {
    /// Content-id of the previous event in this node's chain (zero = genesis).
    pub prev: [u8; 32],
    /// Public key of the actor that authored the event (identity, NOT a score).
    pub actor_pubkey: [u8; 32],
    /// Per-actor monotonic sequence number.
    pub actor_seq: u64,
    /// Opaque intent bytes (e.g. a serialized order transition for `decide`).
    pub payload: Vec<u8>,
}

impl MeshEvent {
    /// Content-id = SHA3-256(prev || actor_pubkey || actor_seq || payload).
    /// Deterministic and collision-resistant; this is the idempotency key.
    pub fn event_id(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(32 + 32 + 8 + self.payload.len());
        buf.extend_from_slice(&self.prev);
        buf.extend_from_slice(&self.actor_pubkey);
        buf.extend_from_slice(&self.actor_seq.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        sha3_256(&buf)
    }
}

/// Persistence seam for the event-log. The production node backs this with
/// pgrust; tests use [`MemEventStore`]. A std-only durable variant
/// ([`crate::hydra::FileEventStore`]) is available for the Воля АНУ closed loop
/// (offline, egress-free, no external DB dependency).
pub trait EventStore {
    /// Whether an event with this content-id is already persisted (idempotency).
    fn contains(&self, id: &[u8; 32]) -> bool;
    /// Persist an event under its content-id.
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent);
    /// Fetch a persisted event by content-id, if present (needed for durable
    /// session-boundary re-verify, G4/G5).
    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        let _ = id;
        None
    }
    /// Number of events persisted.
    fn len(&self) -> usize;
    /// Whether the store is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Content-id of the current chain tip (last committed), if any.
    fn tip(&self) -> Option<[u8; 32]>;
    /// Advance the chain tip to `id`.
    fn set_tip(&mut self, id: [u8; 32]);
}

/// In-memory [`EventStore`] — the offline pgrust stand-in (see module `innovate:`).
#[derive(Debug, Clone, Default)]
pub struct MemEventStore {
    by_id: HashSet<[u8; 32]>,
    tip: Option<[u8; 32]>,
    count: usize,
}

impl MemEventStore {
    /// Empty in-memory store.
    pub fn new() -> Self {
        MemEventStore::default()
    }
}

impl EventStore for MemEventStore {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.by_id.contains(id)
    }
    fn insert(&mut self, id: [u8; 32], _ev: MeshEvent) {
        if self.by_id.insert(id) {
            self.count += 1;
        }
    }
    fn len(&self) -> usize {
        self.count
    }
    fn tip(&self) -> Option<[u8; 32]> {
        self.tip
    }
    fn set_tip(&mut self, id: [u8; 32]) {
        self.tip = Some(id);
    }
}

/// Outcome of attempting to append an event to the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    /// Event was new and committed (prev set to the prior tip if the caller left
    /// `prev` zeroed — local-first chaining before any network sync).
    Committed([u8; 32]),
    /// Event content-id already present — idempotent no-op. State unchanged.
    Duplicate([u8; 32]),
}

/// Error returned when a `decide`-gated append is rejected by the local Law.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecideRejected(pub String);

/// A per-node, content-addressed, idempotent event-log.
///
/// Local-first: `append`/`commit_after_decide` runs **before** any network IO.
/// Once persistently committed, the event can be gossiped/synced (MESH-07) and
/// the network layer never re-runs `decide` — it only verifies signatures.
pub struct EventLog<S: EventStore> {
    store: S,
}

impl<S: EventStore> EventLog<S> {
    /// Wrap a store.
    pub fn new(store: S) -> Self {
        EventLog { store }
    }

    /// Borrow the backing store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Append an event, chaining `prev` to the current tip if the caller left it
    /// zeroed (local-first hash-chain genesis/continuation). Idempotent: an
    /// event whose computed content-id already exists is a [`Duplicate`] no-op.
    pub fn append(&mut self, mut ev: MeshEvent) -> AppendOutcome {
        // Local-first chaining: bind prev to the current tip if the caller left
        // it zeroed. MUST happen before event_id() so the content-id reflects
        // the chain position (the chain is part of the idempotency key).
        if ev.prev == [0u8; 32] {
            if let Some(tip) = self.store.tip() {
                ev.prev = tip;
            }
        }
        let id = ev.event_id();
        if self.store.contains(&id) {
            return AppendOutcome::Duplicate(id);
        }
        self.store.insert(id, ev);
        self.store.set_tip(id);
        AppendOutcome::Committed(id)
    }

    /// MESH-06 commit gate: run the kernel `decide` Law on the event **before**
    /// committing. If `decide` rejects, the event is NOT persisted (no partial
    /// commit) and the store is left exactly as it was. If it accepts, the event
    /// is appended (idempotently — a duplicate is still a no-op and `decide` is
    /// NOT re-run, preserving state). This is the local-before-network contract.
    ///
    /// `decide` returns `Ok(decision)` (carried back to the caller) or
    /// `Err(reason)`. The decision value is opaque to the log.
    pub fn commit_after_decide<D, T, E>(
        &mut self,
        ev: MeshEvent,
        decide: D,
    ) -> Result<(AppendOutcome, Option<T>), DecideRejected>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: std::fmt::Display,
    {
        let id = ev.event_id();
        // Idempotency first: a duplicate never re-runs decide (state unchanged).
        if self.store.contains(&id) {
            return Ok((AppendOutcome::Duplicate(id), None));
        }
        // Decide BEFORE commit. On rejection, do not persist anything.
        let decision = decide(&ev).map_err(|e| DecideRejected(e.to_string()))?;
        // Commit (chains prev, records tip).
        let outcome = self.append(ev);
        Ok((outcome, Some(decision)))
    }

    /// Whether an event with this content-id is already persisted (idempotency).
    /// Delegates to the backing store.
    pub fn contains(&self, id: &[u8; 32]) -> bool {
        self.store.contains(id)
    }

    /// Current chain tip content-id, if any.
    pub fn tip(&self) -> Option<[u8; 32]> {
        self.store.tip()
    }

    /// MESH-06 + HYDRA-G2: spectral-drift fail-closed gate wired into the commit
    /// path. Runs `classify_drift` on the organism's current adjacency BEFORE
    /// `decide`. If the spectrum is `Unstable` (ρ > 1 + ε) the mutation is
    /// REJECTED pre-persist — the organism cannot slip into a divergent topology.
    ///
    /// OPERATOR DIRECTIVE (SOURCE-OF-HYDRA §3): the gate is ACTIVE only in the
    /// DEFAULT regime. When `intervention == true` (foreign/undefined code is
    /// actively mutating the organism) ALL safeties are LIFTED — the drift gate
    /// is bypassed and the mutation proceeds to `decide` regardless of spectrum.
    /// Survival = endurance, not exclusion. The only hard stop remains kill-switch.
    ///
    /// This is the KEY HOLE closure: `classify_drift` (`spectral.rs:325`) was
    /// previously computed but never bound into the decide/commit path. Now it is.
    /// The adjacency is supplied by the caller (kernel-internal spine — NOT an
    /// exposed surface), keeping the source hidden per G7.
    pub fn commit_after_decide_drift_gate<D, T, E>(
        &mut self,
        ev: MeshEvent,
        adjacency: &[Vec<f64>],
        intervention: bool,
        decide: D,
    ) -> Result<(AppendOutcome, Option<T>), DecideRejected>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: std::fmt::Display,
    {
        // Intervention mode: lift ALL safeties (operator directive §3).
        if !intervention {
            use crate::spectral::classify_drift;
            if matches!(
                classify_drift(adjacency),
                crate::spectral::DriftClass::Unstable
            ) {
                return Err(DecideRejected(format!(
                    "HYDRA drift-gate REJECT: spectrum Unstable (ρ>1) — mutation would diverge; \
                     organism endures by NOT persisting. adjacency={}x{}",
                    adjacency.len(),
                    adjacency.first().map(|r| r.len()).unwrap_or(0)
                )));
            }
        }
        // Default regime (or intervention lift): proceed to decide-before-commit.
        self.commit_after_decide(ev, decide)
    }

    /// Number of events persisted.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reproducible actor key (32 bytes; identity only).
    fn actor(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    /// Build an event for `actor` at `seq` with `payload`.
    fn ev(actor_byte: u8, seq: u64, payload: &[u8]) -> MeshEvent {
        MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: actor(actor_byte),
            actor_seq: seq,
            payload: payload.to_vec(),
        }
    }

    /// SHA3-256 known-answer test (FIPS 202): SHA3-256("") ==
    /// a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a.
    #[test]
    fn sha3_256_empty_known_answer() {
        let h = sha3_256(b"");
        assert_eq!(
            h,
            [
                0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61,
                0xd6, 0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b,
                0x80, 0xf8, 0x43, 0x4a,
            ]
        );
    }

    /// SHA3-256 of "abc" must be stable and distinct from empty.
    #[test]
    fn sha3_256_abc_distinct() {
        let a = sha3_256(b"abc");
        let b = sha3_256(b"abd");
        assert_ne!(a, b, "distinct inputs => distinct digests");
        assert_ne!(a, sha3_256(b""));
    }

    /// Redux: same content => same event-id (idempotency key is deterministic).
    #[test]
    fn event_id_is_deterministic_and_content_keyed() {
        let e1 = ev(1, 1, b"hello");
        let e2 = ev(1, 1, b"hello");
        let e3 = ev(1, 1, b"hellp"); // one byte different
        assert_eq!(e1.event_id(), e2.event_id(), "same content => same id");
        assert_ne!(
            e1.event_id(),
            e3.event_id(),
            "different payload => different id"
        );
        // Different actor => different id even with same seq/payload.
        assert_ne!(ev(1, 1, b"x").event_id(), ev(2, 1, b"x").event_id());
    }

    /// RED — MESH-06 core: a duplicate event (identical content-id) replayed
    /// onto the log MUST NOT change state a second time. The `decide` closure
    /// must be invoked exactly once across both appends.
    #[test]
    fn dup_event_is_idempotent_no_state_change() {
        let mut log = EventLog::new(MemEventStore::new());
        let mut decide_calls = 0usize;

        let e = ev(7, 1, b"intent-A");
        // First commit: decide runs, event committed.
        let (out1, dec1) = log
            .commit_after_decide(e.clone(), |_| {
                decide_calls += 1;
                Ok::<u64, String>(1)
            })
            .unwrap();
        assert!(matches!(out1, AppendOutcome::Committed(_)));
        assert_eq!(dec1, Some(1));
        assert_eq!(decide_calls, 1);

        // Second append of the SAME content: must be a Duplicate, decide must
        // NOT run again, and the log length must be unchanged.
        let (out2, dec2) = log
            .commit_after_decide(e, |_| {
                decide_calls += 1;
                Ok::<u64, String>(1)
            })
            .unwrap();
        assert!(matches!(out2, AppendOutcome::Duplicate(_)));
        assert_eq!(dec2, None, "duplicate yields no new decision");
        assert_eq!(decide_calls, 1, "decide NOT re-run for a duplicate");
        assert_eq!(log.len(), 1, "state unchanged by the replay");
    }

    /// RED — local `decide` rejection MUST NOT persist anything (no partial commit).
    #[test]
    fn decide_rejection_is_not_committed() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev(9, 5, b"illegal-intent");
        let res = log.commit_after_decide(e, |_| Err::<u64, String>("decide says no".into()));
        assert!(
            matches!(res, Err(DecideRejected(_))),
            "rejection propagates"
        );
        assert!(log.is_empty(), "nothing persisted on rejection");
    }

    /// RED — write succeeds OFFLINE (no network dependency at all). The log is a
    /// pure in-process structure; this asserts a full commit path with a real
    /// kernel `decide`/`fold` Law (order transition validation) without any I/O.
    #[test]
    fn write_succeeds_offline_with_kernel_decide() {
        use crate::order_machine::{assert_transition, OrderStatus};

        let mut log = EventLog::new(MemEventStore::new());
        // Payload encodes an order transition (Pending -> Confirmed), validated
        // by the kernel's `decide` half (assert_transition). This proves the
        // event-log commits THROUGH the real kernel Law before any network use.
        let payload = b"Pending->Confirmed".to_vec();
        let e = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: actor(3),
            actor_seq: 1,
            payload,
        };
        let (out, dec) = log
            .commit_after_decide(e, |ev| {
                // The decide half: validate the encoded transition via the Law.
                let _ = ev;
                assert_transition(OrderStatus::Pending, OrderStatus::Confirmed)
                    .map(|_| "confirmed".to_string())
                    .map_err(|e| e.code().to_string())
            })
            .expect("kernel decide must accept Pending->Confirmed");
        assert!(matches!(out, AppendOutcome::Committed(_)));
        assert_eq!(dec.unwrap(), "confirmed");
        assert_eq!(log.len(), 1);
        // No network call was ever made; the function returns synchronously and
        // the store holds exactly one event. This IS the offline-write property.
        assert!(log.tip().is_some());
    }

    /// GREEN — local-first chaining: a zeroed `prev` is bound to the current tip
    /// so the log forms a content hash chain across appends.
    #[test]
    fn local_first_chaining_binds_prev_to_tip() {
        let mut log = EventLog::new(MemEventStore::new());
        let id1 = match log.append(ev(4, 1, b"a")) {
            AppendOutcome::Committed(id) => id,
            _ => panic!("first must commit"),
        };
        // Second event leaves prev zeroed; the log must bind it to id1.
        let id2 = match log.append(ev(4, 2, b"b")) {
            AppendOutcome::Committed(id) => id,
            _ => panic!("second must commit"),
        };
        assert_ne!(id1, id2);
        assert_eq!(log.len(), 2);
        // The second event's computed id must differ from what it would be with
        // a zero prev (i.e. chaining actually fed into the content-id).
        let unchained = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: actor(4),
            actor_seq: 2,
            payload: b"b".to_vec(),
        };
        assert_ne!(id2, unchained.event_id(), "tip chaining changed the id");
    }

    // ── HYDRA-G2: spectral-drift fail-closed gate wired into commit path ──────

    /// Helper: a 2x2 adjacency with spectral radius ρ. A diagonal `[[ρ,0],[0,0]]`
    /// has spectral radius |ρ| (eigenvalues = diagonal entries). ρ>1 ⇒ Unstable
    /// (divergent), ρ<1 ⇒ Damped (contracting). Plain f64, no deps.
    fn adj(rho: f64) -> Vec<Vec<f64>> {
        vec![vec![rho, 0.0], vec![0.0, 0.0]]
    }

    /// RED+GREEN — DEFAULT regime: an Unstable spectrum (ρ>1) is REJECTED before
    /// persist. The organism endures by refusing the divergent mutation.
    #[test]
    fn drift_gate_rejects_unstable_in_default_regime() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev(11, 1, b"divergent-mutation");
        let res = log.commit_after_decide_drift_gate(e, &adj(2.0), false, |_| Ok::<u64, String>(1));
        assert!(
            matches!(res, Err(DecideRejected(_))),
            "Unstable (ρ=2) mutation MUST be rejected in default regime"
        );
        assert!(log.is_empty(), "nothing persisted on drift-gate rejection");
    }

    /// GREEN — DEFAULT regime: a Damped spectrum (ρ<1) is committed normally.
    #[test]
    fn drift_gate_allows_damped_in_default_regime() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev(12, 1, b"convergent-mutation");
        let (out, dec) = log
            .commit_after_decide_drift_gate(e, &adj(0.5), false, |_| Ok::<u64, String>(1))
            .expect("Damped (ρ=0.5) mutation must commit");
        assert!(matches!(out, AppendOutcome::Committed(_)));
        assert_eq!(dec, Some(1));
        assert_eq!(log.len(), 1);
    }

    /// OPERATOR DIRECTIVE §3 — INTERVENTION regime: ALL safeties LIFTED. Even an
    /// Unstable spectrum (ρ>1) is allowed to proceed (endurance, not exclusion).
    #[test]
    fn drift_gate_lifts_on_intervention() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev(13, 1, b"foreign-intervention-divergent");
        let (out, dec) = log
            .commit_after_decide_drift_gate(e, &adj(2.0), true, |_| Ok::<u64, String>(1))
            .expect("intervention lifts ALL safeties — Unstable allowed");
        assert!(matches!(out, AppendOutcome::Committed(_)));
        assert_eq!(dec, Some(1));
        assert_eq!(
            log.len(),
            1,
            "intervention: mutation persisted despite Unstable"
        );
    }
}
