//! What a dish is made of, and what follows from that.
//!
//! The model is the old DeliveryOS one, restored: a SUPPLY is an inventory
//! item with a kind (food, condiment, packaging, utensil), a free-text
//! category, a base unit (g, ml, unit), nutrition per 100 of that unit and a
//! reorder threshold. A dish's RECIPE is a list of `{supply, qty}` lines, one
//! portion's worth, each carrying a snapshot of the supply's numbers scaled to
//! that quantity, so the dish reads the same even after the supply changes.
//! Two things the old service never had and the operator asked for: a COST
//! per supply and a WEIGHT per piece, so a dish's food cost and weight follow
//! from its components too.
//!
//! The hub's stock ledger reads only `bom[].supply` and `bom[].qty`
//! (`dowiz_hub::stock::bom_of`); every other key on a line is ours.
//!
//! Taste is authored per dish, never derived: five axes, three levels, an
//! absent axis is "not declared" (the old contract, `attributes.taste`).

use serde::Deserialize;
use serde_json::{json, Value};

pub mod apply;

/// The four kinds of supply. The first two are food and carry nutrition.
pub const KINDS: &[&str] = &["food_ingredient", "condiment", "packaging", "utensil"];
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
/// A millilitre of kitchen liquid weighs about a gram; close enough for a
/// portion's weight, and the venue may override the dish's weight by hand.
const ML_TO_G: f64 = 1.0;

pub fn is_food(kind: &str) -> bool {
    kind == "food_ingredient" || kind == "condiment"
}

/// How many base units the per-basis numbers describe.
pub fn basis_of(unit: &str) -> i64 {
    if unit == "unit" { PER_PIECE } else { PER_MASS }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BomLineIn {
    pub supply: String,
    pub qty: i64,
}

/// One line as stored: the ledger's two keys, then the snapshot.
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
    /// Grams, for this line's quantity.
    pub weight_g: Option<f64>,
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

/// Scale a supply's per-basis numbers to one line's quantity.
pub fn line_of(supply_id: &str, qty: i64, supply: &Value) -> Line {
    let unit = supply.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
    let kind = supply.get("kind").and_then(Value::as_str).unwrap_or("food_ingredient").to_string();
    let ratio = qty as f64 / basis_of(&unit) as f64;
    let food = is_food(&kind);
    let scaled = |k: &str| if food { num(supply, k).map(|x| x * ratio) } else { None };
    let cost = num(supply, "costPerBasis").map(|c| (c * ratio).round() as i64);
    let weight_g = match unit.as_str() {
        "g" => Some(qty as f64),
        "ml" => Some(qty as f64 * ML_TO_G),
        _ => num(supply, "weightPerUnit").map(|w| w * qty as f64),
    };
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

/// The stored `bom` array: the ledger's keys first, then the snapshot.
pub fn bom_json(lines: &[Line]) -> Value {
    json!(lines
        .iter()
        .map(|l| json!({
            "supply": l.supply, "qty": l.qty, "name": l.name, "unit": l.unit, "kind": l.kind,
            "kcal": l.kcal.map(|x| x.round()), "protein": l.protein.map(|x| (x * 10.0).round() / 10.0),
            "fat": l.fat.map(|x| (x * 10.0).round() / 10.0), "carbs": l.carbs.map(|x| (x * 10.0).round() / 10.0),
            "cost": l.cost, "weightG": l.weight_g.map(|x| x.round()),
        }))
        .collect::<Vec<_>>())
}

/// A taste map is five known axes at levels 1…3; anything else is refused.
pub fn validate_taste(m: &serde_json::Map<String, Value>) -> Result<serde_json::Map<String, Value>, String> {
    let mut out = serde_json::Map::new();
    for (k, v) in m {
        if !TASTE_AXES.contains(&k.as_str()) {
            return Err(format!("unknown taste axis {k:?}"));
        }
        match v.as_i64() {
            Some(n) if (TASTE_MIN..=TASTE_MAX).contains(&n) => {
                out.insert(k.clone(), json!(n));
            }
            Some(0) | None if v.is_null() || v.as_i64() == Some(0) => {} // 0 or null = not declared
            _ => return Err(format!("taste {k} is 1 to 3")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn salmon() -> Value {
        json!({ "name": "Salmon", "unit": "g", "kind": "food_ingredient", "kcalPer100": 208, "proteinPer100": 20.4, "fatPer100": 13.4, "carbsPer100": 0, "costPerBasis": 1800 })
    }
    fn box_() -> Value {
        json!({ "name": "Box", "unit": "unit", "kind": "packaging", "costPerBasis": 40, "weightPerUnit": 12 })
    }

    #[test]
    fn a_line_scales_per_hundred_and_per_piece() {
        let s = line_of("salmon", 40, &salmon());
        assert_eq!(s.kcal.map(|x| x.round()), Some(83.0));
        assert_eq!(s.cost, Some(720));
        assert_eq!(s.weight_g, Some(40.0));
        let b = line_of("box", 2, &box_());
        assert_eq!(b.kcal, None);
        assert_eq!(b.cost, Some(80));
        assert_eq!(b.weight_g, Some(24.0));
    }

    #[test]
    fn the_dish_sums_food_only_and_costs_everything() {
        let lines = vec![line_of("salmon", 40, &salmon()), line_of("box", 1, &box_())];
        let d = derive(&lines);
        assert_eq!(d.kcal, 83);
        assert_eq!(d.protein, 8);
        assert!(d.nutrition_complete);
        assert_eq!(d.cost, Some(760));
        assert_eq!(d.weight_g, Some(40));
        assert_eq!(d.ingredients, vec!["Salmon".to_string()]);
    }

    #[test]
    fn a_food_line_without_kcal_makes_the_sum_incomplete() {
        let rice = json!({ "name": "Rice", "unit": "g", "kind": "food_ingredient" });
        let d = derive(&[line_of("salmon", 40, &salmon()), line_of("rice", 90, &rice)]);
        assert!(!d.nutrition_complete);
        assert_eq!(d.cost, None);
        assert_eq!(d.weight_g, Some(130));
    }

    #[test]
    fn bom_json_keeps_the_ledgers_keys_first() {
        let v = bom_json(&[line_of("salmon", 40, &salmon())]);
        let lines = dowiz_hub::stock::bom_of(&json!({ "bom": v }).to_string());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].supply, "salmon");
        assert_eq!(lines[0].qty, 40);
    }

    #[test]
    fn taste_is_five_axes_at_three_levels() {
        let ok = validate_taste(&serde_json::from_value(json!({ "spicy": 3, "sweet": 0 })).unwrap()).unwrap();
        assert_eq!(ok.len(), 1);
        assert!(validate_taste(&serde_json::from_value(json!({ "umami": 2 })).unwrap()).is_err());
        assert!(validate_taste(&serde_json::from_value(json!({ "spicy": 5 })).unwrap()).is_err());
    }
}
