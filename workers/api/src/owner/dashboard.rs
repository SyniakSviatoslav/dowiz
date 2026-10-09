//! The owner dashboard's four tiles, as a pure fold over the venue's orders
//! (W-LOOPA row 3, lifted out of `owner::dashboard` so it can be measured and
//! held to the old code).
//!
//! THE CHEAP TEST FIRST. The loop parsed every order's JSON and only then asked
//! `e.seq < day_start`, a comparison of two integers it already had. At 300
//! orders with 50 of them today that was 4.6 ms of a 10 ms Worker cap on every
//! dashboard poll (R-LOOPS B4); testing the seq first parses only today's rows.
//! Every skip in this loop is a pure `continue`, so their order cannot change
//! the tiles (`dashboard/tests.rs` holds the old loop as the oracle).
//!
//! NOT A `break`. The list is newest-first by LOG POSITION (`fold::projection`
//! keys its rows by the newest event's index), but a row's `seq` is
//! `next_seq(prev, now)` with the writer's own clock -- and the e-bill import
//! appends with the TILL's time (`ebills/import/bill.rs`), so a row stamped
//! yesterday can sit above a row stamped today. Stopping at the first older
//! row would drop today's orders below it; the test proves that listing exists.

use crate::hubdo::OrderView;
use serde_json::Value;

/// `todayOrders`, `todayRevenue`, `pending`, `active`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tiles {
    pub count: i64,
    pub revenue: i64,
    pub pending: i64,
    pub active: i64,
}

pub fn tiles(listed: &[OrderView], loc: &str, day_start: i64) -> Tiles {
    let mut t = Tiles::default();
    for e in listed {
        if (e.seq as i64) < day_start {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        // THE SAME TENANCY RULE THE ANALYTICS USE. This was `!= Some(loc)`,
        // which drops an order whose `location_id` is absent, while
        // `orders_of` kept it -- so the takings tile and the analytics pane on
        // the same screen could disagree about the same order. One rule now,
        // in `services::orders::mine`, with a test saying which way it goes
        // and why.
        if !crate::services::orders::mine::belongs_to(&v, loc) {
            continue;
        }
        t.count += 1;
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if status == "PENDING" {
            t.pending += 1;
        } else if crate::services::orders::status::is_active(status) {
            t.active += 1;
        }
        // ── ONE DEFINITION OF TODAY'S TAKINGS ──
        //
        // This counted DELIVERED only while the native adapter and the
        // analytics count every order that was not REFUSED -- so the same
        // product showed an owner two different numbers depending on which
        // deployment they opened, and the dashboard disagreed with its own
        // analytics pane on the same screen. The rule is the analytics one,
        // because that is what the copy on both panes describes: money the
        // venue took, and a rejected order is not that.
        //
        // The tip is subtracted wherever the venue's money is counted: it is
        // the courier's, passing through.
        t.revenue += crate::services::orders::status::venue_took(
            v.get("total").and_then(|t| t.as_i64()).unwrap_or(0),
            v.get("tip").and_then(|t| t.as_i64()).unwrap_or(0),
            status,
        );
    }
    t
}

#[cfg(test)]
mod tests;
