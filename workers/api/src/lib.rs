//! dowiz API Worker — transport and storage ONLY.
//!
//! Every order decision is made by `dowiz_kernel::json_api`. This module never
//! re-implements the FSM, a price, or a total: it moves bytes between HTTP, D1
//! and the kernel. That is the same rule `web/src/app.js` currently breaks with
//! its hand-rolled status chain, and the reason the kernel is linked directly
//! rather than reached over a network hop.
//!
//! Identity and the clock come from the EDGE, never from the kernel — see
//! `place_order_at`. A process-local counter is unsound here by construction:
//! Workers recycles isolates constantly and runs many at once.

mod accounts;
mod auth;
mod bootstrap;
mod courier;
mod hubstore;
mod otel;
mod owner;
mod storefront;
mod stripe;

use dowiz_kernel::json_api;
use serde::Deserialize;
use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;

#[derive(Deserialize)]
struct PlaceOrderBody {
    #[serde(default)]
    customer_id: Option<String>,
    items_json: String,
    #[serde(default)]
    channel: Option<String>,
}

#[derive(Deserialize)]
struct AdvanceBody {
    next_status: String,
}

/// A CSPRNG-backed order id from the platform's Web Crypto.
///
/// FAIL-CLOSED on purpose: if `crypto.randomUUID` is not reachable we refuse the
/// request rather than fall back to a weaker source. An order id that can repeat
/// is a primary-key collision between two customers, which is exactly the defect
/// the kernel's old `AtomicU64` counter produced once it left a single process.
pub fn edge_id() -> Option<String> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto")).ok()?;
    let f = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID")).ok()?;
    let f = f.dyn_ref::<js_sys::Function>()?;
    f.call0(&crypto).ok()?.as_string()
}

/// Pull `status` out of a kernel-serialized order so it can live in its own
/// column. The kernel's JSON stays the single source of truth in `order_json`;
/// this is a projection for querying, never a second authority.
fn status_of(order_json: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(order_json)
        .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
    v.get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::RustError("kernel order json has no status".into()))
}

/// The kernel answers `Err(String)` for malformed input and illegal transitions
/// alike; both are client errors at this boundary, never a 500.
fn kernel_reject(msg: String) -> Result<Response> {
    Response::error(msg, 400)
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // One span per request, continuing an incoming W3C traceparent if there is
    // one. The export happens in waitUntil AFTER the response is returned, so
    // tracing costs the customer nothing in latency.
    let method = req.method().to_string();
    let path = req.path();
    let mut trace = otel::Trace::begin(&req, &format!("{method} {path}"));
    trace.attr(0, "http.request.method", serde_json::json!(method));
    trace.attr(0, "url.path", serde_json::json!(path));

    let out = route(req, env.clone()).await;
    let status = match &out {
        Ok(r) => r.status_code(),
        Err(_) => 500,
    };
    if let Err(e) = &out {
        trace.fail(0, &e.to_string());
    }
    ctx.wait_until(async move { trace.export(&env, status).await });
    out
}

async fn route(req: Request, env: Env) -> Result<Response> {
    Router::new()
        .get("/healthz", |_, _| Response::ok("ok"))
        // ── public storefront ──
        .get_async("/api/public/locations/:slug/menu", storefront::menu)
        .post_async("/api/public/locations/:slug/orders", storefront::place)
        // ── accounts ──
        .post_async("/api/bootstrap", bootstrap::seed)
        .post_async("/api/webhooks/stripe", stripe::webhook)
        .post_async("/api/auth/login", accounts::owner_login)
        .post_async("/api/auth/refresh", accounts::owner_refresh)
        .post_async("/api/auth/logout", accounts::owner_logout)
        .post_async("/api/courier/auth/login", accounts::courier_login)
        // ── owner ──
        .get_async("/api/owner/orders", owner::orders)
        .post_async("/api/owner/orders/:id/action", owner::order_action)
        .get_async("/api/owner/dashboard", owner::dashboard)
        .post_async("/api/owner/products/:id", owner::update_product)
        .post_async("/api/owner/location", owner::update_location)
        // ── courier ──
        .get_async("/api/courier/tasks", courier::tasks)
        .post_async("/api/courier/shift", courier::shift)
        .post_async("/api/courier/orders/:id/accept", courier::accept)
        .post_async("/api/courier/orders/:id/pickup", courier::pickup)
        .post_async("/api/courier/orders/:id/deliver", courier::deliver)
        .post_async("/api/courier/position", courier::position)
        .get_async("/api/courier/earnings", courier::earnings)
        .post_async("/api/order", |mut req, ctx| async move {
            let body: PlaceOrderBody = match req.json().await {
                Ok(b) => b,
                Err(e) => return Response::error(format!("bad request body: {e}"), 400),
            };
            let Some(id) = edge_id() else {
                return Response::error("no platform CSPRNG for order id", 500);
            };
            let created_at_ms = Date::now().as_millis() as i64;

            let order_json = match json_api::place_order_at(
                id.clone(),
                body.customer_id,
                &body.items_json,
                created_at_ms,
                body.channel,
            ) {
                Ok(j) => j,
                Err(e) => return kernel_reject(e),
            };
            let status = status_of(&order_json)?;


            let seq = created_at_ms as u64;
            let ev_id = id.clone();
            let ev_json = order_json.clone();
            crate::hubstore::with_hub(&ctx.d1("DB")?, move |hub| {
                hub.append(dowiz_hub::EventKind::Placed, &ev_id, &ev_json, seq, [0u8; 32])
                    .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))
            })
            .await?;


            let mut res = Response::ok(order_json)?;
            res.headers_mut()
                .set("content-type", "application/json; charset=utf-8")?;
            Ok(res)
        })
        .get_async("/api/order/:id", |_req, ctx| async move {
            let Some(id) = ctx.param("id").cloned() else {
                return Response::error("missing order id", 400);
            };
            // Read from the hub's event log. An order's state is the fold over
            // its events, so there is no row here that could have drifted from
            // what actually happened to it.
            let loaded = hubstore::load(&ctx.d1("DB")?).await?;
            match loaded.hub.order(&id) {
                Ok(order_json) => {
                    let mut res = Response::ok(order_json)?;
                    res.headers_mut()
                        .set("content-type", "application/json; charset=utf-8")?;
                    Ok(res)
                }
                Err(_) => Response::error("order not found", 404),
            }
        })
        .post_async("/api/order/:id/advance", |mut req, ctx| async move {
            let Some(id) = ctx.param("id").cloned() else {
                return Response::error("missing order id", 400);
            };
            let body: AdvanceBody = match req.json().await {
                Ok(b) => b,
                Err(e) => return Response::error(format!("bad request body: {e}"), 400),
            };
            let db = ctx.d1("DB")?;
            let next = body.next_status.clone();

            // One read-modify-write against the hub image, replayed if another
            // writer moved it first. The kernel decides whether the edge is
            // legal; the Worker only records its answer.
            let out = hubstore::with_hub(&db, move |hub| {
                let current = hub
                    .order(&id)
                    .map_err(|_| Error::RustError("order not found".into()))?;
                let updated = json_api::apply_event_logic(&current, &next)
                    .map_err(Error::RustError)?;
                let merged = carry_envelope(&current, &updated);
                hub.append(
                    dowiz_hub::EventKind::Advanced,
                    &id,
                    &merged,
                    Date::now().as_millis() as u64,
                    [0u8; 32],
                )
                .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
                Ok(merged)
            })
            .await;

            match out {
                Ok(merged) => {
                    let mut res = Response::ok(merged)?;
                    res.headers_mut()
                        .set("content-type", "application/json; charset=utf-8")?;
                    Ok(res)
                }
                // An illegal transition is the caller's mistake, so 409 -- never
                // a 500, which would blame the server for a refusal it was right
                // to make.
                Err(e) => {
                    let msg = e.to_string();
                    let code = if msg.contains("not found") { 404 } else { 409 };
                    Response::error(msg, code)
                }
            }
        })
        .run(req, env)
        .await
}

/// Carry the fields the kernel does not model across a transition.
///
/// The kernel owns items, status, subtotal and the ledger. Delivery address,
/// contact and payment ride alongside until the aggregate's new fields reach
/// this boundary, and they have to survive every advance: losing a delivery
/// address on a status change is a silent loss that only surfaces at the door.
fn carry_envelope(old_raw: &str, updated: &str) -> String {
    let Ok(mut merged) = serde_json::from_str::<serde_json::Value>(updated) else {
        return updated.to_string();
    };
    let old: serde_json::Value = serde_json::from_str(old_raw).unwrap_or(serde_json::Value::Null);
    for k in [
        "location_id", "contact", "fulfilment", "payment", "delivery_fee", "total",
        "courier_id", "rejection_reason",
    ] {
        if let Some(v) = old.get(k) {
            merged[k] = v.clone();
        }
    }
    serde_json::to_string(&merged).unwrap_or_else(|_| updated.to_string())
}
