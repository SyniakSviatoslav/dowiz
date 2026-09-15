//! The owner's surface, against the hub's own log and catalogue.
//!
//! These paths existed in the front-end before the hub did. They used to reach
//! the cap-gated API, which answers 401 forever behind an empty AnchorRoster,
//! while the orders they wanted lived here. The admin pane is unchanged; what
//! changed is that there is now something behind it.
//!
//! THE KERNEL STILL DECIDES. An action names a destination — the kernel's own
//! `OwnerOrderAction::target` says which — and `apply_event_logic` decides
//! whether that edge is legal. There is no ordered list of statuses in this
//! file and no map from action to status: both live in the kernel, and a copy
//! here would be a second authority that eventually disagrees.
//!
//! `location_id` ARRIVES IN EVERY BODY and is ignored. One hub is one venue, so
//! it identifies nothing; it is accepted so the same front-end works against a
//! hub and against a multi-tenant deployment without a build flag.

use axum::extract::{Path as AxPath, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_core::order_machine::OrderStatus;
use dowiz_core::ports::owner_surface::OwnerOrderAction;
use dowiz_hub::EventKind;
use dowiz_kernel::json_api;

use crate::hub::{now_ms, HubHttpError, Shared};
use crate::hubauth::OwnerCaller;

/// Statuses an order passes through while it is still the venue's problem.
/// Derived from the kernel's own terminal check rather than listed here.
fn is_live(status: &str) -> bool {
    OrderStatus::from_str(status).is_some_and(|s| {
        !matches!(
            s,
            OrderStatus::Delivered
                | OrderStatus::PickedUp
                | OrderStatus::Rejected
                | OrderStatus::Cancelled
        )
    })
}

/// `GET /api/owner/orders` — the queue, newest first.
pub async fn orders(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let mut out: Vec<Value> = hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .collect();
    // Newest first: the owner is looking at what just came in, not at history.
    out.sort_by(|a, b| {
        b.get("created_at_ms")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .cmp(&a.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0))
    });
    Ok(Json(json!({ "orders": out })))
}

#[derive(Deserialize)]
pub struct ActionIn {
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// Accepted and ignored — see the module header.
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/orders/{id}/action`.
pub async fn order_action(
    State(st): State<Shared>,
    who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<ActionIn>,
) -> Result<Json<Value>, HubHttpError> {
    let action = OwnerOrderAction::from_verb(&body.action)
        .ok_or_else(|| HubHttpError::Invalid(format!("unknown action {:?}", body.action)))?;
    // A rejection the customer is shown must say why. The kernel makes `reason`
    // part of the signed bytes for exactly this reason; here it is at least
    // required rather than optional.
    if action == OwnerOrderAction::Reject
        && body.reason.as_deref().map_or(true, |r| r.trim().is_empty())
    {
        return Err(HubHttpError::Invalid("a rejection must carry a reason".into()));
    }

    let reason = body.reason.clone();
    let actor = who.0.person.id.clone();
    let notify_id = id.clone();
    let out = st
        .with_log(move |hub| {
            let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
            let cur: Value = serde_json::from_str(&current).unwrap_or(json!({}));
            let from = cur
                .get("status")
                .and_then(Value::as_str)
                .and_then(OrderStatus::from_str)
                .ok_or(HubHttpError::Corrupt("order status"))?;
            // The KERNEL names the destination. This handler does not know the
            // order of statuses and must not learn it.
            let target = action
                .target(from)
                .ok_or_else(|| HubHttpError::Refused(format!(
                    "{} cannot apply to an order that is {}",
                    action.verb(),
                    from.as_str()
                )))?;

            let updated = json_api::apply_event_logic(&current, target.as_str())
                .map_err(HubHttpError::Refused)?;
            let mut merged: Value = serde_json::from_str(&updated)
                .map_err(|_| HubHttpError::Corrupt("kernel order"))?;
            crate::hub::carry_over(&cur, &mut merged);
            if let Some(r) = reason {
                merged["rejection_reason"] = json!(r);
            }
            // Who did it. An order that changed state with nobody's name on it
            // is unanswerable the next morning.
            merged["last_actor"] = json!(actor);
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;

    st.notify_advanced(&notify_id, &out);
    Ok(Json(out))
}

/// `GET /api/owner/dashboard` — the four numbers on the top of the pane.
///
/// Computed from the log on every call rather than kept as counters. At one
/// venue the log is small and a derived number cannot drift from the events;
/// a maintained counter can, and does, exactly when something crashes midway.
pub async fn dashboard(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let orders = hub.orders();

    let day_start = start_of_day_ms(now_ms());
    let (mut today, mut pending, mut active, mut revenue) = (0i64, 0i64, 0i64, 0i64);
    for ev in &orders {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if status == OrderStatus::Pending.as_str() {
            pending += 1;
        }
        if is_live(status) {
            active += 1;
        }
        if created >= day_start {
            today += 1;
            // Revenue counts orders that were not refused. A rejected order is
            // not money the venue took, and counting it would overstate the day
            // every time the kitchen turned something down.
            if !matches!(status, "REJECTED" | "CANCELLED") {
                revenue += o.get("total").and_then(Value::as_i64).unwrap_or(0);
            }
        }
    }
    Ok(Json(json!({
        "todayOrders": today,
        "pending": pending,
        "active": active,
        "todayRevenue": revenue
    })))
}

/// Midnight, local to the venue.
///
/// `TZ_OFFSET_MINUTES` because "today" is the venue's day, not UTC's. Durrës is
/// UTC+1/+2, so a UTC day boundary would reset the owner's takings at one or two
/// in the morning — during service on a Saturday.
fn start_of_day_ms(now: i64) -> i64 {
    let offset_min: i64 = std::env::var("TZ_OFFSET_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let day = 24 * 60 * 60 * 1000;
    let local = now + offset_min * 60 * 1000;
    local - local.rem_euclid(day) - offset_min * 60 * 1000
}

#[derive(Deserialize)]
pub struct ProductIn {
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub unavailable_note: Option<String>,
    #[serde(default)]
    pub price: Option<i64>,
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/products/{id}` — stop-list a dish or change its price.
pub async fn update_product(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<ProductIn>,
) -> Result<Json<Value>, HubHttpError> {
    if let Some(p) = body.price {
        // Integer minor units, and a negative price is not a discount, it is a
        // typo that would make the kernel's ledger owe the customer money.
        if p < 0 {
            return Err(HubHttpError::Invalid("price cannot be negative".into()));
        }
    }
    st.with_catalog(move |cat| {
        let raw = cat.product(&id).ok_or(HubHttpError::NotFound("product"))?;
        let mut p: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        if let Some(a) = body.available {
            p["available"] = json!(a);
            // The note only makes sense while the dish is off. Leaving a stale
            // "none today" on a dish that is back is a lie the customer reads.
            if a {
                p["unavailableNote"] = Value::Null;
            }
        }
        if let Some(n) = body.unavailable_note {
            p["unavailableNote"] = if n.trim().is_empty() { Value::Null } else { json!(n) };
        }
        if let Some(price) = body.price {
            p["price"] = json!(price);
        }
        cat.set_product(&id, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(Json(p))
    })
    .await
}

#[derive(Deserialize)]
pub struct LocationIn {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub delivery_paused: Option<bool>,
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/location` — open or close the venue.
pub async fn update_location(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<LocationIn>,
) -> Result<Json<Value>, HubHttpError> {
    if let Some(s) = &body.status {
        if !matches!(s.as_str(), "open" | "closed") {
            return Err(HubHttpError::Invalid(format!("unknown status {s:?}")));
        }
    }
    st.with_catalog(move |cat| {
        let raw = cat.location().ok_or(HubHttpError::NotFound("venue"))?;
        let mut loc: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
        if let Some(s) = body.status {
            loc["status"] = json!(s);
        }
        if let Some(p) = body.delivery_paused {
            loc["delivery_paused"] = json!(if p { 1 } else { 0 });
        }
        cat.set_location(&serde_json::to_string(&loc).unwrap_or(raw));
        Ok(Json(loc))
    })
    .await
}

/// `GET /api/owner/couriers` — who can be assigned, and who is on shift.
pub async fn couriers(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let roster = st.read_roster()?;
    let shifts = st.shifts().await;
    let list: Vec<Value> = roster
        .couriers()
        .into_iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "active": c.active,
                "onShift": shifts.contains(&c.id),
            })
        })
        .collect();
    Ok(Json(json!({ "couriers": list })))
}

#[derive(Deserialize)]
pub struct AssignIn {
    pub courier_id: String,
}

/// `POST /api/owner/orders/{id}/assign` — hand an order to a courier.
///
/// The gap this closes: until now nothing could put a `courier_id` on an order,
/// so the courier app's task list was empty by construction no matter who was
/// logged into it.
pub async fn assign(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<AssignIn>,
) -> Result<Json<Value>, HubHttpError> {
    let roster = st.read_roster()?;
    match roster.person(&body.courier_id) {
        Some(p) if p.role == dowiz_hub::token::Role::Courier && p.active => {}
        // Assigning to someone who is not an active courier would produce an
        // order that no app will ever show, which looks exactly like a lost
        // order to the person waiting for it.
        _ => return Err(HubHttpError::Invalid("not an active courier".into())),
    }

    let courier = body.courier_id.clone();
    st.with_log(move |hub| {
        let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
        let mut o: Value =
            serde_json::from_str(&current).map_err(|_| HubHttpError::Corrupt("order"))?;
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !is_live(status) {
            return Err(HubHttpError::Refused(format!(
                "order is {status} and no longer needs a courier"
            )));
        }
        o["courier_id"] = json!(courier);
        let body = serde_json::to_string(&o).unwrap_or(current);
        hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        Ok(Json(o))
    })
    .await
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        .route("/api/owner/orders", get(orders))
        .route("/api/owner/orders/{id}/action", post(order_action))
        .route("/api/owner/orders/{id}/assign", post(assign))
        .route("/api/owner/dashboard", get(dashboard))
        .route("/api/owner/products/{id}", post(update_product))
        .route("/api/owner/location", post(update_location))
        .route("/api/owner/couriers", get(couriers))
        .with_state(state)
}
