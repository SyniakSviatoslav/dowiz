//! The menu's shape, edited by hand: a new dish, a dish removed, a category
//! made, renamed, reordered or removed.
//!
//! Until now a dish could only be born from a CSV import or the seed. A venue
//! that adds one special on a Friday should not have to export a spreadsheet
//! for it. The records written here are the same shape the importer writes
//! (`dowiz_hub::import`), so the storefront, the console and the stock ledger
//! read them without knowing which door they came in by.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_at;

/// Ids are slugs of the name, like the importer's; a clash gets a numeric tail.
const ID_MAX: usize = 64;
const NAME_MAX: usize = 120;
/// How many `-2`, `-3`… tails are tried before giving up on a name.
const ID_TAIL_TRIES: i64 = 99;
/// A new record sorts after everything in its category.
const SORT_STEP: i64 = 10;

fn bump_menu_version(cat: &mut dowiz_hub::catalog::Catalog) {
    if let Some(lj) = cat.location() {
        if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
            let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
            l["menu_version"] = json!(v + 1);
            cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
        }
    }
}

/// A free id for a name: its slug, or the slug with the first free tail.
fn free_id(taken: impl Fn(&str) -> bool, name: &str, wanted: Option<&str>) -> Option<String> {
    let base = match wanted.map(str::trim).filter(|s| !s.is_empty()) {
        Some(w) => dowiz_hub::import::slug(w),
        None => dowiz_hub::import::slug(name),
    };
    let base: String = base.chars().take(ID_MAX).collect();
    if base.is_empty() {
        return None;
    }
    if !taken(&base) {
        return Some(base);
    }
    (2..=ID_TAIL_TRIES).map(|n| format!("{base}-{n}")).find(|c| !taken(c))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductNew {
    location_id: String,
    category_id: String,
    name: String,
    price: i64,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    available: Option<bool>,
}

/// `POST /api/owner/products` — a new dish, minimal; the editor fills the rest
/// through `POST /api/owner/products/:id`.
pub async fn create_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ProductNew = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // THE PLACE IS THE VENUE THAT WAS AUTHORISED, not the one in the token.
    // See `Place::of_authorised`: these differ for an owner of two venues, and
    // the write used to land in the other one.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    let name = body.name.trim().to_string();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Response::error("a dish needs a name", 400);
    }
    if body.price < 0 {
        return Response::error("price must be >= 0", 400);
    }
    let category = body.category_id.trim().to_string();
    let made = crate::hubstore::with_catalog(&place, move |cat| {
        let cats = cat.categories();
        if !cats.iter().any(|(id, _)| *id == category) {
            return Err(Error::RustError("unknown category".into()));
        }
        let products = cat.products();
        let Some(id) = free_id(|c| products.iter().any(|(p, _)| p == c), &name, body.id.as_deref()) else {
            return Err(Error::RustError("no id could be made from that name".into()));
        };
        let last_sort = products
            .iter()
            .filter_map(|(_, j)| serde_json::from_str::<Value>(j).ok())
            .filter(|p| p.get("categoryId").and_then(Value::as_str) == Some(category.as_str()))
            .filter_map(|p| p.get("sortOrder").and_then(Value::as_i64))
            .max()
            .unwrap_or(0);
        let rec = json!({
            "id": id, "categoryId": category, "name": name,
            "description": body.description.clone().unwrap_or_default(),
            "price": body.price, "available": body.available.unwrap_or(true),
            "sortOrder": last_sort + SORT_STEP,
        });
        cat.set_product(&id, &rec.to_string());
        bump_menu_version(cat);
        Ok(rec)
    })
    .await;
    match made {
        Ok(rec) => Response::from_json(&rec),
        Err(e) if e.to_string().contains("unknown category") => Response::error("unknown category", 400),
        Err(e) if e.to_string().contains("no id") => Response::error("no id could be made from that name", 400),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocOnly {
    location_id: String,
}

/// `POST /api/owner/products/:id/delete` — gone from the menu. Past orders
/// keep their own copy of the name and price, so nothing they show changes.
pub async fn delete_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: LocOnly = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing product id", 400) };
    // THE PLACE IS THE VENUE THAT WAS AUTHORISED, not the one in the token.
    // See `Place::of_authorised`: these differ for an owner of two venues, and
    // the write used to land in the other one.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    let removed = crate::hubstore::with_catalog(&place, move |cat| {
        let was = cat.remove_product(&id);
        if was {
            bump_menu_version(cat);
        }
        Ok(was)
    })
    .await?;
    if !removed {
        return Response::error("unknown product", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CategoryIn {
    location_id: String,
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    sort_order: Option<i64>,
}

/// `POST /api/owner/categories` — make one, or rename / reorder one by id.
pub async fn set_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: CategoryIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // THE PLACE IS THE VENUE THAT WAS AUTHORISED, not the one in the token.
    // See `Place::of_authorised`: these differ for an owner of two venues, and
    // the write used to land in the other one.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    let name = body.name.trim().to_string();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Response::error("a category needs a name", 400);
    }
    let rec = crate::hubstore::with_catalog(&place, move |cat| {
        let cats = cat.categories();
        let existing = body.id.as_deref().map(str::trim).filter(|s| !s.is_empty()).and_then(|id| {
            cats.iter().find(|(c, _)| c == id).map(|(c, j)| (c.clone(), serde_json::from_str::<Value>(j).unwrap_or(json!({}))))
        });
        let (id, sort) = match existing {
            Some((id, old)) => (id, body.sort_order.or_else(|| old.get("sortOrder").and_then(Value::as_i64)).unwrap_or(0)),
            None => {
                let Some(id) = free_id(|c| cats.iter().any(|(k, _)| k == c), &name, body.id.as_deref()) else {
                    return Err(Error::RustError("no id could be made from that name".into()));
                };
                let last = cats.iter().filter_map(|(_, j)| serde_json::from_str::<Value>(j).ok()).filter_map(|c| c.get("sortOrder").and_then(Value::as_i64)).max().unwrap_or(0);
                (id, body.sort_order.unwrap_or(last + SORT_STEP))
            }
        };
        let rec = json!({ "id": id, "name": name, "sortOrder": sort });
        cat.set_category(&id, &rec.to_string());
        bump_menu_version(cat);
        Ok(rec)
    })
    .await;
    match rec {
        Ok(rec) => Response::from_json(&rec),
        Err(e) if e.to_string().contains("no id") => Response::error("no id could be made from that name", 400),
        Err(e) => Err(e),
    }
}

/// `POST /api/owner/categories/:id/delete` — only an EMPTY category goes;
/// dishes are moved or deleted first, on purpose, so nothing vanishes by
/// accident with its heading.
pub async fn delete_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: LocOnly = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing category id", 400) };
    // THE PLACE IS THE VENUE THAT WAS AUTHORISED, not the one in the token.
    // See `Place::of_authorised`: these differ for an owner of two venues, and
    // the write used to land in the other one.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    let out = crate::hubstore::with_catalog(&place, move |cat| {
        let used = cat
            .products()
            .iter()
            .filter_map(|(_, j)| serde_json::from_str::<Value>(j).ok())
            .filter(|p| p.get("categoryId").and_then(Value::as_str) == Some(id.as_str()))
            .count();
        if used > 0 {
            return Err(Error::RustError(format!("not empty: {used}")));
        }
        let was = cat.remove_category(&id);
        if was {
            bump_menu_version(cat);
        }
        Ok(was)
    })
    .await;
    match out {
        Ok(true) => Response::from_json(&json!({ "ok": true })),
        Ok(false) => Response::error("unknown category", 404),
        Err(e) if e.to_string().starts_with("not empty") => Response::error("the category still has dishes; move or delete them first", 409),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_come_from_names_and_dodge_clashes() {
        let taken = |c: &str| c == "sake-nigiri" || c == "sake-nigiri-2";
        assert_eq!(free_id(taken, "Sake Nigiri", None).as_deref(), Some("sake-nigiri-3"));
        assert_eq!(free_id(|_| false, "Філадельфія", None).as_deref(), Some("філадельфія"));
        // A name with nothing alphanumeric still gets the importer's fallback id.
        assert_eq!(free_id(|_| false, "!!!", None).as_deref(), Some("item"));
        assert_eq!(free_id(|_| false, "x", Some("my-id")).as_deref(), Some("my-id"));
    }
}


/// `GET /api/owner/categories` — every category, EMPTY ONES INCLUDED, with
/// its dish count. The public menu drops a category with nothing to sell,
/// which is right for a customer and wrong for the owner about to put the
/// first dish into it.
pub async fn list_categories(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let loaded = crate::hubstore::load_catalog(&place).await?;
    let products: Vec<Value> = loaded.catalog.products().iter().filter_map(|(_, j)| serde_json::from_str(j).ok()).collect();
    let mut cats: Vec<Value> = loaded
        .catalog
        .categories()
        .iter()
        .filter_map(|(id, j)| {
            let c: Value = serde_json::from_str(j).ok()?;
            let n = products.iter().filter(|p| p.get("categoryId").and_then(Value::as_str) == Some(id.as_str())).count();
            Some(json!({ "id": id, "name": c.get("name").cloned().unwrap_or(json!(id)), "sortOrder": c.get("sortOrder").and_then(Value::as_i64).unwrap_or(0), "count": n }))
        })
        .collect();
    cats.sort_by_key(|c| c.get("sortOrder").and_then(Value::as_i64).unwrap_or(0));
    Response::from_json(&json!({ "categories": cats }))
}
