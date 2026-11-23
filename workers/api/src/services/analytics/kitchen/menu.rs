//! PURE. THE MENU-ENGINEERING MATRIX (W-HIST P3): Kasavana and Smith's
//! popularity x profit, on the cost STAMPED when each portion was sold.
//!
//! * POPULAR: a dish's share of the portions sold is at least 70 % of an even
//!   share (1/N over the N dishes sold in the window).
//! * PROFITABLE: its contribution margin per portion is at least the
//!   portion-weighted average over the dishes whose cost is known.
//! * star = both; plowhorse = popular only; puzzle = profitable only; dog =
//!   neither. One action per quadrant; the screen words it from templates in
//!   four languages (`analytics-i18n.js`), never a model.
//!
//! A COST IS NEVER INVENTED. A portion's cost is its stamp (R4); a portion
//! sold before stamps existed takes today's portion cost from its recipe, as
//! `report.rs` does, and the row says which (`costBasis`). A dish with
//! neither -- no recipe, or a supply nothing has priced -- is "cost unknown":
//! it is counted for popularity and is in no quadrant.
//!
//! Integer arithmetic only: every comparison is cross-multiplied in i128.

use super::super::history::pm;
use super::sales::{cost_line, per_micro, portion_cost, Dish, Sales, Supply};
use crate::recipe::Qty;
use dowiz_hub::stock::cost::CostBook;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

/// One sold dish, as the matrix reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuIn {
    pub id: String,
    pub name: String,
    pub sold: i64,
    pub revenue: i64,
    /// Portions carrying a stamp, and what they cost then.
    pub stamped: i64,
    pub stamped_cogs: i64,
    /// One portion's cost today, from the recipe; `None` when not every line has one.
    pub today: Option<i64>,
    /// The recipe line that costs most per portion: (supply id, name, cost).
    pub costliest: Option<(String, String, i64)>,
    /// Of `sold`, till portions counted under this dish at read time (`till.rs`).
    pub attributed: i64,
    pub has_recipe: bool,
    /// What an unknown cost waits on: (supply id, name), see [`missing_of`].
    pub missing: Vec<(String, String)>,
}

/// What a dish's cost waits on: each supply of its recipe (or of the raw
/// items its ПФ expands to) that has no price, or that the catalogue lost --
/// (id, name), sorted. Empty when every line has a cost or there is no recipe.
pub fn missing_of(dish: &Dish, supplies: &HashMap<String, Supply>, book: &CostBook) -> Vec<(String, String)> {
    let priced = |id: &str, qty: Option<Qty>| {
        supplies.get(id).is_some_and(|s| match qty {
            Some(q) => cost_line(s, book, q).is_some(),
            None => per_micro(s, book).is_some(),
        })
    };
    let named = |id: &str| (id.to_string(), supplies.get(id).map_or(id.to_string(), |s| s.name.clone()));
    let mut out: Vec<(String, String)> = if dish.leaves.is_empty() {
        dish.lines.iter().filter(|l| !priced(&l.supply, Some(l.qty))).map(|l| named(&l.supply)).collect()
    } else {
        dish.leaves.iter().filter(|(id, _)| !priced(id, None)).map(|(id, _)| named(id)).collect()
    };
    out.sort();
    out.dedup();
    out
}

fn missing_json(m: &[(String, String)]) -> Value {
    m.iter().map(|(id, name)| json!({ "id": id, "name": name })).collect()
}

/// THE MENU TODAY: every dish of the catalogue, sold or not, one portion at
/// today's recipe against today's price (`price`), sorted by id. A cost that
/// is not known stays null and the row names what it waits on (`missing`);
/// never a zero.
pub fn today(dishes: &HashMap<String, Dish>, supplies: &HashMap<String, Supply>, book: &CostBook, price: &dyn Fn(&str) -> Option<i64>) -> Vec<Value> {
    let mut ids: Vec<&String> = dishes.keys().collect();
    ids.sort();
    ids.into_iter()
        .map(|id| {
            let d = &dishes[id];
            let cost = portion_cost(d, supplies, book);
            let p = price(id);
            let fc = cost.zip(p.filter(|p| *p > 0)).and_then(|(c, p)| pm(c, p));
            let missing = if cost.is_some() { Vec::new() } else { missing_of(d, supplies, book) };
            json!({ "id": id, "price": p, "portionCost": cost, "marginPortion": cost.zip(p).map(|(c, p)| p - c), "foodCostPm": fc,
                    "hasRecipe": !d.lines.is_empty(), "missing": missing_json(&missing) })
        })
        .collect()
}

/// The matrix's input from the kitchen's folds; `attributed` is what
/// `till::attribute` moved onto each dish.
pub fn inputs(sold: &Sales, dishes: &HashMap<String, Dish>, supplies: &HashMap<String, Supply>, book: &CostBook, attributed: &BTreeMap<String, i64>) -> Vec<MenuIn> {
    sold.dishes
        .iter()
        .map(|(id, row)| {
            let dish = dishes.get(id);
            let costliest = dish.and_then(|d| {
                d.lines
                    .iter()
                    .filter_map(|l| supplies.get(&l.supply).and_then(|s| cost_line(s, book, l.qty).map(|c| (s.id.clone(), s.name.clone(), c))))
                    .max_by(|a, b| a.2.cmp(&b.2).then(b.0.cmp(&a.0)))
            });
            MenuIn {
                id: id.clone(),
                name: dish.map_or(id.clone(), |d| d.name.clone()),
                sold: row.sold,
                revenue: row.revenue,
                stamped: row.stamped,
                stamped_cogs: row.stamped_cogs,
                today: dish.and_then(|d| portion_cost(d, supplies, book)),
                costliest,
                attributed: attributed.get(id).copied().unwrap_or(0),
                has_recipe: dish.is_some_and(|d| !d.lines.is_empty()),
                missing: dish.map(|d| missing_of(d, supplies, book)).unwrap_or_default(),
            }
        })
        .collect()
}

/// The cost of the dish's portions and where it came from, or `None`.
pub fn cogs(d: &MenuIn) -> Option<(i64, &'static str)> {
    let rest = d.sold - d.stamped;
    match (rest, d.today) {
        (0, _) => Some((d.stamped_cogs, "stamped")),
        (_, Some(c)) if d.stamped > 0 => Some((d.stamped_cogs + c * rest, "mixed")),
        (_, Some(c)) => Some((c * rest, "today")),
        (_, None) => None,
    }
}

/// The quadrant's name, from the two answers.
pub fn quadrant(popular: bool, profitable: bool) -> &'static str {
    match (popular, profitable) {
        (true, true) => "star",
        (true, false) => "plowhorse",
        (false, true) => "puzzle",
        (false, false) => "dog",
    }
}

fn extend(row: &mut Value, parts: &Value) {
    if let (Some(r), Some(p)) = (row.as_object_mut(), parts.as_object()) {
        r.extend(p.clone());
    }
}

/// The matrix over the dishes sold in the window.
pub fn matrix(rows: &[MenuIn]) -> Value {
    let sold: Vec<&MenuIn> = rows.iter().filter(|d| d.sold > 0).collect();
    let n = sold.len() as i128;
    let total: i128 = sold.iter().map(|d| i128::from(d.sold)).sum();
    // The weighted average margin per portion, over the costed dishes.
    let (mut cm_total, mut cm_sold) = (0i128, 0i128);
    for d in &sold {
        if let Some((c, _)) = cogs(d) {
            cm_total += i128::from(d.revenue - c);
            cm_sold += i128::from(d.sold);
        }
    }
    let avg = (cm_sold > 0).then(|| (cm_total / cm_sold) as i64);
    let mut out: Vec<Value> = sold
        .iter()
        .map(|d| {
            let popular = 10 * i128::from(d.sold) * n >= 7 * total;
            let mix_pm = (total > 0).then(|| (i128::from(d.sold) * 1000 / total) as i64);
            // The basis of each part of the cost, and what an unknown one waits on.
            let rest = d.sold - d.stamped;
            let parts = json!({ "stamped": d.stamped, "stampedCogs": d.stamped_cogs, "todayPortions": rest, "todayCogs": d.today.map(|c| c * rest),
                                "attributed": d.attributed, "hasRecipe": d.has_recipe, "missing": missing_json(&d.missing) });
            let Some((c, basis)) = cogs(d) else {
                let mut row = json!({ "id": d.id, "name": d.name, "sold": d.sold, "revenue": d.revenue, "mixPm": mix_pm,
                               "popular": popular, "costUnknown": true, "quadrant": null, "action": "cost_unknown" });
                extend(&mut row, &parts);
                return row;
            };
            let margin = d.revenue - c;
            let profitable = cm_sold > 0 && i128::from(margin) * cm_sold >= cm_total * i128::from(d.sold);
            let q = quadrant(popular, profitable);
            let per = margin / d.sold;
            // The gap to the average margin per portion, in whole minor
            // units: what a price rise or a cheaper line has to find.
            let raise = (q == "plowhorse" || q == "dog").then(|| avg.map(|a| (a - per).max(0))).flatten();
            let mut row = json!({
                "id": d.id, "name": d.name, "sold": d.sold, "revenue": d.revenue, "cogs": c, "costBasis": basis,
                "margin": margin, "marginPortion": per, "mixPm": mix_pm, "popular": popular, "profitable": profitable,
                "costUnknown": false, "quadrant": q, "action": q, "raiseBy": raise, "foodCostPm": pm(c, d.revenue).filter(|_| d.revenue > 0),
                "costliest": d.costliest.as_ref().map(|(id, name, cost)| json!({ "id": id, "name": name, "cost": cost })),
            });
            extend(&mut row, &parts);
            row
        })
        .collect();
    let place_of = |v: &Value| match v["quadrant"].as_str() {
        Some("star") => 0,
        Some("plowhorse") => 1,
        Some("puzzle") => 2,
        Some("dog") => 3,
        _ => 4,
    };
    out.sort_by(|a, b| place_of(a).cmp(&place_of(b)).then(b["sold"].as_i64().cmp(&a["sold"].as_i64())).then(a["id"].as_str().cmp(&b["id"].as_str())));
    json!({
        "dishes": out,
        "portions": total as i64,
        "count": n as i64,
        // The popularity line, in ‰ of the mix: 70 % of an even share.
        "popularPm": (n > 0).then(|| (700 / n) as i64),
        "averageMarginPortion": avg,
        "unknown": sold.iter().filter(|d| cogs(d).is_none()).count(),
    })
}

#[cfg(test)]
#[path = "menu/tests.rs"]
mod tests;
