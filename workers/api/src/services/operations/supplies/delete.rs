//! DELETING AN INGREDIENT FOR GOOD (W-NOM, 2026-09-28): `POST
//! /api/owner/supplies/delete {ids: [...]}`. NOT a write-off, NOT a retire:
//! the item leaves the nomenclature, every recipe that used it, and the shelf.
//!
//! TWO STEPS, CATALOGUE FIRST. (1) One catalogue write: each supply removed;
//! every dish whose recipe (or option recipe, W-QTYHUB H5) names one loses
//! the line and is re-derived as a hand-edited recipe is (`set_bom`; a TYPED
//! value stays). No order can reserve it from here. THE DISH IS MARKED
//! (CONTRACT-QTY §2): `recipeReview: [{supply, name, qty, via?, option?}]`, a
//! ПФ `review: [...]`; the mark goes with the next hand save, never a
//! re-derivation. (2) The ledger, in the venue's object: one signed `Removed`
//! per id it knows (`stock/removed.rs`); history stays, level/holds/lots/price
//! leave every fold. No waste is recorded.
//!
//! RETRYABLE: deleting again finishes step 2; an id neither side knows is a
//! 404. OWNER ONLY: irreversible, like the reset (staff retire, `/retire`).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::recipe::apply::{clear_derived, set_bom_keeping_review, Typed};
use crate::recipe::{BomLineIn, Qty};
use dowiz_hub::catalog::Catalog;

/// What the catalogue step changed.
#[derive(Debug, Default, PartialEq)]
pub struct Removal {
    /// Supplies the catalogue held and no longer does.
    pub deleted: Vec<String>,
    /// Dishes that lost a recipe line, re-derived.
    pub dishes: Vec<String>,
    /// The marks set: `{id, name, removed: [...]}` per dish, `prep: true` on a ПФ.
    pub review: Vec<Value>,
}

/// The review marks a deletion leaves (contract §2), pure.
pub mod review;
use review::{mark, mark_on, options_only, strip_option_bom};

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
        if u.preps.is_empty() && u.dishes.is_empty() && u.options.is_empty() {
            continue;
        }
        let rows = |l: &[(String, String)]| l.iter().map(|(i, n)| json!({ "id": i, "name": n })).collect::<Vec<_>>();
        out.insert(id.clone(), review::with_options(json!({ "preps": rows(&u.preps), "dishes": rows(&u.dishes) }), &u.options));
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
    let name_of = |id: &str| -> String {
        before.iter().find(|(i, _)| i == id).and_then(|(_, j)| dowiz_hub::minijson::str_field(j, "name")).unwrap_or_else(|| id.to_string())
    };
    // (ПФ, the marks it takes): which cards reach a deleted item, READ BEFORE
    // anything is removed; a dish naming such a card is re-derived and marked
    // below although its own lines do not change.
    let mut touched: Vec<(String, Vec<Value>)> = Vec::new();
    let deleted: Vec<String> = ids.iter().filter(|id| cat.remove_supply(id)).cloned().collect();
    let mut review = Vec::new();
    // The cards first, so the dishes below re-derive from cards without the line.
    for (sid, j) in &before {
        // A ПФ deleted in this same call is not rewritten: that would write it back.
        if gone(sid) {
            continue;
        }
        let Some(mut card) = dowiz_hub::prep::card_of(j) else { continue };
        let marks: Vec<Value> = card.lines.iter().filter(|l| gone(&l.item)).map(|l| mark(&l.item, &name_of(&l.item), crate::recipe::Qty::of_micro(l.uq).json(), None)).collect();
        if marks.is_empty() {
            continue;
        }
        card.lines.retain(|l| !gone(&l.item));
        let Ok(mut v) = serde_json::from_str::<Value>(j) else { continue };
        v["card"] = dowiz_hub::prep::card_json(&card);
        // The ПФ's mark lives at the TOP of its record, under `review`: a key
        // that sorts after `name`, so `minijson::str_field(j, "name")` (the
        // first "name" in the text) still answers the ПФ's own name and not the
        // removed ingredient's; `preps::record` rebuilds the record without it.
        mark_on(&mut v, "review", marks.clone());
        cat.set_supply(sid, &v.to_string());
        review.push(json!({ "id": sid, "name": name_of(sid), "prep": true, "removed": marks.clone() }));
        touched.push((sid.clone(), marks));
    }
    // Every card ABOVE a touched one reaches the removed item too: marked (2026-10-16).
    review::mark_above(cat, &mut touched, &mut review);
    let mut dishes = Vec::new();
    for (pid, j) in cat.products() {
        let Ok(mut p) = serde_json::from_str::<Value>(&j) else { continue };
        // W-QTYHUB H5: an option's recipe loses the line too, the dish marked naming the option.
        let opt_marks = strip_option_bom(&mut p, &gone, &name_of);
        let Some(bom) = p.get("bom").and_then(Value::as_array).cloned() else {
            if let Some(r) = options_only(&mut p, &pid, opt_marks) {
                cat.set_product(&pid, &p.to_string());
                review.push(r);
                dishes.push(pid);
            }
            continue;
        };
        let (lost, keep): (Vec<Value>, Vec<Value>) = bom.into_iter().partition(|l| l.get("supply").and_then(Value::as_str).is_some_and(gone));
        let mut marks: Vec<Value> = lost
            .iter()
            .filter_map(|l| Some(mark(l.get("supply")?.as_str()?, &name_of(l.get("supply")?.as_str()?), Qty::from_stored(l).map(Qty::json).unwrap_or(Value::Null), None)))
            .collect();
        // A line naming a touched card: the dish reaches the removed item through it.
        for l in &keep {
            if let Some((card, via)) = l.get("supply").and_then(Value::as_str).and_then(|s| touched.iter().find(|(t, _)| t == s)) {
                marks.extend(via.iter().map(|m| {
                    let mut m = m.clone();
                    if m.get("via").is_none() {
                        m["via"] = json!(card);
                    }
                    m
                }));
            }
        }
        marks.extend(opt_marks);
        if marks.is_empty() {
            continue;
        }
        let lines: Vec<BomLineIn> = keep
            .iter()
            .filter_map(|l| {
                let weighed = |k: &str| l.get(k).and_then(Value::as_i64);
                Some(BomLineIn { supply: l.get("supply")?.as_str()?.to_string(), qty: Qty::from_stored(l)?, net: weighed("net"), out: weighed("out") })
            })
            .collect();
        let typed = Typed::from_record(&p);
        clear_derived(&mut p);
        mark_on(&mut p, "recipeReview", marks.clone());
        // A remaining line whose supply is itself long gone cannot be derived:
        // the recipe keeps its lines, minus the deleted ones, as stored.
        if set_bom_keeping_review(&mut p, &lines, |s| cat.supply(s), typed).is_err() {
            p["bom"] = Value::Array(keep);
        }
        cat.set_product(&pid, &p.to_string());
        review.push(json!({ "id": pid, "name": p.get("name").cloned().unwrap_or(json!(pid)), "removed": marks }));
        dishes.push(pid);
    }
    Removal { deleted, dishes, review }
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
    Response::from_json(&json!({ "ok": true, "deleted": removal.deleted, "dishes": removal.dishes, "review": removal.review, "stock": stock }))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod combo_tests;
