//! The owner's category list: every category, EMPTY ONES INCLUDED, with its
//! dish count and its kitchen / drinks side (W-DRINKS-KITCHEN). Split out of
//! `catalog_edit.rs` when the side flag took it past the 300-line line.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// `GET /api/owner/categories` — every category, EMPTY ONES INCLUDED, with
/// its dish count. The public menu drops a category with nothing to sell,
/// which is right for a customer and wrong for the owner about to put the
/// first dish into it.
pub async fn list_categories(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &crate::services::identity::staff::guard::MENU).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // DERIVED IN THE OBJECT (BN1, `/fold/catalogue?q=categories`): the list is
    // built there by `categories_view` and the list alone crosses the hop --
    // and is handed on as it came, not parsed and encoded again (W-OWNER-CPU).
    crate::fold::ask::relay_ok(&place, "https://hub/fold/catalogue?q=categories").await
}

/// Every category with its dish count, EMPTY ONES INCLUDED, sorted as the
/// owner ordered them. PURE: the venue's object answers
/// `/fold/catalogue?q=categories` with it. `side` is what the owner's
/// numbers count the category's dishes on; `sideSet` whether the owner chose
/// it or its name did (`analytics::side`).
pub(crate) fn categories_view(cat: &dowiz_hub::catalog::Catalog) -> Value {
    use crate::services::analytics::side::{flag_set, of_category};
    let products: Vec<Value> = cat.products().iter().filter_map(|(_, j)| serde_json::from_str(j).ok()).collect();
    let mut cats: Vec<Value> = cat
        .categories()
        .iter()
        .filter_map(|(id, j)| {
            let c: Value = serde_json::from_str(j).ok()?;
            let n = products.iter().filter(|p| p.get("categoryId").and_then(Value::as_str) == Some(id.as_str())).count();
            Some(json!({ "id": id, "name": c.get("name").cloned().unwrap_or(json!(id)), "sortOrder": c.get("sortOrder").and_then(Value::as_i64).unwrap_or(0), "count": n,
                         "side": of_category(&c).key(), "sideSet": flag_set(&c) }))
        })
        .collect();
    cats.sort_by_key(|c| c.get("sortOrder").and_then(Value::as_i64).unwrap_or(0));
    json!({ "categories": cats })
}
