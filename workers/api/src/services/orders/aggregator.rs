//! `POST /api/staff/orders/aggregator` — a marketplace order typed in from the
//! platform's tablet (§2.9, no partner API). The rules are `command::aggregator`;
//! this handler authenticates, reads the catalogue for the dishes' names and
//! recipes, and hands the object one `PlaceIn`.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::aggregator::{bell, envelope, AggregatorOut, Entry};
use crate::command::place::PlaceIn;

#[derive(Deserialize)]
struct Body {
    location_id: String,
    #[serde(flatten)]
    entry: Entry,
}

pub async fn enter(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: Body = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (by, _caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakeOrders).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let now = ctx.data.now_ms;
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &by,
        "staff.aggregator",
        &raw,
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    // EVERY DISH IS THE VENUE'S: the kitchen cooks from it and the ledger
    // reserves its recipe. A platform product id this catalogue does not have
    // is refused, not guessed at.
    let catalog = crate::hubstore::load_catalog(&place).await?.catalog;
    let loc = catalog.location().and_then(|l| serde_json::from_str::<Value>(&l).ok()).unwrap_or(Value::Null);
    let currency = loc.get("currencyCode").and_then(Value::as_str).unwrap_or("ALL").to_string();
    let venue = loc.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let mut names = Vec::new();
    let mut bom_lines = Vec::new();
    for l in &body.entry.lines {
        let Some(rec) = catalog.product(&l.product_id) else {
            return Response::error(format!("{} is not on this venue's menu", l.product_id), 400);
        };
        let name = serde_json::from_str::<Value>(&rec)
            .ok()
            .and_then(|p| p.get("name").and_then(Value::as_str).map(String::from))
            .unwrap_or_default();
        names.push((l.product_id.clone(), name));
        bom_lines.push((rec, l.quantity));
    }
    let (order_id, mut env, subtotal) = match envelope(&body.entry, &body.location_id, &currency, &names, now) {
        Ok(v) => v,
        Err(r) => return Response::error(r.message().to_string(), r.status()),
    };
    // G4: the source is stamped by the handler that builds the PlaceIn, with
    // the SET's word for the platform staff picked -- never the body's string,
    // and never a first-party source (`envelope` refused those already).
    let Some(source) = crate::services::ordering::channel::marketplace_word(&body.entry.channel) else {
        return Response::error("not a marketplace", 400);
    };
    if let Err(e) = crate::services::ordering::channel::stamp(&mut env, source) {
        return Response::error(e.to_string(), 400);
    }
    env["entered_by"] = json!(by);
    let input = PlaceIn {
        order_id,
        envelope: env.to_string(),
        seq: now as u64,
        bom_lines,
        promo: None,
        promo_code: None,
        subtotal,
        fee: 0,
        tip: 0,
        now_ms: now,
        // THE BELL, as a placed order rings it: Telegram and the print rail.
        notify_text: Some(bell(&env, &body.entry, &names, &currency, &venue)),
        // An aggregator's customer is the platform's, not the venue's card.
        stamps: None,
    };
    let out: AggregatorOut = match crate::command::send(&place, "aggregator", &input).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let answer = json!({
        "order": serde_json::from_str::<Value>(&out.stored).unwrap_or(Value::Null),
        "existing": out.existing,
    });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}
