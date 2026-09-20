//! The courier's shift: go online, take a run, pick up, deliver, hand over cash.
//!
//! Like the owner surface, this module names INTENTS and lets the kernel FSM
//! decide. It holds no ordered list of statuses.
//!
//! Cash is the default here, not an edge case: the market this serves settles
//! about three orders in four in cash, so `cash_due` is recorded at assignment
//! and reconciled at delivery rather than inferred later.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::auth::{self, Principal};
use dowiz_kernel::json_api;

fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

/// GPS sanity, from the old platform's courier UX rules: reject a fix worse than
/// 100 m or a speed above 150 km/h. Both are wrong-by-construction for someone
/// on a scooter in Durrës, and a bad fix poisons every ETA computed from it.
const MAX_ACCURACY_M: i64 = 100;
const MAX_SPEED_MPS_MILLI: i64 = 41_667; // 150 km/h

async fn courier_at(
    req: &Request,
    ctx: &RouteContext<()>,
    db: &D1Database,
) -> std::result::Result<(String, String), Response> {
    match auth::authenticate(req, &ctx.env, db, now_ms()).await {
        Ok(Principal::Courier { courier_id, active_location_id, .. }) => {
            Ok((courier_id, active_location_id))
        }
        Ok(_) => Err(Response::error("forbidden role", 403).unwrap()),
        Err(e) => Err(e.into_response().unwrap()),
    }
}

/// `GET /api/courier/tasks` — what is mine, and what is up for grabs.
pub async fn tasks(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };

    let loaded = crate::hubstore::load(&place).await?;

    #[derive(Deserialize)]
    struct A {
        order_id: String,
        courier_id: String,
    }
    let assigned = db
        .prepare("SELECT order_id, courier_id FROM courier_assignments WHERE delivered_at_ms IS NULL")
        .all()
        .await?
        .results::<A>()?;

    let mut mine = Vec::new();
    let mut open = Vec::new();
    for e in loaded.hub.orders() {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if v.get("location_id").and_then(|x| x.as_str()) != Some(loc.as_str()) {
            continue;
        }
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if !matches!(status, "READY" | "IN_DELIVERY") {
            continue;
        }
        let holder = assigned
            .iter()
            .find(|a| a.order_id == e.order_id)
            .map(|a| a.courier_id.as_str());
        let f = v.get("fulfilment").cloned().unwrap_or(Value::Null);
        let mut card = json!({
            "id": e.order_id, "status": status,
            "total": v.get("total").cloned().unwrap_or(json!(0)),
            "payment": v.get("payment").cloned().unwrap_or(json!("cash")),
            "contact": v.get("contact").cloned().unwrap_or(Value::Null),
            "address": f.get("address").cloned().unwrap_or(Value::Null),
            "items": v.get("items").and_then(|i| i.as_array()).map(|a| a.len()).unwrap_or(0)
        });
        // ── THE FIVE-MINUTE OFFER WINDOW ──
        //
        // An assignment nobody answers must not sit on one courier's screen for
        // the rest of the evening while the food goes cold. After the window it
        // goes back to the pool -- NOT declined, not held against them, just no
        // longer exclusively theirs, and they can still take it if nobody else
        // did. An ACCEPTED order never lapses however long the ride takes.
        //
        // The deadline is sent as an INSTANT: a server-computed "seconds left"
        // is stale the moment it is sent, and a phone polling every few seconds
        // would show it jumping backwards.
        let lapsed = crate::extra::offer_lapsed(&v, now_ms());
        match holder {
            Some(c) if c == courier_id => {
                if v.get("accepted_at_ms").and_then(Value::as_i64).is_none() {
                    if let Some(at) = v.get("assigned_at_ms").and_then(Value::as_i64) {
                        card["offerEndsMs"] = json!(at + crate::extra::OFFER_WINDOW_MS);
                    }
                }
                mine.push(card)
            }
            // An offer that lapsed is back in the pool for everybody.
            Some(_) if lapsed && status == "READY" => open.push(card),
            // An order someone else is carrying is not shown at all rather than
            // greyed: a courier's screen during a run holds one job.
            Some(_) => {}
            None if status == "READY" => open.push(card),
            None => {}
        }
    }

    #[derive(Deserialize)]
    struct S {
        id: String,
        deliveries: i64,
        cash_collected: i64,
    }
    let shift: Option<S> = db
        .prepare(
            "SELECT id, deliveries, cash_collected FROM courier_shifts \
             WHERE courier_id = ?1 AND ended_at_ms IS NULL ORDER BY started_at_ms DESC LIMIT 1",
        )
        .bind(&[courier_id.into()])?
        .first(None)
        .await?;

    Response::from_json(&json!({
        "onShift": shift.is_some(),
        "shift": shift.map(|s| json!({ "id": s.id, "deliveries": s.deliveries, "cash": s.cash_collected })),
        "mine": mine, "available": open
    }))
}

/// `POST /api/courier/shift` — `{open: bool}`
pub async fn shift(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        open: bool,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let now = now_ms();

    if body.open {
        // Idempotent: opening a shift twice must not create two.
        let existing: Option<String> = db
            .prepare("SELECT id FROM courier_shifts WHERE courier_id = ?1 AND ended_at_ms IS NULL LIMIT 1")
            .bind(&[courier_id.clone().into()])?
            .first(Some("id"))
            .await?;
        if existing.is_none() {
            let Some(id) = crate::edge_id() else {
                return Response::error("no platform CSPRNG", 500);
            };
            db.prepare(
                "INSERT INTO courier_shifts (id,courier_id,location_id,started_at_ms) VALUES (?1,?2,?3,?4)",
            )
            .bind(&[id.into(), courier_id.clone().into(), loc.into(), JsValue::from_f64(now as f64)])?
            .run()
            .await?;
        }
    } else {
        // Refuse to close a shift with a run still in hand: the order would be
        // stranded with nobody holding it.
        let live: Option<String> = db
            .prepare(
                "SELECT order_id FROM courier_assignments \
                 WHERE courier_id = ?1 AND delivered_at_ms IS NULL LIMIT 1",
            )
            .bind(&[courier_id.clone().into()])?
            .first(Some("order_id"))
            .await?;
        if live.is_some() {
            return Response::error("finish the delivery in hand before ending the shift", 409);
        }
        db.prepare("UPDATE courier_shifts SET ended_at_ms = ?2 WHERE courier_id = ?1 AND ended_at_ms IS NULL")
            .bind(&[courier_id.into(), JsValue::from_f64(now as f64)])?
            .run()
            .await?;
    }
    Response::from_json(&json!({ "onShift": body.open }))
}

/// Read one order out of the hub log, scoped to this hub's location.
async fn load_order(place: &crate::hubstore::Place, id: &str, loc: &str) -> Result<Option<(String, Value)>> {
    let loaded = crate::hubstore::load(&place).await?;
    let Ok(raw) = loaded.hub.order(id) else { return Ok(None) };
    let v: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
    if v.get("location_id").and_then(|x| x.as_str()) != Some(loc) {
        return Ok(None);
    }
    Ok(Some((raw, v)))
}

/// Advance one order through the kernel and record the result as an event.
async fn write_status(
    place: &crate::hubstore::Place,
    id: &str,
    next: &'static str,
) -> Result<Value> {
    write_status_with(place, id, next, -1).await
}

/// `cash` of -1 means "not a cash-collecting transition"; anything else is
/// recorded on the order.
async fn write_status_with(
    place: &crate::hubstore::Place,
    id: &str,
    next: &'static str,
    cash: i64,
) -> Result<Value> {
    let id_s = id.to_string();
    crate::hubstore::with_hub(&place, move |hub| {
        let current = hub
            .order(&id_s)
            .map_err(|_| Error::RustError("order not found".into()))?;
        let updated = json_api::apply_event_logic(&current, next).map_err(Error::RustError)?;
        let mut merged: Value = serde_json::from_str(&updated)
            .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
        let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        crate::hubstore::carry_over(&old, &mut merged);
        crate::live_eta::stamp(&mut merged, next, now_ms());
        if cash >= 0 {
            merged["cash_collected"] = json!(cash);
        }
        let body = serde_json::to_string(&merged).unwrap_or(updated);
        hub.append(dowiz_hub::EventKind::Advanced, &id_s, &body, now_ms() as u64, [0u8; 32])
            .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        Ok(merged)
    })
    .await
}

/// `POST /api/courier/orders/:id/accept`
pub async fn accept(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let Some((_, v)) = load_order(&place, &id, &loc).await? else {
        return Response::error("not found", 404);
    };
    let cash_due = if v.get("payment").and_then(|p| p.as_str()) == Some("cash") {
        v.get("total").and_then(|t| t.as_i64()).unwrap_or(0)
    } else {
        0
    };

    // The PRIMARY KEY on order_id is what makes this a race the database settles:
    // two couriers tapping at once, one INSERT wins, the other is told plainly.
    let res = db
        .prepare(
            "INSERT INTO courier_assignments (order_id,courier_id,location_id,assigned_at_ms,cash_due) \
             VALUES (?1,?2,?3,?4,?5)",
        )
        .bind(&[
            id.clone().into(),
            courier_id.clone().into(),
            loc.into(),
            JsValue::from_f64(now_ms() as f64),
            JsValue::from_f64(cash_due as f64),
        ])?
        .run()
        .await;
    if res.is_err() {
        return Response::error("another courier took this order", 409);
    }

    // ── THE ORDER CARRIES ITS COURIER ──
    //
    // The row above settles the race; this is what every screen actually reads.
    // The courier's tasks, wallet and history all fold from the ORDER LOG, and
    // so does the owner's queue -- so a courier recorded only in a side table
    // is a courier none of them can see. Measured: a delivered order came back
    // with `courier_id: null`, its courier's wallet showed zero deliveries and
    // their history was empty, while the assignment row said otherwise.
    //
    // The INSERT is the authority on who won; this write only repeats its
    // answer where the rest of the system looks.
    let now = now_ms();
    let oid = id.clone();
    let who = courier_id.clone();
    let claimed = crate::hubstore::with_hub(&place, move |hub| {
        let current = hub
            .order(&oid)
            .map_err(|_| Error::RustError("order not found".into()))?;
        let mut o: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        o["courier_id"] = json!(who);
        // Taking it ends any offer window: from here it is theirs until it is
        // delivered or the owner moves it.
        o["accepted_at_ms"] = json!(now);
        let body = serde_json::to_string(&o).unwrap_or(current);
        // `Noted`, not `Advanced`: taking an order is not a transition the
        // order machine decided, and writing it as one would put an edge in
        // the log that does not exist.
        hub.append(dowiz_hub::EventKind::Noted, &oid, &body, now as u64, [0u8; 32])
            .map_err(|e| Error::RustError(format!("{e:?}")))
    })
    .await;
    if let Err(e) = claimed {
        // LOUD. The assignment row stands, so the order is not lost -- but the
        // courier's screens will not show it, and that is worth knowing.
        crate::loud!(
            &place.db,
            Some(&place.venue),
            "courier.claim",
            "{courier_id} took {id} and the log did not record it: {e}"
        );
    }

    Response::from_json(&json!({ "ok": true, "orderId": id, "cashDue": cash_due }))
}

/// `POST /api/courier/orders/:id/pickup` — READY → IN_DELIVERY
pub async fn pickup(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let held: Option<String> = db
        .prepare("SELECT order_id FROM courier_assignments WHERE order_id = ?1 AND courier_id = ?2")
        .bind(&[id.clone().into(), courier_id.into()])?
        .first(Some("order_id"))
        .await?;
    if held.is_none() {
        return Response::error("not your delivery", 403);
    }
    if load_order(&place, &id, &loc).await?.is_none() {
        return Response::error("not found", 404);
    }
    let merged = match write_status(&place, &id, "IN_DELIVERY").await {
        Ok(v) => v,
        Err(e) => return Response::error(e.to_string(), 409),
    };
    db.prepare("UPDATE courier_assignments SET picked_up_at_ms = ?2 WHERE order_id = ?1")
        .bind(&[id.into(), JsValue::from_f64(now_ms() as f64)])?
        .run()
        .await?;
    Response::from_json(&merged)
}

/// `POST /api/courier/orders/:id/deliver` — `{cash_collected?}`
pub async fn deliver(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        cash_collected: Option<i64>,
    }
    let body: In = req.json().await.unwrap_or(In { cash_collected: None });
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };

    #[derive(Deserialize)]
    struct A {
        cash_due: i64,
    }
    let a: Option<A> = db
        .prepare("SELECT cash_due FROM courier_assignments WHERE order_id = ?1 AND courier_id = ?2 AND delivered_at_ms IS NULL")
        .bind(&[id.clone().into(), courier_id.clone().into()])?
        .first(None)
        .await?;
    let Some(a) = a else {
        return Response::error("not your delivery", 403);
    };
    let collected = body.cash_collected.unwrap_or(a.cash_due);
    if collected < 0 {
        return Response::error("cash cannot be negative", 400);
    }
    // A short handover is RECORDED, never silently rounded. The difference is
    // what a settlement dispute is later resolved from.
    let short = a.cash_due - collected;

    if load_order(&place, &id, &loc).await?.is_none() {
        return Response::error("not found", 404);
    }
    // THE CASH GOES ON THE ORDER, not only into a shifts table. The courier's
    // wallet folds `cash_collected` from the orders themselves -- the same
    // reason the takings and the promo count do -- so a number kept only in a
    // side table is a number that screen will never show. It read zero for
    // every delivery until now.
    let merged = match write_status_with(
        &place, &id, "DELIVERED", collected).await {
        Ok(v) => v,
        Err(e) => return Response::error(e.to_string(), 409),
    };
    let now = now_ms();
    db.prepare("UPDATE courier_assignments SET delivered_at_ms = ?2, cash_collected = ?3 WHERE order_id = ?1")
        .bind(&[id.into(), JsValue::from_f64(now as f64), JsValue::from_f64(collected as f64)])?
        .run()
        .await?;
    db.prepare(
        "UPDATE courier_shifts SET deliveries = deliveries + 1, cash_collected = cash_collected + ?2 \
         WHERE courier_id = ?1 AND ended_at_ms IS NULL",
    )
    .bind(&[courier_id.into(), JsValue::from_f64(collected as f64)])?
    .run()
    .await?;
    Response::from_json(&json!({ "order": merged, "cashDue": a.cash_due, "cashCollected": collected, "short": short }))
}

/// `POST /api/courier/position` — `{lat, lon, accuracy_m?, speed_mps?, order_id?}`
pub async fn position(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        lat: f64,
        lon: f64,
        #[serde(default)]
        accuracy_m: Option<f64>,
        #[serde(default)]
        speed_mps: Option<f64>,
        #[serde(default)]
        order_id: Option<String>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let (courier_id, _) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };

    if !body.lat.is_finite() || !body.lon.is_finite()
        || body.lat.abs() > 90.0 || body.lon.abs() > 180.0
    {
        return Response::error("impossible coordinates", 400);
    }
    let acc = body.accuracy_m.map(|a| a.round() as i64);
    if acc.is_some_and(|a| a > MAX_ACCURACY_M) {
        // Refuse rather than store: a 500 m fix looks like data and is not.
        return Response::error("fix too coarse", 422);
    }
    let speed = body.speed_mps.map(|s| (s * 1000.0).round() as i64);
    if speed.is_some_and(|s| s > MAX_SPEED_MPS_MILLI) {
        return Response::error("implausible speed", 422);
    }

    // Stored as INTEGER micro-degrees, the same rule as everywhere else: nothing
    // the kernel may later fold depends on float rounding.
    let lat_udeg = (body.lat * 1_000_000.0).round() as i64;
    let lon_udeg = (body.lon * 1_000_000.0).round() as i64;

    db.prepare(
        "INSERT INTO courier_positions (courier_id,order_id,lat_udeg,lon_udeg,accuracy_m,speed_mps_milli,recorded_at_ms) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&[
        courier_id.into(),
        body.order_id.map(JsValue::from).unwrap_or(JsValue::NULL),
        JsValue::from_f64(lat_udeg as f64),
        JsValue::from_f64(lon_udeg as f64),
        acc.map(|a| JsValue::from_f64(a as f64)).unwrap_or(JsValue::NULL),
        speed.map(|s| JsValue::from_f64(s as f64)).unwrap_or(JsValue::NULL),
        JsValue::from_f64(now_ms() as f64),
    ])?
    .run()
    .await?;
    Response::from_json(&json!({ "ok": true }))
}

/// `GET /api/courier/earnings` — folded from the shift log, not a running total.
pub async fn earnings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // FOLDED FROM THE ORDERS, not from a shifts table. The table was a second
    // place the same numbers lived, and the response it produced did not even
    // have the shape the courier app reads -- `d.today.cash` was undefined, so
    // the wallet showed nothing at all.
    let loaded = crate::hubstore::load(&place).await?;
    let now = now_ms();
    let day = 86_400_000i64;
    let today = ((now + 2 * 60 * 60 * 1000) / day) * day - 2 * 60 * 60 * 1000;
    let (week, month) = (today - 6 * day, today - 29 * day);

    let (mut d_t, mut d_w, mut d_m) = (0i64, 0i64, 0i64);
    let (mut c_t, mut c_w, mut c_m) = (0i64, 0i64, 0i64);
    // Tips kept APART from the float: at the end of a shift one is handed over
    // and one is theirs, and a single figure is the wrong number to reach for
    // whichever way you reach.
    let (mut t_t, mut t_w, mut t_m) = (0i64, 0i64, 0i64);
    let mut open_cash = 0i64;
    let mut in_hand = 0i64;

    for e in loaded.hub.orders() {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if v.get("location_id").and_then(Value::as_str).map(|l| l != loc).unwrap_or(false) {
            continue;
        }
        if v.get("courier_id").and_then(Value::as_str) != Some(courier_id.as_str()) {
            continue;
        }
        let status = v.get("status").and_then(Value::as_str).unwrap_or("");
        let at = v.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let cash = v.get("cash_collected").and_then(Value::as_i64).unwrap_or(0);
        let tip = v.get("tip").and_then(Value::as_i64).unwrap_or(0);
        if status == "DELIVERED" {
            if at >= month { d_m += 1; c_m += cash; t_m += tip; }
            if at >= week { d_w += 1; c_w += cash; t_w += tip; }
            if at >= today { d_t += 1; c_t += cash; t_t += tip; in_hand += cash; }
        } else if !matches!(status, "CANCELLED" | "REJECTED")
            && v.get("payment").and_then(Value::as_str) == Some("cash")
        {
            // Still out and payable in cash: what they are ABOUT to hold, shown
            // separately so the two are never added together by mistake.
            open_cash += v.get("total").and_then(Value::as_i64).unwrap_or(0);
        }
    }

    Response::from_json(&json!({
        "today":  { "deliveries": d_t, "cash": c_t, "tips": t_t },
        "week":   { "deliveries": d_w, "cash": c_w, "tips": t_w },
        "month":  { "deliveries": d_m, "cash": c_m, "tips": t_m },
        "cashInHand": in_hand,
        "expectedCash": open_cash
    }))
}
