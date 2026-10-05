//! `GET /api/staff/kitchen/prep?day=yyyy-mm-dd` -- THE PREP LIST (W-PREP,
//! P6 of `docs/research/2026-10-03-depth-stock-analytics-models.md`, K14 of
//! the kitchen report): how many orders and portions the day will see, per
//! band of the venue's hours and per dish, expanded through the recipes into
//! semi-finished products (ПФ) and raw items; what to make (forecast ПФ use −
//! ПФ on hand + tonight's bookings × the usual basket); and how far off the
//! forecast usually is, measured on the venue's own last four weeks.
//!
//! WHO: the owner, or a member of staff holding the shelf's or the menu's
//! word (`guard::NUMBERS`) -- the kitchen. The answer carries NO MONEY and no
//! person, so both read the same bytes.
//!
//! ORCHESTRATION ONLY on the Worker: the venue's object folds (`/fold/prep`,
//! `hubdo/forecast.rs`) with `answer_with`, PURE; every number is decided in
//! `dowiz_hub::forecast` and `plan`, where it has a test. The history is the
//! daily sales cube (W-HIST, `cube.rs`) plus the hot log's days -- the same
//! two sources `/api/owner/analytics` reads, never a re-fold of archives.
//!
//! STRICT QUERY: a key other than `day` and `location_id` is a 400, so a
//! typo is said rather than silently forecasting today.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::cube::DayCube;
use dowiz_hub::forecast::{self as fc, History};
use dowiz_hub::stock::meta::{day_number, day_of_local_ms, day_of_number, parse_day, show_day};
use dowiz_hub::tz::Zone;

/// The forecast composed, and the answer's shape.
pub mod plan;
/// Through the recipes into ПФ and raw; the prep list; P7's daily use.
pub mod expand;

pub const CONTRACT: &str = "kitchen.prep_forecast.v1";
/// The furthest day ahead a prep list is asked for: within a week, every
/// sample of the day is already in the past.
pub const DAYS_AHEAD: i64 = 6;
/// The query keys this route reads; any other is refused.
pub const QUERY: [&str; 2] = ["day", "location_id"];
/// The days of history one answer folds: eight weeks of samples, four weeks
/// of error measured as of each day, and the naive guess's week before.
pub const HISTORY_DAYS: i64 = fc::WEEKS_MAX as i64 * 7 + fc::BACKTEST_DAYS + 7;

/// The first query key this route does not read, if any.
pub fn unknown_key<'a>(keys: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    keys.into_iter().find(|k| !QUERY.contains(k))
}

pub async fn prep(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use crate::services::identity::staff::guard;
    let url = req.url()?;
    let keys: Vec<String> = url.query_pairs().map(|(k, _)| k.to_string()).collect();
    if let Some(k) = unknown_key(keys.iter().map(String::as_str)) {
        return Response::error(format!("{k:?} is not a query of this route (day, location_id)"), 400);
    }
    let loc = match guard::staff_venue_as(&req, &ctx, &guard::NUMBERS).await {
        Ok((_, l, _)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let enc = crate::mcp::enc;
    let mut ask = format!("https://hub/fold/prep?venue={}&now={}", enc(&loc), ctx.data.now_ms);
    if let Some((_, d)) = url.query_pairs().find(|(k, _)| k == "day") {
        ask.push_str(&format!("&day={}", enc(&d)));
    }
    let (status, text) = crate::fold::ask::text(&place, &ask).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out = Response::ok(text)?;
    out.headers_mut().set("content-type", "application/json")?;
    out.headers_mut().set("cache-control", "private, no-store")?;
    Ok(out)
}

/// `(today, the day asked for)` as day numbers in the venue's clock. Refused:
/// a date that is not one, a day before today, more than `DAYS_AHEAD` ahead.
pub fn days(zone: Zone, now: i64, day: Option<&str>) -> std::result::Result<(i64, i64), String> {
    let today = day_number(day_of_local_ms(dowiz_hub::tz::local_ms(zone, now)));
    let target = match day.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => parse_day(s).map(day_number).ok_or(format!("{s:?} is not a date (yyyy-mm-dd)"))?,
        None => today,
    };
    if target < today || target > today + DAYS_AHEAD {
        return Err(format!("a prep list is for today up to {DAYS_AHEAD} days ahead"));
    }
    Ok((today, target))
}

/// The forecast's days from the cube's rows and the hot log's (`yyyymmdd`
/// keys, disjoint by construction, added as `history::report` adds them),
/// keeping only days before `as_of` -- today's partial is not a sample.
pub fn history_of(cold: &BTreeMap<i64, DayCube>, hot: &BTreeMap<i64, DayCube>, as_of: i64) -> History {
    let mut rows = cold.clone();
    for (d, r) in hot {
        rows.entry(*d).or_insert_with(|| DayCube::new(*d)).add(r);
    }
    let mut h = History::default();
    for (d, r) in rows {
        let num = day_number(d);
        if num >= as_of {
            continue;
        }
        let dishes = r.m.iter().filter(|(_, v)| v[0] > 0).map(|(k, v)| (k.clone(), v[0])).collect();
        h.days.insert(num, fc::Day { orders: r.o - r.x, bands: fc::bands_of(&r.h), dishes });
    }
    h
}

/// Tonight's booked covers: `(from, to)` in slot minutes -> guests.
pub type Covers<'a> = &'a dyn Fn(i64, i64) -> i64;

/// The venue's record from its catalogue.
pub fn venue_of(cat: &dowiz_hub::catalog::Catalog) -> Option<Value> {
    cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok())
}

/// The history the venue's object holds, as of the venue's `today`: the
/// cube's rows over `HISTORY_DAYS` and the hot log. The second answer is the
/// reason the cube could not be read (said, never a silent zero).
pub fn history_with(listed: Vec<crate::hubdo::OrderView>, loc: &str, zone: Zone, now: i64, today: i64, cold: super::kitchen::ColdRows) -> (History, Option<String>, usize) {
    let (rows, unread) = match cold(day_of_number(today - HISTORY_DAYS), day_of_number(today - 1)) {
        Ok(r) => (r, None),
        Err(why) => (BTreeMap::new(), Some(why)),
    };
    let hot = super::cube::fold_orders(&crate::services::orders::mine::of_venue(listed, loc), zone, now);
    (history_of(&rows, &hot, today), unread, rows.len())
}

/// The prep list. PURE: the venue's object calls this (`/fold/prep`) with
/// the orders, catalogue and stock log it holds, its bookings' covers and
/// the cube's rows. A day that is not one is the 400's text.
#[allow(clippy::too_many_arguments)]
pub fn answer_with(
    listed: Vec<crate::hubdo::OrderView>,
    cat: &dowiz_hub::catalog::Catalog,
    stock: &dowiz_hub::stock::StockLog,
    loc: &str,
    now: i64,
    day: Option<&str>,
    covers: Covers,
    cold: super::kitchen::ColdRows,
) -> std::result::Result<Value, (u16, String)> {
    let venue = venue_of(cat);
    let zone = crate::hubstore::zone_of(venue.as_ref());
    let (today, target) = days(zone, now, day).map_err(|why| (400, why))?;
    let (h, unread, archived) = history_with(listed, loc, zone, now, today, cold);
    let led = stock.ledger().map_err(|e| (500, e.to_string()))?;
    let midnight = |n: i64| super::history::midnight(zone, n) / 60_000;
    let booked = covers(midnight(target), midnight(target + 1));
    let windows = plan::windows(venue.as_ref(), super::history::weekday(target));
    let mut out = plan::answer(&h, today, target, cat, &led, booked, windows);
    out["history"] = json!({ "archivedDays": archived, "error": unread });
    out["today"] = json!(show_day(day_of_number(today)));
    Ok(out)
}

#[cfg(test)]
#[path = "forecast/tests.rs"]
mod tests;
