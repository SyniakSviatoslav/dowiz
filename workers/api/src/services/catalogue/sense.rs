//! `POST /api/owner/products/:id/sense/suggest` -- a DRAFT of the dish's taste, texture and aroma
//! for the owner to review (W-SENSE row 1). It NEVER WRITES: the owner edits the draft in the dish
//! sheet and saves it with the ordinary `POST /api/owner/products/:id` (`sense`), which validates it
//! again (`dowiz_hub::sense::validate`).
//!
//! THE DRAFT, in this build, is the deterministic lexicon (`dowiz_hub::sense::lexicon`): the
//! starter defaults of the common sushi menu by the dish's name, and the venue's own description
//! and ingredient words in sq/en/uk/ru. Every value carries the word that gave it. The venue's own
//! AI chain (W-AI) may add a model's draft through `dowiz_hub::sense::from_model`, which holds a
//! model's answer to the vocabulary; that wiring is the merge's (lane verdict, hand-back).

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use dowiz_hub::sense;

pub const CONTRACT: &str = "menu.sense-suggest.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct In {
    location_id: String,
}

/// PURE. The draft for one stored product, beside what it declares now.
pub fn draft(id: &str, product: &Value) -> Value {
    let s = |k: &str| product.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let ingredients: Vec<String> = product
        .get("ingredients")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let (d, why) = sense::lexicon::suggest(&s("name"), &s("description"), &ingredients);
    json!({
        "contract": CONTRACT,
        "id": id,
        "draft": d.json(),
        "why": why.iter().map(|(k, w)| json!({ "key": k, "from": w })).collect::<Vec<_>>(),
        "current": sense::of_product(product).map(|c| c.json()).unwrap_or(Value::Null),
        "source": "lexicon",
        "saved": false,
    })
}

pub async fn suggest(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: In = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product id", 400);
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = crate::courier::staff_at(&req, &ctx, &body.location_id, crate::auth::Cap::Catalog).await {
        return Ok(r);
    }
    // One product, answered by the object: the catalogue image never crosses the hop (dataflow.sh).
    let got = crate::fold::ask::catalogue(&place, &format!("q=product&id={}", crate::mcp::enc(&id))).await?;
    let Some(pj) = got["product"].as_str() else {
        return Response::error("unknown product", 404);
    };
    let p: Value = serde_json::from_str(pj).unwrap_or_default();
    Response::from_json(&draft(&id, &p))
}

#[cfg(test)]
#[path = "sense/tests.rs"]
mod tests;
