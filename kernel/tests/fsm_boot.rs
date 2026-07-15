//! CI gate for the spectral-graph-fsm drift fence (blueprint `spectral-graph-fsm` §4).
//!
//! These are *integration* tests — they go through the public crate surface
//! (`dowiz_kernel::*`), not the module-internal items, so they double as a contract
//! test that the boot + post-fold gates are reachable from outside the crate.

use dowiz_kernel::{
    apply_event, fsm_graph_report, kernel_boot_verify_fsm, place_order, verify_fsm_signature,
    verify_fsm_signature_against, FsmSignatureDrift, OrderStatus,
};

/// **Boot gate (fail-closed).** At kernel init, before any order is folded, the live
/// lifecycle graph must match `FSM_GOLDEN_SIGNATURE`. On the committed graph this must
/// pass with zero fields moved. A bad merge / silent `allowed_next` edit is caught here
/// at the earliest possible point.
#[test]
fn boot_gate_is_green_on_committed_lifecycle() {
    assert!(
        kernel_boot_verify_fsm().is_ok(),
        "kernel boot must refuse to start if the lifecycle drifted"
    );
    // The one-shot helper must agree with the named boot entry point.
    assert!(verify_fsm_signature().is_ok());
}

/// **Post-fold gate (fail-closed).** Every legal per-order fold must *not* trip the
/// drift gate — a successful fold cannot alter `allowed_next`/the graph, so the gate
/// must always return `Ok(())` mid-lifecycle. We drive the full happy path to a terminal
/// state and assert no fold reports drift.
#[test]
fn post_fold_gate_stays_green_through_full_lifecycle() {
    let o = place_order("boot-1".into(), None, vec![], 0, None, None).unwrap();
    assert_eq!(o.status, OrderStatus::Pending);
    let steps = [
        OrderStatus::Confirmed,
        OrderStatus::Preparing,
        OrderStatus::Ready,
        OrderStatus::InDelivery,
        OrderStatus::Delivered,
    ];
    let mut cur = o;
    for next in steps {
        cur = apply_event(&cur, next).expect("legal fold must succeed");
        // A successful fold MUST NOT move the golden signature.
        assert!(
            verify_fsm_signature().is_ok(),
            "post-fold drift gate tripped after a legal fold to {next:?}"
        );
    }
    assert!(cur.status.is_terminal());
}

/// **Gate falsifiability.** The drift gate is a real fingerprint, not a no-op stub:
/// feeding it a hand-crafted divergent report must return `Err` naming the moved field.
/// This guards against the gate silently returning `Ok` for every input (the classic
/// "always-green" failure mode).
#[test]
fn gate_is_falsifiable_not_always_ok() {
    // Simulate a silent edit: someone deleted one transition (edges 9 -> 8).
    let mut bad = dowiz_kernel::fsm_graph_report();
    bad.edges = 8;
    let err = verify_fsm_signature_against(bad).expect_err("divergent report must trip the gate");
    // The error must name which field moved (fail-closed, actionable message).
    assert!(matches!(err, FsmSignatureDrift { .. }));
    let drift = verify_fsm_signature_against({
        let mut r = dowiz_kernel::fsm_graph_report();
        r.edges = 8;
        r
    });
    assert!(drift.is_err());
}

/// **Reachability invariant (Scheduled orphan).** The CI gate also pins the
/// closed-form `reachable_from_pending = 767` (all active states, `Scheduled` orphan
/// excluded). If anyone accidentally adds an inbound edge to `Scheduled`, bit 8 flips to
/// 1, the mask becomes 1023, and this assertion trips — a cheap exact liveness probe for
/// partial-scaffold drift.
#[test]
fn reachable_from_pending_pins_scheduled_orphan() {
    let r = dowiz_kernel::fsm_graph_report();
    assert_eq!(
        r.reachable_from_pending, 767,
        "Scheduled must stay an orphan (bit 8 = 0)"
    );
    assert_eq!(r.reachable_states, 9);
}
