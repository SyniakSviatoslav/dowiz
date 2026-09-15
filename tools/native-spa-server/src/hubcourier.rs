//! The courier's surface.
//!
//! WHAT WAS MISSING AND WHY IT MATTERED. The courier app has always polled
//! `/api/courier/tasks`. Nothing could put a `courier_id` on an order, so that
//! list was empty by construction — the app worked, logged in, drew its map, and
//! had nothing to show, forever. The delivery leg of a delivery service was open.
//!
//! POSITIONS ARE VOLATILE, ON PURPOSE. A courier's location is worth something
//! for the next few seconds and nothing after that, so it lives in memory and
//! dies with the process. Writing it to the store would rewrite the whole KV
//! several times a minute per courier to persist a number that is stale by the
//! time anyone reads it — and would leave a movement history on disk that
//! nobody asked for and D0 gives no reason to keep.
//!
//! SHIFTS ARE ALSO IN MEMORY, and that is a weaker claim: a hub restart clears
//! every shift and couriers must reopen. That is the honest trade for now — a
//! shift is a statement about right now, and a stale "on shift" surviving a
//! crash would route orders to a phone that is off.

use std::collections::HashMap;

use axum::extract::{Path as AxPath, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_core::order_machine::OrderStatus;
use dowiz_hub::EventKind;
use dowiz_kernel::json_api;

use crate::hub::{now_ms, HubHttpError, Shared};
use crate::hubauth::CourierCaller;

/// Where a courier was, and when.
///
/// Coordinates are integer MICRO-DEGREES, per MANIFESTO C2: no float ever
/// touches a coordinate that the kernel might later compare or store. A
/// micro-degree is about 11 cm, which is finer than any phone's GPS.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub lat_udeg: i64,
    pub lon_udeg: i64,
    pub at_ms: i64,
}

/// The volatile half of the courier surface.
#[derive(Default)]
pub struct Live {
    pub positions: HashMap<String, Position>,
    pub shifts: HashMap<String, bool>,
}

/// `GET /api/courier/tasks` — this courier's work, and what is waiting for one.
pub async fn tasks(
    State(st): State<Shared>,
    who: CourierCaller,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    let hub = st.read_log()?;
    let all = hub.orders();

    let (mut mine, mut offered) = (Vec::new(), Vec::new());
    for ev in &all {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        // Only delivery orders concern a courier. A pickup order is the
        // customer's own journey.
        if o.get("fulfilment").and_then(|f| f.get("kind")).and_then(Value::as_str)
            != Some("delivery")
        {
            continue;
        }
        match o.get("courier_id").and_then(Value::as_str) {
            Some(c) if c == me => {
                if !matches!(status, "DELIVERED" | "CANCELLED" | "REJECTED") {
                    mine.push(task_shape(&o));
                }
            }
            // Unassigned and ready to leave the kitchen: anyone on shift may
            // take it. This is what makes the venue workable without the owner
            // hand-assigning every order during a rush.
            None if status == OrderStatus::Ready.as_str() => offered.push(task_shape(&o)),
            _ => {}
        }
    }
    // THE KEYS THE APP READS, and this is the second half of a break worth
    // naming. The courier app does:
    //
    //     S.onShift = d.onShift; S.mine = d.mine; S.available = d.available;
    //
    // and this endpoint answered `{tasks, available, courier}`. `onShift` came
    // back undefined, which is falsy, so the app rendered "you are offline" --
    // permanently, for every courier, no matter what. It never showed a single
    // task. The integration tests checked the API's own shape and passed, which
    // is exactly the gap between testing a response and testing a surface.
    //
    // `tasks` is kept alongside `mine` so anything written against the newer
    // spelling keeps working; they are the same list.
    let on_shift = st.shifts().await.iter().any(|c| c == &me);
    Ok(Json(json!({
        "onShift": on_shift,
        "mine": mine,
        "tasks": mine,
        "available": offered,
        "shift": if on_shift { json!({ "open": true }) } else { Value::Null },
        "courier": { "id": me, "name": who.0.person.name }
    })))
}

/// The order, shaped the way the courier app reads it.
///
/// A PROJECTION, not the raw envelope, and it fixes a real break: the app reads
/// `o.address.line`, while the order carries the address under
/// `fulfilment.address`. Handing it the envelope meant the delivery screen
/// showed NO address and no maps link at all -- the one thing a courier
/// actually needs from it.
///
/// Flattening here rather than teaching the app two shapes keeps the surface's
/// contract intact: it was written against an API that put these at the top
/// level, and that contract is fine. `lat_udeg`/`lon_udeg` ride along so the
/// map can drop a pin.
fn task_shape(o: &Value) -> Value {
    let addr = o.get("fulfilment").and_then(|f| f.get("address"));
    json!({
        "id": o.get("id").cloned().unwrap_or(Value::Null),
        "status": o.get("status").cloned().unwrap_or(Value::Null),
        "total": o.get("total").cloned().unwrap_or(Value::Null),
        "subtotal": o.get("subtotal").cloned().unwrap_or(Value::Null),
        "delivery_fee": o.get("delivery_fee").cloned().unwrap_or(Value::Null),
        "payment": o.get("payment").cloned().unwrap_or(Value::Null),
        "items": o.get("items").cloned().unwrap_or(Value::Null),
        "contact": o.get("contact").cloned().unwrap_or(Value::Null),
        "created_at_ms": o.get("created_at_ms").cloned().unwrap_or(Value::Null),
        "address": addr.cloned().unwrap_or(Value::Null),
        // The venue may not be able to verify where this goes; the courier is
        // the one who finds out at the door, so they are told in advance.
        "delivery_area_unverified": o
            .get("delivery_area_unverified")
            .cloned()
            .unwrap_or(Value::Bool(false)),
    })
}

/// One place where "this order is mine and still open" is decided.
fn claim_check(o: &Value, me: &str) -> Result<(), HubHttpError> {
    match o.get("courier_id").and_then(Value::as_str) {
        Some(c) if c == me => Ok(()),
        Some(_) => Err(HubHttpError::Refused("this order is assigned to another courier".into())),
        None => Err(HubHttpError::Refused("this order is not assigned to you".into())),
    }
}

/// `POST /api/courier/orders/{id}/accept` — take an unassigned order.
pub async fn accept(
    State(st): State<Shared>,
    who: CourierCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    st.with_log(move |hub| {
        let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
        let mut o: Value =
            serde_json::from_str(&current).map_err(|_| HubHttpError::Corrupt("order"))?;
        // FIRST WRITER WINS, and the write lock is what makes that true. Two
        // couriers tapping at once both read an unassigned order; only one of
        // them is inside the lock when it is written, and the second sees the
        // assignment and is refused rather than silently overwriting it.
        if let Some(existing) = o.get("courier_id").and_then(Value::as_str) {
            if existing != me {
                return Err(HubHttpError::Conflict("another courier took this order".into()));
            }
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if status != OrderStatus::Ready.as_str() {
            return Err(HubHttpError::Refused(format!(
                "an order can be taken when it is READY, not {status}"
            )));
        }
        o["courier_id"] = json!(me);
        let body = serde_json::to_string(&o).unwrap_or(current);
        hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        Ok(Json(o))
    })
    .await
}

/// Move an order the courier holds to `target`, with the kernel deciding
/// whether the edge is legal.
/// The courier transition, reachable from the voice surface as well as the
/// buttons. Same rules, same claim check, same log.
pub(crate) async fn advance_confirmed(
    st: &Shared,
    me: String,
    id: String,
    target: OrderStatus,
) -> Result<Value, HubHttpError> {
    advance_as_courier(st, me, id, target).await.map(|j| j.0)
}

async fn advance_as_courier(
    st: &Shared,
    me: String,
    id: String,
    target: OrderStatus,
) -> Result<Json<Value>, HubHttpError> {
    let notify_id = id.clone();
    let out = st
        .with_log(move |hub| {
            let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
            let cur: Value =
                serde_json::from_str(&current).map_err(|_| HubHttpError::Corrupt("order"))?;
            claim_check(&cur, &me)?;

            let updated = json_api::apply_event_logic(&current, target.as_str())
                .map_err(HubHttpError::Refused)?;
            let mut merged: Value = serde_json::from_str(&updated)
                .map_err(|_| HubHttpError::Corrupt("kernel order"))?;
            crate::hub::carry_over(&cur, &mut merged);
            merged["last_actor"] = json!(me);
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;
    st.notify_advanced(&notify_id, &out);
    Ok(Json(out))
}

/// `POST /api/courier/orders/{id}/pickup` — the food is with the courier.
pub async fn pickup(
    State(st): State<Shared>,
    who: CourierCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    advance_as_courier(&st, who.0.person.id, id, OrderStatus::InDelivery).await
}

#[derive(Deserialize)]
pub struct DeliverIn {
    /// Cash collected at the door, in minor units. Recorded, not trusted as
    /// authority: the kernel already knows the total, and this is what the
    /// courier says they took.
    #[serde(default)]
    pub cash_collected: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
}

/// `POST /api/courier/orders/{id}/deliver`.
pub async fn deliver(
    State(st): State<Shared>,
    who: CourierCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<DeliverIn>,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    let out = advance_as_courier(&st, me.clone(), id.clone(), OrderStatus::Delivered).await?;

    // The cash note rides as a separate append so the delivery itself is
    // recorded even if this second write is the one that fails.
    if body.cash_collected.is_some() || body.note.is_some() {
        let mut o = out.0.clone();
        if let Some(c) = body.cash_collected {
            o["cash_collected"] = json!(c);
        }
        if let Some(n) = body.note {
            o["courier_note"] = json!(n);
        }
        let payload = serde_json::to_string(&o).unwrap_or_default();
        let _ = st
            .with_log(move |hub| {
                hub.append(EventKind::Advanced, &id, &payload, now_ms() as u64, [0u8; 32])
                    .map_err(|e| HubHttpError::Io(format!("{e:?}")))
            })
            .await;
        return Ok(Json(o));
    }
    Ok(out)
}

#[derive(Deserialize)]
pub struct PositionIn {
    /// Degrees, as the browser's Geolocation API gives them. Converted to
    /// integer micro-degrees on arrival so no float crosses into storage.
    pub lat: f64,
    pub lon: f64,
}

/// `POST /api/courier/position`.
pub async fn position(
    State(st): State<Shared>,
    who: CourierCaller,
    Json(body): Json<PositionIn>,
) -> Result<Json<Value>, HubHttpError> {
    if !body.lat.is_finite() || !body.lon.is_finite() {
        return Err(HubHttpError::Invalid("position is not a number".into()));
    }
    if !(-90.0..=90.0).contains(&body.lat) || !(-180.0..=180.0).contains(&body.lon) {
        return Err(HubHttpError::Invalid("position is off the planet".into()));
    }
    let p = Position {
        lat_udeg: (body.lat * 1_000_000.0).round() as i64,
        lon_udeg: (body.lon * 1_000_000.0).round() as i64,
        at_ms: now_ms(),
    };
    st.set_position(&who.0.person.id, p).await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ShiftIn {
    pub open: bool,
}

/// `POST /api/courier/shift`.
pub async fn shift(
    State(st): State<Shared>,
    who: CourierCaller,
    Json(body): Json<ShiftIn>,
) -> Result<Json<Value>, HubHttpError> {
    st.set_shift(&who.0.person.id, body.open).await;
    Ok(Json(json!({ "open": body.open })))
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        .route("/api/courier/tasks", get(tasks))
        .route("/api/courier/orders/{id}/accept", post(accept))
        .route("/api/courier/orders/{id}/pickup", post(pickup))
        .route("/api/courier/orders/{id}/deliver", post(deliver))
        .route("/api/courier/position", post(position))
        .route("/api/courier/shift", post(shift))
        .route("/api/courier/assist", post(courier_assist))
        .route("/api/courier/orders/{id}/proof", post(proof))
        .route("/api/courier/earnings", get(earnings))
        .route("/api/courier/history", get(history))
        .with_state(state)
}

/// `POST /api/courier/assist` — the same assistant, scoped to this courier's run.
///
/// The facts are ONLY their own orders. A courier asking a question must not be
/// able to learn about work that is not theirs, and the cheapest way to
/// guarantee that is to never put it in the prompt.
pub async fn courier_assist(
    State(st): State<Shared>,
    who: CourierCaller,
    Json(body): Json<crate::hubowner::AskIn>,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    let now = now_ms();
    let hub = st.read_log()?;
    let mine: Vec<Value> = hub
        .orders()
        .iter()
        .filter_map(|ev| serde_json::from_str::<Value>(&ev.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        .filter(|o| {
            !matches!(
                o.get("status").and_then(Value::as_str).unwrap_or(""),
                "DELIVERED" | "CANCELLED" | "REJECTED"
            )
        })
        .map(|o| {
            let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(Value::Null),
                "payment": o.get("payment").cloned().unwrap_or(Value::Null),
                "waiting_minutes": (now - created) / 60_000,
                "address": o.get("fulfilment").and_then(|f| f.get("address")).cloned().unwrap_or(Value::Null),
                "contact": o.get("contact").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    crate::hubowner::assist_public(
        &st,
        crate::ai::SYSTEM_COURIER,
        json!({ "my_open_deliveries": mine, "currency": "ALL" }),
        &body.question,
    )
    .await
}

// ── what a courier did, and what they are holding ────────────────────────────

/// Midnight in the venue's timezone, and the two windows before it.
fn day_start(now: i64) -> i64 {
    let offset_min: i64 = std::env::var("TZ_OFFSET_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let day = 24 * 60 * 60 * 1000;
    let local = now + offset_min * 60 * 1000;
    local - local.rem_euclid(day) - offset_min * 60 * 1000
}

/// `GET /api/courier/earnings`
///
/// NOT A PAYOUT, and the naming is deliberate. dowiz has no settlement model
/// wired -- P47's rails are designed and not built -- so a figure called
/// "earnings" would be a number this hub invented about somebody's wages. What
/// it can report is fact: how many deliveries were completed and how much CASH
/// the courier is holding on the venue's behalf. Both come from the order log.
///
/// The cash figure is the one that matters day to day: it is what has to be
/// handed over at the end of a shift, and a courier who cannot see it is
/// reconciling from memory.
/// `POST /api/courier/orders/{id}/proof` — a photo at the door.
///
/// WHAT THIS IS FOR, and what it is not for. A customer who says the food never
/// arrived and a courier who says it did are, without this, two accounts and no
/// facts. One photo of the door settles almost all of them, and the ones it does
/// not settle it settles quickly.
///
/// It is NOT surveillance of the courier. It is taken once, at one moment they
/// choose, of a doorway -- not a track, not a stream, not a face. There is no
/// requirement to take one: a courier who does not is not flagged, scored, or
/// asked why, because a "proof rate" is a ranking and dowiz does not rank the
/// people who work through it.
///
/// Recorded as `Noted`: it adds a fact without moving the status, and a
/// photograph is not a transition the order machine ever decided.
pub async fn proof(
    State(st): State<Shared>,
    who: CourierCaller,
    AxPath(id): AxPath<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    // THEIRS, and still running. A photo attached to somebody else's delivery
    // is a stranger's doorway in a stranger's order.
    let raw = st.read_log()?.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
    let env: Value = serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("order"))?;
    if env.get("courier_id").and_then(Value::as_str) != Some(me.as_str()) {
        return Err(HubHttpError::Unauthorized("that run is not yours"));
    }
    let stored = st.put_media(&body)?;
    let url = stored.url();

    let (at, url2) = (now_ms(), url.clone());
    st.with_log(move |hub| {
        let raw = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
        let mut env: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("order"))?;
        // ONE photo. A second would replace the first, and a proof that can be
        // replaced is not proof of anything.
        if env.get("proof").is_some() {
            return Err(HubHttpError::Conflict("this run already has a photo".into()));
        }
        env["proof"] = json!({ "url": url2, "at": at, "by": me });
        let stored = serde_json::to_string(&env).unwrap_or(raw);
        hub.append(dowiz_hub::EventKind::Noted, &id, &stored, at as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))
    })
    .await?;
    Ok(Json(json!({ "url": url, "bytes": stored.bytes })))
}

pub async fn earnings(
    State(st): State<Shared>,
    who: CourierCaller,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    let now = now_ms();
    let today = day_start(now);
    let week = today - 6 * 24 * 60 * 60 * 1000;
    let month = today - 29 * 24 * 60 * 60 * 1000;

    let hub = st.read_log()?;
    let (mut d_today, mut d_week, mut d_month) = (0i64, 0i64, 0i64);
    let (mut c_today, mut c_week, mut c_month) = (0i64, 0i64, 0i64);
    let mut open_cash = 0i64;
    // Tips, kept apart from the cash a courier is holding FOR the venue. The
    // two numbers mean opposite things at the end of a shift: one is theirs and
    // one they hand over, and a single figure invites the wrong one.
    let (mut t_today, mut t_week, mut t_month) = (0i64, 0i64, 0i64);

    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        if o.get("courier_id").and_then(Value::as_str) != Some(me.as_str()) {
            continue;
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let cash = o.get("cash_collected").and_then(Value::as_i64).unwrap_or(0);

        let tip = o.get("tip").and_then(Value::as_i64).unwrap_or(0);
        if status == "DELIVERED" {
            if at >= month {
                d_month += 1;
                c_month += cash;
                t_month += tip;
            }
            if at >= week {
                d_week += 1;
                c_week += cash;
                t_week += tip;
            }
            if at >= today {
                d_today += 1;
                c_today += cash;
                t_today += tip;
            }
        } else if !matches!(status, "CANCELLED" | "REJECTED")
            && o.get("payment").and_then(Value::as_str) == Some("cash")
        {
            // Still out for delivery and payable in cash: what they are ABOUT
            // to be holding. Shown separately from what they already hold, so
            // the two are never added together by mistake.
            open_cash += o.get("total").and_then(Value::as_i64).unwrap_or(0);
        }
    }

    Ok(Json(json!({
        "today":  { "deliveries": d_today, "cash": c_today, "tips": t_today },
        "week":   { "deliveries": d_week,  "cash": c_week,  "tips": t_week },
        "month":  { "deliveries": d_month, "cash": c_month, "tips": t_month },
        // Cash on this shift that has not been handed over. The number a
        // courier is asked for at the end of the night.
        "cashInHand": c_today,
        "expectedCash": open_cash,
        "currency": "ALL",
        // Said out loud in the payload rather than only in a doc: nothing here
        // is a wage, and a surface that renders it as one is wrong.
        "note": "deliveries and cash collected; dowiz does not compute pay",
    })))
}

/// `GET /api/courier/history` — the runs that are finished.
pub async fn history(
    State(st): State<Shared>,
    who: CourierCaller,
) -> Result<Json<Value>, HubHttpError> {
    let me = who.0.person.id.clone();
    let hub = st.read_log()?;
    let mut rows: Vec<Value> = hub
        .orders()
        .iter()
        .filter_map(|ev| serde_json::from_str::<Value>(&ev.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        .filter(|o| {
            matches!(
                o.get("status").and_then(Value::as_str),
                Some("DELIVERED") | Some("CANCELLED") | Some("REJECTED")
            )
        })
        .map(|o| {
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(Value::Null),
                "payment": o.get("payment").cloned().unwrap_or(Value::Null),
                "cashCollected": o.get("cash_collected").cloned().unwrap_or(Value::Null),
                "at": o.get("created_at_ms").cloned().unwrap_or(Value::Null),
                // The STREET only, not the door number, and no phone. A
                // finished run does not need a way to contact the customer
                // again, and a history screen left open on a table should not
                // be a list of addresses.
                "street": o.get("fulfilment")
                    .and_then(|f| f.get("address"))
                    .and_then(|a| a.get("line"))
                    .and_then(Value::as_str)
                    .map(|l| l.split(',').next().unwrap_or(l).trim().to_string())
                    .map(Value::String)
                    .unwrap_or(Value::Null),
                "note": o.get("courier_note").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    rows.sort_by_key(|r| -r["at"].as_i64().unwrap_or(0));
    rows.truncate(100);
    Ok(Json(json!({ "history": rows })))
}
