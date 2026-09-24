//! Writing a recipe onto a dish, and what follows from it -- ONE path.
//!
//! This was the body of `owner::update_product`'s recipe block. It is here so
//! the bulk import (`services/catalogue/import/bulk.rs`) writes a recipe the
//! way a hand-typed one is written: each line scaled from the supply as it
//! is now (`line_of`), the dish's nutrition, ingredient list, weight and cost
//! summed (`derive`), the stored array lean, `{supply, qty}` (`bom_json`). Two copies of this would drift, and the ledger reads the one
//! nobody tested.

use serde_json::{json, Value};

use super::{bom_json, derive, line_of, BomLineIn, Derived, Line};

/// What the caller typed in the same request. A value the owner typed wins
/// over the sum and stays marked as theirs; the import types none of them.
#[derive(Debug, Default, Clone, Copy)]
pub struct Typed {
    pub nutrition: bool,
    pub weight: bool,
    /// A non-empty ingredients box. An empty one beside a recipe means "use
    /// the recipe's names" -- the console always sends the box.
    pub ingredients: bool,
}

impl Typed {
    /// What the STORED record says the owner typed: a value marked
    /// `...Derived: false` was the owner's. The bulk import passed
    /// `Typed::default()` and so overwrote hand-typed kcal, weight and
    /// ingredients with the recipe's sums (audit D19); it reads this now.
    pub fn from_record(p: &Value) -> Self {
        let owned = |k: &str| p.get(k).and_then(Value::as_bool) == Some(false);
        Typed { nutrition: owned("nutritionDerived"), weight: owned("weightDerived"), ingredients: owned("ingredientsDerived") }
    }
}

/// Drop what the recipe put there, and only that (audit D40): a value marked
/// derived followed a recipe that is gone; a value the owner typed stays.
fn clear_derived(p: &mut Value) {
    for (value, mark) in [("nutrition", "nutritionDerived"), ("weightG", "weightDerived"), ("ingredients", "ingredientsDerived")] {
        if p.get(mark).and_then(Value::as_bool) == Some(true) {
            p[value] = Value::Null;
            p[mark] = Value::Null;
        }
    }
    p["nutritionComplete"] = Value::Null;
}

/// Set `p`'s recipe to `lines`. `supply` answers a supply's stored JSON by id.
/// An unknown supply refuses the whole write, naming it; a second line for the
/// same supply is dropped (one line per supply, as the old editor enforced).
/// No lines clears the recipe. Answers what was derived, if anything.
pub fn set_bom(
    p: &mut Value,
    lines: &[BomLineIn],
    supply: impl Fn(&str) -> Option<String>,
    typed: Typed,
) -> Result<Option<Derived>, String> {
    let mut snap: Vec<Line> = Vec::with_capacity(lines.len());
    for l in lines {
        let Some(sj) = supply(&l.supply) else {
            return Err(format!("unknown supply {}", l.supply));
        };
        let sv: Value = serde_json::from_str(&sj).unwrap_or(json!({}));
        if snap.iter().any(|x| x.supply == l.supply) {
            continue;
        }
        snap.push(line_of(&l.supply, l.qty, &sv));
    }
    if snap.is_empty() {
        clear_derived(p);
        p["bom"] = Value::Null;
        p["nutritionDerived"] = Value::Null;
        p["cost"] = Value::Null;
        return Ok(None);
    }
    let d = derive(&snap);
    p["bom"] = bom_json(&snap);
    if !typed.nutrition && d.nutrition_complete {
        p["nutrition"] = json!({ "kcal": d.kcal, "protein": d.protein, "fat": d.fat, "carbs": d.carbs, "approx": false });
        p["nutritionDerived"] = json!(true);
    } else if typed.nutrition {
        p["nutritionDerived"] = json!(false);
    }
    if !typed.weight {
        if let Some(w) = d.weight_g {
            p["weightG"] = json!(w);
            p["weightDerived"] = json!(true);
        }
    } else {
        p["weightDerived"] = json!(false);
    }
    if !typed.ingredients && !d.ingredients.is_empty() {
        p["ingredients"] = json!(d.ingredients);
        p["ingredientsDerived"] = json!(true);
    } else if typed.ingredients {
        p["ingredientsDerived"] = json!(false);
    }
    p["cost"] = d.cost.map(|c| json!(c)).unwrap_or(Value::Null);
    p["nutritionComplete"] = json!(d.nutrition_complete);
    Ok(Some(d))
}

#[cfg(test)]
mod tests;
