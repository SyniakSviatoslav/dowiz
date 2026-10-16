//! Writing a recipe onto a dish, and what follows from it -- ONE path.
//!
//! This was the body of `owner::update_product`'s recipe block. It is here so
//! the bulk import (`services/catalogue/import/bulk.rs`) writes a recipe the
//! way a hand-typed one is written: each line scaled from the supply as it
//! is now (`line_of`), the dish's nutrition, ingredient list, weight and cost
//! summed (`derive`), the stored array lean, `{supply, qty}` (`bom_json`). Two copies of this would drift, and the ledger reads the one
//! nobody tested.

use serde_json::{json, Value};

use super::{bom_json, derive, line_with, weights, BomLineIn, Derived, Line, Qty, QTY_MAX};

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
pub(crate) fn clear_derived(p: &mut Value) {
    for (value, mark) in [("nutrition", "nutritionDerived"), ("weightG", "weightDerived"), ("ingredients", "ingredientsDerived")] {
        if p.get(mark).and_then(Value::as_bool) == Some(true) {
            p[value] = Value::Null;
            p[mark] = Value::Null;
        }
    }
    p["nutritionComplete"] = Value::Null;
}

/// THE REVIEW FLAG (contract §2): set by a supply's deletion, cleared by the
/// next recipe write -- any `bom` the editor sends is the owner's review.
pub(crate) fn clear_review(p: &mut Value) {
    if let Some(m) = p.as_object_mut() {
        m.remove("recipeReview");
    }
}

/// [`set_bom`] for a write that is NOT the owner's review -- a supply's
/// price change re-deriving the dish, a deletion -- so the flag stays.
pub fn set_bom_keeping_review(
    p: &mut Value,
    lines: &[BomLineIn],
    supply: impl Fn(&str) -> Option<String>,
    typed: Typed,
) -> Result<Option<Derived>, String> {
    let review = p.get("recipeReview").cloned();
    let out = set_bom(p, lines, supply, typed);
    if let Some(r) = review {
        p["recipeReview"] = r;
    }
    out
}

/// NO LOSS (operator 2026-10-16): two lines of one supply are ONE line, the
/// quantities summed in milli-units (exact), net/out summed when both lines
/// carry one, else kept from the line that does. Each line is bounded first,
/// then the sum, refused naming the supply -- it was silently DROPPED.
pub(crate) fn merged(lines: &[BomLineIn]) -> Result<Vec<BomLineIn>, String> {
    let add = |a: Option<i64>, b: Option<i64>| match (a, b) {
        (Some(x), Some(y)) => Some(x.saturating_add(y)),
        (x, y) => x.or(y),
    };
    let mut out: Vec<BomLineIn> = Vec::with_capacity(lines.len());
    for l in lines {
        if l.qty <= 0 || l.qty > QTY_MAX {
            return Err(format!("{}: a quantity is more than 0 and at most {QTY_MAX}", l.supply));
        }
        let Some(x) = out.iter_mut().find(|x| x.supply == l.supply) else {
            out.push(l.clone());
            continue;
        };
        let sum = Qty::milli(x.qty.milli.saturating_add(l.qty.milli));
        if sum > QTY_MAX {
            return Err(format!("{}: its lines add up to {sum}, more than the {QTY_MAX} one line takes", l.supply));
        }
        (x.qty, x.net, x.out) = (sum, add(x.net, l.net), add(x.out, l.out));
    }
    Ok(out)
}

/// The marker a refused recipe carries out of a catalogue closure (which can
/// only fail with `Error`), so the route answers it as [`refusal`].
pub(crate) const REFUSED: &str = "recipe refused: ";

/// The lines' own checks, before the catalogue is read: a supply named, each
/// quantity and each supply's sum in bounds (`merged`).
pub(crate) fn precheck(lines: &[BomLineIn]) -> Result<(), String> {
    match lines.iter().find(|l| l.supply.trim().is_empty()) {
        Some(_) => Err("(no supply): a recipe line names its supply".into()),
        None => merged(lines).map(|_| ()),
    }
}

/// A refused recipe as the owner reads it (2026-10-16; it was a bare 500):
/// `400 {"error": "<supply>: <reason>"}`.
pub(crate) fn refusal(why: &str) -> worker::Result<crate::wire::Reply> {
    Ok(crate::wire::Reply::from_json(&json!({ "error": why }))?.with_status(400))
}

/// Set `p`'s recipe to `lines`. `supply` answers a supply's stored JSON by id.
/// An unknown supply refuses the whole write, naming it; two lines of the same
/// supply are merged into one ([`merged`]), never dropped.
/// No lines clears the recipe. Answers what was derived, if anything.
pub fn set_bom(
    p: &mut Value,
    lines: &[BomLineIn],
    supply: impl Fn(&str) -> Option<String>,
    typed: Typed,
) -> Result<Option<Derived>, String> {
    if let Some(l) = lines.iter().find(|l| supply(&l.supply).is_none()) {
        return Err(format!("unknown supply {}", l.supply));
    }
    let supply = super::prep::lookup(&supply);
    let lines = merged(lines)?;
    let mut snap: Vec<Line> = Vec::with_capacity(lines.len());
    for l in &lines {
        let sv: Value = supply(&l.supply).and_then(|sj| serde_json::from_str(&sj).ok()).unwrap_or(json!({}));
        let line = line_with(&l.supply, l.qty, l.net, l.out, &sv);
        weights::check(l.net, l.out, line.w.gross).map_err(|e| format!("{}: {e}", l.supply))?;
        snap.push(line);
    }
    clear_review(p);
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
#[cfg(test)]
mod route_tests;
