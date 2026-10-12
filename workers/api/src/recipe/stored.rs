//! THE STORED `bom` -- written lean, read back with today's supplies, and the
//! dish as the owner sees it (split out of `recipe.rs` by W-QTY).
//!
//! A whole line is `{"supply","qty"}`, byte for byte what every dish stored
//! before fractions existed; a fractional one is `{"supply","uq"}` in
//! MILLIONTHS of the base unit -- the form `dowiz_hub::stock::bom_of` reads
//! exactly and `stock::draws_for` books with a carried remainder. A line
//! never carries both. `net`/`out` (weighed grams) follow either.

use dowiz_hub::minijson::esc;
use serde_json::{json, Value};

use super::{derive, line_with, weights, Line, Qty, KINDS};

/// The stored `bom` array. `stock::bom_of` reads it back line for line.
pub fn bom_json(lines: &[Line]) -> Value {
    let parts: Vec<String> = lines
        .iter()
        .map(|l| {
            let mut s = match l.qty.as_whole() {
                Some(q) => format!(r#"{{"supply":"{}","qty":{q}"#, esc(&l.supply)),
                None => format!(r#"{{"supply":"{}","uq":{}"#, esc(&l.supply), l.qty.uq()),
            };
            if let Some(n) = l.w.net_set {
                s.push_str(&format!(r#","net":{n}"#));
            }
            if let Some(o) = l.w.out_set {
                s.push_str(&format!(r#","out":{o}"#));
            }
            s.push('}');
            s
        })
        .collect();
    serde_json::from_str(&format!("[{}]", parts.join(","))).expect("a bom line is JSON")
}

/// The lines as the OWNER reads them -- the console's dish sheet and the
/// import preview: each with its supply's name and numbers. Never stored.
/// `qty` is the base-unit number (`40`, `0.5`); `qtyMilli` and `uq` are the
/// exact integers (contract §1).
pub fn bom_view(lines: &[Line]) -> Value {
    json!(lines
        .iter()
        .map(|l| json!({
            "supply": l.supply, "qty": l.qty.json(), "qtyMilli": l.qty.milli, "uq": l.qty.uq(),
            "name": l.name, "unit": l.unit, "kind": l.kind,
            "kcal": l.kcal.map(|x| x.round()), "protein": l.protein.map(|x| (x * 10.0).round() / 10.0),
            "fat": l.fat.map(|x| (x * 10.0).round() / 10.0), "carbs": l.carbs.map(|x| (x * 10.0).round() / 10.0),
            "cost": l.cost, "costPerBasis": l.cost_per_basis, "weightG": l.weight_g.map(|x| x.round()),
            "grossG": l.w.gross, "netG": l.w.net, "outG": l.w.out, "cleanPm": l.w.clean_pm, "cookPm": l.w.cook_pm,
            "net": l.w.net_set, "out": l.w.out_set, "lossPm": l.w.loss_pm(),
        }))
        .collect::<Vec<_>>())
}

/// A STORED `bom` as lines, each scaled from its supply AS IT IS NOW.
///
/// Reads three forms: the lean `{supply, qty}`, the exact `{supply, uq}`,
/// and the snapshot lines stored before 2026-09-24, which stay live on a
/// venue until the dish is saved again. A line whose supply the catalogue no
/// longer holds keeps what its own snapshot says, else just its id. A line
/// without a supply id or a quantity is not a line (the ledger skips it too).
pub fn lines_of_stored(bom: &Value, supply: impl Fn(&str) -> Option<String>) -> Vec<Line> {
    let supply = super::prep::lookup(&supply);
    let mut out = Vec::new();
    for l in bom.as_array().map(Vec::as_slice).unwrap_or_default() {
        let (Some(id), Some(qty)) = (l.get("supply").and_then(Value::as_str), Qty::from_stored(l)) else {
            continue;
        };
        let weighed = |k: &str| l.get(k).and_then(Value::as_i64);
        out.push(match supply(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) {
            Some(sv) => line_with(id, qty, weighed("net"), weighed("out"), &sv),
            None => snapshot_of(id, qty, l),
        });
    }
    out
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

/// What a line's own snapshot says, for a supply that is gone.
fn snapshot_of(id: &str, qty: Qty, l: &Value) -> Line {
    let text = |k: &str, d: &str| l.get(k).and_then(Value::as_str).unwrap_or(d).to_string();
    let cost = l.get("cost").and_then(Value::as_i64);
    Line {
        supply: id.to_string(),
        qty,
        name: text("name", id),
        unit: text("unit", "g"),
        kind: text("kind", KINDS[0]),
        kcal: num(l, "kcal"),
        protein: num(l, "protein"),
        fat: num(l, "fat"),
        carbs: num(l, "carbs"),
        cost,
        cost_nano: cost.map(|c| i128::from(c) * 1_000_000_000),
        cost_per_basis: None,
        weight_g: num(l, "weightG"),
        w: weights::Weights { clean_pm: weights::PM, cook_pm: weights::PM, ..weights::Weights::default() },
    }
}

/// Food cost per mille of the price: `round_half_up(cost x 1000 / price)`;
/// `None` without a cost or a price. PURE (contract §3).
pub fn food_cost_pm(cost: Option<i64>, price: Option<i64>) -> Option<i64> {
    let (c, p) = (cost?, price.filter(|p| *p > 0)?);
    Some((i128::from(c) * 1000 + i128::from(p) / 2).div_euclid(i128::from(p)) as i64)
}

/// A stored dish as the owner reads it: its `bom`, if it has one, in the
/// display form ([`bom_view`] of [`lines_of_stored`]) -- and its cost, the
/// food cost per mille of its price, and the nutrition and weight the recipe
/// derived, REFRESHED from today's supplies (SPEC-SEMI-FINISHED §d: a raw
/// price or a card changed since the dish was saved shows on the next read,
/// nothing re-saved). What the owner typed (`…Derived: false`) is left alone.
/// `recipeReview`, when stored, passes through as it is (contract §2).
pub fn hydrate(p: &mut Value, supply: impl Fn(&str) -> Option<String>) {
    if !p.get("bom").is_some_and(Value::is_array) {
        return;
    }
    let lines = lines_of_stored(&p["bom"], supply);
    let d = derive(&lines);
    p["cost"] = d.cost.map(|c| json!(c)).unwrap_or(Value::Null);
    if let Some(pm) = food_cost_pm(d.cost, p.get("price").and_then(Value::as_i64)) {
        p["foodCostPm"] = json!(pm);
    }
    if p.get("nutritionDerived").and_then(Value::as_bool) == Some(true) && d.nutrition_complete {
        p["nutrition"] = json!({ "kcal": d.kcal, "protein": d.protein, "fat": d.fat, "carbs": d.carbs, "approx": false });
    }
    if p.get("weightDerived").and_then(Value::as_bool) == Some(true) {
        if let Some(w) = d.weight_g {
            p["weightG"] = json!(w);
        }
    }
    p["bom"] = bom_view(&lines);
}
