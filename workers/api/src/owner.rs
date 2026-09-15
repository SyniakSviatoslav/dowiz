//! The owner's day of service: see the queue, move an order, stop a dish,
//! open or close the venue.
//!
//! Every status change goes through the kernel FSM. This module never contains a
//! list of statuses in order — the one in `web/src/app.js` is exactly the
//! duplicate authority the architecture forbids, and it is why an order there
//! could be walked anywhere. Here an illegal edge is the kernel's refusal.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::auth::{self, Principal};
use dowiz_kernel::json_api;

fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

/// Authenticate, require the owner role, and confirm the membership covers this
/// location. The membership is read LIVE — an owner removed a moment ago is
/// refused here even holding a valid token.
async fn owner_at(
    req: &Request,
    ctx: &RouteContext<()>,
    db: &D1Database,
    location_id: &str,
) -> std::result::Result<String, Response> {
    let p = match auth::authenticate(req, &ctx.env, db, now_ms()).await {
        Ok(p) => p,
        Err(e) => return Err(e.into_response().unwrap()),
    };
    let Principal::Owner { user_id, .. } = p else {
        return Err(Response::error("forbidden role", 403).unwrap());
    };
    #[derive(Deserialize)]
    struct M {
        id: String,
    }
    let m: std::result::Result<Option<M>, _> = async {
        db.prepare(
            "SELECT id FROM memberships WHERE user_id = ?1 AND location_id = ?2 \
             AND role = 'owner' AND status = 'active' LIMIT 1",
        )
        .bind(&[user_id.clone().into(), location_id.into()])?
        .first(None)
        .await
    }
    .await;
    match m {
        Ok(Some(_)) => Ok(user_id),
        // Cross-tenant is 404, never 403: a 403 confirms the location exists.
        Ok(None) => Err(Response::error("not found", 404).unwrap()),
        Err(e) => Err(Response::error(format!("auth backend unavailable: {e}"), 503).unwrap()),
    }
}

fn location_of(req: &Request) -> Option<String> {
    req.url()
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "location_id")
        .map(|(_, v)| v.to_string())
}

/// `GET /api/owner/orders?location_id=&status=`
pub async fn orders(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = location_of(&req) else {
        return Response::error("location_id required", 400);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }

    let status = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "status").map(|(_, v)| v.to_string()));

    // The hub log is the source. Reading it folds every order to its newest
    // state, so the queue cannot show a status the events do not support.
    let loaded = crate::hubstore::load(&db).await?;
    let out: Vec<Value> = loaded
        .hub
        .orders()
        .into_iter()
        .filter_map(|e| {
            let v: Value = serde_json::from_str(&e.order_json).ok()?;
            if v.get("location_id").and_then(|x| x.as_str()) != Some(loc.as_str()) {
                return None;
            }
            let st = v.get("status").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if let Some(want) = &status {
                if &st != want {
                    return None;
                }
            }
            Some(json!({
                "id": e.order_id, "status": st, "createdAtMs": e.seq,
                "total": v.get("total").cloned().unwrap_or(json!(0)),
                "subtotal": v.get("subtotal").cloned().unwrap_or(json!(0)),
                "items": v.get("items").cloned().unwrap_or(json!([])),
                "contact": v.get("contact").cloned().unwrap_or(Value::Null),
                "fulfilment": v.get("fulfilment").cloned().unwrap_or(Value::Null),
                "payment": v.get("payment").cloned().unwrap_or(Value::Null),
                "courierId": v.get("courier_id").cloned().unwrap_or(Value::Null)
            }))
        })
        .collect();
    Response::from_json(&json!({ "orders": out }))
}

/// `POST /api/owner/orders/:id/action` — `{location_id, action, reason?}`
///
/// `action` names an intent, never a target status. The mapping from intent to
/// status lives in one place and the FSM decides whether the edge is legal.
pub async fn order_action(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        action: String,
        #[serde(default)]
        reason: Option<String>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let db = ctx.d1("DB")?;
    if let Err(r) = owner_at(&req, &ctx, &db, &body.location_id).await {
        return Ok(r);
    }

    let next = match body.action.as_str() {
        "confirm" => "CONFIRMED",
        "reject" => "REJECTED",
        "preparing" => "PREPARING",
        "ready" => "READY",
        "cancel" => "CANCELLED",
        other => return Response::error(format!("unknown action: {other}"), 400),
    };

    let want_loc = body.location_id.clone();
    let reason = body.reason.clone();
    let out = crate::hubstore::with_hub(&db, move |hub| {
        let current = hub
            .order(&id)
            .map_err(|_| Error::RustError("order not found".into()))?;
        {
            let v: Value = serde_json::from_str(&current).unwrap_or(json!({}));
            if v.get("location_id").and_then(|x| x.as_str()) != Some(want_loc.as_str()) {
                return Err(Error::RustError("order not found".into()));
            }
        }
        // The kernel decides. An illegal edge is its refusal, not ours.
        let updated = json_api::apply_event_logic(&current, next).map_err(Error::RustError)?;
        let mut merged: Value = serde_json::from_str(&updated)
            .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
        let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        for k in ["location_id", "contact", "fulfilment", "payment", "delivery_fee", "courier_id"] {
            if let Some(v) = old.get(k) {
                merged[k] = v.clone();
            }
        }
        if let Some(total) = old.get("total") {
            merged["total"] = total.clone();
        }
        // A rejection carries WHY, recorded with the event so the customer can be
        // told something true rather than "rejected".
        if next == "REJECTED" {
            merged["rejection_reason"] = json!(reason);
        }
        let body_s = serde_json::to_string(&merged).unwrap_or(updated);
        hub.append(
            dowiz_hub::EventKind::Advanced,
            &id,
            &body_s,
            now_ms() as u64,
            [0u8; 32],
        )
        .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        Ok(merged)
    })
    .await;

    let merged = match out {
        Ok(m) => m,
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("not found") { 404 } else { 409 };
            return Response::error(msg, code);
        }
    };
    Response::from_json(&merged)
}

/// `GET /api/owner/dashboard?location_id=` — the numbers an owner looks at
/// between orders, computed from the log rather than kept as a running total
/// that can drift.
pub async fn dashboard(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = location_of(&req) else {
        return Response::error("location_id required", 400);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }

    // "Today" starts at local midnight for the venue. Without the timezone this
    // would silently mean UTC, and an owner in Durrës would see the day roll over
    // two hours early.
    let tz_offset_ms: i64 = 2 * 60 * 60 * 1000; // Europe/Tirane, standard time
    let now = now_ms();
    let day_start = ((now + tz_offset_ms) / 86_400_000) * 86_400_000 - tz_offset_ms;

    let loaded = crate::hubstore::load(&db).await?;
    let (mut count, mut revenue, mut pending, mut active) = (0i64, 0i64, 0i64, 0i64);
    for e in loaded.hub.orders() {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if v.get("location_id").and_then(|x| x.as_str()) != Some(loc.as_str()) {
            continue;
        }
        if (e.seq as i64) < day_start {
            continue;
        }
        count += 1;
        match v.get("status").and_then(|x| x.as_str()).unwrap_or("") {
            "PENDING" => pending += 1,
            "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY" => active += 1,
            // Revenue counts DELIVERED only. Counting a pending order as money
            // is how a dashboard starts lying.
            "DELIVERED" => revenue += v.get("total").and_then(|t| t.as_i64()).unwrap_or(0),
            _ => {}
        }
    }
    Response::from_json(&json!({
        "todayOrders": count, "todayRevenue": revenue,
        "pending": pending, "active": active, "dayStartMs": day_start
    }))
}

/// `PATCH /api/owner/products/:id` — the stop-list and the price.
pub async fn update_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        #[serde(default)]
        available: Option<bool>,
        #[serde(default)]
        unavailable_note: Option<String>,
        #[serde(default)]
        price: Option<i64>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product id", 400);
    };
    let db = ctx.d1("DB")?;
    if let Err(r) = owner_at(&req, &ctx, &db, &body.location_id).await {
        return Ok(r);
    }
    if let Some(p) = body.price {
        // Money is integer minor units and never negative. Refuse rather than clamp.
        if p < 0 {
            return Response::error("price must be >= 0", 400);
        }
    }

    let want_id = id.clone();
    let price = body.price;
    let available = body.available;
    let note = body.unavailable_note.clone();
    crate::hubstore::with_catalog(&db, move |cat| {
        let Some(pj) = cat.product(&want_id) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&pj)
            .map_err(|e| Error::RustError(format!("catalogue product unreadable: {e}")))?;
        if let Some(v) = price {
            p["price"] = json!(v);
        }
        if let Some(a) = available {
            p["available"] = json!(a);
            // Clearing the note when a dish comes back is the point: a stale
            // reason on an available dish reads as a contradiction.
            p["unavailableNote"] = if a { Value::Null } else { json!(note) };
        }
        cat.set_product(&want_id, &serde_json::to_string(&p).unwrap_or(pj));

        // Any catalogue write moves the menu version, which is how a client
        // notices its cart went stale.
        if let Some(lj) = cat.location() {
            if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
                let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
                l["menu_version"] = json!(v + 1);
                cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
            }
        }
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "ok": true, "id": id }))
}

/// `PATCH /api/owner/location` — open, close, go busy, pause delivery.
pub async fn update_location(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        delivery_paused: Option<bool>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    if let Err(r) = owner_at(&req, &ctx, &db, &body.location_id).await {
        return Ok(r);
    }
    if let Some(st) = &body.status {
        if !matches!(st.as_str(), "open" | "closed" | "busy") {
            return Response::error("status must be open, closed or busy", 400);
        }
    }
    let status = body.status.clone();
    let paused = body.delivery_paused;
    crate::hubstore::with_catalog(&db, move |cat| {
        let Some(lj) = cat.location() else {
            return Err(Error::RustError("no venue in the catalogue".into()));
        };
        let mut l: Value = serde_json::from_str(&lj)
            .map_err(|e| Error::RustError(format!("catalogue location unreadable: {e}")))?;
        if let Some(st) = &status {
            l["status"] = json!(st);
        }
        if let Some(p) = paused {
            l["delivery_paused"] = json!(if p { 1 } else { 0 });
        }
        cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "ok": true }))
}
