//! THE TILL'S HTTP SURFACE: `POST /api/staff/till/{open,count,close,pay_in,pay_out}`.
//!
//! THE TILL IS THE VENUE'S, not an order's, so `location_id` is in the body
//! and there is no `:id` in the path. The shape is `pay.rs`'s: authorise the
//! signer for `Cap::OpenTill` at that venue (`courier::staff_at`), guard the
//! request with its idempotency key, send ONE command to the venue's object,
//! answer.
//!
//! A BLIND COUNT. The counter enters what the drawer holds without having
//! seen what it should hold — otherwise the count is a copy of the screen.
//! So only the CLOSE answers with `expected` and `over_short`: not the count,
//! and not the open or a pay-in either, since any answer that carried the
//! expected figure would let someone read it before counting by moving one
//! lek. `answer` is the one place that decides, and it is pure and tested.

use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::till::{Cmd, Report, TillOut, CLOSED};

/// The drawer a body names, or the venue's one drawer. A second register
/// costs a second name and nothing else.
pub const DEFAULT_TILL: &str = "main";

/// Which command a path segment is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Open,
    Count,
    Close,
    PayIn,
    PayOut,
}

impl Verb {
    /// The idempotency route, paired with the waiter app's queued verb.
    pub fn route(self) -> &'static str {
        match self {
            Verb::Open => "staff.till_open",
            Verb::Count => "staff.till_count",
            Verb::Close => "staff.till_close",
            Verb::PayIn => "staff.till_pay_in",
            Verb::PayOut => "staff.till_pay_out",
        }
    }
}

/// The command a body becomes. PURE. The signer and the clock are the
/// Worker's and overwrite anything the body says: a client cannot sign as
/// someone else or date a count.
pub fn command(verb: Verb, mut body: Value, by: &str, now_ms: i64) -> std::result::Result<Cmd, String> {
    let Some(obj) = body.as_object_mut() else {
        return Err("the body is a JSON object".into());
    };
    obj.insert("by".into(), json!(by));
    obj.insert("now_ms".into(), json!(now_ms));
    obj.entry("till_id").or_insert_with(|| json!(DEFAULT_TILL));
    let bad = |e: serde_json::Error| format!("bad request body: {e}");
    Ok(match verb {
        Verb::Open => Cmd::Open(serde_json::from_value(body).map_err(bad)?),
        Verb::Count => Cmd::Count(serde_json::from_value(body).map_err(bad)?),
        Verb::Close => Cmd::Close(serde_json::from_value(body).map_err(bad)?),
        Verb::PayIn => Cmd::PayIn(serde_json::from_value(body).map_err(bad)?),
        Verb::PayOut => Cmd::PayOut(serde_json::from_value(body).map_err(bad)?),
    })
}

/// WHAT A PERSON IS SHOWN. Pure: the blind count is proved here.
pub fn answer(out: &TillOut) -> Value {
    let p = &out.period.period;
    let mut v = json!({
        "kind": out.kind,
        "till_id": out.till_id,
        "open": p.closed_at.is_none(),
        "opened_at": p.opened_at,
        "opened_by": p.opened_by,
        "float": p.float,
    });
    if out.kind == CLOSED {
        v["pay_in"] = json!(p.pay_in);
        v["pay_out"] = json!(p.pay_out);
        v["cash_paid"] = json!(out.period.cash_paid);
        v["counted"] = json!(p.counted);
        v["expected"] = json!(out.period.expected);
        v["over_short"] = json!(p.over_short);
        v["closed_at"] = json!(p.closed_at);
    } else if p.counted_at.is_some() {
        // What the counter entered, and nothing it could compare it with.
        v["counted"] = json!(p.counted);
    }
    v
}

async fn send(place: &crate::hubstore::Place, cmd: &Cmd) -> std::result::Result<TillOut, (u16, String)> {
    use crate::command::send as s;
    match cmd {
        Cmd::Open(i) => s(place, "room/till_open", i).await,
        Cmd::PayIn(i) => s(place, "room/till_pay_in", i).await,
        Cmd::PayOut(i) => s(place, "room/till_pay_out", i).await,
        Cmd::Count(i) => s(place, "room/till_count", i).await,
        Cmd::Close(i) => s(place, "room/till_close", i).await,
    }
}

async fn run(mut req: Request, ctx: RouteContext<crate::Req>, verb: Verb) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: Value = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(loc) = body.get("location_id").and_then(Value::as_str).map(String::from) else {
        return Response::error("location_id is required: the till is the venue's", 400);
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &loc, Cap::OpenTill).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let cmd = match command(verb, body, &by, ctx.data.now_ms) {
        Ok(c) => c,
        Err(e) => return Response::error(e, 400),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        verb.route(),
        &raw,
        ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let out = match send(&place, &cmd).await {
        Ok(v) => v,
        Err((status, said)) => return idem.refused(&place, status, &said).await,
    };
    let shown = answer(&out);
    idem.done(&place, 200, &shown.to_string()).await;
    Response::from_json(&shown)
}

/// `POST /api/staff/till/open` — `{location_id, till_id?, float: {cur: n}}`.
pub async fn open(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    run(req, ctx, Verb::Open).await
}

/// `POST /api/staff/till/count` — `{location_id, till_id?, observed: {cur: n}}`. Blind.
pub async fn count(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    run(req, ctx, Verb::Count).await
}

/// `POST /api/staff/till/close` — `{location_id, till_id?}`. The Z report.
pub async fn close(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    run(req, ctx, Verb::Close).await
}

/// `POST /api/staff/till/pay_in` — `{location_id, till_id?, currency, amount, reason, source?}`.
pub async fn pay_in(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    run(req, ctx, Verb::PayIn).await
}

/// `POST /api/staff/till/pay_out` — `{location_id, till_id?, currency, amount, reason}`.
pub async fn pay_out(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    run(req, ctx, Verb::PayOut).await
}

/// THE PERIOD a tips read asks about. PURE. `from_ms` is the Z report's
/// `opened_at` when the phone knows the drawer; ABSENT it is the venue's day
/// (`today_ms`, its local midnight): a card-only day opens no till, and its
/// tips are still somebody's (live walk 2026-09-24). `to_ms` absent is now,
/// and a period that ends before it starts is refused rather than read as empty.
pub fn tips_period(from: Option<&str>, to: Option<&str>, now_ms: i64, today_ms: i64) -> std::result::Result<(i64, i64), String> {
    let from_ms = match from {
        None => today_ms,
        Some(v) => v.trim().parse::<i64>().map_err(|_| "from_ms is a number".to_string())?,
    };
    let to_ms = match to {
        None => now_ms,
        Some(v) => v.trim().parse::<i64>().map_err(|_| "to_ms is a number".to_string())?,
    };
    if to_ms < from_ms {
        return Err("the period ends before it starts".into());
    }
    Ok((from_ms, to_ms))
}

/// `GET /api/staff/till/tips?[location_id=]&[from_ms=]&[to_ms=]` — who took how
/// much in tips over a period, per currency (`command::tips`), the Z report's
/// companion: the till screen passes the period's `opened_at`/`closed_at`, or
/// nothing, which is the venue's day so far (`tips_period`).
/// No distribution. Read-only, so no idempotency key.
///
/// THE VENUE IS THE HOST'S (`floor::venue_for`): a venue's own subdomain
/// names it, `location_id` only on the apex; a request naming two is refused.
/// THE ORDERS ARE THE SERVER'S: loaded from the venue's own log, filtered to
/// its `location_id`; nothing about a payment is taken from the client.
pub async fn tips(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let url = req.url()?;
    let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
    let host = match crate::hubstore::Place::slug_of_host(&req, &ctx) {
        Some(slug) => {
            let v = crate::hubstore::Place::of_slug(&ctx, &slug).await?.venue;
            (v != crate::hubstore::UNNAMED_VENUE).then_some(v)
        }
        None => None,
    };
    let loc = match super::floor::venue_for(q("location_id").as_deref(), host.as_deref()) {
        Ok(l) => l,
        Err((s, m)) => return Response::error(m, s),
    };
    if let Err(r) = crate::courier::staff_at(&req, &ctx, &loc, Cap::OpenTill).await {
        return Ok(r);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // The venue's own day, in the venue's own zone (never the phone's).
    let zone = crate::hubstore::zone_of(crate::hubstore::venue_record(&place).await?.as_ref());
    let today = dowiz_hub::tz::start_of_local_day_ms(zone, ctx.data.now_ms);
    let (from_ms, to_ms) = match tips_period(q("from_ms").as_deref(), q("to_ms").as_deref(), ctx.data.now_ms, today) {
        Ok(p) => p,
        Err(e) => return Response::error(e, 400),
    };
    let currency = crate::services::venue::currency_of(&crate::hubstore::load_catalog(&place).await?.catalog);
    let orders: Vec<Value> = crate::hubstore::orders(&place)
        .await?
        .iter()
        .filter_map(|o| serde_json::from_str::<Value>(&o.order_json).ok())
        .filter(|v| v.get("location_id").and_then(Value::as_str) == Some(loc.as_str()))
        .collect();
    let t = match crate::command::tips::tips_by_person(&orders, &currency, from_ms, to_ms) {
        Ok(t) => t,
        Err(e) => return Response::error(e, 500),
    };
    let rows = json!(t);
    // The display names of the venue's own members, as the exceptions view has them.
    let names = crate::exceptions::names(&ctx.env, &loc, &rows).await?;
    Response::from_json(&json!({ "from_ms": from_ms, "to_ms": to_ms, "day": q("from_ms").is_none(), "tips": rows, "names": names }))
}

/// The `health.till` block, for `/api/owner/health` (law 10). The owner's
/// route has already authorised the venue; this only asks its object.
pub async fn report(place: &crate::hubstore::Place, location_id: &str) -> std::result::Result<Report, (u16, String)> {
    crate::command::send(place, "room/till_report", &json!({ "location_id": location_id })).await
}

#[cfg(test)]
mod tests;
