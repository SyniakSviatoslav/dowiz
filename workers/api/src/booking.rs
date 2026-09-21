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

use crate::owner::now_ms;

/// Minutes since the Unix epoch — the kernel's unit for a slot.
fn now_min() -> i64 {
    now_ms() / 60_000
}

/// Bind an integer to D1.
///
/// NOT `i64::into()`. That produces a JavaScript **BigInt**, which the D1 driver
/// rejects — and rejects by throwing, so the Worker returns a bare 500 with no
/// body and nothing in the response says why. Every integer in this file goes
/// through here, as `accounts.rs` already does. Timestamps in milliseconds and
/// slot minutes are far below 2^53, so f64 carries them exactly.
fn num(n: i64) -> worker::wasm_bindgen::JsValue {
    worker::wasm_bindgen::JsValue::from_f64(n as f64)
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
    location_id: String,
    party: i64,
    slot_min: i64,
    occasion: String,
    contact_name: String,
    contact_phone: String,
    status: String,
    created_at_ms: i64,
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
fn ev_key(reservation_id: &str, seq: i64) -> String {
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

fn user_key(user_id: &str, slot_min: i64, id: &str) -> String {
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
pub async fn detail(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. See `auth::principal_at`: this family
    // was mounted under `/api/public/` with no guard at all.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await
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
pub async fn list(req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. `?user=` was a client-declared
    // identity: anyone could list anyone's reservations by naming them.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await
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
    /// The caller's own key. Sending the same one twice returns the same
    /// booking rather than making a second — a retried request must not book a
    /// second table.
    #[serde(rename = "requestId")]
    request_id: String,
}

/// `POST /api/public/locations/:slug/reservations`
pub async fn create(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

    let db = match ctx.d1("DB") {
        Ok(d) => d,
        Err(e) => return Response::error(format!("create/db: {e}"), 500),
    };
    let place = match crate::hubstore::Place::of_slug(&ctx, &slug).await {
        Ok(p) => p,
        Err(e) => return Response::error(format!("create/place: {e}"), 500),
    };
    // AUTHENTICATED, AND TO THIS VENUE. A booking form open to the world is a
    // way to fill a restaurant with tables nobody will sit at, under names
    // nobody chose; when a public one exists it will mint a token the way the
    // storefront does for an order.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await
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
    if let Err(e) = reservation::validate_request(&request, &policy, now_min()) {
        return Response::error(e.message(), 422);
    }

    let now = now_ms();
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
    })
    .to_string();
    let ev = json!({
        "to_status": "REQUESTED", "seq": 1, "actor": "CUSTOMER", "reason": "", "at_ms": now,
    })
    .to_string();
    let (rid, slot) = (id.clone(), body.slot_min);
    let index: Vec<(String, String)> = match &user_id {
        Some(u) => vec![(user_key(u, slot, &rid), rid.clone())],
        None => vec![],
    };
    // ONE WRITE. This was `db.batch` of two statements — the reservation and
    // its first event — and a batch that half-applies leaves a booking with no
    // history, which `fold_status` then reports as "reservation has no events".
    if let Err(e) = crate::hubstore::with_table(
        &place,
        IMAGE_BOOKINGS,
        BOOKINGS_BYTES,
        move |t| {
            t.put(K_RSV, &rid, &rsv, &index, &[])
                .map_err(|x| Error::RustError(format!("booking: {x}")))?;
            t.put(K_EV, &ev_key(&rid, 1), &ev, &[], &[])
                .map_err(|x| Error::RustError(format!("booking event: {x}")))?;
            Ok(())
        },
    )
    .await
    {
        // Named, not swallowed. A bare 500 here cost a deploy cycle to diagnose
        // because the router renders an `Err` with no body at all.
        return Response::error(format!("create/write: {e}"), 500);
    }

    Response::from_json(&json!({ "id": id, "status": "REQUESTED", "replayed": false }))
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
pub async fn action(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let body: ActionBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE, BEFORE THE BODY IS JUDGED. This route
    // drives the reservation's state machine and writes `body.actor` into the
    // audit trail, so an unauthenticated caller could both cancel a stranger's
    // table and sign somebody else's name to it. The guard is first because a
    // caller with no token should learn nothing at all -- not even which
    // status names this kernel knows.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await
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
    let now = now_ms();
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
            let index: Vec<(String, String)> = r
                .get("user_id")
                .and_then(serde_json::Value::as_str)
                .map(|u| {
                    vec![(
                        user_key(
                            u,
                            r.get("slot_min").and_then(serde_json::Value::as_i64).unwrap_or(0),
                            &rid,
                        ),
                        rid.clone(),
                    )]
                })
                .unwrap_or_default();
            t.put(K_RSV, &rid, &r.to_string(), &index, &[])
                .map_err(|x| Error::RustError(format!("booking: {x}")))?;
        }
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "id": id, "status": to.as_str(), "seq": seq }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry passes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct KeyRow {
    key_b64: String,
}

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
pub async fn issue_pass(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, AND TO THIS VENUE. See `auth::principal_at`: this family
    // was mounted under `/api/public/` with no guard at all.
    if let Err(r) =
        crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await
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
pub async fn verify_pass(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: VerifyBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };

    let db = ctx.d1("DB")?;
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
        now_min(),
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
    use super::{ev_key, user_key, SLOT_MAX};

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
