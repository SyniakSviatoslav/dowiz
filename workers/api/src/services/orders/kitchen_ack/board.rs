//! What the kitchen may see of an order: the TICKET, and nothing about the
//! customer or the money.
//!
//! THE GAP THIS CLOSES (docs/research/2026-09-26-kitchen-role.md §1.2). A
//! kitchen token could WRITE to orders it could not READ -- both list routes
//! asked for `take_orders` or an owner -- and the one read it had, the live
//! socket, was the console's: every order with the customer's name, phone and
//! delivery address. A cook at the pass needs the dishes, the table, the time.
//!
//! STRIPPED AT THE SOURCE, BY A WHITELIST. `ticket` copies the fields a pass
//! uses and nothing else, so a field somebody adds to the order tomorrow
//! (a loyalty id, a second phone) is absent here until somebody decides the
//! kitchen should have it. A blacklist would have leaked it on the day it was
//! written. The same function strips a whole order and a delta (`_d`), so the
//! list, the `?since=` catch-up and the socket frame are one contract.

use serde_json::{json, Map, Value};
use worker::*;

use crate::auth::Cap;

/// The socket audience for kitchen principals (`live.rs`). A kitchen socket
/// is sent `frame()` -- the stripped delta -- and never the console's.
pub const TAG_KITCHEN: &str = "kitchen";

/// The statuses a pass works: the order is the kitchen's until it is READY
/// and handed over. The kernel's own words (`vocabulary.sh`).
pub const OPEN: [&str; 4] = ["PENDING", "CONFIRMED", "PREPARING", "READY"];

/// Top-level fields a ticket carries.
const KEEP: [&str; 12] = [
    "id", "status", "location_id", "created_at_ms", "scheduled_for_ms", "items", "fulfilment",
    "kitchen", "at", "channel", "rejection_reason", "sitting_id",
];
/// Of `fulfilment`: how it leaves and where it goes IN THE ROOM. The address
/// and the delivery fee stay behind.
const KEEP_FULFILMENT: [&str; 3] = ["kind", "table", "note"];
/// Of a line: what to cook. The price and the tax rate stay behind.
const KEEP_LINE: [&str; 6] = ["product_id", "name", "quantity", "modifier_ids", "station", "note"];

fn pick(v: &Value, keep: &[&str]) -> Value {
    let Some(o) = v.as_object() else { return Value::Null };
    let mut out = Map::new();
    for k in keep {
        if let Some(x) = o.get(*k) {
            out.insert((*k).to_string(), x.clone());
        }
    }
    Value::Object(out)
}

/// One order -- or one delta of one -- as the kitchen may see it.
pub fn ticket(v: &Value) -> Value {
    let Some(o) = v.as_object() else { return json!({}) };
    let mut out = Map::new();
    for k in KEEP {
        let Some(x) = o.get(k) else { continue };
        let kept = match k {
            "items" => Value::Array(
                x.as_array().map(|a| a.iter().map(|l| pick(l, &KEEP_LINE)).collect()).unwrap_or_default(),
            ),
            "fulfilment" if x.is_object() => pick(x, &KEEP_FULFILMENT),
            _ => x.clone(),
        };
        out.insert(k.to_string(), kept);
    }
    // A DELTA STAYS A DELTA: its marker, and the removals that name a field
    // the kitchen is allowed to know about.
    if let Some(d) = o.get("_d") {
        out.insert("_d".into(), d.clone());
    }
    if let Some(x) = o.get("_x").and_then(Value::as_array) {
        let gone: Vec<Value> =
            x.iter().filter(|k| k.as_str().is_some_and(|k| KEEP.contains(&k))).cloned().collect();
        out.insert("_x".into(), Value::Array(gone));
    }
    Value::Object(out)
}

/// A stored payload (JSON text), stripped. Unreadable text is an empty delta,
/// never the original bytes.
pub fn ticket_text(payload: &str) -> String {
    match serde_json::from_str::<Value>(payload) {
        Ok(v) => ticket(&v).to_string(),
        Err(_) => json!({ "_d": 1 }).to_string(),
    }
}

/// Is this order on the pass?
pub fn is_open(v: &Value) -> bool {
    v.get("status").and_then(Value::as_str).is_some_and(|s| OPEN.contains(&s))
}

/// The socket message a kitchen socket receives for one order event: the
/// console's frame with the payload stripped. `hubdo::broadcast` sends this to
/// `TAG_KITCHEN` beside the console's own frame.
pub fn frame(kind: u8, order_id: &str, payload: &str, generation: i64) -> String {
    json!({
        "t": "event",
        "kind": kind,
        "orderId": order_id,
        "payload": ticket_text(payload),
        "generation": generation,
    })
    .to_string()
}

/// The open tickets of one venue, oldest first -- the order a pass reads.
pub fn board(orders: &[(String, String)], venue: &str) -> Vec<Value> {
    let mut out: Vec<Value> = orders
        .iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(j).ok()?;
            if v.get("location_id").and_then(Value::as_str) != Some(venue) || !is_open(&v) {
                return None;
            }
            let mut t = ticket(&v);
            t["id"] = json!(id);
            Some(t)
        })
        .collect();
    out.sort_by_key(|t| t.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0));
    out
}

/// The `?since=` catch-up, stripped and fenced to one venue. A change naming
/// ANOTHER venue is dropped; one naming none is about an order the client
/// already holds (the owner route's rule).
pub fn changes(changes: &[crate::hubdo::Change], venue: &str) -> Vec<Value> {
    changes
        .iter()
        .filter(|c| {
            serde_json::from_str::<Value>(&c.payload)
                .ok()
                .and_then(|v| v.get("location_id").and_then(Value::as_str).map(String::from))
                .is_none_or(|l| l == venue)
        })
        .map(|c| json!({
            "generation": c.generation,
            "kind": c.kind,
            "order_id": c.order_id,
            "payload": ticket_text(&c.payload),
        }))
        .collect()
}

/// `GET /api/staff/kitchen?location_id=&since=` -- the pass's tickets.
///
/// `Cap::Advance` (the kitchen, or an owner through `staff_at`). No catalogue
/// read: the dish names ride on the lines, and the 538 KB catalogue is what
/// trips the 10 ms CPU kill at dinner (FREE-TIER FT1).
pub async fn kitchen_orders(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(loc) = crate::services::orders::room::handlers::query_location(&req) else {
        return Response::error("location_id is required", 400);
    };
    if let Err(r) = crate::courier::staff_at(&req, &ctx, &loc, Cap::Advance).await {
        return Ok(r);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let since = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "since")
        .and_then(|(_, v)| v.parse::<i64>().ok());
    if let Some(since) = since {
        if let Ok((generation, Some(ch))) = crate::hubstore::changes_since(&place, since).await {
            return Response::from_json(&json!({
                "generation": generation, "changes": changes(&ch, &loc), "full": false,
            }));
        }
    }
    let (generation, listed) = crate::hubstore::orders_at(&place).await?;
    let pairs: Vec<(String, String)> = listed.into_iter().map(|o| (o.order_id, o.order_json)).collect();
    let mut res = Response::from_json(&json!({
        "orders": board(&pairs, &loc), "generation": generation, "full": true,
    }))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

#[cfg(test)]
mod tests;
