//! The room's HTTP surface for a round: read the room, amend a round.
//!
//! EVERY HANDLER HERE HAS ONE SHAPE: authorise the signer for a capability at
//! the venue the request names (`courier::staff_at`), do the part that belongs
//! to the Worker — pricing, the catalogue — and send ONE command to the
//! venue's object, which decides and writes in one turn. No handler here
//! writes an image itself (`tools/gates/one-image.sh`).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::Cap;
use crate::command::amend::{AmendIn, AmendOut, Op};

/// The venue a request names: `?location_id=` or the body's `location_id`.
pub fn query_location(req: &Request) -> Option<String> {
    req.url().ok()?.query_pairs().find(|(k, _)| k == "location_id").map(|(_, v)| v.to_string())
}

/// `GET /api/staff/room?location_id=` — every open sitting, from the same
/// memoised projection the console reads.
pub async fn room_view(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(loc) = query_location(&req) else {
        return Response::error("location_id is required", 400);
    };
    if let Err(r) = crate::courier::staff_at(&req, &ctx, &loc, Cap::TakeOrders).await {
        return Ok(r);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let listed = crate::hubstore::orders(&place).await?;
    let mine: Vec<_> = listed
        .into_iter()
        .filter(|o| {
            serde_json::from_str::<Value>(&o.order_json)
                .is_ok_and(|v| v.get("location_id").and_then(Value::as_str) == Some(loc.as_str()))
        })
        .collect();
    Response::from_json(&json!({ "sittings": crate::command::sitting::room(&mine) }))
}

/// What a tablet sends: an INTENT per op, never a price.
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum WireOp {
    Add { product_id: String, #[serde(default)] modifier_ids: Vec<String>, quantity: i64 },
    Remove { line: usize },
    SetQty { line: usize, qty: i64 },
    Comp { line: usize },
    Table { table: String },
}

#[derive(Deserialize)]
struct AmendBody {
    location_id: String,
    base_seq: u64,
    ops: Vec<WireOp>,
    #[serde(default)]
    reason: Option<String>,
}

/// Price one added line with THE pricer, exactly as a basket line is priced.
fn priced_line(
    catalog: &dowiz_hub::catalog::Catalog,
    product_id: &str,
    modifier_ids: &[String],
    quantity: i64,
) -> std::result::Result<Value, (u16, String)> {
    use crate::services::ordering::pricing::{price_basket, Want};
    let basket = price_basket(|id| catalog.product(id), [Want { product_id, modifier_ids, quantity }])
        .map_err(|r| (r.status(), r.text()))?;
    let l = basket.lines.into_iter().next().ok_or((400, "nothing priced".to_string()))?;
    let mut line = json!({
        "product_id": l.product_id, "modifier_ids": l.modifier_ids,
        "quantity": l.quantity, "unit_price": l.unit_price, "name": l.name,
    });
    // A round's added line is rung at its own station (`bell_route`).
    crate::bell_route::stamp_line(&mut line, l.station);
    Ok(line)
}

/// Every product that carries a recipe, so the object can move the shelf with
/// the lines. Empty at a venue that models no ingredients.
pub fn recipes(catalog: &dowiz_hub::catalog::Catalog) -> Vec<(String, String)> {
    catalog
        .products()
        .into_iter()
        .filter(|(_, j)| !dowiz_hub::stock::bom_of(j).is_empty())
        .collect()
}

/// `POST /api/staff/orders/:id/amend` — `{location_id, base_seq, ops, reason?}`.
pub async fn amend(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let body: AmendBody = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let (by, caps) = match crate::courier::staff_at(&req, &ctx, &body.location_id, Cap::TakeOrders).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let idem = match crate::idempotency::guard(
        &place, req.headers().get("idempotency-key").ok().flatten(), &by,
        "staff.amend", &format!("{id}:{raw}"), ctx.data.now_ms,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    let loaded = match crate::hubstore::load_catalog(&place).await {
        Ok(l) => l,
        Err(e) => return idem.answered(&place, Err(e)).await,
    };
    let mut ops = Vec::with_capacity(body.ops.len());
    for w in body.ops {
        ops.push(match w {
            WireOp::Add { product_id, modifier_ids, quantity } => {
                match priced_line(&loaded.catalog, &product_id, &modifier_ids, quantity) {
                    Ok(line) => Op::Add { line },
                    Err((s, m)) => return idem.refused(&place, s, &m).await,
                }
            }
            WireOp::Remove { line } => Op::Remove { line },
            WireOp::SetQty { line, qty } => Op::SetQty { line, qty },
            WireOp::Comp { line } => Op::Comp { line },
            WireOp::Table { table } => Op::Table { table },
        });
    }
    let input = AmendIn {
        order_id: id,
        location_id: body.location_id,
        base_seq: body.base_seq,
        ops,
        by,
        reason: body.reason,
        may_void: caps.allows(Cap::Void),
        boms: recipes(&loaded.catalog),
        now_ms: ctx.data.now_ms,
    };
    let out: AmendOut = match crate::command::send(&place, "room/amend", &input).await {
        Ok(v) => v,
        Err((status, said)) => return idem.refused(&place, status, &said).await,
    };
    let answer = json!({ "order": serde_json::from_str::<Value>(&out.merged).unwrap_or(Value::Null), "seq": out.seq });
    idem.done(&place, 200, &answer.to_string()).await;
    Response::from_json(&answer)
}
