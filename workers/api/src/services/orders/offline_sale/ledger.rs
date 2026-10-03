//! PURE. The owner's pane of offline sales (`GET /api/owner/offline_sales`):
//! how many, the oldest, which are past their 48 hours (red in the console),
//! and every conflict a sale was synced with -- read from the orders and the
//! `fiscal` image the object already holds.
//!
//! THE DEADLINE IS READ FROM THE FISCAL ENTRY when there is one (its
//! `queued_at_ms` is the issue instant, `fiscal::queue`), else from the
//! order's own `offline.fiscal_deadline_ms` -- a sale whose document could not
//! be built still owes one, and the pane says `not_queued` beside it.

use super::{overdue, Conflict, PREFIX};
use crate::fiscal::queue::DEADLINE_MS;
use crate::outbox::Entry;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Item {
    pub order_id: String,
    pub sold_at_ms: i64,
    pub synced_at_ms: i64,
    pub total: i64,
    pub currency: String,
    pub by: String,
    pub deadline_ms: i64,
    /// `registered` (codes on the order), `queued`, or `not_queued`.
    pub fiscal: &'static str,
    pub overdue: bool,
    pub alerted: bool,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Pane {
    pub count: usize,
    pub oldest_sold_at_ms: Option<i64>,
    pub overdue: usize,
    pub conflicted: usize,
    /// The platform switch, from the build's own constant (W-WIRE row 6).
    pub send_enabled: bool,
    pub items: Vec<Item>,
}

fn int(v: &Value, p: &str) -> i64 {
    v.pointer(p).and_then(Value::as_i64).unwrap_or(0)
}

/// The pane at `now_ms`. `orders` are the venue's folded orders (any; only
/// offline ones are read); `entries` the `fiscal` image's entries.
pub fn pane(orders: &[Value], entries: &[Entry], marked: &dyn Fn(&str) -> bool, now_ms: i64, send_enabled: bool) -> Pane {
    let mut items: Vec<Item> = orders
        .iter()
        .filter(|o| o.get("id").and_then(Value::as_str).is_some_and(|i| i.starts_with(PREFIX)))
        .map(|o| {
            let id = o["id"].as_str().unwrap_or("").to_string();
            let entry = overdue::offline(entries).into_iter().find(|e| e.to == id);
            let sold = int(o, "/offline/sold_at_ms");
            let deadline = entry.map_or_else(|| sold.saturating_add(DEADLINE_MS), |e| e.queued_at_ms.saturating_add(DEADLINE_MS));
            let registered = o.get("fiscal").is_some_and(|f| !f.is_null());
            let fiscal = if registered { "registered" } else if entry.is_some() { "queued" } else { "not_queued" };
            Item {
                order_id: id,
                sold_at_ms: sold,
                synced_at_ms: int(o, "/offline/synced_at_ms"),
                total: int(o, "/total"),
                currency: o.get("currency").and_then(Value::as_str).unwrap_or("").to_string(),
                by: o.get("placed_by").and_then(Value::as_str).unwrap_or("").to_string(),
                deadline_ms: deadline,
                fiscal,
                overdue: !registered && deadline <= now_ms,
                alerted: entry.is_some_and(|e| marked(&e.id)),
                conflicts: o.pointer("/offline/conflicts").and_then(|c| serde_json::from_value(c.clone()).ok()).unwrap_or_default(),
            }
        })
        .collect();
    items.sort_by_key(|i| (i.sold_at_ms, i.order_id.clone()));
    Pane {
        count: items.len(),
        oldest_sold_at_ms: items.first().map(|i| i.sold_at_ms),
        overdue: items.iter().filter(|i| i.overdue).count(),
        conflicted: items.iter().filter(|i| !i.conflicts.is_empty()).count(),
        send_enabled,
        items,
    }
}
