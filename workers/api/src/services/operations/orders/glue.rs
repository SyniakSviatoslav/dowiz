//! SHEAF S2b: THE KITCHEN TABLET'S ORDER TAPS, SYNCED (R-BEBOP-LEVERS §3.4, AC-LV-LOC-03).
//!
//!   POST /api/staff/orders/glue   {location_id, ctx, tail:[op line, ...]}
//!
//! THE CONTRACT. Producer: the kitchen board's tablet module (`public/room/canvas/tablet.js`
//! `syncOrders`, over `bebop-wasm --features tablet` `bw_tab_order_tail`). Consumer: the venue's
//! object, `/fold/glue_orders` (`hubdo/glue_turn/orders.rs`), which runs
//! `dowiz_hub::glue::orders::rebase` with the object's own deciders (`command::advance`,
//! `command::kitchen_ack`). `tail` is the taps the tablet took since its last sync, OLDEST FIRST,
//! each one op line `{"op":"ack"|"advance","oid","order","from"?,"next"?,"at"}` and nothing else
//! (NO-TRACKING: any other key refuses the whole tail). The answer:
//! `{sent, merged, applied, held:[{pos, why}], notes}` with the law `sent == merged + held`;
//! each held tap is a decision on the kitchen's strip (`GET /api/staff/stock/decisions`).
//! A refused tail is 422 naming the line, and nothing is written. Version 1 (2026-10-28).
//!
//! THIS FILE ONLY AUTHENTICATES: the pass capability at ONE venue, and that venue's object is the
//! one asked (`Place::of_authorised`). The signer of every ack is the token's, never the body's.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::auth::Cap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    location_id: String,
    ctx: String,
    tail: Vec<String>,
}

/// The object's input (`/fold/glue_orders`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GlueOrdersIn {
    pub ctx: String,
    pub location_id: String,
    /// Who the token says synced: the signer of every ack in the tail.
    pub by: String,
    pub tail: Vec<String>,
    pub now_ms: i64,
}

/// `POST /api/staff/orders/glue`.
pub async fn glue(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: Body = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    if !crate::services::operations::stock::glue::ctx_ok(&body.ctx) {
        return Response::error("ctx: a tablet's name, letters, digits, - and _", 400);
    }
    let by = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::Advance).await {
        Ok((who, _)) => who,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let input = GlueOrdersIn { ctx: body.ctx, location_id: body.location_id, by, tail: body.tail, now_ms: ctx.data.now_ms };
    match crate::command::send::<_, Value>(&place, "glue_orders", &input).await {
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
