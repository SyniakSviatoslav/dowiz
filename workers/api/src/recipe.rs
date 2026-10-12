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
use serde_json::{json, Value};

pub mod apply;
/// A semi-finished supply, hydrated with the numbers its card derives.
pub mod prep;
/// A quantity read from its text into milli-units (W-QTY).
pub mod qty;
/// The stored `bom` -- written, read back and hydrated for the owner.
pub mod stored;
/// The five taste axes.
pub mod taste;
pub mod weights;
pub use qty::Qty;
pub use stored::{bom_json, bom_view, hydrate, lines_of_stored};
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

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BomLineIn {
    pub supply: String,
    /// GROSS, in the supply's base unit: what the ledger reserves. Read from
    /// its text -- `40`, `40.5`, `"40,5"` -- into milli-units (`qty.rs`).
    pub qty: Qty,
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
    /// Milli-units of the supply's base unit; a whole line is `n x 1000`.
    pub qty: Qty,
    pub name: String,
    pub unit: String,
    pub kind: String,
    pub kcal: Option<f64>,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbs: Option<f64>,
    /// Minor units, for this line's quantity, rounded once for display.
    pub cost: Option<i64>,
    /// The same, EXACT: nano-minor units (minor x 10^9), what the dish sums
    /// before rounding once (contract §3's rounding rule).
    pub cost_nano: Option<i128>,
    /// The supply's list price per basis as it is now (a ПФ: derived).
    pub cost_per_basis: Option<i64>,
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
    line_with(supply_id, Qty::whole(qty), None, None, supply)
}

/// NUTRITION FOLLOWS THE EDIBLE PART (research §3.2). Figures declared on the
/// RAW weight (the default; tables usually are) scale by the gross times the
/// edible share -- calories are in the fillet, not the bones -- and cooking's
/// loss is mostly water, so it does not change the portion's total. Figures
/// declared `cooked` scale by the out, per 100 g.
fn nutrition_scale(unit: &str, qty: Qty, supply: &Value, w: &weights::Weights) -> f64 {
    let cooked = supply.get("nutritionBasis").and_then(Value::as_str) == Some("cooked");
    match (cooked && unit != "unit", w.out) {
        (true, Some(o)) => o as f64 / PER_MASS as f64,
        _ => qty.units_f64() / basis_of(unit) as f64 * w.edible(),
    }
}

/// Nano-minor units (minor x 10^9) per base unit, times `qty`: EXACT in i128.
/// A ПФ carries `costMicroPerUnit` (millionths per base unit, hub-rounded
/// once); a raw supply its list price per basis (100 g/ml or 1 piece), read
/// as an integer -- an old record that stored it as `12.0` still reads.
fn cost_nano(qty: Qty, supply: &Value) -> Option<i128> {
    if let Some(m) = supply.get("costMicroPerUnit").and_then(Value::as_i64) {
        return Some(i128::from(m) * i128::from(qty.milli));
    }
    let per_basis = supply.get("costPerBasis").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|x| x.round() as i64)))?;
    let unit = supply.get("unit").and_then(Value::as_str).unwrap_or("g");
    // per_basis x 10^9 x (milli / 1000) / basis = per_basis x milli x 10^6 / basis, exact for 1 and 100.
    Some(i128::from(per_basis) * i128::from(qty.milli) * 1_000_000 / i128::from(basis_of(unit)))
}

/// Nano-minor units to minor units, half up, once.
pub fn nano_to_minor(nano: i128) -> i64 {
    ((nano + NANO / 2).div_euclid(NANO)) as i64
}
const NANO: i128 = 1_000_000_000;

/// [`line_of`] with the line's own weighed net and out, grams.
pub fn line_with(supply_id: &str, qty: impl Into<Qty>, net: Option<i64>, out: Option<i64>, supply: &Value) -> Line {
    let qty = qty.into();
    let unit = supply.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let kind = supply.get("kind").and_then(Value::as_str).unwrap_or("food_ingredient").to_string();
    let food = is_food(&kind);
    let w = weights::weights(&unit, qty, supply, net, out);
    let n = nutrition_scale(&unit, qty, supply, &w);
    let scaled = |k: &str| if food { num(supply, k).map(|x| x * n) } else { None };
    // Money is exact until the one rounding at the end (contract §3).
    let nano = cost_nano(qty, supply);
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
        cost: nano.map(nano_to_minor),
        cost_nano: nano,
        cost_per_basis: supply.get("costPerBasis").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|x| x.round() as i64))),
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
    // Σ exact, rounded ONCE: the dish's cost is the truth, a line's is display.
    let cost = if !lines.is_empty() && lines.iter().all(|l| l.cost_nano.is_some()) {
        Some(nano_to_minor(lines.iter().filter_map(|l| l.cost_nano).sum()))
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

#[cfg(test)]
mod tests;
