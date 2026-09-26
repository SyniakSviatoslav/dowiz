//! THE PRODUCERS (W0a/W0b): what an object turn owes the venue's groups,
//! PURE. Each turn that writes an event asks here which routed entries it
//! owes, then writes them into the outbox in its own turn (`hubdo/routed.rs`)
//! -- never as a second image write in a Worker handler.
//!
//! NOTHING IS OWED TO NOBODY: [`wants`] is false unless an active group hears
//! the event, so a venue that never subscribed pays no outbox write.
//!
//! ONCE, WITHOUT A MEMORY WHERE THE LOG IS THE MEMORY: `order.status` is one
//! entry per transition (id = order + status), `stock.low` fires on the
//! CROSSING (free above the threshold before the turn, at or below after).
//! `order.late` and `stock.expiring` have no crossing in the log, so each
//! leaves a MARK in the outbox (a record kind the drain never reads) and
//! prunes the marks that no longer matter.

use serde_json::{json, Value};

use super::{routed, Group, Mode};
use crate::outbox::Entry;

/// The settings key of the late threshold, in minutes (`settings/known.rs`).
pub const LATE_KEY: &str = "notify.order.late_min";
/// Its default when the owner never set it.
pub const LATE_DEFAULT_MIN: i64 = 20;

/// The threshold in ms from the stored text; 0 = off.
pub fn late_ms(raw: Option<&str>) -> i64 {
    raw.and_then(|s| s.trim().parse::<i64>().ok()).filter(|m| *m >= 0).unwrap_or(LATE_DEFAULT_MIN) * 60_000
}

/// Outbox record kind: an order already told as late.
pub const LATE_MARK: &str = "lt";
/// Outbox record kind: the venue day whose expiring lots were told.
pub const EXPIRING_MARK: &str = "sx";

/// Does any active group hear `ev` (now or in its summary)?
pub fn wants(groups: &[Group], ev: &str) -> bool {
    groups.iter().any(|g| g.mode(ev) != Mode::Off)
}

/// `order.status`: one per transition. None of it names a customer.
pub fn status(order_id: &str, next: &str, now_ms: i64) -> Entry {
    let data = json!({ "data": { "order": order_id, "status": next } });
    routed(format!("{order_id}/st/{next}/route"), "order.status", &data, now_ms)
}

/// The statuses in which an order is still waiting for the venue.
const WAITING: [&str; 3] = ["PENDING", "CONFIRMED", "PREPARING"];

/// `order.late`: every open order older than `late_ms` that carries no mark,
/// with the marks to write, and the marks to drop (orders no longer waiting).
/// `orders` are `(id, order json)`.
pub fn late(orders: &[(String, String)], marked: &[String], now_ms: i64, late_ms: i64) -> (Vec<Entry>, Vec<String>, Vec<String>) {
    let mut owed = Vec::new();
    let mut mark = Vec::new();
    let mut waiting: Vec<&str> = Vec::new();
    for (id, j) in orders {
        let Ok(v) = serde_json::from_str::<Value>(j) else { continue };
        let st = v.get("status").and_then(Value::as_str).unwrap_or("");
        if !WAITING.contains(&st) {
            continue;
        }
        waiting.push(id);
        let at = v.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now_ms);
        if late_ms <= 0 || now_ms - at <= late_ms || marked.iter().any(|m| m == id) {
            continue;
        }
        let minutes = (now_ms - at) / 60_000;
        owed.push(routed(format!("{id}/late/route"), "order.late", &json!({ "data": { "order": id, "minutes": minutes } }), now_ms));
        mark.push(id.clone());
    }
    let drop = marked.iter().filter(|m| !waiting.contains(&m.as_str())).cloned().collect();
    (owed, mark, drop)
}

/// One supply, as a message names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supply {
    pub name: String,
    pub unit: String,
    pub low_at: i64,
}

/// A supply's shelf at one instant: what is free, and whether anyone ever counted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shelf {
    pub free: i64,
    pub counted: bool,
}

/// `stock.low`: the items that CROSSED their threshold in this turn --
/// counted (an uncounted zero is unknown, not empty), a threshold set, free
/// above it before and at or below it after. `None` when nothing crossed.
pub fn low(items: &[String], before: &dyn Fn(&str) -> Shelf, after: &dyn Fn(&str) -> Shelf, supply: &dyn Fn(&str) -> Option<Supply>) -> Option<Value> {
    let mut seen: Vec<&str> = Vec::new();
    let mut rows = Vec::new();
    for id in items {
        if seen.contains(&id.as_str()) {
            continue;
        }
        seen.push(id);
        let Some(s) = supply(id) else { continue };
        let (b, a) = (before(id), after(id));
        if s.low_at > 0 && a.counted && b.free > s.low_at && a.free <= s.low_at {
            rows.push(json!({ "name": s.name, "on_hand": a.free, "low_at": s.low_at, "unit": s.unit }));
        }
    }
    (!rows.is_empty()).then(|| json!({ "items": rows }))
}

/// The entries of one turn's events: `base` makes every id unique to the turn.
pub fn entries(base: &str, events: Vec<(&'static str, Value)>, now_ms: i64) -> Vec<Entry> {
    events.into_iter().map(|(ev, data)| routed(format!("{base}/{ev}/route"), ev, &json!({ "data": data }), now_ms)).collect()
}

#[cfg(test)]
#[path = "produce/tests.rs"]
mod tests;
