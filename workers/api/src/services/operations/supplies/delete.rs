//! DELETING AN INGREDIENT FOR GOOD (W-NOM, 2026-09-28):
//! `POST /api/owner/supplies/delete {ids: [...]}`. NOT a write-off, NOT a
//! retire: the item leaves the nomenclature, every recipe that used it, and
//! the shelf.
//!
//! TWO STEPS, CATALOGUE FIRST. (1) One catalogue write: each supply removed,
//! and every dish whose recipe names one of them loses that line and is
//! re-derived from what is left -- nutrition, weight, ingredient list and
//! cost, exactly as a hand-edited recipe is (`recipe::apply::set_bom`); a
//! value the owner TYPED stays theirs. From this moment no order can reserve
//! it: no recipe names it. (2) The stock ledger, in the venue's object:
//! one signed `Removed` per id it knows (`stock/removed.rs`) -- the history
//! stays in the append-only log, the level, holds, lots and price leave every
//! fold. No waste is recorded and no report counts it as waste.
//!
//! RETRYABLE: a failure between the steps leaves ids the catalogue no longer
//! has and the ledger still knows; deleting them again finishes step 2. An id
//! neither side knows is a 404 -- the second click of a double tap.
//!
//! OWNER ONLY: an irreversible act on the venue's books, like the reset. A
//! member of staff retires (`/retire`), which is reversible.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::recipe::apply::{clear_derived, set_bom, Typed};
use crate::recipe::BomLineIn;
use dowiz_hub::catalog::Catalog;

/// What the catalogue step changed.
#[derive(Debug, Default, PartialEq)]
pub struct Removal {
    /// Supplies the catalogue held and no longer does.
    pub deleted: Vec<String>,
    /// Dishes that lost a recipe line, re-derived.
    pub dishes: Vec<String>,
}

/// The ids of a request: trimmed, distinct, 1 to `MAX_IDS`. PURE.
pub fn ids_of(raw: &[String]) -> std::result::Result<Vec<String>, String> {
    let mut ids: Vec<String> = Vec::new();
    for id in raw.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if !ids.iter().any(|x| x == id) {
            ids.push(id.to_string());
        }
    }
    let max = crate::services::operations::stock::removed::MAX_IDS;
    if ids.is_empty() || ids.len() > max {
        return Err(format!("1 to {max} ingredients at once"));
    }
    Ok(ids)
}

/// Remove `ids` from the catalogue and from every recipe. PURE on `cat`.
pub fn remove_supplies(cat: &mut Catalog, ids: &[String]) -> Removal {
    let gone = |s: &str| ids.iter().any(|i| i == s);
    let deleted: Vec<String> = ids.iter().filter(|id| cat.remove_supply(id)).cloned().collect();
    let mut dishes = Vec::new();
    for (pid, j) in cat.products() {
        let Ok(mut p) = serde_json::from_str::<Value>(&j) else { continue };
        let Some(bom) = p.get("bom").and_then(Value::as_array).cloned() else { continue };
        let keep: Vec<Value> = bom.into_iter().filter(|l| !l.get("supply").and_then(Value::as_str).is_some_and(gone)).collect();
        if keep.len() == p["bom"].as_array().map_or(0, Vec::len) {
            continue;
        }
        let lines: Vec<BomLineIn> = keep
            .iter()
            .filter_map(|l| {
                let weighed = |k: &str| l.get(k).and_then(Value::as_i64);
                Some(BomLineIn { supply: l.get("supply")?.as_str()?.to_string(), qty: l.get("qty")?.as_i64()?, net: weighed("net"), out: weighed("out") })
            })
            .collect();
        let typed = Typed::from_record(&p);
        clear_derived(&mut p);
        // A remaining line whose supply is itself long gone cannot be derived:
        // the recipe keeps its lines, minus the deleted ones, as stored.
        if set_bom(&mut p, &lines, |s| cat.supply(s), typed).is_err() {
            p["bom"] = Value::Array(keep);
        }
        cat.set_product(&pid, &p.to_string());
        dishes.push(pid);
    }
    Removal { deleted, dishes }
}

/// `POST /api/owner/supplies/delete` -- see the module.
pub async fn delete_supplies(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        ids: Vec<String>,
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    // AUTHORITY BEFORE THE BODY (W-FIX O9); the owner's, as the reset's.
    let (by, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let ids = match ids_of(&body.ids) {
        Ok(v) => v,
        Err(why) => return Response::error(why, 400),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let now = ctx.data.now_ms;
    let wanted = ids.clone();
    let (removal, today) = crate::hubstore::with_catalog(&place, move |cat| {
        let r = remove_supplies(cat, &wanted);
        if !r.dishes.is_empty() {
            crate::services::catalogue::import::bump_menu_version(cat);
        }
        Ok((r, crate::services::operations::stock::today_of(cat, now)))
    })
    .await?;
    let input = crate::services::operations::stock::turn::StockTurnIn {
        kind: "removed".into(),
        body: json!({ "items": ids }),
        by,
        now_ms: now,
        today,
        supplies: Default::default(),
    };
    let shelf: Value = match crate::command::send(&place, "stock_move", &input).await {
        Ok(v) => v,
        Err((status, said)) => {
            let why = format!("deleted from the list and the recipes; the shelf was not cleared ({status}: {said}) -- delete again to finish");
            return Response::error(why, 503);
        }
    };
    let stock = shelf.get("removed").cloned().unwrap_or(json!([]));
    if removal.deleted.is_empty() && stock.as_array().is_none_or(Vec::is_empty) {
        return Response::error("unknown ingredient", 404);
    }
    Response::from_json(&json!({ "ok": true, "deleted": removal.deleted, "dishes": removal.dishes, "stock": stock }))
}

#[cfg(test)]
mod tests;
