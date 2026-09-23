//! The INGREDIENTS file: one row per thing a kitchen buys.

use super::num::{self, CostScale};
use super::{DraftSupply, Opts, RecipeDraft, Sheet};
use crate::import::slug;

const KINDS: &[&str] = &["food_ingredient", "condiment", "packaging", "utensil"];

fn kind_of(raw: &str) -> Option<&'static str> {
    let l = raw.trim().to_lowercase();
    if let Some(k) = KINDS.iter().find(|k| **k == l) {
        return Some(k);
    }
    Some(match l.as_str() {
        "ingredient" | "food" | "përbërës" | "інгредієнт" | "ингредиент" => "food_ingredient",
        "sauce" | "spice" | "salcë" | "соус" | "спеції" => "condiment",
        "package" | "box" | "ambalazh" | "упаковка" => "packaging",
        "cutlery" | "tool" | "прибори" => "utensil",
        _ => return None,
    })
}

/// A nutrition figure: a non-negative finite number, else refused with a warning.
fn nutrition(sheet: &Sheet, cells: &[String], key: &str, row: usize, w: &mut Vec<String>) -> Option<f64> {
    let raw = sheet.get(cells, key);
    if raw.is_empty() {
        return None;
    }
    match num::parse_decimal(raw) {
        Ok(d) if d.scale <= 9 => Some(d.mant as f64 / 10f64.powi(d.scale as i32)),
        _ => {
            w.push(format!("ingredients row {row}: {key} {raw:?} is not a number; left out"));
            None
        }
    }
}

/// Read the file into `draft.supplies`. `false` means the file was REFUSED
/// WHOLE — it is in a currency the venue does not trade in, and converting
/// would put a rate nobody chose into every dish's cost.
pub(super) fn read(text: &str, opts: &Opts, draft: &mut RecipeDraft) -> bool {
    if text.trim().is_empty() {
        return true;
    }
    let Some(sheet) = Sheet::parse(text) else { return true };
    if !sheet.has("name") || !sheet.has("unit") {
        draft.warnings.push("the ingredients file needs a name column and a unit column".into());
        return true;
    }
    // THE CURRENCY, before a single row is taken: a column or a word beside a
    // cost naming another currency refuses the whole file.
    for (row, cells) in &sheet.rows {
        let word = match sheet.get(cells, "currency") {
            "" => num::split_unit(sheet.get(cells, "cost")).1.unwrap_or(""),
            c => c,
        };
        if let Some(code) = num::currency_word(word) {
            if !code.eq_ignore_ascii_case(opts.currency) {
                draft.warnings.push(format!(
                    "ingredients row {row}: this file is in {code}; this venue trades in {}. \
                     Nothing was imported — export it in {} or change nothing",
                    opts.currency, opts.currency
                ));
                return false;
            }
        }
    }
    if sheet.has("cost") && opts.cost_scale.is_none() {
        draft.warnings.push(
            "costs were not imported: say whether the cost column is in whole units \
             (cost=major) or in hundredths, as Poster writes it (cost=hundredths)"
                .into(),
        );
    }
    for (row, cells) in &sheet.rows {
        let row = *row;
        let name = sheet.get(cells, "name");
        if name.is_empty() {
            draft.warnings.push(format!("ingredients row {row}: no name, skipped"));
            continue;
        }
        let unit_raw = sheet.get(cells, "unit");
        let Some(unit) = num::unit_of(unit_raw) else {
            draft.warnings.push(format!(
                "ingredients row {row} ({name}): unit {unit_raw:?} is not g, kg, ml, l or pieces; skipped"
            ));
            continue;
        };
        let explicit = sheet.get(cells, "id");
        let id = slug(if explicit.is_empty() { name } else { explicit });
        if draft.supplies.iter().any(|s| s.id == id) {
            draft.warnings.push(format!("ingredients row {row} ({name}): {id:?} appears more than once; the first row wins"));
            continue;
        }
        if let Some((_, was)) = opts.existing_supplies.iter().find(|(e, u)| *e == id && u != unit.0.as_str()) {
            draft.warnings.push(format!(
                "ingredients row {row} ({name}): counted in {was} now, the file says {}; \
                 the stock ledger's counts would change meaning, so it is skipped",
                unit.0.as_str()
            ));
            continue;
        }
        let kind = match sheet.get(cells, "kind") {
            "" => None,
            k => match kind_of(k) {
                Some(k) => Some(k.to_string()),
                None => {
                    draft.warnings.push(format!("ingredients row {row} ({name}): kind {k:?} is not food, condiment, packaging or utensil; left as it was"));
                    None
                }
            },
        };
        let cost = match (opts.cost_scale, num::split_unit(sheet.get(cells, "cost")).0) {
            (_, "") | (None, _) => None,
            (Some(scale), raw) => match cost_of(raw, scale, opts.currency, unit) {
                Ok(c) => Some(c),
                Err(why) => {
                    draft.warnings.push(format!("ingredients row {row} ({name}): cost {raw:?} {why}; left out"));
                    None
                }
            },
        };
        let w = &mut draft.warnings;
        let kcal = nutrition(&sheet, cells, "kcal", row, w);
        let protein = nutrition(&sheet, cells, "protein", row, w);
        let fat = nutrition(&sheet, cells, "fat", row, w);
        let carbs = nutrition(&sheet, cells, "carbs", row, w);
        draft.supplies.push(DraftSupply {
            id,
            name: name.to_string(),
            unit: unit.0.as_str(),
            kind,
            category: sheet.get(cells, "category").to_string(),
            cost_per_basis: cost,
            kcal,
            protein,
            fat,
            carbs,
        });
    }
    true
}

fn cost_of(raw: &str, scale: CostScale, currency: &str, unit: (num::Base, i128)) -> Result<i64, String> {
    let d = num::parse_decimal(raw)?;
    num::cost_per_basis(d, scale, currency, unit)
}
