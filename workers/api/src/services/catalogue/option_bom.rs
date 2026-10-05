//! `GET|POST /api/owner/products/:id/option-bom` -- AN OPTION'S RECIPE
//! (R13, W-LOST): what "extra salmon" takes off the shelf.
//!
//! GET answers the dish's options, each with its lines, and the supplies an
//! option may draw (raw ones: a semi-finished product is expanded through its
//! card at the dish's level only, so an option line naming one is refused).
//! POST sets ONE option's lines (`[]` clears it) on the dish record's
//! `optionBom` (`dowiz_hub::modifiers::bom`). The menu's own `modifierGroups`
//! and the public menu never change: the guest sees no recipe.
//!
//! The menu's door (`guard::MENU`) for both, like the dish sheet that hosts
//! the screen (`admin/option-bom.js`). Contract: `catalog.modifier_bom.v1`.

use dowiz_hub::modifiers::bom::{option_bom, options_of, set_option_bom};
use dowiz_hub::stock::BomLine;
use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The contract this route answers.
pub const CONTRACT: &str = "catalog.modifier_bom.v1";

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct LineIn {
    pub supply: String,
    pub qty: i64,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OptionBomIn {
    pub location_id: String,
    pub option: String,
    pub bom: Vec<LineIn>,
}

/// The GET answer, PURE: the dish's options with their lines, and the raw
/// supplies (`(id, record)`) an option may name.
pub fn view(product_id: &str, product_json: &str, supplies: &[(String, String)]) -> Value {
    let p: Value = serde_json::from_str(product_json).unwrap_or(Value::Null);
    let options: Vec<Value> = options_of(product_json)
        .into_iter()
        .map(|(gid, gname, id, name)| {
            let bom: Vec<Value> = option_bom(&p, &id).iter().map(|l| json!({ "supply": l.supply, "qty": l.qty })).collect();
            json!({ "group": gid, "groupName": gname, "id": id, "name": name, "bom": bom })
        })
        .collect();
    let raw: Vec<Value> = supplies
        .iter()
        .filter(|(_, j)| !dowiz_hub::prep::is_prep(j))
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(j).ok()?;
            Some(json!({ "id": id, "name": v.get("name").and_then(Value::as_str).unwrap_or(id), "unit": v.get("unit").and_then(Value::as_str).unwrap_or("g") }))
        })
        .collect();
    json!({ "contract": CONTRACT, "product": product_id, "options": options, "supplies": raw })
}

/// The POST's change to the dish record, PURE. Refused (the 400's text), the
/// record unchanged: an unknown or semi-finished supply, or anything
/// `set_option_bom` refuses.
pub fn apply(product: &mut Value, body: &OptionBomIn, supply: &dyn Fn(&str) -> Option<String>) -> std::result::Result<(), String> {
    for l in &body.bom {
        match supply(&l.supply) {
            None => return Err(format!("unknown supply {}", l.supply)),
            Some(j) if dowiz_hub::prep::is_prep(&j) => return Err(format!("{} is semi-finished: an option draws raw supplies", l.supply)),
            Some(_) => {}
        }
    }
    let lines: Vec<BomLine> = body.bom.iter().map(|l| BomLine::whole(l.supply.trim(), l.qty)).collect();
    set_option_bom(product, body.option.trim(), &lines, crate::recipe::QTY_MAX)
}

/// The GET answer derived IN THE VENUE'S OBJECT (`/fold/catalogue?q=option_bom`,
/// `hubdo/catalogue.rs`): the catalogue never crosses the hop. `null` for a dish
/// the catalogue does not have.
pub fn fold(cat: &dowiz_hub::catalog::Catalog, id: &str) -> Value {
    cat.product(id).map_or(Value::Null, |pj| view(id, &pj, &cat.supplies()))
}

pub async fn read(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use crate::services::identity::staff::guard;
    let (_, loc) = match guard::staff_venue(&req, &ctx, &guard::MENU).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing product id", 400) };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let ask = format!("https://hub/fold/catalogue?q=option_bom&id={}", crate::mcp::enc(&id));
    let (status, text) = crate::fold::ask::text(&place, &ask).await?;
    if status != 200 {
        return Response::error(text, status);
    }
    if text.trim() == "null" {
        return Response::error("not found", 404);
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| Error::RustError(format!("option-bom: unreadable answer: {e}")))?;
    Response::from_json(&v)
}

pub async fn write(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use crate::services::identity::staff::guard;
    let (_, loc) = match guard::staff_venue(&req, &ctx, &guard::MENU).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let body: OptionBomIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    if body.location_id != loc {
        return Response::error("which venue?", 400);
    }
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing product id", 400) };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // A refusal fails the closure, so the catalogue is not written for it.
    let written = crate::hubstore::with_catalog(&place, |cat| {
        let Some(pj) = cat.product(&id) else { return Err(Error::RustError(format!("{REFUSED}404:not found"))) };
        let mut p: Value = serde_json::from_str(&pj).map_err(|e| Error::RustError(format!("catalogue product unreadable: {e}")))?;
        apply(&mut p, &body, &|s| cat.supply(s)).map_err(|why| Error::RustError(format!("{REFUSED}400:{why}")))?;
        cat.set_product(&id, &p.to_string());
        Ok(view(&id, &p.to_string(), &cat.supplies()))
    })
    .await;
    match written {
        Ok(v) => Response::from_json(&v),
        Err(e) => match refusal(&e.to_string()) {
            Some((status, why)) => Response::error(why, status),
            None => Err(e),
        },
    }
}

/// The marker a refusal crosses the catalogue closure with (it can only fail with `Error`).
const REFUSED: &str = "option-bom refused:";

/// `(status, text)` of a refusal the closure raised, or `None` for a real error.
pub fn refusal(e: &str) -> Option<(u16, String)> {
    let rest = &e[e.find(REFUSED)? + REFUSED.len()..];
    let (status, why) = rest.split_once(':')?;
    Some((status.parse().ok()?, why.to_string()))
}

#[cfg(test)]
#[path = "option_bom/tests.rs"]
mod tests;
