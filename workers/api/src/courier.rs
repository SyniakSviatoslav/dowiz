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
use worker::*;

// `now_ms` WAS DEFINED HERE, and identically in `accounts.rs` and `owner.rs` --
// three copies of `Date::now().as_millis() as i64`. Three identical clock
// functions is how an injection gets done twice and missed once, which is what
// `tools/gates/clock.sh` counts. `owner::now_ms` is the one.
use crate::auth::{self, Principal};
use dowiz_kernel::json_api;

pub(crate) mod run;
mod door;
pub use door::refused;

/// GPS sanity, from the old platform's courier UX rules: reject a fix worse than
/// 100 m or a speed above 150 km/h. Both are wrong-by-construction for someone
/// on a scooter in Durrës, and a bad fix poisons every ETA computed from it.
const MAX_ACCURACY_M: i64 = 100;
const MAX_SPEED_MPS_MILLI: i64 = 41_667; // 150 km/h

async fn courier_at(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
) -> std::result::Result<(String, String), Response> {
    match auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
        Ok(Principal::Courier { courier_id, active_location_id, .. }) => {
            Ok((courier_id, active_location_id))
        }
        Ok(_) => Err(Response::error("forbidden role", 403).unwrap()),
        Err(e) => Err(e.into_response().unwrap()),
    }
}

/// The room's door, beside the courier's: who is signing, and with what.
///
/// A STAFF TOKEN is authenticated in full — signature, the session row, the
/// live membership word — and then asked `room_admits` for the capability.
/// ANY OTHER TOKEN is handed to `owner::owner_at`, unchanged, so an owner's
/// console and an owner's API key keep exactly the path they had: a route that
/// moves from `owner_at` to this one loses nothing an owner could do, and gains
/// a member of staff who holds `need`.
///
/// Returns the signer's id and capabilities. The id is what every room event
/// records as `by`.
pub(crate) async fn staff_at(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    venue: &str,
    need: auth::Cap,
) -> std::result::Result<(String, auth::Caps), Response> {
    let is_staff = auth::bearer(req)
        .ok()
        .and_then(|raw| auth::verify(&ctx.env, &raw, ctx.data.now_ms).ok())
        .is_some_and(|c| matches!(c, auth::Claims::Staff { .. }));
    if !is_staff {
        let who = crate::owner::owner_at(req, ctx, venue).await?;
        return Ok((who, auth::Caps::of(&auth::Cap::ALL)));
    }
    let p = match auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
        Ok(p) => p,
        Err(e) => return Err(e.into_response().unwrap()),
    };
    auth::room_admits(&p, venue, need).map_err(|(s, m)| Response::error(m, s).unwrap())
}

/// `GET /api/courier/tasks` — what is mine, and what is up for grabs.
pub async fn tasks(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };

    // The folded queue from the object. A courier app polls every twelve
    // seconds on shift; what it needs is the orders, not the log.
    let listed = crate::hubstore::orders(&place).await?;

    // THIS VENUE'S OPEN DELIVERIES. The query it replaces had no location
    // filter, so a courier's task list was computed against every venue's
    // assignments on the platform.
    let t = ops(&place).await?;
    struct A {
        order_id: String,
        courier_id: String,
    }
    let assigned: Vec<A> = t
        .all(K_ASG)
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
        .filter(|(_, v)| v.get("delivered_at_ms").map_or(true, |x| x.is_null()))
        .map(|(id, v)| A { order_id: id, courier_id: field_str(&v, "courier_id") })
        .collect();

    let mut mine = Vec::new();
    let mut open = Vec::new();
    for e in listed {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if v.get("location_id").and_then(|x| x.as_str()) != Some(loc.as_str()) {
            continue;
        }
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if !matches!(status, "READY" | "IN_DELIVERY") {
            continue;
        }
        // A COLLECTION ORDER IS NOT A DELIVERY. The customer said they would
        // come and get it; nothing about it belongs in a courier's pool. This
        // list filtered on status alone, so every pickup order that reached
        // READY was offered to every courier on shift as a job -- and a
        // courier who took one would carry food to an address the customer
        // never gave, while the customer waited at the counter.
        // ASKED AS "DOES IT LEAVE THE BUILDING", not as "is it a pickup".
        // This was `== Some("pickup")`, which is the same answer for two kinds
        // and the WRONG one for the third: a table order would have been
        // offered to a courier as a job, and a courier who took it would carry
        // food out to an address nobody gave while the customer sat at the
        // table.
        if !crate::services::ordering::fulfilment::leaves_the_building(crate::services::ordering::fulfilment::of(&v)) {
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
        let lapsed = crate::services::courier::offer::offer_lapsed(&v, ctx.data.now_ms);
        match holder {
            Some(c) if c == courier_id => {
                if v.get("accepted_at_ms").and_then(Value::as_i64).is_none() {
                    if let Some(at) = v.get("assigned_at_ms").and_then(Value::as_i64) {
                        card["offerEndsMs"] = json!(at + crate::services::courier::offer::OFFER_WINDOW_MS);
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

    let shift = shift_of(&t, &courier_id);

    Response::from_json(&json!({
        // WHO THIS IS, said by the server. The app needs its own id to put a
        // position on the socket, and reading it out of a token in JavaScript
        // is how a client ends up believing something the server did not say.
        "courierId": courier_id,
        "onShift": shift.is_some(),
        "shift": shift.map(|s| json!({
            "id": field_str(&s, "id"),
            "deliveries": field_i64(&s, "deliveries"),
            "cash": field_i64(&s, "cash_collected"),
        })),
        "mine": mine, "available": open
    }))
}

/// `POST /api/courier/shift` — `{open: bool}`
pub async fn shift(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        open: bool,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, _loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let now = ctx.data.now_ms;

    // WHICH OPEN RUNS HAVE ENDED, read from the log before the ops write: a
    // refunded order leaves its assignment open (see `run`), and only the
    // order's own status says it is no longer a delivery.
    let mut read = Vec::new();
    if !body.open {
        for id in run::open_runs(&ops(&place).await?.all(K_ASG), &courier_id) {
            let raw = crate::hubstore::order(&place, &id).await?;
            read.push((id, raw));
        }
    }
    let over = run::ended(&read);
    let cid = courier_id.clone();
    let open = body.open;
    let refused = with_ops(&place, move |t| {
        if open {
            // Idempotent: opening a shift twice must not create two. The
            // record's id IS the courier, so "two open shifts" is not a state
            // this layout can represent -- which is stronger than the
            // `LIMIT 1` that used to stand in for it.
            if shift_of(t, &cid).is_none() {
                let rec = json!({
                    "id": cid, "courier_id": cid, "started_at_ms": now,
                    "ended_at_ms": Value::Null, "deliveries": 0, "cash_collected": 0,
                })
                .to_string();
                t.put(K_SHIFT, &cid, &rec, &[], &[])
                    .map_err(|e| Error::RustError(format!("shift: {e}")))?;
            }
            return Ok(None);
        }
        // Refuse to close a shift with a run still in hand: the order would be
        // stranded with nobody holding it. A run whose order was refunded is
        // not in hand; one assigned since the read above is (not in `over`).
        if !run::in_hand(&t.all(K_ASG), &cid, &over).is_empty() {
            return Ok(Some("finish the delivery in hand before ending the shift"));
        }
        if let Some(mut s) = t
            .get(K_SHIFT, &cid)
            .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        {
            s["ended_at_ms"] = json!(now);
            t.put(K_SHIFT, &cid, &s.to_string(), &[], &[])
                .map_err(|e| Error::RustError(format!("shift: {e}")))?;
        }
        Ok(None)
    })
    .await?;
    if let Some(why) = refused {
        return Response::error(why, 409);
    }
    Response::from_json(&json!({ "onShift": body.open }))
}


// ── the venue's courier operations, in the venue's own image ───────────────
//
// `courier_assignments`, `courier_shifts` and `courier_positions` were three
// tables, and the first of them was read with `WHERE delivered_at_ms IS NULL`
// AND NO LOCATION FILTER AT ALL -- every venue's open deliveries, on every
// courier's task list. That is the sixth instance of this platform's defining
// defect and the last one in this file: the image is the venue, so there is no
// other venue's assignment in it to leak.

const K_ASG: &str = "asg";
const K_SHIFT: &str = "shift";
const K_POS: &str = "pos";

async fn ops(place: &crate::hubstore::Place) -> Result<dowiz_hub::table::Table> {
    Ok(crate::hubstore::load_table(place, crate::hubstore::IMAGE_OPS, crate::hubstore::OPS_BYTES)
        .await?
        .table)
}

async fn with_ops<F, T>(place: &crate::hubstore::Place, f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::table::Table) -> Result<T>,
{
    crate::hubstore::with_table(place, crate::hubstore::IMAGE_OPS, crate::hubstore::OPS_BYTES, f)
        .await
}

fn asg_of(t: &dowiz_hub::table::Table, order_id: &str) -> Option<Value> {
    t.get(K_ASG, order_id).and_then(|j| serde_json::from_str(&j).ok())
}

/// The shift a courier is on, if any. One open shift per courier by
/// construction: the record's id IS the courier, and the `ended_at_ms` field
/// says whether it is still running.
fn shift_of(t: &dowiz_hub::table::Table, courier_id: &str) -> Option<Value> {
    t.get(K_SHIFT, courier_id)
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .filter(|s| s.get("ended_at_ms").map_or(true, |v| v.is_null()))
}

fn field_i64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}

fn field_str(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Read one order out of the hub log, scoped to this hub's location.
async fn load_order(place: &crate::hubstore::Place, id: &str, loc: &str) -> Result<Option<(String, Value)>> {
    // ONE ORDER, NOT THE WHOLE LOG. The object folds and answers; this used to
    // pull every order the venue had ever taken to read one of them.
    let Some(raw) = crate::hubstore::order(place, id).await? else { return Ok(None) };
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
    now_ms: i64,
) -> Result<Value> {
    write_status_with(place, id, next, -1, now_ms).await
}

/// `cash` of -1 means "not a cash-collecting transition"; anything else is
/// recorded on the order.
async fn write_status_with(
    place: &crate::hubstore::Place,
    id: &str,
    next: &'static str,
    cash: i64,
    now_ms: i64,
) -> Result<Value> {
    let id_s = id.to_string();
    crate::hubstore::append_for(&place, &id_s.clone(), now_ms, move |current| {
        let current = current.ok_or_else(|| Error::RustError("order not found".into()))?;
        let updated = json_api::apply_event_logic(&current, next).map_err(Error::RustError)?;
        let mut merged: Value = serde_json::from_str(&updated)
            .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
        let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        crate::hubstore::carry_over(&old, &mut merged);
        crate::live_eta::stamp(&mut merged, next, now_ms);
        if cash >= 0 {
            merged["cash_collected"] = json!(cash);
        }
        // WHAT CHANGED, not what is. The whole envelope was written six times
        // per delivery; the fold puts it back together on the way out.
        let body = crate::fold::delta(&old, &merged).to_string();
        Ok(Some((dowiz_hub::EventKind::Advanced, body, merged)))
    })
    .await
    .and_then(|v| v.ok_or_else(|| Error::RustError("order not found".into())))
}

/// `POST /api/courier/orders/:id/accept`
pub async fn accept(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // ONE CLOCK READ FOR THE WHOLE REQUEST. The idempotency window, the
    // assignment stamp and the log stamp are all "when this request happened",
    // and three separate reads of the wall clock is three answers to one
    // question -- see `tools/gates/clock.sh`.
    let now = ctx.data.now_ms;
    // ── THE RETRY THAT MUST NOT BECOME A SECOND ANYTHING ──
    //
    // A courier taps in a basement, the tap is queued on the phone, and the
    // queue replays it when the signal returns (`public/lib/outbox.js`). The
    // FSM and the assignment row already make a replay SAFE -- a repeated
    // transition is an illegal edge and a second `accept` finds the row taken
    // -- but safe is not the same as truthful: without this the courier whose
    // first call LANDED and whose response was lost is told the order changed
    // while they were away, or that somebody else took a job that is theirs.
    // Rule 1 gives them the first call's own answer instead.
    //
    // THE ORDER ID IS THE BODY, because this route has none. Without it every
    // `accept` a courier makes under one key would look like the same call.
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &courier_id,
        "courier.accept",
        &id,
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let Some((_, v)) = load_order(&place, &id, &loc).await? else {
        return Response::error("not found", 404);
    };
    // ── AN ORDER HAS TO BE READY TO BE TAKEN ──
    //
    // There was no status check at all here: `load_order` compares the venue
    // and nothing else, and accepting is a `Noted` event, so the kernel's FSM
    // is never consulted either. A courier could therefore claim ANY order at
    // the venue in ANY state, and three things followed from it.
    //
    // Accept a PENDING order the owner then rejects, and the assignment row
    // keeps `delivered_at_ms IS NULL` for ever -- so `shift(open: false)`
    // refuses and that courier can never close a shift again. Accept anything,
    // and the owner's `assign_courier` answers 409 "already has a courier", so
    // a courier can pre-empt the dispatcher. Accept a CONFIRMED order and
    // `pickup` is legal straight to IN_DELIVERY, walking the food past the
    // kitchen: PREPARING is never entered, so the ingredients it reserved are
    // never consumed and the hold is stranded for good.
    //
    // READY is what the pool offers and CONFIRMED is what an owner assigns
    // ahead of the kitchen, so those two are the whole legitimate set.
    let status = v.get("status").and_then(Value::as_str).unwrap_or("");
    if !matches!(status, "READY" | "CONFIRMED") {
        return Response::error(format!("this order is {status}, not ready to be taken"), 409);
    }

    let cash_due = if v.get("payment").and_then(|p| p.as_str()) == Some("cash") {
        v.get("total").and_then(|t| t.as_i64()).unwrap_or(0)
    } else {
        0
    };

    // THE RACE IS SETTLED BY THE OBJECT, not by a primary key. Two couriers
    // tapping at once are two calls to one Durable Object, which runs them one
    // after the other; the first finds no assignment and writes one, the second
    // finds it and is told plainly. That is the same guarantee the UNIQUE
    // constraint gave, arriving as a property of where the bytes live.
    let (oid, cid) = (id.clone(), courier_id.clone());
    let taken = with_ops(&place, move |t| {
        if asg_of(t, &oid).is_some() {
            return Ok(false);
        }
        let rec = json!({
            "order_id": oid, "courier_id": cid, "assigned_at_ms": now,
            "cash_due": cash_due, "picked_up_at_ms": Value::Null,
            "delivered_at_ms": Value::Null, "cash_collected": Value::Null,
        })
        .to_string();
        t.put(K_ASG, &oid, &rec, &[], &[])
            .map_err(|e| Error::RustError(format!("assignment: {e}")))?;
        Ok(true)
    })
    .await?;
    if !taken {
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
    let oid = id.clone();
    let who = courier_id.clone();
    let claimed = crate::hubstore::append_for(&place, &oid.clone(), now, move |current| {
        let current = current.ok_or_else(|| Error::RustError("order not found".into()))?;
        let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        let mut o = old.clone();
        o["courier_id"] = json!(who);
        // Taking it ends any offer window: from here it is theirs until it is
        // delivered or the owner moves it.
        o["accepted_at_ms"] = json!(now);
        let body = crate::fold::delta(&old, &o).to_string();
        // `Noted`, not `Advanced`: taking an order is not a transition the
        // order machine decided, and writing it as one would put an edge in
        // the log that does not exist.
        Ok(Some((dowiz_hub::EventKind::Noted, body, json!(true))))
    })
    .await;
    if let Err(e) = claimed {
        // LOUD. The assignment row stands, so the order is not lost -- but the
        // courier's screens will not show it, and that is worth knowing.
        crate::loud!(
            &place.ns,
            Some(&place.venue),
            "courier.claim",
            "{courier_id} took {id} and the log did not record it: {e}"
        );
    }

    let out = json!({ "ok": true, "orderId": id, "cashDue": cash_due });
    idem.done(&place, 200, &out.to_string()).await;
    Response::from_json(&out)
}

/// `POST /api/courier/orders/:id/pickup` — READY → IN_DELIVERY
pub async fn pickup(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // One clock read for the whole request; see `accept`.
    let now = ctx.data.now_ms;
    // See `accept` for why a courier route needs this at all: the tap that is
    // replayed out of the phone's outbox landed the first time, and without a
    // key its replay is an illegal edge answered 409 -- a true refusal with a
    // false meaning.
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &courier_id,
        "courier.pickup",
        &id,
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let t = ops(&place).await?;
    if asg_of(&t, &id).map(|a| field_str(&a, "courier_id")) != Some(courier_id.clone()) {
        return Response::error("not your delivery", 403);
    }
    if load_order(&place, &id, &loc).await?.is_none() {
        return Response::error("not found", 404);
    }
    let merged = match write_status(&place, &id, "IN_DELIVERY", now).await {
        Ok(v) => v,
        Err(e) => return Response::error(e.to_string(), 409),
    };
    let oid = id.clone();
    with_ops(&place, move |t| {
        if let Some(mut a) = asg_of(t, &oid) {
            a["picked_up_at_ms"] = json!(now);
            t.put(K_ASG, &oid, &a.to_string(), &[], &[])
                .map_err(|e| Error::RustError(format!("assignment: {e}")))?;
        }
        Ok(())
    })
    .await?;
    idem.done(&place, 200, &merged.to_string()).await;
    Response::from_json(&merged)
}

/// `POST /api/courier/orders/:id/deliver` — `{cash_collected?}`
pub async fn deliver(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        cash_collected: Option<i64>,
    }
    // THE RAW BODY IS READ ONCE, because the idempotency layer fingerprints it:
    // rule 3 is "same key, different body is a 409", and it cannot answer that
    // from a parsed struct that has already dropped whatever else was sent.
    let raw_body = req.text().await.unwrap_or_default();
    let body: In = serde_json::from_str(&raw_body).unwrap_or(In { cash_collected: None });
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // One clock read for the whole request; see `accept`.
    let now = ctx.data.now_ms;
    // THE ONE WHERE A REPLAY LOSES REAL INFORMATION. This answer carries
    // `short` -- the cash the courier came back without -- and a retry whose
    // first response was lost would otherwise be told the order changed,
    // taking the shortfall with it. Rule 1 hands back the first call's body,
    // shortfall and all.
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &courier_id,
        "courier.deliver",
        &format!("{id}:{raw_body}"),
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };

    let t = ops(&place).await?;
    let a = asg_of(&t, &id).filter(|a| {
        field_str(a, "courier_id") == courier_id
            && a.get("delivered_at_ms").map_or(true, |x| x.is_null())
    });
    let Some(a) = a else {
        return Response::error("not your delivery", 403);
    };
    let cash_due = field_i64(&a, "cash_due");
    let collected = body.cash_collected.unwrap_or(cash_due);
    if collected < 0 {
        return Response::error("cash cannot be negative", 400);
    }
    // A short handover is RECORDED, never silently rounded. The difference is
    // what a settlement dispute is later resolved from.
    let short = cash_due - collected;

    if load_order(&place, &id, &loc).await?.is_none() {
        return Response::error("not found", 404);
    }
    // THE CASH GOES ON THE ORDER, not only into a shifts table. The courier's
    // wallet folds `cash_collected` from the orders themselves -- the same
    // reason the takings and the promo count do -- so a number kept only in a
    // side table is a number that screen will never show. It read zero for
    // every delivery until now.
    let merged = match write_status_with(&place, &id, "DELIVERED", collected, now).await {
        Ok(v) => v,
        Err(e) => return Response::error(e.to_string(), 409),
    };
    // THE ASSIGNMENT AND THE SHIFT IN ONE WRITE. They were two UPDATEs, and
    // half of that is a delivery recorded against nobody's shift -- the
    // courier's own count and cash silently short by one run.
    let (oid, cid) = (id.clone(), courier_id.clone());
    with_ops(&place, move |t| {
        if let Some(mut a) = asg_of(t, &oid) {
            a["delivered_at_ms"] = json!(now);
            a["cash_collected"] = json!(collected);
            t.put(K_ASG, &oid, &a.to_string(), &[], &[])
                .map_err(|e| Error::RustError(format!("assignment: {e}")))?;
        }
        if let Some(mut s) = t
            .get(K_SHIFT, &cid)
            .and_then(|j| serde_json::from_str::<Value>(&j).ok())
            .filter(|s| s.get("ended_at_ms").map_or(true, |v| v.is_null()))
        {
            s["deliveries"] = json!(field_i64(&s, "deliveries") + 1);
            s["cash_collected"] = json!(field_i64(&s, "cash_collected") + collected);
            t.put(K_SHIFT, &cid, &s.to_string(), &[], &[])
                .map_err(|e| Error::RustError(format!("shift: {e}")))?;
        }
        Ok(())
    })
    .await?;
    let out =
        json!({ "order": merged, "cashDue": cash_due, "cashCollected": collected, "short": short });
    idem.done(&place, 200, &out.to_string()).await;
    Response::from_json(&out)
}

/// `POST /api/courier/position` — `{lat, lon, accuracy_m?, speed_mps?, order_id?}`
pub async fn position(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
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
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, _) = match courier_at(&req, &ctx).await {
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

    // ONE RECORD PER COURIER, overwritten. The table kept every fix and a
    // nightly job deleted the old ones; what any reader ever wanted was the
    // LATEST, which is why `live_eta` had to join the table to a `MAX
    // (recorded_at_ms) GROUP BY courier_id` of itself. A position that is
    // superseded the moment the next one arrives is a value, not a history.
    let cid = courier_id.clone();
    let rec = json!({
        "courier_id": cid, "order_id": body.order_id,
        "lat_udeg": lat_udeg, "lon_udeg": lon_udeg,
        "accuracy_m": acc, "speed_mps_milli": speed,
        "recorded_at_ms": ctx.data.now_ms,
    })
    .to_string();
    with_ops(&place, move |t| {
        t.put(K_POS, &cid, &rec, &[], &[])
            .map_err(|e| Error::RustError(format!("position: {e}")))
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}

/// `GET /api/courier/earnings` — folded from the shift log, not a running total.
pub async fn earnings(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // FOLDED FROM THE ORDERS, not from a shifts table. The table was a second
    // place the same numbers lived, and the response it produced did not even
    // have the shape the courier app reads -- `d.today.cash` was undefined, so
    // the wallet showed nothing at all.
    // The orders and the venue's own record, together: the record is ~1 KB and
    // carries the time zone the day boundary below needs. It was the constant
    // `2 * 60 * 60 * 1000` -- Tirane's SUMMER offset -- so from 25 October a
    // courier's "today" would have started an hour early, and the cash they
    // are holding is counted against it.
    let (listed, venue) = futures_util::future::try_join(
        crate::hubstore::orders(&place),
        crate::hubstore::venue_record(&place),
    )
    .await?;
    let zone = crate::hubstore::zone_of(venue.as_ref());
    let now = ctx.data.now_ms;
    let day = 86_400_000i64;
    let today = dowiz_hub::tz::start_of_local_day_ms(zone, now);
    let (week, month) = (today - 6 * day, today - 29 * day);

    let (mut d_t, mut d_w, mut d_m) = (0i64, 0i64, 0i64);
    let (mut c_t, mut c_w, mut c_m) = (0i64, 0i64, 0i64);
    // Tips kept APART from the float: at the end of a shift one is handed over
    // and one is theirs, and a single figure is the wrong number to reach for
    // whichever way you reach.
    let (mut t_t, mut t_w, mut t_m) = (0i64, 0i64, 0i64);
    let mut open_cash = 0i64;
    let mut in_hand = 0i64;

    for e in listed {
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
        } else if !run::run_over(status)
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
