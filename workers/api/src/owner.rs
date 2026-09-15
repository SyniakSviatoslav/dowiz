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

pub(crate) fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

/// Authenticate, require the owner role, and confirm the membership covers this
/// location. The membership is read LIVE — an owner removed a moment ago is
/// refused here even holding a valid token.
pub(crate) async fn owner_at(
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

/// Which venue, resolved rather than demanded.
///
/// A HUB IMAGE HOLDS EXACTLY ONE VENUE. The `location_id` query parameter is a
/// leftover from the multi-tenant table shape, and every console pane written
/// against the native adapter omits it -- which meant the owner surface on this
/// Worker answered 400 to its own admin pane for every route. When the caller
/// says nothing, the catalogue is asked.
pub(crate) async fn venue_of(req: &Request, db: &D1Database) -> Option<String> {
    if let Some(l) = location_of(req) {
        return Some(l);
    }
    let loaded = crate::hubstore::load_catalog(db).await.ok()?;
    let j = loaded.catalog.location()?;
    let v: Value = serde_json::from_str(&j).ok()?;
    v.get("id").and_then(|x| x.as_str()).map(String::from)
}

pub(crate) fn location_of(req: &Request) -> Option<String> {
    req.url()
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "location_id")
        .map(|(_, v)| v.to_string())
}

/// `GET /api/owner/orders?location_id=&status=`
pub async fn orders(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
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
            // THE ENVELOPE AS STORED, not a re-spelling of it.
            //
            // This projection sent `createdAtMs` and `courierId` while the
            // console reads `created_at_ms` and `courier_id`, and it dropped
            // `promo`, `discount`, `tip` and `feedback` entirely -- so every row
            // rendered without its money detail and with no time on it. That is
            // the same shape-mismatch that once left the courier app showing
            // "you are offline" for every courier: a hand-written projection is
            // a second contract, and the second contract is the one that drifts.
            //
            // Handing back the order itself removes the second contract. The
            // fields are the ones the log holds, which is what every other
            // reader already agrees on.
            let mut o = v.clone();
            o["id"] = json!(e.order_id);
            o["status"] = json!(st);
            Some(o)
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
        // ── EVERY FIELD THE HUB OWNS, CARRIED ACROSS THE TRANSITION ──
        //
        // The kernel returns its own order and knows nothing about delivery,
        // contact, discounts or tips, so anything not on this list is ERASED by
        // the first status change. The native adapter lost `discount` and
        // `promo` that way, and the consequence was not cosmetic: the use-count
        // folds over orders looking for `promo.code`, so a max-uses code became
        // infinitely reusable the moment the kitchen accepted the first order
        // that used it. Measured here as well -- a live order on Cloudflare came
        // back from confirm/preparing/ready with no promo and no tip, and the
        // day's takings were 200 lek too high because the tip could no longer
        // be subtracted.
        for k in [
            "location_id",
            "contact",
            "fulfilment",
            "payment",
            "payment_status",
            "delivery_fee",
            "courier_id",
            "created_at_ms",
            "rejection_reason",
            "cash_collected",
            "scheduled_for_ms",
            "tip",
            "discount",
            "promo",
            "feedback",
            "assigned_at_ms",
            "accepted_at_ms",
            "total",
        ] {
            if let Some(v) = old.get(k) {
                merged[k] = v.clone();
            }
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
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
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        match status {
            "PENDING" => pending += 1,
            "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY" => active += 1,
            _ => {}
        }
        // ── ONE DEFINITION OF TODAY'S TAKINGS ──
        //
        // This counted DELIVERED only while the native adapter and the
        // analytics count every order that was not REFUSED -- so the same
        // product showed an owner two different numbers depending on which
        // deployment they opened, and the dashboard disagreed with its own
        // analytics pane on the same screen. The rule is the analytics one,
        // because that is what the copy on both panes describes: money the
        // venue took, and a rejected order is not that.
        //
        // The tip is subtracted wherever the venue's money is counted: it is
        // the courier's, passing through.
        if !matches!(status, "REJECTED" | "CANCELLED") {
            revenue += v.get("total").and_then(|t| t.as_i64()).unwrap_or(0)
                - v.get("tip").and_then(|t| t.as_i64()).unwrap_or(0);
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
        /// The fourteen declarable allergens. AN EMPTY ARRAY IS A CLAIM --
        /// "none of the fourteen" -- and absent is not, which is why this is
        /// `Option<Vec<_>>` all the way from the wire.
        #[serde(default)]
        allergens: Option<Vec<String>>,
        /// The dish's real widest dimension, in centimetres. Without it the
        /// storefront shows no AR button, which is correct: a guessed size
        /// answers the customer's question wrongly.
        #[serde(default)]
        size_cm: Option<i64>,
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

    // Checked before the write, so a refused list never reaches the record. A
    // dish tagged `shelfish` would match no filter, so the customer who
    // filtered for shellfish would be shown it as safe.
    let allergens = match &body.allergens {
        None => None,
        Some(raw) => match dowiz_hub::allergens::validate(raw) {
            Ok(v) => Some(v),
            Err(e) => return Response::error(e, 400),
        },
    };
    if let Some(cm) = body.size_cm {
        if !(3..=120).contains(&cm) {
            return Response::error("a dish is between 3 and 120 cm across", 400);
        }
    }

    let want_id = id.clone();
    let price = body.price;
    let available = body.available;
    let note = body.unavailable_note.clone();
    let size_cm = body.size_cm;
    let written = crate::hubstore::with_catalog(&db, move |cat| {
        let Some(pj) = cat.product(&want_id) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&pj)
            .map_err(|e| Error::RustError(format!("catalogue product unreadable: {e}")))?;
        if let Some(v) = price {
            p["price"] = json!(v);
        }
        if let Some(cm) = size_cm {
            p["sizeCm"] = json!(cm);
        }
        if let Some(list) = &allergens {
            p["allergens"] = json!(list);
        }
        // ── THE ALLERGEN PUBLISH GATE ──
        //
        // A dish nobody has declared cannot go on sale. Not a badge, a refusal:
        // a customer with an allergy cannot tell "we checked and it is clear"
        // from "nobody filled this in", and every surface renders both as no
        // warning. Declaring costs one action and "none of the fourteen" is a
        // valid answer; the gate is per DISH, so a venue is never blocked
        // wholesale.
        if available == Some(true) {
            let decided = allergens.as_ref().map(|l| {
                if l.is_empty() {
                    dowiz_hub::allergens::Declaration::None
                } else {
                    dowiz_hub::allergens::Declaration::Contains(l.clone())
                }
            });
            let state = decided.unwrap_or_else(|| dowiz_hub::allergens::read(&pj));
            if !state.is_declared() {
                return Err(Error::RustError("undeclared".into()));
            }
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
    .await;
    // The gate's refusal is a 409 with the sentence that tells the owner what
    // to do, not a 500 with a marker word. The marker only exists because the
    // closure can only fail with `Error`.
    if let Err(e) = written {
        if e.to_string().contains("undeclared") {
            return Response::error(
                "declare this dish's allergens before putting it on sale; \
                 'none of the fourteen' is a valid answer, an empty field is not",
                409,
            );
        }
        if e.to_string().contains("unknown product") {
            return Response::error("not found", 404);
        }
        return Err(e);
    }

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
        /// The number a customer rings when the app cannot help them, and one
        /// of the two ways a venue satisfies the notifications leg.
        #[serde(default)]
        phone: Option<String>,
        /// Can a customer come and collect? The other half of fulfilment.
        #[serde(default)]
        pickup: Option<bool>,
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
    if let Some(ph) = &body.phone {
        // Empty CLEARS it -- a venue with no phone should be able to say so
        // rather than keep a number that no longer answers.
        if !ph.trim().is_empty() && ph.chars().filter(|c| c.is_ascii_digit()).count() < 8 {
            return Response::error("that does not look like a phone number", 400);
        }
    }
    let status = body.status.clone();
    let paused = body.delivery_paused;
    let phone = body.phone.clone();
    let pickup = body.pickup;
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
        if let Some(ph) = &phone {
            l["phone"] = if ph.trim().is_empty() { Value::Null } else { json!(ph.trim()) };
        }
        if let Some(p) = pickup {
            l["pickup"] = json!(p);
        }
        cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "ok": true }))
}
