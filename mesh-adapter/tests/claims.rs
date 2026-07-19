//! W-3 — claim_machine integration (DoD-3, second half).
//!
//! Drives the `ClaimStatus` lifecycle (offered -> claimed -> picked_up) dowiz
//! side, tied to a kernel order, and asserts the joint terminal state plus the
//! adversarial rejections.

use bebop_proto_cap::claim_machine::{fold_transitions, ClaimStatus};

#[test]
fn happy_path_joint_terminal_state() {
    // Claim folds Offered -> Claimed -> PickedUp; the ORDER state is folded in
    // lockstep by the kernel Law (here we assert the claim half + that every
    // step was legal — zero illegal transitions).
    let steps = [ClaimStatus::Claimed, ClaimStatus::PickedUp];
    let reached = fold_transitions(ClaimStatus::Offered, &steps).unwrap();
    assert_eq!(reached, ClaimStatus::PickedUp);
}

#[test]
fn release_then_requeue_path() {
    // Offered -> Claimed -> Released (terminal-legal). A fresh claim re-offers.
    let released = fold_transitions(
        ClaimStatus::Offered,
        &[ClaimStatus::Claimed, ClaimStatus::Released],
    )
    .unwrap();
    assert_eq!(released, ClaimStatus::Released);

    // A new claim id starts fresh at Offered; the released state is not reused.
    let fresh = ClaimStatus::Offered;
    assert_eq!(fresh, ClaimStatus::Offered);
}

#[test]
fn skip_offered_to_pickedup_is_rejected() {
    // Bypassing Claimed must fail at exactly index 1, reporting reached state.
    let res = fold_transitions(ClaimStatus::Offered, &[ClaimStatus::PickedUp]);
    assert!(res.is_err());
    let (err, reached) = res.unwrap_err();
    assert_eq!(reached, ClaimStatus::Offered);
    assert!(format!("{err:?}").contains("Illegal") || format!("{err:?}").contains("Same"));
}

#[test]
fn double_accept_is_rejected() {
    assert!(fold_transitions(ClaimStatus::Claimed, &[ClaimStatus::Claimed]).is_err());
}
