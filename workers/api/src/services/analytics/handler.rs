//! `GET /api/owner/analytics?location_id=&days=7|30`
//!
//! ORCHESTRATION ONLY on the Worker: it authorises the owner and asks the
//! venue's object for the answer. The object builds it with `answer` (pure):
//! the orders through `fold`, the catalogue's names and currency on top.
//! Every number in it is decided in `fold`, where it has a test.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::fold;
use crate::owner::{owner_and_venue};

pub async fn analytics(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let days = fold::window(
        req.url()
            .ok()
            .and_then(|u| {
                u.query_pairs().find(|(k, _)| k == "days").map(|(_, v)| v.to_string())
            })
            .as_deref(),
    );

    // DERIVED IN THE OBJECT (BN1, `/fold/analytics`, `hubdo/reads.rs`): the
    // folded orders and the catalogue's names and currency are both there, so
    // the answer is built there by `answer` and only the answer crosses.
    let url = format!(
        "https://hub/fold/analytics?venue={}&days={days}&now={}",
        crate::mcp::enc(&loc),
        ctx.data.now_ms
    );
    let (status, text) = crate::fold::ask::text(&place, &url).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out = Response::ok(text)?;
    out.headers_mut().set("content-type", "application/json")?;
    Ok(out)
}

/// The owner's numbers over the venue's orders and catalogue. PURE: the
/// venue's object calls this (`/fold/analytics`) with the images it holds.
/// `days` is the window as `fold::window` read it.
pub fn answer(listed: Vec<crate::hubdo::OrderView>, cat: &dowiz_hub::catalog::Catalog, loc: &str, now: i64, days: i64) -> Value {
    // The venue's own record is in the catalogue and the day boundary needs
    // its time zone. This used to be computed first, from a constant.
    let zone = crate::hubstore::zone_of(
        cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref(),
    );
    let starts = fold::day_starts(zone, now, days);
    let r = fold::fold(&crate::services::orders::mine::of_venue(listed, loc), zone, &starts, now);

    json!({
        "days": days,
        "orders": r.orders,
        "revenue": r.revenue,
        "rejected": r.rejected,
        "averageOrder": r.average_order,
        "delivery": r.delivery,
        "pickup": r.pickup,
        "dineIn": r.dine_in,
        "byChannel": r.by_channel,
        "byDay": r.by_day.iter()
            .map(|d| json!({ "at": d.at, "orders": d.orders, "revenue": d.revenue }))
            .collect::<Vec<_>>(),
        "byHour": r.by_hour.to_vec(),
        "topProducts": r.top_products.iter().map(|d| json!({
            "id": d.id,
            "name": cat.product(&d.id)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|p| p.get("name").cloned())
                .unwrap_or(json!(d.id)),
            "quantity": d.quantity, "revenue": d.revenue
        })).collect::<Vec<_>>(),
        "currency": crate::services::venue::currency_of(cat),
    })
}
