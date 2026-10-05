//! ORDERS LOST TO STOCK-OUTS (A13, W-LOST): the `lost` block of the
//! kitchen's numbers. Per dish per venue day: how many baskets the shelf
//! refused, the portions in them, and the revenue they were worth in integer
//! lek (the dish's unit price at the refusal, options included, x portions).
//! The key is `revenue` on purpose: a kitchen token's answer drops every
//! `revenue` key (`access::numbers_for_kitchen`), so staff see counts only.
//!
//! PURE: the stock log's `refused` notes (`dowiz_hub::stock::refused`) and
//! the window in; the block out. A LOWER BOUND, said in the block itself
//! (`rateLimitedMinutes`): one record per supply per ten minutes, so a guest
//! who tried five times is one lost sale.

use super::sales::{Dish, Window};
use dowiz_hub::stock::meta::show_day;
use dowiz_hub::stock::refused::{LostRow, WINDOW_MS};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
struct Acc {
    rows: i64,
    portions: i64,
    value: i64,
    by_day: Vec<(i64, i64)>,
}

/// The `lost` block over `rows` (oldest first, any window).
pub fn report(rows: &[LostRow], dishes: &HashMap<String, Dish>, w: &Window) -> Value {
    let n = w.starts.len();
    let mut per: BTreeMap<String, Acc> = BTreeMap::new();
    let mut days = vec![(0i64, 0i64, 0i64); n];
    for r in rows {
        let Some(d) = w.bucket(r.at) else { continue };
        let value = r.sale.price.saturating_mul(r.sale.qty);
        let a = per.entry(r.sale.dish.clone()).or_default();
        if a.by_day.is_empty() {
            a.by_day = vec![(0, 0); n];
        }
        a.rows += 1;
        a.portions += r.sale.qty;
        a.value = a.value.saturating_add(value);
        a.by_day[d].0 += 1;
        a.by_day[d].1 = a.by_day[d].1.saturating_add(value);
        days[d].0 += 1;
        days[d].1 += r.sale.qty;
        days[d].2 = days[d].2.saturating_add(value);
    }
    let mut list: Vec<Value> = per
        .into_iter()
        .map(|(id, a)| {
            json!({
                "id": id, "name": dishes.get(&id).map_or(id.clone(), |d| d.name.clone()),
                "rows": a.rows, "portions": a.portions, "revenue": a.value,
                "byDay": a.by_day.iter().map(|(r, v)| json!({ "rows": r, "revenue": v })).collect::<Vec<_>>(),
            })
        })
        .collect();
    list.sort_by(|a, b| b["revenue"].as_i64().cmp(&a["revenue"].as_i64()).then(a["id"].as_str().cmp(&b["id"].as_str())));
    json!({
        "contract": "stock.refused.v1",
        "rows": days.iter().map(|d| d.0).sum::<i64>(),
        "portions": days.iter().map(|d| d.1).sum::<i64>(),
        "revenue": days.iter().map(|d| d.2).sum::<i64>(),
        "rateLimitedMinutes": WINDOW_MS / 60_000,
        "dishes": list,
        "byDay": (0..n).map(|d| json!({ "day": show_day(w.days[d]), "rows": days[d].0, "portions": days[d].1, "revenue": days[d].2 })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
#[path = "lost/tests.rs"]
mod tests;
