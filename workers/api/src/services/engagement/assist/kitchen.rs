//! The kitchen's assistant: a question about the pass, the menu and the shelf.
//!
//! OPERATOR Q9 (2026-09-26): agent support for the kitchen, a chat inside the
//! hub. The owner's assistant is shown live orders WITH their money and a
//! graph of the whole venue; the kitchen's is shown the TICKET (the same
//! whitelist the board reads, `kitchen_ack::board::ticket`), what is off the
//! menu, and the shelf. No customer, no price, no courier -- a kitchen-facing
//! surface carries no personal data, and a hosted model is a processor the
//! customer was never told about.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::services::orders::kitchen_ack::board;

/// The kitchen's system prompt. The facts are the truth; the model is not.
pub const SYSTEM_KITCHEN: &str = "\
You help the kitchen of one restaurant read its own open tickets, its menu and its stock. \
The FACTS block below is the truth; it was computed by the system, not by you. \
Never invent a dish, a quantity, a ticket or a time that is not in it. \
If the answer is not in the FACTS, say you do not have it. \
Answer in the language the question was asked in, in one to three short lines: \
it is read at the pass, with wet hands.";

/// Who may ask: anyone who works the pass, the menu or the shelf.
pub const ASKERS: [Cap; 3] = [Cap::Advance, Cap::Catalog, Cap::Stock];

/// One open ticket as the kitchen's assistant sees it.
fn ticket_fact(t: &Value, now: i64) -> Value {
    let created = t.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
    let items: Vec<Value> = t
        .get("items")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|l| json!({
            "dish": l.get("name").cloned().unwrap_or(Value::Null),
            "qty": l.get("quantity").cloned().unwrap_or(Value::Null),
            "station": l.get("station").and_then(Value::as_str).unwrap_or("kitchen"),
        })).collect())
        .unwrap_or_default();
    json!({
        "ticket": t.get("id").cloned().unwrap_or(Value::Null),
        "status": t.get("status").cloned().unwrap_or(Value::Null),
        "waiting_minutes": (now - created).max(0) / 60_000,
        "table": t.pointer("/fulfilment/table").cloned().unwrap_or(Value::Null),
        "kind": t.pointer("/fulfilment/kind").cloned().unwrap_or(Value::Null),
        "items": items,
    })
}

/// EVERYTHING THE KITCHEN'S ASSISTANT IS SHOWN, as one value: a test of this
/// is a test of the payload a model receives.
///
/// `orders` and `products`/`supplies` are `(id, json)` as the hub stores them;
/// `shelf` is `(item, on_hand, reserved)`.
pub fn kitchen_facts(
    orders: &[(String, String)],
    products: &[(String, String)],
    supplies: &[(String, String)],
    shelf: &[(String, i64, i64)],
    loc: &str,
    now: i64,
) -> Value {
    let tickets: Vec<Value> = board::board(orders, loc).iter().map(|t| ticket_fact(t, now)).collect();
    let off: Vec<Value> = products
        .iter()
        .filter_map(|(_, j)| {
            let v: Value = serde_json::from_str(j).ok()?;
            (!v.get("available").and_then(Value::as_bool).unwrap_or(true)).then(|| json!({
                "dish": v.get("name").cloned().unwrap_or(Value::Null),
                "why": v.get("unavailableNote").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect();
    let name_of = |id: &str| -> Value {
        supplies
            .iter()
            .find(|(s, _)| s == id)
            .and_then(|(_, j)| serde_json::from_str::<Value>(j).ok())
            .and_then(|v| v.get("name").cloned())
            .unwrap_or_else(|| json!(id))
    };
    let stock: Vec<Value> = shelf
        .iter()
        .map(|(item, on_hand, reserved)| json!({
            "ingredient": name_of(item), "id": item, "on_hand": on_hand,
            "reserved": reserved, "available": (on_hand - reserved).max(0),
        }))
        .collect();
    json!({ "now_ms": now, "open_tickets": tickets, "off_the_menu": off, "shelf": stock })
}

#[derive(Deserialize)]
struct AskIn {
    question: String,
}

/// `POST /api/staff/assist` — `{question}`, venue from `?location_id=` or the
/// token. The kitchen's facts, never the owner's.
pub async fn kitchen_assist(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &ASKERS).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let (listed, cat, stock) = futures_util::future::join3(
        crate::hubstore::orders(&place),
        crate::hubstore::load_catalog(&place),
        crate::hubstore::load_stock(&place),
    )
    .await;
    let orders: Vec<(String, String)> = listed?.into_iter().map(|o| (o.order_id, o.order_json)).collect();
    let cat = cat?.catalog;
    let shelf: Vec<(String, i64, i64)> = stock?
        .stock
        .ledger()
        .map(|l| l.items().into_iter().map(|(i, lv)| (i, lv.on_hand, lv.reserved)).collect())
        .unwrap_or_default();
    let facts = kitchen_facts(&orders, &cat.products(), &cat.supplies(), &shelf, &loc, ctx.data.now_ms);
    crate::assist::ask(&place, SYSTEM_KITCHEN, facts, &body.question).await
}

#[cfg(test)]
mod tests;
