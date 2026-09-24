//! The owner console's bookings: the day's list and the venue's moves.
//!
//! Both routes derive the venue from the owner's authorisation
//! (`owner::owner_and_venue`) and load the bookings SERVER-SIDE; the console
//! sends a window and an intent, never data the hub holds.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use super::guest::Side;
use super::store::day_rows;
use super::{load_bookings, reservations::move_as};

/// The widest window one read answers. A day is what the screen shows; a
/// week is the most any caller has a reason to ask for at once.
pub const WINDOW_MAX_MIN: i64 = 7 * 24 * 60;

/// `GET /api/owner/reservations?from=<slotMin>&to=<slotMin>`
///
/// THE WINDOW IS THE CALLER'S, THE DAY IS THE VENUE'S: the console computes
/// the venue's local midnight (`lib/booking-time.js`, the same arithmetic the
/// storefront books with) and asks for `[from, to)`. Both are required -- an
/// open-ended list of every booking a venue ever took is not a screen.
pub async fn venue_day(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let url = req.url()?;
    let q = |name: &str| -> Option<i64> {
        url.query_pairs().find(|(k, _)| k == name).and_then(|(_, v)| v.parse::<i64>().ok())
    };
    let (Some(from), Some(to)) = (q("from"), q("to")) else {
        return Response::error("from and to (slot minutes) are required", 400);
    };
    if to <= from || to - from > WINDOW_MAX_MIN {
        return Response::error(format!("the window is 1..{WINDOW_MAX_MIN} minutes"), 400);
    }
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let t = load_bookings(&place).await?;
    Response::from_json(&json!({ "from": from, "to": to, "reservations": day_rows(&t, from, to) }))
}

#[derive(Deserialize)]
struct VenueMove {
    to: String,
    #[serde(default)]
    reason: String,
}

/// `POST /api/owner/reservations/:id/action` -- confirm, decline, seat,
/// complete, no-show or cancel. Signed `VENUE` with the owner's id as `by`.
/// CANCEL IS AN EVENT: the booking and its history stay; nothing is erased.
pub async fn venue_action(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing id", 400);
    };
    let (user, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(x) => x,
        Err(r) => return Ok(r),
    };
    let body: VenueMove = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    move_as(&place, &id, &body.to, Side::Venue, &user, &body.reason, ctx.data.now_ms).await
}
