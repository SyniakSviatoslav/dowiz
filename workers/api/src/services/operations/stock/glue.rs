//! SHEAF S2: THE KITCHEN TABLET'S SYNC, and the "decision needed" list (R-BEBOPDB G.5 row S2).
//!
//!   POST /api/staff/stock/glue              {location_id, ctx, base, tip, tail:[record, ...]}
//!   GET  /api/staff/stock/decisions?location_id=
//!   POST /api/staff/stock/decisions/close   {location_id, did}
//!
//! THE CONTRACT. Producer: the kitchen board's tablet module (`public/room/canvas/tablet.js`,
//! over the wasm writer `bebop-wasm --features tablet`). Consumer: the venue's object, `/fold/glue`
//! and `/fold/desk` (`hubdo/glue_turn.rs`), which run `dowiz_hub::glue::{merge, desk}`.
//! `base` and `tip` are 64 hex characters (a record id); `tail` is the tablet's records above
//! `base`, OLDEST FIRST, each the JSON text it chained. The answer:
//! `{tip, tail:[record | index, ...], merged, held:[{pos, why}], conflicts:[{kind, item, ..}],
//!   grants, replay, wire_up, wire_down}` -- `tail` is everything the hub holds above `base`,
//! an index meaning "your own record at this position, adopted as written". A refusal is 422
//! with the glue's reason (`Tracking`, `TipMismatch`, ...), 409 when the hub no longer holds
//! `base` (the tablet starts over from an all-zero base), nothing written either way.
//!
//! THIS FILE ONLY AUTHENTICATES: the stock or the pass capability at ONE venue, and that venue's
//! object is the one asked (`Place::of_authorised`, memory dowiz-tenant-isolation-hub-image).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::auth::Cap;

/// What may open the tablet's sync and the list: the shelf, or the pass.
const NEEDS: [Cap; 2] = [Cap::Stock, Cap::Advance];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlueBody {
    location_id: String,
    ctx: String,
    base: String,
    tip: String,
    tail: Vec<String>,
}

/// The object's input (`/fold/glue`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GlueIn {
    pub ctx: String,
    pub base: String,
    pub tip: String,
    pub tail: Vec<String>,
    pub now_ms: i64,
}

/// The object's input (`/fold/desk`): close `did` when given, then the open list.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeskIn {
    pub did: Option<String>,
    pub by: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseBody {
    location_id: String,
    did: String,
}

/// A context's name: what a tablet calls itself in its records (`ctx`). Short, plain.
pub(crate) fn ctx_ok(c: &str) -> bool {
    !c.is_empty() && c.len() <= 32 && c != dowiz_hub::stock::rights::HUB && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `POST /api/staff/stock/glue`.
pub async fn glue(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: GlueBody = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    if !ctx_ok(&body.ctx) {
        return Response::error("ctx: a tablet's name, letters, digits, - and _", 400);
    }
    if let Err(r) = crate::courier::staff_any_at(&req, &ctx, &body.location_id, &NEEDS).await {
        return Ok(r);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let input = GlueIn { ctx: body.ctx, base: body.base, tip: body.tip, tail: body.tail, now_ms: ctx.data.now_ms };
    match crate::command::send::<_, Value>(&place, "glue", &input).await {
        Ok(v) => Response::from_json(&v),
        Err((status, said)) => Response::error(said, status),
    }
}

/// `GET /api/staff/stock/decisions?location_id=`.
pub async fn decisions(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(loc) = crate::services::orders::room::handlers::query_location(&req) else {
        return Response::error("location_id is required", 400);
    };
    let (by, _) = match crate::courier::staff_any_at(&req, &ctx, &loc, &NEEDS).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    desk(&ctx, &loc, DeskIn { did: None, by }).await
}

/// `POST /api/staff/stock/decisions/close`.
pub async fn close(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: CloseBody = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let (by, _) = match crate::courier::staff_any_at(&req, &ctx, &body.location_id, &NEEDS).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    desk(&ctx, &body.location_id, DeskIn { did: Some(body.did), by }).await
}

async fn desk(ctx: &RouteContext<crate::Req>, loc: &str, input: DeskIn) -> Result<Response> {
    let place = crate::hubstore::Place::of_authorised(ctx, loc)?;
    match crate::command::send::<_, Value>(&place, "desk", &input).await {
        Ok(v) => {
            let mut res = Response::from_json(&v)?;
            res.headers_mut().set("cache-control", "private, no-store")?;
            Ok(res)
        }
        Err((status, said)) => Response::error(said, status),
    }
}

#[cfg(test)]
#[path = "glue/tests.rs"]
mod tests;
