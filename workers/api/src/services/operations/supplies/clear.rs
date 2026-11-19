//! The editor's emptied boxes (`SupplyIn::clear`). Absent keeps what is stored,
//! so without an explicit clear an emptied price came back on every save.

use super::SupplyIn;
use serde_json::{Map, Value};

/// The fields an emptied editor box removes; the rest have a default instead
/// (an emptied category is sent as "", the minimum as 0).
pub(crate) const CLEARABLE: [&str; 8] =
    ["kcalPer100", "proteinPer100", "fatPer100", "carbsPer100", "costPerBasis", "weightPerUnit", "shelfDays", "supplier"];

/// Why the clear list cannot be applied, or nothing. PURE.
pub(crate) fn check(body: &SupplyIn) -> std::result::Result<(), String> {
    for k in body.clear.iter().flatten() {
        if !CLEARABLE.contains(&k.as_str()) {
            return Err(format!("{k} cannot be cleared; clearable: {}", CLEARABLE.join(", ")));
        }
        if given(body, k) {
            return Err(format!("{k} is both given and cleared"));
        }
    }
    Ok(())
}

/// The record without the cleared fields (`check` allowed only these names).
pub(crate) fn apply(body: &SupplyIn, m: &mut Map<String, Value>) {
    for k in body.clear.iter().flatten().filter(|k| CLEARABLE.contains(&k.as_str())) {
        m.remove(k.as_str());
    }
}

/// Does the body carry a value for the clearable field `k`?
fn given(b: &SupplyIn, k: &str) -> bool {
    match k {
        "kcalPer100" => b.kcal_per100.is_some(),
        "proteinPer100" => b.protein_per100.is_some(),
        "fatPer100" => b.fat_per100.is_some(),
        "carbsPer100" => b.carbs_per100.is_some(),
        "costPerBasis" => b.cost_per_basis.is_some(),
        "weightPerUnit" => b.weight_per_unit.is_some(),
        "shelfDays" => b.shelf_days.is_some(),
        "supplier" => b.supplier.is_some(),
        _ => false,
    }
}
