//! The status vocabulary, pinned against the fourteen hand-written copies it
//! replaces.

use super::status::*;

/// THE DEFECT THIS FILE EXISTS FOR. `feedback` asked
/// `DELIVERED | REJECTED | CANCELLED` and `PICKED_UP` was not in its copy, so
/// a customer who COLLECTED their order was told "this order is still running
/// -- call the venue if something is wrong" and could never leave a note.
#[test]
fn a_collected_order_is_over_and_so_is_a_refunded_one() {
    assert!(is_terminal("PICKED_UP"), "the whole pickup branch could not leave feedback");
    assert!(is_terminal("DELIVERED"));
    assert!(is_terminal("REJECTED"));
    assert!(is_terminal("CANCELLED"));
    assert!(is_terminal("COMPENSATED_REFUND"));
}

/// AN ORDER STILL IN FLIGHT IS NOT OVER, and `REFUNDING` least of all: the
/// money has not moved yet.
#[test]
fn an_order_in_flight_is_not_over() {
    for s in ["PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "REFUNDING"] {
        assert!(!is_terminal(s), "{s}");
    }
}

/// AN UNREADABLE STATUS IS NOT OVER. A running order wrongly called finished
/// disappears from the kitchen's queue; one wrongly called running is visibly
/// stuck and somebody fixes it.
#[test]
fn an_unknown_status_is_never_terminal() {
    for s in ["", "delivered", "DONE", "PICKEDUP", "🙂"] {
        assert!(!is_terminal(s), "{s:?}");
    }
}

/// A REFUSED ORDER IS NOT MONEY TAKEN, and neither is a refunded one.
#[test]
fn refused_and_refunded_orders_are_not_takings() {
    assert!(!took_money("REJECTED"));
    assert!(!took_money("CANCELLED"));
    assert!(!took_money("COMPENSATED_REFUND"), "the venue gave it back");
    assert!(took_money("REFUNDING"), "still the venue's until the refund completes");
    assert!(took_money("DELIVERED"));
    assert!(took_money("PICKED_UP"));
}

/// AN UNKNOWN STATUS COUNTS AS MONEY, deliberately. The other way round, the
/// day the kernel gains a status this Worker has not been redeployed for,
/// every order in it silently leaves the takings and an owner's revenue falls
/// with nothing to explain it.
#[test]
fn an_unknown_status_counts_as_money_rather_than_vanishing() {
    assert!(took_money(""));
    assert!(took_money("SOMETHING_NEW"));
}

/// THE TIP IS THE COURIER'S, passing through — at every one of the four sites
/// that count the venue's money.
#[test]
fn the_tip_is_never_the_venues_and_a_refusal_is_worth_nothing() {
    assert_eq!(venue_took(1000, 200, "DELIVERED"), 800);
    assert_eq!(venue_took(1000, 200, "PICKED_UP"), 800);
    assert_eq!(venue_took(1000, 0, "DELIVERED"), 1000);
    assert_eq!(venue_took(5000, 500, "REJECTED"), 0, "nothing, not 4500");
    assert_eq!(venue_took(5000, 500, "CANCELLED"), 0);
    assert_eq!(venue_took(5000, 500, "COMPENSATED_REFUND"), 0);
}

/// THE KITCHEN'S QUEUE. `PENDING` is waiting to be accepted and is counted
/// separately on the dashboard; a finished order is not work.
#[test]
fn active_is_accepted_and_not_yet_finished() {
    for s in ["CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "REFUNDING"] {
        assert!(is_active(s), "{s}");
    }
    for s in ["PENDING", "DELIVERED", "PICKED_UP", "REJECTED", "CANCELLED", "", "SCHEDULED"] {
        assert!(!is_active(s), "{s}");
    }
}
