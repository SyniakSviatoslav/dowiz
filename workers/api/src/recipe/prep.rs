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
use serde_json::{json, Value};

use super::{basis_of, is_food, line_with, PER_MASS};

/// Half a millionth: what a half-up rounding of micro units adds.
pub const MICRO_HALF: i64 = MICRO / 2;

/// Levels a hydration follows before it stops (cycles are refused at save;
/// a stored one would still not hang a request).
const DEPTH: usize = prep::DEPTH_MAX + 1;

/// A raw supply's list price in millionths of a minor unit per base unit:
/// `costPerBasis` is per 100 g/ml or per piece.
pub fn price_micro(supply_json: &str) -> Option<i128> {
    let v: Value = serde_json::from_str(supply_json).ok()?;
    if prep::is_prep(supply_json) {
        return None;
    }
    let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
    let per = v.get("costPerBasis").and_then(Value::as_i64)?;
    Some(i128::from(per) * i128::from(MICRO) / i128::from(basis_of(unit)))
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
            Some(line_with(&l.item, l.qty, None, None, &sv))
        })
        .collect();
    let food: Vec<&super::Line> = lines.iter().filter(|l| is_food(&l.kind)).collect();
    let per_basis = |f: fn(&super::Line) -> Option<f64>| -> Option<f64> {
        if lines.len() != card.lines.len() || food.is_empty() || !food.iter().all(|l| f(l).is_some()) {
            return None;
        }
        let sum: f64 = food.iter().filter_map(|l| f(l)).sum();
        Some((sum / card.yield_qty as f64 * basis as f64 * 10.0).round() / 10.0)
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
                let line = line_with(&l.item, l.qty, None, None, &sv);
                json!({ "item": l.item, "qty": l.qty, "name": line.name, "unit": line.unit, "kind": line.kind, "cost": line.cost, "grossG": line.w.gross,
                    "kcal": line.kcal.map(|x| x.round()), "untracked": sv.get("untracked").and_then(Value::as_bool).unwrap_or(false) })
            }
            None => json!({ "item": l.item, "qty": l.qty, "name": l.item, "missing": true }),
        })
        .collect();
    let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let micro = v.get("costMicroPerUnit").and_then(Value::as_i64);
    // Per kg or l for mass, per piece for pieces: what a kitchen compares.
    let per = if unit == "unit" { 1 } else { 10 * PER_MASS };
    v["costPer"] = micro.map(|m| json!((i128::from(m) * i128::from(per) + i128::from(MICRO) / 2) / i128::from(MICRO))).unwrap_or(Value::Null);
    v["costPerQty"] = json!(per);
    v["batchCost"] = micro.map(|m| json!((i128::from(m) * i128::from(card.yield_qty) + i128::from(MICRO) / 2) / i128::from(MICRO))).unwrap_or(Value::Null);
    v["grossG"] = json!(lines.iter().filter_map(|l| l.get("grossG").and_then(Value::as_i64)).sum::<i64>());
    v["lines"] = Value::Array(lines);
    v["yield"] = json!(card.yield_qty);
    v
}

#[cfg(test)]
#[path = "prep/tests.rs"]
mod tests;
