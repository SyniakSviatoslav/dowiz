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
        Err((status, said)) => return Response::error(said, status),
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

/// The `health.till` block, for `/api/owner/health` (law 10). The owner's
/// route has already authorised the venue; this only asks its object.
pub async fn report(place: &crate::hubstore::Place, location_id: &str) -> std::result::Result<Report, (u16, String)> {
    crate::command::send(place, "room/till_report", &json!({ "location_id": location_id })).await
}

#[cfg(test)]
mod tests;
