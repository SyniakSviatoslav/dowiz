//! DELETING FROM THE MENU FOR GOOD (W-NOM, 2026-09-28): many dishes at once,
//! and a category WITH its dishes once the owner has confirmed how many.
//!
//! A REAL DELETE (`Catalog::remove_product`), not `available: false` -- that
//! is the stop list, and stays the reversible thing. The menu memo refolds on
//! the catalogue write (`hubdo/menu.rs`); a past order keeps its own lines
//! (id, quantity, price, and -- from W-NOM -- the name), so its history and its
//! money do not move.
//!
//! A CATEGORY WITH DISHES GOES ONLY WITH A COUNT: the request says how many
//! dishes it is deleting (`with_dishes`), the count the console showed the
//! owner; any other number -- a dish added in between -- is a 409 that names
//! the real one. Without it, a category that has dishes is refused, as always.
//!
//! WHO: many dishes, or a category with its dishes: the OWNER only. One dish,
//! or an empty category: the owner or staff holding `catalog`, as before.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::services::catalogue::import::bump_menu_version;
use dowiz_hub::catalog::Catalog;

pub const IDS_MAX: usize = 500;

/// Remove the dishes of `ids` the catalogue holds; answers those. PURE on `cat`.
pub fn remove_products(cat: &mut Catalog, ids: &[String]) -> Vec<String> {
    // A second mention of one id finds nothing left to remove.
    let out: Vec<String> = ids.iter().map(|s| s.trim()).filter(|id| !id.is_empty() && cat.remove_product(id)).map(str::to_string).collect();
    if !out.is_empty() {
        bump_menu_version(cat);
    }
    out
}

/// Why a category cannot go, or the dishes that went with it.
#[derive(Debug, PartialEq, Eq)]
pub enum CategoryOut {
    Gone(Vec<String>),
    Unknown,
    /// It holds this many dishes, and the request did not confirm that number.
    NotEmpty(usize),
}

/// Remove a category, and its dishes only when `with_dishes` is their count. PURE on `cat`.
pub fn remove_category(cat: &mut Catalog, id: &str, with_dishes: Option<usize>) -> CategoryOut {
    if !cat.categories().iter().any(|(c, _)| c == id) {
        return CategoryOut::Unknown;
    }
    let dishes: Vec<String> = cat
        .products()
        .into_iter()
        .filter(|(_, j)| serde_json::from_str::<Value>(j).ok().and_then(|p| p.get("categoryId").and_then(Value::as_str).map(|c| c == id)) == Some(true))
        .map(|(pid, _)| pid)
        .collect();
    if !dishes.is_empty() && with_dishes != Some(dishes.len()) {
        return CategoryOut::NotEmpty(dishes.len());
    }
    for d in &dishes {
        cat.remove_product(d);
    }
    cat.remove_category(id);
    bump_menu_version(cat);
    CategoryOut::Gone(dishes)
}

/// `POST /api/owner/products/delete {ids}` -- many dishes, one write, owner only.
pub async fn delete_products(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        ids: Vec<String>,
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.ids.iter().all(|s| s.trim().is_empty()) || body.ids.len() > IDS_MAX {
        return Response::error(format!("1 to {IDS_MAX} dishes at once"), 400);
    }
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let gone = crate::hubstore::with_catalog(&place, move |cat| Ok(remove_products(cat, &body.ids))).await?;
    if gone.is_empty() {
        return Response::error("unknown product", 404);
    }
    Response::from_json(&json!({ "ok": true, "deleted": gone }))
}

/// `POST /api/owner/categories/:id/delete {location_id, with_dishes?}`.
pub async fn delete_category(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        location_id: String,
        /// The number of dishes the owner was shown and confirmed.
        #[serde(default)]
        with_dishes: Option<usize>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing category id", 400) };
    // THE PLACE IS THE VENUE THAT WAS AUTHORISED, not the one in the token.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = crate::courier::staff_at(&req, &ctx, &body.location_id, crate::auth::Cap::Catalog).await {
        return Ok(r);
    }
    // Its dishes with it: the owner's act, at this same venue.
    if body.with_dishes.is_some_and(|n| n > 0) {
        match crate::owner::owner_and_venue(&req, &ctx).await {
            Ok((_, v)) if v == body.location_id => {}
            Ok(_) => return Response::error("only the owner deletes a category with its dishes", 403),
            Err(r) => return Ok(r),
        }
    }
    let with = body.with_dishes;
    let out = crate::hubstore::with_catalog(&place, move |cat| Ok(remove_category(cat, &id, with))).await?;
    match out {
        CategoryOut::Gone(dishes) => Response::from_json(&json!({ "ok": true, "dishes": dishes })),
        CategoryOut::Unknown => Response::error("unknown category", 404),
        CategoryOut::NotEmpty(n) => {
            let mut r = Response::from_json(&json!({ "error": format!("the category still has {n} dishes; move or delete them first"), "dishes": n }))?;
            r = r.with_status(409);
            Ok(r)
        }
    }
}

#[cfg(test)]
mod tests;
