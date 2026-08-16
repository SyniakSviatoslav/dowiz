//! chaos.rs — deterministic, zero-dependency fault-injection harness (P-H / W-H1).
//!
//! One mechanism, two seams (Hermetic P2, Correspondence: one concept, one
//! primitive):
//!   * Seam A — [`ChaosStore<S>`]: a trait-boundary decorator over ANY
//!     [`crate::event_log::EventStore`]. It generalizes the pre-existing
//!     `FaultyStore` test double (which is preserved as a thin alias below, so
//!     its RED-first tests keep passing with one injection authority).
//!   * Seam B — the `chaos_point!` macro: an inline injection point for code
//!     with no trait seam (e.g. mid-commit, inside a `Mutex` critical section).
//!     That macro and its thread-local plumbing live in the *kernel* shim
//!     (`dowiz-kernel::chaos`), because they need `std` (`thread_local!`); the
//!     pure, deterministic schedule machinery below is `no_std`.
//!
//! ## Compiled out of production (standard §2 item 6)
//!
//! The *kernel* shim gates the whole module `#[cfg(any(test, feature =
//! "chaos"))]`; in a release build the macro expands to `()` and `ChaosStore`
//! is absent. This core module is unconditional (it is a pure data structure +
//! a trait decorator, with no std and no side effects), so the kernel shim's
//! `pub use dowiz_core::chaos::*;` can gate it as one unit.
//!
//! `FaultPlan` draws from the existing seeded [`crate::rng::Rng`] (SplitMix64 →
//! PCG64), so every fault reproduces bit-identically from `(seed, plan)` — no
//! wall-clock, no real sleep, no real network (Hermetic P6, Cause-and-Effect).
//!
//! Firing is recorded (`insert_calls`, call-count per site) so *ordering*
//! properties are falsifiable (e.g. A1: the drift gate rejects BEFORE any store
//! touch ⇒ `ChaosStore.insert_calls == 0`).

use crate::event_log::{EventStore, MeshEvent, StoreError};
use crate::rng::Rng;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Closed set of injection points. Adding a variant is a spec change reviewed
/// against the P-H blueprint (F32 closed-set discipline, mirroring P24's site
/// table — these are INJECTION points, distinct from P24's MEASUREMENT sites).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChaosSite {
    /// Inside `ChaosStore::insert` (seam A).
    StoreInsert,
    /// Event-log commit path: after `decide` returns `Ok`, before
    /// `store.insert` (seam B).
    BetweenDecideAndInsert,
    /// Spool consumer work: between `claim_next` and `ack` (seam B, driver level).
    SpoolConsumerWork,
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
    counts: BTreeMap<ChaosSite, u32>,
}

impl FaultPlan {
    /// Empty plan — every `chaos_point!` / `ChaosStore` is inert.
    pub fn none() -> Self {
        FaultPlan {
            seed: 0,
            stream: 1,
            arms: Vec::new(),
            counts: BTreeMap::new(),
        }
    }

    /// Build a plan from explicit arms. `seed`+`stream` drive any `Probability`
    /// draws so the schedule is reproducible across runs.
    pub fn new(seed: u64, stream: u64, arms: Vec<(ChaosSite, FaultInjection, Trigger)>) -> Self {
        FaultPlan {
            seed,
            stream,
            arms,
            counts: BTreeMap::new(),
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
}
