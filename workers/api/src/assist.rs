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

/// Ask the configured model one question about a set of facts.
///
/// The OpenAI chat-completions shape, because Ollama, vLLM, OpenRouter, Groq
/// and the managed APIs all speak it -- so a venue can move between them
/// without anything here changing.
pub async fn ask(
    place: &crate::hubstore::Place,
    system: &str,
    facts: Value,
    question: &str,
) -> Result<Response> {
    let question = question.trim();
    if question.is_empty() {
        return Response::error("no question", 400);
    }
    if question.chars().count() > 2000 {
        return Response::error("question too long", 400);
    }

    let s = crate::hubstore::load_settings(&place).await?.settings;
    if !s.flag("ai.enabled") {
        // REFUSED, not broken. The console reads this to tell the owner the
        // assistant is off rather than that something failed.
        return Response::error("the assistant is off for this venue", 409);
    }
    let endpoint = s.known("ai.endpoint").trim_end_matches('/').to_string();
    if !endpoint.starts_with("https://") {
        return Response::error("ai.endpoint must be https from a Worker", 400);
    }
    let model = s.known("ai.model");
    if model.trim().is_empty() {
        return Response::error("ai.model is not set", 400);
    }

    let payload = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user",
              "content": format!("FACTS:\n{}\n\nQUESTION:\n{question}", redact(&facts)) }
        ],
        // Short and deterministic-ish: this answers a question about data the
        // system already computed, not a creative brief.
        "temperature": 0.2,
        "max_tokens": 400,
        "stream": false,
    });
    let mut headers = Headers::new();
    headers.set("content-type", "application/json")?;
    if let Some(tok) = s.get("ai.token").filter(|t| !t.trim().is_empty()) {
        headers.set("authorization", &format!("Bearer {}", tok.trim()))?;
    }
    let req = Request::new_with_init(
        &format!("{endpoint}/chat/completions"),
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(payload.to_string().into())),
    )?;
    let mut res = Fetch::Request(req).send().await?;
    if res.status_code() >= 400 {
        // The provider's own words, truncated. "The assistant failed" sends an
        // owner to a forum; "401 invalid api key" sends them to the settings.
        let body = res.text().await.unwrap_or_default();
        return Response::error(
            format!("the model refused: {} {}", res.status_code(), &body[..body.len().min(200)]),
            502,
        );
    }
    let v: Value = res.json().await?;
    let answer = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if answer.is_empty() {
        return Response::error("the model answered with nothing", 502);
    }
    Response::from_json(&json!({ "answer": answer, "redacted": true }))
}
