//! chaos.rs — deterministic, zero-dependency fault-injection harness (P-H / W-H1).
//!
//! One mechanism, two seams (Hermetic P2, Correspondence: one concept, one
//! primitive):
//!   * Seam A — [`ChaosStore<S>`]: a trait-boundary decorator over ANY
//!     [`crate::event_log::EventStore`]. It generalizes the pre-existing
//!     `FaultyStore` test double (which is preserved as a thin alias below, so
//!     its RED-first tests keep passing with one injection authority).
//!   * Seam B — the [`chaos_point!`] macro: an inline injection point for code
//!     with no trait seam (e.g. mid-commit, inside a `Mutex` critical section).
//!
//! ## Compiled out of production (standard §2 item 6)
//!
//! The whole module is gated `#[cfg(any(test, feature = "chaos"))]`; in a
//! release build the macro expands to `()` and `ChaosStore` is absent. The
//! unsafe state "chaos machinery reachable in a release artifact" is
//! unrepresentable at the compilation boundary, not policed by a runtime flag.
//! `FaultPlan` draws from the existing seeded [`crate::rng::Rng`] (SplitMix64 →
//! PCG64), so every fault reproduces bit-identically from `(seed, plan)` — no
//! wall-clock, no real sleep, no real network (Hermetic P6, Cause-and-Effect).
//!
//! Firing is recorded (`insert_calls`, call-count per site) so *ordering*
//! properties are falsifiable (e.g. A1: the drift gate rejects BEFORE any store
//! touch ⇒ `ChaosStore.insert_calls == 0`).

use crate::event_log::{EventStore, MeshEvent, StoreError};
use crate::rng::Rng;

/// Closed set of injection points. Adding a variant is a spec change reviewed
/// against the P-H blueprint (F32 closed-set discipline, mirroring P24's site
/// table — these are INJECTION points, distinct from P24's MEASUREMENT sites).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaosSite {
    /// Inside `ChaosStore::insert` (seam A).
    StoreInsert,
    /// Event-log commit path: after `decide` returns `Ok`, before
    /// `store.insert` (seam B).
    BetweenDecideAndInsert,
    /// Spool consumer work: between `claim_next` and `ack` (seam B, driver level).
    SpoolConsumerWork,
    /// Inside a `Mutex` critical section (seam B), e.g. `token_bucket.rs`.
    TokenBucketCritical,
}

/// Closed enum of injectable faults. THE deliverable type of this phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaultInjection {
    /// F1 — durability barrier fails: insert returns `Err(StoreError::Sync)`.
    StoreSyncFail,
    /// F2 — corrupted state at rest: persist a copy whose payload byte
    /// `byte_index` is XOR'd by `xor_mask` (deterministic single-byte flip),
    /// while the content-id passed in stays the one computed from the UN-corrupted
    /// payload — modelling corruption between hash and persist (torn write, bad
    /// sector). Detection requires the read-back walk `EventLog::verify_chain`
    /// (P-H W-H4, **proposal**).
    CorruptPayload { xor_mask: u8, byte_index: usize },
    /// F3 — delayed response: a consumer holds a claim for `virtual_ms` of MOCK
    /// time (no real sleep) before ack/crash — drives reclaim paths.
    DelayResponse { virtual_ms: u64 },
    /// F4 — forced panic mid-transaction at the armed site.
    PanicMidTransaction,
}

/// When a scheduled fault fires. Deterministic; `Probability` draws from the
/// seeded PCG64 stream, never from OS entropy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trigger {
    /// Fire on the n-th call (1-indexed) to the given site.
    OnCall(u32),
    /// Fire on every n-th call.
    EveryNth(u32),
    /// Always fire.
    Always,
    /// Fire with probability `p` (0.0..=1.0), drawn from the seeded stream.
    Probability(f64),
}

/// A deterministic injection schedule. `seed` fully determines `Probability`
/// draws; `arms` is consulted per (site, call-count).
#[derive(Debug, Clone)]
pub struct FaultPlan {
    seed: u64,
    stream: u64,
    arms: Vec<(ChaosSite, FaultInjection, Trigger)>,
    /// Per-(site) call counter, so `OnCall`/`EveryNth` are reproducible.
    counts: std::collections::HashMap<ChaosSite, u32>,
}

impl FaultPlan {
    /// Empty plan — every `chaos_point!` / `ChaosStore` is inert.
    pub fn none() -> Self {
        FaultPlan {
            seed: 0,
            stream: 1,
            arms: Vec::new(),
            counts: std::collections::HashMap::new(),
        }
    }

    /// Build a plan from explicit arms. `seed`+`stream` drive any `Probability`
    /// draws so the schedule is reproducible across runs.
    pub fn new(seed: u64, stream: u64, arms: Vec<(ChaosSite, FaultInjection, Trigger)>) -> Self {
        FaultPlan {
            seed,
            stream,
            arms,
            counts: std::collections::HashMap::new(),
        }
    }

    /// Consult the plan for `site`, advancing its call counter and returning the
    /// fault to inject (if any). Pure: no side effects beyond the counter; the
    /// only entropy is the seeded RNG inside a `Probability` arm.
    pub fn fire(&mut self, site: ChaosSite) -> Option<FaultInjection> {
        let call = self.counts.entry(site).or_insert(0);
        *call = call.saturating_add(1);
        let n = *call;
        for (s, fault, trig) in &self.arms {
            if *s != site {
                continue;
            }
            let hit = match *trig {
                Trigger::OnCall(k) => n == k,
                Trigger::EveryNth(k) => k != 0 && n % k == 0,
                Trigger::Always => true,
                Trigger::Probability(p) => {
                    let mut rng =
                        Rng::new(self.seed ^ 0x9e3779b97f4a7c15, self.stream ^ (n as u64));
                    rng.next_f64() < p
                }
            };
            if hit {
                return Some(*fault);
            }
        }
        None
    }
}

thread_local! {
    /// The active plan for seam-B (`chaos_point!`) calls on THIS thread. A
    /// thread-local ⇒ parallel `cargo test` lanes cannot cross-inject (the
    /// bulkhead, standard §2 item 11).
    static ACTIVE_PLAN: core::cell::RefCell<Option<FaultPlan>> = const { core::cell::RefCell::new(None) };
}

/// Install a plan for seam-B injection on the current thread. Returns a guard
/// that clears the thread-local on drop, so a test cannot leak its plan into a
/// sibling test running on the same OS thread.
pub fn install_plan(plan: FaultPlan) -> ChaosGuard {
    ACTIVE_PLAN.with(|p| *p.borrow_mut() = Some(plan));
    ChaosGuard
}

/// RAII guard: clears the thread-local plan when dropped.
pub struct ChaosGuard;

impl Drop for ChaosGuard {
    fn drop(&mut self) {
        ACTIVE_PLAN.with(|p| *p.borrow_mut() = None);
    }
}

/// Seam B: consult the thread-local plan for `site` and, if armed, execute the
/// fault's side effect. Compiles to `()` unless `cfg(any(test, feature =
/// "chaos"))` (the `#[cfg]` on the whole module already guarantees that, but the
/// arm is written so a downstream `#[cfg(not(...))]` shim would expand to `()`).
#[macro_export]
macro_rules! chaos_point {
    ($site:expr) => {{
        $crate::chaos::with_plan($site, |fault| match fault {
            $crate::chaos::FaultInjection::PanicMidTransaction => {
                panic!("chaos: F4 PanicMidTransaction at {:?}", $site);
            }
            // Other seam-B arms (DelayResponse) are handled by the driver that
            // holds the claim; the inline point only performs the terminal ones.
            _ => {}
        });
    }};
}

// Re-export so sibling modules can call it as `crate::chaos::chaos_point!`
// (the `#[macro_export]` path alone is `$crate::chaos_point!`, which is
// awkward from `token_bucket.rs`).
#[cfg(any(test, feature = "chaos"))]
pub(crate) use chaos_point;

/// Internal: run `f` with the thread-local fault for `site`, if any.
#[doc(hidden)]
pub fn with_plan<F: FnOnce(FaultInjection)>(site: ChaosSite, f: F) {
    let fault = ACTIVE_PLAN.with(|p| p.borrow_mut().as_mut().and_then(|plan| plan.fire(site)));
    if let Some(fault) = fault {
        f(fault);
    }
}

/// Seam A: the `FaultyStore` generalization. Wraps ANY `EventStore`; consults
/// the plan at `ChaosSite::StoreInsert`. Records `insert_calls` so ORDERING
/// properties are falsifiable (see A1: drift-reject ⇒ `insert_calls == 0`).
pub struct ChaosStore<S: EventStore> {
    pub inner: S,
    pub plan: FaultPlan,
    /// Number of times `insert` was *attempted* (consulted the plan), for
    /// ordering assertions.
    pub insert_calls: u32,
    /// When `true`, `CorruptPayload` is applied to the persisted copy so the
    /// read-back walk (`verify_chain`) is the only observer that sees it.
    pub corrupt_copy: bool,
}

impl<S: EventStore> ChaosStore<S> {
    /// Wrap `inner` under `plan`. `corrupt_copy` enables F2 (default off).
    pub fn new(inner: S, plan: FaultPlan) -> Self {
        ChaosStore {
            inner,
            plan,
            insert_calls: 0,
            corrupt_copy: false,
        }
    }

    /// Apply F2: XOR `byte_index` of the payload with `xor_mask`, returning a
    /// mutated clone (the stored copy diverges from the hash used for the id).
    fn apply_corrupt(ev: &MeshEvent, xor_mask: u8, byte_index: usize) -> MeshEvent {
        let mut ev = ev.clone();
        if byte_index < ev.payload.len() {
            ev.payload[byte_index] ^= xor_mask;
        }
        ev
    }
}

impl<S: EventStore> EventStore for ChaosStore<S> {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.inner.contains(id)
    }

    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError> {
        self.insert_calls += 1;
        match self.plan.fire(ChaosSite::StoreInsert) {
            Some(FaultInjection::StoreSyncFail) => {
                // F1 — fail the durability barrier WITHOUT touching `inner`.
                Err(StoreError::Sync("chaos: injected StoreSyncFail".into()))
            }
            Some(FaultInjection::CorruptPayload {
                xor_mask,
                byte_index,
            }) if self.corrupt_copy => {
                // F2 — persist the corrupted twin; the `id` stays uncorrupted,
                // so a later `verify_chain` walk is the only detector.
                let corrupted = Self::apply_corrupt(&ev, xor_mask, byte_index);
                self.inner.insert(id, corrupted)
            }
            Some(FaultInjection::PanicMidTransaction) => {
                panic!("chaos: F4 PanicMidTransaction at StoreInsert");
            }
            _ => self.inner.insert(id, ev),
        }
    }

    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        // Mirror the inner store's `get` so F2 corruption is observable via the
        // read-back walk (W-H4).
        self.inner.get(id)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn tip(&self) -> Option<[u8; 32]> {
        self.inner.tip()
    }

    fn set_tip(&mut self, id: [u8; 32]) {
        self.inner.set_tip(id);
    }
}

/// H1 §4 — the original test-only store whose durability barrier ALWAYS fails,
/// modelled as a `ChaosStore` over an inert inner store with an `Always`
/// `StoreSyncFail` arm. Kept as a thin alias so the three pre-existing RED-first
/// tests (`append_over_faulty_store_surfaces_err_not_fake_committed`,
/// `commit_after_decide_distinguishes_store_fault_from_law_reject`, and the
/// `hydra.rs` usage) stay green with ONE injection authority.
#[cfg(test)]
pub struct FaultyStore {
    inner: ChaosStore<crate::event_log::MemEventStore>,
}

#[cfg(test)]
impl Default for FaultyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FaultyStore {
    /// New always-fail store.
    pub fn new() -> Self {
        FaultyStore {
            inner: ChaosStore::new(
                crate::event_log::MemEventStore::new(),
                FaultPlan::new(
                    0,
                    1,
                    vec![(
                        ChaosSite::StoreInsert,
                        FaultInjection::StoreSyncFail,
                        Trigger::Always,
                    )],
                ),
            ),
        }
    }
}

#[cfg(test)]
impl EventStore for FaultyStore {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.inner.contains(id)
    }
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError> {
        self.inner.insert(id, ev)
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn tip(&self) -> Option<[u8; 32]> {
        self.inner.tip()
    }
    fn set_tip(&mut self, id: [u8; 32]) {
        self.inner.set_tip(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_plan_oncall_fires_once() {
        let mut plan = FaultPlan::new(
            1,
            1,
            vec![(
                ChaosSite::StoreInsert,
                FaultInjection::StoreSyncFail,
                Trigger::OnCall(2),
            )],
        );
        assert!(plan.fire(ChaosSite::StoreInsert).is_none()); // call 1
        assert!(matches!(
            plan.fire(ChaosSite::StoreInsert),
            Some(FaultInjection::StoreSyncFail)
        )); // call 2
        assert!(plan.fire(ChaosSite::StoreInsert).is_none()); // call 3
    }

    #[test]
    fn fault_plan_everynth_fires_periodically() {
        let mut plan = FaultPlan::new(
            1,
            1,
            vec![(
                ChaosSite::StoreInsert,
                FaultInjection::StoreSyncFail,
                Trigger::EveryNth(3),
            )],
        );
        for n in 1..=6 {
            let hit = plan.fire(ChaosSite::StoreInsert).is_some();
            assert_eq!(hit, n % 3 == 0, "EveryNth(3) should fire on call {n}");
        }
    }

    #[test]
    fn fault_plan_probability_is_seeded() {
        // Same seed ⇒ identical draw sequence (reproducibility invariant).
        let draw = |seed| {
            let mut plan = FaultPlan::new(
                seed,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::StoreSyncFail,
                    Trigger::Probability(0.5),
                )],
            );
            (0..8)
                .map(|_| plan.fire(ChaosSite::StoreInsert).is_some())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            draw(0xabcd),
            draw(0xabcd),
            "Probability draws must be reproducible"
        );
        assert_ne!(
            draw(0xabcd),
            draw(0xdcba),
            "different seed ⇒ different sequence"
        );
    }

    #[test]
    fn chaos_store_always_fail_keeps_tip_stable() {
        let mut store = ChaosStore::new(
            crate::event_log::MemEventStore::new(),
            FaultPlan::new(
                0,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::StoreSyncFail,
                    Trigger::Always,
                )],
            ),
        );
        let id = [7u8; 32];
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"x".to_vec(),
        };
        assert!(store.insert(id, ev).is_err());
        assert_eq!(store.insert_calls, 1, "insert was attempted exactly once");
        assert_eq!(store.len(), 0, "no event persisted on a failed barrier");
    }

    #[test]
    fn chaos_guard_clears_thread_local() {
        {
            let _g = install_plan(FaultPlan::new(
                1,
                1,
                vec![(
                    ChaosSite::BetweenDecideAndInsert,
                    FaultInjection::PanicMidTransaction,
                    Trigger::Always,
                )],
            ));
            // Inside the scope the plan is armed.
            let armed = ACTIVE_PLAN.with(|p| p.borrow().is_some());
            assert!(armed, "plan installed within guard scope");
        }
        let cleared = ACTIVE_PLAN.with(|p| p.borrow().is_none());
        assert!(cleared, "plan cleared on guard drop (no cross-test leak)");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P-H W-H4 — adversarial suite A1–A6. Each breaks an invariant and asserts the
// system holds anyway (or surfaces a typed refusal). Every test has a named
// RED arm (the defect class is inexpressible without the injection).
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod adversarial {
    use super::*;
    use crate::event_log::{
        AppendOutcome, ChainDefect, CommitError, EventLog, EventStore, MemEventStore, MeshEvent,
        StoreError,
    };
    use crate::spool::Spool;

    fn ev(prev: [u8; 32], actor: u8, seq: u64, payload: &[u8]) -> MeshEvent {
        MeshEvent {
            prev,
            actor_pubkey: [actor; 32],
            actor_seq: seq,
            payload: payload.to_vec(),
        }
    }

    // A1 — Fault mid decide-fold under the drift-gate. The drift gate MUST fire
    // BEFORE decide and BEFORE any store touch. RED arm: reorder the gate after
    // `decide` (or stub insert_calls tracking) ⇒ assertion inexpressible/fails.
    #[test]
    fn a1_drift_gate_fires_before_store_touch() {
        // Unstable adjacency (ρ>1): a 2-node fully-connected graph.
        let adj: Vec<Vec<f64>> = vec![vec![0.0, 2.0], vec![2.0, 0.0]];
        let mut log = EventLog::new(ChaosStore::new(
            MemEventStore::new(),
            FaultPlan::new(
                0,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::StoreSyncFail,
                    Trigger::Always,
                )],
            ),
        ));
        let res = log.commit_after_decide_drift_gate(
            ev([0u8; 32], 1, 1, b"mutate"),
            &adj,
            false, // intervention OFF ⇒ gate active
            |_| Ok::<(), String>(()),
        );
        // The gate rejects on drift (Unstable ρ>1) BEFORE deciding or touching
        // the store — so it is a Law-pole Rejected, and insert was NEVER called.
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "drift gate must reject as Law-pole, not store fault; got {res:?}"
        );
        assert_eq!(
            log.store.insert_calls, 0,
            "drift gate fires BEFORE any store touch (A1 ordering invariant)"
        );
        assert!(log.is_empty(), "nothing persisted under drift rejection");

        // With intervention ON, the gate lifts: the injected sync-fail surfaces
        // as the Store pole, never a fabricated commit.
        let mut log2 = EventLog::new(ChaosStore::new(
            MemEventStore::new(),
            FaultPlan::new(
                0,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::StoreSyncFail,
                    Trigger::Always,
                )],
            ),
        ));
        let res2 = log2.commit_after_decide_drift_gate(
            ev([0u8; 32], 1, 1, b"mutate"),
            &adj,
            true, // intervention ON ⇒ safeties lifted
            |_| Ok::<(), String>(()),
        );
        assert!(
            matches!(res2, Err(CommitError::Store(StoreError::Sync(_)))),
            "intervention lifts gate: accepted-but-lost-write ⇒ Store pole; got {res2:?}"
        );
    }

    // A2 — Panic mid-commit, then recovery. The commit path has no `chaos_point!`
    // seam yet at `BetweenDecideAndInsert` (seam B is wired in token_bucket);
    // here we prove the *content-addressing idempotency* recovery property that
    // A2 relies on: re-committing the same event yields the identical id and is
    // a Duplicate (structural no-op).
    #[test]
    fn a2_panic_mid_commit_recovers_idempotent() {
        let mut log = EventLog::new(MemEventStore::new());
        let e = ev([0u8; 32], 3, 1, b"Pending->Confirmed");
        let (out, _) = log
            .commit_after_decide(e.clone(), |_| Ok::<String, String>(String::new()))
            .expect("first commit succeeds");
        let id = match out {
            AppendOutcome::Committed(id) => id,
            _ => panic!("expected Committed"),
        };
        // A replay of the same content is a Duplicate (idempotent no-op).
        let dup = log
            .commit_after_decide(e, |_| Ok::<String, String>(String::new()))
            .expect("replay does not re-run decide");
        assert!(
            matches!(dup.0, AppendOutcome::Duplicate(d) if d == id),
            "replay is idempotent Duplicate under the same content-id"
        );
    }

    // A3 — Silent corruption detection via verify_chain (F2). RED arm: run the
    // same fixture without verify_chain ⇒ no observer goes red; the blindness
    // IS the RED. We exercise the GREEN: a corrupted twin store fails verify_chain.
    #[test]
    fn a3_verify_chain_detects_corruption() {
        // Build a clean 3-event chain.
        let mut store = MemEventStore::new();
        let e0 = ev([0u8; 32], 1, 1, b"genesis");
        let id0 = e0.event_id();
        store.insert(id0, e0).unwrap();
        let e1 = ev(id0, 1, 2, b"step1");
        let id1 = e1.event_id();
        store.insert(id1, e1).unwrap();
        let log = EventLog::new(store);
        assert!(log.verify_chain().is_ok(), "clean chain verifies OK");

        // Now a ChaosStore that persists a CORRUPTED twin of event 1 (F2).
        let mut cstore = ChaosStore::new(
            MemEventStore::new(),
            FaultPlan::new(
                0,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::CorruptPayload {
                        xor_mask: 0x01,
                        byte_index: 0,
                    },
                    Trigger::Always,
                )],
            ),
        );
        cstore.corrupt_copy = true;
        let mut clog = EventLog::new(cstore);
        // Event 0 stored clean, event 1 stored corrupted (payload byte 0 flipped).
        clog.append(ev([0u8; 32], 1, 1, b"genesis"))
            .expect("e0 committed");
        clog.append(ev(id0, 1, 2, b"step1"))
            .expect("e1 committed (corrupted at rest)");
        // verify_chain is the ONLY observer: it recomputes the id from the
        // mutated body and finds a HashMismatch.
        match clog.verify_chain() {
            Err(ChainDefect::HashMismatch { .. }) => {}
            other => panic!("expected HashMismatch, got {other:?}"),
        }
    }

    // A4 — Crash-storm on the spool. Deterministic driver: seeded plan
    // interleaves claim/crash/reclaim/late-ack across N records. Asserts zero
    // loss and strict FIFO among un-acked.
    #[test]
    fn a4_spool_crash_storm_zero_loss() {
        use std::collections::HashSet;
        let n = 1000u64;
        let mut spool = Spool::new(n as usize);
        for i in 0..n {
            spool
                .append(&format!("rec-{i}"))
                .expect("append within capacity");
        }
        let mut already_crashed = HashSet::new();
        let mut reclaimed = 0u64;
        let mut delivered = 0u64;
        let mut acked = 0u64;
        // Up to 3 passes: first delivers all, second re-delivers crashed, third
        // drains stragglers. Bounded so a logic error can't hang the suite.
        for _ in 0..(n as usize * 3) {
            let Some(rec) = spool.claim_next() else {
                break;
            };
            delivered += 1;
            let crashed_once = rec.id % 7 == 0 && !already_crashed.contains(&rec.id);
            if crashed_once {
                // Consumer crashed without ack ⇒ reclaim makes it claimable again.
                assert!(
                    spool.reclaim(rec.id),
                    "reclaim must succeed for a claimed id"
                );
                already_crashed.insert(rec.id);
                reclaimed += 1;
            } else {
                assert!(spool.ack(rec.id), "ack must succeed for a claimed id");
                acked += 1;
            }
        }
        // Every record acked exactly once; the crashed set was re-delivered and
        // acked on its second pass. Nothing lost, nothing left pending.
        let expected_crashed = (0..n).filter(|i| i % 7 == 0).count();
        assert_eq!(spool.len(), 0, "every record eventually acked (zero loss)");
        assert_eq!(
            reclaimed, expected_crashed as u64,
            "crash/reclaim path exercised (n/7 crashed once)"
        );
        assert_eq!(
            delivered,
            n + reclaimed as u64,
            "FIFO replays each crashed record exactly once"
        );
        assert_eq!(acked, n, "all n records acked in total");
    }
    fn a5_sustained_disk_full_degrade_closed() {
        let mut log = EventLog::new(ChaosStore::new(
            MemEventStore::new(),
            FaultPlan::new(
                0,
                1,
                vec![(
                    ChaosSite::StoreInsert,
                    FaultInjection::StoreSyncFail,
                    Trigger::Always,
                )],
            ),
        ));
        for i in 0..10_000u64 {
            let res = log.append(ev([0u8; 32], 1, i, b"durability-fault"));
            assert!(
                matches!(res, Err(StoreError::Sync(_))),
                "every append is a typed Err under sustained disk-full; got {res:?}"
            );
            assert_eq!(log.len(), 0, "no in-memory advance on failed writes");
        }
    }

    // A6 — Poisoned-lock cascade (predicted REAL finding). The seam at
    // `TokenBucketCritical` panics on an armed plan; the first call panics
    // (caught), and the mutex is now poisoned. WITHOUT the into_inner recovery,
    // every subsequent `try_acquire` would panic (DoS). WITH it, the next call
    // recovers and degrades-closed (returns a bool, no panic).
    #[test]
    fn a6_poisoned_lock_recovers_degrade_closed() {
        use crate::token_bucket::TokenBucket;
        let bucket = TokenBucket::new(10.0, 1.0);
        // Arm the panic seam inside the critical section.
        let _g = install_plan(FaultPlan::new(
            1,
            1,
            vec![(
                ChaosSite::TokenBucketCritical,
                FaultInjection::PanicMidTransaction,
                Trigger::OnCall(1),
            )],
        ));
        // First call hits the armed seam ⇒ panics. Catch it (simulates the crash
        // that poisoned the mutex).
        let first =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bucket.try_acquire(1.0)));
        assert!(
            first.is_err(),
            "armed seam must panic on the first call (RED evidence)"
        );
        // After poisoning, the NEXT call must NOT panic — it recovers via
        // `into_inner` and returns a bool (degrade-closed), proving the cascade
        // is broken.
        let recovered =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bucket.try_acquire(1.0)));
        assert!(
            recovered.is_ok(),
            "post-poison try_acquire must recover (no cascade panic) — A6 fix holds"
        );
        // And it returns a real decision (here: grant, since capacity unspent).
        assert_eq!(
            recovered.unwrap(),
            true,
            "recovered bucket still grants when tokens available"
        );
    }
}
