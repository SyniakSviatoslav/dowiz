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

/// Where each of `ids` is still used (SPEC-SEMI-FINISHED §e): the owner sees
/// this list and confirms before a line leaves any card or recipe. PURE.
pub fn uses_of(cat: &Catalog, ids: &[String]) -> Value {
    let (supplies, products) = (cat.supplies(), cat.products());
    let mut out = serde_json::Map::new();
    for id in ids {
        let u = dowiz_hub::prep::uses_of(id, &supplies, &products);
        if u.preps.is_empty() && u.dishes.is_empty() {
            continue;
        }
        let rows = |l: &[(String, String)]| l.iter().map(|(i, n)| json!({ "id": i, "name": n })).collect::<Vec<_>>();
        out.insert(id.clone(), json!({ "preps": rows(&u.preps), "dishes": rows(&u.dishes) }));
    }
    Value::Object(out)
}

/// Remove `ids` from the catalogue, from every semi-finished card and from
/// every recipe. PURE on `cat`.
pub fn remove_supplies(cat: &mut Catalog, ids: &[String]) -> Removal {
    let gone = |s: &str| ids.iter().any(|i| i == s);
    // Which cards reach a deleted item, READ BEFORE anything is removed: a
    // dish naming such a card is re-derived below although its own lines
    // do not change.
    let before = cat.supplies();
    let touched: Vec<String> = ids.iter().flat_map(|id| dowiz_hub::prep::uses_of(id, &before, &[]).preps).map(|(p, _)| p).collect();
    let deleted: Vec<String> = ids.iter().filter(|id| cat.remove_supply(id)).cloned().collect();
    // The cards first, so the dishes below re-derive from cards without the line.
    for (sid, j) in before {
        // A ПФ deleted in this same call is not rewritten: that would write it back.
        if gone(&sid) {
            continue;
        }
        let Some(mut card) = dowiz_hub::prep::card_of(&j) else { continue };
        let n = card.lines.len();
        card.lines.retain(|l| !gone(&l.item));
        if card.lines.len() == n {
            continue;
        }
        let Ok(mut v) = serde_json::from_str::<Value>(&j) else { continue };
        v["card"] = dowiz_hub::prep::card_json(&card);
        cat.set_supply(&sid, &v.to_string());
    }
    let mut dishes = Vec::new();
    for (pid, j) in cat.products() {
        let Ok(mut p) = serde_json::from_str::<Value>(&j) else { continue };
        let Some(bom) = p.get("bom").and_then(Value::as_array).cloned() else { continue };
        let keep: Vec<Value> = bom.into_iter().filter(|l| !l.get("supply").and_then(Value::as_str).is_some_and(gone)).collect();
        // Unchanged lines, and no card under them touched: not rewritten.
        let through_card = keep.iter().any(|l| l.get("supply").and_then(Value::as_str).is_some_and(|s| touched.iter().any(|t| t == s)));
        if keep.len() == p["bom"].as_array().map_or(0, Vec::len) && !through_card {
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

/// Why a delete was held back before anything changed.
#[derive(Debug, PartialEq)]
pub enum Held {
    /// Where the ids are used (`uses_of`), for the owner to confirm.
    InUse(Value),
    /// Semi-finished products whose card would be left with NO line (and
    /// which are not deleted in the same call), by id. Refused even when
    /// confirmed (W-VERIFY 2026-09-30).
    Emptied(Vec<String>),
}

/// The catalogue step of a delete, PURE on `cat`: where-used first unless the
/// owner `confirmed`, then the removal.
pub fn delete_in(cat: &mut Catalog, ids: &[String], confirmed: bool) -> std::result::Result<Removal, Held> {
    if !confirmed {
        let used = uses_of(cat, ids);
        if !used.as_object().is_some_and(|m| m.is_empty()) {
            return Err(Held::InUse(used));
        }
    }
    // A card this would strip to NO line: its dishes would sell drawing
    // nothing and cost 0 (a card `check_card` refuses on save).
    let gone = |s: &str| ids.iter().any(|i| i == s);
    let emptied: Vec<String> = cat
        .supplies()
        .into_iter()
        .filter(|(sid, _)| !gone(sid))
        .filter(|(_, j)| dowiz_hub::prep::card_of(j).is_some_and(|c| !c.lines.is_empty() && c.lines.iter().all(|l| gone(&l.item))))
        .map(|(sid, _)| sid)
        .collect();
    if !emptied.is_empty() {
        return Err(Held::Emptied(emptied));
    }
    Ok(remove_supplies(cat, ids))
}

/// `POST /api/owner/supplies/delete` -- see the module.
pub async fn delete_supplies(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    struct In {
        ids: Vec<String>,
        /// The owner saw where the ids are used (`uses_of`) and still says delete.
        #[serde(default)]
        confirm_uses: bool,
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    // AUTHORITY BEFORE THE BODY (W-FIX O9); the owner's, as the reset's.
    let (by, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let body: In = match crate::body::parse(&mut req).await {
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
    let confirmed = body.confirm_uses;
    let (removal, today) = match crate::hubstore::with_catalog(&place, move |cat| {
        // WHERE IT IS USED, FIRST (SPEC §e): a line leaves a card or a recipe
        // only after the owner has seen it named.
        let r = match delete_in(cat, &wanted, confirmed) {
            Ok(r) => r,
            Err(Held::InUse(used)) => return Err(Error::RustError(format!("in-use: {used}"))),
            Err(Held::Emptied(ids)) => return Err(Error::RustError(format!("emptied: {}", json!(ids)))),
        };
        if !r.dishes.is_empty() {
            crate::services::catalogue::import::bump_menu_version(cat);
        }
        Ok((r, crate::services::operations::stock::today_of(cat, now)))
    })
    .await
    {
        Ok(v) => v,
        Err(e) => {
            let said = e.to_string();
            if let Some((_, used)) = said.split_once("in-use: ") {
                let uses: Value = serde_json::from_str(used).unwrap_or(Value::Null);
                return Ok(Response::from_json(&json!({ "error": "in use", "uses": uses }))?.with_status(409));
            }
            if let Some((_, ids)) = said.split_once("emptied: ") {
                let emptied: Value = serde_json::from_str(ids).unwrap_or(Value::Null);
                let why = "a semi-finished product would be left with no lines: delete it too, or give its card another line first";
                return Ok(Response::from_json(&json!({ "error": why, "emptied": emptied }))?.with_status(409));
            }
            return Err(e);
        }
    };
    let input = crate::services::operations::stock::turn::StockTurnIn {
        kind: "removed".into(),
        body: json!({ "items": ids }),
        by,
        now_ms: now,
        today,
        supplies: Default::default(),
        currency: String::new(),
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
#[cfg(test)]
mod combo_tests;
