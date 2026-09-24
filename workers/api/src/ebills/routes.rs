//! THE OWNER'S TILL-LINK ROUTES. Authenticated as every owner route is
//! (`owner_and_venue`), acting on the venue the caller was authorised for and
//! no other (`Place::of_authorised`), and each one a command the venue's
//! object executes (`hubdo/ebills.rs`).
//!
//!   GET  /api/owner/ebills         the link, the poller, the crosswalk, the floor
//!   POST /api/owner/ebills/config  {enabled, pos_id, user, password?}
//!   POST /api/owner/ebills/map     {code, product_id?}  -- empty clears
//!
//! A MAPPING IS ONLY EVER SET HERE, BY THE OWNER. The status answer carries
//! SUGGESTIONS (a normalised name match, and whether the price agrees); none
//! of them is applied by anything.

use super::cmd::{ConfigIn, MapIn};
use crate::owner::owner_and_venue;
use crate::hubstore::Place;
use serde_json::Value;
use worker::*;

/// The venue the caller owns, as its object; `Err` is the refusal to send.
async fn place_of(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<std::result::Result<Place, Response>> {
    match owner_and_venue(req, ctx).await {
        Ok((_, loc)) => Ok(Ok(Place::of_authorised(ctx, &loc)?)),
        Err(r) => Ok(Err(r)),
    }
}

async fn command<I: serde::Serialize>(place: &Place, what: &str, input: &I) -> Result<Response> {
    match crate::command::send::<_, Value>(place, what, input).await {
        Ok(v) => Response::from_json(&v),
        Err((status, msg)) => Response::error(msg, status),
    }
}

/// `GET /api/owner/ebills`
pub(crate) async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    command(&place, "ebills/status", &serde_json::json!({})).await
}

/// `POST /api/owner/ebills/config`
pub(crate) async fn config(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ConfigIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    command(&place, "ebills/config", &body).await
}

/// `POST /api/owner/ebills/map`
pub(crate) async fn map(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let mut body: MapIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    body.now_ms = ctx.data.now_ms;
    command(&place, "ebills/map", &body).await
}

/// The health lines (`/api/owner/health`): the link's state in one small
/// object, or the reason it could not be read -- never an empty "fine".
pub(crate) async fn health(place: &Place) -> Value {
    match crate::command::send::<_, Value>(place, "ebills/status", &serde_json::json!({})).await {
        Ok(v) => v["health"].clone(),
        Err((s, m)) => serde_json::json!({ "error": format!("{s}: {m}") }),
    }
}
