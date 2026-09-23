//! Refund an order — the owner, or staff holding `void` (A10).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::refund::returned::{ReturnedIn, ReturnedOut};
use crate::command::refund::{RefundIn, RefundOut};

#[derive(Deserialize)]
struct RefundBody {
    location_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/staff/orders/:id/refund` — `{location_id, reason}` starts a
/// refund; `{location_id, complete: true}` records the money handed back.
/// The signer is the authenticated principal, never the body.
pub async fn refund(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: RefundBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::Void).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.refund",
        &format!("{id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let input = RefundIn {
        order_id: id,
        location_id: body.location_id,
        by,
        reason: body.reason,
        complete: body.complete,
        now_ms: ctx.data.now_ms,
        at_door: false,
        note: body.note,
    };
    let out: RefundOut = match crate::command::send(&place, "refund", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = json!({ "order": serde_json::from_str::<Value>(&out.merged).unwrap_or(Value::Null), "seq": out.seq });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}

#[derive(Deserialize)]
struct ReturnedBody {
    location_id: String,
    choice: String,
}

/// `POST /api/staff/orders/:id/returned` — `{location_id, choice: "resell"|"waste"}`:
/// the food a refused delivery brought back, chosen once (§2.4). The same door
/// as the refund; the signer is the authenticated principal.
pub async fn returned(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: ReturnedBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::Void).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.returned",
        &format!("{id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let input = ReturnedIn { order_id: id, location_id: body.location_id, by, choice: body.choice, now_ms: ctx.data.now_ms };
    let out: ReturnedOut = match crate::command::send(&place, "returned", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = serde_json::to_value(&out).unwrap_or(Value::Null);
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}
