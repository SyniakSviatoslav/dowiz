//! THE FLOOR'S HTTP SURFACE: `GET /api/staff/floor` and
//! `POST /api/staff/floor/:sitting/cleared` (BLUEPRINT-OPERATIONAL-BLIND-SPOTS
//! §2.7). The fold and the mark are `command::floor`; this file only resolves
//! the ONE venue, authorises the signer, loads, and writes.
//!
//! THE SERVER LOADS EVERYTHING. The plan is the owner's (`booking::set_plan`,
//! the venue record's `floor_plan`), the holds are the booking image's table
//! index, and the sittings are the venue's own order log. Nothing about the
//! floor is taken from the client.
//!
//! ONE VENUE PER REQUEST (`tools/gates/one-venue.sh`). The venue is the one
//! the HOST names (a venue's own subdomain) or, on the apex and workers.dev,
//! the `location_id` the request names; a request naming two is refused. The
//! signer is then authorised for exactly that venue (`courier::staff_at`, as
//! `till.rs` `run` does) and the place is built from it (`of_authorised`).

use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::floor;

/// The capability both routes need: the waiter's, the one that reads the room.
pub const CAP: Cap = Cap::TakeOrders;

/// The venue record's field the owner's plan is stored in (`booking::PLAN_FIELD`).
const PLAN_FIELD: &str = "floor_plan";

/// WHICH VENUE. PURE. The host's venue when it names one, else the request's;
/// both, disagreeing, is a request about two venues and is refused.
pub fn venue_for(named: Option<&str>, host: Option<&str>) -> std::result::Result<String, (u16, String)> {
    match (named.map(str::trim).filter(|s| !s.is_empty()), host) {
        (Some(n), Some(h)) if n != h => {
            Err((400, format!("this request names two venues: {n:?} in the request and {h:?} by its host")))
        }
        (_, Some(h)) => Ok(h.to_string()),
        (Some(n), None) => Ok(n.to_string()),
        (None, None) => Err((400, "location_id is required on this host".into())),
    }
}

/// The venue the Host names, or `None` on the apex and `*.workers.dev`
/// (`services::identity::staff::host_venue`'s rule).
async fn host_venue(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<Option<String>> {
    let Some(slug) = crate::hubstore::Place::slug_of_host(req, ctx) else { return Ok(None) };
    let venue = crate::hubstore::Place::of_slug(ctx, &slug).await?.venue;
    Ok((venue != crate::hubstore::UNNAMED_VENUE).then_some(venue))
}

/// Resolve the one venue and authorise the signer for it. `Err` is the answer.
async fn door(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    named: Option<String>,
) -> Result<std::result::Result<(String, String), Response>> {
    let host = host_venue(req, ctx).await?;
    let loc = match venue_for(named.as_deref(), host.as_deref()) {
        Ok(l) => l,
        Err((s, m)) => return Ok(Err(Response::error(m, s)?)),
    };
    Ok(match crate::courier::staff_at(req, ctx, &loc, CAP).await {
        Ok((by, _)) => Ok((loc, by)),
        Err(r) => Err(r),
    })
}

/// The venue's orders, only its own.
async fn listed(place: &crate::hubstore::Place, loc: &str) -> Result<Vec<crate::hubdo::OrderView>> {
    Ok(crate::hubstore::orders(place)
        .await?
        .into_iter()
        .filter(|o| {
            serde_json::from_str::<Value>(&o.order_json)
                .is_ok_and(|v| v.get("location_id").and_then(Value::as_str) == Some(loc))
        })
        .collect())
}

/// `GET /api/staff/floor[?location_id=]` — the plan with every table's state.
pub async fn get(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let named = super::handlers::query_location(&req);
    let (loc, _by) = match door(&req, &ctx, named).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // THE PLAN, as the owner saved it. One that will not parse is EMPTY and
    // said so, never guessed: `set_plan` parses it before storing.
    let rec = crate::hubstore::venue_record(&place).await?;
    let raw = rec.as_ref().and_then(|r| r.get(PLAN_FIELD)).map(Value::to_string).unwrap_or_default();
    let plan_ok = raw.is_empty() || dowiz_hub::tables::from_json(&raw).is_ok();
    let plan = dowiz_hub::tables::from_json(&raw).unwrap_or_default();
    // THE HOLDS, read out of the booking image's table index.
    let bookings = crate::hubstore::load_table(&place, crate::booking::IMAGE_BOOKINGS, crate::booking::BOOKINGS_BYTES)
        .await?
        .table;
    let held: Vec<_> = bookings.scan("rsv.tbl/").into_iter().filter_map(|(k, id)| floor::held_of(&k, id)).collect();
    let orders = listed(&place, &loc).await?;
    let mut out = floor::floor(&plan, &held, &orders, ctx.data.now_ms);
    out["location_id"] = json!(loc);
    out["plan_unreadable"] = json!(!plan_ok);
    Response::from_json(&out)
}

/// `POST /api/staff/floor/:sitting/cleared` — `{location_id?}`. The table was
/// cleared: ONE `Noted` on the sitting's last round, through the order log's
/// own append (`hubstore::append_for`). Only a `dirty` table can be cleared.
pub async fn post_cleared(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(sitting_id) = ctx.param("sitting").cloned() else {
        return Response::error("missing sitting id", 400);
    };
    let raw = req.text().await.unwrap_or_default();
    let body: Value = if raw.trim().is_empty() {
        json!({})
    } else {
        match serde_json::from_str(&raw) {
            Ok(b) => b,
            Err(e) => return Response::error(format!("bad request body: {e}"), 400),
        }
    };
    let named = body.get("location_id").and_then(Value::as_str).map(String::from);
    let (loc, by) = match door(&req, &ctx, named).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.floor_cleared",
        &format!("{sitting_id}:{raw}"),
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    // G1 / D1: every exit below is an ANSWER, recorded (or, for a 5xx or an
    // internal error, released) by `answered` -- never a claim left standing.
    let res: Result<Response> = async {
        let orders = listed(&place, &loc).await?;
        let (round_id, needed) = match floor::clear_target(&orders, &sitting_id) {
            Ok(v) => v,
            Err(r) => return Response::error(r.message().to_string(), r.status()),
        };
        if needed {
            let (at, who, venue) = (ctx.data.now_ms, by.clone(), loc.clone());
            let wrote = crate::hubstore::append_for(&place, &round_id, at, move |current| {
                let current = current.ok_or_else(|| Error::RustError("no such order".into()))?;
                let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
                if old.get("location_id").and_then(Value::as_str) != Some(venue.as_str()) {
                    return Err(Error::RustError("no such order".into()));
                }
                let new = floor::mark(&old, &who, at).map_err(|r| Error::RustError(r.message().to_string()))?;
                Ok(Some((dowiz_hub::EventKind::Noted, crate::fold::delta(&old, &new).to_string(), json!(true))))
            })
            .await;
            match wrote {
                Ok(_) => {}
                Err(e) if e.to_string().contains("no such order") => return Response::error("not found", 404),
                Err(e) => return Err(e),
            }
        }
        let answer = json!({ "sitting_id": sitting_id, "round_id": round_id, "state": "free", "fresh": needed });
        Response::from_json(&answer)
    }
    .await;
    idem.answered(&place, res).await
}

#[cfg(test)]
mod tests;
