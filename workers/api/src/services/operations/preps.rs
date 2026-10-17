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
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use dowiz_hub::catalog::Catalog;
use dowiz_hub::prep::{self, Card, Line};
use dowiz_hub::stock::cost::CostBook;

use super::supplies::delete::review::has_mark;

/// Re-deriving the dishes an item reaches, shared with the supply write.
pub mod rederive;
/// What one sale of a dish takes off the shelf, and its cost.
mod takes;
#[cfg(test)]
pub use takes::takes;
pub use takes::takes_priced;

/// One card line as the console sends it.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LineIn {
    pub item: String,
    /// Read from its text (`recipe::qty`): `0.5`, `"0,5"`, `40` -- half a
    /// gram of saffron is a line (W-QTYHUB H2; stored `{"item","uq"}`).
    pub qty: crate::recipe::Qty,
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
    pub yield_qty: crate::recipe::Qty,
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
    let lines = body.lines.iter().map(|l| Line { item: l.item.trim().to_string(), uq: l.qty.uq() }).collect();
    Ok((id, Card { lines, yield_uq: body.yield_qty.uq() }))
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
    // The marks a deletion left THROUGH this card go with its save (2026-10-16).
    let cleared = super::supplies::delete::review::clear_via(cat, id);
    let mut dishes = rederive::dishes_using(cat, id);
    dishes.extend(cleared.into_iter().filter(|d| !dishes.contains(d)).collect::<Vec<_>>());
    Ok((rec, dishes))
}

/// `POST /api/owner/preps` -- see the module.
pub async fn set_prep(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &super::supplies::quick::ADD).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY (W-FIX O9).
    let body: PrepIn = match crate::body::parse(&mut req).await {
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

/// Every ПФ of the catalogue, hydrated at LIST prices, with where each is used. PURE.
#[cfg(test)] // the routes read the priced twin; the tests keep the list-price read
pub fn list(cat: &Catalog) -> Vec<Value> {
    list_priced(cat, &CostBook::default())
}

/// [`list`], each card's raw leaves priced at the AVERAGE a priced delivery
/// set, else the list price (`recipe::prep::averaged`, 2026-10-17). PURE.
pub fn list_priced(cat: &Catalog, book: &CostBook) -> Vec<Value> {
    let supplies = cat.supplies();
    let products = cat.products();
    let bare = |s: &str| cat.supply(s);
    let supply = crate::recipe::prep::averaged(&bare, book);
    supplies
        .iter()
        .filter(|(_, j)| prep::is_prep(j))
        .map(|(id, j)| {
            let mut v = crate::recipe::prep::view(id, j, &supply);
            let u = prep::uses_of(id, &supplies, &products);
            v["uses"] = uses_json(&u, cat);
            v
        })
        .collect()
}

/// Where-used rows, each saying whether it is marked for review (contract §2).
fn uses_json(u: &prep::Uses, cat: &Catalog) -> Value {
    let rows = |l: &[(String, String)], look: &dyn Fn(&str) -> Option<String>| {
        l.iter()
            .map(|(id, name)| {
                let mut row = json!({ "id": id, "name": name });
                // Only when marked: an unmarked row keeps the bytes it always had.
                if look(id).is_some_and(|j| has_mark(&j)) {
                    row["review"] = json!(true);
                }
                row
            })
            .collect::<Vec<_>>()
    };
    super::supplies::delete::review::with_options(json!({ "preps": rows(&u.preps, &|s| cat.supply(s)), "dishes": rows(&u.dishes, &|p| cat.product(p)) }), &u.options)
}

/// What `answer` says to a question it does not know.
pub const NO_SUCH_QUESTION: &str = "no such question: list, uses:<id> or takes:<id>";

/// THE THREE READS' ANSWERS, derived where the catalogue is: the venue's
/// object calls this (`/fold/preps?q=`, `hubdo/preps.rs`) so the catalogue
/// image never crosses the hop (R3, `tools/gates/dataflow.sh`). `q` is `list`,
/// `uses:<supply id>` or `takes:<product id>`; a refusal is a status and its
/// words, exactly the ones the Worker's handlers gave. PURE.
pub fn answer(cat: &Catalog, book: &CostBook, q: &str) -> std::result::Result<Value, (u16, &'static str)> {
    match q.split_once(':') {
        None if q == "list" => Ok(json!({ "preps": list_priced(cat, book) })),
        Some(("uses", id)) => {
            let j = cat.supply(id).ok_or((404, "unknown supply"))?;
            let u = prep::uses_of(id, &cat.supplies(), &cat.products());
            Ok(json!({ "id": id, "kind": if prep::is_prep(&j) { prep::KIND } else { "raw" }, "uses": uses_json(&u, cat) }))
        }
        Some(("takes", id)) => {
            let j = cat.product(id).ok_or((404, "unknown dish"))?;
            let mut v = takes_priced(cat, book, &j);
            v["id"] = json!(id);
            Ok(v)
        }
        _ => Err((400, NO_SUCH_QUESTION)),
    }
}

/// Ask the venue's object: the venue is checked HERE, for a reader holding
/// the shelf's or the menu's word, and the object answers the derived JSON
/// (`answer`); its refusals pass through with their status and words.
async fn ask(req: &Request, ctx: &RouteContext<crate::Req>, q: &str) -> Result<Response> {
    let loc = match crate::services::identity::staff::guard::staff_venue(req, ctx, &crate::services::identity::staff::guard::NUMBERS).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = match crate::hubstore::Place::of_authorised(ctx, &loc) {
        Ok(p) => p,
        Err(e) => return Response::error(e.to_string(), 500),
    };
    let url = format!("https://hub/fold/preps?q={}", crate::mcp::enc(q));
    let mut res = place.stub()?.fetch_with_request(Request::new(&url, Method::Get)?).await?;
    let (status, text) = (res.status_code(), res.text().await?);
    if status != 200 {
        return Response::error(text, status);
    }
    let mut out = Response::ok(text)?;
    out.headers_mut().set("content-type", "application/json")?;
    Ok(out)
}

/// `GET /api/owner/preps`.
pub async fn list_preps(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    ask(&req, &ctx, "list").await
}

/// `GET /api/owner/supplies/:id/uses`.
pub async fn uses(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing supply id", 400) };
    ask(&req, &ctx, &format!("uses:{id}")).await
}

/// `GET /api/owner/products/:id/takes`.
pub async fn takes_of(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing product id", 400) };
    ask(&req, &ctx, &format!("takes:{id}")).await
}

#[cfg(test)]
mod tests;
