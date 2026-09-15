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

mod auth;
mod storefront;

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
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get("/healthz", |_, _| Response::ok("ok"))
        // ── public storefront ──
        .get_async("/api/public/locations/:slug/menu", storefront::menu)
        .post_async("/api/public/locations/:slug/orders", storefront::place)
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

            ctx.d1("DB")?
                .prepare(
                    "INSERT INTO orders (id, status, order_json, created_at_ms, updated_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?4)",
                )
                .bind(&[
                    id.into(),
                    status.into(),
                    order_json.clone().into(),
                    JsValue::from_f64(created_at_ms as f64),
                ])?
                .run()
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
            let found: Option<String> = ctx
                .d1("DB")?
                .prepare("SELECT order_json FROM orders WHERE id = ?1")
                .bind(&[id.into()])?
                .first(Some("order_json"))
                .await?;
            match found {
                Some(order_json) => {
                    let mut res = Response::ok(order_json)?;
                    res.headers_mut()
                        .set("content-type", "application/json; charset=utf-8")?;
                    Ok(res)
                }
                None => Response::error("order not found", 404),
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
            let current: Option<String> = db
                .prepare("SELECT order_json FROM orders WHERE id = ?1")
                .bind(&[id.clone().into()])?
                .first(Some("order_json"))
                .await?;
            let Some(current) = current else {
                return Response::error("order not found", 404);
            };

            // The kernel refuses an illegal edge; the Worker never decides this.
            let updated = match json_api::apply_event_logic(&current, &body.next_status) {
                Ok(j) => j,
                Err(e) => return kernel_reject(e),
            };
            let status = status_of(&updated)?;

            db.prepare(
                "UPDATE orders SET status = ?1, order_json = ?2, updated_at_ms = ?3 WHERE id = ?4",
            )
            .bind(&[
                status.into(),
                updated.clone().into(),
                JsValue::from_f64(Date::now().as_millis() as f64),
                id.into(),
            ])?
            .run()
            .await?;

            let mut res = Response::ok(updated)?;
            res.headers_mut()
                .set("content-type", "application/json; charset=utf-8")?;
            Ok(res)
        })
        .run(req, env)
        .await
}
