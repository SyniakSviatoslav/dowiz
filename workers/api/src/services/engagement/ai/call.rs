//! ONE QUESTION TO A MODEL, DOWN THE PLAN (`provider.rs`) UNTIL ONE ANSWERS.
//!
//! The only file here that talks to the outside: the owner's endpoint through
//! `edge::fetch` (the OpenAI chat-completions shape Ollama, vLLM, OpenRouter
//! and the managed APIs all speak), and Workers AI through the `[ai]` binding.
//! A route that fails is recorded and the next is tried; the key is scrubbed
//! out of every word that comes back.
//!
//! WORKERS AI IS CHARGED HERE, after it answered and before anyone reads the
//! answer, through `hubstore::with_settings` (a compare-and-swap that retries):
//! two questions at once each add their own cost to what the other wrote.

use super::budget::{self, Meter};
use super::provider::{self, Cfg, Key, Off, Route};
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// What is asked.
#[derive(Clone, Debug)]
pub struct Prompt {
    pub system: String,
    pub user: String,
    pub max_tokens: i64,
}

/// An answer, and who gave it.
#[derive(Clone, Debug)]
pub struct Said {
    pub text: String,
    pub route: &'static str,
    pub model: String,
    pub neurons: i64,
}

/// A route that was tried and failed.
#[derive(Clone, Debug)]
pub struct Failed {
    pub route: &'static str,
    pub status: u16,
    pub why: String,
}

#[derive(Debug)]
pub struct Outcome {
    pub said: Option<Said>,
    pub failed: Vec<Failed>,
    pub skipped: Vec<(&'static str, Off)>,
    pub meter: Meter,
}

impl Outcome {
    /// The words an owner reads when no model answered.
    pub fn why_off(&self) -> String {
        let mut parts: Vec<String> = self.failed.iter().map(|f| format!("{}: {} {}", f.route, f.status, f.why)).collect();
        parts.extend(self.skipped.iter().filter(|(_, o)| *o != Off::NotChosen).map(|(r, o)| format!("{r}: {}", o.code())));
        parts.join("; ")
    }
    /// The provenance block every AI route answers with. No key, ever.
    pub fn json(&self) -> Value {
        json!({
            "provider": self.said.as_ref().map(|s| s.route),
            "model": self.said.as_ref().map(|s| s.model.clone()),
            "neurons": self.said.as_ref().map_or(0, |s| s.neurons),
            "skipped": self.skipped.iter().map(|(r, o)| json!({ "route": r, "why": o.code() })).collect::<Vec<_>>(),
            "failed": self.failed.iter().map(|f| json!({ "route": f.route, "status": f.status, "why": f.why })).collect::<Vec<_>>(),
            "budget": self.meter.json(),
        })
    }
}

/// The venue's share of today's Workers AI allowance.
pub fn meter(env: &Env, s: &dowiz_hub::settings::Settings, now: i64) -> Meter {
    let cap = budget::cap_of(env.var(budget::CAP_VAR).ok().as_deref());
    Meter::of(s.get(budget::SPENT_KEY).as_deref(), now, cap)
}

/// The Worker has an `[ai]` binding. In tests: a hook was installed.
pub fn has_binding(env: &Env) -> bool {
    #[cfg(test)]
    if hook::installed() {
        return true;
    }
    env.live().map(|e| e.ai(provider::BINDING).is_ok()).unwrap_or(false)
}

/// Ask, down the plan.
pub async fn complete(env: &Env, place: &crate::hubstore::Place, s: &dowiz_hub::settings::Settings, p: &Prompt, now: i64) -> Result<Outcome> {
    run(env, place, Cfg::of(s), meter(env, s, now), p, now).await
}

/// Ask with a configuration the caller holds (the test button forces a route).
pub async fn run(env: &Env, place: &crate::hubstore::Place, cfg: Cfg, m: Meter, p: &Prompt, now: i64) -> Result<Outcome> {
    let worst = budget::worst_case(&format!("{}{}", p.system, p.user), p.max_tokens);
    let plan = provider::plan(&cfg, has_binding(env), m.admits(worst));
    let mut out = Outcome { said: None, failed: Vec::new(), skipped: plan.skipped, meter: m };
    for route in plan.routes {
        let tried = match &route {
            Route::Own { endpoint, model, key } => own(endpoint, model, key.as_ref(), p).await.map(|t| (t, 0)),
            Route::Workers { model } => workers(env, model, p).await.map(|(t, used)| (t, used.unwrap_or(worst))),
        };
        match tried {
            Ok((text, neurons)) => {
                if neurons > 0 {
                    let day = budget::utc_day(now);
                    crate::hubstore::with_settings(place, move |st| {
                        let cur = st.get(budget::SPENT_KEY);
                        st.set(budget::SPENT_KEY, &budget::record(cur.as_deref(), day, neurons));
                        Ok(())
                    })
                    .await?;
                    out.meter.used += neurons;
                }
                // A model that echoes what it was sent cannot echo the key into an answer either.
                let text = cfg.key.as_ref().map_or(text.clone(), |k| k.scrub(&text));
                out.said = Some(Said { text, route: route.name(), model: route.model().to_string(), neurons });
                return Ok(out);
            }
            Err((status, why)) => {
                let why = cfg.key.as_ref().map_or(why.clone(), |k| k.scrub(&why));
                out.failed.push(Failed { route: route.name(), status, why });
            }
        }
    }
    Ok(out)
}

/// The text of a chat answer in either shape: OpenAI's `choices[0].message`
/// or Workers AI's `response`.
pub fn text_of(v: &Value) -> String {
    let t = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .or_else(|| v.get("response").and_then(Value::as_str))
        .unwrap_or("");
    provider::without_thinking(t)
}

/// `usage` as neurons, when the model reported it.
pub fn neurons_of(v: &Value) -> Option<i64> {
    let u = v.get("usage")?;
    let i = u.get("prompt_tokens").and_then(Value::as_i64)?;
    let o = u.get("completion_tokens").and_then(Value::as_i64)?;
    Some(budget::neurons(i, o))
}

async fn own(endpoint: &str, model: &str, key: Option<&Key>, p: &Prompt) -> std::result::Result<String, (u16, String)> {
    let payload = json!({
        "model": model,
        "messages": [{ "role": "system", "content": p.system }, { "role": "user", "content": p.user }],
        // Short and near-deterministic: this words numbers the system computed.
        "temperature": 0.2,
        "max_tokens": p.max_tokens,
        "stream": false,
    });
    let fail = |e: Error| (502u16, e.to_string());
    let mut headers = Headers::new();
    headers.set("content-type", "application/json").map_err(fail)?;
    if let Some(k) = key {
        headers.set("authorization", &format!("Bearer {}", k.expose())).map_err(fail)?;
    }
    let req = Request::new_with_init(
        &format!("{endpoint}/chat/completions"),
        RequestInit::new().with_method(Method::Post).with_headers(headers).with_body(Some(payload.to_string().into())),
    )
    .map_err(fail)?;
    let mut res = crate::edge::fetch(req).await.map_err(fail)?;
    if res.status_code() >= 400 {
        // The provider's own words, cut: "401 invalid api key" sends an owner
        // to the settings; "the assistant failed" sends them to a forum.
        let body = res.text().await.unwrap_or_default();
        let cut: String = body.chars().take(200).collect();
        return Err((res.status_code(), cut));
    }
    let v: Value = res.json().await.map_err(fail)?;
    let text = text_of(&v);
    if text.is_empty() {
        return Err((502, "the model answered with nothing".into()));
    }
    Ok(text)
}

#[derive(serde::Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: &'a str,
}

/// A struct, not a `json!` map: the binding converts through
/// `serde_wasm_bindgen`, which turns a map into a JS `Map` the model refuses.
#[derive(serde::Serialize)]
struct WorkersIn<'a> {
    messages: Vec<Msg<'a>>,
    max_tokens: i64,
}

async fn workers(env: &Env, model: &str, p: &Prompt) -> std::result::Result<(String, Option<i64>), (u16, String)> {
    // Qwen3 is a reasoning model; `/no_think` is its documented switch to
    // answer at once, so the few output tokens allowed are the answer's.
    let system = format!("{} /no_think", p.system);
    let input = WorkersIn {
        messages: vec![Msg { role: "system", content: &system }, Msg { role: "user", content: &p.user }],
        max_tokens: p.max_tokens,
    };
    #[cfg(test)]
    let v: Value = match hook::call(model, &serde_json::to_value(&input).unwrap_or(Value::Null)) {
        Some(r) => r.map_err(|e| (502u16, e))?,
        None => return Err((503, "Workers AI unavailable".into())),
    };
    #[cfg(not(test))]
    let v: Value = {
        let ai = env.live().and_then(|e| e.ai(provider::BINDING)).map_err(|_| (503u16, "Workers AI unavailable".to_string()))?;
        ai.run::<WorkersIn, Value>(model, input).await.map_err(|e| (502u16, e.to_string()))?
    };
    #[cfg(test)]
    let _ = env;
    let text = text_of(&v);
    if text.is_empty() {
        return Err((502, "Workers AI answered with nothing".into()));
    }
    Ok((text, neurons_of(&v)))
}

/// The `[ai]` binding in memory: a test installs what Workers AI answers.
#[cfg(test)]
pub mod hook {
    use serde_json::Value;
    use std::cell::RefCell;
    type Answer = Box<dyn Fn(&str, &Value) -> Result<Value, String>>;
    thread_local! {
        static AI: RefCell<Option<Answer>> = RefCell::new(None);
        static SEEN: RefCell<Vec<(String, Value)>> = const { RefCell::new(Vec::new()) };
    }
    pub fn answer(f: impl Fn(&str, &Value) -> Result<Value, String> + 'static) {
        AI.with(|a| *a.borrow_mut() = Some(Box::new(f)));
    }
    pub fn installed() -> bool {
        AI.with(|a| a.borrow().is_some())
    }
    pub fn seen() -> Vec<(String, Value)> {
        SEEN.with(|s| s.borrow().clone())
    }
    pub(super) fn call(model: &str, input: &Value) -> Option<Result<Value, String>> {
        SEEN.with(|s| s.borrow_mut().push((model.to_string(), input.clone())));
        AI.with(|a| a.borrow().as_ref().map(|f| f(model, input)))
    }
}
