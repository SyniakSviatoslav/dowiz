//! The list of things a kitchen buys: declaring one, and retiring one.
//!
//! RETIRING IS NOT DELETING. A dish whose recipe still names a supply keeps
//! reserving it, which is the honest outcome -- the kitchen still uses it, the
//! owner just stopped tracking it -- and the ledger is kept either way.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SupplyIn {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    low_at: Option<i64>,
    /// food_ingredient | condiment | packaging | utensil (`recipe::KINDS`).
    #[serde(default)]
    kind: Option<String>,
    /// Free text: "Fish", "Sauces", "Containers"…
    #[serde(default)]
    category: Option<String>,
    /// Per 100 g/ml, or per piece: kcal and grams of macros.
    #[serde(default)]
    kcal_per100: Option<f64>,
    #[serde(default)]
    protein_per100: Option<f64>,
    #[serde(default)]
    fat_per100: Option<f64>,
    #[serde(default)]
    carbs_per100: Option<f64>,
    /// Minor units per 100 g/ml, or per piece.
    #[serde(default)]
    cost_per_basis: Option<i64>,
    /// Grams per piece, for supplies counted in units.
    #[serde(default)]
    weight_per_unit: Option<f64>,
    #[serde(default)]
    nutrition_confirmed: Option<bool>,
    #[serde(default)]
    active: Option<bool>,
}

/// `POST /api/owner/supplies` — add or edit an ingredient.
pub async fn set_supply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: SupplyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Response::error("an ingredient needs a short id", 400);
    }
    if body.low_at.is_some_and(|v| v < 0) {
        return Response::error("a threshold cannot be negative", 400);
    }
    if let Some(k) = &body.kind {
        if !crate::recipe::KINDS.contains(&k.as_str()) {
            return Response::error("kind is food_ingredient, condiment, packaging or utensil", 400);
        }
    }
    if let Some(u) = &body.unit {
        if !crate::recipe::UNITS.contains(&u.as_str()) {
            return Response::error("unit is g, ml or unit", 400);
        }
    }
    for (name, v) in [("kcal", body.kcal_per100), ("protein", body.protein_per100), ("fat", body.fat_per100), ("carbs", body.carbs_per100), ("weight", body.weight_per_unit)] {
        if v.is_some_and(|x| !x.is_finite() || x < 0.0) {
            return Response::error(format!("{name} cannot be negative"), 400);
        }
    }
    if body.cost_per_basis.is_some_and(|c| c < 0) {
        return Response::error("cost cannot be negative", 400);
    }
    let rec = crate::hubstore::with_catalog(&place, move |cat| {
        let existing: Value =
            cat.supply(&id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        // Each field: the body's value, else what was there, else the default.
        let keep = |key: &str, given: Option<Value>, default: Value| given.or_else(|| existing.get(key).cloned()).unwrap_or(default);
        let opt = |key: &str, given: Option<Value>| given.or_else(|| existing.get(key).cloned()).unwrap_or(Value::Null);
        let rec = json!({
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
            // Saving through the editor is an act of keeping: a retired supply
            // written again comes back to the list unless the body says otherwise.
            "active": json!(body.active.unwrap_or(true)),
        });
        cat.set_supply(&id, &rec.to_string());
        Ok(rec)
    })
    .await?;
    Response::from_json(&rec)
}

/// `POST /api/owner/supplies/:id/retire` — off the list, ledger kept. A dish
/// whose recipe still names it keeps reserving it, which is the honest
/// outcome: the kitchen still uses it, the owner just stopped tracking it.
pub async fn retire_supply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        #[allow(dead_code)]
        location_id: Option<String>,
    }
    let _body: In = req.json().await.unwrap_or(In { location_id: None });
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing supply id", 400) };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
