//! `POST /api/public/locations/:slug/reservations` -- a table is booked.
//!
//! A GUEST NEEDS NO ACCOUNT, the way a guest who orders needs none
//! (`storefront::place`): a name, a phone, a party, a time and, if the venue
//! has a plan, a table. The answer carries a token scoped to this ONE booking
//! -- it reads and cancels it, nothing else (`guest::side_of`).
//!
//! The same route takes the VENUE'S booking (the owner or staff, with their
//! own token): signed `VENUE`, landing `CONFIRMED`, not capped per phone.
//!
//! Retries are safe twice over, as placement's are: `idempotency::guard`
//! answers a replay with the first call's whole body (token included), and the
//! reservation id is derived from the caller's `requestId`, so even a lost
//! guard record cannot make a second booking.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_kernel::reservation::{self, BookingPolicy, ReservationRequest};

use super::guest::{self, Side};
use super::store::{write_new, NewBooking};
use super::{id64, load_bookings, now_min, reservation_of, IMAGE_BOOKINGS, BOOKINGS_BYTES};

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
    /// The table the guest tapped. BOTH OR NEITHER: a zone with no number
    /// names no table, and half a choice is a booking the floor cannot check.
    #[serde(default, rename = "zoneId")]
    zone_id: Option<String>,
    #[serde(default, rename = "tableN")]
    table_n: Option<i64>,
    /// The caller's own key; the same one twice is the same booking.
    #[serde(rename = "requestId")]
    request_id: String,
}

/// A booking's guest token outlives the sitting by a day, and never lives
/// less than a customer's order token: a booking made for next month must
/// still be readable and cancellable next month.
const TOKEN_AFTER_SLOT_MS: i64 = 24 * 60 * 60 * 1000;

fn guest_token(env: &Env, id: &str, venue: &str, slot_min: i64, now: i64) -> Option<String> {
    let exp = (slot_min * 60_000 + TOKEN_AFTER_SLOT_MS).max(now + crate::auth::CUSTOMER_TTL_MS);
    crate::auth::sign(
        env,
        &crate::auth::Claims::Customer {
            // No account exists, so the subject is the booking -- exactly as
            // an order's token names the order.
            sub: id.to_string(),
            order_id: id.to_string(),
            location_id: venue.to_string(),
            sitting_id: None,
            iat: now,
            exp,
        },
    )
    .ok()
}

pub async fn create(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    // AS TEXT FIRST: the idempotency layer fingerprints exactly what was sent.
    let raw = match req.text().await {
        Ok(t) => t,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    let body: CreateBody = match serde_json::from_str(&raw) {
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
    let now = ctx.data.now_ms;

    // ── WHO IS BOOKING ──
    // No Authorization is a guest. A token that is sent must be good and must
    // be this venue's: a stale one is refused, not quietly downgraded.
    let has_bearer = req.headers().get("authorization").ok().flatten().is_some();
    let who = if has_bearer {
        match crate::auth::principal_at(&req, &ctx.env, &place.venue, now).await {
            Ok(p) => Some(p),
            Err(r) => return Ok(r),
        }
    } else {
        None
    };
    let side = match &who {
        None | Some(crate::auth::Principal::Customer { .. }) => Side::Guest,
        Some(crate::auth::Principal::Owner { .. }) => Side::Venue,
        // D40 (G6): staff book for the venue with the floor's capability.
        Some(p @ crate::auth::Principal::Staff { .. }) => match guest::side_of(p, "") {
            Ok(side) => side,
            Err((code, why)) => return Response::error(why, code),
        },
        Some(crate::auth::Principal::Courier { .. }) => return Response::error("forbidden role", 403),
    };
    // D40 (G6): the booking's user is the token's, never the body's.
    let user_id = guest::booking_user(who.as_ref(), body.user_id.as_deref());

    // ── WHAT A GUEST MUST GIVE ──
    let secret = crate::services::customers::handlers::signing_secret(&ctx.env);
    let (name, phone) = match side {
        Side::Guest => match guest::contact(&body.contact_name, &body.contact_phone) {
            Ok(c) => c,
            Err(why) => return Response::error(why, 400),
        },
        // A call taken at the console may not have a number worth keeping;
        // it always has a name to say at the door.
        Side::Venue if body.contact_name.trim().is_empty() => {
            return Response::error("a booking needs the name it is under", 400)
        }
        Side::Venue => (body.contact_name.trim().to_string(), body.contact_phone.trim().to_string()),
    };
    let phone_key = guest::phone_key(&secret, &phone);

    // ── THE PLACEMENT'S IDEMPOTENCY, keyed to the phone ──
    let idem_key = req
        .headers()
        .get("idempotency-key")
        .ok()
        .flatten()
        .or_else(|| Some(body.request_id.trim().to_string()));
    let who = phone_key.clone().unwrap_or_else(|| crate::auth::sha256_hex(&name));
    let idem =
        match crate::idempotency::guard(&place, idem_key, &who, "booking.create", &raw, now).await {
            Ok(g) => g,
            Err(r) => return Ok(r),
        };

    // A REFUSAL IS AN ANSWER TOO, and is recorded as one: an unrecorded claim
    // answers a same-key retry "still running" (`idempotency::begin`). A 500
    // is NOT recorded -- replaying a failure would make it permanent -- and
    // `Guard::answered` gives its claim back (G1), so the retry runs again;
    // the reservation id (from `requestId`) still dedupes.
    let answer: Result<(u16, String)> = async {
        let id = format!("rsv_{:016x}", id64(&format!("{}:{}", place.venue, body.request_id)));
        let t = load_bookings(&place).await?;
        if let Some(r) = reservation_of(&t, &id) {
            // The guard's record was lost; the booking was not. A token goes back
            // only to the phone the booking is under.
            let mut out = json!({ "id": r.id, "status": r.status, "replayed": true });
            if r.phone_key.is_some() && r.phone_key == phone_key {
                out["access_token"] = json!(guest_token(&ctx.env, &r.id, &place.venue, r.slot_min, now));
            }
            return Ok((200, out.to_string()));
        }

        // ── THE DECISION IS THE KERNEL'S ──
        let request = ReservationRequest {
            id: id64(&id),
            venue: place.venue.clone(),
            party: body.party,
            slot_min: body.slot_min,
            occasion: body.occasion.clone(),
        };
        if let Err(e) = reservation::validate_request(&request, &BookingPolicy::default_policy(), now_min(now)) {
            return Ok((422, e.message()));
        }
        // ── THE VENUE'S HOURS AND THE GRID, for a guest (audit D29) ──
        // The storefront offers only these times; a direct POST is held to the
        // same rule. The venue's own booking (a call at the console) is not:
        // the owner may seat a party at any minute they choose.
        if side == Side::Guest {
            let rec = crate::hubstore::venue_record(&place).await?;
            let sched = super::hours::schedule_of(rec.as_ref().and_then(|r| r.get("hours")));
            let zone = crate::hubstore::zone_of(rec.as_ref());
            if let Some(why) = super::hours::slot_verdict(&sched, zone, body.slot_min) {
                return Ok((422, why));
            }
        }

        // ── AND THE FLOOR IS THE SECOND ──
        let table = match (body.zone_id.as_deref().map(str::trim), body.table_n) {
            (Some(z), Some(n)) if !z.is_empty() => Some((z.to_string(), n)),
            (None, None) | (Some(""), None) => None,
            _ => return Ok((400, "a table is a zoneId AND a tableN, or neither".into())),
        };
        let plan = super::floor::floor_plan(&place).await?;
        if table.is_some() && plan.is_empty() {
            return Ok((409, "this venue has published no floor plan, so a booking here cannot name a table".into()));
        }

        let new = NewBooking {
            id: id.clone(),
            venue: place.venue.clone(),
            party: body.party as i64,
            slot_min: body.slot_min,
            occasion: body.occasion.clone(),
            name,
            phone,
            phone_key,
            user_id: user_id.clone(),
            table: table.clone(),
            side,
            now_ms: now,
        };
        let at = now_min(now);
        let outcome = crate::hubstore::with_table(&place, IMAGE_BOOKINGS, BOOKINGS_BYTES, move |t| {
            write_new(t, &new, &plan, at).map_err(Error::RustError)
        })
        .await;
        let (status, fresh) = match outcome {
            Ok(Ok(s)) => s,
            // The floor is not free (409) or the phone is at its cap (429): the
            // sentence names the table or the rule, never "not available".
            Ok(Err((code, why))) => return Ok((code, why)),
            Err(e) => return Ok((500, format!("create/write: {e}"))),
        };

        let mut out = json!({
            "id": id, "status": status, "replayed": !fresh,
            "zoneId": table.as_ref().map(|(z, _)| z.clone()),
            "tableN": table.as_ref().map(|(_, n)| *n),
            "slotMin": body.slot_min, "party": body.party,
        });
        if fresh && side == Side::Guest {
            // Returned once; the hub keeps no copy, the browser keeps the list.
            out["access_token"] = json!(guest_token(&ctx.env, &id, &place.venue, body.slot_min, now));
        }
        Ok((200, out.to_string()))
    }
    .await;
    let res = match answer {
        Ok((200, text)) => Response::ok(text).and_then(|mut r| {
            r.headers_mut().set("content-type", "application/json")?;
            Ok(r)
        }),
        Ok((code, text)) => Response::error(text, code),
        Err(e) => Err(e),
    };
    idem.answered(&place, res).await
}
