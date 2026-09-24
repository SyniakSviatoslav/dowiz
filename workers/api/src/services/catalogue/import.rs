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
pub async fn import_menu(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
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
    let mut draft = dowiz_hub::import::from_csv(&text);

    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let existing: Vec<(String, String)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((id, v.get("name")?.as_str()?.to_string()))
        })
        .collect();
    // ONE DISH, ONE ID, BEFORE ANYTHING IS COMPARED (audit D6): a dish the
    // console made is found by its name, so the file updates it rather than
    // adding a twin -- and `retire` does not stop the original.
    let owned: Vec<(String, String, String)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            let c = v.get("categoryId").and_then(Value::as_str).unwrap_or("").to_string();
            Some((id, v.get("name")?.as_str()?.to_string(), c))
        })
        .collect();
    resolve_ids(&mut draft, &owned);
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
    let gate = allergen_gate(&place).await;
    let held = crate::hubstore::with_catalog(&place, move |cat| {
        let mut held: Vec<String> = Vec::new();
        let mut used: Vec<String> = Vec::new();
        for p in &draft.products {
            // An existing dish keeps what the file has no column for (`imported_product`).
            let old = cat.product(&p.id).and_then(|j| serde_json::from_str::<Value>(&j).ok());
            let mut rec = imported_product(p, old.as_ref(), draft.category_column);
            // THE ALLERGEN GATE HOLDS ON IMPORT TOO (audit D17), and follows the feature.
            if gate && hold_on_import(&mut rec, old.is_none(), p.available) {
                held.push(p.id.clone());
            }
            if let Some(c) = rec.get("categoryId").and_then(Value::as_str) {
                used.push(c.to_string());
            }
            cat.set_product(&p.id, &rec.to_string());
        }
        // Only the categories a dish now sits in: a file with no category
        // column must not add an empty "Menu" to the storefront.
        for c in draft.categories.iter().filter(|c| used.contains(&c.id)) {
            cat.set_category(
                &c.id,
                &json!({ "id": c.id, "name": c.name, "sortOrder": c.sort_order }).to_string(),
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
        bump_menu_version(cat);
        Ok(held)
    })
    .await?;
    summary["heldForAllergens"] = json!(held);
    Response::from_json(&summary)
}

/// Is the allergen publish gate on for this venue? THE GATE FOLLOWS THE
/// FEATURE (`owner::update_product`): a venue that switched the filter off is
/// not asked to declare what it no longer shows. Unreadable settings keep it on.
pub(crate) async fn allergen_gate(place: &crate::hubstore::Place) -> bool {
    match crate::hubstore::load_settings(place).await {
        Ok(l) => dowiz_hub::features::is_on(&l.settings, "feature.allergen_filter"),
        Err(_) => true,
    }
}

/// The note a dish is held with until its allergens are declared.
pub(crate) const UNDECLARED_NOTE: &str = "declare this dish's allergens before it goes on sale";

/// Hold an undeclared dish off sale. `true` when it was held. PURE.
///
/// THE GATE LIVED ONLY IN `update_product` (audit D17): "Add dish" and the
/// importer both wrote `available: true` with no allergens, and the dish was
/// on sale undeclared -- a guest with an allergy cannot tell "checked and
/// clear" from "nobody filled this in". Held, not refused: the dish exists,
/// the owner declares and switches it on from the dish sheet.
pub(crate) fn hold_undeclared(rec: &mut Value) -> bool {
    let on_sale = rec.get("available").and_then(Value::as_bool).unwrap_or(false);
    if !on_sale || dowiz_hub::allergens::read(&rec.to_string()).is_declared() {
        return false;
    }
    rec["available"] = json!(false);
    rec["unavailableNote"] = json!(UNDECLARED_NOTE);
    true
}

/// The import's half of the gate. `true` when the dish was held. PURE.
///
/// ONLY WHAT THE FILE PUTS ON SALE: a new dish (which lands on sale) or a row
/// whose Available column says yes. A price sheet with no Available column
/// asks nothing about sale, so an undeclared dish already on sale is left as
/// the owner has it -- stopping it would be the "import that looked like it
/// only touched prices" stopping a menu (F1), the gate's own failure mode.
pub(crate) fn hold_on_import(rec: &mut Value, fresh: bool, asked: Option<bool>) -> bool {
    (fresh || asked == Some(true)) && hold_undeclared(rec)
}

/// Point each row at the dish it already is. PURE.
///
/// The console names a dish `slug(name)` ("sake-nigiri"), the importer
/// `category-slug(name)` ("sushi-sake-nigiri"), so a re-import used to ADD a
/// second dish -- and with `retire`, stop the original, the one carrying the
/// recipe, the photo and the allergens (audit D6). A row whose id is not in
/// the catalogue takes the id of the one existing dish of the same name
/// (preferring the row's own category when the file has one). Two candidates
/// are ambiguous: the row keeps its id and the owner is told why.
pub fn resolve_ids(draft: &mut dowiz_hub::import::MenuDraft, existing: &[(String, String, String)]) {
    use dowiz_hub::import::slug;
    let mut claimed: Vec<String> = draft.products.iter().map(|p| p.id.clone()).collect();
    for i in 0..draft.products.len() {
        let p = &draft.products[i];
        if existing.iter().any(|(id, _, _)| *id == p.id) {
            continue;
        }
        let key = slug(&p.name);
        let named: Vec<&(String, String, String)> = existing
            .iter()
            .filter(|(id, n, _)| slug(n) == key && !claimed.contains(id))
            .collect();
        let in_cat: Vec<&&(String, String, String)> =
            named.iter().filter(|(_, _, c)| draft.category_column && *c == p.category_id).collect();
        let pick = match (in_cat.as_slice(), named.as_slice()) {
            ([one], _) => Some(one.0.clone()),
            (_, [one]) => Some(one.0.clone()),
            (_, []) => None,
            _ => {
                let w = format!("{:?} matches {} dishes by name; add an id column to say which", p.name, named.len());
                draft.warnings.push(w);
                None
            }
        };
        if let Some(id) = pick {
            claimed.push(id.clone());
            draft.products[i].id = id;
        }
    }
}

/// Any catalogue write moves the menu version, which is how a client notices
/// its cart went stale.
pub(crate) fn bump_menu_version(cat: &mut dowiz_hub::catalog::Catalog) {
    if let Some(lj) = cat.location() {
        if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
            let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
            l["menu_version"] = json!(v + 1);
            cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
        }
    }
}

/// The record an imported row becomes. PURE.
///
/// AN EXISTING DISH KEEPS WHAT THE FILE HAS NO COLUMN FOR: its photo, its
/// measured size, its option groups, its STATION (`bell_route`: a re-imported
/// price list must not send the bar's drinks back to the kitchen's chat) and
/// its ALLERGENS. Blanking the last of those is the worst: a re-imported price
/// list would make every declared dish undeclared, the publish gate would then
/// refuse to keep them on sale, and a venue would find its whole menu stopped
/// by an import that looked like it only touched prices.
pub fn imported_product(p: &dowiz_hub::import::DraftProduct, old: Option<&Value>, category_column: bool) -> Value {
    // EVERY KEY, NOT A LIST OF THEM (F1, 2026-09-24). This kept eight named
    // keys and dropped the rest -- so a re-imported price list erased every
    // dish's RECIPE (`bom`), and with it what the stock ledger reserves, plus
    // weight, nutrition, ingredients, taste, tags and cooking time. The old
    // record is the base now and only the file's own columns overwrite it.
    let mut rec = match old {
        Some(Value::Object(m)) => Value::Object(m.clone()),
        _ => json!({
            "imageUrl": null, "imageUrlSmall": null, "sizeCm": null,
            "station": null, "modifierGroups": null, "allergens": null
        }),
    };
    for (k, v) in [("id", json!(p.id)), ("name", json!(p.name)), ("price", json!(p.price))] {
        rec[k] = v;
    }
    // ONLY THE FILE'S OWN COLUMNS OVERWRITE (audit D5). A column the file does
    // not have is `None` and keeps what is stored; a NEW dish gets the
    // defaults a fresh row always had.
    let fresh = old.is_none();
    if category_column || fresh {
        rec["categoryId"] = json!(p.category_id);
        rec["sortOrder"] = json!(p.sort_order);
    }
    match &p.description {
        Some(d) => rec["description"] = json!(d),
        None if fresh => rec["description"] = json!(""),
        None => {}
    }
    match p.available {
        Some(a) => {
            rec["available"] = json!(a);
            // A reason for being off sale does not outlive the file saying it is on.
            if a {
                rec["unavailableNote"] = Value::Null;
            }
        }
        None if fresh => rec["available"] = json!(true),
        None => {}
    }
    rec
}

pub mod bulk;

#[cfg(test)]
mod tests;
