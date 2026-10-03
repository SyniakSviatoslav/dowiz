//! THE OWNER'S AI: ITS SETTINGS CARD, ITS QUESTIONS, ITS EXPLANATIONS (lane W-AI, 2026-10-03).
//!
//!   GET  /api/owner/ai           the configuration (the key only as "set"), today's
//!                                Workers AI budget, the plan of routes and why each other is off
//!   POST /api/owner/ai/test      one tiny prompt down the chosen route: ok, needs-key, ...
//!   POST /api/owner/ai/ask       a question about the venue's numbers, answered from its folds
//!   GET  /api/owner/ai/explain   the explain cards of the analytics or the kitchen screen
//!
//! THE NUMBERS ARE THE FOLDS'. A question becomes a closed [`query::Query`]
//! (by the 4-language lexicon, or by a model asked to pick from the list and
//! held to it by `query::validate`); the number is read from the same
//! `/fold/analytics` or `/fold/kitchen` answer the screens draw; the sentence is
//! a template, which a model may reword only if every number survives
//! (`words::keeps_numbers`). Every number carries its source route, the
//! pointers into it and the days whose records it sums.
//!
//! OWNER ONLY: margins and takings are the owner's (the kitchen's own screen
//! strips them, `access::numbers_for_kitchen`). NO KEY IN ANY ANSWER: the
//! settings card says whether a key is set, never what it is; a provider's
//! error text is scrubbed of it (`provider::Key::scrub`).

pub mod answer;
pub mod budget;
pub mod call;
pub mod explain;
pub mod lexicon;
pub mod provider;
pub mod query;
pub mod words;

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::owner::owner_and_venue;
use answer::Fold;
use provider::{Cfg, Mode, Off};

/// The contract every route here answers under (`tools/live-proof/contracts/feature-ai.json`).
pub const CONTRACT: &str = "ai.owner.v1";
/// The longest question read.
pub const MAX_QUESTION: usize = 400;

const SYSTEM_REWORD: &str = "You reword short sentences about a restaurant's own numbers for its owner. \
Keep EVERY number exactly as written, digits and all, and add no other number. Keep the language of the sentence. \
One sentence per line, the same count of lines, in the same order. No pleasantries, no markdown.";

fn param(req: &Request, k: &str) -> Option<String> {
    req.url().ok().and_then(|u| u.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.to_string()))
}

fn lang_or(asked: Option<&str>, question: &str) -> &'static str {
    asked.and_then(dowiz_hub::lang::norm).unwrap_or_else(|| lexicon::lang_of(question))
}

/// `GET /api/owner/ai`
pub async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let cfg = Cfg::of(&s);
    let m = call::meter(&ctx.env, &s, ctx.data.now_ms);
    let binding = call::has_binding(&ctx.env);
    let plan = provider::plan(&cfg, binding, m.left() > 0);
    Response::from_json(&json!({
        "contract": CONTRACT,
        "enabled": cfg.enabled, "mode": cfg.mode.word(), "endpoint": cfg.endpoint, "model": cfg.model,
        "keySet": cfg.key.is_some(),
        "workersAi": { "available": binding, "model": provider::WORKERS_MODEL },
        "budget": m.json(),
        "plan": plan.routes.iter().map(|r| r.name()).collect::<Vec<_>>(),
        "skipped": plan.skipped.iter().map(|(r, o)| json!({ "route": r, "why": o.code() })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestIn {
    /// `own`, `workers` or `auto` (the venue's own choice when absent).
    #[serde(default)]
    pub provider: Option<String>,
}

/// `POST /api/owner/ai/test` -- the button beside the key. The prompt carries
/// no venue data at all ("answer OK"), so the tap is allowed with the
/// assistant still switched off: an owner tests a key before turning it on.
pub async fn test(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: TestIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let mut cfg = Cfg::of(&s);
    if let Some(p) = body.provider.as_deref() {
        cfg.mode = match Mode::of(p) {
            Some(m) => m,
            None => return Response::error(format!("provider is auto, own or workers, not {p:?}"), 400),
        };
    }
    cfg.enabled = true;
    let prompt = call::Prompt { system: "Answer with the single word OK.".into(), user: "OK?".into(), max_tokens: 8 };
    let out = call::run(&ctx.env, &place, cfg.clone(), call::meter(&ctx.env, &s, ctx.data.now_ms), &prompt, ctx.data.now_ms).await?;
    let state = match (&out.said, out.failed.first()) {
        (Some(_), _) => "ok",
        (None, Some(f)) if f.status == 401 && cfg.key.is_none() => Off::NeedsKey.code(),
        (None, Some(_)) => "failed",
        (None, None) => out.skipped.iter().find(|(_, o)| *o != Off::NotChosen).map_or("off", |(_, o)| o.code()),
    };
    let said: String = out.said.as_ref().map(|x| x.text.chars().take(40).collect()).unwrap_or_default();
    let said = cfg.key.as_ref().map_or(said.clone(), |k| k.scrub(&said));
    let mut v = out.json();
    v["contract"] = json!(CONTRACT);
    v["ok"] = json!(state == "ok");
    v["state"] = json!(state);
    v["said"] = json!(said);
    Response::from_json(&v)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AskIn {
    pub question: String,
    #[serde(default)]
    pub lang: Option<String>,
    /// Let the model reword the template (default: when AI is on).
    #[serde(default)]
    pub reword: Option<bool>,
}

/// The fold a kind reads, asked of the venue's object for `days`.
async fn fold(place: &crate::hubstore::Place, loc: &str, now: i64, f: Fold, days: i64) -> Result<Value> {
    let enc = crate::mcp::enc;
    let url = match f {
        Fold::Analytics => format!("https://hub/fold/analytics?venue={}&days={}&now={now}&v=2", enc(loc), answer::analytics_days(days)),
        Fold::Kitchen => format!("https://hub/fold/kitchen?venue={}&days={days}&now={now}&v=2", enc(loc)),
    };
    crate::fold::ask::json(place, &url).await
}

/// `POST /api/owner/ai/ask`
pub async fn ask(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: AskIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let question = body.question.trim().to_string();
    if question.is_empty() || question.chars().count() > MAX_QUESTION {
        return Response::error(format!("a question is 1 to {MAX_QUESTION} characters"), 400);
    }
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let (now, lang) = (ctx.data.now_ms, lang_or(body.lang.as_deref(), &question));
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let cfg = Cfg::of(&s);
    let menu = crate::services::engagement::voice::menu::load(&place, lang).await.unwrap_or_default();
    let dishes: Vec<query::Dish> = menu.iter().flat_map(|d| d.names.iter().map(move |n| (d.id.clone(), n.clone()))).collect();
    let mut provenance: Vec<Value> = Vec::new();
    let (q, by) = match lexicon::read(&question, &dishes) {
        // NO SCORING OF ANY PARTICIPANT: a question about a person is not
        // handed to a model to be squeezed into a dish question.
        None if lexicon::about_person(&question) => (Err("questions about people are not answered here".to_string()), "lexicon"),
        Some(q) => (Ok(q), "lexicon"),
        None if cfg.enabled => {
            let p = call::Prompt { system: query::picker_prompt(&dishes), user: lexicon::withheld(&question), max_tokens: 80 };
            let out = call::complete(&ctx.env, &place, &s, &p, now).await?;
            provenance.push(out.json());
            let picked = match out.said.as_ref().and_then(|x| query::object_in(&x.text)) {
                Some(v) if v.get("query").and_then(Value::as_str) == Some("none") => Err("the model found no query on the list for it".to_string()),
                Some(v) => query::validate(&v, &dishes),
                None if out.said.is_some() => Err("the model's pick is not a JSON object".to_string()),
                None => Err(format!("AI is off: {}", out.why_off())),
            };
            (picked, "model")
        }
        None => (Err(format!("AI is off: {}", Off::Disabled.code())), "lexicon"),
    };
    let q = match q {
        Ok(q) => q,
        Err(why) => {
            return Response::from_json(&json!({
                "contract": CONTRACT, "understood": false, "lang": lang, "pickedBy": by, "why": why,
                "answer": words::fill(&words::UNKNOWN, lang, &[]), "ai": provenance,
            }))
        }
    };
    let f = answer::fold_of(q.kind);
    let data = fold(&place, &loc, now, f, q.days).await?;
    let a = answer::answer(&q, &data, lang);
    let (text, reworded) = if body.reword.unwrap_or(cfg.enabled) && cfg.enabled {
        let (lines, out) = reword(&ctx.env, &place, std::slice::from_ref(&a.text), now).await?;
        provenance.push(out);
        (lines[0].clone(), lines[0] != a.text)
    } else {
        (a.text.clone(), false)
    };
    let mut v = a.json();
    v["contract"] = json!(CONTRACT);
    v["understood"] = json!(true);
    v["lang"] = json!(lang);
    v["pickedBy"] = json!(by);
    v["query"] = q.json();
    v["template"] = json!(a.text);
    v["answer"] = json!(text);
    v["reworded"] = json!(reworded);
    v["ai"] = json!(provenance);
    Response::from_json(&v)
}

/// The template lines, each replaced by the model's rewording only when the
/// rewording keeps its numbers. One call for all of them.
/// The settings are read again here: a pick just before may have charged the
/// meter, and the admission must see that charge.
async fn reword(env: &Env, place: &crate::hubstore::Place, lines: &[String], now: i64) -> Result<(Vec<String>, Value)> {
    let s = crate::hubstore::load_settings(place).await?.settings;
    let p = call::Prompt { system: SYSTEM_REWORD.into(), user: lines.join("\n"), max_tokens: 60 * lines.len() as i64 + 40 };
    let out = call::complete(env, place, &s, &p, now).await?;
    let got: Vec<&str> = out.said.as_ref().map_or(vec![], |x| x.text.lines().map(str::trim).filter(|l| !l.is_empty()).collect());
    let kept = lines
        .iter()
        .enumerate()
        .map(|(i, t)| match got.get(i) {
            Some(r) if got.len() == lines.len() && words::keeps_numbers(t, r) => r.to_string(),
            _ => t.clone(),
        })
        .collect();
    Ok((kept, out.json()))
}

/// `GET /api/owner/ai/explain?screen=analytics|kitchen&days=7&lang=&reword=1`
pub async fn explain(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let screen = param(&req, "screen").unwrap_or_default();
    let days = param(&req, "days").and_then(|d| d.parse::<i64>().ok()).filter(|d| query::WINDOWS.contains(d)).unwrap_or(7);
    let lang = lang_or(param(&req, "lang").as_deref(), "");
    let now = ctx.data.now_ms;
    let cards = match screen.as_str() {
        "analytics" => explain::analytics(&fold(&place, &loc, now, Fold::Analytics, days).await?, lang, answer::analytics_days(days)),
        "kitchen" => explain::kitchen(&fold(&place, &loc, now, Fold::Kitchen, days).await?, lang, days),
        other => return Response::error(format!("screen is analytics or kitchen, not {other:?}"), 400),
    };
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let mut texts: Vec<String> = cards.iter().map(|c| c.text.clone()).collect();
    let mut ai = Value::Null;
    if param(&req, "reword").as_deref() == Some("1") && Cfg::of(&s).enabled && !texts.is_empty() {
        let (kept, out) = reword(&ctx.env, &place, &texts, now).await?;
        texts = kept;
        ai = out;
    }
    Response::from_json(&json!({
        "contract": CONTRACT, "screen": screen, "days": days, "lang": lang, "ai": ai,
        "cards": cards.iter().zip(&texts).map(|(c, t)| {
            let mut v = c.json();
            v["template"] = json!(c.text);
            v["text"] = json!(t);
            v["reworded"] = json!(t != &c.text);
            v
        }).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "ai/routes/tests.rs"]
mod route_tests;
