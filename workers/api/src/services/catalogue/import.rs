//! `POST /api/owner/import` — a spreadsheet becomes a menu.
//!
//! THE PARSER IS THE HUB'S (`dowiz_hub::import`), so a file that imports here
//! imports identically on the native adapter. This is the doorway and the
//! write.

use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

/// `POST /api/owner/menu/import?apply=true&retire=true` — body is the CSV.
///
/// PREVIEW BY DEFAULT. An import that applies on the first click is one the
/// owner cannot inspect first, and a menu is the thing customers buy from.
pub async fn import_menu(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let flag = |name: &str| {
        req.url()
            .ok()
            .and_then(|u| {
                u.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.to_string())
            })
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    };
    let (apply, retire) = (flag("apply"), flag("retire"));
    let text = req.text().await?;
    let draft = dowiz_hub::import::from_csv(&text);

    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let existing: Vec<(String, String)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((id, v.get("name")?.as_str()?.to_string()))
        })
        .collect();
    // What is on the menu now but not in the file. Reported either way, so the
    // owner sees the consequence before choosing to act on it.
    let missing: Vec<Value> = existing
        .iter()
        .filter(|(id, _)| !draft.products.iter().any(|p| &p.id == id))
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    let mut summary = json!({
        "applied": apply,
        "categories": draft.categories.len(),
        "products": draft.products.len(),
        "warnings": draft.warnings,
        "notInFile": missing,
        "retired": if apply && retire { missing.len() } else { 0 },
    });
    if !apply {
        summary["draft"] = serde_json::from_str(&draft.as_json()).unwrap_or(Value::Null);
        return Response::from_json(&summary);
    }
    // REFUSE to apply a file that produced nothing: applying an empty draft
    // would wipe a working menu because of a wrong separator or a missing
    // header, which is the exact failure the parser warns about.
    if draft.products.is_empty() {
        return Response::error(
            format!("nothing to import: {}", draft.warnings.join("; ")),
            400,
        );
    }

    let missing_ids: Vec<String> = missing
        .iter()
        .filter_map(|m| m.get("id").and_then(Value::as_str).map(String::from))
        .collect();
    crate::hubstore::with_catalog(&place, move |cat| {
        for c in &draft.categories {
            cat.set_category(
                &c.id,
                &json!({ "id": c.id, "name": c.name, "sortOrder": c.sort_order }).to_string(),
            );
        }
        for p in &draft.products {
            // AN EXISTING DISH KEEPS WHAT THE FILE HAS NO COLUMN FOR: its
            // photo, its measured size, its option groups and its ALLERGENS.
            // Blanking the last of those is the worst: a re-imported price list
            // would make every declared dish undeclared, the publish gate would
            // then refuse to keep them on sale, and a venue would find its whole
            // menu stopped by an import that looked like it only touched prices.
            let old = cat.product(&p.id).and_then(|j| serde_json::from_str::<Value>(&j).ok());
            let keep = |k: &str| {
                old.as_ref().and_then(|v| v.get(k).cloned()).unwrap_or(Value::Null)
            };
            cat.set_product(
                &p.id,
                &json!({
                    "id": p.id, "categoryId": p.category_id, "name": p.name,
                    "description": p.description, "price": p.price,
                    "available": p.available, "sortOrder": p.sort_order,
                    "imageUrl": keep("imageUrl"), "imageUrlSmall": keep("imageUrlSmall"),
                    "sizeCm": keep("sizeCm"),
                    "modifierGroups": keep("modifierGroups"), "allergens": keep("allergens")
                })
                .to_string(),
            );
        }
        if retire {
            for id in &missing_ids {
                let Some(raw) = cat.product(id) else { continue };
                let Ok(mut v) = serde_json::from_str::<Value>(&raw) else { continue };
                v["available"] = json!(false);
                v["unavailableNote"] = json!("not on the current menu");
                cat.set_product(id, &v.to_string());
            }
        }
        // Any catalogue write moves the menu version, which is how a client
        // notices its cart went stale.
        if let Some(lj) = cat.location() {
            if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
                let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
                l["menu_version"] = json!(v + 1);
                cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
            }
        }
        Ok(())
    })
    .await?;
    Response::from_json(&summary)
}
