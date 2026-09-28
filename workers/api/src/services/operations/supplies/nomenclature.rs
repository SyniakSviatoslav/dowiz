//! THE NOMENCLATURE'S OWN FIELDS (W-NOM, 2026-09-28): what a venue calls an
//! item in its own books (`code`), what is printed on the pack (`barcode`),
//! and how it is bought (`packs`: "box 5 kg" = 5000 g), so a delivery can be
//! typed as "2 boxes" instead of "10000". Integers in the supply's base unit,
//! like every quantity in the ledger. All optional; absent keys are left out
//! of the stored record (the catalogue spends a cell per byte).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::SupplyIn;

pub const CODE_MAX: usize = 32;
pub const PACK_NAME_MAX: usize = 24;
pub const PACKS_MAX: usize = 6;
/// A pack holds at most a tonne / 1000 l / a million pieces.
pub const PACK_QTY_MAX: i64 = 1_000_000;

/// One way the item is bought, in its base unit.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub name: String,
    pub qty: i64,
}

/// Why the nomenclature fields cannot be written, or `Ok`. PURE.
pub fn check(body: &SupplyIn) -> Result<(), String> {
    if body.code.as_deref().is_some_and(|c| c.trim().chars().count() > CODE_MAX) {
        return Err(format!("a code is at most {CODE_MAX} characters"));
    }
    if let Some(b) = body.barcode.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
        if !(6..=18).contains(&b.len()) || !b.bytes().all(|c| c.is_ascii_digit()) {
            return Err("a barcode is 6 to 18 digits".into());
        }
    }
    let packs = body.packs.as_deref().unwrap_or_default();
    if packs.len() > PACKS_MAX {
        return Err(format!("at most {PACKS_MAX} packs"));
    }
    for (i, p) in packs.iter().enumerate() {
        let n = p.name.trim();
        if n.is_empty() || n.chars().count() > PACK_NAME_MAX {
            return Err(format!("a pack needs a name of up to {PACK_NAME_MAX} characters"));
        }
        if !(1..=PACK_QTY_MAX).contains(&p.qty) {
            return Err(format!("{n}: a pack holds 1 to {PACK_QTY_MAX} of the base unit"));
        }
        if packs[..i].iter().any(|q| q.name.trim().to_lowercase() == n.to_lowercase()) {
            return Err(format!("{n}: two packs with one name"));
        }
    }
    Ok(())
}

/// A text field as the record stores it: absent keeps, "" clears. PURE.
pub fn text(v: &Option<String>) -> Option<Value> {
    v.as_deref().map(|s| match s.trim() {
        "" => Value::Null,
        t => json!(t),
    })
}

/// The packs as the record stores them: absent keeps, [] clears. PURE.
pub fn packs(v: &Option<Vec<Pack>>) -> Option<Value> {
    v.as_ref().map(|ps| {
        if ps.is_empty() {
            return Value::Null;
        }
        Value::Array(ps.iter().map(|p| json!({ "name": p.name.trim(), "qty": p.qty })).collect())
    })
}

#[cfg(test)]
mod tests;
