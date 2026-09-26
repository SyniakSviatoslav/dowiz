//! The shelf: what the kitchen has, what is held against open orders, and
//! what an owner counted.
//!
//! THE LEDGER IS THE TRUTH AND IT IS APPEND-ONLY. `dowiz_hub::stock` decides
//! every movement; nothing here computes a balance.

use serde_json::{json, Value};
use worker::*;

/// Dishes with no recipe, linked to a piece they are sold as (I0c).
pub mod as_is;
/// What a movement request becomes, pure.
pub mod moves;
/// What the Stock screen shows, pure.
pub mod view;
pub use moves::StockMoveIn;
#[cfg(test)]
use moves::movement;
#[cfg(test)]
use dowiz_hub::stock::{StockEvent, WasteReason};

/// The venue's zone and today's local day, `yyyymmdd`, from its own record.
fn today_of(cat: &dowiz_hub::catalog::Catalog, now_ms: i64) -> i64 {
    let zone = crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref());
    dowiz_hub::stock::meta::day_of_local_ms(dowiz_hub::tz::local_ms(zone, now_ms))
}

//
// §4's deterministic ledger, reachable at last. A stock level is a FOLD over
// what happened to the shelf -- received, reserved, consumed, released, wasted,
// counted -- never a number somebody edits. That is what makes the refusal at
// checkout trustworthy: when an order cannot be made, the reason is in the log
// and can be replayed.
//
// THE REFUSAL IS THE AUTOMATED 86. A venue that models its ingredients gets a
// basket refused before the order exists, naming the ingredient. A venue that
// models none reserves nothing and this is a no-op -- stock control that must
// be complete before anything can be sold is stock control nobody switches on.

/// `GET /api/owner/stock` — what is on the shelf, and what is running out.
pub async fn stock(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // Two images, and they do not depend on each other either.
    let (_, _loc, (cat, log)) = match crate::services::identity::staff::guard::staff_beside(
        &req,
        &ctx,
        &place,
        &crate::services::identity::staff::guard::SHELF,
        async {
            let (c, s) = futures_util::future::join(
                crate::hubstore::load_catalog(&place),
                crate::hubstore::load_stock(&place),
            )
            .await;
            Ok((c?.catalog, s?.stock))
        },
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // ONE PASS: shelf, cost, lots and history from a single walk of the log.
    let journal = match log.journal() {
        Ok(j) => j,
        Err(e) => return Response::error(e.to_string(), 500),
    };
    let led = &journal.ledger;
    let today = today_of(&cat, ctx.data.now_ms);
    let rows: Vec<Value> = cat
        .supplies()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            // A retired supply keeps its ledger history and leaves the list.
            if v.get("active").and_then(Value::as_bool) == Some(false) {
                return None;
            }
            let level = led.level(&id);
            let low_at = v.get("lowAt").and_then(Value::as_i64).unwrap_or(0);
            let take = |k: &str| v.get(k).cloned().unwrap_or(Value::Null);
            let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
            let more = view::extras(&id, crate::recipe::basis_of(unit), &journal, today);
            let mut row = json!({
                "id": id,
                "name": v.get("name").cloned().unwrap_or(Value::Null),
                "unit": v.get("unit").cloned().unwrap_or(json!("g")),
                "kind": v.get("kind").cloned().unwrap_or(json!(crate::recipe::KINDS[0])),
                "category": v.get("category").cloned().unwrap_or(json!("")),
                "kcalPer100": take("kcalPer100"), "proteinPer100": take("proteinPer100"), "fatPer100": take("fatPer100"), "carbsPer100": take("carbsPer100"),
                "costPerBasis": take("costPerBasis"), "weightPerUnit": take("weightPerUnit"),
                "cleanPm": take("cleanPm"), "cookPm": take("cookPm"), "nutritionBasis": take("nutritionBasis"),
                "shelfDays": take("shelfDays"), "supplier": take("supplier"),
                "nutritionConfirmed": v.get("nutritionConfirmed").and_then(Value::as_bool).unwrap_or(false),
                "onHand": level.on_hand,
                "reserved": level.reserved,
                "available": level.available(),
                "lowAt": low_at,
                // Never received or counted: its zero is unknown, not empty,
                // and it neither refuses orders nor raises a low alarm.
                "counted": led.is_counted(&id),
                "low": led.is_counted(&id) && low_at > 0 && level.available() <= low_at,
                // Never counted, and orders have taken from it (I0): its level
                // is use nobody measured against -- a count is due, not an alarm.
                "needsCount": !led.is_counted(&id) && level.on_hand < 0,
            });
            if let (Some(m), Some(x)) = (row.as_object_mut(), more.as_object()) {
                for (k, val) in x {
                    m.insert(k.clone(), val.clone());
                }
            }
            Some(row)
        })
        .collect();
    let (recent, suppliers) = view::recent_and_suppliers(&journal);
    // Reservations whose order never settled. Surfaced rather than swept: a
    // stranded hold makes a kitchen believe it is out of something it has.
    let stranded: Vec<Value> = led
        .stranded()
        .into_iter()
        .map(|(order, item, qty)| json!({ "order": order, "item": item, "qty": qty }))
        .collect();
    Response::from_json(&json!({
        "supplies": rows, "stranded": stranded, "today": dowiz_hub::stock::meta::show_day(today),
        "recent": recent, "suppliers": suppliers, "sessions": view::sessions(&journal, 5),
        "expiryWarnDays": view::EXPIRY_WARN_DAYS,
        // I0c: every dish that takes nothing off the shelf yet.
        "noRecipe": as_is::without_recipe(&cat),
    }))
}

/// Who may record a movement, and at which venue: `(signer, venue)`.
///
/// A WRITE-OFF IS A STAFF ACT (§2.1): the owner, or a member of staff holding
/// `open_till` -- the drawer's holder is who bins the stock at midnight. Their
/// person id is the signer. Every other movement (delivery, count, prep, sold
/// as is) is the owner's or a member of staff holding `stock` -- the kitchen.
async fn signer_for(req: &Request, ctx: &RouteContext<crate::Req>, kind: &str) -> std::result::Result<(String, String), Response> {
    if kind != "wasted" {
        return crate::services::identity::staff::guard::staff_venue(req, ctx, &crate::services::identity::staff::guard::SHELF).await;
    }
    waste_signer(req, ctx).await
}

/// A write-off's signer: staff holding `OpenTill` (or the owner, through
/// `staff_at`'s fallback) at the ONE venue this request names -- `?location_id=`,
/// else the token's claim or the Host -- and that same venue is the one acted on.
async fn waste_signer(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<(String, String), Response> {
    let venue = match crate::owner::location_of(req) {
        Some(v) => v,
        None => crate::hubstore::Place::of_any(req, ctx)
            .await
            .map_err(|e| Response::error(format!("which venue? {e}"), 400).unwrap())?
            .venue,
    };
    let (by, _caps) = crate::courier::staff_any_at(req, ctx, &venue, &crate::services::identity::staff::guard::BIN).await?;
    Ok((by, venue))
}

/// `POST /api/owner/stock/:kind` — received, wasted, stocktake, count (a
/// session of many lines) or produced (prep). See [`moves`].
///
/// WHAT A HUMAN CAUSES. Reserved, Consumed and Released are emitted by the
/// order lifecycle and are deliberately unreachable here: a hand-written
/// reservation has no order to settle it and would strand immediately.
pub async fn stock_move(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: StockMoveIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(kind) = ctx.param("kind").cloned() else {
        return Response::error("which movement?", 400);
    };
    let (by, loc) = match signer_for(&req, &ctx, &kind).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let now = ctx.data.now_ms;
    // SOLD AS IS is a catalogue write, not a movement: one per request.
    if kind == "as-is" {
        let ids = body.products.clone().unwrap_or_default();
        if ids.is_empty() {
            return Response::error("which dishes?", 400);
        }
        return as_is::write(&place, ids).await;
    }
    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let shelf = |id: &str| {
        cat.supply(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()).and_then(|v| v.get("shelfDays").and_then(Value::as_i64))
    };
    let plan = match moves::plan(&kind, body, &by, now, today_of(&cat, now), shelf) {
        Ok(p) => p,
        Err((status, said)) => return Response::error(said, status),
    };
    if let Some(unknown) = plan.items().into_iter().find(|i| cat.supply(i).is_none()) {
        return Response::error(format!("not found: {unknown}"), 404);
    }
    let outcome = crate::hubstore::with_stock(&place, move |log| {
        log.set_clock(now);
        plan.apply(log).map_err(|e| Error::RustError(e.to_string()))
    })
    .await;
    match outcome {
        Ok(shown) => Response::from_json(&shown),
        // The ledger's refusals are the venue's business, not a server fault.
        Err(e) => Response::error(e.to_string(), 409),
    }
}

#[cfg(test)]
mod tests;
