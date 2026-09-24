//! The recipes file ALONE, read against the supplies the catalogue already has.
//!
//! `from_csv` reads the two files together because an incumbent exports both
//! at once. An owner filling a venue by hand does it in two steps -- the
//! supplies first, then the recipes, a day later -- and the second file names
//! supplies that are in the catalogue, not in a file beside it. This is that
//! second step: every rule of `cards` unchanged (a dish with any unreadable
//! line gets NO recipe, never half of one), with the catalogue standing in for
//! the ingredients file, and nothing about the supplies changed or retired.

use super::{cards, DraftSupply, Opts, RecipeDraft};

impl DraftSupply {
    /// A supply as the catalogue holds it, for matching a recipe line by id or
    /// by name. `None` for a unit outside g/ml/unit: such a record was never
    /// writable through the console, and guessing its unit would scale every
    /// line that names it by the wrong factor.
    pub fn known(id: &str, name: &str, unit: &str) -> Option<DraftSupply> {
        let unit = ["g", "ml", "unit"].into_iter().find(|u| *u == unit)?;
        Some(DraftSupply {
            id: id.to_string(),
            name: name.to_string(),
            unit,
            kind: None,
            category: String::new(),
            cost_per_basis: None,
            kcal: None,
            protein: None,
            fat: None,
            carbs: None,
            low_at: None,
            weight_per_unit: None,
            supplier: None,
        })
    }
}

/// Read `recipes` against `catalogue`. The draft's `supplies` and `retired`
/// are always empty: this step writes recipes and nothing else.
pub fn recipes_against(recipes: &str, opts: &Opts, catalogue: &[DraftSupply]) -> RecipeDraft {
    let mut draft = RecipeDraft { supplies: catalogue.to_vec(), ..RecipeDraft::default() };
    if recipes.trim().is_empty() {
        draft.warnings.push("the recipes file is empty".into());
    } else if catalogue.is_empty() {
        draft.warnings.push("there are no supplies yet: import the supplies file first".into());
    } else {
        cards::read(recipes, opts, &mut draft);
    }
    draft.supplies.clear();
    draft
}

#[cfg(test)]
mod tests;
