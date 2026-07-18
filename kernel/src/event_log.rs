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

/// Durability fault taxonomy for a persistent [`EventStore`] (H1 — restore the
/// missing failure pole on the substrate everything replays from). A durable
/// append has four failure points; naming each lets a caller/operator tell an
/// open-permission problem from a lost-fsync (the genuinely dangerous one).
///
/// Carries a rendered `io::Error` string (not the non-`Clone`/non-`Eq`
/// `io::Error` itself) so the enum stays `Debug + Clone + PartialEq + Eq` and
/// test assertions / the `Copy` success type are unaffected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Backing file could not be opened for append (was: silent fall-through).
    Open(String),
    /// `write_all` of the event line failed.
    Write(String),
    /// `flush` of buffered bytes to the OS failed.
    Flush(String),
    /// `sync_all`/fsync failed — bytes may not have reached stable storage.
    Sync(String),
}

/// Persistence seam for the event-log. The production node backs this with
/// pgrust; tests use [`MemEventStore`]. A std-only durable variant
/// ([`crate::hydra::FileEventStore`]) is available for the Воля АНУ closed loop
/// (offline, egress-free, no external DB dependency).
pub trait EventStore {
    /// Whether an event with this content-id is already persisted (idempotency).
    fn contains(&self, id: &[u8; 32]) -> bool;
    /// Persist an event under its content-id. Returns `Err(StoreError)` if the
    /// durability barrier fails (open/write/flush/sync) — a lost write is now
    /// typed, never a silent success (H1: restore the failure pole).
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError>;
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
    /// Full event bodies, keyed by content-id, so the read-back walk
    /// (`EventLog::verify_chain`, P-H W-H4 F2) can detect corruption at rest.
    by_event: std::collections::HashMap<[u8; 32], MeshEvent>,
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
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError> {
        // A HashSet insert does no IO; honest — it cannot fail.
        if self.by_id.insert(id) {
            self.count += 1;
        }
        self.by_event.insert(id, ev);
        Ok(())
    }
    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        self.by_event.get(id).cloned()
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

/// The two distinct failure poles a `decide`-gated commit can hit (H1). They
/// MUST NOT blur: a Law rejection (correct, never retry, nothing persisted) is
/// categorically different from a store fault (event accepted, durably lost —
/// safe to retry / must alarm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitError {
    /// Law/drift/Locked refused it — do NOT retry (nothing was persisted).
    Rejected(DecideRejected),
    /// Accepted but not durable — retry / raise alarm (the dangerous pole).
    Store(StoreError),
}

/// A per-node, content-addressed, idempotent event-log.
///
/// Local-first: `append`/`commit_after_decide` runs **before** any network IO.
/// Once persistently committed, the event can be gossiped/synced (MESH-07) and
/// the network layer never re-runs `decide` — it only verifies signatures.
pub struct EventLog<S: EventStore> {
    /// The backing store. `pub(crate)` so the chaos adversarial suite (P-H W-H4
    /// A1) can assert `insert_calls == 0` (drift-gate-before-store-touch).
    pub(crate) store: S,
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
    pub fn append(&mut self, mut ev: MeshEvent) -> Result<AppendOutcome, StoreError> {
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
            // Duplicate takes no IO → cannot fail durability.
            return Ok(AppendOutcome::Duplicate(id));
        }
        // The durability barrier gates the tip: `?` short-circuits BEFORE
        // set_tip, so in-memory state never claims an event the store rejected.
        self.store.insert(id, ev)?;
        self.store.set_tip(id);
        Ok(AppendOutcome::Committed(id))
    }

    /// Append WITHOUT local-first `prev` chaining. The event's `prev` field is
    /// taken verbatim (caller-supplied), so the content-id depends only on the
    /// event's own fields — making it deterministic/idempotent across replays
    /// regardless of current chain tip. Used by the Воля АНУ self-witness rows,
    /// whose content-id must be a STABLE function of `(node_id, group_size)` so a
    /// re-received breach alert is a structural no-op, not a new row. Still
    /// idempotent on plain duplicate content-ids (the store dedups by id).
    pub fn append_raw(&mut self, ev: MeshEvent) -> Result<AppendOutcome, StoreError> {
        let id = ev.event_id();
        if self.store.contains(&id) {
            return Ok(AppendOutcome::Duplicate(id));
        }
        self.store.insert(id, ev)?;
        self.store.set_tip(id);
        Ok(AppendOutcome::Committed(id))
    }

    /// MESH-06 commit gate: run the kernel `decide` Law on the event **before**
    /// committing. If `decide` rejects, the event is NOT persisted (no partial
    /// commit) and the store is left exactly as it was. If it accepts, the event
    /// is appended (idempotently — a duplicate is still a no-op and `decide` is
    /// NOT re-run, preserving state). This is the local-before-network contract.
    ///
    /// `decide` returns `Ok(decision)` (carried back to the caller) or
    /// `Err(reason)`. The decision value is opaque to the log.
    ///
    /// # P07 §2 — dedup keys off a STABLE content-id (no local-first rebind)
    ///
    /// The dedup guard below computes `ev.event_id()` from the event's fields
    /// **exactly as the caller supplied them** (its raw content-id), and the
    /// commit is persisted under that SAME raw id via [`Self::append_raw`] — NOT
    /// via [`Self::append`], which rebinds a zero `prev` to the tip and would key
    /// the store under a *different* (chain-bound) id. Keying dedup and storage on
    /// one id is what makes a replay a true `Duplicate`: the module contract
    /// ("replays of the same content are idempotent") only holds for a
    /// chain-independent id. Were this to rebind (as `append` does), a replay of a
    /// zero-`prev` event onto a NON-EMPTY log would compute the raw id, miss the
    /// stored rebound id, re-run `decide`, and chain the replay onto the current
    /// tip as a *fresh* event — a double-run of `decide` and a double-commit of
    /// one logical event (the P07 §2 defect; the money-law violation B2's DoD
    /// gates on: a replayed `SettlementClaimed` must never re-run its hashlock
    /// side effect). A caller that wants explicit local-first chaining calls
    /// [`Self::append`] directly; a decide-gated commit is idempotent by content.
    pub fn commit_after_decide<D, T, E>(
        &mut self,
        ev: MeshEvent,
        decide: D,
    ) -> Result<(AppendOutcome, Option<T>), CommitError>
    where
        D: FnOnce(&MeshEvent) -> Result<T, E>,
        E: std::fmt::Display,
    {
        // Idempotency first: dedup on the RAW content-id — the same id `append_raw`
        // stores under below — so a replay is caught regardless of how far the tip
        // has advanced. A duplicate never re-runs decide (state unchanged).
        let id = ev.event_id();
        if self.store.contains(&id) {
            return Ok((AppendOutcome::Duplicate(id), None));
        }
        // Decide BEFORE commit. On rejection, do not persist anything — this is
        // the Law pole (never retry), kept distinct from the store-fault pole.
        let decision =
            decide(&ev).map_err(|e| CommitError::Rejected(DecideRejected(e.to_string())))?;
        // Commit under the SAME raw id the dedup check tested (stable content-id,
        // no rebind). A durability fault here is the Store pole — accepted but not
        // durable, NOT a Law rejection.
        let outcome = self.append_raw(ev).map_err(CommitError::Store)?;
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
    ) -> Result<(AppendOutcome, Option<T>), CommitError>
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
                // Drift rejection is a Law-pole reject (never retry), not a
                // durability fault.
                return Err(CommitError::Rejected(DecideRejected(format!(
                    "HYDRA drift-gate REJECT: spectrum Unstable (ρ>1) — mutation would diverge; \
                     organism endures by NOT persisting. adjacency={}x{}",
                    adjacency.len(),
                    adjacency.first().map(|r| r.len()).unwrap_or(0)
                ))));
            }
        }
        // Default regime (or intervention lift): proceed to decide-before-commit.
        self.commit_after_decide(ev, decide)
    }

    /// Number of events persisted.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Number of events persisted.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// P-H W-H4 (F2) — read-back integrity walk. Starting from the current tip,
    /// follow `prev` links back to the genesis event, recomputing each event's
    /// content-id (`MeshEvent::event_id()`) from its stored body and comparing
    /// against the id under which it was persisted. Returns the first mismatch
    /// as a typed [`ChainDefect`], or `Ok(())` if the chain is internally
    /// consistent. A corruption *between* hash and persist (torn write / bad
    /// sector) leaves the stored id unchanged but mutates the body — this walk
    /// is the only observer that catches it (the `CorruptPayload` injection in
    /// `chaos.rs` exercises exactly this).
    ///
    /// Requires the backing store to honor `EventStore::get` (the default
    /// returns `None`; `MemEventStore` overrides it). On a store that cannot
    /// read bodies back, the walk degrades-closed: it cannot prove integrity,
    /// so it returns `Err(ChainDefect::Unreadable)`.
    pub fn verify_chain(&self) -> Result<(), ChainDefect> {
        let mut cursor = match self.store.tip() {
            Some(id) => id,
            None => return Ok(()), // empty log is trivially consistent
        };
        // Walk at most `len()` hops to avoid an infinite loop on a cyclic chain.
        let max_hops = self.store.len();
        for _ in 0..=max_hops {
            let ev = match self.store.get(&cursor) {
                Some(ev) => ev,
                None => return Err(ChainDefect::BrokenPrev { at: cursor }),
            };
            // Recompute the id from the (possibly corrupted) body.
            let recomputed = ev.event_id();
            if recomputed != cursor {
                return Err(ChainDefect::HashMismatch {
                    at: cursor,
                    stored: cursor,
                    recomputed,
                });
            }
            // Genesis has a zero `prev`.
            if ev.prev.iter().all(|&b| b == 0) {
                return Ok(());
            }
            cursor = ev.prev;
        }
        // More hops than events ⇒ cycle; treat as broken-prev (unreachable genesis).
        Err(ChainDefect::BrokenPrev { at: cursor })
    }
}

/// P-H W-H4 — the typed defect a [`EventLog::verify_chain`] walk can surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainDefect {
    /// A stored id has no corresponding body (dangling `prev` / missing event).
    BrokenPrev {
        /// The content-id at which the walk could not continue.
        at: [u8; 32],
    },
    /// A persisted body no longer hashes to the id under which it was stored —
    /// the signature of corruption between hash and persist (F2).
    HashMismatch {
        /// The content-id under which the event was persisted.
        stored: [u8; 32],
        /// The id recomputed from the (possibly mutated) body.
        recomputed: [u8; 32],
        /// (kept for symmetry with `BrokenPrev` readability)
        at: [u8; 32],
    },
    /// The backing store cannot read event bodies back (default `get`), so
    /// integrity is unprovable — degrade-closed rather than falsely green.
    Unreadable,
}

/// H1 §4 — test-only store whose durability barrier ALWAYS fails, modelling a
/// full disk / read-only mount. Its `insert` returns `Err(StoreError::Sync)`.
/// This shape is INEXPRESSIBLE against the pre-fix infallible `-> ()` trait —
/// that impossibility is the RED (§4 criterion 2). `set_tip`/`len` track real
/// state so the "no in-memory advance on a failed durability barrier"
/// assertions are genuine (falsifiable), not tautological: a correct `append`
/// short-circuits on the failed `insert?` BEFORE `set_tip`, so the tip/count
/// stay at their empty values. Shared by the event_log + hydra test suites.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct FaultyStore {
    tip: Option<[u8; 32]>,
    count: usize,
}

#[cfg(test)]
impl EventStore for FaultyStore {
    fn contains(&self, _id: &[u8; 32]) -> bool {
        false
    }
    fn insert(&mut self, _id: [u8; 32], _ev: MeshEvent) -> Result<(), StoreError> {
        Err(StoreError::Sync("simulated fsync failure".into()))
    }
    fn len(&self) -> usize {
        self.count
    }
    fn tip(&self) -> Option<[u8; 32]> {
        self.tip
    }
    fn set_tip(&mut self, id: [u8; 32]) {
        // Only reached if a caller advanced the tip; a correct `append`
        // short-circuits on the failed `insert?` before this — keeping the
        // §4 tip/len assertions false-when-buggy.
        self.tip = Some(id);
        self.count += 1;
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

    /// RED→GREEN — P07 §2 dedup-ordering fix. Replaying a zero-`prev` event that
    /// was ORIGINALLY committed onto a NON-EMPTY log must be recognised as a true
    /// `Duplicate` and MUST NOT re-run `decide`.
    ///
    /// The non-empty precondition is load-bearing (B2 DoD hard-gate 1): on an
    /// empty log `tip()` is `None` so there is nothing to rebind onto and the bug
    /// is invisible. Only a replay onto a non-empty log exposes it. Pre-fix,
    /// `commit_after_decide` dedup-checked the RAW zero-`prev` id but persisted via
    /// `append`, which rebinds `prev` to the tip and stores the event under a
    /// *different* (chain-bound) id — so the raw-id dedup check missed, `decide`
    /// re-ran, and `append` chained the replay onto the now-advanced tip as a
    /// brand-new event: a double-run of `decide` AND a full double-commit (log
    /// grew, `Committed` returned for a logical duplicate). The fix keys dedup and
    /// storage on the SAME raw content-id (`append_raw`), so a replay is a
    /// structural no-op. Pinned to a `SettlementClaimed`-shaped payload per the B2
    /// DoD sharpening: a claim replay whose `decide` re-runs the hashlock/preimage
    /// side effect twice is a money-law violation, not a cosmetic bug.
    #[test]
    fn commit_after_decide_replay_on_nonempty_log_is_true_duplicate() {
        let mut log = EventLog::new(MemEventStore::new());
        let mut decide_calls = 0usize;

        // (1) Genesis event onto the EMPTY log — makes the log non-empty (a tip
        // now exists), which is the precondition that exposed the pre-fix bug.
        log.commit_after_decide(ev(1, 1, b"genesis"), |_| {
            decide_calls += 1;
            Ok::<u64, String>(1)
        })
        .expect("genesis commits");
        assert_eq!(log.len(), 1);

        // (2) The ORIGINAL logical event: a zero-`prev`, SettlementClaimed-shaped
        // event committed onto the NON-EMPTY log. `decide` runs exactly once here.
        let target = ev(2, 1, b"SettlementClaimed{s}");
        let (out1, dec1) = log
            .commit_after_decide(target.clone(), |_| {
                decide_calls += 1;
                Ok::<u64, String>(1)
            })
            .expect("original target commits");
        let stored_id = match out1 {
            AppendOutcome::Committed(id) => id,
            other => panic!("first target commit must be Committed, got {other:?}"),
        };
        assert_eq!(dec1, Some(1));
        assert_eq!(log.len(), 2);
        assert_eq!(decide_calls, 2, "one decide per distinct event so far");
        // Content-address consistency (the fix): a decide-gated commit is stored
        // under the event's OWN raw content-id — NOT a rebound/chain-bound id — so
        // the dedup key and the store key are one and the same. (Pre-fix this went
        // through `append`, which rebinds, storing under a *different* id — the
        // divergence the bug exploited.)
        assert_eq!(
            stored_id,
            target.event_id(),
            "commit_after_decide stores under the stable raw content-id (no rebind)"
        );

        // (3) Replay the ORIGINAL zero-`prev` bytes again onto the now-non-empty
        // log (same content, prev still zero, NOT rebound) — the bug trigger.
        let (out2, dec2) = log
            .commit_after_decide(target, |_| {
                decide_calls += 1;
                Ok::<u64, String>(1)
            })
            .expect("replay must not error");

        // Every one of these is FALSE against pre-P07 code, where the replay
        // double-commits: out2 == Committed(new id), dec2 == Some(1), counter==3,
        // len==3.
        assert!(
            matches!(out2, AppendOutcome::Duplicate(_)),
            "replay onto a non-empty log must be a Duplicate, got {out2:?}"
        );
        assert_eq!(
            dec2, None,
            "a duplicate carries NO decision — pre-fix leaks Some(decision)"
        );
        assert_eq!(
            decide_calls, 2,
            "decide MUST run exactly once for the target — pre-fix runs it twice (counter==3)"
        );
        assert_eq!(log.len(), 2, "replay must not grow the log");

        // DoD sharpening: the Duplicate keys off the SAME content-id the commit was
        // stored under — catches a partial fix that reorders the check but mis-keys
        // the store (e.g. dedup on raw id, store under a rebound id, or vice-versa).
        if let AppendOutcome::Duplicate(dup_id) = out2 {
            assert_eq!(
                dup_id, stored_id,
                "duplicate id must equal the stored content-id"
            );
        }
    }

    /// RED — local `decide` rejection MUST NOT persist anything (no partial commit).
    #[test]
    fn decide_rejection_is_not_committed() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev(9, 5, b"illegal-intent");
        let res = log.commit_after_decide(e, |_| Err::<u64, String>("decide says no".into()));
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "rejection propagates as the Law pole"
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
        let id1 = match log.append(ev(4, 1, b"a")).expect("first append durable") {
            AppendOutcome::Committed(id) => id,
            _ => panic!("first must commit"),
        };
        // Second event leaves prev zeroed; the log must bind it to id1.
        let id2 = match log.append(ev(4, 2, b"b")).expect("second append durable") {
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
            matches!(res, Err(CommitError::Rejected(_))),
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

    // ── H1 §4 — the failure pole is real, typed, and tested ──────────────────

    /// ★§4 criterion 2 (RED-first, load-bearing) — a store whose durability
    /// barrier fails makes `append` return `Err(StoreError::Sync(_))`, NEVER a
    /// fabricated `Ok(AppendOutcome::Committed(_))`, and the in-memory tip/len do
    /// NOT advance. This assertion is INEXPRESSIBLE on the pre-fix infallible
    /// `insert(...) -> ()` / `append(...) -> AppendOutcome` signatures — there
    /// was no `Err` to observe (that impossibility is the RED); it passes only
    /// against the fixed fallible substrate.
    #[test]
    fn append_over_faulty_store_surfaces_err_not_fake_committed() {
        let mut log = EventLog::new(FaultyStore::default());
        let res = log.append(ev(1, 1, b"durability-fault"));
        assert!(
            matches!(res, Err(StoreError::Sync(_))),
            "a failed durability barrier must surface as Err(StoreError::Sync), \
             not Ok(Committed); got {res:?}"
        );
        assert!(
            !matches!(res, Ok(AppendOutcome::Committed(_))),
            "MUST NOT fabricate a Committed outcome on a lost write"
        );
        assert!(
            log.tip().is_none(),
            "tip must not advance on a failed barrier"
        );
        assert_eq!(log.len(), 0, "no in-memory advance on a failed barrier");
    }

    /// §4 criterion 3 — `commit_after_decide` keeps the two poles distinct over a
    /// faulty store: a *decide-accepted* event yields `Err(CommitError::Store(_))`
    /// (accepted-but-not-durable); a *decide-rejected* event yields
    /// `Err(CommitError::Rejected(_))` with nothing attempted on the store. The
    /// two are never conflated.
    #[test]
    fn commit_after_decide_distinguishes_store_fault_from_law_reject() {
        // Pole 1 — decide ACCEPTS, but the durability barrier fails → Store.
        let mut log = EventLog::new(FaultyStore::default());
        let accepted = log.commit_after_decide(ev(2, 1, b"accepted"), |_| Ok::<u64, String>(1));
        assert!(
            matches!(accepted, Err(CommitError::Store(StoreError::Sync(_)))),
            "decide-accepted + store fault ⇒ CommitError::Store; got {accepted:?}"
        );
        assert!(log.tip().is_none(), "no tip on a durability fault");
        assert_eq!(log.len(), 0, "no in-memory advance on a durability fault");

        // Pole 2 — decide REJECTS → Rejected, the store is never touched.
        let mut log2 = EventLog::new(FaultyStore::default());
        let rejected =
            log2.commit_after_decide(ev(3, 1, b"illegal"), |_| Err::<u64, String>("no".into()));
        assert!(
            matches!(rejected, Err(CommitError::Rejected(_))),
            "decide-rejected ⇒ CommitError::Rejected (store never attempted); got {rejected:?}"
        );
        assert!(log2.tip().is_none(), "rejection persists nothing");
        assert_eq!(log2.len(), 0, "rejection persists nothing");
    }
}
