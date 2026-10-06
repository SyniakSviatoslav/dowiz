//! `POST /api/owner/products/:id/sense/suggest` -- a DRAFT of the dish's taste, texture and aroma
//! for the owner to review (W-SENSE row 1). It NEVER WRITES: the owner edits the draft in the dish
//! sheet and saves it with the ordinary `POST /api/owner/products/:id` (`sense`), which validates it
//! again (`dowiz_hub::sense::validate`).
//!
//! THE DRAFT, in this build, is the deterministic lexicon (`dowiz_hub::sense::lexicon`): the
//! starter defaults of the common sushi menu by the dish's name, and the venue's own description
//! and ingredient words in sq/en/uk/ru. Every value carries the word that gave it. The venue's own
//! AI chain (W-AI) adds a model's draft through `dowiz_hub::sense::from_model` when the venue turned
//! AI on (W-TASTE, `sense/model.rs`): merged UNDER the lexicon, never trusted raw, never an error.

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

/// The dish's own words: name, description, ingredients.
fn words_of(product: &Value) -> (String, String, Vec<String>) {
    let s = |k: &str| product.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let ingredients: Vec<String> = product
        .get("ingredients")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    (s("name"), s("description"), ingredients)
}

/// PURE. The draft for one stored product, beside what it declares now (the lexicon alone).
pub fn draft(id: &str, product: &Value) -> Value {
    draft_with(id, product, None)
}

/// PURE. The draft with the venue's model's answer merged UNDER the lexicon's (W-TASTE row 3a):
/// `model_text` goes through `sense::from_model` (`sense/model.rs`), never stored or echoed.
pub fn draft_with(id: &str, product: &Value, model_text: Option<&str>) -> Value {
    let (name, description, ingredients) = words_of(product);
    let (lex, why) = sense::lexicon::suggest(&name, &description, &ingredients);
    let (d, added) = match model_text {
        Some(t) => model::merge(&lex, &model::read(t)),
        None => (lex, Vec::new()),
    };
    let mut why: Vec<Value> = why.iter().map(|(k, w)| json!({ "key": k, "from": w })).collect();
    why.extend(added.iter().map(|k| json!({ "key": k, "from": "model" })));
    json!({
        "contract": CONTRACT,
        "id": id,
        "draft": d.json(),
        "why": why,
        "current": sense::of_product(product).map(|c| c.json()).unwrap_or(Value::Null),
        "source": if added.is_empty() { "lexicon" } else { "lexicon+model" },
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
    // W-TASTE row 3a: the venue's own AI chain, only when the venue turned AI on. Any failure
    // (no route, a refusal, an error) is the lexicon draft alone, never an error to the owner.
    let mut ai = Value::Null;
    let mut said: Option<String> = None;
    if let Ok(st) = crate::hubstore::load_settings(&place).await {
        let s = st.settings;
        if crate::services::engagement::ai::provider::Cfg::of(&s).enabled {
            let (name, description, ingredients) = words_of(&p);
            let q = model::prompt(&name, &description, &ingredients);
            if let Ok(out) = crate::services::engagement::ai::call::complete(&ctx.env, &place, &s, &q, ctx.data.now_ms).await {
                ai = out.json();
                said = out.said.map(|x| x.text);
            }
        }
    }
    let mut v = draft_with(&id, &p, said.as_deref());
    v["ai"] = ai;
    Response::from_json(&v)
}

/// W-TASTE row 3a: the model's draft, held to the vocabulary and merged under the lexicon.
#[path = "sense/model.rs"]
pub mod model;

#[cfg(test)]
#[path = "sense/tests.rs"]
mod tests;
