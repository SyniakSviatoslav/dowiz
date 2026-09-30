//! THE COST OF GOODS, STAMPED AT PLACEMENT (research 2026-09-26 R4,
//! BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.10): each line that has a recipe
//! records what ONE portion cost at the weighted average of that moment, the
//! way it records its price (`unit_price`) and its tax rate (`vat_ppm`).
//!
//! `unit_cost` is minor units per portion; `cost_at` is the stock log's
//! length the cost book was folded at, so `stock::cost::rebuild` reproduces
//! the number from the log alone (law 8). A line with no recipe, or with a
//! supply no priced delivery has reached, is NOT stamped: a partial sum is not
//! a cost, and the kitchen's numbers fall back to today's average for it, as
//! they do for every order placed before stamps existed.
//!
//! A SALE FROM A BATCH COOKED AHEAD (W-PF3 T1) is stamped with what the
//! ledger booked: the part a ready semi-finished batch gave at that BATCH's
//! own average (the raw it was cooked from, as it cost then), the rest at the
//! raw averages -- each dish the same share of each batch as the basket took
//! (`Basket::portion_cost`), exact inside, rounded once. Law 8's `rebuild`
//! re-folds the all-raw `bom` only, so it does not reproduce such a stamp.
//!
//! THE STAMP IS THE VENUE'S, NOT THE GUEST'S. What a dish costs the kitchen is
//! its margin in plain sight; every read a customer or a courier gets goes
//! through [`strip`] first.

use dowiz_hub::prep::stocked::Shelf;
use dowiz_hub::stock::cost::CostBook;
use serde_json::{json, Value};

/// The two keys a stamp writes on a line.
pub const KEYS: [&str; 2] = ["unit_cost", "cost_at"];

/// Stamp every line of `envelope` whose product has a fully priced recipe.
/// `bom_lines` is the placement's `(product JSON, quantity)`; the product is
/// matched by its top-level `id`. `shelf` is the share of each semi-finished
/// product a ready batch gave (`StockLog::append_draws_split`; empty: none).
pub fn stamp_lines(envelope: &mut Value, bom_lines: &[(String, i64)], book: &CostBook, at: usize, shelf: &Shelf) {
    let costs: Vec<(String, i64)> = bom_lines
        .iter()
        .filter_map(|(product, _)| {
            let id = serde_json::from_str::<Value>(product).ok()?.get("id")?.as_str()?.to_string();
            let from_batch = match shelf.is_empty() {
                true => None,
                false => dowiz_hub::stock::basket::of(&[(product.clone(), 1)]).and_then(|b| b.portion_cost(shelf, book)),
            };
            Some((id, from_batch.unwrap_or_else(|| book.dish_cost(&dowiz_hub::stock::bom_of(product)))?))
        })
        .collect();
    for line in envelope.get_mut("items").and_then(Value::as_array_mut).into_iter().flatten() {
        let Some(pid) = line.get("product_id").and_then(Value::as_str) else { continue };
        if let Some((_, c)) = costs.iter().find(|(id, _)| id == pid) {
            line["unit_cost"] = json!(c);
            line["cost_at"] = json!(at);
        }
    }
}

/// The order as a guest or a courier may read it: no line carries its cost.
pub fn strip(order: &mut Value) {
    for line in order.get_mut("items").and_then(Value::as_array_mut).into_iter().flatten() {
        if let Some(o) = line.as_object_mut() {
            for k in KEYS {
                o.remove(k);
            }
        }
    }
}

/// A line's stamped cost for `q` portions, or `None` for an unstamped line.
pub fn line_cost(line: &Value, q: i64) -> Option<i64> {
    line.get("unit_cost").and_then(Value::as_i64).map(|c| c * q)
}

#[cfg(test)]
mod tests;
