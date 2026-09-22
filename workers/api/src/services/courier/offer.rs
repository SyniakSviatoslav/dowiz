//! What a courier has been offered, and what they have finished.

use serde_json::Value;


/// How long a courier has to answer an assignment before it goes back to the
/// pool.
///
/// FIVE MINUTES, and the number is a judgement rather than a constant somebody
/// liked. A minute is not enough: a courier is holding a bike, or a door, or the
/// previous order's change. An evening is far too long -- the food is cold and
/// the customer has phoned twice.
///
/// NOTHING IS AUTO-DECLINED. Lapsing refuses nothing on the courier's behalf and
/// counts against them in no way; it only stops the order being exclusively
/// theirs.
pub const OFFER_WINDOW_MS: i64 = 5 * 60 * 1000;

/// Has this order been offered to somebody who has not answered?
///
/// An ACCEPTED order never lapses however long it takes, and an order assigned
/// before this field existed is not retroactively reopened by one deploy.
pub fn offer_lapsed(o: &Value, now: i64) -> bool {
    if o.get("accepted_at_ms").and_then(Value::as_i64).is_some() {
        return false;
    }
    match o.get("assigned_at_ms").and_then(Value::as_i64) {
        None => false,
        Some(at) => now.saturating_sub(at) >= OFFER_WINDOW_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// AN ACCEPTED ORDER NEVER LAPSES, however long the run takes. A courier
    /// stuck in traffic with the food in their bag must not have the order
    /// taken back into the pool behind them.
    #[test]
    fn an_accepted_order_never_lapses() {
        let o = json!({ "assigned_at_ms": 0, "accepted_at_ms": 1 });
        assert!(!offer_lapsed(&o, i64::MAX));
    }

    /// AN ORDER NOBODY HAS BEEN OFFERED HAS NOTHING TO LAPSE, and an order
    /// assigned before this field existed is not retroactively reopened by
    /// one deploy.
    #[test]
    fn an_unassigned_order_has_no_window() {
        assert!(!offer_lapsed(&json!({}), i64::MAX));
        assert!(!offer_lapsed(&json!({ "assigned_at_ms": null }), i64::MAX));
    }

    /// THE WINDOW IS FIVE MINUTES AND ITS EDGE IS INCLUSIVE: at exactly five
    /// minutes the order goes back to the pool.
    #[test]
    fn the_window_is_five_minutes_and_closes_on_the_boundary() {
        let o = json!({ "assigned_at_ms": 1_000_000 });
        assert!(!offer_lapsed(&o, 1_000_000), "the instant it was offered");
        assert!(!offer_lapsed(&o, 1_000_000 + OFFER_WINDOW_MS - 1));
        assert!(offer_lapsed(&o, 1_000_000 + OFFER_WINDOW_MS));
        assert_eq!(OFFER_WINDOW_MS, 5 * 60 * 1000);
    }

    /// A CLOCK THAT WENT BACKWARDS DOES NOT LAPSE AN OFFER. `saturating_sub`
    /// is the reason: a negative elapsed time would wrap and reopen an order
    /// that was offered one second ago.
    #[test]
    fn a_backwards_clock_does_not_reopen_a_fresh_offer() {
        let o = json!({ "assigned_at_ms": i64::MAX });
        assert!(!offer_lapsed(&o, 0));
    }
}
