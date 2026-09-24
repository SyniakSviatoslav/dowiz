//! A booking read and moved by whoever holds it: the guest with the booking's
//! own token, or the venue. `guest::side_of` decides which, from the
//! principal -- never from the body.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_kernel::reservation::ReservationStatus;

use super::guest::{self, Side};
use super::store::write_transition;
use super::{events_of, fold_status, load_bookings, reservation_of, IMAGE_BOOKINGS, BOOKINGS_BYTES};

/// Authenticate, require this venue, and place the caller on a side FOR THIS
/// booking. A customer token for another booking is a 404.
pub(super) async fn side_at(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    place: &crate::hubstore::Place,
    id: &str,
) -> std::result::Result<(Side, String), Response> {
    let p = crate::auth::principal_at(req, &ctx.env, &place.venue, ctx.data.now_ms).await?;
    let by = match &p {
        crate::auth::Principal::Owner { user_id, .. } => user_id.clone(),
        crate::auth::Principal::Staff { person_id, .. } => person_id.clone(),
        _ => id.to_string(),
    };
    match guest::side_of(&p, id) {
        Ok(side) => Ok((side, by)),
        Err((code, why)) => Err(Response::error(why, code).unwrap()),
    }
}

/// `GET /api/public/locations/:slug/reservations/:id`
pub async fn detail(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let side = match side_at(&req, &ctx, &place, &id).await {
        Ok((s, _)) => s,
        Err(r) => return Ok(r),
    };

    let t = load_bookings(&place).await?;
    let Some(row) = reservation_of(&t, &id) else {
        return Response::error("not found", 404);
    };
    let events = events_of(&t, &id);
    let folded = match fold_status(&events) {
        Ok(s) => s,
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
        "status": folded.as_str(),
        "zoneId": row.zone_id,
        "tableN": row.table_n,
        // Loud, not silent: the cache disagreeing with the log is a fact the
        // caller gets to see.
        "statusCacheDrifted": folded.as_str() != row.status,
        "createdAtMs": row.created_at_ms,
        // What THIS caller may do next, decided here: the guest's page shows
        // a cancel button exactly when the kernel and the side both allow it.
        "next": guest::moves(side, folded).iter().map(|m| m.as_str()).collect::<Vec<_>>(),
        "history": events.iter().map(|e| json!({
            "status": e.to_status, "seq": e.seq, "actor": e.actor,
            "reason": e.reason, "atMs": e.at_ms,
        })).collect::<Vec<_>>(),
    }))
}

/// `GET /api/public/locations/:slug/reservations?user=<id>` -- the VENUE'S
/// read of one user's bookings. `?user=` is a string the caller chooses, so a
/// guest's token (which every customer who ever ordered here holds) must not
/// open it: that read anyone's bookings by naming them.
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let url = req.url()?;
    let Some(user) = url.query_pairs().find(|(k, _)| k == "user").map(|(_, v)| v.to_string()) else {
        return Response::error("missing user", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    match crate::auth::principal_at(&req, &ctx.env, &place.venue, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Owner { .. } | crate::auth::Principal::Staff { .. }) => {}
        Ok(_) => return Response::error("forbidden role", 403),
        Err(r) => return Ok(r),
    }
    let t = load_bookings(&place).await?;
    // The index IS the `ORDER BY slot_min DESC`: the key holds the complement
    // of the slot, so the sorted scan comes back newest first.
    let rows: Vec<_> = t
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

#[derive(Deserialize)]
struct ActionBody {
    /// The status to move into, in the kernel's wire form.
    to: String,
    /// NOT READ for authority. An old client may still send `actor`; the
    /// event's actor is the caller's side, whatever this says.
    #[serde(default)]
    #[allow(dead_code)]
    actor: Option<String>,
    #[serde(default)]
    reason: String,
}

/// Move a booking for a caller already placed on a side. Shared by this
/// route and the owner console's.
pub(super) async fn move_as(
    place: &crate::hubstore::Place,
    id: &str,
    to_word: &str,
    side: Side,
    by: &str,
    reason: &str,
    now: i64,
) -> Result<Response> {
    let Some(to) = ReservationStatus::from_str(to_word) else {
        return Response::error(format!("unknown status {to_word:?}"), 400);
    };
    if let Err(why) = guest::may_move(side, to) {
        return Response::error(why, 403);
    }
    let (rid, by, reason) = (id.to_string(), by.to_string(), reason.trim().chars().take(200).collect::<String>());
    let outcome = crate::hubstore::with_table(place, IMAGE_BOOKINGS, BOOKINGS_BYTES, move |t| {
        write_transition(t, &rid, to, side, &by, &reason, now).map_err(Error::RustError)
    })
    .await?;
    match outcome {
        Ok(seq) => Response::from_json(&json!({ "id": id, "status": to.as_str(), "seq": seq })),
        Err((code, why)) => Response::error(why, code),
    }
}

/// `POST /api/public/locations/:slug/reservations/:id/action`
///
/// AUTHENTICATED BEFORE THE BODY IS JUDGED: a caller with no token learns
/// nothing, not even which status names this kernel knows.
pub async fn action(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (Some(slug), Some(id)) = (ctx.param("slug").cloned(), ctx.param("id").cloned()) else {
        return Response::error("missing slug or id", 400);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let (side, by) = match side_at(&req, &ctx, &place, &id).await {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let body: ActionBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    move_as(&place, &id, &body.to, side, &by, &body.reason, ctx.data.now_ms).await
}
