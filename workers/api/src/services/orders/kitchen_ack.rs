//! `POST /api/staff/orders/:id/kitchen-ack` — the kitchen says it SAW the
//! ticket (BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23 §2.6, P1-6).
//!
//! The rule and the fold are `command::kitchen_ack`; the write is the venue's
//! object (`hubdo/kitchen_ack.rs`, `/fold/kitchen_ack`). This file only
//! authenticates: `Cap::Advance` (the kitchen, or the owner doing the
//! kitchen's job), the venue resolved from the signer, and the route-level
//! idempotency guard every `order_action` has, so a retried tap with the same
//! key replays its first answer.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;

#[derive(Deserialize)]
struct KitchenAckBody {
    location_id: String,
}

/// `POST /api/staff/orders/:id/kitchen-ack` — `{location_id}`.
/// Staff with Cap::Advance (the kitchen) records that they saw the order.
pub async fn kitchen_ack(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: KitchenAckBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::Advance).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.kitchen_ack",
        &format!("{id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };

    let input = crate::command::kitchen_ack::KitchenAckIn {
        order_id: id,
        location_id: body.location_id,
        by,
        now_ms: ctx.data.now_ms,
    };
    let out: crate::command::kitchen_ack::KitchenAckOut = match crate::command::send(&place, "kitchen_ack", &input).await {
        Ok(v) => v,
        Err((status, said)) => return idem.refused(&place, status, &said).await,
    };
    let answer = json!({
        "order": serde_json::from_str::<Value>(&out.merged).unwrap_or(Value::Null),
        "seq": out.seq,
        // False: the kitchen had already said so, and nothing was written.
        "fresh": out.fresh,
    });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kitchen_ack_body_parses() {
        let raw = r#"{"location_id":"venue_1"}"#;
        let body: KitchenAckBody = serde_json::from_str(raw).expect("should parse");
        assert_eq!(body.location_id, "venue_1");
    }

    #[test]
    fn kitchen_ack_body_missing_location_id_fails() {
        let raw = r#"{}"#;
        let result: Result<KitchenAckBody, _> = serde_json::from_str(raw);
        assert!(result.is_err());
    }
}
