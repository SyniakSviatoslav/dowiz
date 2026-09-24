//! Will this import FIT? Asked on the dry run (2026-09-24).
//!
//! The catalogue is one compacted image with a hard ceiling
//! (`dowiz_hub::catalog::DEFAULT_CATALOG_BYTES`). A recipes file for a whole
//! menu once modelled to 1245 per mille of it: the preview said nothing, and
//! Apply would have failed at the save. The preview now carries the number.

use dowiz_hub::catalog::Catalog;
use dowiz_hub::import::recipes::RecipeDraft;
use serde_json::{json, Value};

use super::{apply_recipes, apply_supplies, Kind};

/// What the catalogue would spend with this draft applied, and whether it
/// could be saved: `{nowPerMille, perMille, fits}`, plus `said` when it could
/// not. Asked on the DRY RUN, so a file too large for the venue's catalogue
/// is refused before Apply rather than failing the save after it. Applies
/// the draft to a COPY; `cat` is not changed.
pub(crate) fn projected(cat: &mut Catalog, draft: &RecipeDraft, kind: Kind, retire: bool) -> Value {
    let now = cat.projected().map(|p| p.usage.used_per_mille()).ok();
    let mut trial = match cat.to_bytes().and_then(|b| Catalog::load(&b)) {
        Ok(t) => t,
        Err(e) => return json!({ "nowPerMille": now, "perMille": null, "fits": false, "said": format!("the catalogue cannot be read to measure this import: {e:?}") }),
    };
    let applied = match kind {
        Kind::Supplies => apply_supplies(&mut trial, draft, retire),
        Kind::Recipes => apply_recipes(&mut trial, draft),
    };
    if applied.is_err() {
        // Not a question of room: Apply refuses it with its own reason.
        return json!({ "nowPerMille": now, "perMille": null, "fits": null });
    }
    let after = trial.projected().ok();
    let per_mille = after.map(|p| p.usage.used_per_mille());
    if after.is_some_and(|p| p.fits) {
        return json!({ "nowPerMille": now, "perMille": per_mille, "fits": true });
    }
    let much = per_mille.map_or("more than four times".to_string(), |n| format!("{n} per mille of"));
    json!({ "nowPerMille": now, "perMille": per_mille, "fits": false,
            "said": format!("this import would not fit: the catalogue would be {much} its ceiling; nothing would be saved") })
}

/// The dry run's "it would not fit": a warning on the preview, and the reason
/// Apply is refused BEFORE the write. `None` when there is room, or when the
/// draft was not measured (Apply names that reason itself).
pub(crate) fn refusal(room: &Value, warnings: &mut Vec<String>) -> Option<String> {
    let why = room["said"].as_str()?.to_string();
    warnings.push(why.clone());
    Some(why)
}
