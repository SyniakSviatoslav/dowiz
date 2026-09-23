//! Lines between rounds, and a sitting to another table (BLUEPRINT-POS-THE-ROOM
//! §2.9). The same shape as every room handler (`handlers.rs`): authorise the
//! signer, read the catalogue for the shelf, send ONE command to the object.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use super::handlers::recipes;
use crate::auth::Cap;
use crate::command::transfer::sitting::{MoveIn, MoveOut};
use crate::command::transfer::{TransferIn, TransferOut};

#[derive(Deserialize)]
struct TransferBody {
    location_id: String,
    to_order_id: String,
    from_base_seq: u64,
    to_base_seq: u64,
    lines: Vec<usize>,
}

/// `POST /api/staff/orders/:id/transfer` —
/// `{location_id, to_order_id, from_base_seq, to_base_seq, lines}`.
pub async fn transfer(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: TransferBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(from_id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, _) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakeOrders).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place, req.headers().get("idempotency-key").ok().flatten(), &by,
        "staff.transfer", &format!("{from_id}:{raw}"), ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let loaded = crate::hubstore::load_catalog(&place).await?;
    let input = TransferIn {
        from_order_id: from_id,
        to_order_id: body.to_order_id,
        location_id: body.location_id,
        from_base_seq: body.from_base_seq,
        to_base_seq: body.to_base_seq,
        lines: body.lines,
        by,
        boms: recipes(&loaded.catalog),
        now_ms: ctx.data.now_ms,
    };
    let out: TransferOut = match crate::command::send(&place, "room/transfer", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = json!({
        "from": { "order": serde_json::from_str::<Value>(&out.from_merged).unwrap_or(Value::Null), "seq": out.from_seq },
        "to": { "order": serde_json::from_str::<Value>(&out.to_merged).unwrap_or(Value::Null), "seq": out.to_seq },
    });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}

#[derive(Deserialize)]
struct MoveBody {
    location_id: String,
    table: String,
}

/// `POST /api/staff/sittings/:id/move` — `{location_id, table}`: every round
/// of the sitting still in the room goes to `table`.
pub async fn move_sitting(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: MoveBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(sitting_id) = ctx.param("id").cloned() else {
        return Response::error("missing sitting id", 400);
    };
    let (by, _) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakeOrders).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place, req.headers().get("idempotency-key").ok().flatten(), &by,
        "staff.move_sitting", &format!("{sitting_id}:{raw}"), ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let input = MoveIn { sitting_id, location_id: body.location_id, table: body.table, by, now_ms: ctx.data.now_ms };
    let out: MoveOut = match crate::command::send(&place, "room/move_sitting", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = json!({ "sitting_id": out.sitting_id, "table": out.table, "moved": out.moved });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}
