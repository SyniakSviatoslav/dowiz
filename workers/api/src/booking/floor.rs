//! The floor: the owner's plan, which tables are held for a slot, and the
//! public "is there room tonight". `dowiz_hub::tables` decides which tables
//! can seat a party and which are taken; nothing in this file works that out.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_hub::tables as floor;
use dowiz_kernel::reservation::{self, BookingPolicy, ReservationRequest};

use super::{load_bookings, now_min};

/// Where the owner's floor plan lives: a field on the venue's own record, in
/// the catalogue image, beside `delivery_zones` -- a fact about the venue that
/// an owner changes without a deploy (`services/orders/room/floor.rs` reads it).
pub(crate) const PLAN_FIELD: &str = "floor_plan";

/// The index key that holds a table for a slot.
///
/// ZERO-PADDED for the reason `ev_key` is: keys sort as strings, and the scan
/// that reads these back parses the slot out of the key rather than opening
/// every reservation. `{n:06}` keeps table 10 after table 2.
///
/// NO RESERVATION ID IN IT, deliberately, so the key can be declared UNIQUE to
/// `Table::put` -- which refuses a second record claiming it inside the
/// object's own turn. That is the backstop under the check in `store`.
pub(crate) fn table_key(zone: &str, n: i64, slot_min: i64) -> String {
    format!("rsv.tbl/{zone}/{n:06}/{slot_min:012}")
}

/// Every table currently held, READ BACK OUT OF THE KEYS: one sorted prefix
/// scan, no reservation opened. A key that does not parse is skipped rather
/// than panicking -- an unreadable index entry must not take the page down.
pub(super) fn held_tables(t: &dowiz_hub::table::Table) -> Vec<floor::Held> {
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

/// The stored plan's zones, as the owner saved them, or none.
async fn stored_zones(place: &crate::hubstore::Place) -> Result<serde_json::Value> {
    let rec = crate::hubstore::venue_record(place).await?;
    Ok(rec
        .as_ref()
        .and_then(|r| r.get(PLAN_FIELD))
        .and_then(|p| p.get("zones"))
        .cloned()
        .unwrap_or_else(|| json!([])))
}

/// The venue's floor plan, or an empty one. A plan that will not parse is
/// EMPTY rather than guessed at -- and it cannot get in: [`set_plan`] parses
/// it before it is stored.
pub(super) async fn floor_plan(place: &crate::hubstore::Place) -> Result<floor::Plan> {
    let zones = stored_zones(place).await?;
    Ok(floor::from_json(&json!({ "zones": zones }).to_string()).unwrap_or_default())
}

/// May this party have this table at this minute? `None` is yes.
///
/// PURE, so the refusal is tested without a Worker runtime (`floor/tests.rs`,
/// and the double-booking RED proof). EVERY REFUSAL NAMES THE TABLE: "that
/// time is not available" sends a guest away without telling them the next
/// table over is free.
pub(super) fn table_verdict(
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
        // A RETRIED REQUEST IS NOT A DOUBLE BOOKING: the same `requestId`
        // produces the same reservation id, and its own hold is not a rival.
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

/// The owner's plan, PARSED BACK before it may be stored, with the reader's
/// own sentence as the refusal: two tables with one number in a zone, a table
/// with no seats, a table off the drawing, a zone id that cannot be a key.
/// Answers `(zones, tables)` for the reply.
pub(super) fn checked_plan(zones: &[serde_json::Value]) -> std::result::Result<(usize, usize), String> {
    let doc = json!({ "zones": zones }).to_string();
    let parsed = floor::from_json(&doc).map_err(|e| format!("floor plan refused: {}", e.message()))?;
    Ok((parsed.zones.len(), parsed.zones.iter().map(|z| z.tables.len()).sum()))
}

#[derive(Deserialize)]
struct PlanIn {
    /// The zones as the owner drew them. An EMPTY list removes the plan, which
    /// is how a venue that does not seat by table turns the feature off.
    zones: Vec<serde_json::Value>,
}

/// `POST /api/owner/floorplan` -- the console's floor editor saves here.
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
    let (zones, tables) = match checked_plan(&body.zones) {
        Ok(n) => n,
        Err(why) => return Response::error(why, 400),
    };
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

/// `GET /api/owner/floorplan` -- the plan as stored, for the editor to open.
/// The drawing's bounds and the seat ceiling come with it, so the editor's
/// grid is the reader's and not a second copy of the numbers.
pub async fn get_plan(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let zones = stored_zones(&place).await?;
    Response::from_json(&json!({
        "zones": zones,
        "planW": floor::PLAN_W, "planH": floor::PLAN_H,
        "maxSeats": floor::MAX_SEATS, "maxTables": floor::MAX_TABLES,
    }))
}

/// `GET /api/public/locations/:slug/tables?slotMin=<n>&party=<n>`
///
/// THE SLOT IS REQUIRED: a table is free or taken only for a minute, so an
/// answer without one would be a lie the surface would then draw. PUBLIC: it
/// is the venue's own furniture; it carries no name, no party and no
/// reservation id. The guard stays on everything that WRITES.
pub async fn availability(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let url = req.url()?;
    let q = |name: &str| -> Option<i64> {
        url.query_pairs().find(|(k, _)| k == name).and_then(|(_, v)| v.parse::<i64>().ok())
    };
    let Some(slot_min) = q("slotMin") else {
        return Response::error("slotMin is required: a table is free or taken only for a slot", 400);
    };
    let party = q("party").unwrap_or(1);
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;

    // THE SAME RULE THE WRITE USES: a plan for a slot the kernel would refuse
    // is a plan the guest cannot book from.
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

#[cfg(test)]
mod tests;
