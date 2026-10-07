//! THE MENU'S EDIT HISTORY (W-PITR, R-BEBOPDB D.1 #11): the journal every catalogue write
//! appends to, the owner's "Історія змін меню" list, and "restore this dish to before this
//! edit".
//!
//! WHERE THE APPEND HAPPENS (W-PITR2). In the venue's object, in the SAME atomic storage write
//! as the catalogue it describes (`hubdo/journal.rs`, called from `put_image_as`): every
//! catalogue write the object stores is journaled, whoever asked for it, and a cut leaves both
//! or neither. The Worker only says who signed (`x-edit-by`/`x-edit-at`, [`stamp_of`]); the
//! journal never crosses the hop. `every_catalogue_write_is_journaled_in_the_objects_write`
//! (tests) holds that door.
//!
//! PERSONAL DATA: `by` is the STAFF or owner id that signed the request (`Place::by`), never
//! a customer; the privacy registry row is `catalog.edits`.

use dowiz_hub::catalog::edits::{self, Edit};
use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The journal image. Backed up with the catalogue (`IMAGES`, `hubstore/backup.rs`).
pub const IMAGE: &str = "catalog.edits";
/// The contract version of both routes' answers.
pub const VERSION: i64 = 1;
const LIMIT_DEFAULT: usize = 30;
const LIMIT_MAX: usize = 200;

/// The signer of a catalogue PUT, as `hubstore::save_image` sends it (`x-edit-by`, `x-edit-at`).
/// The object journals the write with it in the same storage write (`hubdo/journal.rs`).
pub fn stamp_of(h: &crate::wire::Fields) -> Option<crate::hubstore::Edited> {
    let get = |k: &str| h.get(k).ok().flatten();
    let at_ms = get("x-edit-at").and_then(|v| v.parse().ok())?;
    Some(crate::hubstore::Edited { by: get("x-edit-by").unwrap_or_default(), at_ms })
}

/// `product:d1` -> `("product", "d1")`; `location` -> `("location", "")`.
fn split(key: &str) -> (&str, &str) {
    key.split_once(':').unwrap_or((key, ""))
}

/// May the console offer "restore to before this edit"? A dish, and a real edit: a
/// baseline or unseen record that ADDED the dish is where the history starts, not a change.
pub fn restorable(e: &Edit) -> bool {
    split(&e.key).0 == "product" && !(e.old.is_none() && (e.by == edits::UNSEEN || e.by == edits::BASELINE))
}

/// One record as the console draws it. PURE.
pub fn row(e: &Edit) -> Value {
    let (kind, id) = split(&e.key);
    let v: Value = e.new.as_deref().and_then(|j| serde_json::from_str(j).ok()).unwrap_or(Value::Null);
    json!({
        "seq": e.seq, "at": e.at_ms, "by": e.by, "key": e.key, "kind": kind, "id": id,
        "name": v.get("name").cloned().unwrap_or(Value::Null),
        "price": v.get("price").cloned().unwrap_or(Value::Null),
        "available": v.get("available").cloned().unwrap_or(Value::Null),
        "added": e.old.is_none(), "removed": e.new.is_none(),
        "restorable": restorable(e),
    })
}

/// `GET /api/owner/menu/history?location_id=&limit=` — the newest edits, newest first.
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_, loc) = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &crate::services::identity::staff::guard::MENU).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let limit = req.url()?.query_pairs().find(|(k, _)| k == "limit").and_then(|(_, v)| v.parse().ok()).unwrap_or(LIMIT_DEFAULT).clamp(1, LIMIT_MAX);
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let log = crate::hubstore::load_log(&place, IMAGE).await?.log;
    match edits::recent(&log, limit) {
        Ok(list) => Response::from_json(&json!({
            "version": VERSION, "total": log.len(), "edits": list.iter().map(row).collect::<Vec<_>>(),
        })),
        // A journal this build cannot read is a refusal by name, never an empty history.
        Err(e) => Response::error(format!("the menu history does not read: {e:?}"), 500),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreIn {
    location_id: String,
    seq: u64,
    /// The record's key as the console listed it: a journal compacted in between renumbers
    /// its records, and a restore must not land on a different dish.
    key: String,
}

/// `POST /api/owner/menu/history/restore` — put one dish back to what it was just BEFORE the
/// edit `seq` (a dish that edit created is removed). The restore is itself journaled.
pub async fn restore(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: RestoreIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let who = match crate::courier::staff_at(&req, &ctx, &body.location_id, crate::auth::Cap::Catalog).await {
        Ok((who, _)) => who,
        Err(r) => return Ok(r),
    };
    let log = crate::hubstore::load_log(&place, IMAGE).await?.log;
    let (edit, was) = match edits::before(&log, body.seq) {
        Ok(Some(found)) => found,
        Ok(None) => return Response::error("no such edit; reload the history", 404),
        Err(e) => return Response::error(format!("the menu history does not read: {e:?}"), 500),
    };
    if edit.key != body.key {
        return Response::error("the history moved since it was shown; reload it", 409);
    }
    if !restorable(&edit) {
        return Response::error("only a dish's edit can be restored", 400);
    }
    let place = place.by(&who);
    let key = edit.key.clone();
    let value = was.clone();
    crate::hubstore::with_catalog(&place, move |cat| {
        edits::set_key(cat, &key, value.as_deref());
        crate::services::catalogue::import::bump_menu_version(cat);
        Ok(())
    })
    .await?;
    // A restore that removes the dish (the edit created it) takes its translations too, as a delete does.
    if was.is_none() {
        let id = split(&edit.key).1.to_string();
        crate::catalog_edit::forget::forget_or_log(&place, "product", &[id]).await;
    }
    let v: Value = was.as_deref().and_then(|j| serde_json::from_str(j).ok()).unwrap_or(Value::Null);
    Response::from_json(&json!({
        "version": VERSION, "ok": true, "key": edit.key, "removed": was.is_none(),
        "price": v.get("price").cloned().unwrap_or(Value::Null),
    }))
}

#[cfg(test)]
mod tests;
/// The restore drill over a copy the REAL nightly path produced (W-PITR row 1).
#[cfg(test)]
#[path = "catalog_history/drill/tests.rs"]
mod drill_tests;
