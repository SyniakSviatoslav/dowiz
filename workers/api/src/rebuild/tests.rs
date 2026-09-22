//! The rebuild-and-diff rules, over nothing but two lists.
//!
//! None of this needs a Durable Object, a log image or a network, which is the
//! point: the check that decides whether the platform's projections can be
//! trusted must itself be checkable.

use super::*;
use serde_json::json;

fn order(id: &str, status: &str) -> (String, String) {
    (id.to_string(), json!({ "id": id, "order_id": id, "status": status }).to_string())
}

fn held(order_id: &str) -> (String, String, i64) {
    (order_id.to_string(), "salmon".to_string(), 80)
}

/// A HEALTHY VENUE IS SILENT. The fold and the memo agree, and every open
/// reservation belongs to an order that has not been prepared yet.
#[test]
fn a_venue_whose_images_agree_reports_nothing() {
    let fresh = vec![order("o1", "PENDING"), order("o2", "DELIVERED")];
    let r = compare(&fresh, &fresh.clone(), &[held("o1")], true);
    assert!(r.intact(), "{r:?}");
    assert_eq!(r.orders, 2);
    assert!(r.unheld.is_empty(), "o2 has been prepared, so nothing is owed for it");
}

/// THE ONE THE WHOLE MODULE EXISTS FOR. The order log says the order is over;
/// the stock ledger is still holding its salmon. Nothing in the platform
/// reconciled these two images before this.
#[test]
fn stock_held_for_an_order_that_has_ended_is_named() {
    let fresh = vec![order("o1", "CANCELLED")];
    let r = compare(&fresh, &fresh.clone(), &[held("o1")], true);
    assert!(!r.intact());
    assert_eq!(r.stranded, vec!["o1"], "a cancelled order may hold nothing");
}

/// AND THE WORSE CASE: a reservation whose order the log has never heard of.
/// No screen anywhere will ever show it, because every screen folds the log.
#[test]
fn stock_held_for_an_order_that_does_not_exist_is_the_worse_case() {
    let r = compare(&[], &[], &[held("ghost")], true);
    assert_eq!(r.stranded, vec!["ghost"]);
    assert_eq!(r.orders, 0);
}

/// A LIVE ORDER MAY HOLD STOCK. This is the case that must NOT be reported, or
/// the gate cries on every venue during service.
#[test]
fn a_pending_order_holding_its_ingredients_is_correct() {
    let fresh = vec![order("o1", "PENDING")];
    let r = compare(&fresh, &fresh.clone(), &[held("o1")], true);
    assert!(r.stranded.is_empty(), "{r:?}");
}

/// PREPARING HAS ALREADY CONSUMED. The ledger holding nothing for it is the
/// right answer, not a missing reservation.
#[test]
fn a_preparing_order_is_not_owed_a_reservation() {
    let fresh = vec![order("o1", "PREPARING")];
    let r = compare(&fresh, &fresh.clone(), &[], true);
    assert!(r.unheld.is_empty(), "{r:?}");
    assert!(r.intact());
}

/// A MEMO THAT DISAGREES WITH THE LOG IS A SERVED LIE, and it is the defect a
/// rebuild exists to find: every reader asks the same memo, so nothing else
/// can ever disagree with it.
#[test]
fn a_memo_whose_payload_differs_from_the_log_is_named() {
    let fresh = vec![order("o1", "READY")];
    let memo = vec![order("o1", "PENDING")];
    let r = compare(&fresh, &memo, &[], false);
    assert_eq!(r.stale, vec!["o1"]);
    assert!(!r.intact());
}

/// BOTH DIRECTIONS. An order missing from the memo is as wrong as one the memo
/// invented, and only checking one way would miss half of it.
#[test]
fn an_order_in_only_one_of_the_two_is_named_either_way() {
    let only_fresh = compare(&[order("o1", "PENDING")], &[], &[], false);
    assert_eq!(only_fresh.stale, vec!["o1"], "the memo is short an order");

    let only_memo = compare(&[], &[order("o1", "PENDING")], &[], false);
    assert_eq!(only_memo.stale, vec!["o1"], "the memo invented an order");
}

/// A VENUE THAT MODELS NO INGREDIENTS RESERVES NOTHING, and saying so is the
/// difference between a useful gate and one every venue without recipes trips.
/// Both live venues are in exactly this state.
#[test]
fn a_venue_with_no_recipes_is_not_accused_of_missing_reservations() {
    let fresh = vec![order("o1", "PENDING"), order("o2", "CONFIRMED")];
    let r = compare(&fresh, &fresh.clone(), &[], false);
    assert!(r.unheld.is_empty(), "nothing is owed when nothing is modelled");
    assert!(r.intact());
    assert!(!r.modelled);
}

/// THE TERMINAL SET COMES FROM THE KERNEL. A hand-written list here would be
/// the fourteenth copy of the status vocabulary — see `tools/gates/vocab.sh`
/// and the four copies it found that were each short by this very state.
#[test]
fn what_counts_as_over_is_the_fsms_own_answer() {
    assert!(is_over("DELIVERED"));
    assert!(is_over("CANCELLED"));
    assert!(is_over("REJECTED"));
    assert!(is_over("PICKED_UP"));
    assert!(is_over("COMPENSATED_REFUND"), "the state four hand copies were missing");
    assert!(!is_over("PENDING"));
    assert!(!is_over("READY"));
    assert!(!is_over("REFUNDING"), "a refund in progress is not over");
    assert!(!is_over("NOT_A_STATUS"), "an unknown word is not terminal, it is unknown");
}

/// One order holding three ingredients is ONE stranded order, not three.
/// A report that counts reservations rather than orders reads as a crisis.
#[test]
fn an_order_holding_several_ingredients_is_named_once() {
    let fresh = vec![order("o1", "REJECTED")];
    let held = vec![
        ("o1".to_string(), "salmon".to_string(), 80),
        ("o1".to_string(), "rice".to_string(), 200),
        ("o1".to_string(), "nori".to_string(), 2),
    ];
    let r = compare(&fresh, &fresh.clone(), &held, true);
    assert_eq!(r.stranded, vec!["o1"]);
}
