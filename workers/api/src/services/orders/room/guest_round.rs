//! A GUEST'S ROUND, after it is placed (L66, A9).
//!
//! `POST /api/staff/orders/:id/guest` `{location_id, action: confirm|reject}` —
//! the room answers a round a guest placed from a table's code. Such a round is
//! born PENDING and unsigned by any person (`placer::GUEST`); a waiter with
//! `take_orders` confirms it into the kitchen or rejects it. ONLY a guest's
//! PENDING round: every other move stays the kitchen's (`owner::order_action`,
//! `Cap::Advance`), which a waiter does not hold.
//!
//! `GET /api/order/:id/sitting` — the whole sitting's bill, read-only, for the
//! guest holding a customer token minted with that sitting (`Claims::Customer
//! { sitting_id }`). No name, no phone, no signer: amounts, lines and statuses.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use super::placer::GUEST;
use crate::auth::{self, Cap, Claims};
use crate::command::{sitting, Refused};

/// PURE. What `action` moves this round to, or why the room may not.
pub fn answer(order: &Value, venue: &str, action: &str) -> std::result::Result<&'static str, Refused> {
    if order.get("location_id").and_then(Value::as_str) != Some(venue) {
        return Err(Refused::NotFound);
    }
    if order.get("placed_by").and_then(Value::as_str) != Some(GUEST) {
        return Err(Refused::Conflict("only a round a guest placed from the table waits for the room".into()));
    }
    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "PENDING" {
        return Err(Refused::Conflict(format!("this round is already {status}")));
    }
    match action {
        "confirm" => Ok("CONFIRMED"),
        "reject" => Ok("REJECTED"),
        other => Err(Refused::Invalid(format!("unknown action: {other}"))),
    }
}

/// PURE. The guest's read of the sitting: the card, without who signed what
/// and without the payment records.
pub fn guest_view(card: &Value) -> Value {
    let mut v = card.clone();
    if let Some(rounds) = v.get_mut("rounds").and_then(Value::as_array_mut) {
        for r in rounds {
            if let Some(o) = r.as_object_mut() {
                for k in ["placed_by", "payments", "amended", "adjustments", "seq"] {
                    o.remove(k);
                }
            }
        }
    }
    v
}

#[derive(Deserialize)]
struct AnswerIn {
    location_id: String,
    action: String,
}

/// `POST /api/staff/orders/:id/guest`
pub async fn confirm(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let raw = req.text().await.unwrap_or_default();
    let body: AnswerIn = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let by = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakeOrders).await {
        Ok((by, _)) => by,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.guest_round",
        &format!("{id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    // G1 / D1: every exit below is an ANSWER, recorded (or, for a 5xx or an
    // internal error, released) by `answered` -- never a claim left standing.
    let res: Result<Response> = async {
        let Some(order_json) = crate::hubstore::order(&place, &id).await? else {
            return Response::error("not found", 404);
        };
        let order: Value = serde_json::from_str(&order_json).unwrap_or(json!({}));
        let next = match answer(&order, &body.location_id, &body.action) {
            Ok(n) => n,
            Err(r) => return Response::error(r.message().to_string(), r.status()),
        };
        let input = crate::command::advance::AdvanceIn {
            order_id: id.clone(),
            location_id: body.location_id.clone(),
            next: next.to_string(),
            reason: Some(format!("guest round, answered by {by}")),
            now_ms: ctx.data.now_ms,
        };
        let out: crate::command::advance::AdvanceOut = match crate::command::send(&place, "advance", &input).await {
            Ok(v) => v,
            Err((status, said)) => return Response::error(said, status),
        };
        let mut res = Response::ok(out.merged)?;
        res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
        Ok(res)
    }
    .await;
    idem.answered(&place, res).await
}

/// `GET /api/order/:id/sitting`
pub async fn sitting_bill(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let claims = auth::bearer(&req).and_then(|t| auth::verify(&ctx.env, &t, ctx.data.now_ms));
    let (location_id, sitting_id) = match claims {
        Ok(Claims::Customer { order_id, location_id, sitting_id: Some(s), .. }) if order_id == id => (location_id, s),
        _ => return Response::error("this bill needs the link you were given at the table", 401),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &location_id)?;
    let listed: Vec<_> = crate::hubstore::orders(&place)
        .await?
        .into_iter()
        .filter(|o| {
            serde_json::from_str::<Value>(&o.order_json)
                .is_ok_and(|v| v.get("location_id").and_then(Value::as_str) == Some(location_id.as_str()))
        })
        .collect();
    let rounds = sitting::rounds(&listed, &sitting_id);
    // THE TOKEN'S ORDER MUST BE ONE OF THE SITTING'S ROUNDS: a round moved to
    // another table (`transfer`) takes its guest with it, not the old bill.
    if !rounds.iter().any(|r| r.view.order_id == id) {
        return Response::error("not found", 404);
    }
    let mut res = Response::from_json(&guest_view(&sitting::card(&sitting_id, &rounds)))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

#[cfg(test)]
mod tests;
