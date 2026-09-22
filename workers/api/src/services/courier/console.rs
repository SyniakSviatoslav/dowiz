//! The OWNER'S side of the crew: the roster, the invites, one courier's
//! record, and turning somebody off.
//!
//! `crate::courier` is what the COURIER'S OWN APP calls -- shifts, positions,
//! accepting a delivery. This is the same vocabulary seen from the console,
//! and it is the part that was living in `extra.rs`.
//!
//! ORCHESTRATION ONLY. Every rule it applies is in `roster` or `record`
//! beside it, where it has a test.

use serde_json::{json, Value};
use worker::*;

use crate::owner::{owner_and_venue};


/// `GET /api/owner/couriers` — the roster, and the invites still outstanding.
///
/// PENDING INVITES SIT IN THE SAME LIST as the people. An owner asking who
/// delivers for them counts the person they invited yesterday among the answer,
/// and a separate panel for invites is a panel nobody opens.
pub async fn couriers(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use crate::services::courier::roster;

    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of(&req, &ctx, Some(&loc))?;
    // THIS VENUE'S ROSTER, from the prefix that is the roster. The join it
    // replaces was `couriers JOIN courier_locations WHERE cl.location_id = ?`
    // with a correlated subquery counting open shifts -- three tables to answer
    // "who works here and who is out right now".
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    let shifts = crate::hubstore::load_table(
        &place,
        crate::hubstore::IMAGE_OPS,
        crate::hubstore::OPS_BYTES,
    )
    .await?
    .table;
    // Sorted by when they joined, which is the order an owner remembers them
    // in; the row itself is `roster::roster_row`, tested beside its defects.
    let mut crew_rows: Vec<(i64, Value)> = crew
        .scan(&format!("roster.venue/{loc}/"))
        .into_iter()
        .filter_map(|(_, id)| {
            let r = crate::identity_store::rec(&crew, crate::identity_store::K_COURIER, &id)?;
            let on = roster::shift_is_open(shifts.get("shift", &id).as_deref());
            Some((crate::identity_store::i_of(&r, "created_at_ms"), roster::roster_row(&id, &r, on)))
        })
        .collect();
    crew_rows.sort_by_key(|(at, _)| *at);

    let now = ctx.data.now_ms;
    let mut invite_rows: Vec<(i64, Value)> = crew
        .scan(&format!("invite.loc/{loc}/"))
        .into_iter()
        .filter_map(|(_, id)| {
            let r = crate::identity_store::rec(&crew, crate::identity_store::K_INVITE, &id)?;
            let row = roster::invite_row(&id, &r, now)?;
            Some((crate::identity_store::i_of(&r, "created_at_ms"), row))
        })
        .collect();
    invite_rows.sort_by_key(|(at, _)| *at);

    Response::from_json(&json!({
        "couriers": crew_rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
        "invites": invite_rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
    }))
}


/// `GET /api/owner/couriers/:id` -- one courier: who, whether on shift, where
/// they were last seen, what they did today.
pub async fn courier_detail(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing courier id", 400);
    };
    use crate::services::courier::{record, roster};

    let (_, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // BY ID OR BY THE PHONE THE CONSOLE DISPLAYS, and on THIS venue's roster --
    // the roster key is what the `JOIN courier_locations WHERE location_id`
    // was, and a courier of another restaurant simply has no key here.
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    let found = if crew
        .get(crate::identity_store::K_ROSTER, &crate::identity_store::roster_id(&loc, &id))
        .is_some()
    {
        Some(id.clone())
    } else {
        crate::identity_store::courier_id_for_phone(&crew, &crate::auth::sha256_hex(&id))
            .filter(|cid| {
                crew.get(
                    crate::identity_store::K_ROSTER,
                    &crate::identity_store::roster_id(&loc, cid),
                )
                .is_some()
            })
    };
    let shift_img = crate::hubstore::load_table(
        &place,
        crate::hubstore::IMAGE_OPS,
        crate::hubstore::OPS_BYTES,
    )
    .await?
    .table;
    let Some((cid, row)) = found
        .and_then(|cid| {
            crate::identity_store::rec(&crew, crate::identity_store::K_COURIER, &cid)
                .map(|r| (cid, r))
        })
        .map(|(cid, r)| {
            let on = roster::shift_is_open(shift_img.get("shift", &cid).as_deref());
            let row = roster::roster_row(&cid, &r, on);
            (cid, row)
        })
    else {
        return Response::error("not found", 404);
    };
    let now = ctx.data.now_ms;
    // THE OBJECT FIRST HERE TOO: this screen is an owner looking at one
    // courier, so the fresher answer is the one worth a request.
    let fix = crate::live_eta::fixes_at(&place, &loc, now, true)
        .await
        .into_iter()
        .find(|f| f.courier_id == cid);
    // THE VENUE'S MIDNIGHT, not UTC's. This was `now - now.rem_euclid(DAY)`,
    // which is not even the old +2 constant -- it is a UTC day, so an owner
    // looking at a courier at 01:00 local saw a "today" that had already
    // started, and the cash the courier is carrying counted against the wrong
    // one. The record is ~1 KB and the zone is the only field read from it.
    let zone = crate::hubstore::zone_of(crate::hubstore::venue_record(&place).await?.as_ref());
    let day_start = dowiz_hub::tz::start_of_local_day_ms(zone, now);
    // The tiles: today, a month of runs, and what is on the road right now.
    // The arithmetic is `record::tally`, where it can be tested at the day
    // boundary that decides which day a courier's cash belongs to.
    let ops = crate::hubstore::load_table(
        &place,
        crate::hubstore::IMAGE_OPS,
        crate::hubstore::OPS_BYTES,
    )
    .await?
    .table;
    let rows: Vec<Value> = ops
        .all("asg")
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str::<Value>(&j).ok())
        .collect();
    let t = record::tally(&rows, &cid, day_start, now);
    let mut out = row;
    out["lastFix"] = fix
        .map(|f| json!({ "latUdeg": f.lat_udeg, "lonUdeg": f.lon_udeg, "recordedAtMs": f.recorded_at_ms }))
        .unwrap_or(Value::Null);
    out["today"] = json!({ "deliveries": t.today_deliveries, "cashCollected": t.today_cash });
    out["delivered30d"] = json!(t.delivered_30d);
    out["inFlight"] = json!(t.in_flight);
    Response::from_json(&out)
}
