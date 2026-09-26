//! SOLD AS IS (operator, 2026-09-26, I0c): a dish with no recipe takes nothing
//! off the shelf -- on dubin-sushi 92 of 165 dishes, mostly drinks. One tap
//! links such a dish to its own 1:1 stock item: a supply `asis-<dish>`,
//! counted in pieces, kind `resale`, named after the dish, and the dish's
//! recipe becomes that one piece. A bottle sold is then one bottle off the
//! shelf, reserved, consumed and counted like everything else.
//!
//! A dish that HAS a recipe is left alone (its recipe is the truth), and a
//! dish linked before is linked again to the same item, never a second one.
//! The recipe is written as the lean `{supply, qty}` the ledger reads; no
//! nutrition, weight or cost is derived from a bottle.
//!
//! ONE CATALOGUE WRITE for the whole list, so a category is one tap.

use dowiz_hub::catalog::Catalog;
use serde_json::{json, Value};

/// The id of a dish's own stock item.
pub fn item_of(product_id: &str) -> String {
    let mut id = format!("asis-{product_id}");
    id.truncate(64);
    id
}

/// Does this product take anything off the shelf? A non-empty `bom`.
pub fn has_recipe(p: &Value) -> bool {
    p.get("bom").and_then(Value::as_array).is_some_and(|b| !b.is_empty())
}

/// Link each of `ids` to its own piece. Answers what was linked and what was
/// skipped, and why. PURE over the catalogue.
pub fn link(cat: &mut Catalog, ids: &[String]) -> Value {
    let mut linked = Vec::new();
    let mut skipped = Vec::new();
    for id in ids {
        let Some(mut p) = cat.product(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else {
            skipped.push(json!({ "id": id, "why": "no such dish" }));
            continue;
        };
        if has_recipe(&p) {
            skipped.push(json!({ "id": id, "why": "has a recipe" }));
            continue;
        }
        let item = item_of(id);
        if cat.supply(&item).is_none() {
            let name = p.get("name").and_then(Value::as_str).unwrap_or(id).to_string();
            let category = p
                .get("categoryId")
                .and_then(Value::as_str)
                .and_then(|c| cat.categories().into_iter().find(|(cid, _)| cid == c))
                .and_then(|(_, j)| serde_json::from_str::<Value>(&j).ok())
                .and_then(|c| c.get("name").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default();
            // Through the one supply write the form and the import use.
            let body = crate::services::operations::supplies::SupplyIn {
                id: item.clone(),
                name: Some(name),
                unit: Some("unit".into()),
                kind: Some("resale".into()),
                category: Some(category),
                ..Default::default()
            };
            let rec = crate::services::operations::supplies::record(&item, &body, &json!({}));
            cat.set_supply(&item, &rec.to_string());
        }
        p["bom"] = json!([{ "supply": item, "qty": 1 }]);
        cat.set_product(id, &p.to_string());
        linked.push(json!({ "id": id, "item": item }));
    }
    json!({ "ok": true, "linked": linked, "skipped": skipped })
}

/// `POST /api/owner/stock/as-is` `{products:[..]}`, after the route's owner
/// check: ONE catalogue write for the whole list.
pub async fn write(place: &crate::hubstore::Place, ids: Vec<String>) -> worker::Result<worker::Response> {
    let out = crate::hubstore::with_catalog(place, move |cat| Ok(link(cat, &ids))).await?;
    worker::Response::from_json(&out)
}

/// Every dish that takes NOTHING off the shelf, with its category: what the
/// Stock screen and the menu flag as "does not reduce stock yet".
pub fn without_recipe(cat: &Catalog) -> Vec<Value> {
    let mut out: Vec<Value> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let p: Value = serde_json::from_str(&j).ok()?;
            (!has_recipe(&p)).then(|| json!({ "id": id, "name": p.get("name").cloned().unwrap_or(json!(id)), "categoryId": p.get("categoryId") }))
        })
        .collect();
    out.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    out
}

#[cfg(test)]
mod tests;
