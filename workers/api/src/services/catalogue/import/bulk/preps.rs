//! THE RECIPES IMPORT CREATES REAL SEMI-FINISHED PRODUCTS (lane W-PF2 R1).
//!
//! The hub's reader answers them in `RecipeDraft::preps`, children first
//! (`dowiz_hub::import::recipes::DraftPrep`); each goes through the console's
//! OWN writer for a card (`operations::preps::{check, save}`: unknown item,
//! cycle, depth, bounds, and every dish it reaches re-derived) BEFORE the
//! dishes whose lines name it are written. A dish then names the ПФ, so the
//! kitchen edits the sauce once and every dish follows.
//!
//! THE PREVIEW SHOWS THEM: which ПФ the file creates and which it updates,
//! each with its lines, yield and K, and every dish row is judged against a
//! COPY of the catalogue that already holds them -- so the dry run's "after"
//! is what Apply writes, not "unknown supply".

use dowiz_hub::catalog::Catalog;
use dowiz_hub::import::recipes::{DraftPrep, DraftSupply, RecipeDraft};
use serde_json::{json, Value};

use crate::services::operations::preps::{check, save, LineIn, PrepIn};

/// A drafted ПФ as the console's own body.
pub(crate) fn prep_in(p: &DraftPrep) -> PrepIn {
    PrepIn {
        id: p.id.clone(),
        name: Some(p.name.clone()),
        unit: Some(p.unit.to_string()),
        lines: p.lines.iter().map(|(item, qty)| LineIn { item: item.clone(), qty: *qty }).collect(),
        yield_qty: p.yield_qty,
        ..PrepIn::default()
    }
}

/// Write every drafted ПФ, children first. Answers how many. PURE on `cat`;
/// a refused card refuses the whole request (the object turn is dropped).
pub(crate) fn stage(cat: &mut Catalog, draft: &RecipeDraft) -> Result<usize, String> {
    for p in &draft.preps {
        let body = prep_in(p);
        let (id, card) = check(&body).map_err(|e| format!("{}: {e}", p.name))?;
        save(cat, &id, &body, &card).map_err(|e| format!("{}: {e}", p.name))?;
    }
    Ok(draft.preps.len())
}

/// A copy of `cat` holding the drafted ПФ, for the preview's dish rows. A
/// copy that cannot be made or staged answers the catalogue as it is (the
/// rows then say what Apply would refuse).
pub(crate) fn staged(cat: &Catalog, draft: &RecipeDraft) -> Option<Catalog> {
    if draft.preps.is_empty() {
        return None;
    }
    let mut trial = Catalog::create().ok()?;
    // What a card's save and a dish's recipe read: the venue, its supplies, its dishes.
    if let Some(l) = cat.location() {
        trial.set_location(&l);
    }
    for (id, j) in cat.supplies() {
        trial.set_supply(&id, &j);
    }
    for (id, j) in cat.products() {
        trial.set_product(&id, &j);
    }
    stage(&mut trial, draft).ok()?;
    Some(trial)
}

/// The preview's ПФ rows: new or updated, the lines by name, yield and K.
pub(crate) fn rows(cat: &Catalog, draft: &RecipeDraft) -> Vec<Value> {
    let trial = staged(cat, draft);
    draft
        .preps
        .iter()
        .map(|p| {
            let view = trial.as_ref().and_then(|t| t.supply(&p.id).map(|j| crate::recipe::prep::view(&p.id, &j, &|s| t.supply(s))));
            let name_of = |id: &str| view.as_ref().and_then(|v| v["lines"].as_array()?.iter().find(|l| l["item"] == id)?.get("name").cloned());
            let lines: Vec<Value> = p.lines.iter().map(|(i, q)| json!({ "item": i, "qty": q, "name": name_of(i).unwrap_or(json!(i)) })).collect();
            json!({ "id": p.id, "name": p.name, "unit": p.unit, "yield": p.yield_qty, "lines": lines,
                    "new": cat.supply(&p.id).is_none(), "k": view.as_ref().map_or(Value::Null, |v| v["k"].clone()) })
        })
        .collect()
}

/// The catalogue's supplies for the reader: every one as matchable (a ПФ
/// marked `kind: prep`, so a second import UPDATES the one it made), and the
/// RAW ones as `(id, unit)` -- a ПФ is in no ingredients file, so it is never
/// "not in the file" and never retired by one.
pub(crate) fn supplies_of(cat: &Catalog) -> (Vec<DraftSupply>, Vec<(String, String)>) {
    let mut known = Vec::new();
    let mut existing = Vec::new();
    for (id, j) in cat.supplies() {
        let Ok(v) = serde_json::from_str::<Value>(&j) else { continue };
        let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
        let name = v.get("name").and_then(Value::as_str).unwrap_or(&id);
        let prep = dowiz_hub::prep::is_prep(&j);
        if let Some(mut d) = DraftSupply::known(&id, name, unit) {
            d.kind = prep.then(|| dowiz_hub::prep::KIND.to_string());
            known.push(d);
        }
        if !prep {
            existing.push((id, unit.to_string()));
        }
    }
    (known, existing)
}

#[cfg(test)]
mod tests;
