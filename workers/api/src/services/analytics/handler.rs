//! `GET /api/owner/analytics?location_id=&days=7|30`
//!
//! ORCHESTRATION ONLY. It reads the two images, hands the orders to `fold`,
//! and puts the catalogue's names and currency on the answer. Every number in
//! it is decided in `fold`, where it has a test.

use serde_json::{json, Value};
use worker::*;

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

    // The folded orders and the catalogue, fetched together: neither answer
    // depends on the other, and the analytics need a product's name.
    let (listed, cat) = futures_util::future::try_join(
        crate::hubstore::orders(&place),
        crate::hubstore::load_catalog(&place),
    )
    .await?;
    let cat = cat.catalog;
    // AFTER the catalogue, because the venue's own record is in it and the day
    // boundary needs its time zone. This used to be computed first, from a
    // constant; reading it from bytes already in hand costs nothing.
    let zone = crate::hubstore::zone_of(
        cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref(),
    );
    let now = ctx.data.now_ms;
    let starts = fold::day_starts(zone, now, days);
    let r = fold::fold(&crate::services::orders::mine::of_venue(listed, &loc), zone, &starts, now);

    Response::from_json(&json!({
        "days": days,
        "orders": r.orders,
        "revenue": r.revenue,
        "rejected": r.rejected,
        "averageOrder": r.average_order,
        "delivery": r.delivery,
        "pickup": r.pickup,
        "dineIn": r.dine_in,
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
        "currency": crate::services::venue::currency_of(&cat),
    }))
}
