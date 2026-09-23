//! Take a payment on a round.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::pay::{PayIn, PayOut};

#[derive(Deserialize)]
struct PayBody {
    location_id: String,
    amount: i64,
    method: String,
    #[serde(default)]
    till_id: Option<String>,
    #[serde(default)]
    covers: Option<Vec<String>>,
    /// The currency handed over; absent is the order's (P3-2).
    #[serde(default)]
    currency: Option<String>,
    /// Required for a foreign currency: order minor units per payment minor
    /// unit × 1 000 000 (`command::pay::fx`).
    #[serde(default)]
    rate_ppm: Option<i64>,
}

/// `POST /api/staff/orders/:id/pay` — `{location_id, amount, method, till_id?, covers?, currency?, rate_ppm?}`.
/// Cash is refused with no till open (409 "open the till first").
pub async fn pay(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: PayBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakePayment).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.pay",
        &format!("{id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let input = PayIn {
        order_id: id,
        location_id: body.location_id,
        amount: body.amount,
        method: body.method,
        by,
        till_id: body.till_id,
        covers: body.covers,
        currency: body.currency,
        rate_ppm: body.rate_ppm,
        now_ms: ctx.data.now_ms,
    };
    let out: PayOut = match crate::command::send(&place, "room/pay", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = json!({ "order": serde_json::from_str::<Value>(&out.merged).unwrap_or(Value::Null), "seq": out.seq });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}
