//! THE OWNER'S MONEY ROUTES. Authenticated as every owner route is
//! (`owner_and_venue`), acting on the venue the caller was authorised for and no other
//! (`Place::of_authorised`).
//!
//!   GET    /api/owner/finance?month=yyyy-mm      the month's entries, plus / minus / net, history
//!   POST   /api/owner/finance/expenses           {date, name, amount, kind, client_id?}; a repeated
//!                                                `client_id` answers with the entry already filed
//!   POST   /api/owner/finance/expenses/:id       the same body: the entry, replaced
//!   DELETE /api/owner/finance/expenses/:id

use super::{client_key, new_id, validate, Expense, ExpenseIn, FINANCE_BYTES, IMAGE, KIND};
use crate::hubstore::{venue_record, with_table, zone_of, Place};
use crate::owner::owner_and_venue;
use dowiz_hub::stock::meta::day_of_local_ms;
use dowiz_hub::tz::local_ms;
use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The venue the caller owns, as its object; `Err` is the refusal to send.
async fn place_of(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<std::result::Result<(Place, String), Response>> {
    match owner_and_venue(req, ctx).await {
        Ok((_, loc)) => Ok(Ok((Place::of_authorised(ctx, &loc)?, loc))),
        Err(r) => Ok(Err(r)),
    }
}

/// The venue's day now, `yyyymmdd`.
async fn today_of(place: &Place, now: i64) -> Result<i64> {
    let record = venue_record(place).await?;
    Ok(day_of_local_ms(local_ms(zone_of(record.as_ref()), now)))
}

/// `GET /api/owner/finance` -- everything the sheet draws, read through the venue's object.
pub async fn finance(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (place, loc) = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let month = req.url()?.query_pairs().find(|(k, _)| k == "month").map(|(_, v)| v.to_string()).unwrap_or_default();
    let enc = crate::mcp::enc;
    let ask = format!("https://hub/fold/finance?venue={}&now={}&month={}", enc(&loc), ctx.data.now_ms, enc(&month));
    let (status, text) = crate::fold::ask::text(&place, &ask).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| Error::RustError(format!("finance: unreadable answer: {e}")))?;
    let mut out = Response::from_json(&v)?;
    out.headers_mut().set("cache-control", "private, no-store")?;
    Ok(out)
}

/// The entry the body describes and the key its form asked for, or the refusal: the owner
/// first, then the body.
async fn body_of(mut req: Request, ctx: &RouteContext<crate::Req>) -> Result<std::result::Result<(Place, Expense, Option<String>), Response>> {
    let (place, _) = match place_of(&req, ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(Err(r)),
    };
    let body: ExpenseIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Ok(Err(Response::error(format!("bad request body: {e}"), 400)?)),
    };
    let key = match body.client_id.as_deref().map(client_key).transpose() {
        Ok(k) => k,
        Err(why) => return Ok(Err(Response::error(why, 400)?)),
    };
    let now = ctx.data.now_ms;
    let today = today_of(&place, now).await?;
    match validate(body, today) {
        Ok(e) => Ok(Ok((place, e, key))),
        Err(why) => Ok(Err(Response::error(why, 400)?)),
    }
}

/// `POST /api/owner/finance/expenses`.
pub async fn add(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let now = ctx.data.now_ms;
    let (place, mut e, key) = match body_of(req, &ctx).await? {
        Ok(x) => x,
        Err(r) => return Ok(r),
    };
    e.at = now;
    let json_of = serde_json::to_string(&e).map_err(|err| Error::RustError(format!("finance: {err}")))?;
    let (id, repeated) = with_table(&place, IMAGE, FINANCE_BYTES, move |t| {
        let id = match &key {
            // The form was already filed: the first entry stands, whatever this copy says.
            Some(k) if t.has(KIND, k) => return Ok((k.clone(), true)),
            Some(k) => k.clone(),
            None => {
                let mut n = now;
                while t.has(KIND, &new_id(n)) {
                    n += 1;
                }
                new_id(n)
            }
        };
        t.put(KIND, &id, &json_of, &[], &[]).map_err(|err| Error::RustError(format!("finance: {err:?}")))?;
        Ok((id, false))
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "id": id, "repeated": repeated }))
}

/// `POST /api/owner/finance/expenses/:id`.
pub async fn edit(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let id = ctx.param("id").cloned().unwrap_or_default();
    let (place, e, _) = match body_of(req, &ctx).await? {
        Ok(x) => x,
        Err(r) => return Ok(r),
    };
    let found = with_table(&place, IMAGE, FINANCE_BYTES, move |t| {
        let Some(old) = t.get(KIND, &id) else { return Ok(false) };
        let mut e = e.clone();
        e.at = serde_json::from_str::<Expense>(&old).map(|o| o.at).unwrap_or(e.at);
        let j = serde_json::to_string(&e).map_err(|err| Error::RustError(format!("finance: {err}")))?;
        t.put(KIND, &id, &j, &[], &[]).map_err(|err| Error::RustError(format!("finance: {err:?}")))?;
        Ok(true)
    })
    .await?;
    if found {
        Response::from_json(&json!({ "ok": true }))
    } else {
        Response::error("no such entry", 404)
    }
}

/// `DELETE /api/owner/finance/expenses/:id`.
pub async fn remove(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let id = ctx.param("id").cloned().unwrap_or_default();
    let (place, _) = match place_of(&req, &ctx).await? {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let found = with_table(&place, IMAGE, FINANCE_BYTES, move |t| Ok(t.remove(KIND, &id))).await?;
    if found {
        Response::from_json(&json!({ "ok": true }))
    } else {
        Response::error("no such entry", 404)
    }
}

#[cfg(test)]
mod tests;
