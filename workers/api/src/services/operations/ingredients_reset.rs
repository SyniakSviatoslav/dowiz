//! The owner's ingredients reset: one wipe of everything the venue said about
//! its ingredients, so it can be filled in again from nothing.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// `POST /api/owner/ingredients/reset` — the owner wipes the venue's ingredient
/// data so it can be filled in again from nothing: every supply is DELETED
/// (not retired), every dish loses its recipe, the values the recipe derived
/// and its allergen list (back to "not declared", not "contains none"), and
/// the stock ledger is replaced by an empty one.
///
/// OWNER ONLY, and the body must repeat the venue id as `confirm`: a wipe is
/// not something a stray click or a replayed request should do. Re-running it
/// is harmless, which is why the stock reset may go first and the catalogue
/// second: a failure between them is finished by running it again.
pub async fn reset_ingredients(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        confirm: String,
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    let body: In = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    if body.confirm != loc {
        return Response::error("confirm must repeat this venue's id", 400);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let stock: Value = match crate::command::send(&place, "stock_reset", &json!({})).await {
        Ok(v) => v,
        Err((status, said)) => return Response::error(said, status),
    };
    let (supplies, recipes, allergens) = crate::hubstore::with_catalog(&place, |cat| {
        let ids: Vec<String> = cat.supplies().into_iter().map(|(id, _)| id).collect();
        for id in &ids {
            cat.remove_supply(id);
        }
        let (mut recipes, mut allergens) = (0usize, 0usize);
        for (id, j) in cat.products() {
            let mut p: Value = serde_json::from_str(&j).unwrap_or(json!({}));
            let had_bom = p.get("bom").is_some_and(|b| !b.is_null());
            let had_allergens = p.get("allergens").is_some_and(|a| !a.is_null());
            if !had_bom && !had_allergens {
                continue;
            }
            if had_bom {
                let typed = crate::recipe::apply::Typed::from_record(&p);
                let _ = crate::recipe::apply::set_bom(&mut p, &[], |_| None, typed);
                recipes += 1;
            }
            if had_allergens {
                p["allergens"] = Value::Null;
                allergens += 1;
            }
            cat.set_product(&id, &p.to_string());
        }
        Ok((ids.len(), recipes, allergens))
    })
    .await?;
    Response::from_json(&json!({
        "supplies": supplies, "recipes": recipes, "allergens": allergens,
        "stockRecords": stock.get("dropped").cloned().unwrap_or(Value::Null),
    }))
}
