//! Supplies and recipes in bulk (F1): a spreadsheet, a dry run, then Apply.
//!
//! THE PARSER IS THE HUB'S (`dowiz_hub::import::recipes`): pure, three
//! languages of header, `1.200,00` and `1200` the same number, every row it
//! cannot read a warning naming the row. THE WRITERS ARE THE CONSOLE'S: a
//! supply goes through `supplies::check` + `supplies::record` exactly as the
//! form's save does, and a recipe through `recipe::apply::set_bom` exactly as
//! the dish sheet's save does -- so `line_of` / `derive` / `bom_json` run and
//! the dish's kcal, weight and cost follow from its lines. ONE OBJECT TURN per
//! request, not one per row (`tools/gates/one-image.sh`).
//!
//! PREVIEW BY DEFAULT, as `menu/import`: nothing is written without `?apply=1`.

use dowiz_hub::catalog::Catalog;
use dowiz_hub::import::recipes::{self, CostScale, DraftRecipe, DraftSupply, Opts, RecipeDraft};
use serde_json::{json, Value};
use worker::*;

use crate::recipe::apply::{set_bom, Typed};
use crate::recipe::BomLineIn;
use crate::services::operations::supplies::{check, record, SupplyIn};

/// A spreadsheet of a thousand recipe lines is ~40 KiB; this is room for
/// twenty of them and refuses a file that is not a spreadsheet at all.
const MAX_BYTES: usize = dowiz_hub::CEILING_BYTES;

/// `POST /api/owner/supplies/import[?apply=1][&retire=1][&cost=hundredths]` — body is the CSV.
pub async fn import_supplies(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    bulk(req, ctx, Kind::Supplies).await
}

/// `POST /api/owner/recipes/import[?apply=1]` — body is the CSV.
pub async fn import_recipes(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    bulk(req, ctx, Kind::Recipes).await
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Supplies,
    Recipes,
}

async fn bulk(mut req: Request, ctx: RouteContext<crate::Req>, kind: Kind) -> Result<Response> {
    let loc = match crate::services::identity::staff::guard::staff_venue(&req, &ctx, &crate::services::identity::staff::guard::MENU).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let url = req.url()?;
    let q = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.to_string());
    let flag = |name: &str| q(name).is_some_and(|v| v == "1" || v == "true");
    let (apply, retire) = (flag("apply"), flag("retire"));
    let scale = if q("cost").as_deref() == Some("hundredths") { CostScale::Hundredths } else { CostScale::Major };
    let text = req.text().await?;
    if text.len() > MAX_BYTES {
        return Response::error("the file is larger than a spreadsheet of supplies or recipes", 413);
    }
    // Everything the draft is judged against is read SERVER-SIDE, now.
    let mut cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let (mut draft, preview) = read(&cat, &text, kind, scale, ctx.data.now_ms);
    let room = projected(&mut cat, &draft, kind, retire);
    let no_room = room::refusal(&room, &mut draft.warnings);
    let mut summary = json!({
        "applied": apply,
        "supplies": draft.supplies.len(),
        "recipes": draft.recipes.len(),
        "withoutRecipe": draft.without_recipe,
        "notInFile": draft.retired,
        "retired": if apply && retire { draft.retired.len() } else { 0 },
        "flattened": draft.flattened,
        "warnings": draft.warnings,
        "rows": preview,
        "catalogue": room,
    });
    if !apply {
        return Response::from_json(&summary);
    }
    // A file that produced nothing is refused: applying it is a wrong
    // separator or a missing header, not an intent.
    if draft.supplies.is_empty() && draft.recipes.is_empty() {
        return Response::error(format!("nothing to import: {}", draft.warnings.join("; ")), 400);
    }
    // The dry run's answer, given again BEFORE the write rather than as a
    // failed save after it.
    if let Some(why) = no_room {
        return Response::error(why, 413);
    }
    let written = crate::hubstore::with_catalog(&place, move |cat| {
        let n = match kind {
            Kind::Supplies => apply_supplies(cat, &draft, retire),
            Kind::Recipes => apply_recipes(cat, &draft),
        };
        n.map_err(Error::RustError)
    })
    .await?;
    summary["written"] = json!(written);
    Response::from_json(&summary)
}

/// The draft and its preview rows, against the catalogue as it is. PURE.
pub(crate) fn read(cat: &Catalog, text: &str, kind: Kind, scale: CostScale, now_ms: i64) -> (RecipeDraft, Vec<Value>) {
    let location: Option<Value> = cat.location().and_then(|j| serde_json::from_str(&j).ok());
    let zone = crate::hubstore::zone_of(location.as_ref());
    let currency = crate::services::venue::currency_of(cat);
    let products = products_of(cat);
    let (known, existing) = supplies_of(cat);
    let opts = Opts {
        currency: &currency,
        cost_scale: Some(scale),
        unit_hint: None,
        local_now_ms: dowiz_hub::tz::local_ms(zone, now_ms),
        products: &products,
        existing_supplies: &existing,
    };
    match kind {
        Kind::Supplies => {
            let d = recipes::from_csv(text, "", &opts);
            let rows = d.supplies.iter().map(|s| supply_row(s, cat.supply(&s.id).is_none())).collect();
            (d, rows)
        }
        Kind::Recipes => {
            let d = recipes::recipes_against(text, &opts, &known);
            let rows = d.recipes.iter().map(|r| recipe_row(cat, r)).collect();
            (d, rows)
        }
    }
}

fn products_of(cat: &Catalog) -> Vec<(String, String)> {
    cat.products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((id, v.get("name")?.as_str()?.to_string()))
        })
        .collect()
}

/// The catalogue's supplies: as matchable drafts, and as `(id, unit)`.
fn supplies_of(cat: &Catalog) -> (Vec<DraftSupply>, Vec<(String, String)>) {
    let mut known = Vec::new();
    let mut existing = Vec::new();
    for (id, j) in cat.supplies() {
        let Ok(v) = serde_json::from_str::<Value>(&j) else { continue };
        let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g");
        let name = v.get("name").and_then(Value::as_str).unwrap_or(&id);
        if let Some(d) = DraftSupply::known(&id, name, unit) {
            known.push(d);
        }
        existing.push((id, unit.to_string()));
    }
    (known, existing)
}

/// `GET /api/owner/products[?id=]` — the dishes AS STORED, recipe included.
///
/// The public menu never carries `bom` (a recipe is the venue's, not the
/// customer's), and it was the console's only read of the catalogue: the dish
/// sheet opened every recipe EMPTY and its save sent `bom: []`, which cleared
/// the recipe of any dish whose price was edited. The sheet reads here now,
/// and so does `e2e/gates/recipes.mjs`.
pub async fn owner_products(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, _loc, cat) =
        match crate::services::identity::staff::guard::staff_beside(&req, &ctx, &place, &crate::services::identity::staff::guard::MENU, crate::hubstore::load_catalog(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let want = req.url()?.query_pairs().find(|(k, _)| k == "id").map(|(_, v)| v.to_string());
    let products = owner_view(&cat.catalog, want.as_deref());
    let mut res = Response::from_json(&json!({ "products": products }))?;
    // Costs and recipes are the venue's; nothing between here and the owner keeps a copy.
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

/// The dishes as the owner reads them, `want` alone if given. PURE. A recipe
/// is stored lean (`{supply, qty}`); each line is read with its supply's name
/// and numbers as they are now.
pub(crate) fn owner_view(cat: &Catalog, want: Option<&str>) -> Vec<Value> {
    cat.products()
        .into_iter()
        .filter(|(id, _)| want.is_none_or(|w| w == id))
        .filter_map(|(_, j)| serde_json::from_str(&j).ok())
        .map(|mut p: Value| {
            crate::recipe::hydrate(&mut p, |s| cat.supply(s));
            p
        })
        .collect()
}

/// A draft supply as the console's own body. `None` keeps what is stored.
pub(crate) fn supply_in(d: &DraftSupply) -> SupplyIn {
    SupplyIn {
        id: d.id.clone(),
        name: Some(d.name.clone()),
        unit: Some(d.unit.to_string()),
        low_at: d.low_at,
        kind: d.kind.clone(),
        category: Some(d.category.clone()).filter(|c| !c.trim().is_empty()),
        kcal_per100: d.kcal,
        protein_per100: d.protein,
        fat_per100: d.fat,
        carbs_per100: d.carbs,
        cost_per_basis: d.cost_per_basis,
        weight_per_unit: d.weight_per_unit,
        supplier: d.supplier.clone(),
        ..SupplyIn::default()
    }
}

fn supply_row(s: &DraftSupply, new: bool) -> Value {
    json!({ "id": s.id, "name": s.name, "unit": s.unit, "kind": s.kind, "costPerBasis": s.cost_per_basis,
            "kcalPer100": s.kcal, "weightPerUnit": s.weight_per_unit, "lowAt": s.low_at, "supplier": s.supplier, "new": new })
}

fn lines_of(r: &DraftRecipe) -> Vec<BomLineIn> {
    r.lines.iter().map(|l| BomLineIn { supply: l.supply.clone(), qty: l.qty }).collect()
}

/// What the dish reads now, and what it would read after this recipe.
fn recipe_row(cat: &Catalog, r: &DraftRecipe) -> Value {
    let before: Value = cat.product(&r.product_id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
    let mut after = before.clone();
    let shown = |p: &Value| json!({ "lines": p["bom"].as_array().map_or(0, Vec::len), "kcal": p["nutrition"]["kcal"], "weightG": p["weightG"], "cost": p["cost"] });
    // As Apply writes it: what the owner typed on the dish stays theirs.
    let err = set_bom(&mut after, &lines_of(r), |s| cat.supply(s), Typed::from_record(&before)).err();
    crate::recipe::hydrate(&mut after, |s| cat.supply(s));
    json!({ "productId": r.product_id, "dish": before.get("name").cloned().unwrap_or(json!(r.dish)),
            "bom": after["bom"], "before": shown(&before), "after": shown(&after),
            "complete": after["nutritionComplete"], "error": err })
}

/// Write every supply of the draft, and retire what the file left out when
/// asked. Answers how many records were written. PURE over the catalogue.
pub(crate) fn apply_supplies(cat: &mut Catalog, draft: &RecipeDraft, retire: bool) -> std::result::Result<usize, String> {
    let mut n = 0;
    for s in &draft.supplies {
        let body = supply_in(s);
        let id = check(&body).map_err(|e| format!("{}: {e}", s.name))?;
        let existing: Value = cat.supply(&id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        cat.set_supply(&id, &record(&id, &body, &existing).to_string());
        n += 1;
    }
    if retire {
        for id in &draft.retired {
            let Some(j) = cat.supply(id) else { continue };
            let mut v: Value = serde_json::from_str(&j).unwrap_or(json!({}));
            v["active"] = json!(false);
            cat.set_supply(id, &v.to_string());
            n += 1;
        }
    }
    Ok(n)
}

/// Write every recipe of the draft onto its dish through `set_bom`. A dish
/// removed since the dry run is skipped; an unknown supply refuses the whole
/// request (nothing is saved -- the closure's error aborts the object turn).
pub(crate) fn apply_recipes(cat: &mut Catalog, draft: &RecipeDraft) -> std::result::Result<usize, String> {
    let mut n = 0;
    for r in &draft.recipes {
        let Some(pj) = cat.product(&r.product_id) else { continue };
        let mut p: Value = serde_json::from_str(&pj).map_err(|e| format!("{}: unreadable: {e}", r.product_id))?;
        // What the owner typed on this dish stays theirs (audit D19).
        let typed = Typed::from_record(&p);
        set_bom(&mut p, &lines_of(r), |s| cat.supply(s), typed).map_err(|e| format!("{}: {e}", r.dish))?;
        cat.set_product(&r.product_id, &p.to_string());
        n += 1;
    }
    if n > 0 {
        super::bump_menu_version(cat);
    }
    Ok(n)
}

pub(crate) mod room;
pub(crate) use room::projected;

#[cfg(test)]
mod tests;
