//! W-AUDIT F3 (2026-09-27): the webhook's status is what tells Stripe whether
//! to send the event again. Final answers are 2xx; a transient failure is not.

use super::ack;

#[test]
fn an_applied_or_duplicate_event_is_acknowledged() {
    let (s, b) = ack(&Ok(true), "o1");
    assert_eq!((s, b["applied"].as_str()), (200, Some("o1")));
    let (s, b) = ack(&Ok(false), "o1");
    assert_eq!((s, b["duplicate"].as_str()), (200, Some("o1")));
}

/// The one FINAL failure: retrying will not make an unknown order appear.
#[test]
fn an_unknown_order_is_acknowledged_so_stripe_stops() {
    let (s, b) = ack(&Err("order not found".into()), "o1");
    assert_eq!(s, 200);
    assert_eq!(b["ok"], serde_json::json!(true));
}

/// Every other failure is transient to this handler's knowledge, and a 2xx
/// would be the last Stripe ever heard of a card the customer paid with.
#[test]
fn a_transient_failure_is_a_503_so_stripe_retries() {
    for e in ["hub unavailable: object reset", "hub object refused an append: 500", "hub log is contended; five attempts lost the generation guard"] {
        let (s, b) = ack(&Err(e.into()), "o1");
        assert_eq!(s, 503, "{e}");
        assert_eq!(b["ok"], serde_json::json!(false));
        assert_eq!(b["retry"].as_str(), Some(e));
    }
}
