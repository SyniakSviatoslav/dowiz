//! THE OWNER'S WINDOW ON THE PUBLISHED MENU (W-PUBUI, over BN2; operator rule
//! 2026-09-26: no backend without UI). The venue's object publishes the
//! storefront's read path to R2 on every menu write and answers
//! `/fold/publish` with what it holds (`hubdo/publish.rs`): `GET` is the
//! record and whether publishing is on, `POST[?all=1]` publishes now, and a
//! `503` says publishing is off while the `CDN` binding is absent.
//!
//! This file is the owner's door to that route and nothing more. It
//! authenticates the owner, names the one venue they are authorised for
//! (`owner_and_venue` -> `Place::of_authorised`, the one-venue rule), and
//! relays the object's answer -- status and body -- so the console shows the
//! object's own words. Nothing is decided here: the object decides what is
//! published and whether it can, and a handler that re-derived either would
//! be a second truth (`publish/tests.rs`).

use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::owner::owner_and_venue;

/// `GET /api/owner/publish`: the record (`published`), the manifest's key on
/// the CDN, and `enabled`.
pub async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    relay(req, ctx, Method::Get).await
}

/// `POST /api/owner/publish[?all=1]`: publish the current generation now;
/// `all` rewrites every object, not only what changed.
pub async fn publish_now(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    relay(req, ctx, Method::Post).await
}

/// One call of the object's `/fold/publish`, with the method the owner used
/// and the `all` flag if they set it. A refusal comes back with the object's
/// status and words; a 200 as the JSON it already is.
async fn relay(req: Request, ctx: RouteContext<crate::Req>, method: Method) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let all = req.url()?.query_pairs().any(|(k, _)| k == "all");
    let url = if all { "https://hub/fold/publish?all=1" } else { "https://hub/fold/publish" };
    let mut answer = place.stub()?.fetch_with_request(Request::new(url, method)?).await?;
    let (status, text) = (answer.status_code(), answer.text().await?);
    if status != 200 {
        return Response::error(text, status);
    }
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Error::RustError(format!("publish: unreadable answer: {e}")))?;
    Response::from_json(&body)
}

#[cfg(test)]
#[path = "publish/tests.rs"]
mod tests;
