//! What a dish is made of, and what follows from that.
//!
//! The model is the old DeliveryOS one, restored: a SUPPLY is an inventory
//! item with a kind (food, condiment, packaging, utensil), a free-text
//! category, a base unit (g, ml, unit), nutrition per 100 of that unit and a
//! reorder threshold. A dish's RECIPE is a list of `{supply, qty}` lines, one
//! portion's worth, STORED AS JUST THAT (`dowiz_hub::catalog::bom`). A line
//! used to carry a snapshot of its supply's numbers too; that cost ~159 bytes
//! a line and made a 73-recipe menu too large for its catalogue, and nothing
//! read it: the ledger reads `supply` + `qty`, a sale's cost is stamped from
//! the ledger's purchases, and the console re-scaled every line from today's
//! supplies. Names and numbers are now derived on read
//! ([`lines_of_stored`]); a snapshot line stored before still reads.
//! Two things the old service never had and the operator asked for: a COST
//! per supply and a WEIGHT per piece, so a dish's food cost and weight follow
//! from its components too.
//!
//! The hub's stock ledger reads only `bom[].supply` and `bom[].qty`
//! (`dowiz_hub::stock::bom_of`).
//!
//! Taste is authored per dish, never derived: five axes, three levels, an
//! absent axis is "not declared" (the old contract, `attributes.taste`).

use serde::Deserialize;
use dowiz_hub::stock::BomLine;
use serde_json::{json, Value};

pub mod apply;
/// A semi-finished supply, hydrated with the numbers its card derives.
pub mod prep;
/// The five taste axes.
pub mod taste;
pub mod weights;
pub use taste::validate_taste;

/// The kinds of supply. The first two are food and carry nutrition. `resale`
/// is a thing bought and SOLD AS IT IS -- a bottle, a can -- so a dish with no
/// recipe can still take one off the shelf per sale (I0, 2026-09-26).
pub const KINDS: &[&str] = &["food_ingredient", "condiment", "packaging", "utensil", "resale"];
/// The base units a supply is counted in.
pub const UNITS: &[&str] = &["g", "ml", "unit"];
/// Nutrition and cost are declared per this many base units for g and ml,
/// and per ONE for pieces.
pub const PER_MASS: i64 = 100;
pub const PER_PIECE: i64 = 1;
/// The five taste axes, and the levels 1 (low) … 3 (high).
pub const TASTE_AXES: &[&str] = &["spicy", "sweet", "salty", "sour", "richness"];
pub const TASTE_MIN: i64 = 1;
pub const TASTE_MAX: i64 = 3;
/// A recipe line's quantity is bounded: a kitchen does not put a tonne in a roll.
pub const QTY_MAX: i64 = 100_000;

/// A semi-finished product is eaten too: its line counts towards nutrition,
/// weight and the ingredient list like any food.
pub fn is_food(kind: &str) -> bool {
    kind == "food_ingredient" || kind == "condiment" || kind == dowiz_hub::prep::KIND
}

/// How many base units the per-basis numbers describe.
pub fn basis_of(unit: &str) -> i64 {
    if unit == "unit" { PER_PIECE } else { PER_MASS }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BomLineIn {
    pub supply: String,
    /// GROSS, in the supply's base unit: what the ledger reserves.
    pub qty: i64,
    /// Weighed after cleaning, grams. Absent: the supply's `cleanPm`.
    #[serde(default)]
    pub net: Option<i64>,
    /// Weighed on the plate, grams. Absent: net x the supply's `cookPm`.
    #[serde(default)]
    pub out: Option<i64>,
}

/// One line as the dish is derived from it: the ledger's two keys, then its
/// supply's name and numbers scaled to `qty`. Stored as the first two only.
#[derive(Debug, Default, Clone)]
pub struct Line {
    pub supply: String,
    pub qty: i64,
    pub name: String,
    pub unit: String,
    pub kind: String,
    pub kcal: Option<f64>,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbs: Option<f64>,
    /// Minor units, for this line's quantity.
    pub cost: Option<i64>,
    /// Grams ON THE PLATE for this line: its `out` (research R6). It was the
    /// gross until 2026-09-26, which over-stated anything trimmed.
    pub weight_g: Option<f64>,
    /// Gross, net and out in grams, and the losses between them.
    pub w: weights::Weights,
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

/// Scale a supply's per-basis numbers to one line's quantity, at the
/// supply's default losses. The writers take [`line_with`], which carries a
/// line's weighed net and out; this is the defaults-only form the tests read.
#[cfg(test)]
pub fn line_of(supply_id: &str, qty: i64, supply: &Value) -> Line {
    line_with(supply_id, qty, None, None, supply)
}

/// NUTRITION FOLLOWS THE EDIBLE PART (research §3.2). Figures declared on the
/// RAW weight (the default; tables usually are) scale by the gross times the
/// edible share -- calories are in the fillet, not the bones -- and cooking's
/// loss is mostly water, so it does not change the portion's total. Figures
/// declared `cooked` scale by the out, per 100 g.
fn nutrition_scale(unit: &str, qty: i64, supply: &Value, w: &weights::Weights) -> f64 {
    let cooked = supply.get("nutritionBasis").and_then(Value::as_str) == Some("cooked");
    match (cooked && unit != "unit", w.out) {
        (true, Some(o)) => o as f64 / PER_MASS as f64,
        _ => qty as f64 / basis_of(unit) as f64 * w.edible(),
    }
}

/// [`line_of`] with the line's own weighed net and out, grams.
pub fn line_with(supply_id: &str, qty: i64, net: Option<i64>, out: Option<i64>, supply: &Value) -> Line {
    let unit = supply.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let kind = supply.get("kind").and_then(Value::as_str).unwrap_or("food_ingredient").to_string();
    let ratio = qty as f64 / basis_of(&unit) as f64;
    let food = is_food(&kind);
    let w = weights::weights(&unit, qty, supply, net, out);
    let n = nutrition_scale(&unit, qty, supply, &w);
    let scaled = |k: &str| if food { num(supply, k).map(|x| x * n) } else { None };
    // A hydrated semi-finished record carries its cost exactly, in millionths
    // per base unit (`recipe::prep`); a raw supply its list price per basis.
    let cost = match supply.get("costMicroPerUnit").and_then(Value::as_i64) {
        Some(m) => Some(((i128::from(m) * i128::from(qty) + i128::from(prep::MICRO_HALF)) / i128::from(dowiz_hub::prep::MICRO)) as i64),
        None => num(supply, "costPerBasis").map(|c| (c * ratio).round() as i64),
    };
    let weight_g = w.out.map(|o| o as f64);
    Line {
        supply: supply_id.to_string(),
        qty,
        name: supply.get("name").and_then(Value::as_str).unwrap_or(supply_id).to_string(),
        unit,
        kind,
        kcal: scaled("kcalPer100"),
        protein: scaled("proteinPer100"),
        fat: scaled("fatPer100"),
        carbs: scaled("carbsPer100"),
        cost,
        weight_g,
        w,
    }
}

/// What a recipe adds up to, per serving.
#[derive(Debug, Default, PartialEq)]
pub struct Derived {
    pub kcal: i64,
    pub protein: i64,
    pub fat: i64,
    pub carbs: i64,
    /// Every FOOD line carries a kcal figure. Packaging never does and is not
    /// counted against completeness. (The old service's rule, per its README:
    /// only a missing kcal makes the sum "incomplete".)
    pub nutrition_complete: bool,
    /// Every line's cost is known.
    pub cost: Option<i64>,
    /// Every food line's weight is known (packaging is not eaten).
    pub weight_g: Option<i64>,
    /// The food lines' names, for the customer's ingredient list.
    pub ingredients: Vec<String>,
}

pub fn derive(lines: &[Line]) -> Derived {
    let food: Vec<&Line> = lines.iter().filter(|l| is_food(&l.kind)).collect();
    let sum = |f: fn(&Line) -> Option<f64>| food.iter().filter_map(|l| f(l)).sum::<f64>().round() as i64;
    let nutrition_complete = !food.is_empty() && food.iter().all(|l| l.kcal.is_some());
    let cost = if !lines.is_empty() && lines.iter().all(|l| l.cost.is_some()) {
        Some(lines.iter().filter_map(|l| l.cost).sum())
    } else {
        None
    };
    let weight_g = if !food.is_empty() && food.iter().all(|l| l.weight_g.is_some()) {
        Some(food.iter().filter_map(|l| l.weight_g).sum::<f64>().round() as i64)
    } else {
        None
    };
    Derived {
        kcal: sum(|l| l.kcal),
        protein: sum(|l| l.protein),
        fat: sum(|l| l.fat),
        carbs: sum(|l| l.carbs),
        nutrition_complete,
        cost,
        weight_g,
        ingredients: food.iter().map(|l| l.name.clone()).collect(),
    }
}

/// The stored `bom` array: `{supply, qty}` per line -- plus `net`/`out` only
/// where the owner weighed them -- written by the hub's writer so the
/// ledger's reader and it are one pair.
pub fn bom_json(lines: &[Line]) -> Value {
    let lean: Vec<(BomLine, Option<i64>, Option<i64>)> = lines
        .iter()
        .map(|l| (BomLine::whole(l.supply.clone(), l.qty), l.w.net_set, l.w.out_set))
        .collect();
    serde_json::from_str(&dowiz_hub::catalog::bom::to_json_weighed(&lean)).expect("the hub's bom writer writes JSON")
}

/// The lines as the OWNER reads them -- the console's dish sheet and the
/// import preview: each with its supply's name and numbers. Never stored.
pub fn bom_view(lines: &[Line]) -> Value {
    json!(lines
        .iter()
        .map(|l| json!({
            "supply": l.supply, "qty": l.qty, "name": l.name, "unit": l.unit, "kind": l.kind,
            "kcal": l.kcal.map(|x| x.round()), "protein": l.protein.map(|x| (x * 10.0).round() / 10.0),
            "fat": l.fat.map(|x| (x * 10.0).round() / 10.0), "carbs": l.carbs.map(|x| (x * 10.0).round() / 10.0),
            "cost": l.cost, "weightG": l.weight_g.map(|x| x.round()),
            "grossG": l.w.gross, "netG": l.w.net, "outG": l.w.out, "cleanPm": l.w.clean_pm, "cookPm": l.w.cook_pm,
            "net": l.w.net_set, "out": l.w.out_set, "lossPm": l.w.loss_pm(),
        }))
        .collect::<Vec<_>>())
}

/// A STORED `bom` as lines, each scaled from its supply AS IT IS NOW.
///
/// Reads both forms: the lean `{supply, qty}` and the snapshot lines stored
/// before 2026-09-24, which stay live on a venue until the dish is saved
/// again. A line whose supply the catalogue no longer holds keeps what its
/// own snapshot says, else just its id. A line without a supply id or an
/// integer qty is not a line (the ledger skips it too).
pub fn lines_of_stored(bom: &Value, supply: impl Fn(&str) -> Option<String>) -> Vec<Line> {
    let supply = prep::lookup(&supply);
    let mut out = Vec::new();
    for l in bom.as_array().map(Vec::as_slice).unwrap_or_default() {
        let (Some(id), Some(qty)) = (l.get("supply").and_then(Value::as_str), l.get("qty").and_then(Value::as_i64)) else {
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

/// What a line's own snapshot says, for a supply that is gone.
fn snapshot_of(id: &str, qty: i64, l: &Value) -> Line {
    let text = |k: &str, d: &str| l.get(k).and_then(Value::as_str).unwrap_or(d).to_string();
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
        cost: l.get("cost").and_then(Value::as_i64),
        weight_g: num(l, "weightG"),
        w: weights::Weights { clean_pm: weights::PM, cook_pm: weights::PM, ..weights::Weights::default() },
    }
}

/// A stored dish as the owner reads it: its `bom`, if it has one, in the
/// display form ([`bom_view`] of [`lines_of_stored`]) -- and its cost, and
/// the nutrition and weight the recipe derived, REFRESHED from today's
/// supplies (SPEC-SEMI-FINISHED §d: a raw price or a card changed since the
/// dish was saved shows on the next read, nothing re-saved). What the owner
/// typed (`…Derived: false`) is left alone.
pub fn hydrate(p: &mut Value, supply: impl Fn(&str) -> Option<String>) {
    if !p.get("bom").is_some_and(Value::is_array) {
        return;
    }
    let lines = lines_of_stored(&p["bom"], supply);
    let d = derive(&lines);
    p["cost"] = d.cost.map(|c| json!(c)).unwrap_or(Value::Null);
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

#[cfg(test)]
mod tests;
