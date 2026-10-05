//! `GET /api/owner/customers/taste/builder?location_id=&key=&min=&not_seen_days=&min_orders=&band=&weather=`
//! -- the owner's segment builder (W-SENSE row 7). I/O ONLY: the rule is `taste/builder.rs`.
//!
//! Answers how many guests fall in the segment, the six-month trend, where in the week they
//! order, and a PURCHASE PLANNING HINT: the dishes on the menu that carry the key strongly and the
//! supplies their recipes take ("segment X is active Thu evening -> stock Y"). Never a list of
//! people; reaching them is a campaign (`campaigns`, segment `taste`), which needs each guest's
//! own marketing consent on its channel.

use serde_json::{json, Map, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::taste::{self, builder::Filter, IMAGE_TASTE, KIND, TASTE_BYTES};
use crate::hubstore::Place;

/// A dish carries the key "strongly" from this share of its scale (2 of 3, or 4 of 5 rounded).
pub const STRONG: i64 = 667;
pub const PLAN_DISHES: usize = 5;
pub const PLAN_SUPPLIES: usize = 8;
const NUMERIC: [&str; 3] = ["min", "not_seen_days", "min_orders"];

/// PURE. The query as a `Filter`, or why not. `location_id` is the door's; anything else unknown
/// is refused by the closed shape.
pub fn filter_of(pairs: &[(String, String)]) -> std::result::Result<Filter, String> {
    let mut m = Map::new();
    for (k, v) in pairs.iter().filter(|(k, _)| k != "location_id") {
        let val = if NUMERIC.contains(&k.as_str()) {
            json!(v.parse::<i64>().map_err(|_| format!("{k} is a whole number"))?)
        } else {
            json!(v)
        };
        m.insert(k.clone(), val);
    }
    let f: Filter = serde_json::from_value(Value::Object(m)).map_err(|e| format!("segment: {e}"))?;
    f.check()?;
    Ok(f)
}

/// PURE. The dishes that carry `key` strongly, strongest first, and the supplies their recipes
/// name. `products` are the catalogue's (id, json); `supply_name` reads a supply's name.
pub fn plan(key: &str, products: &[(String, String)], supply_name: impl Fn(&str) -> Option<String>) -> Value {
    let mut dishes: Vec<(i64, String, String, Vec<String>)> = Vec::new();
    for (id, j) in products {
        let p: Value = serde_json::from_str(j).unwrap_or_default();
        if p.get("available").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let Some(share) = dowiz_hub::sense::of_product(&p).and_then(|s| dowiz_hub::sense::vector(&s).get(key).copied()) else {
            continue;
        };
        if share < STRONG {
            continue;
        }
        let bom: Vec<String> = p.get("bom").and_then(Value::as_array).into_iter().flatten()
            .filter_map(|l| l.get("supply").and_then(Value::as_str).map(str::to_string)).collect();
        dishes.push((share, id.clone(), p.get("name").and_then(Value::as_str).unwrap_or("").to_string(), bom));
    }
    dishes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut supplies: Vec<String> = Vec::new();
    for (_, _, _, bom) in dishes.iter().take(PLAN_DISHES) {
        for s in bom {
            let name = supply_name(s).unwrap_or_else(|| s.clone());
            if !supplies.contains(&name) && supplies.len() < PLAN_SUPPLIES {
                supplies.push(name);
            }
        }
    }
    json!({
        "dishes": dishes.iter().take(PLAN_DISHES).map(|(w, id, name, _)| json!({ "id": id, "name": name, "share": w })).collect::<Vec<_>>(),
        "supplies": supplies,
    })
}

pub async fn builder(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_who, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let pairs: Vec<(String, String)> = req.url()?.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let f = match filter_of(&pairs) {
        Ok(f) => f,
        Err(why) => return Response::error(why, 400),
    };
    let today = taste_routes_day(ctx.data.now_ms);
    let people = crate::hubstore::load_table(&place, IMAGE_TASTE, TASTE_BYTES).await?;
    let profiles: Vec<taste::Profile> = people.table.all(KIND).iter().filter_map(|(_, j)| taste::parse(j)).collect();
    let mut out = taste::builder::report(&profiles, &f, today);
    // The plan is a derived node of the catalogue, answered by the object (`/fold/catalogue?q=sense_plan`):
    // the 538 KB image never crosses the hop (tools/gates/dataflow.sh).
    let q = format!("q=sense_plan&key={}", crate::mcp::enc(&f.key));
    let mut got = crate::fold::ask::catalogue(&place, &q).await?;
    out["plan"] = got["plan"].take();
    Response::from_json(&out)
}

/// PURE. [`plan`] over a whole catalogue, with the supplies' own names: what the object answers.
pub fn plan_of(cat: &dowiz_hub::catalog::Catalog, key: &str) -> Value {
    let supply_name = |id: &str| {
        cat.supply(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()).and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_string))
    };
    plan(key, &cat.products(), supply_name)
}

fn taste_routes_day(now_ms: i64) -> i64 {
    super::taste_routes::day_of(now_ms)
}

#[cfg(test)]
#[path = "taste_builder/tests.rs"]
mod tests;
