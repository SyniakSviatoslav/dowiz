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

async fn load_events(db: &D1Database, reservation_id: &str) -> Result<Vec<EventRow>> {
    Ok(db
        .prepare(
            "SELECT to_status, seq, actor, reason, at_ms FROM reservation_events \
             WHERE reservation_id = ?1 ORDER BY seq ASC",
        )
        .bind(&[reservation_id.into()])?
        .all()
        .await?
        .results()?)
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

    // The location is bound into the query, not checked afterwards: a booking
    // belonging to another venue must not be readable through this venue's host.
    let row: Option<ReservationRow> = db
        .prepare(
            "SELECT id, location_id, party, slot_min, occasion, contact_name, contact_phone, \
             status, created_at_ms FROM reservations WHERE id = ?1 AND location_id = ?2",
        )
        .bind(&[id.clone().into(), place.venue.clone().into()])?
        .first(None)
        .await?;

    let Some(row) = row else {
        return Response::error("not found", 404);
    };
    let events = load_events(&db, &id).await?;

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
    let rows: Vec<ReservationRow> = db
        .prepare(
            "SELECT id, location_id, party, slot_min, occasion, contact_name, contact_phone, \
             status, created_at_ms FROM reservations \
             WHERE location_id = ?1 AND user_id = ?2 ORDER BY slot_min DESC LIMIT 100",
        )
        .bind(&[place.venue.clone().into(), user.into()])?
        .all()
        .await?
        .results()?;

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
    let existing: Option<ReservationRow> = match db
        .prepare("SELECT id, location_id, party, slot_min, occasion, contact_name, \
                  contact_phone, status, created_at_ms FROM reservations WHERE id = ?1")
        .bind(&[id.clone().into()])
    {
        Ok(stmt) => match stmt.first(None).await {
            Ok(r) => r,
            Err(e) => return Response::error(format!("create/lookup: {e}"), 500),
        },
        Err(e) => return Response::error(format!("create/bind-lookup: {e}"), 500),
    };
    if let Some(r) = existing {
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
    let event_id = format!("rev_{:016x}", id64(&format!("{id}:1")));
    let stmts = vec![
        db.prepare(
            "INSERT INTO reservations (id, location_id, user_id, party, slot_min, occasion, \
             contact_name, contact_phone, status, created_at_ms, updated_at_ms) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'REQUESTED',?9,?10)",
        )
        .bind(&[
            id.clone().into(),
            place.venue.clone().into(),
            // NULL, not "". The column references `users(id)`, and an empty
            // string is a value that names no user — SQLite rejects it against
            // the foreign key, which is what turned a guest booking into a 500.
            match body.user_id.as_deref() {
                Some(u) if !u.trim().is_empty() => u.into(),
                _ => worker::wasm_bindgen::JsValue::NULL.into(),
            },
            num(body.party as i64),
            num(body.slot_min),
            body.occasion.into(),
            body.contact_name.into(),
            body.contact_phone.into(),
            num(now),
            num(now),
        ])?,
        db.prepare(
            "INSERT INTO reservation_events (id, reservation_id, location_id, to_status, seq, \
             actor, reason, at_ms) VALUES (?1,?2,?3,'REQUESTED',1,'CUSTOMER','',?4)",
        )
        .bind(&[
            event_id.into(),
            id.clone().into(),
            place.venue.clone().into(),
            num(now),
        ])?,
    ];
    if let Err(e) = db.batch(stmts).await {
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

    let owns: Option<ReservationRow> = db
        .prepare("SELECT id, location_id, party, slot_min, occasion, contact_name, \
                  contact_phone, status, created_at_ms FROM reservations \
                  WHERE id = ?1 AND location_id = ?2")
        .bind(&[id.clone().into(), place.venue.clone().into()])?
        .first(None)
        .await?;
    if owns.is_none() {
        return Response::error("not found", 404);
    }

    let events = load_events(&db, &id).await?;
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
    let event_id = format!("rev_{:016x}", id64(&format!("{id}:{seq}")));
    db.batch(vec![
        db.prepare(
            "INSERT INTO reservation_events (id, reservation_id, location_id, to_status, seq, \
             actor, reason, at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )
        .bind(&[
            event_id.into(),
            id.clone().into(),
            place.venue.clone().into(),
            to.as_str().into(),
            num(seq),
            body.actor.into(),
            body.reason.into(),
            num(now),
        ])?,
        db.prepare("UPDATE reservations SET status = ?1, updated_at_ms = ?2 WHERE id = ?3")
            .bind(&[to.as_str().into(), num(now), id.clone().into()])?,
    ])
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
async fn pass_key(db: &D1Database, location_id: &str) -> Result<Vec<u8>> {
    let row: Option<KeyRow> = db
        .prepare("SELECT key_b64 FROM venue_pass_keys WHERE location_id = ?1")
        .bind(&[location_id.into()])?
        .first(None)
        .await?;

    if let Some(r) = row {
        return base64_decode(&r.key_b64)
            .ok_or_else(|| Error::RustError("venue pass key is not base64".into()));
    }

    // First use: mint 32 bytes from the runtime's CSPRNG. Not from a hash of the
    // venue id — that would make every deployment's keys derivable from public
    // data.
    let mut key = [0u8; 32];
    getrandom_fill(&mut key)?;
    let b64 = base64_encode(&key);
    db.prepare(
        "INSERT INTO venue_pass_keys (location_id, key_b64, created_at_ms) VALUES (?1,?2,?3) \
         ON CONFLICT(location_id) DO NOTHING",
    )
    .bind(&[location_id.into(), b64.into(), num(now_ms())])?
    .run()
    .await?;
    Ok(key.to_vec())
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

    let row: Option<ReservationRow> = db
        .prepare("SELECT id, location_id, party, slot_min, occasion, contact_name, \
                  contact_phone, status, created_at_ms FROM reservations \
                  WHERE id = ?1 AND location_id = ?2")
        .bind(&[id.clone().into(), place.venue.clone().into()])?
        .first(None)
        .await?;
    let Some(row) = row else {
        return Response::error("not found", 404);
    };

    let events = load_events(&db, &id).await?;
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

    let key = pass_key(&db, &place.venue).await?;
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
    let key = pass_key(&db, &place.venue).await?;

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
