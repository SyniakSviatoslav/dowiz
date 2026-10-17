//! A SEMI-FINISHED SUPPLY AS A RECIPE LINE READS IT (SPEC-SEMI-FINISHED §d).
//!
//! `line_with` scales a supply's per-basis numbers to a line; a ПФ record
//! stores none of them -- only its card. This file DERIVES them on read from
//! the card and the raw supplies as they are now, and hands `line_with` a
//! HYDRATED record: `costMicroPerUnit` (exact, millionths of a minor unit per
//! base unit, through `dowiz_hub::prep::cost_micro`), the nutrition per basis
//! (Σ of the card's lines ÷ yield), `nutritionBasis: "cooked"` (a yield IS the
//! cooked weight) and no losses (a ПФ line's out is its qty). A raw supply
//! passes through untouched, so a recipe with no ПФ derives byte for byte
//! what it did before.
//!
//! Nothing here is stored: change the dry rice's price and every ПФ and dish
//! reads the new cost on the next request.

use dowiz_hub::prep::{self, card_of, k_pm, MICRO};
use dowiz_hub::stock::cost::CostBook;
use serde_json::{json, Value};

use super::{basis_of, is_food, line_with, Qty, PER_MASS};

/// Half a millionth: what a half-up rounding of micro units adds.
pub const MICRO_HALF: i64 = MICRO / 2;

/// Levels a hydration follows before it stops (cycles are refused at save;
/// a stored one would still not hang a request).
const DEPTH: usize = prep::DEPTH_MAX + 1;

/// THE PRICE RULE, in one place (2026-10-17): millionths of a minor unit per
/// base unit -- the weighted average purchase cost when a priced delivery
/// set one, else the list price per basis (`list_per_basis` per `basis` base
/// units); `None` with neither, never 0. The console's `priceOf`, the kitchen
/// report's `per_micro` and every hydrated ПФ read this same rule.
pub fn unit_micro(avg: Option<i128>, list_per_basis: Option<i64>, basis: i64) -> Option<i128> {
    match avg {
        Some(a) => Some(a),
        None => Some(i128::from(list_per_basis?) * i128::from(MICRO) / i128::from(basis.max(1))),
    }
}

/// A raw supply's price in millionths of a minor unit per base unit, by
/// [`unit_micro`]: the average [`averaged`] put on the record
/// (`costMicroPerUnit`), else the list price (`costPerBasis` per 100 g/ml or
/// per piece). A ПФ has none of its own: its card prices it.
pub fn price_micro(supply_json: &str) -> Option<i128> {
    let v: Value = serde_json::from_str(supply_json).ok()?;
    if prep::is_prep(supply_json) {
        return None;
    }
    let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
    unit_micro(v.get("costMicroPerUnit").and_then(Value::as_i64).map(i128::from), v.get("costPerBasis").and_then(Value::as_i64), basis_of(unit))
}

/// `supply`, with each RAW record that a priced delivery reached carrying its
/// weighted average as `costMicroPerUnit` -- what [`price_micro`], a recipe
/// line (`recipe::cost_nano`) and so every hydrated ПФ read first. A ПФ and a
/// supply no priced delivery reached pass through untouched, so an empty
/// book reads exactly what `supply` did. For DERIVING only: what a screen
/// lists as the supply's own fields is never read from this.
pub fn averaged<'a>(supply: &'a dyn Fn(&str) -> Option<String>, book: &'a CostBook) -> impl Fn(&str) -> Option<String> + 'a {
    move |id: &str| {
        let j = supply(id)?;
        match book.avg_micro(id).and_then(|a| i64::try_from(a).ok()) {
            Some(a) if !prep::is_prep(&j) => match serde_json::from_str::<Value>(&j) {
                Ok(mut v) if v.is_object() => {
                    v["costMicroPerUnit"] = json!(a);
                    Some(v.to_string())
                }
                _ => Some(j),
            },
            _ => Some(j),
        }
    }
}

/// The record `line_with` reads for `id`: hydrated when it is a ПФ.
pub fn hydrated(id: &str, json: &str, supply: &dyn Fn(&str) -> Option<String>) -> String {
    match hydrate_at(id, json, supply, 0) {
        Some(v) => v.to_string(),
        None => json.to_string(),
    }
}

/// `supply`, answering hydrated ПФ records: what every reader of a recipe
/// passes on instead of the bare catalogue closure.
pub fn lookup<'a>(supply: &'a dyn Fn(&str) -> Option<String>) -> impl Fn(&str) -> Option<String> + 'a {
    move |id: &str| supply(id).map(|j| hydrated(id, &j, supply))
}

fn hydrate_at(id: &str, json: &str, supply: &dyn Fn(&str) -> Option<String>, depth: usize) -> Option<Value> {
    let card = card_of(json)?;
    let mut v: Value = serde_json::from_str(json).ok()?;
    if depth > DEPTH {
        return Some(v);
    }
    let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let basis = basis_of(&unit);
    // Each line of the card, scaled from ITS supply (a ПФ inside is hydrated first).
    let lines: Vec<super::Line> = card
        .lines
        .iter()
        .filter_map(|l| {
            let j = supply(&l.item)?;
            let sv = hydrate_at(&l.item, &j, supply, depth + 1).or_else(|| serde_json::from_str(&j).ok())?;
            Some(line_with(&l.item, Qty::of_micro(l.uq), None, None, &sv))
        })
        .collect();
    let food: Vec<&super::Line> = lines.iter().filter(|l| is_food(&l.kind)).collect();
    let per_basis = |f: fn(&super::Line) -> Option<f64>| -> Option<f64> {
        if lines.len() != card.lines.len() || food.is_empty() || !food.iter().all(|l| f(l).is_some()) {
            return None;
        }
        let sum: f64 = food.iter().filter_map(|l| f(l)).sum();
        // Nutrition is f64 already (kcal); the yield in base units only scales it.
        Some((sum / (card.yield_uq as f64 / MICRO as f64) * basis as f64 * 10.0).round() / 10.0)
    };
    for (key, f) in [("kcalPer100", (|l: &super::Line| l.kcal) as fn(&super::Line) -> Option<f64>), ("proteinPer100", |l| l.protein), ("fatPer100", |l| l.fat), ("carbsPer100", |l| l.carbs)] {
        v[key] = per_basis(f).map(|x| json!(x)).unwrap_or(Value::Null);
    }
    v["nutritionBasis"] = json!("cooked");
    v["kind"] = json!(prep::KIND);
    for gone in ["cleanPm", "cookPm"] {
        v[gone] = Value::Null;
    }
    let micro = prep::cost_micro(&[(id.to_string(), 1)], supply, &|s: &str| supply(s).and_then(|j| price_micro(&j))).ok().flatten();
    v["costMicroPerUnit"] = micro.map(|m| json!(m)).unwrap_or(Value::Null);
    // The list-price form the console draws: minor units per 100 g/ml or per piece.
    v["costPerBasis"] = micro.map(|m| json!((i128::from(m) * i128::from(basis) + i128::from(MICRO) / 2) / i128::from(MICRO))).unwrap_or(Value::Null);
    v["k"] = k_pm(&card, &unit, v.get("weightPerUnit").and_then(Value::as_f64), supply).map(|k| json!(k)).unwrap_or(Value::Null);
    if let Some(m) = v.as_object_mut() {
        m.retain(|_, x| !x.is_null());
    }
    Some(v)
}

/// The console's ПФ: the hydrated record plus each line with its name, unit,
/// kind and cost, the batch's cost, and the cost per kg / l / piece.
pub fn view(id: &str, json: &str, supply: &dyn Fn(&str) -> Option<String>) -> Value {
    let mut v = hydrate_at(id, json, supply, 0).unwrap_or_else(|| serde_json::from_str(json).unwrap_or(json!({ "id": id })));
    let Some(card) = card_of(json) else { return v };
    let look = lookup(supply);
    let lines: Vec<Value> = card
        .lines
        .iter()
        .map(|l| match look(&l.item).and_then(|j| serde_json::from_str::<Value>(&j).ok()) {
            Some(sv) => {
                let line = line_with(&l.item, Qty::of_micro(l.uq), None, None, &sv);
                json!({ "item": l.item, "qty": Qty::of_micro(l.uq).json(), "qtyMilli": Qty::of_micro(l.uq).milli, "name": line.name, "unit": line.unit, "kind": line.kind, "cost": line.cost, "grossG": line.w.gross,
                    "kcal": line.kcal.map(|x| x.round()), "untracked": sv.get("untracked").and_then(Value::as_bool).unwrap_or(false) })
            }
            None => json!({ "item": l.item, "qty": Qty::of_micro(l.uq).json(), "qtyMilli": Qty::of_micro(l.uq).milli, "name": l.item, "missing": true }),
        })
        .collect();
    let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let micro = v.get("costMicroPerUnit").and_then(Value::as_i64);
    // Per kg or l for mass, per piece for pieces: what a kitchen compares.
    let per = if unit == "unit" { 1 } else { 10 * PER_MASS };
    v["costPer"] = micro.map(|m| json!((i128::from(m) * i128::from(per) + i128::from(MICRO) / 2) / i128::from(MICRO))).unwrap_or(Value::Null);
    v["costPerQty"] = json!(per);
    // micro per unit x yield in millionths: rounded once, from 10^12.
    let m2 = i128::from(MICRO) * i128::from(MICRO);
    v["batchCost"] = micro.map(|m| json!((i128::from(m) * i128::from(card.yield_uq) + m2 / 2) / m2)).unwrap_or(Value::Null);
    v["grossG"] = json!(lines.iter().filter_map(|l| l.get("grossG").and_then(Value::as_i64)).sum::<i64>());
    v["lines"] = Value::Array(lines);
    v["yield"] = Qty::of_micro(card.yield_uq).json();
    v["yieldMilli"] = json!(Qty::of_micro(card.yield_uq).milli);
    // The deletion's mark on the ПФ (contract §2) passes through `hydrate_at` as stored.
    v
}

#[cfg(test)]
#[path = "prep/tests.rs"]
mod tests;
