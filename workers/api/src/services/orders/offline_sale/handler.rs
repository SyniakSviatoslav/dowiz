//! THE TWO ROUTES of the offline sale.
//!
//!   POST /api/staff/offline_sales   the tablet's outbox replays one sale (Cap::TakePayment)
//!   GET  /api/owner/offline_sales   the owner's pane (`ledger::pane`)
//!
//! The sale's shape is `pay.rs`'s: authorise the signer at the venue the body
//! names (`courier::staff_at`; an owner's token passes as it does there), guard
//! the request with its idempotency key, price against the catalogue the
//! object answers (`ordering::basket::ask`, the storefront's own read), send
//! ONE command, answer. Nothing here sends anything anywhere else.

use super::{rules, SaleIn, SyncIn, SyncOut};
use crate::auth::Cap;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The idempotency route, paired with the room app's queued verb.
pub const ROUTE: &str = "staff.offline_sale";

/// `POST /api/staff/offline_sales` -- `SaleIn`.
pub async fn sell(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: SaleIn = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if let Err(why) = rules::check(&body) {
        return Response::error(why, 400);
    }
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakePayment).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let now = ctx.data.now_ms;
    let idem = match crate::idempotency::guard(&place, req.headers().get("idempotency-key").ok().flatten(), &by, ROUTE, &raw, now).await {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let ids: Vec<String> = body.lines.iter().map(|l| l.product_id.clone()).collect();
    let nodes = match crate::services::ordering::basket::ask(&place, &ids, None).await {
        Ok(n) => n,
        Err(e) => return idem.answered(&place, Err(e)).await,
    };
    let (sold_at, clock) = rules::when(body.sold_at_ms, now);
    let envelope = rules::envelope(&body, &by, sold_at, now, &|id| nodes.product(id), clock.into_iter().collect());
    let input = SyncIn {
        order_id: rules::order_id(&body.sale_key),
        envelope,
        bom_lines: rules::bom_lines(&body, &|id| nodes.ledger.get(id).cloned()),
        sold_at_ms: sold_at,
        now_ms: now,
    };
    let out: SyncOut = match crate::command::send(&place, "room/offline_sync", &input).await {
        Ok(v) => v,
        Err((status, said)) => return idem.refused(&place, status, &said).await,
    };
    let answer = serde_json::to_value(&out).unwrap_or(Value::Null);
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}

/// `GET /api/owner/offline_sales?location_id=` -- `ledger::Pane`.
pub async fn pane(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, loc)) => loc,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    match crate::command::send::<_, Value>(&place, "room/offline_list", &json!({ "now_ms": ctx.data.now_ms })).await {
        Ok(v) => Response::from_json(&v),
        Err((status, msg)) => Response::error(msg, status),
    }
}
