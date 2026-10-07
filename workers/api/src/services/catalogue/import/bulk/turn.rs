//! ONE IMPORT AS ONE FUNCTION OF THE CATALOGUE AND THE REQUEST (BN1, by the
//! BN4 shape): the dry run used to read the catalogue across the Worker<->object
//! hop to judge the file against it, and Apply read it again inside the write.
//! `turn` is both halves, PURE over the catalogue it is given, so the venue's
//! object runs it in one turn (`hubdo/bulk.rs`, `POST /fold/bulk`) and the
//! catalogue never crosses.

use super::{apply_recipes, apply_supplies_with, extras, preps, projected, read, room, Kind};
use dowiz_hub::catalog::Catalog;
use dowiz_hub::import::recipes::CostScale;
use serde_json::{json, Value};

/// What the Worker sends the venue's object for one import: the file and the
/// switches, never the catalogue.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct BulkIn {
    pub text: String,
    pub kind: Kind,
    /// `?cost=hundredths`: the file's costs are already in minor units.
    pub hundredths: bool,
    pub apply: bool,
    pub retire: bool,
    pub now_ms: i64,
    /// Who signed it, for the menu's edit journal (W-PITR); `""` from an older Worker.
    #[serde(default)]
    pub by: String,
}

/// What one turn of an import decides. `Written` means `cat` now holds the
/// applied draft and is to be saved; the other two wrote nothing.
pub(crate) enum Turn {
    /// The dry run's summary.
    Shown(Value),
    /// Refused, with the status and the words the owner reads.
    Refused(u16, String),
    /// Applied onto `cat`; the summary says how many records were written.
    Written(Value),
}

/// Judge the file against the catalogue as it is, and apply it when asked and
/// nothing refuses. See the module.
pub(crate) fn turn(cat: &mut Catalog, input: &BulkIn) -> Turn {
    let scale = if input.hundredths { CostScale::Hundredths } else { CostScale::Major };
    let (mut draft, mut preview) = read(cat, &input.text, input.kind, scale, input.now_ms);
    // The columns the hub's parser does not read: losses and the pack (W-STOCK P1).
    let more = if input.kind == Kind::Supplies { extras::read(&input.text) } else { Default::default() };
    draft.warnings.extend(more.1.iter().cloned());
    for row in preview.iter_mut() {
        if let Some(x) = row["id"].as_str().and_then(|id| more.0.get(id)) {
            row["cleanPm"] = json!(x.clean_pm);
            row["cookPm"] = json!(x.cook_pm);
            row["pack"] = json!(x.pack.as_ref().map(|p| json!({ "name": p.name, "qty": p.qty })));
        }
    }
    let room = projected(cat, &draft, input.kind, input.retire);
    let no_room = room::refusal(&room, &mut draft.warnings);
    let mut summary = json!({
        "applied": input.apply,
        "supplies": draft.supplies.len(),
        "recipes": draft.recipes.len(),
        "withoutRecipe": draft.without_recipe,
        "notInFile": draft.retired,
        "retired": if input.apply && input.retire { draft.retired.len() } else { 0 },
        "flattened": draft.flattened,
        "preps": preps::rows(cat, &draft),
        "warnings": draft.warnings,
        "rows": preview,
        "catalogue": room,
    });
    if !input.apply {
        return Turn::Shown(summary);
    }
    // A file that produced nothing is refused: applying it is a wrong
    // separator or a missing header, not an intent.
    if draft.supplies.is_empty() && draft.recipes.is_empty() {
        return Turn::Refused(400, format!("nothing to import: {}", draft.warnings.join("; ")));
    }
    // The dry run's answer, given again BEFORE the write rather than as a
    // failed save after it.
    if let Some(why) = no_room {
        return Turn::Refused(413, why);
    }
    let written = match input.kind {
        Kind::Supplies => apply_supplies_with(cat, &draft, input.retire, &more.0),
        Kind::Recipes => apply_recipes(cat, &draft),
    };
    match written {
        Ok(n) => {
            summary["written"] = json!(n);
            Turn::Written(summary)
        }
        // Nothing is saved: the object drops its copy with this answer.
        Err(why) => Turn::Refused(500, why),
    }
}
