//! The owner's payroll tab: hourly rates, hours entered by date, the month and per-employee
//! history. The month's pay total the money sheet subtracts is `rules::month_total`, the rule
//! the table's own total is checked against.
//!
//! THE VENUE IS THE ONE THE QUERY NAMES (`?location_id=`, `owner_and_venue`), then the
//! token's; a body carries no venue.
//!
//! OWNER-ONLY AND VENUE-SCOPED. Every handler asks `owner_and_venue` first and acts on the
//! venue it was authorised for (`Place::of_authorised`); the data lives in that venue's own
//! object, so another venue's rows are not filtered out -- they are not there. Employees are
//! the existing staff (`staff_admin`): an id that is not a member of this venue is a 404.
//! The rules are `rules.rs`; this file only sequences calls.

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::hubstore::Place;
use crate::identity_store as ids;
use crate::owner::owner_and_venue;
use crate::services::identity::staff_rules as sr;
use rules::{Entry, Person, BYTES, IMAGE, K_ENTRY, K_RATE};

pub mod rules;

fn query(req: &Request, key: &str) -> Result<Option<String>> {
    Ok(req.url()?.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.to_string()))
}

/// The venue's civil date right now, in its own zone.
async fn today_of(place: &Place, now_ms: i64) -> Result<String> {
    let record = crate::hubstore::venue_record(place).await?;
    let zone = crate::hubstore::zone_of(record.as_ref());
    Ok(rules::date_of(dowiz_hub::tz::local_ms(zone, now_ms)))
}

/// The venue's staff, from the staff entity (not a second person list).
async fn people(env: &Env, venue: &str) -> Result<Vec<Person>> {
    let t = ids::identity(env).await?;
    Ok(t.scan(&format!("member.venue/{venue}/"))
        .into_iter()
        .filter_map(|(_, uid)| {
            let m = ids::rec(&t, ids::K_MEMBER, &ids::member_id(venue, &uid))?;
            if !sr::is_staff_member(&m) {
                return None;
            }
            let name = ids::rec(&t, ids::K_USER, &uid).map(|u| ids::s_of(&u, "display_name")).unwrap_or_default();
            Some(Person { active: ids::s_of(&m, "status") == "active", role: ids::s_of(&m, "role"), id: uid, name })
        })
        .collect())
}

/// Whether `uid` is staff of this venue.
async fn is_staff(env: &Env, venue: &str, uid: &str) -> Result<bool> {
    let t = ids::identity(env).await?;
    Ok(ids::rec(&t, ids::K_MEMBER, &ids::member_id(venue, uid)).is_some_and(|m| sr::is_staff_member(&m)))
}

fn loud<T>(r: std::result::Result<T, String>) -> Result<T> {
    r.map_err(Error::RustError)
}

fn too_big() -> Error {
    Error::RustError("payroll: a total too large to be money".into())
}

/// `GET /api/owner/payroll?month=YYYY-MM` -- the month as a table (the venue's current month by default).
pub async fn month_route(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let today = today_of(&place, ctx.data.now_ms).await?;
    let month = query(&req, "month")?.unwrap_or_else(|| rules::month_of(&today).to_string());
    if let Err(why) = rules::check_month(&month) {
        return Response::error(why, 400);
    }
    let people = people(&ctx.env, &loc).await?;
    let t = crate::hubstore::load_table(&place, IMAGE, BYTES).await?.table;
    let entries = loud(rules::parse_all(t.all(K_ENTRY)))?;
    let rates: BTreeMap<String, i64> = t.all(K_RATE).into_iter().filter_map(|(id, j)| Some((id, rules::rate_of(&j)?))).collect();
    let Some(mut view): Option<Value> = rules::month_view(&month, &people, &rates, &entries) else {
        return Err(too_big());
    };
    view["today"] = json!(today);
    private(Response::from_json(&view)?)
}

/// Pay is nobody else's to read: no shared or browser cache keeps an answer.
fn private(mut r: Response) -> Result<Response> {
    r.headers_mut().set("cache-control", "private, no-store")?;
    Ok(r)
}

/// `GET /api/owner/payroll/history?staff_id=` -- one employee's pay by month, past months included.
pub async fn history_route(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(staff_id) = query(&req, "staff_id")? else {
        return Response::error("missing staff_id", 400);
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let t = crate::hubstore::load_table(&place, IMAGE, BYTES).await?.table;
    let entries = loud(rules::parse_all(t.all(K_ENTRY)))?;
    let Some(v) = rules::history(&entries, &staff_id) else {
        return Err(too_big());
    };
    private(Response::from_json(&v)?)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryIn {
    staff_id: String,
    date: String,
    minutes: i64,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/owner/payroll/entry` -- set one person's hours on one date (a new entry or an edit).
pub async fn set_entry(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY: nobody's JSON is parsed before the door.
    let body: EntryIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if let Err(why) = rules::check_date(&body.date).and_then(|_| rules::check_minutes(body.minutes)) {
        return Response::error(why, 400);
    }
    let note = match rules::clean_note(body.note.as_deref()) {
        Ok(n) => n,
        Err(why) => return Response::error(why, 400),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    if body.date > today_of(&place, ctx.data.now_ms).await? {
        return Response::error("that day has not happened yet", 400);
    }
    if !is_staff(&ctx.env, &loc, &body.staff_id).await? {
        return Response::error("not found", 404);
    }
    let (staff, date, minutes) = (body.staff_id.clone(), body.date.clone(), body.minutes);
    let saved = crate::hubstore::with_table(&place, IMAGE, BYTES, move |t| {
        let id = rules::entry_id(&date, &staff);
        // An edit keeps the entry's own rate; a new entry takes the current one.
        let kept = t.get(K_ENTRY, &id).and_then(|j| serde_json::from_str::<Entry>(&j).ok()).map(|e| e.rate_minor);
        let Some(rate_minor) = kept.or_else(|| t.get(K_RATE, &staff).as_deref().and_then(rules::rate_of)) else {
            return Ok(None);
        };
        let e = Entry { staff_id: staff.clone(), date: date.clone(), minutes, rate_minor, note: note.clone() };
        let json = serde_json::to_string(&e).map_err(|e| Error::RustError(e.to_string()))?;
        t.put(K_ENTRY, &id, &json, &[], &[]).map_err(|e| Error::RustError(format!("payroll entry: {e}")))?;
        Ok(Some(e))
    })
    .await?;
    let Some(e) = saved else {
        return Response::error("set the employee's hourly rate first", 409);
    };
    Response::from_json(&json!({ "ok": true, "entry": { "staffId": e.staff_id, "date": e.date, "minutes": e.minutes, "note": e.note, "rateMinor": e.rate_minor } }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryKeyIn {
    staff_id: String,
    date: String,
}

/// `POST /api/owner/payroll/entry/delete` -- remove a mistaken day.
pub async fn delete_entry(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let body: EntryKeyIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if let Err(why) = rules::check_date(&body.date) {
        return Response::error(why, 400);
    }
    let place = Place::of_authorised(&ctx, &loc)?;
    let id = rules::entry_id(&body.date, &body.staff_id);
    let gone = crate::hubstore::with_table(&place, IMAGE, BYTES, move |t| Ok(t.remove(K_ENTRY, &id))).await?;
    if !gone {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RateIn {
    staff_id: String,
    rate_minor: i64,
}

/// `POST /api/owner/payroll/rate` -- an employee's hourly rate, in minor units.
pub async fn set_rate(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let body: RateIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if let Err(why) = rules::check_rate(body.rate_minor) {
        return Response::error(why, 400);
    }
    if !is_staff(&ctx.env, &loc, &body.staff_id).await? {
        return Response::error("not found", 404);
    }
    let place = Place::of_authorised(&ctx, &loc)?;
    let now = ctx.data.now_ms;
    let (staff, rate) = (body.staff_id, body.rate_minor);
    crate::hubstore::with_table(&place, IMAGE, BYTES, move |t| {
        t.put(K_RATE, &staff, &rules::rate_record(rate, now), &[], &[]).map_err(|e| Error::RustError(format!("payroll rate: {e}")))
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "rateMinor": rate }))
}

#[cfg(test)]
mod tests;
