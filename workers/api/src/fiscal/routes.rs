//! THE OWNER'S FISCAL ROUTES (card L70). Authenticated as every owner route is
//! (`owner_and_venue`), acting on the venue the caller was authorised for and
//! no other (`Place::of_authorised`); the state is read server-side.
//!
//!   GET  /api/owner/fiscal               the pane: armed or why not, the queue, each order's stage
//!   POST /api/owner/fiscal/ebills        {armed?, confirm?, sale_unit?, fee_item?, cancel_armed?}
//!   GET  /api/owner/orders/:id/receipt   the art. 29 receipt text, with the codes once registered
//!
//! ARMING IS A LEGAL ACT, and this is the only door to it: the four keys are
//! not declared settings (`ebills_arm.rs` says why), and arming without the
//! typed confirmation is refused here, before the settings image is touched.

use super::ebills_arm::{arm, ArmIn};
use crate::hubstore::Place;
use crate::owner::owner_and_venue;
use serde_json::{json, Value};
use worker::*;

async fn place_of(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<std::result::Result<Place, Response>> {
    match owner_and_venue(req, ctx).await {
        Ok((_, loc)) => Ok(Ok(Place::of_authorised(ctx, &loc)?)),
        Err(r) => Ok(Err(r)),
    }
}

async fn command(place: &Place, what: &str, input: &Value) -> Result<Response> {
    match crate::command::send::<_, Value>(place, what, input).await {
        Ok(v) => Response::from_json(&v),
        Err((status, msg)) => Response::error(msg, status),
    }
}

/// `GET /api/owner/fiscal`
pub async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    // THE PLATFORM SWITCH TRAVELS WITH THE STATUS (W-WIRE row 6), so the
    // console says "sending is off for every venue" from the build's own
    // constant rather than from a sentence somebody has to keep true.
    match crate::command::send::<_, Value>(&place, "ebills/fiscal_status", &json!({ "now_ms": ctx.data.now_ms })).await {
        Ok(v) => Response::from_json(&with_switch(v, super::SEND_ENABLED)),
        Err((status, msg)) => Response::error(msg, status),
    }
}

/// The status with the platform's send switch beside it. PURE.
pub fn with_switch(mut v: Value, send_enabled: bool) -> Value {
    if let Some(m) = v.as_object_mut() {
        m.insert("sendEnabled".into(), Value::Bool(send_enabled));
    }
    v
}

/// `POST /api/owner/fiscal/ebills`
pub async fn set(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    if !super::SEND_ENABLED {
        return Response::error("sending invoices to ebills is switched off for the platform; the till import (ebills -> dowiz) is unaffected", 409);
    }
    let body: ArmIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let writes = match arm(&body) {
        Ok(w) => w,
        Err(why) => return Response::error(why, 400),
    };
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let keys: Vec<&str> = writes.iter().map(|(k, _)| *k).collect();
    crate::hubstore::with_settings(&place, move |s| {
        for (k, v) in &writes {
            if v.is_empty() {
                s.clear(k);
            } else {
                s.set(k, v);
            }
        }
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "written": keys }))
}

/// `GET /api/owner/orders/:id/receipt`
pub async fn receipt(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else { return Response::error("no order named", 400) };
    let place = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    command(&place, "ebills/fiscal_receipt", &json!({ "order_id": id })).await
}

#[cfg(test)]
mod switch_tests {
    use super::with_switch;
    use serde_json::json;

    /// The console reads the platform switch from the status, never from a sentence.
    #[test]
    fn the_status_carries_the_platform_send_switch() {
        assert_eq!(with_switch(json!({ "armed": true }), false), json!({ "armed": true, "sendEnabled": false }));
        assert_eq!(with_switch(json!({}), true)["sendEnabled"], true);
        assert_eq!(with_switch(json!("odd"), false), json!("odd"), "a non-object answer passes through");
        assert!(!crate::fiscal::SEND_ENABLED, "operator 2026-09-24: import-only");
    }
}
