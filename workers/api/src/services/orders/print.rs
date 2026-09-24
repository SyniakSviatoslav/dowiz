//! `/api/print/*` — the kitchen printer's three routes (BLUEPRINT-LAST-MILE
//! §3.1 step 1 (b)). The rules are `print_rail.rs`; the writes happen in the
//! venue's object (`hubdo/print.rs`). This file only resolves WHICH venue,
//! from the printer's key, and translates the wire.
//!
//! THE VENUE COMES FROM THE KEY AND FROM NOTHING ELSE. No route takes a
//! `location_id`: a venue API key is scoped to the venue it was minted for
//! (`auth::api_key_principal`), so a printer holding venue A's key can only
//! ever reach A's object. There is no parameter to point it elsewhere.
//!
//! IDEMPOTENT BY CONSTRUCTION, not by a guard table: a repeated poll inside
//! the lease hands out nothing new, a GET is a read, and a repeated DELETE is
//! answered from the order's own record (`print_rail::decide_ack`).

use worker::*;

/// The venue a printer's `Authorization` belongs to, or the refusal.
async fn venue_of(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<String, Response> {
    let refuse = || Response::error("a printer signs in with a venue key", 401).unwrap();
    let header = req.headers().get("authorization").ok().flatten().unwrap_or_default();
    let key = crate::print_rail::key_of(&header).ok_or_else(refuse)?;
    match crate::auth::authenticate_token(&key, &ctx.env, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Owner { active_location_id: Some(loc), .. }) => Ok(loc),
        _ => Err(refuse()),
    }
}

async fn place_of(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<crate::hubstore::Place, Response> {
    let loc = venue_of(req, ctx).await?;
    crate::hubstore::Place::of_authorised(ctx, &loc).map_err(|e| Response::error(e.to_string(), 503).unwrap())
}

/// `POST /api/print/poll` — the CloudPRNT status poll. The body (the
/// printer's status) is read and not acted on today; the answer is
/// `{jobReady, mediaTypes, jobToken}`.
pub async fn poll(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match place_of(&req, &ctx).await {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let input = crate::print_rail::PollIn { now_ms: ctx.data.now_ms };
    match crate::command::send::<_, serde_json::Value>(&place, "print/poll", &input).await {
        Ok(v) => Response::from_json(&v),
        Err((status, said)) => Response::error(said, status),
    }
}

/// `GET /api/print/job/:token` — the ticket, as `text/plain`.
pub async fn job(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match place_of(&req, &ctx).await {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let token = ctx.param("token").cloned().unwrap_or_default();
    let input = crate::print_rail::JobIn { token };
    match crate::command::send::<_, serde_json::Value>(&place, "print/job", &input).await {
        Ok(v) => {
            let text = v.get("text").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
            let mut res = Response::ok(text)?;
            res.headers_mut().set("content-type", "text/plain; charset=utf-8")?;
            res.headers_mut().set("cache-control", "no-store")?;
            Ok(res)
        }
        Err((status, said)) => Response::error(said, status),
    }
}

/// `DELETE /api/print/job/:token?code=200%20OK` — the ack. No `code` is a
/// plain DELETE, which is the printer saying it printed.
pub async fn ack(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match place_of(&req, &ctx).await {
        Ok(p) => p,
        Err(r) => return Ok(r),
    };
    let token = ctx.param("token").cloned().unwrap_or_default();
    let code = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.to_string()))
        .unwrap_or_else(|| "200".into());
    let input = crate::print_rail::AckIn { token, code: code.clone(), now_ms: ctx.data.now_ms };
    let out: crate::print_rail::AckOut = match crate::command::send(&place, "print/ack", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    // GIVEN UP ON IS SAID OUT LOUD: the venue's error log is what the health
    // pane lists, and a ticket that never printed is exactly what it is for.
    if out.abandoned {
        crate::errlog::record(
            &place.ns,
            Some(&place.venue),
            "print",
            &format!("a kitchen ticket was abandoned after {} failed prints; last code {code:?}", out.tries),
        )
        .await;
    }
    Response::from_json(&out)
}

/// `GET /api/owner/print/jobs` — the tickets still in the outbox, and which of
/// them a printer holds (§3.1 step 3's `queued` / `printing`). `printed` and
/// `failed` are on the orders themselves (`kitchen.printed`,
/// `kitchen.print_failed`); this is the half only the outbox knows. Read-only.
pub async fn jobs(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let now = ctx.data.now_ms;
    let rows: Vec<serde_json::Value> = crate::outbox::waiting(&place)
        .await?
        .iter()
        .filter(|e| e.kind == crate::print_rail::KIND)
        .map(|e| {
            serde_json::json!({
                "orderId": crate::print_rail::order_of(&e.id),
                "state": crate::print_rail::state_of(Some(e), &serde_json::Value::Null, now),
                "tries": e.tries,
                "code": e.code,
            })
        })
        .collect();
    let mut res = Response::from_json(&serde_json::json!({ "jobs": rows }))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}
