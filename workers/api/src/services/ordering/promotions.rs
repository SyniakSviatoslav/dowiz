//! The owner's side of a promo code: list, create or replace, delete.
//!
//! THE RULES ARE THE HUB'S. `dowiz_hub::promo` decides what a code may look
//! like, what a value may be and whether one may be redeemed; this file is the
//! doorway, and `preview.rs` beside it is the customer's side of the same
//! vocabulary. Keeping the two in one directory is the point: a rule that
//! changes has one place to change.

use serde_json::{json, Value};
use worker::*;

use crate::owner::{now_ms, owner_and_venue};
use crate::services::venue::currency_of;
use super::promo_fields::PromoIn;


/// `GET /api/owner/promotions?location_id=`
pub async fn promotions(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and the image read do not depend on each other;
    // `owner_with_hub` runs them together. See it for why the token is still
    // verified before either is issued.
    let (_, _loc, (loaded, cat)) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_both(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let cat = cat.catalog;
    let now = now_ms();
    let mut rows: Vec<Value> = cat
        .promos()
        .into_iter()
        .filter_map(|(_, j)| dowiz_hub::promo::Promo::parse(&j))
        .map(|p| {
            // Status is DERIVED, never stored: a stored one goes stale the
            // moment the clock passes the window, and the owner would be
            // reading a label that no longer describes the code.
            let used = crate::hubstore::promo_uses(&loaded.hub, &p.code);
            json!({
                "code": p.code, "kind": p.kind.as_str(), "value": p.value,
                "minOrder": p.min_order, "fromMs": p.from_ms, "untilMs": p.until_ms,
                "maxUses": p.max_uses, "active": p.active,
                "used": used, "status": p.status(now, used).as_str(),
            })
        })
        .collect();
    // Alphabetical: a code is looked up by its name, and sorting by status would
    // move a row under the owner's cursor the moment a window closed.
    rows.sort_by(|a, b| a["code"].as_str().cmp(&b["code"].as_str()));
    Response::from_json(&json!({ "promotions": rows, "currency": currency_of(&cat) }))
}

/// `POST /api/owner/promotions?location_id=` — create or replace.
///
/// AN UNKNOWN FIELD IS A REFUSAL. serde's default is to ignore what it does not
/// recognise, and for a promo that gives money away: a client sending `until`
/// instead of `untilMs` gets a code with no expiry, silently, for ever.
/// Deserialised by hand so the refusal names the field instead of arriving as a
/// bare 422 with an empty body.
pub async fn set_promotion(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let raw: Value = match req.json().await {
        Ok(v) => v,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let body: PromoIn = match serde_json::from_value(raw) {
        Ok(b) => b,
        Err(e) => return Response::error(e.to_string(), 400),
    };
    // The form's rules are `promo_fields`, where each one is tested against
    // the money it lets out of the till.
    let p = match super::promo_fields::to_promo(&body) {
        Ok(p) => p,
        Err(why) => return Response::error(why, 400),
    };
    let code = p.code.clone();
    let stored = p.to_json();
    crate::hubstore::with_catalog(&place, move |cat| {
        cat.set_promo(&code, &stored);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "code": p.code }))
}

/// `POST /api/owner/promotions/:code/delete?location_id=` — a real delete.
///
/// Distinct from the active switch: switching off is reversible and keeps the
/// dates, deleting frees the word.
pub async fn delete_promotion(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(code) = ctx.param("code").cloned() else {
        return Response::error("missing code", 400);
    };
    let code = dowiz_hub::promo::normalise(&code);
    let gone = crate::hubstore::with_catalog(&place, move |cat| Ok(cat.remove_promo(&code))).await?;
    if !gone {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}
