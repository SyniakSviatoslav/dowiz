//! The owner's link and unlink (§3.4). ORCHESTRATION ONLY: the rules are
//! `alias::{may_link, check_link, link, unlink, audit}`, tested natively.
//!
//! THE AUDIT IS WRITTEN BEFORE THE LINK, for the reason `reveal_customer`
//! gives: an un-audited link is the one thing this route must not do. A link
//! refused after its audit (another tab linked first) leaves an audit of an
//! attempt, which over-reports and never hides.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use super::alias::{self, By};
use super::record_routes::is_key;
use crate::hubstore::{Place, IMAGE_PEOPLE, PEOPLE_BYTES};

/// The event an owner's link is audited under. `EventKind::Linked` does not
/// exist in `dowiz_hub` yet (HAND-BACK); until it does the entry is a
/// `Revealed` -- the shape §3.4 asks for, and a link IS a reveal of both --
/// carrying `"act": "linked"`.
const LINKED: dowiz_hub::EventKind = dowiz_hub::EventKind::Revealed;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkIn {
    to: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnlinkIn {
    reason: String,
}

/// The owner, their venue and the path's key -- or the answer that refuses.
async fn authorised(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    reason: &str,
) -> std::result::Result<(String, Place, String), Response> {
    let claims = crate::auth::bearer(req)
        .and_then(|raw| crate::auth::verify(&ctx.env, &raw, ctx.data.now_ms))
        .map_err(|e| e.into_response().unwrap())?;
    if !alias::may_link(&claims) {
        return Err(Response::error("only the owner links customers", 403).unwrap());
    }
    let (who, loc) = crate::owner::owner_and_venue(req, ctx).await?;
    let place = Place::of_authorised(ctx, &loc).map_err(|e| Response::error(e.to_string(), 500).unwrap())?;
    if reason.trim().len() < 3 {
        return Err(Response::error("say why", 400).unwrap());
    }
    match ctx.param("key").cloned().filter(|k| is_key(k)) {
        Some(key) => Ok((who, place, key)),
        None => Err(Response::error("not a customer key", 400).unwrap()),
    }
}

async fn write_audit(place: &Place, entries: [(String, String); 2], now: i64) -> Result<()> {
    for (subject, entry) in entries {
        crate::hubstore::append_blind(place, LINKED, &subject, &entry, now).await?;
    }
    Ok(())
}

/// `POST /api/owner/customers/:key/link?location_id=` `{to, reason}` -- show
/// `:key`'s orders under `to`. Nothing moves; `unlink` undoes it.
pub async fn link(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: LinkIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (who, place, key) = match authorised(&req, &ctx, &body.reason).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let to = body.to.trim().to_string();
    if !is_key(&to) {
        return Response::error("not a customer key", 400);
    }
    let reason = body.reason.trim().to_string();
    let people = crate::hubstore::load_table(&place, IMAGE_PEOPLE, PEOPLE_BYTES).await?;
    if let Err(why) = alias::check_link(&people.table, &key, &to) {
        return Response::error(why, 409);
    }
    let now = ctx.data.now_ms;
    write_audit(&place, alias::audit("linked", &who, now, &reason, &key, &to), now).await?;
    let wrote = crate::hubstore::with_table(&place, IMAGE_PEOPLE, PEOPLE_BYTES, |t| {
        Ok(alias::link(t, &key, &to, By::Owner, &reason, now))
    })
    .await?;
    match wrote {
        Ok(()) => Response::from_json(&json!({ "key": key, "canonical": to })),
        Err(why) => Response::error(why, 409),
    }
}

/// `POST /api/owner/customers/:key/unlink?location_id=` `{reason}` -- the two
/// rows come back. A rule-written link is unlinked the same way, and the next
/// order does not relink it (`alias::rule_link`).
pub async fn unlink(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: UnlinkIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (who, place, key) = match authorised(&req, &ctx, &body.reason).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let people = crate::hubstore::load_table(&place, IMAGE_PEOPLE, PEOPLE_BYTES).await?;
    let Some(to) = people.table.get(alias::KIND, &key).and_then(|j| {
        serde_json::from_str::<serde_json::Value>(&j).ok()?.get("canonical")?.as_str().map(str::to_string)
    }) else {
        return Response::error("not linked", 404);
    };
    let now = ctx.data.now_ms;
    write_audit(&place, alias::audit("unlinked", &who, now, body.reason.trim(), &key, &to), now).await?;
    let removed =
        crate::hubstore::with_table(&place, IMAGE_PEOPLE, PEOPLE_BYTES, |t| Ok(alias::unlink(t, &key))).await?;
    Response::from_json(&json!({ "key": key, "unlinked": removed }))
}
