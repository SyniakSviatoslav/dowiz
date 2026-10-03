//! `GET  /api/owner/analytics?days=7|30|90|365` or `&from=&to=` (yyyy-mm-dd):
//!       the owner's numbers (`analytics.owner.v2`); `&trace=yyyy-mm-dd` the
//!       records behind one day and the cube's verification of it;
//!       `&verify=1` the same for the newest archived day.
//! `POST /api/owner/analytics/history`: fold every archive the cube has not
//!       folded yet (the back-fill; rotation does the same for its own).
//!
//! ORCHESTRATION ONLY on the Worker: it authorises the owner and asks the
//! venue's object for the answer. The object builds it with `answer_with`
//! (pure): the hot orders and the cube's rows through `history`, the
//! catalogue's names and currency on top. Every number in it is decided in
//! `cube` and `history`, where it has a test.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::{cube, history};
use crate::owner::owner_and_venue;

/// The query keys the Worker passes to the object, and no others: a console
/// cannot name `op`, so a GET never reaches the catch-up.
pub const PASSED: [&str; 6] = ["days", "from", "to", "trace", "verify", "v"];

pub async fn analytics(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let url = req.url()?;
    let enc = crate::mcp::enc;
    // DERIVED IN THE OBJECT (BN1, `/fold/analytics`, `hubdo/reads.rs`): the
    // folded orders, the cube and the catalogue are all there, so the answer
    // is built there and only the answer crosses.
    let mut ask = format!("https://hub/fold/analytics?venue={}&now={}", enc(&loc), ctx.data.now_ms);
    for (k, v) in url.query_pairs().filter(|(k, _)| PASSED.contains(&k.as_ref())) {
        ask.push_str(&format!("&{k}={}", enc(&v)));
    }
    if url.query_pairs().any(|(k, _)| k == "trace" || k == "verify") {
        ask.push_str("&op=trace");
    }
    reply(crate::fold::ask::text(&place, &ask).await?)
}

/// `POST /api/owner/analytics/history`. On the QA hub alone,
/// `{"rotateAsOf": ms}` first rotates as if it were that instant, so a live
/// probe can watch days leave the hot log without waiting thirty days.
pub async fn history(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let body: HistoryIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let mut rotated = Value::Null;
    if let Some(at) = body.rotate_as_of {
        if !qa_only(&loc) {
            return Response::error("rotating as of another instant is for the QA hub only", 403);
        }
        rotated = crate::hubstore::rotate(&place, at).await?;
    }
    let (status, text) = catch_up(&place, ctx.data.now_ms, body.rebuild).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out: Value = serde_json::from_str(&text).map_err(|e| Error::RustError(format!("cube: unreadable answer: {e}")))?;
    out["rotated"] = rotated;
    Response::from_json(&out)
}

#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryIn {
    #[serde(default)]
    pub rotate_as_of: Option<i64>,
    /// Start from an empty cube and fold every archive again.
    #[serde(default)]
    pub rebuild: bool,
}

/// The QA hub's venue id: the one place a rotation may be told another time.
pub const QA_VENUE: &str = "qa-durres";

pub fn qa_only(venue: &str) -> bool {
    venue == QA_VENUE
}

/// Ask the venue's object to fold the archives its cube lacks: the back-fill,
/// and the one line a rotation calls after it cuts the hot log.
pub async fn catch_up(place: &crate::hubstore::Place, now: i64, rebuild: bool) -> Result<(u16, String)> {
    let url = format!(
        "https://hub/fold/analytics?venue={}&now={now}&op=catch_up{}",
        crate::mcp::enc(&place.venue),
        if rebuild { "&rebuild=1" } else { "" }
    );
    crate::fold::ask::text(place, &url).await
}

/// THE ROTATION'S ONE LINE (`hubstore::rotate`, after the hot log is cut):
/// the days that just left it enter the cube. A failure never fails the
/// rotation -- the archive is written and the next catch-up folds it -- and
/// it is never silent either.
// Called only by the hand-back line in `hubstore::rotate`; until it lands, nothing names it.
#[allow(dead_code)]
pub async fn after_rotation(place: &crate::hubstore::Place, now: i64) {
    match catch_up(place, now, false).await {
        Ok((200, _)) => {}
        Ok((status, why)) => crate::loud!(&place.ns, Some(&place.venue), "hub.cube", "catch-up refused {status}: {why}"),
        Err(e) => crate::loud!(&place.ns, Some(&place.venue), "hub.cube", "catch-up failed: {e}"),
    }
}

fn reply((status, text): (u16, String)) -> Result<Response> {
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out = Response::ok(text)?;
    out.headers_mut().set("content-type", "application/json")?;
    out.headers_mut().set("cache-control", "private, no-store")?;
    Ok(out)
}

/// The owner's numbers over the venue's orders, its cube and its catalogue.
/// PURE: the venue's object calls this (`/fold/analytics`) with what it holds.
/// `cold(from, to)` reads the cube's rows (`yyyymmdd`), or says why it cannot.
pub fn answer_with(
    listed: Vec<crate::hubdo::OrderView>,
    cat: &dowiz_hub::catalog::Catalog,
    loc: &str,
    now: i64,
    (days, from, to): (Option<&str>, Option<&str>, Option<&str>),
    cold: super::kitchen::ColdRows,
) -> std::result::Result<Value, (u16, String)> {
    let zone = zone_of(cat);
    let s = history::span(zone, now, days, from, to).map_err(|why| (400, why))?;
    let hot_orders = crate::services::orders::mine::of_venue(listed, loc);
    let hot = cube::fold_orders(&hot_orders, zone, now);
    // The period, the one before it, and the four weeks before its last day.
    let lo = s.prev().first.min(s.last() - 7 * history::WEEKS);
    let (rows, unread) = match cold(dowiz_hub::stock::meta::day_of_number(lo), s.days().1) {
        Ok(r) => (r, None),
        Err(why) => (BTreeMap::new(), Some(why)),
    };
    let name = |id: &str| {
        cat.product(id)
            .and_then(|j| serde_json::from_str::<Value>(&j).ok())
            .and_then(|p| p.get("name").cloned())
            .unwrap_or(json!(id))
    };
    let mut out = history::report(zone, s, &rows, &hot, &name);
    out["repeat"] = history::repeat(&hot_orders, zone, s);
    out["history"] = json!({ "archivedDays": rows.len(), "error": unread });
    out["currency"] = json!(crate::services::venue::currency_of(cat));
    Ok(out)
}

/// THE v1 SHAPE (`/api/owner/analytics` before W-HIST), from a v2 answer:
/// the same numbers under the same keys, nothing added, so a console that
/// does not ask for `v=2` reads the bytes it always read (the route pin in
/// `catalogue_routes/tests.rs` holds this).
pub const V1_KEYS: [&str; 13] =
    ["averageOrder", "byChannel", "byDay", "byHour", "currency", "days", "delivery", "dineIn", "orders", "pickup", "rejected", "revenue", "topProducts"];

pub fn v1_of(v: Value) -> Value {
    let mut out = serde_json::Map::new();
    for k in V1_KEYS {
        out.insert(k.to_string(), v.get(k).cloned().unwrap_or(Value::Null));
    }
    if let Some(days) = out.get_mut("byDay").and_then(Value::as_array_mut) {
        for d in days.iter_mut() {
            *d = json!({ "at": d["at"], "orders": d["orders"], "revenue": d["revenue"] });
        }
    }
    Value::Object(out)
}

/// The venue's zone, from its own record in the catalogue.
pub fn zone_of(cat: &dowiz_hub::catalog::Catalog) -> dowiz_hub::tz::Zone {
    crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref())
}
