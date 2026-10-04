//! `GET /api/public/locations/:slug/menu/week` -- the storefront's "most
//! ordered this week: N" (W-MR0, contract `menu.week-top.v1`,
//! tools/live-proof/contracts/feature-menu-flags.json).
//!
//! ORCHESTRATION ONLY: the venue's object folds its hot log with the pure
//! `week_top::public` and only the badged dishes cross. Public by design: it
//! names dishes and plate counts, never a person, and only dishes at or over
//! the threshold. Edge-cached like the menu, for longer: a weekly count moved
//! by a few plates is not news within five minutes.

use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// Seconds the edge and the browser may keep the answer.
pub const MAX_AGE_S: i64 = 300;

pub async fn week(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let key = req.url()?.to_string();
    if let Some(hit) = crate::edge::cache_get(&ctx.env, &key).await? {
        return Ok(hit);
    }
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let ask = format!("https://hub/fold/week_top?venue={}&now={}", crate::mcp::enc(&place.venue), ctx.data.now_ms);
    let (status, text) = crate::fold::ask::text(&place, &ask).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    let mut res = Response::ok(text)?;
    res.headers_mut().set("content-type", "application/json")?;
    res.headers_mut().set("cache-control", &format!("public, max-age={MAX_AGE_S}"))?;
    if let Ok(copy) = res.cloned() {
        let _ = crate::edge::cache_put(&ctx.env, &key, &copy).await;
    }
    Ok(res)
}

#[cfg(test)]
#[path = "week_route/tests.rs"]
mod tests;
