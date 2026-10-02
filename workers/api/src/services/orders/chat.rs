//! CUSTOMER <-> COURIER CHAT, per order (operator decisions 2026-10-02).
//!
//! WHY. A courier at the wrong gate and a customer who cannot be reached used
//! to need each other's phone number -- the one thing this platform tries not
//! to hand around. The chat opens when a courier is assigned, closes when the
//! order is delivered or cancelled, stays readable for thirty days and is then
//! erased with the order's personal data. The venue's owner may read it for a
//! dispute and never writes in it; the kitchen never sees it.
//!
//! WHO decides is `party::thread_party` (pure); WHAT is stored is `store`
//! (pure where it can be); this file sequences the two around the order the
//! object folds. The live socket is nudged after an append (`hubdo/chat.rs`),
//! and `?since=<ms>` is the poll for a client without one.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

pub mod party;
pub mod store;

use party::Side;

/// What the chat is doing, from the order alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// No courier yet: nobody to talk to.
    Waiting,
    Open,
    /// The order is over: read-only.
    Closed,
}

impl State {
    pub fn of(order: &Value) -> Self {
        let status = order.get("status").and_then(Value::as_str).unwrap_or("");
        if crate::services::orders::status::is_terminal(status) {
            Self::Closed
        } else if order.get("courier_id").and_then(Value::as_str).is_some_and(|c| !c.is_empty()) {
            Self::Open
        } else {
            Self::Waiting
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

/// The order and the caller's side, or the answer that ends the request.
async fn admit(req: &Request, ctx: &RouteContext<crate::Req>, speaking: bool)
    -> Result<std::result::Result<(crate::hubstore::Place, String, Value, Side), Response>>
{
    let Some(id) = ctx.param("id").cloned() else {
        return Ok(Err(Response::error("missing order id", 400)?));
    };
    let place = crate::hubstore::Place::of_any(req, ctx).await?;
    let Some(order_json) = crate::hubstore::order(&place, &id).await? else {
        return Ok(Err(Response::error("order not found", 404)?));
    };
    let order: Value = serde_json::from_str(&order_json).unwrap_or(json!({}));
    let p = match crate::auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
        Ok(p) => p,
        Err(e) => return Ok(Err(e.into_response()?)),
    };
    match party::thread_party(&p, &place.venue, &order, speaking) {
        Ok(side) => Ok(Ok((place, id, order, side))),
        Err((code, why)) => Ok(Err(Response::error(why, code)?)),
    }
}

/// `GET /api/order/:id/chat?since=<ms>` -- the messages after `since`, oldest
/// first, with the chat's state and the caller's own side.
pub async fn read(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let since: i64 = req.url()?.query_pairs().find(|(k, _)| k == "since").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let (place, id, order, side) = match admit(&req, &ctx, false).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let rows = store::load(&place, &id).await?;
    let messages: Vec<Value> = rows
        .iter()
        .filter(|r| r.at_ms > since)
        .map(|r| json!({ "id": r.id, "from": r.from, "text": r.text, "atMs": r.at_ms }))
        .collect();
    let mut res = Response::from_json(&json!({
        "state": State::of(&order).as_str(),
        "me": side.as_str(),
        "messages": messages,
        "total": rows.len(),
    }))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendIn {
    text: String,
    /// The caller's idempotency key: the same one again is the same message.
    #[serde(rename = "clientId")]
    client_id: String,
}

/// `POST /api/order/:id/chat` `{text, clientId}` -- one message from the
/// customer or the assigned courier, while the chat is open.
pub async fn send(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    // AUTHORITY BEFORE THE BODY: nobody's JSON is parsed before the door.
    let (place, id, order, side) = match admit(&req, &ctx, true).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(from) = side.as_str() else {
        return Response::error("the venue reads this conversation and does not speak in it", 403);
    };
    match State::of(&order) {
        State::Waiting => return Response::error("the chat opens when a courier takes the order", 409),
        State::Closed => return Response::error("the chat is closed: the order is over", 409),
        State::Open => {}
    }
    let body: SendIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    if body.client_id.trim().is_empty() {
        return Response::error("clientId is required", 400);
    }
    let text = match store::clean(&body.text) {
        Ok(t) => t,
        Err(why) => return Response::error(why, 400),
    };
    let now = ctx.data.now_ms;
    let rows = store::load(&place, &id).await?;
    let msg_id = store::message_id(&id, body.client_id.trim());
    if let Some(same) = rows.iter().find(|r| r.id == msg_id) {
        return Response::from_json(&json!({ "id": msg_id, "atMs": same.at_ms, "replayed": true }));
    }
    if store::sent_in_last_hour(&rows, from, now) >= store::RATE_PER_HOUR {
        return Response::error("slow down: thirty messages an hour is the limit", 429);
    }
    let row = store::Row { id: msg_id.clone(), from: from.to_string(), text, at_ms: now };
    store::append(&place, &id, &row).await?;
    // THE TWO PARTIES' SOCKETS HEAR IT (`hubdo/chat.rs`); a lost nudge costs
    // one poll, never the message, so its failure is not this request's.
    let courier = order.get("courier_id").and_then(Value::as_str).unwrap_or("");
    let nudge = format!("https://hub/fold/chat?order={}&courier={}", crate::mcp::enc(&id), crate::mcp::enc(courier));
    if let Ok(stub) = place.stub() {
        let _ = stub.fetch_with_request(Request::new(&nudge, Method::Post)?).await;
    }
    Response::from_json(&json!({ "id": msg_id, "atMs": now, "replayed": false }))
}

#[cfg(test)]
mod tests;
