//! Reservations and entry passes — the transport for a kernel domain.
//!
//! **This module decides nothing.** Whether a booking may be made, whether a
//! transition is legal, whether a pass is valid — every one of those answers
//! comes from `dowiz_kernel::reservation` and `dowiz_kernel::pass`. What is here
//! is SQL, HTTP and the venue's key: bytes in, bytes out.
//!
//! That split is the same one `storefront`/`owner` already make for orders, and
//! it is why a booking replays identically on a phone that has never spoken to
//! this Worker.
//!
//! # Events, not state
//!
//! `reservations.status` is a cache. The authority is `reservation_events`,
//! appended once per transition and never updated, and [`fold_status`] replays
//! it through the kernel. When the two disagree the events win — and
//! [`reservation_detail`] says so out loud instead of quietly trusting the
//! column, because a cache that can silently diverge is a cache that will.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_kernel::pass::{self, PassClaims, PassWindow};
use dowiz_kernel::reservation::{
    self, BookingPolicy, ReservationRequest, ReservationStatus,
};

// The FLOOR. `dowiz_hub::tables` decides which tables can seat a party and
// which are taken for a slot; nothing in this file works that out.
use dowiz_hub::tables as floor;


/// Minutes since the Unix epoch — the kernel's unit for a slot.
///
/// TAKES THE INSTANT RATHER THAN READING ONE. Every caller is a handler that
/// already has the request's own `now`, and a slot computed from a SECOND read
/// of the clock can land a minute away from the one the rest of the handler
/// used — which for a booking window is the difference between accepted and
/// refused.
fn now_min(now_ms: i64) -> i64 {
    now_ms / 60_000
}

/// A stable 64-bit id from a string, for the kernel types that take integers.
/// FNV-1a: not a hash anybody's security rests on, only a way to carry a TEXT
/// primary key into a `u64` field without inventing a second id space.
fn id64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ─────────────────────────────────────────────────────────────────────────────
// Reading
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EventRow {
    to_status: String,
    seq: i64,
    actor: String,
    reason: String,
    at_ms: i64,
}

#[derive(Deserialize)]
struct ReservationRow {
    id: String,
    /// Read but not used: the venue is already decided by the object this
    /// record came out of. It stays because a read model that omits a field
    /// stops describing the record.
    #[allow(dead_code)]
    location_id: String,
    party: i64,
    slot_min: i64,
    occasion: String,
    contact_name: String,
    contact_phone: String,
    status: String,
    created_at_ms: i64,
    /// The table, when the guest chose one. `default` because every booking
    /// written before the floor existed has neither field, and a read model
    /// that refused those would make the venue's history unreadable.
    #[serde(default)]
    zone_id: Option<String>,
    #[serde(default)]
    table_n: Option<i64>,
}

/// Replay a reservation's events through the kernel.
///
/// Returns the folded status. An event naming a status the kernel does not know
/// stops the replay — a booking whose history cannot be read must not be shown
/// as if it could.
fn fold_status(events: &[EventRow]) -> std::result::Result<ReservationStatus, String> {
    let mut steps = Vec::with_capacity(events.len());
    for e in events {
        let s = ReservationStatus::from_str(&e.to_status)
            .ok_or_else(|| format!("unknown status {:?} in event log", e.to_status))?;
        steps.push(s);
    }
    // The first event IS the creation, so the fold starts at the state it names
    // and replays the rest.
    let Some((first, rest)) = steps.split_first() else {
        return Err("reservation has no events".into());
    };
    reservation::fold_transitions(*first, rest)
        .map_err(|(e, reached)| format!("{} (stopped at {})", e.message(), reached.as_str()))
}

/// The venue's bookings: the reservations and their events, in ONE image.
///
/// ONE AGGREGATE, ONE TRANSACTION. A reservation and its event log are the same
/// thing changing — the status is the FOLD of the events — so putting them in
/// two images would mean two generations and a window in which a reservation
/// says one status and its events another. In one image a transition is one
/// write, and the old `db.batch([...])` of two statements stops being two
/// statements that could half-apply.
pub const IMAGE_BOOKINGS: &str = "bookings";
pub const BOOKINGS_BYTES: usize = 2 * 1024 * 1024;

const K_RSV: &str = "rsv";
const K_EV: &str = "ev";

/// An event's key. ZERO-PADDED, because keys sort as strings and `.../10` must
/// not come before `.../2` — that would replay a reservation's history in the
/// wrong order and the fold would reach a different status.
pub(crate) fn ev_key(reservation_id: &str, seq: i64) -> String {
    format!("{reservation_id}/{seq:012}")
}

/// The index that answers "this user's bookings, newest slot first".
///
/// `ORDER BY slot_min DESC` is a sorted prefix scan over the COMPLEMENT of the
/// slot, because keys sort ascending and the answer wants descending. The
/// complement is taken against a number comfortably past any slot this product
/// will mint, and it is derived here rather than written so the next person can
/// check it: minutes in ten thousand years.
const SLOT_MAX: i64 = 10_000 * 365 * 24 * 60;

pub(crate) fn user_key(user_id: &str, slot_min: i64, id: &str) -> String {
    format!("rsv.user/{user_id}/{:012}/{id}", SLOT_MAX - slot_min)
}

async fn load_bookings(place: &crate::hubstore::Place) -> Result<dowiz_hub::table::Table> {
    Ok(crate::hubstore::load_table(place, IMAGE_BOOKINGS, BOOKINGS_BYTES).await?.table)
}

/// One reservation, or nothing.
///
/// `location_id` IS NOT IN THE QUESTION any more, and that is the point: the
/// old read bound it into the WHERE clause so "a booking belonging to another
/// venue must not be readable through this venue's host". The image is this
/// venue's object, so there is no other venue's booking in it to exclude.
fn reservation_of(t: &dowiz_hub::table::Table, id: &str) -> Option<ReservationRow> {
    t.get(K_RSV, id).and_then(|j| serde_json::from_str(&j).ok())
}

/// A reservation's events, oldest first, which is the order the kernel replays.
fn events_of(t: &dowiz_hub::table::Table, reservation_id: &str) -> Vec<EventRow> {
    let prefix = format!("{reservation_id}/");
    let mut rows: Vec<EventRow> = t
        .all(K_EV)
        .into_iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .filter_map(|(_, j)| serde_json::from_str(&j).ok())
        .collect();
    rows.sort_by_key(|e: &EventRow| e.seq);
    rows
}

/// `GET /api/public/locations/:slug/reservations/:id`
pub async fn detail(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. See `auth::principal_at`: this family
    // was mounted under `/api/public/` with no guard at all.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await
    {
        return Ok(r);
    }

    let t = load_bookings(&place).await?;
    let Some(row) = reservation_of(&t, &id) else {
        return Response::error("not found", 404);
    };
    let events = events_of(&t, &id);

    let folded = fold_status(&events);
    let (status, drift) = match &folded {
        Ok(s) => (s.as_str().to_string(), s.as_str() != row.status),
        // A history that will not replay is reported as such rather than
        // papered over with the cached column.
        Err(why) => return Response::error(format!("reservation unreadable: {why}"), 500),
    };

    Response::from_json(&json!({
        "id": row.id,
        "party": row.party,
        "slotMin": row.slot_min,
        "occasion": row.occasion,
        "contactName": row.contact_name,
        "contactPhone": row.contact_phone,
        "status": status,
        "zoneId": row.zone_id,
        "tableN": row.table_n,
        // Loud, not silent: the cache disagreeing with the log is a fact the
        // caller gets to see.
        "statusCacheDrifted": drift,
        "createdAtMs": row.created_at_ms,
        "history": events.iter().map(|e| json!({
            "status": e.to_status, "seq": e.seq, "actor": e.actor,
            "reason": e.reason, "atMs": e.at_ms,
        })).collect::<Vec<_>>(),
    }))
}

/// `GET /api/public/locations/:slug/reservations?user=<id>`
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let url = req.url()?;
    let user = url
        .query_pairs()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v.to_string());
    let Some(user) = user else {
        return Response::error("missing user", 400);
    };

    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. `?user=` was a client-declared
    // identity: anyone could list anyone's reservations by naming them.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await
    {
        return Ok(r);
    }
    let t = load_bookings(&place).await?;
    // The index IS the `ORDER BY slot_min DESC`: the key holds the complement
    // of the slot, so the sorted scan comes back newest first and the limit is
    // a `take`.
    let rows: Vec<ReservationRow> = t
        .scan(&format!("rsv.user/{user}/"))
        .into_iter()
        .take(100)
        .filter_map(|(_, id)| reservation_of(&t, &id))
        .collect();

    Response::from_json(&json!({
        "reservations": rows.iter().map(|r| json!({
            "id": r.id, "party": r.party, "slotMin": r.slot_min,
            "occasion": r.occasion, "status": r.status,
            "zoneId": r.zone_id, "tableN": r.table_n,
        })).collect::<Vec<_>>()
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Writing
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateBody {
    party: u16,
    #[serde(rename = "slotMin")]
    slot_min: i64,
    #[serde(default)]
    occasion: String,
    #[serde(default, rename = "contactName")]
    contact_name: String,
    #[serde(default, rename = "contactPhone")]
    contact_phone: String,
    #[serde(default, rename = "userId")]
    user_id: Option<String>,
    /// The table the guest tapped on the plan. BOTH OR NEITHER: a zone with no
    /// table number names no table, and half a choice written into a booking
    /// is a booking the floor cannot check.
    #[serde(default, rename = "zoneId")]
    zone_id: Option<String>,
    #[serde(default, rename = "tableN")]
    table_n: Option<i64>,
    /// The caller's own key. Sending the same one twice returns the same
    /// booking rather than making a second — a retried request must not book a
    /// second table.
    #[serde(rename = "requestId")]
    request_id: String,
}

/// `POST /api/public/locations/:slug/reservations`
pub async fn create(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: CreateBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    if body.request_id.trim().is_empty() {
        return Response::error("requestId is required", 400);
    }

    let place = match crate::hubstore::Place::of_slug(&ctx, &slug).await {
        Ok(p) => p,
        Err(e) => return Response::error(format!("create/place: {e}"), 500),
    };
    // AUTHENTICATED, AND TO THIS VENUE. A booking form open to the world is a
    // way to fill a restaurant with tables nobody will sit at, under names
    // nobody chose; when a public one exists it will mint a token the way the
    // storefront does for an order.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await
    {
        return Ok(r);
    }

    // The request id IS the reservation id, scoped to the venue. Replaying the
    // request finds the existing row and returns it.
    let id = format!("rsv_{:016x}", id64(&format!("{}:{}", place.venue, body.request_id)));
    let t = load_bookings(&place).await?;
    if let Some(r) = reservation_of(&t, &id) {
        return Response::from_json(&json!({
            "id": r.id, "status": r.status, "replayed": true
        }));
    }

    // ── THE DECISION IS THE KERNEL'S ──
    let request = ReservationRequest {
        id: id64(&id),
        venue: place.venue.clone(),
        party: body.party,
        slot_min: body.slot_min,
        occasion: body.occasion.clone(),
    };
    let policy = BookingPolicy::default_policy();
    if let Err(e) = reservation::validate_request(&request, &policy, now_min(ctx.data.now_ms)) {
        return Response::error(e.message(), 422);
    }

    // ── AND THE FLOOR IS THE SECOND DECISION ──
    // The kernel says whether a party and a time are a legal booking. It does
    // not know the room is a finite number of tables, so that is asked here.
    let table = match (body.zone_id.as_deref().map(str::trim), body.table_n) {
        (Some(z), Some(n)) if !z.is_empty() => Some((z.to_string(), n)),
        (None, None) | (Some(""), None) => None,
        // Half a choice written into a booking is a booking the floor cannot
        // check -- and it would look accepted.
        _ => return Response::error("a table is a zoneId AND a tableN, or neither", 400),
    };
    let plan = floor_plan(&place).await?;
    if table.is_some() && plan.is_empty() {
        return Response::error(
            "this venue has published no floor plan, so a booking here cannot name a table",
            409,
        );
    }

    let now = ctx.data.now_ms;
    // A GUEST BOOKING HAS NO USER, and that used to be a 500: the column
    // references `users(id)` and an empty string names no user, so SQLite
    // rejected it against the foreign key. Here the absence is just an absence
    // -- and it means no `rsv.user/` index entry, because nobody's list it
    // belongs on.
    let user_id = body
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string);
    let rsv = json!({
        "id": id, "location_id": place.venue, "party": body.party as i64,
        "slot_min": body.slot_min, "occasion": body.occasion,
        "contact_name": body.contact_name, "contact_phone": body.contact_phone,
        "status": "REQUESTED", "created_at_ms": now, "user_id": user_id,
        "zone_id": table.as_ref().map(|(z, _)| z.clone()),
        "table_n": table.as_ref().map(|(_, n)| *n),
    })
    .to_string();
    let ev = json!({
        "to_status": "REQUESTED", "seq": 1, "actor": "CUSTOMER", "reason": "", "at_ms": now,
    })
    .to_string();
    let (rid, slot) = (id.clone(), body.slot_min);
    let party = body.party as i64;
    let picked = table.clone();
    // ONE WRITE. This was `db.batch` of two statements — the reservation and
    // its first event — and a batch that half-applies leaves a booking with no
    // history, which `fold_status` then reports as "reservation has no events".
    //
    // THE FLOOR IS CHECKED INSIDE THIS CLOSURE, against the image about to be
    // written. A check done against the earlier read would be a window: two
    // requests a millisecond apart would both pass it and both write, and the
    // venue would learn about it when two parties arrived for one table.
    // `with_table` re-runs the closure when the generation moved, so the
    // verdict is re-taken on the state that actually wins.
    let outcome = crate::hubstore::with_table(
        &place,
        IMAGE_BOOKINGS,
        BOOKINGS_BYTES,
        move |t| {
            let mut index: Vec<(String, String)> = match &user_id {
                Some(u) => vec![(user_key(u, slot, &rid), rid.clone())],
                None => vec![],
            };
            let mut unique: Vec<String> = Vec::new();
            if let Some((z, n)) = &picked {
                if let Some(why) =
                    table_verdict(&plan, &held_tables(t), z, *n, party, slot, &rid)
                {
                    return Ok(Some(why));
                }
                let k = table_key(z, *n, slot);
                unique.push(k.clone());
                index.push((k, rid.clone()));
            }
            let u: Vec<&str> = unique.iter().map(String::as_str).collect();
            t.put(K_RSV, &rid, &rsv, &index, &u)
                .map_err(|x| Error::RustError(format!("booking: {x}")))?;
            t.put(K_EV, &ev_key(&rid, 1), &ev, &[], &[])
                .map_err(|x| Error::RustError(format!("booking event: {x}")))?;
            Ok(None)
        },
    )
    .await;
    match outcome {
        Ok(None) => {}
        // 409, not 400: the request is well formed and the floor is simply not
        // free. The message names the table, because a guest told only "not
        // available" does not know the next table over is empty.
        Ok(Some(why)) => return Response::error(why, 409),
        // Named, not swallowed. A bare 500 here cost a deploy cycle to diagnose
        // because the router renders an `Err` with no body at all.
        Err(e) => return Response::error(format!("create/write: {e}"), 500),
    }

    Response::from_json(&json!({
        "id": id, "status": "REQUESTED", "replayed": false,
        "zoneId": table.as_ref().map(|(z, _)| z.clone()),
        "tableN": table.as_ref().map(|(_, n)| *n),
    }))
}

#[derive(Deserialize)]
struct ActionBody {
    /// The status to move into, in the kernel's wire form.
    to: String,
    /// CUSTOMER | VENUE | SYSTEM
    #[serde(default = "default_actor")]
    actor: String,
    #[serde(default)]
    reason: String,
}

fn default_actor() -> String {
    "CUSTOMER".into()
}

/// `POST /api/public/locations/:slug/reservations/:id/action`
///
/// The transition is checked by the kernel against the FOLDED state, never
/// against the cached column — otherwise a stale cache would authorise a move
/// the history forbids.
pub async fn action(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let body: ActionBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE, BEFORE THE BODY IS JUDGED. This route
    // drives the reservation's state machine and writes `body.actor` into the
    // audit trail, so an unauthenticated caller could both cancel a stranger's
    // table and sign somebody else's name to it. The guard is first because a
    // caller with no token should learn nothing at all -- not even which
    // status names this kernel knows.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await
    {
        return Ok(r);
    }
    let Some(to) = ReservationStatus::from_str(&body.to) else {
        return Response::error(format!("unknown status {:?}", body.to), 400);
    };

    let t = load_bookings(&place).await?;
    if reservation_of(&t, &id).is_none() {
        return Response::error("not found", 404);
    }

    let events = events_of(&t, &id);
    let current = match fold_status(&events) {
        Ok(s) => s,
        Err(why) => return Response::error(format!("reservation unreadable: {why}"), 500),
    };

    if let Err(e) = reservation::assert_transition(current, to) {
        // 409, not 400: the request is well formed and the booking is simply
        // not in a state where it can happen.
        return Response::error(e.message(), 409);
    }

    let seq = events.iter().map(|e| e.seq).max().unwrap_or(0) + 1;
    let now = ctx.data.now_ms;
    let ev = json!({
        "to_status": to.as_str(), "seq": seq,
        "actor": body.actor, "reason": body.reason, "at_ms": now,
    })
    .to_string();
    let (rid, status) = (id.clone(), to.as_str().to_string());
    // THE EVENT AND THE STATUS IN ONE WRITE. They were two statements in a
    // batch, and the status is the FOLD of the events — so a half-applied
    // batch is a reservation whose stored status disagrees with its own
    // history, which is the exact divergence the replay gate exists to catch.
    crate::hubstore::with_table(&place, IMAGE_BOOKINGS, BOOKINGS_BYTES, move |t| {
        t.put(K_EV, &ev_key(&rid, seq), &ev, &[], &[])
            .map_err(|x| Error::RustError(format!("booking event: {x}")))?;
        if let Some(mut r) = t.get(K_RSV, &rid).and_then(|j| {
            serde_json::from_str::<serde_json::Value>(&j).ok()
        }) {
            r["status"] = json!(status);
            r["updated_at_ms"] = json!(now);
            // The index keys are recomputed from the record, so a status change
            // cannot silently drop the entry that puts this booking on its
            // owner's list.
            let slot_min =
                r.get("slot_min").and_then(serde_json::Value::as_i64).unwrap_or(0);
            let mut index: Vec<(String, String)> = r
                .get("user_id")
                .and_then(serde_json::Value::as_str)
                .map(|u| vec![(user_key(u, slot_min, &rid), rid.clone())])
                .unwrap_or_default();
            // AND THIS IS HOW A TABLE IS RELEASED. `Table::put` replaces the
            // keys the old record owned with exactly this list, so a booking
            // that reaches a status which no longer holds its table simply
            // stops writing the key -- there is no separate "free the table"
            // step anybody can forget, and no window where the record says
            // cancelled while the floor still says taken.
            if floor::holds_table(&status) {
                if let (Some(z), Some(n)) = (
                    r.get("zone_id").and_then(serde_json::Value::as_str),
                    r.get("table_n").and_then(serde_json::Value::as_i64),
                ) {
                    index.push((table_key(z, n, slot_min), rid.clone()));
                }
            }
            t.put(K_RSV, &rid, &r.to_string(), &index, &[])
                .map_err(|x| Error::RustError(format!("booking: {x}")))?;
        }
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "id": id, "status": to.as_str(), "seq": seq }))
}


// ─────────────────────────────────────────────────────────────────────────────
// The floor
// ─────────────────────────────────────────────────────────────────────────────

/// Where the owner's floor plan lives: a field on the venue's own record, in
/// the catalogue image, beside `delivery_zones`. The same place and the same
/// shape as `services/venue/zones.rs` puts the delivery boundary, for the same
/// reason -- it is a fact about the venue, not about any one booking, and an
/// owner must be able to change it without a deploy.
const PLAN_FIELD: &str = "floor_plan";

/// The index key that holds a table for a slot.
///
/// ZERO-PADDED for the reason `ev_key` is: keys sort as strings, and the scan
/// that reads these back parses the slot out of the key rather than opening
/// every reservation. `{n:06}` keeps table 10 after table 2.
///
/// NO RESERVATION ID IN IT, deliberately, so the key can be declared UNIQUE to
/// `Table::put` -- which refuses a second record claiming it inside the
/// object's own turn. That is the backstop under the check in `create`.
pub(crate) fn table_key(zone: &str, n: i64, slot_min: i64) -> String {
    format!("rsv.tbl/{zone}/{n:06}/{slot_min:012}")
}

/// Every table currently held, READ BACK OUT OF THE KEYS.
///
/// The key carries the zone, the table and the slot, so the whole floor's
/// occupancy is one sorted prefix scan -- no reservation record is opened at
/// all. A key that does not parse is skipped rather than panicking: an
/// unreadable index entry must not take the venue's booking page down.
fn held_tables(t: &dowiz_hub::table::Table) -> Vec<floor::Held> {
    t.scan("rsv.tbl/")
        .into_iter()
        .filter_map(|(k, id)| {
            let mut p = k.split('/');
            let (_, zone, n, slot) = (p.next()?, p.next()?, p.next()?, p.next()?);
            Some(floor::Held {
                zone: zone.to_string(),
                // `"000012".parse()` is 12: the padding does not need stripping.
                n: n.parse().ok()?,
                slot_min: slot.parse().ok()?,
                reservation: id,
            })
        })
        .collect()
}

/// The venue's floor plan, or an empty one.
///
/// `venue_record` rather than `load_catalog`: the object parses its own
/// catalogue and answers with the venue row, so reading the plan does not pull
/// half a megabyte of menu across. A plan that will not parse is reported as
/// EMPTY and named in the log rather than guessed at -- but it cannot get in:
/// [`set_plan`] parses it before it is stored.
async fn floor_plan(place: &crate::hubstore::Place) -> Result<floor::Plan> {
    let rec = crate::hubstore::venue_record(place).await?;
    let raw = rec
        .as_ref()
        .and_then(|r| r.get(PLAN_FIELD))
        .map(|v| v.to_string())
        .unwrap_or_default();
    Ok(floor::from_json(&raw).unwrap_or_default())
}

/// May this party have this table at this minute? `None` is yes.
///
/// PURE, AND SEPARATE FROM THE HANDLER, so the refusal can be tested without a
/// Worker runtime -- which is what `key_tests` at the foot of this file exists
/// to do and what the double-booking RED proof exercises.
///
/// EVERY REFUSAL NAMES THE TABLE. "That time is not available" sends a guest
/// away without telling them the next table over is free.
fn table_verdict(
    plan: &floor::Plan,
    held: &[floor::Held],
    zone: &str,
    n: i64,
    party: i64,
    slot_min: i64,
    me: &str,
) -> Option<String> {
    let Some(t) = plan.find(zone, n) else {
        return Some(format!("there is no table {n} in zone {zone:?} on this venue's plan"));
    };
    if !t.seats_party(party) {
        return Some(format!(
            "table {n} in zone {zone:?} seats {}; a party of {party} needs a larger table",
            t.seats
        ));
    }
    match floor::holder(held, zone, n, slot_min, floor::DWELL_MIN) {
        // A RETRIED REQUEST IS NOT A DOUBLE BOOKING. The same `requestId`
        // produces the same reservation id, and the hold it already owns must
        // not be read as somebody else's.
        Some(h) if h.reservation == me => None,
        Some(h) => Some(format!(
            "table {n} in zone {zone:?} is already booked for minute {} and is held for \
             {} minutes; choose another table or another time",
            h.slot_min,
            floor::DWELL_MIN
        )),
        None => None,
    }
}

#[derive(Deserialize)]
struct PlanIn {
    /// The zones as the owner drew them. An EMPTY list removes the plan, which
    /// is how a venue that does not seat by table turns the feature off.
    zones: Vec<serde_json::Value>,
}

/// `POST /api/owner/floorplan`
///
/// PARSED BACK BEFORE IT IS STORED, and the refusal is the reader's own
/// sentence. `services/venue/zones.rs` set this pattern because a zone the
/// reader cannot understand is NO zone and silently turns the check off; a
/// table the reader cannot understand is worse -- the table is still there,
/// with guests at it, while the hub believes the room is smaller.
pub async fn set_plan(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: PlanIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let doc = json!({ "zones": body.zones }).to_string();
    let parsed = match floor::from_json(&doc) {
        Ok(p) => p,
        Err(e) => return Response::error(format!("floor plan refused: {}", e.message()), 400),
    };
    let (zones, tables) =
        (parsed.zones.len(), parsed.zones.iter().map(|z| z.tables.len()).sum::<usize>());
    let stored = body.zones.clone();
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: serde_json::Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        l[PLAN_FIELD] = json!({ "zones": stored });
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "zones": zones, "tables": tables }))
}

/// `GET /api/public/locations/:slug/tables?slotMin=<n>&party=<n>`
///
/// THE SLOT IS REQUIRED and that is the model, not a validation preference: a
/// table is free or taken only for a minute, so an answer without one would be
/// a lie the surface would then draw.
///
/// PUBLIC, unlike the rest of this family. It is the venue's own furniture and
/// the same "is there room tonight" a phone call answers; it carries no name,
/// no party and no reservation id -- only how many of the venue's own tables
/// are free. The guard stays on everything that WRITES.
pub async fn availability(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let url = req.url()?;
    let q = |name: &str| -> Option<i64> {
        url.query_pairs().find(|(k, _)| k == name).and_then(|(_, v)| v.parse::<i64>().ok())
    };
    let Some(slot_min) = q("slotMin") else {
        return Response::error(
            "slotMin is required: a table is free or taken only for a slot",
            400,
        );
    };
    let party = q("party").unwrap_or(1);
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;

    // THE SAME RULE THE WRITE USES. Asking the floor about a slot the kernel
    // would refuse would draw a plan the guest cannot book from.
    let request = ReservationRequest {
        id: 0,
        venue: place.venue.clone(),
        party: party.clamp(0, u16::MAX as i64) as u16,
        slot_min,
        occasion: String::new(),
    };
    if let Err(e) = reservation::validate_request(
        &request,
        &BookingPolicy::default_policy(),
        now_min(ctx.data.now_ms),
    ) {
        return Response::error(e.message(), 422);
    }

    let plan = floor_plan(&place).await?;
    let t = load_bookings(&place).await?;
    let held = held_tables(&t);
    let standing = floor::availability(&plan, &held, party, slot_min, floor::DWELL_MIN);
    let at = |zone: &str, n: i64| standing.iter().find(|s| s.zone == zone && s.n == n).cloned();

    Response::from_json(&json!({
        "slotMin": slot_min,
        "party": party,
        "dwellMin": floor::DWELL_MIN,
        "planW": floor::PLAN_W,
        "planH": floor::PLAN_H,
        "zones": plan.zones.iter().map(|z| json!({
            "id": z.id,
            "name": z.name,
            "tables": z.tables.iter().map(|t| {
                let s = at(&z.id, t.n);
                json!({
                    "n": t.n, "x": t.x, "y": t.y, "w": t.w, "h": t.h,
                    "seats": t.seats, "shape": t.shape.as_str(),
                    "occupied": s.as_ref().is_some_and(|s| s.occupied),
                    "tooSmall": s.as_ref().is_some_and(|s| s.too_small),
                })
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry passes
// ─────────────────────────────────────────────────────────────────────────────


/// Fetch the venue's pass key, minting one on first use.
///
/// The key never leaves this Worker: a pass goes out, the key does not.
async fn pass_key(place: &crate::hubstore::Place) -> Result<Vec<u8>> {
    // The venue's settings image, under a key whose LAST SEGMENT IS `key` --
    // which is what `settings::is_secret` matches on, so it is redacted
    // everywhere settings are read back for display. That rule is by shape
    // rather than by a list precisely so a secret added later cannot be
    // forgotten, and this is the first one to rely on it.
    const KEY: &str = "venue.pass.key";
    let loaded = crate::hubstore::load_settings(place).await?;
    if let Some(b64) = loaded.settings.get(KEY).filter(|v| !v.is_empty()) {
        return base64_decode(&b64)
            .ok_or_else(|| Error::RustError("venue pass key is not base64".into()));
    }

    // First use: mint 32 bytes from the runtime's CSPRNG. Not from a hash of the
    // venue id — that would make every deployment's keys derivable from public
    // data.
    let mut key = [0u8; 32];
    getrandom_fill(&mut key)?;
    let b64 = base64_encode(&key);
    // ON CONFLICT DO NOTHING, as a read inside the object's own turn: two
    // requests minting at once must not end with two keys, because a pass
    // signed with one would not verify against the other.
    let mine = b64.clone();
    let settled = crate::hubstore::with_settings(place, move |s| {
        match s.get(KEY).filter(|v| !v.is_empty()) {
            Some(existing) => Ok(existing),
            None => {
                s.set(KEY, &mine);
                Ok(mine.clone())
            }
        }
    })
    .await?;
    base64_decode(&settled)
        .ok_or_else(|| Error::RustError("venue pass key is not base64".into()))
}

fn getrandom_fill(buf: &mut [u8]) -> Result<()> {
    // The Workers runtime exposes the Web Crypto CSPRNG.
    let crypto = worker::js_sys::global()
        .dyn_into::<worker::js_sys::Object>()
        .map_err(|_| Error::RustError("no global object".into()))?;
    let subtle = worker::js_sys::Reflect::get(&crypto, &"crypto".into())
        .map_err(|_| Error::RustError("no crypto".into()))?;
    let arr = worker::js_sys::Uint8Array::new_with_length(buf.len() as u32);
    let f = worker::js_sys::Reflect::get(&subtle, &"getRandomValues".into())
        .map_err(|_| Error::RustError("no getRandomValues".into()))?;
    let f: worker::js_sys::Function = f
        .dyn_into()
        .map_err(|_| Error::RustError("getRandomValues is not callable".into()))?;
    f.call1(&subtle, &arr)
        .map_err(|_| Error::RustError("getRandomValues failed".into()))?;
    arr.copy_to(buf);
    Ok(())
}

use worker::wasm_bindgen::JsCast;

fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}

/// `GET /api/public/locations/:slug/reservations/:id/pass`
///
/// Only a CONFIRMED booking gets a pass. Minting one for a request the venue has
/// not answered would put a code on a phone that the door will refuse.
pub async fn issue_pass(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. See `auth::principal_at`: this family
    // was mounted under `/api/public/` with no guard at all.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await
    {
        return Ok(r);
    }

    let t = load_bookings(&place).await?;
    let Some(row) = reservation_of(&t, &id) else {
        return Response::error("not found", 404);
    };
    let events = events_of(&t, &id);
    let status = match fold_status(&events) {
        Ok(s) => s,
        Err(why) => return Response::error(format!("reservation unreadable: {why}"), 500),
    };
    if status != ReservationStatus::Confirmed {
        return Response::error(
            format!(
                "a pass is issued for a confirmed booking; this one is {}",
                status.as_str()
            ),
            409,
        );
    }

    let key = pass_key(&place).await?;
    let claims = PassClaims {
        venue: id64(&place.venue),
        reservation: id64(&row.id),
        slot_min: row.slot_min,
        party: row.party as u16,
        // The event count: a reissued pass after a change differs from the one
        // it replaces, which is what `nonce` is for.
        nonce: events.len() as u64,
    };
    let minted = pass::issue(&key, claims)
        .map_err(|e| Error::RustError(format!("pass: {}", e.message())))?;

    Response::from_json(&json!({
        "code": pass::encode(&minted),
        "reservationId": row.id,
        "slotMin": row.slot_min,
        "party": row.party,
    }))
}

#[derive(Deserialize)]
struct VerifyBody {
    code: String,
}

/// `POST /api/public/locations/:slug/pass/verify`
///
/// The venue's scanner. Every refusal is named: a scanner that can only say
/// "invalid" sends people away without telling them they are simply early.
pub async fn verify_pass(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: VerifyBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };

    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let key = pass_key(&place).await?;

    let decoded = match pass::decode(&body.code) {
        Ok(p) => p,
        Err(e) => {
            return Response::from_json(&json!({ "ok": false, "why": e.message() }));
        }
    };
    match pass::verify(
        &key,
        &decoded,
        id64(&place.venue),
        &PassWindow::default_window(),
        now_min(ctx.data.now_ms),
    ) {
        Ok(()) => Response::from_json(&json!({
            "ok": true,
            "reservation": decoded.claims.reservation,
            "party": decoded.claims.party,
            "slotMin": decoded.claims.slot_min,
        })),
        // 200 with `ok:false`: the request succeeded, the pass did not. A 4xx
        // here would make a scanner treat a wrong code as a broken scanner.
        Err(e) => Response::from_json(&json!({ "ok": false, "why": e.message() })),
    }
}

#[cfg(test)]
mod key_tests {
    use super::{ev_key, now_min, user_key, SLOT_MAX};

    /// WHAT P3 ACTUALLY BUYS, stated as a test. `now_min` read the wall clock
    /// until 2026-09-22, so a booking window could only be exercised at the
    /// real time — which is why the rule that decides whether a slot is in the
    /// past had no test at all. It takes the instant now, so the boundary can
    /// be put wherever the test wants it.
    #[test]
    fn a_slot_boundary_can_be_examined_at_a_chosen_instant() {
        // The minute the epoch's first hour ends, and the millisecond before.
        assert_eq!(now_min(3_600_000), 60);
        assert_eq!(now_min(3_599_999), 59, "the last millisecond is still the previous minute");
        // A second read of a real clock could land either side of that line;
        // one instant handed down cannot.
        assert_eq!(now_min(3_599_999), now_min(3_599_999));
    }

    /// AND IT IS NOT A DIVISION THAT ROUNDS TOWARDS ZERO BY ACCIDENT. Every
    /// instant this sees is after 1970, so the behaviour below the epoch is
    /// not a rule anyone relies on — it is pinned so that a future change to
    /// signed arithmetic is a decision rather than a surprise.
    #[test]
    fn the_epoch_itself_is_minute_zero() {
        assert_eq!(now_min(0), 0);
        assert_eq!(now_min(59_999), 0);
        assert_eq!(now_min(60_000), 1);
    }

    /// KEYS SORT AS STRINGS, and a reservation's history replays in key order.
    #[test]
    fn event_keys_sort_numerically_because_they_are_padded() {
        let mut keys: Vec<String> = (1..=12).map(|n| ev_key("rsv_a", n)).collect();
        keys.sort();
        // Unpadded, "rsv_a/10" sorts before "rsv_a/2" and the fold would replay
        // the tenth transition third — reaching a different status.
        assert_eq!(keys[0], ev_key("rsv_a", 1));
        assert_eq!(keys[1], ev_key("rsv_a", 2));
        assert_eq!(keys[11], ev_key("rsv_a", 12));
    }

    #[test]
    fn a_reservations_events_do_not_collide_with_another_reservations() {
        assert!(ev_key("rsv_a", 1) != ev_key("rsv_b", 1));
        assert!(ev_key("rsv_a", 1).starts_with("rsv_a/"));
    }

    /// `ORDER BY slot_min DESC` as an ASCENDING scan over the complement.
    #[test]
    fn a_users_bookings_scan_newest_slot_first() {
        let mut keys: Vec<String> = [100i64, 5000, 250, 99999]
            .iter()
            .map(|s| user_key("u1", *s, "id"))
            .collect();
        keys.sort();
        assert_eq!(keys[0], user_key("u1", 99999, "id"), "the latest slot comes first");
        assert_eq!(keys[3], user_key("u1", 100, "id"), "the earliest comes last");
    }

    #[test]
    fn one_users_scan_prefix_cannot_reach_another_users_bookings() {
        assert!(user_key("u1", 10, "a").starts_with("rsv.user/u1/"));
        assert!(!user_key("u2", 10, "a").starts_with("rsv.user/u1/"));
    }

    /// The complement stays positive for every slot this product can mint, and
    /// the padding stays wide enough that it does not itself sort wrongly.
    #[test]
    fn the_complement_is_positive_and_fits_its_padding() {
        let far = 60i64 * 24 * 365 * 100; // a century of minutes
        assert!(SLOT_MAX - far > 0);
        assert_eq!(format!("{:012}", SLOT_MAX).len(), 12, "SLOT_MAX overflows its own padding");
    }
}

#[cfg(test)]
mod floor_tests {
    use super::{table_key, table_verdict};
    use dowiz_hub::tables::{from_json, Held, Plan, DWELL_MIN};

    /// Two tables in one room, one of them a four-top.
    fn plan() -> Plan {
        from_json(
            r#"{"zones":[{"id":"terasa","name":"Тераса","tables":[
                 {"n":1,"x":85,"y":110,"w":44,"h":44,"seats":4},
                 {"n":2,"x":155,"y":130,"w":40,"h":40,"seats":2}]}]}"#,
        )
        .expect("the fixture plan must read")
    }

    fn held(n: i64, slot_min: i64, reservation: &str) -> Held {
        Held { zone: "terasa".into(), n, slot_min, reservation: reservation.into() }
    }

    /// THE DEFECT THIS CLOSES, as one test. Before the floor existed a
    /// reservation was a party size and a time, so the SAME TABLE at the SAME
    /// MINUTE was accepted twice and the venue found out when two parties
    /// walked in. Deleting the `floor::holder` arm of `table_verdict` turns
    /// this red; nothing else in the suite notices.
    #[test]
    fn the_same_table_at_the_same_minute_is_refused_the_second_time() {
        let (p, slot) = (plan(), 29_000_000i64);
        // The first booking meets an empty floor and is allowed.
        assert_eq!(table_verdict(&p, &[], "terasa", 1, 2, slot, "rsv_first"), None);
        // The second one, for the same table and slot, is not.
        let why = table_verdict(
            &p,
            &[held(1, slot, "rsv_first")],
            "terasa",
            1,
            2,
            slot,
            "rsv_second",
        )
        .expect("a table already held must be REFUSED, not booked twice");
        // AND THE REFUSAL NAMES THE TABLE. "not available" sends a guest away
        // without telling them table 2 is free.
        assert!(why.contains("table 1"), "the refusal must name the table: {why}");
        assert!(why.contains("terasa"), "and the zone: {why}");
        assert!(why.contains(&slot.to_string()), "and the slot that took it: {why}");
        // The next table over is still free at that minute.
        assert_eq!(table_verdict(&p, &[held(1, slot, "rsv_first")], "terasa", 2, 2, slot, "x"), None);
    }

    /// OCCUPIED IS PER SLOT. The same table, two hours later, is free.
    #[test]
    fn a_table_held_at_one_hour_is_bookable_at_another() {
        let (p, slot) = (plan(), 29_000_000i64);
        let booked = [held(1, slot, "rsv_first")];
        assert!(table_verdict(&p, &booked, "terasa", 1, 2, slot + 15, "x").is_some(),
                "fifteen minutes later is the same sitting");
        assert_eq!(
            table_verdict(&p, &booked, "terasa", 1, 2, slot + DWELL_MIN, "x"),
            None,
            "past the dwell the table has turned over"
        );
    }

    /// A RETRIED REQUEST IS NOT A DOUBLE BOOKING. `create` derives the
    /// reservation id from the caller's `requestId`, so a retry arrives
    /// holding its own table -- and must not be refused by it.
    #[test]
    fn a_reservation_does_not_collide_with_its_own_hold() {
        let (p, slot) = (plan(), 29_000_000i64);
        assert_eq!(
            table_verdict(&p, &[held(1, slot, "rsv_me")], "terasa", 1, 2, slot, "rsv_me"),
            None
        );
    }

    #[test]
    fn a_party_too_large_for_the_table_is_refused_by_name() {
        let (p, slot) = (plan(), 29_000_000i64);
        let why = table_verdict(&p, &[], "terasa", 2, 4, slot, "x").expect("a deuce seats two");
        assert!(why.contains("table 2") && why.contains("seats 2"), "{why}");
        assert_eq!(table_verdict(&p, &[], "terasa", 1, 4, slot, "x"), None, "the four-top takes it");
    }

    #[test]
    fn a_table_that_is_not_on_the_plan_is_refused() {
        let (p, slot) = (plan(), 29_000_000i64);
        assert!(table_verdict(&p, &[], "terasa", 9, 2, slot, "x").unwrap().contains("no table 9"));
        assert!(table_verdict(&p, &[], "zala", 1, 2, slot, "x").unwrap().contains("no table 1"));
    }

    /// THE KEYS THE HOLD IS MADE OF. They sort as strings and the scan in
    /// `held_tables` parses the slot back out of them, so the padding is
    /// load-bearing exactly as it is for `ev_key`.
    #[test]
    fn table_keys_sort_numerically_and_parse_back() {
        let mut keys: Vec<String> = [2i64, 10, 1].iter().map(|n| table_key("z", *n, 100)).collect();
        keys.sort();
        assert_eq!(keys, vec![table_key("z", 1, 100), table_key("z", 2, 100), table_key("z", 10, 100)]);
        let parts: Vec<&str> = keys[2].split('/').collect();
        assert_eq!(parts[0], "rsv.tbl");
        assert_eq!(parts[1], "z");
        assert_eq!(parts[2].parse::<i64>().unwrap(), 10, "the padding does not need stripping");
        assert_eq!(parts[3].parse::<i64>().unwrap(), 100);
    }

    /// ONE TABLE'S HOLDS ARE ONE PREFIX. `held_tables` scans `rsv.tbl/` whole,
    /// but the key must still keep one zone's tables out of another's range.
    #[test]
    fn one_zones_tables_cannot_be_read_as_anothers() {
        assert!(table_key("terasa", 1, 5).starts_with("rsv.tbl/terasa/"));
        assert!(!table_key("mala", 1, 5).starts_with("rsv.tbl/terasa/"));
    }
}
