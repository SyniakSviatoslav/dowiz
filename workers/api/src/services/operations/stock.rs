//! The shelf: what the kitchen has, what is held against open orders, and
//! what an owner counted.
//!
//! THE LEDGER IS THE TRUTH AND IT IS APPEND-ONLY. `dowiz_hub::stock` decides
//! every movement; nothing here computes a balance.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::owner::owner_and_venue;
use dowiz_hub::stock::{StockEvent, WasteReason};

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
    let (_, _loc, (cat, log)) = match crate::owner::owner_beside(
        &req,
        &ctx,
        &place,
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
    let led = match log.ledger() {
        Ok(l) => l,
        Err(e) => return Response::error(e.to_string(), 500),
    };
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
            Some(json!({
                "id": id,
                "name": v.get("name").cloned().unwrap_or(Value::Null),
                "unit": v.get("unit").cloned().unwrap_or(json!("g")),
                "kind": v.get("kind").cloned().unwrap_or(json!(crate::recipe::KINDS[0])),
                "category": v.get("category").cloned().unwrap_or(json!("")),
                "kcalPer100": take("kcalPer100"), "proteinPer100": take("proteinPer100"), "fatPer100": take("fatPer100"), "carbsPer100": take("carbsPer100"),
                "costPerBasis": take("costPerBasis"), "weightPerUnit": take("weightPerUnit"),
                "nutritionConfirmed": v.get("nutritionConfirmed").and_then(Value::as_bool).unwrap_or(false),
                "onHand": level.on_hand,
                "reserved": level.reserved,
                "available": level.available(),
                "lowAt": low_at,
                "low": low_at > 0 && level.available() <= low_at,
            }))
        })
        .collect();
    // Reservations whose order never settled. Surfaced rather than swept: a
    // stranded hold makes a kitchen believe it is out of something it has.
    let stranded: Vec<Value> = led
        .stranded()
        .into_iter()
        .map(|(order, item, qty)| json!({ "order": order, "item": item, "qty": qty }))
        .collect();
    Response::from_json(&json!({ "supplies": rows, "stranded": stranded }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StockMoveIn {
    item: String,
    #[serde(default)]
    qty: Option<i64>,
    /// For a stocktake: what was actually counted.
    #[serde(default)]
    observed: Option<i64>,
    /// For waste: one of `WasteReason::allowed_words()`. Required.
    #[serde(default)]
    reason: Option<String>,
    // NO `by`. THE SIGNER IS WHO AUTHENTICATED, never a field the caller
    // fills in: a body that could name its signer is a write-off anybody can
    // put on somebody else. `deny_unknown_fields` turns a `by` into a 400.
}

/// The event a movement is, signed by `by` -- the AUTHENTICATED principal.
///
/// PURE, so the refusals are tested without a Worker. A waste reason outside
/// the closed set, or none, is a 400 naming the allowed words: never a
/// default, or the report says the kitchen spoils everything (§2.1).
fn movement(kind: &str, body: StockMoveIn, by: &str, now_ms: i64) -> std::result::Result<StockEvent, (u16, String)> {
    let item = body.item.trim().to_string();
    if item.is_empty() {
        return Err((400, "which ingredient?".into()));
    }
    let by = by.to_string();
    match kind {
        "received" => body.qty.map(|qty| StockEvent::Received { item, qty }).ok_or((400, "how much?".into())),
        "wasted" => {
            let qty = body.qty.ok_or((400, "how much?".to_string()))?;
            let reason = match body.reason.as_deref().map(str::trim) {
                None | Some("") => {
                    return Err((400, format!("a write-off says why: one of {}", WasteReason::allowed_words())))
                }
                Some(r) => WasteReason::from_str(r).ok_or_else(|| {
                    (400, format!("{r:?} is not a waste reason: one of {}", WasteReason::allowed_words()))
                })?,
            };
            Ok(StockEvent::Wasted { item, qty, reason, by })
        }
        "stocktake" => body
            .observed
            .map(|observed| StockEvent::Stocktake { item, observed, stocktake_id: format!("st_{now_ms}"), by })
            .ok_or((400, "what was counted?".into())),
        other => Err((400, format!("no such movement: {other}"))),
    }
}

/// Who may record a movement, and at which venue: `(signer, venue)`.
///
/// A WRITE-OFF IS A STAFF ACT (§2.1): the owner, or a member of staff holding
/// `open_till` -- the drawer's holder is who bins the stock at midnight. Their
/// person id is the signer. A delivery received and a count stay the owner's.
async fn signer_for(req: &Request, ctx: &RouteContext<crate::Req>, kind: &str) -> std::result::Result<(String, String), Response> {
    if kind != "wasted" {
        return owner_and_venue(req, ctx).await;
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
    let (by, _caps) = crate::courier::staff_at(req, ctx, &venue, Cap::OpenTill).await?;
    Ok((by, venue))
}

/// `POST /api/owner/stock/:kind` — received, wasted or counted.
///
/// THE THREE A HUMAN CAUSES. Reserved, Consumed and Released are emitted by the
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
    let ev = match movement(&kind, body, &by, ctx.data.now_ms) {
        Ok(ev) => ev,
        Err((status, said)) => return Response::error(said, status),
    };
    if crate::hubstore::load_catalog(&place).await?.catalog.supply(ev.item()).is_none() {
        return Response::error("not found", 404);
    }
    let outcome = crate::hubstore::with_stock(&place, move |log| {
        log.append(&ev).map_err(|e| Error::RustError(e.to_string()))
    })
    .await;
    match outcome {
        Ok(()) => Response::from_json(&json!({ "ok": true })),
        // The ledger's refusals are the venue's business, not a server fault.
        Err(e) => Response::error(e.to_string(), 409),
    }
}

#[cfg(test)]
mod tests;
