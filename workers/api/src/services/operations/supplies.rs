//! The list of things a kitchen buys: declaring one, and retiring one.
//!
//! RETIRING IS NOT DELETING. A dish whose recipe still names a supply keeps
//! reserving it, which is the honest outcome -- the kitchen still uses it, the
//! owner just stopped tracking it -- and the ledger is kept either way.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

/// One supply as the console sends it -- and as the bulk import builds it
/// (`services/catalogue/import/bulk.rs`), so both go through [`check`] and
/// [`record`] and a file cannot write what the form could not.
#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SupplyIn {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) unit: Option<String>,
    #[serde(default)]
    pub(crate) low_at: Option<i64>,
    /// food_ingredient | condiment | packaging | utensil (`recipe::KINDS`).
    #[serde(default)]
    pub(crate) kind: Option<String>,
    /// Free text: "Fish", "Sauces", "Containers"…
    #[serde(default)]
    pub(crate) category: Option<String>,
    /// Per 100 g/ml, or per piece: kcal and grams of macros.
    #[serde(default)]
    pub(crate) kcal_per100: Option<f64>,
    #[serde(default)]
    pub(crate) protein_per100: Option<f64>,
    #[serde(default)]
    pub(crate) fat_per100: Option<f64>,
    #[serde(default)]
    pub(crate) carbs_per100: Option<f64>,
    /// Minor units per 100 g/ml, or per piece.
    #[serde(default)]
    pub(crate) cost_per_basis: Option<i64>,
    /// Grams per piece, for supplies counted in units.
    #[serde(default)]
    pub(crate) weight_per_unit: Option<f64>,
    #[serde(default)]
    pub(crate) nutrition_confirmed: Option<bool>,
    #[serde(default)]
    pub(crate) active: Option<bool>,
    /// Who the kitchen buys it from; a note, until F3's receipts name one.
    #[serde(default)]
    pub(crate) supplier: Option<String>,
}

/// `POST /api/owner/supplies` — add or edit an ingredient.
pub async fn set_supply(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: SupplyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let id = match check(&body) {
        Ok(id) => id,
        Err(why) => return Response::error(why, 400),
    };
    let rec = crate::hubstore::with_catalog(&place, move |cat| {
        let existing: Value =
            cat.supply(&id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        let rec = record(&id, &body, &existing);
        cat.set_supply(&id, &rec.to_string());
        Ok(rec)
    })
    .await?;
    Response::from_json(&rec)
}

/// Why this supply cannot be written, or its trimmed id. PURE.
pub(crate) fn check(body: &SupplyIn) -> std::result::Result<String, String> {
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Err("an ingredient needs a short id".into());
    }
    if body.low_at.is_some_and(|v| v < 0) {
        return Err("a threshold cannot be negative".into());
    }
    if let Some(k) = &body.kind {
        if !crate::recipe::KINDS.contains(&k.as_str()) {
            return Err("kind is food_ingredient, condiment, packaging or utensil".into());
        }
    }
    if let Some(u) = &body.unit {
        if !crate::recipe::UNITS.contains(&u.as_str()) {
            return Err("unit is g, ml or unit".into());
        }
    }
    for (name, v) in [("kcal", body.kcal_per100), ("protein", body.protein_per100), ("fat", body.fat_per100), ("carbs", body.carbs_per100), ("weight", body.weight_per_unit)] {
        if v.is_some_and(|x| !x.is_finite() || x < 0.0) {
            return Err(format!("{name} cannot be negative"));
        }
    }
    if body.cost_per_basis.is_some_and(|c| c < 0) {
        return Err("cost cannot be negative".into());
    }
    Ok(id)
}

/// The stored record: each field the body's value, else what was there, else
/// the default. PURE.
pub(crate) fn record(id: &str, body: &SupplyIn, existing: &Value) -> Value {
    let keep = |key: &str, given: Option<Value>, default: Value| given.or_else(|| existing.get(key).cloned()).unwrap_or(default);
    let opt = |key: &str, given: Option<Value>| given.or_else(|| existing.get(key).cloned()).unwrap_or(Value::Null);
    json!({
        "id": id,
        "name": keep("name", body.name.clone().map(Value::String), json!(id)),
        "unit": keep("unit", body.unit.clone().map(Value::String), json!("g")),
        "lowAt": keep("lowAt", body.low_at.map(|v| json!(v)), json!(0)),
        "kind": keep("kind", body.kind.clone().map(Value::String), json!(crate::recipe::KINDS[0])),
        "category": keep("category", body.category.clone().map(|c| json!(c.trim())), json!("")),
        "kcalPer100": opt("kcalPer100", body.kcal_per100.map(|v| json!(v))),
        "proteinPer100": opt("proteinPer100", body.protein_per100.map(|v| json!(v))),
        "fatPer100": opt("fatPer100", body.fat_per100.map(|v| json!(v))),
        "carbsPer100": opt("carbsPer100", body.carbs_per100.map(|v| json!(v))),
        "costPerBasis": opt("costPerBasis", body.cost_per_basis.map(|v| json!(v))),
        "weightPerUnit": opt("weightPerUnit", body.weight_per_unit.map(|v| json!(v))),
        "nutritionConfirmed": keep("nutritionConfirmed", body.nutrition_confirmed.map(|v| json!(v)), json!(false)),
        "supplier": opt("supplier", body.supplier.clone().map(|s| json!(s.trim()))),
        // Saving through the editor is an act of keeping: a retired supply
        // written again comes back to the list unless the body says otherwise.
        "active": json!(body.active.unwrap_or(true)),
    })
}

/// `POST /api/owner/supplies/:id/retire` — off the list, ledger kept. A dish
/// whose recipe still names it keeps reserving it, which is the honest
/// outcome: the kitchen still uses it, the owner just stopped tracking it.
pub async fn retire_supply(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        #[serde(rename = "location_id")]
        _location_id: Option<String>,
    }
    let _body: In = req.json().await.unwrap_or(In { _location_id: None });
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing supply id", 400) };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let done = crate::hubstore::with_catalog(&place, move |cat| {
        let Some(j) = cat.supply(&id) else { return Ok(false) };
        let mut v: Value = serde_json::from_str(&j).unwrap_or(json!({}));
        v["active"] = json!(false);
        cat.set_supply(&id, &v.to_string());
        Ok(true)
    })
    .await?;
    if !done {
        return Response::error("unknown supply", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

#[cfg(test)]
mod tests;
