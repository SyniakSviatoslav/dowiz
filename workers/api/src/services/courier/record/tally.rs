//! PURE. Three aggregates over one venue's assignments, in one walk.
//!
//! WHAT IT REPLACED: two statements of `COUNT`, `SUM` and `SUM(CASE WHEN …)`
//! over `courier_assignments`. The assignments are the venue's own image now
//! and a fold over a few hundred records is cheaper than a round trip, let
//! alone two.
//!
//! THE DAY IS THE VENUE'S, NOT UTC'S, and that is not this file's decision —
//! it takes `day_start` as an argument precisely so the timezone rule lives in
//! one place and this one can be tested at any boundary. The old code said
//! `now - now.rem_euclid(DAY)`: an owner looking at a courier at 01:00 local
//! saw a "today" that had already started, and the cash that courier was
//! carrying counted against the wrong one.
//!
//! COUNTS ONLY. There is deliberately no average and no rank — DECISIONS D0,
//! trust is a capability and never a score.

use serde_json::Value;

/// What one courier has done: today, over thirty days, and right now.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub today_deliveries: i64,
    /// In minor units. Cash the courier is carrying and owes the venue.
    pub today_cash: i64,
    pub delivered_30d: i64,
    /// Assigned and not yet delivered. NOT a count of failures: an assignment
    /// taken two minutes ago is in flight and so is one abandoned in March,
    /// and this platform cannot yet tell them apart — see the order FSM's
    /// missing exit.
    pub in_flight: i64,
}

/// Thirty days, in milliseconds. A month is not a fixed number of days, and a
/// rolling window is what the console's tile actually claims.
pub const THIRTY_DAYS_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Fold this venue's assignments for one courier.
///
/// An assignment belonging to another courier is skipped rather than counted
/// as zero — the two look identical in the output and only one of them is
/// true.
pub fn tally<'a>(
    assignments: impl IntoIterator<Item = &'a Value>,
    courier_id: &str,
    day_start: i64,
    now_ms: i64,
) -> Tally {
    let mut t = Tally::default();
    for a in assignments {
        if a.get("courier_id").and_then(Value::as_str) != Some(courier_id) {
            continue;
        }
        match a.get("delivered_at_ms").and_then(Value::as_i64) {
            // UNDELIVERED IS IN FLIGHT, and an explicit null is undelivered:
            // the field is written when the delivery lands.
            None => t.in_flight += 1,
            Some(at) => {
                if at >= day_start {
                    t.today_deliveries += 1;
                    t.today_cash += a.get("cash_collected").and_then(Value::as_i64).unwrap_or(0);
                }
                if at >= now_ms - THIRTY_DAYS_MS {
                    t.delivered_30d += 1;
                }
            }
        }
    }
    t
}
