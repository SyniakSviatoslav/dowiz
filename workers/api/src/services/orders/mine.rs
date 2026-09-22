//! PURE. Which orders are THIS venue's.
//!
//! THE WORKER IS MULTI-TENANT EVEN THOUGH A HUB IS NOT. A venue's Durable
//! Object holds that venue's log, but the venue that predates the objects was
//! seeded from a `hub_image` row keyed by image id with no venue column, so its
//! image can still hold more than one venue's orders. Every fold therefore
//! filters. An unfiltered one shows a neighbouring venue's takings, and the
//! seed being gone does not un-write the records it already left behind.
//!
//! THERE WERE TWO RULES. `extra::orders_of` kept an order whose `location_id`
//! is absent; `owner::dashboard` dropped it. Same log, same screen, two
//! answers -- the takings tile and the analytics pane could disagree about the
//! same order, which is precisely the defect the tile's own comment says was
//! already fixed once.

use serde_json::Value;

/// Does this order belong to `loc`?
///
/// AN ORDER WITH NO `location_id` BELONGS TO THE VENUE WHOSE LOG IT IS IN.
/// This is the permissive answer and it is the right one: a venue's object is
/// addressed by venue id, so an order that reached it was placed against it,
/// and the field is a check rather than the source of truth. Dropping such an
/// order would silently delete a venue's oldest takings -- money vanishing is
/// not a thing an owner can see or report.
pub fn belongs_to(order: &Value, loc: &str) -> bool {
    order.get("location_id").and_then(Value::as_str).is_none_or(|l| l == loc)
}

/// This venue's orders, parsed, from the log's projection.
///
/// AN UNREADABLE ENVELOPE IS DROPPED, not counted as this venue's: it cannot
/// be attributed to anyone, and `witness` is the instrument that notices a log
/// whose records stopped parsing.
pub fn of_venue(listed: Vec<crate::hubdo::OrderView>, loc: &str) -> Vec<Value> {
    listed
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| belongs_to(o, loc))
        .collect()
}
