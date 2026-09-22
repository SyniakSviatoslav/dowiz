//! `GET /api/courier/history` — what this courier has finished.

use serde_json::{json, Value};
use worker::*;

use crate::owner::now_ms;

/// `GET /api/courier/history` — what this courier has finished.
///
/// A fold over the orders, like everything else that counts. No history table:
/// a second list of the same deliveries is a second thing that can disagree.
pub async fn courier_history(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let me = match crate::auth::authenticate(&req, &ctx.env, now_ms()).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let mut rows: Vec<Value> = crate::hubstore::orders(&place)
        .await?
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        // FINISHED IS THE KERNEL'S WORD FOR IT. This spelled out
        // `DELIVERED | CANCELLED | REJECTED` -- the fifteenth hand-written
        // copy of that list in this tree, and one of the ones that did not
        // know about `COMPENSATED_REFUND`, so a refunded run would have
        // vanished from the courier's own record of their day.
        .filter(|o| {
            crate::services::orders::status::is_terminal(
                o.get("status").and_then(Value::as_str).unwrap_or(""),
            )
        })
        .map(|o| {
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "at": o.get("created_at_ms").cloned().unwrap_or(json!(0)),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(json!(0)),
                "cashCollected": o.get("cash_collected").cloned().unwrap_or(json!(0)),
                "street": o.get("fulfilment").and_then(|f| f.get("address"))
                    .and_then(|a| a.get("line")).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    rows.sort_by(|a, b| b["at"].as_i64().cmp(&a["at"].as_i64()));
    rows.truncate(50);
    Response::from_json(&json!({ "history": rows }))
}
