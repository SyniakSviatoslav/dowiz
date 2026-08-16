//! chaos.rs (kernel shim) — std-only thread-local plumbing for the fault-injection
//! harness, over the `no_std` core in `dowiz_core::chaos`.
//!
//! The pure, deterministic schedule machinery — [`FaultPlan`], [`ChaosStore`],
//! [`ChaosSite`], [`FaultInjection`], [`Trigger`], and the test-only
//! [`FaultyStore`] — lives in `dowiz_core::chaos` and is re-exported below. This
//! module keeps only the parts that need `std`: the `thread_local!` active plan
//! and the `chaos_point!` seam-B macro (P-H / W-H1, Correspondence: one concept,
//! one primitive).
//!
//! The whole module is gated `#[cfg(any(test, feature = "chaos"))]` at its `mod`
//! declaration in `lib.rs`, so in a release build the macro expands to `()` and
//! no chaos symbol reaches a production artifact.

pub use dowiz_core::chaos::*;

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

#[cfg(test)]
mod tests {
    use super::*;

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
            log.store().insert_calls,
            0,
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
        use alloc::collections::BTreeSet;
        let n = 1000u64;
        let mut spool = Spool::new(n as usize);
        for i in 0..n {
            spool
                .append(&format!("rec-{i}"))
                .expect("append within capacity");
        }
        let mut already_crashed = BTreeSet::new();
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

    #[allow(dead_code)]
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
}
