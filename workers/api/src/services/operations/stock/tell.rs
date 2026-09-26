//! WHAT A STOCK MOVEMENT TELLS THE GROUPS (W0a), pure: the plan that was
//! applied, the shelf before and after, the lots after, in; the events and
//! their data (`notify::route::render::event`'s shapes) out.
//!
//!   stock.received     {name, qty, unit, lot?, expiry?}
//!   stock.wasted       {name, qty, unit, reason}
//!   stocktake.variance {items: [{name, expected, observed, unit}]}  (drift only)
//!   stock.low          {items: [{name, on_hand, low_at, unit}]}      (crossings)
//!   stock.expiring     {items: [{name, qty, unit, expiry}]}          (once a day)
//!
//! None of them names a customer.

use dowiz_hub::stock::lots::Lot;
use dowiz_hub::stock::meta::{day_number, show_day};
use dowiz_hub::stock::StockEvent;
use serde_json::{json, Value};

use super::moves::Plan;
use super::view::EXPIRY_WARN_DAYS;
use crate::notify::route::produce::{self, Shelf, Supply};

fn named(supply: &dyn Fn(&str) -> Option<Supply>, id: &str) -> (String, String) {
    supply(id).map_or_else(|| (id.to_string(), String::new()), |s| (s.name, s.unit))
}

/// The events of one applied plan. `shown` is `Plan::apply`'s answer (the
/// counted lines carry `expected`). `expiring` is `Some(lots)` on the first
/// movement of the venue's day, else `None`.
pub fn events(
    plan: &Plan,
    shown: &Value,
    before: &dyn Fn(&str) -> Shelf,
    after: &dyn Fn(&str) -> Shelf,
    supply: &dyn Fn(&str) -> Option<Supply>,
    expiring: Option<(&[&Lot], i64)>,
) -> Vec<(&'static str, Value)> {
    let mut out: Vec<(&'static str, Value)> = Vec::new();
    for (ev, meta) in &plan.lines {
        match ev {
            StockEvent::Received { item, qty } => {
                let (name, unit) = named(supply, item);
                let mut d = json!({ "name": name, "qty": qty, "unit": unit });
                if let Some(l) = &meta.lot {
                    d["lot"] = json!(l);
                }
                if let Some(e) = meta.expiry {
                    d["expiry"] = json!(show_day(e));
                }
                out.push(("stock.received", d));
            }
            StockEvent::Wasted { item, qty, reason, .. } => {
                let (name, unit) = named(supply, item);
                out.push(("stock.wasted", json!({ "name": name, "qty": qty, "unit": unit, "reason": reason.as_str() })));
            }
            _ => {}
        }
    }
    let drift: Vec<Value> = shown
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|l| l.get("drift").and_then(Value::as_i64).is_some_and(|d| d != 0))
        .map(|l| {
            let (name, unit) = named(supply, l["item"].as_str().unwrap_or(""));
            json!({ "name": name, "expected": l["expected"], "observed": l["observed"], "unit": unit })
        })
        .collect();
    if !drift.is_empty() {
        out.push(("stocktake.variance", json!({ "items": drift })));
    }
    if let Some(d) = produce::low(&plan.items(), before, after, supply) {
        out.push(("stock.low", d));
    }
    if let Some((lots, today)) = expiring {
        if let Some(d) = soon(lots, today, supply) {
            out.push(("stock.expiring", d));
        }
    }
    out
}

/// The lots still on the shelf whose date is within the warning window
/// (or past it), soonest first. `None` when there are none.
pub fn soon(lots: &[&Lot], today: i64, supply: &dyn Fn(&str) -> Option<Supply>) -> Option<Value> {
    let mut near: Vec<&&Lot> =
        lots.iter().filter(|l| l.left > 0 && l.expiry.is_some_and(|e| day_number(e) - day_number(today) <= EXPIRY_WARN_DAYS)).collect();
    near.sort_by_key(|l| l.expiry);
    let rows: Vec<Value> = near
        .iter()
        .map(|l| {
            let (name, unit) = named(supply, &l.item);
            json!({ "name": name, "qty": l.left, "unit": unit, "expiry": l.expiry.map(show_day) })
        })
        .collect();
    (!rows.is_empty()).then(|| json!({ "items": rows }))
}

#[cfg(test)]
#[path = "tell/tests.rs"]
mod tests;
