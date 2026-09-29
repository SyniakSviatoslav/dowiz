//! SEMI-FINISHED PRODUCTS, THE ROUTES (SPEC-SEMI-FINISHED-2026-09-29 §e, §h):
//!
//!   POST /api/owner/preps                  add or edit a ПФ: its card is checked
//!                                          (unknown item, itself, a cycle with its
//!                                          path, depth, bounds) and every dish
//!                                          whose tree reaches it is re-derived in
//!                                          the same catalogue write;
//!   GET  /api/owner/preps                  every ПФ, hydrated: lines with names and
//!                                          cost, yield, K, cost per kg/l/piece,
//!                                          nutrition per basis, where used;
//!   GET  /api/owner/supplies/:id/uses      where an item (raw or ПФ) is used,
//!                                          transitively -- what the owner sees
//!                                          before deleting it;
//!   GET  /api/owner/products/:id/takes     what ONE sale of a dish takes off the
//!                                          shelf: its raw leaves, exact, and the cost.
//!
//! A ПФ IS A SUPPLY (`kind: "prep"`) with a `card`; `POST /api/owner/supplies`
//! refuses that kind so a card cannot be written half (`supplies::check`).
//! WHO: the owner, or staff holding `catalog` or `stock` -- the kitchen that
//! makes the batch is who writes its card (as `supplies::quick::ADD`).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use dowiz_hub::catalog::Catalog;
use dowiz_hub::prep::{self, Card, Line};

/// Re-deriving the dishes an item reaches, shared with the supply write.
pub mod rederive;

/// One card line as the console sends it.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LineIn {
    pub item: String,
    pub qty: i64,
}

/// A ПФ as the console sends it.
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepIn {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// g, ml or unit: what the yield is counted in.
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub lines: Vec<LineIn>,
    /// The batch's net output, in `unit`.
    #[serde(rename = "yield")]
    pub yield_qty: i64,
    /// Grams per piece, for a ПФ counted in pieces.
    #[serde(default)]
    pub weight_per_unit: Option<f64>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(rename = "location_id", default)]
    pub _location_id: Option<String>,
}

/// The id and the card, or why the body cannot be one. PURE; the catalogue
/// checks (unknown item, cycle, depth) are `prep::check_card`'s.
pub fn check(body: &PrepIn) -> std::result::Result<(String, Card), String> {
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Err("a semi-finished product needs a short id".into());
    }
    if let Some(u) = &body.unit {
        if !crate::recipe::UNITS.contains(&u.as_str()) {
            return Err("unit is g, ml or unit".into());
        }
    }
    if body.weight_per_unit.is_some_and(|w| !w.is_finite() || w < 0.0) {
        return Err("weight cannot be negative".into());
    }
    let lines = body.lines.iter().map(|l| Line { item: l.item.trim().to_string(), qty: l.qty }).collect();
    Ok((id, Card { lines, yield_qty: body.yield_qty }))
}

/// The stored record: the card, the words, and what was there for the rest.
/// PURE. Absent keys are left out, as `supplies::record` leaves them out.
pub fn record(id: &str, body: &PrepIn, card: &Card, existing: &Value) -> Value {
    let keep = |key: &str, given: Option<Value>, default: Value| given.or_else(|| existing.get(key).cloned()).unwrap_or(default);
    let mut rec = json!({
        "id": id,
        "name": keep("name", body.name.clone().map(|n| json!(n.trim())), json!(id)),
        "unit": keep("unit", body.unit.clone().map(Value::String), json!("g")),
        "kind": prep::KIND,
        "category": keep("category", body.category.clone().map(|c| json!(c.trim())), json!("")),
        "weightPerUnit": body.weight_per_unit.map(|w| json!(w)).or_else(|| existing.get("weightPerUnit").cloned()).unwrap_or(Value::Null),
        "card": prep::card_json(card),
        "active": json!(body.active.unwrap_or(true)),
    });
    if let Some(m) = rec.as_object_mut() {
        m.retain(|_, v| !v.is_null());
    }
    rec
}

/// Write a checked card into the catalogue and re-derive what it reaches.
/// Answers the record as stored and the dishes re-derived. PURE on `cat`.
pub fn save(cat: &mut Catalog, id: &str, body: &PrepIn, card: &Card) -> std::result::Result<(Value, Vec<String>), String> {
    let listing = cat.supplies();
    if listing.iter().any(|(i, j)| i == id && !prep::is_prep(j)) {
        return Err(format!("{id} is a raw ingredient; a semi-finished product needs its own id"));
    }
    prep::check_card(id, card, &listing).map_err(|e| e.to_string())?;
    let existing: Value = cat.supply(id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
    let rec = record(id, body, card, &existing);
    cat.set_supply(id, &rec.to_string());
    Ok((rec, rederive::dishes_using(cat, id)))
}

/// `POST /api/owner/preps` -- see the module.
pub async fn set_prep(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &super::supplies::quick::ADD).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY (W-FIX O9).
    let body: PrepIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (id, card) = match check(&body) {
        Ok(v) => v,
        Err(why) => return Response::error(why, 400),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let out = crate::hubstore::with_catalog(&place, move |cat| {
        let (rec, dishes) = save(cat, &id, &body, &card).map_err(|e| Error::RustError(format!("refused: {e}")))?;
        if !dishes.is_empty() {
            crate::services::catalogue::import::bump_menu_version(cat);
        }
        let view = crate::recipe::prep::view(&id, &rec.to_string(), &|s| cat.supply(s));
        Ok(json!({ "prep": view, "dishes": dishes }))
    })
    .await;
    match out {
        Ok(v) => Response::from_json(&v),
        Err(e) => match e.to_string().split_once("refused: ") {
            Some((_, why)) => Response::error(why.to_string(), 400),
            None => Err(e),
        },
    }
}

/// Every ПФ of the catalogue, hydrated, with where each is used. PURE.
pub fn list(cat: &Catalog) -> Vec<Value> {
    let supplies = cat.supplies();
    let products = cat.products();
    let supply = |s: &str| cat.supply(s);
    supplies
        .iter()
        .filter(|(_, j)| prep::is_prep(j))
        .map(|(id, j)| {
            let mut v = crate::recipe::prep::view(id, j, &supply);
            let u = prep::uses_of(id, &supplies, &products);
            v["uses"] = uses_json(&u);
            v
        })
        .collect()
}

fn uses_json(u: &prep::Uses) -> Value {
    let rows = |l: &[(String, String)]| l.iter().map(|(id, name)| json!({ "id": id, "name": name })).collect::<Vec<_>>();
    json!({ "preps": rows(&u.preps), "dishes": rows(&u.dishes) })
}

/// The catalogue, for a reader holding the shelf's or the menu's word.
async fn read_catalog(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<Catalog, Response> {
    let loc = crate::services::identity::staff::guard::staff_venue(req, ctx, &crate::services::identity::staff::guard::NUMBERS).await?.1;
    let place = crate::hubstore::Place::of_authorised(ctx, &loc).map_err(|e| Response::error(e.to_string(), 500).unwrap())?;
    crate::hubstore::load_catalog(&place).await.map(|l| l.catalog).map_err(|e| Response::error(e.to_string(), 500).unwrap())
}

/// `GET /api/owner/preps`.
pub async fn list_preps(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let cat = match read_catalog(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    Response::from_json(&json!({ "preps": list(&cat) }))
}

/// `GET /api/owner/supplies/:id/uses`.
pub async fn uses(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing supply id", 400) };
    let cat = match read_catalog(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let Some(j) = cat.supply(&id) else { return Response::error("unknown supply", 404) };
    let u = prep::uses_of(&id, &cat.supplies(), &cat.products());
    Response::from_json(&json!({ "id": id, "kind": if prep::is_prep(&j) { prep::KIND } else { "raw" }, "uses": uses_json(&u) }))
}

/// What one sale of `product_json` takes off the shelf, and what it costs
/// at list prices. PURE.
pub fn takes(cat: &Catalog, product_json: &str) -> Value {
    let supply = |s: &str| cat.supply(s);
    let bom = dowiz_hub::stock::bom_of(product_json);
    let lines: Vec<(String, i64)> = bom.iter().map(|l| (l.supply.clone(), l.qty)).collect();
    let price = |s: &str| supply(s).and_then(|j| crate::recipe::prep::price_micro(&j));
    match prep::expand(&lines, &supply) {
        Ok(leaves) => {
            let rows: Vec<Value> = leaves
                .iter()
                .map(|l| {
                    let sv: Value = supply(&l.item).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
                    let cost = price(&l.item).map(|p| (p * i128::from(l.uq) + i128::from(prep::MICRO) * i128::from(prep::MICRO) / 2) / (i128::from(prep::MICRO) * i128::from(prep::MICRO)));
                    // Three decimals of the base unit, rounded once.
                    let milli = (l.uq + 500) / 1000;
                    json!({ "supply": l.item, "name": sv.get("name").cloned().unwrap_or(json!(l.item)), "unit": sv.get("unit").cloned().unwrap_or(json!("g")),
                        "uq": l.uq, "qty": format!("{}.{:03}", milli / 1000, milli % 1000), "cost": cost.map(|c| json!(c as i64)).unwrap_or(Value::Null) })
                })
                .collect();
            let total = prep::cost_micro(&lines, &supply, &price).ok().flatten().map(|m| (i128::from(m) + i128::from(prep::MICRO) / 2) / i128::from(prep::MICRO));
            json!({ "leaves": rows, "cost": total.map(|c| json!(c as i64)).unwrap_or(Value::Null), "lines": bom.len() })
        }
        Err(why) => json!({ "leaves": [], "cost": Value::Null, "lines": bom.len(), "refused": why.to_string() }),
    }
}

/// `GET /api/owner/products/:id/takes`.
pub async fn takes_of(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing product id", 400) };
    let cat = match read_catalog(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let Some(j) = cat.product(&id) else { return Response::error("unknown dish", 404) };
    let mut v = takes(&cat, &j);
    v["id"] = json!(id);
    Response::from_json(&v)
}

#[cfg(test)]
mod tests;
