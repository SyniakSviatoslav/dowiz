//! THE KITCHEN'S NUMBERS, assembled (card I7): by DAY and by DISH, per
//! INGREDIENT, waste by reason, raw -> cooked losses, measured yields,
//! supplier prices, days of cover. Pure: the two folds in, one answer out.
//!
//! Theoretical ("the recipes say") and recorded ("the log says") sit side by
//! side and are never mixed into one number: the difference between them is
//! the point of the screen.

use super::sales::{portion_cost, Dish, Sales, Supply, Window};
use super::shelf::Shelf;
use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::meta::show_day;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Days of cover under which a reorder is suggested, and how many days the
/// suggestion buys.
pub const REORDER_BELOW_DAYS: i64 = 3;
pub const REORDER_FOR_DAYS: i64 = 7;

/// ‰ of `part` in `whole`; `None` when there is no whole.
fn pm(part: i64, whole: i64) -> Option<i64> {
    (whole > 0).then(|| part * 1000 / whole)
}

/// Days of cover and a reorder hint from the average daily use.
pub fn cover(available: i64, used: i64, days: i64, counted: bool) -> (Option<i64>, Option<i64>, Option<i64>) {
    if days <= 0 || used <= 0 {
        return (None, None, None);
    }
    let adu = (used + days - 1) / days; // rounded up: a kitchen runs out, it does not run over
    let cover = counted.then(|| available.max(0) / adu);
    let reorder = cover.filter(|c| *c < REORDER_BELOW_DAYS).map(|_| (adu * REORDER_FOR_DAYS - available.max(0)).max(0));
    (Some(adu), cover, reorder)
}

pub fn report(
    sales: &Sales,
    shelf: &Shelf,
    dishes: &HashMap<String, Dish>,
    supplies: &HashMap<String, Supply>,
    j: &Journal,
    w: &Window,
) -> Value {
    let n = w.starts.len();
    let mut cogs_by_day = vec![0i64; n];
    let mut dish_rows: Vec<Value> = Vec::new();
    let mut uncosted = 0i64;
    for (id, row) in &sales.dishes {
        let dish = dishes.get(id);
        let cost = dish.and_then(|d| portion_cost(d, supplies, &j.book));
        match cost {
            Some(c) => {
                for (d, q) in row.by_day.iter().enumerate() {
                    cogs_by_day[d] += c * q;
                }
            }
            None => uncosted += row.sold,
        }
        let cogs = cost.map(|c| c * row.sold);
        dish_rows.push(json!({
            "id": id, "name": dish.map_or(id.clone(), |d| d.name.clone()), "sold": row.sold, "revenue": row.revenue,
            "portionCost": cost, "cogs": cogs, "margin": cogs.map(|c| row.revenue - c),
            "marginPortion": cogs.filter(|_| row.sold > 0).map(|c| (row.revenue - c) / row.sold),
            "foodCostPm": cogs.and_then(|c| pm(c, row.revenue)), "byDay": row.by_day, "hasRecipe": dish.is_some_and(|d| !d.lines.is_empty()),
        }));
    }
    dish_rows.sort_by(|a, b| b["revenue"].as_i64().cmp(&a["revenue"].as_i64()).then(a["id"].as_str().cmp(&b["id"].as_str())));

    let mut ids: Vec<&String> = sales.uses.keys().chain(shelf.moved.keys()).collect();
    ids.sort();
    ids.dedup();
    let mut ingredients: Vec<Value> = Vec::new();
    for id in ids {
        let s = supplies.get(id);
        let u = sales.uses.get(id).cloned().unwrap_or_default();
        let m = shelf.moved.get(id).cloned().unwrap_or_default();
        let cost = s.and_then(|s| super::sales::cost_of(s, &j.book, u.qty));
        let (adu, days_cover, reorder) = cover(j.ledger.available(id), u.qty, n as i64, j.ledger.is_counted(id));
        ingredients.push(json!({
            "id": id, "name": s.map_or(id.clone(), |s| s.name.clone()), "unit": s.map(|s| s.unit.clone()),
            "used": u.qty, "grossG": u.gross_g, "netG": u.net_g, "outG": u.out_g,
            "cleanLossG": u.gross_g - u.net_g, "cookLossG": u.net_g - u.out_g, "cost": cost,
            "drawn": m.drawn, "drawnValue": m.drawn_value, "wasted": m.wasted, "wastedValue": m.wasted_value,
            "drift": m.drift, "driftValue": m.drift_value, "received": m.received, "receivedValue": m.received_value,
            "prepIn": m.prep_in, "prepOut": m.prep_out,
            "available": j.ledger.available(id), "counted": j.ledger.is_counted(id),
            "adu": adu, "daysCover": days_cover, "reorder": reorder,
            "byDay": if u.by_day.is_empty() { vec![0; n] } else { u.by_day.clone() }, "drawnByDay": if m.by_day_drawn.is_empty() { vec![0; n] } else { m.by_day_drawn.clone() },
        }));
    }
    ingredients.sort_by(|a, b| b["cost"].as_i64().cmp(&a["cost"].as_i64()).then(a["id"].as_str().cmp(&b["id"].as_str())));

    let by_day: Vec<Value> = (0..n)
        .map(|d| {
            let rev = sales.days[d].revenue;
            json!({ "day": show_day(w.days[d]), "orders": sales.days[d].orders, "revenue": rev, "cogs": cogs_by_day[d],
                    "foodCostPm": pm(cogs_by_day[d], rev), "waste": shelf.waste_by_day[d], "received": shelf.received_by_day[d] })
        })
        .collect();
    let revenue: i64 = sales.days.iter().map(|d| d.revenue).sum();
    let cogs: i64 = cogs_by_day.iter().sum();
    let sum = |k: &str| ingredients.iter().map(|i| i[k].as_i64().unwrap_or(0)).sum::<i64>();
    let waste: Vec<Value> = shelf.waste.iter().map(|(r, (rows, v))| json!({ "reason": r, "rows": rows, "value": v })).collect();
    let prices: Vec<Value> = shelf
        .prices
        .iter()
        .map(|(id, pts)| {
            let first = pts.first().and_then(|p| p["perBasis"].as_i64());
            let last = pts.last().and_then(|p| p["perBasis"].as_i64());
            let change = match (first, last) {
                (Some(f), Some(l)) if f > 0 => Some((l - f) * 1000 / f),
                _ => None,
            };
            json!({ "id": id, "name": supplies.get(id).map_or(id.clone(), |s| s.name.clone()),
                    "unit": supplies.get(id).map(|s| s.unit.clone()), "points": pts, "changePm": change })
        })
        .collect();
    json!({
        "from": show_day(w.days[0]), "to": show_day(w.days[n - 1]),
        "days": w.days.iter().map(|d| show_day(*d)).collect::<Vec<_>>(),
        "totals": {
            "orders": sales.days.iter().map(|d| d.orders).sum::<i64>(), "revenue": revenue, "cogs": cogs,
            "foodCostPm": pm(cogs, revenue), "margin": revenue - cogs,
            "wasteValue": shelf.waste.values().map(|x| x.1).sum::<i64>(), "receivedValue": sum("receivedValue"),
            "driftValue": sum("driftValue"), "cleanLossG": sum("cleanLossG"), "cookLossG": sum("cookLossG"),
            "undated": shelf.undated, "unmodelled": sales.unmodelled, "uncosted": uncosted,
        },
        "byDay": by_day, "dishes": dish_rows, "ingredients": ingredients, "waste": waste,
        "yields": shelf.yields, "prices": prices,
    })
}

#[cfg(test)]
#[path = "report/tests.rs"]
mod tests;
