//! The assistant, on a Worker.
//!
//! ONE DIFFERENCE FROM THE NATIVE ADAPTER, AND IT CHANGES THE PRIVACY STORY.
//! The native hub's default is a model on the venue's own machine, and its rule
//! is: if the endpoint is loopback, the facts go out whole; otherwise they are
//! redacted. A Worker has no loopback and no machine -- every endpoint it can
//! reach is somebody else's computer.
//!
//! So THE FACTS ARE ALWAYS REDACTED HERE. There is no configuration that turns
//! that off, because there is no configuration under which it would be safe:
//! the "local" branch of the native rule is unreachable on this platform, and
//! leaving the flag readable would let an owner believe they had a local model
//! when they had a hosted one holding their customers' addresses.
//!
//! Off by default, like the native one. Nothing is sent anywhere until the
//! owner turns `ai.enabled` on and names an endpoint.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

pub const SYSTEM_OWNER: &str = "\
You help the owner of one restaurant read their own live order data and stock. \
The FACTS block below is the truth; it was computed by the system, not by you. \
Never invent an order, a number, a name or a time that is not in it. \
If the answer is not in the FACTS, say you do not have it. \
Answer in the language the question was asked in. Be brief: two or three sentences, \
or a short list. Do not add pleasantries.";

pub const SYSTEM_COURIER: &str = "\
You help a delivery courier with their own current run. \
The FACTS block below is the truth; never invent an address, an order or a time. \
If the answer is not in the FACTS, say you do not have it. \
Answer in the language the question was asked in, in one or two sentences. \
Be practical: this is read on a phone, often one-handed, often outdoors.";

/// Strip what must not leave the building.
///
/// The KEY IS KEPT and its value replaced, so the model is not told the field
/// does not exist -- it is told the field was withheld. A model that thinks
/// there is no address will answer as though the order has none.
pub fn redact(facts: &Value) -> Value {
    fn scrub(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, val) in map {
                    match k.as_str() {
                        "contact" | "address" | "phone" | "name" | "note" | "courier_note" => {
                            out.insert(k.clone(), json!("[withheld]"));
                        }
                        _ => {
                            out.insert(k.clone(), scrub(val));
                        }
                    }
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(scrub).collect()),
            other => other.clone(),
        }
    }
    scrub(facts)
}

/// Ask the venue's model one question about a set of facts.
///
/// THE ROUTES ARE `services::engagement::ai`'s (W-AI, 2026-10-03): the
/// owner's own OpenAI-compatible endpoint first (the chat-completions shape
/// Ollama, vLLM, OpenRouter, Groq and the managed APIs all speak, so a venue
/// moves between them without anything here changing), then Cloudflare Workers
/// AI on the venue's daily share, then a refusal that says why. The facts are
/// redacted before either sees them. The answer's shape is unchanged; which
/// route gave it is the `x-ai-provider` header.
pub async fn ask(
    env: &Env,
    now: i64,
    place: &crate::hubstore::Place,
    system: &str,
    facts: Value,
    question: &str,
) -> Result<Response> {
    use crate::services::engagement::ai::{call, provider::Off};
    let question = question.trim();
    if question.is_empty() {
        return Response::error("no question", 400);
    }
    if question.chars().count() > 2000 {
        return Response::error("question too long", 400);
    }

    let s = crate::hubstore::load_settings(place).await?.settings;
    if !s.flag("ai.enabled") {
        // REFUSED, not broken. The console reads this to tell the owner the
        // assistant is off rather than that something failed.
        return Response::error("the assistant is off for this venue", 409);
    }
    let p = call::Prompt {
        system: system.to_string(),
        user: format!("FACTS:\n{}\n\nQUESTION:\n{question}", redact(&facts)),
        max_tokens: 400,
    };
    let out = call::complete(env, place, &s, &p, now).await?;
    if let Some(said) = &out.said {
        let mut res = Response::from_json(&json!({ "answer": said.text.clone(), "redacted": true }))?;
        res.headers_mut().set("x-ai-provider", said.route)?;
        return Ok(res);
    }
    // The provider's own words, already cut and scrubbed of the key: "401
    // invalid api key" sends an owner to the settings, "the assistant failed"
    // sends them to a forum.
    if let Some(f) = out.failed.first() {
        return Response::error(format!("the model refused: {} {}", f.status, f.why), 502);
    }
    if out.skipped.iter().any(|(_, o)| *o == Off::NotHttps) {
        return Response::error("ai.endpoint must be https from a Worker", 400);
    }
    Response::error(format!("the assistant has no model to ask: {}", out.why_off()), 409)
}
