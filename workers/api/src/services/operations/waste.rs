//! The waste report: what was binned, why, and who signed for it.
//!
//! TWO LOGS, ONE PURE FOLD (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.1). Food leaves
//! a venue unsold in two ways and each is recorded where it belongs:
//!
//! - a WRITE-OFF is a `Wasted` event on the STOCK log (ingredient units), and
//! - a line VOIDED AFTER THE KITCHEN HAD IT with reason `dropped` is an
//!   `Amended` event on the ORDER log (portions). The shelf is deliberately not
//!   touched by that void -- the ingredients were `Consumed` at `PREPARING`
//!   and are gone whether the plate was eaten or dropped (`room_rules::Stage`)
//!   -- so the stock log alone never shows it, and a report of the stock log
//!   alone under-reports exactly the loss a floor causes.
//!
//! Every row is ONE event with its signer. A write-off recorded before signers
//! existed is reported with `by: null`, never with an invented name.
//!
//! READS two images; writes none (`tools/gates/one-image.sh` counts writes).

use crate::command::room_rules::{self as rr, Stage, VoidReason};
use dowiz_hub::stock::{self, StockEvent};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use worker::*;

/// One binned thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WasteRow {
    /// `"stock"` (a write-off, in the ingredient's unit) or `"void"` (a line
    /// voided after the kitchen, in portions). Never summed together.
    pub source: &'static str,
    /// The ingredient id, or the voided line's product id.
    pub item: String,
    pub qty: i64,
    pub reason: String,
    /// The signer. `None` only for a write-off recorded before 2026-09-23.
    pub by: Option<String>,
    /// For a void: the round it left.
    pub order: Option<String>,
    /// For a void: when (the stock log carries no clock).
    pub at: Option<i64>,
    /// For food back from a door: who chose to bin it (`by` is the courier
    /// who brought it back).
    pub chosen_by: Option<String>,
}

/// The fold. `stock` oldest first (`StockLog::events`), `orders` oldest first
/// (`Hub::events_oldest_first`). Pure: same logs, same rows.
pub fn fold(stock: &[StockEvent], orders: &[dowiz_hub::Event]) -> Vec<WasteRow> {
    let mut rows: Vec<WasteRow> = stock.iter().filter_map(stock_row).collect();
    let mut state: HashMap<&str, Value> = HashMap::new();
    for e in orders.iter().filter(|e| e.kind.is_order()) {
        let before = state.remove(e.order_id.as_str()).unwrap_or(Value::Null);
        let after = crate::fold::fold_one(before.clone(), &e.order_json);
        if e.kind == dowiz_hub::EventKind::Amended {
            rows.extend(dropped_after_kitchen(&e.order_id, &before, &after));
        }
        state.insert(e.order_id.as_str(), after);
    }
    rows
}

/// One stock event as a waste row, or not waste.
///
/// EXHAUSTIVE ON PURPOSE. This was `_ => None`, and a new variant that bins
/// food (`Returned`, 2026-09-23) was silently absent from the report until it
/// was added by hand. A variant added to `StockEvent` is now a compile error
/// here, and whoever adds it decides whether it is waste.
fn stock_row(ev: &StockEvent) -> Option<WasteRow> {
    match ev {
        StockEvent::Wasted { item, qty, reason, .. } => Some(WasteRow {
            source: "stock",
            item: item.clone(),
            qty: *qty,
            reason: reason.as_str().to_string(),
            by: stock::signer(ev).map(str::to_string),
            order: None,
            at: None,
            chosen_by: None,
        }),
        // FOOD BACK FROM A DOOR, BINNED (§2.4). Its ingredients left the shelf
        // at `Consumed`; this is the record that they were thrown away, not a
        // second subtraction. Resold food is not waste.
        StockEvent::Returned { item, qty, order_id, resell: false, by, chosen_by } => Some(WasteRow {
            source: "stock",
            item: item.clone(),
            qty: *qty,
            reason: "returned".into(),
            by: Some(by.clone()).filter(|b| !b.is_empty()),
            order: Some(order_id.clone()),
            at: None,
            chosen_by: Some(chosen_by.clone()),
        }),
        StockEvent::Returned { resell: true, .. }
        | StockEvent::Received { .. }
        | StockEvent::Reserved { .. }
        | StockEvent::Consumed { .. }
        | StockEvent::Released { .. }
        | StockEvent::Stocktake { .. }
        | StockEvent::Served { .. }
        | StockEvent::Unserved { .. } => None,
    }
}

/// The lines one amendment took off a round the kitchen had, for `dropped`.
fn dropped_after_kitchen(order: &str, before: &Value, after: &Value) -> Vec<WasteRow> {
    let len = |v: &Value| v.get("amended").and_then(Value::as_array).map_or(0, Vec::len);
    if len(after) <= len(before) {
        return Vec::new(); // not an amendment entry (a transfer's has no ops; a delta may touch nothing)
    }
    let status = before.get("status").and_then(Value::as_str).unwrap_or("");
    if rr::stage(status) != Stage::InKitchen {
        return Vec::new(); // before the kitchen the shelf was restocked; nothing was binned
    }
    let Some(entry) = after.get("amended").and_then(Value::as_array).and_then(|a| a.last()) else {
        return Vec::new();
    };
    let reason = entry.get("reason").and_then(Value::as_str).and_then(VoidReason::parse);
    if reason != Some(VoidReason::Dropped) {
        return Vec::new();
    }
    let items = before.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let ops = entry.get("ops").and_then(Value::as_array).cloned().unwrap_or_default();
    ops.iter()
        .filter(|op| op.get("op").and_then(Value::as_str) == Some("remove"))
        .filter_map(|op| items.get(op.get("line")?.as_u64()? as usize))
        .map(|line| WasteRow {
            source: "void",
            item: line.get("product_id").and_then(Value::as_str).unwrap_or("").to_string(),
            qty: line.get("quantity").and_then(Value::as_i64).unwrap_or(0),
            reason: "dropped".into(),
            by: entry.get("by").and_then(Value::as_str).map(str::to_string),
            order: Some(order.to_string()),
            at: entry.get("at").and_then(Value::as_i64),
            chosen_by: None,
        })
        .collect()
}

/// Totals per source and reason, and per signer. Units differ by source, so
/// a write-off's grams and a void's portions are never added together.
pub fn totals(rows: &[WasteRow]) -> Value {
    let mut by_reason: BTreeMap<&str, BTreeMap<&str, i64>> = BTreeMap::new();
    let mut by_signer: BTreeMap<&str, BTreeMap<String, i64>> = BTreeMap::new();
    for r in rows {
        *by_reason.entry(r.source).or_default().entry(r.reason.as_str()).or_default() += r.qty;
        let who = r.by.clone().unwrap_or_else(|| "(unsigned)".into());
        *by_signer.entry(r.source).or_default().entry(who).or_default() += r.qty;
    }
    json!({ "byReason": by_reason, "bySigner": by_signer })
}

/// `GET /api/owner/stock/waste` — every binned thing, with its signer.
pub async fn waste_report(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, loc, (stock, hub)) = match crate::services::identity::staff::guard::staff_beside(&req, &ctx, &place, &crate::services::identity::staff::guard::SHELF, async {
        let (s, h) = futures_util::future::join(crate::hubstore::load_stock(&place), crate::hubstore::load(&place)).await;
        Ok((s?.stock, h?.hub))
    })
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let rows = fold(&stock.events(), &hub.events_oldest_first());
    let totals = totals(&rows);
    Response::from_json(&json!({ "venue": loc, "rows": rows, "totals": totals }))
}

#[cfg(test)]
mod tests;
