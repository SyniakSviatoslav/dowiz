//! The owner's AI through its routes (W-COV C2 seam): the real handlers, the
//! real venue object in memory, the owner's endpoint as the thread's outbound
//! hook and Workers AI as `call::hook`. Tested in-memory, not live: the live
//! proof is `tools/live-proof/probes/feature-ai.mjs`.

use super::call::hook;
use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Reply;
use serde_json::{json, Value};

const KEY: &str = "sk-or-v1-TOPSECRET-0123456789";

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn setting(site: &Site, t: &str, key: &str, value: &str) -> Reply {
    site.run(crate::services::venue::settings::set_setting, post(&at("/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"), &[])
}

fn ai_on(site: &Site, t: &str) {
    let r = site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": true})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

/// AI is ON by default since 2026-10-06; the tests about "AI off" switch it off as an owner would.
fn ai_off(site: &Site, t: &str) {
    let r = site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": false})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

/// A venue with one pickup order of two Futomaki (1800).
fn venue(site: &Site) -> String {
    let (t, dish) = open_venue(site, "alpha", "a@x.test");
    let r = place_pickup(site, "alpha", &dish, 2);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    t
}

fn ask(site: &Site, t: &str, body: Value) -> Reply {
    site.run(super::ask, post(&at("/api/owner/ai/ask"), &body).bearer(t).on("alpha"), &[])
}

fn status(site: &Site, t: &str) -> Value {
    let r = site.run(super::status, get(&at("/api/owner/ai")).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn analytics(site: &Site, t: &str, days: i64) -> Value {
    let r = site.run(crate::services::analytics::analytics, get(&at(&format!("/api/owner/analytics?days={days}&v=2"))).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

/// Each number of an answer equals the sum of the cells it names in the
/// public route's own answer -- the `v=2` answer the cards name as their source
/// (answer.rs) and the analytics screen draws since W-HIST.
fn equals_the_route(answer: &Value, route: &Value) {
    let nums = answer["numbers"].as_array().expect("numbers");
    assert!(!nums.is_empty(), "{answer}");
    for n in nums {
        let sum: i64 = n["pointers"].as_array().unwrap().iter().map(|p| route.pointer(p.as_str().unwrap()).and_then(Value::as_i64).expect("a cell")).sum();
        assert_eq!(sum, n["value"].as_i64().unwrap(), "{n} against {route}");
    }
}

#[test]
fn a_question_is_answered_from_the_fold_with_no_model_and_ai_off() {
    let site = Site::new();
    let t = venue(&site);
    ai_off(&site, &t);
    let r = ask(&site, &t, json!({"question": "скільки виручки за тиждень?"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!((v["understood"].as_bool(), v["pickedBy"].as_str(), v["lang"].as_str()), (Some(true), Some("lexicon"), Some("uk")), "{v}");
    assert_eq!(v["query"]["query"], "revenue");
    assert_eq!(v["source"], "/api/owner/analytics?days=7&v=2");
    equals_the_route(&v, &analytics(&site, &t, 7));
    assert_eq!(v["numbers"][0]["value"], 1800);
    assert!(v["answer"].as_str().unwrap().contains("1800"), "{v}");
    assert_eq!(v["reworded"], false);
    assert!(sent().is_empty() && hook::seen().is_empty(), "no model was asked");
    // The best dish, from the kitchen's fold.
    let r = ask(&site, &t, json!({"question": "What sold best this week?", "lang": "en"}));
    let v = r.body_value();
    assert_eq!(v["query"]["query"], "best_dish", "{v}");
    assert!(v["answer"].as_str().unwrap().contains("Futomaki, 2 portions"), "{v}");
    let k = site.run(crate::services::analytics::kitchen::kitchen, get(&at("/api/owner/analytics/kitchen?days=7")).bearer(&t).on("alpha"), &[]);
    equals_the_route(&v, &k.body_value());
}

#[test]
fn an_unknown_question_with_ai_off_lists_what_can_be_asked_and_sends_nothing() {
    let site = Site::new();
    let t = venue(&site);
    ai_off(&site, &t);
    let v = ask(&site, &t, json!({"question": "tell me a joke"})).body_value();
    assert_eq!(v["understood"], false, "{v}");
    assert!(v["why"].as_str().unwrap().contains("disabled"), "{v}");
    assert!(v["answer"].as_str().unwrap().contains("revenue"), "{v}");
    assert!(sent().is_empty() && hook::seen().is_empty());
    // About a person: refused even with AI on, before any model.
    ai_on(&site, &t);
    hook::answer(|_, _| Ok(json!({"response": "{\"query\": \"best_dish\"}"})));
    let v = ask(&site, &t, json!({"question": "who is the best courier?"})).body_value();
    assert_eq!(v["understood"], false, "{v}");
    assert!(hook::seen().is_empty(), "no model saw a question about a person");
}

#[test]
fn a_long_tail_question_goes_to_workers_ai_which_only_picks_and_the_budget_moves() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    hook::answer(|_, input| {
        let system = input["messages"][0]["content"].as_str().unwrap_or("");
        if system.contains("closed list") {
            Ok(json!({"response": "<think>hm</think>{\"query\": \"orders\", \"days\": 7}", "usage": {"prompt_tokens": 400, "completion_tokens": 20}}))
        } else {
            // A rewording that keeps the numbers.
            Ok(json!({"response": "In the last 7 days you had 1 orders.", "usage": {"prompt_tokens": 100, "completion_tokens": 12}}))
        }
    });
    let before = status(&site, &t)["budget"]["used"].as_i64().unwrap();
    let v = ask(&site, &t, json!({"question": "how busy were we lately?", "lang": "en"})).body_value();
    assert_eq!((v["understood"].as_bool(), v["pickedBy"].as_str()), (Some(true), Some("model")), "{v}");
    assert_eq!(v["query"]["query"], "orders");
    equals_the_route(&v, &analytics(&site, &t, 7));
    assert_eq!(v["answer"], "In the last 7 days you had 1 orders.");
    assert_eq!(v["reworded"], true);
    assert_eq!(v["ai"][0]["provider"], "workers-ai");
    let seen = hook::seen();
    assert_eq!(seen.len(), 2, "one pick, one rewording");
    assert_eq!(seen[0].0, super::provider::WORKERS_MODEL);
    let after = status(&site, &t)["budget"]["used"].as_i64().unwrap();
    assert_eq!(after - before, super::budget::neurons(400, 20) + super::budget::neurons(100, 12), "charged what usage said");
}

#[test]
fn a_rewording_that_changes_a_number_is_not_shown() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    hook::answer(|_, _| Ok(json!({"response": "You made about 2000 ALL this week."})));
    let v = ask(&site, &t, json!({"question": "revenue this week", "lang": "en"})).body_value();
    assert_eq!(v["reworded"], false, "{v}");
    assert_eq!(v["answer"], v["template"]);
    assert!(v["answer"].as_str().unwrap().contains("1800 ALL"), "{v}");
}

#[test]
fn the_picker_refuses_what_the_model_invents() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    hook::answer(|_, _| Ok(json!({"response": "{\"query\": \"popularity_contest\", \"days\": 7}"})));
    let v = ask(&site, &t, json!({"question": "anything unusual?"})).body_value();
    assert_eq!(v["understood"], false, "{v}");
    assert!(v["why"].as_str().unwrap().contains("not on the list"), "{v}");
    assert!(v.get("numbers").is_none(), "no number without a query: {v}");
}

#[test]
fn the_owners_endpoint_fails_and_workers_ai_answers_then_the_share_runs_out() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    assert_eq!(setting(&site, &t, "ai.endpoint", "https://openrouter.ai/api/v1").status_code(), 200);
    assert_eq!(setting(&site, &t, "ai.token", KEY).status_code(), 200);
    answer_outbound(|_| Ok(Reply::error("overloaded", 503).unwrap()));
    // One answer that reports more than the whole share (300 by default).
    hook::answer(|_, _| Ok(json!({"response": "Futomaki.", "usage": {"prompt_tokens": 100_000, "completion_tokens": 0}})));
    let r = site.run(crate::services::engagement::assist::owner_assist, post(&at("/api/owner/assist"), &json!({"question": "what sold?"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.headers().get("x-ai-provider").ok().flatten().as_deref(), Some("workers-ai"));
    assert_eq!(r.body_value()["answer"], "Futomaki.");
    // Charged what `usage` said; the share is spent and the next call is refused.
    let s = status(&site, &t);
    assert_eq!(s["budget"]["used"], super::budget::neurons(100_000, 0));
    assert_eq!(s["budget"]["left"], 0);
    let r = site.run(crate::services::engagement::assist::owner_assist, post(&at("/api/owner/assist"), &json!({"question": "what sold?"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 502, "the owner's endpoint failed and Workers AI is over the share: {}", r.body_str());
    assert_eq!(hook::seen().len(), 1, "the spent share sent nothing to Workers AI");
    let s = status(&site, &t);
    assert!(s["skipped"].to_string().contains("budget-spent"), "{s}");
}

#[test]
fn without_a_binding_workers_ai_is_unavailable_and_said_so() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    let s = status(&site, &t);
    assert_eq!(s["workersAi"]["available"], false);
    assert_eq!(s["skipped"], json!([{"route": "own", "why": "needs-key"}, {"route": "workers-ai", "why": "workers-ai-unavailable"}]));
    let r = site.run(crate::services::engagement::assist::owner_assist, post(&at("/api/owner/assist"), &json!({"question": "what sold?"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("workers-ai-unavailable"), "{}", r.body_str());
}

#[test]
fn the_test_button_says_needs_key_never_ok_without_one() {
    let site = Site::new();
    let t = venue(&site);
    let test = |p: &str| site.run(super::test, post(&at("/api/owner/ai/test"), &json!({"provider": p})).bearer(&t).on("alpha"), &[]).body_value();
    assert_eq!(test("own")["state"], "needs-key");
    assert_eq!(test("workers")["state"], "workers-ai-unavailable");
    // OpenRouter's address with no key: its 401 reads as needs-key.
    assert_eq!(setting(&site, &t, "ai.endpoint", "https://openrouter.ai/api/v1").status_code(), 200);
    answer_outbound(|_| Ok(Reply::error("{\"error\":{\"message\":\"No auth credentials found\",\"code\":401}}", 401).unwrap()));
    let v = test("own");
    assert_eq!((v["ok"].as_bool(), v["state"].as_str()), (Some(false), Some("needs-key")), "{v}");
    answer_outbound(|_| Reply::from_json(&json!({"choices": [{"message": {"content": "OK"}}]})));
    let v = test("own");
    assert_eq!((v["ok"].as_bool(), v["said"].as_str(), v["provider"].as_str()), (Some(true), Some("OK"), Some("own")), "{v}");
    let r = site.run(super::test, post(&at("/api/owner/ai/test"), &json!({"provider": "everything"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400);
    let r = site.run(super::test, post(&at("/api/owner/ai/test"), &json!({"provider": "own", "key": "x"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400, "an unknown field is refused: {}", r.body_str());
}

#[test]
fn the_key_never_appears_in_any_response_body() {
    let site = Site::new();
    let t = venue(&site);
    ai_on(&site, &t);
    assert_eq!(setting(&site, &t, "ai.endpoint", "https://openrouter.ai/api/v1").status_code(), 200);
    assert_eq!(setting(&site, &t, "ai.token", KEY).status_code(), 200);
    // A provider that echoes the authorization header back in its error.
    answer_outbound(|c| {
        let auth = c.headers().get("authorization").ok().flatten().unwrap_or_default();
        Ok(Reply::error(format!("invalid key: {auth}"), 401).unwrap())
    });
    let bodies = vec![
        site.run(super::status, get(&at("/api/owner/ai")).bearer(&t).on("alpha"), &[]),
        site.run(super::test, post(&at("/api/owner/ai/test"), &json!({})).bearer(&t).on("alpha"), &[]),
        ask(&site, &t, json!({"question": "anything unusual?"})),
        ask(&site, &t, json!({"question": "revenue this week"})),
        site.run(super::explain, get(&at("/api/owner/ai/explain?screen=analytics&reword=1")).bearer(&t).on("alpha"), &[]),
        site.run(super::explain, get(&at("/api/owner/ai/explain?screen=kitchen&reword=1")).bearer(&t).on("alpha"), &[]),
        site.run(crate::services::engagement::assist::owner_assist, post(&at("/api/owner/assist"), &json!({"question": "what sold?"})).bearer(&t).on("alpha"), &[]),
        site.run(crate::services::venue::settings::settings, get(&at("/api/owner/settings")).bearer(&t).on("alpha"), &[]),
    ];
    let mut called = 0;
    for r in &bodies {
        assert!(!r.body_str().contains("TOPSECRET"), "the key leaked ({}): {}", r.status_code(), r.body_str());
        called += 1;
    }
    assert_eq!(called, 8);
    // It WAS sent, to the endpoint, and only there.
    assert!(sent().iter().any(|c| c.headers().get("authorization").ok().flatten().is_some_and(|a| a.contains("TOPSECRET"))));
    assert_eq!(status(&site, &t)["keySet"], true);
}

#[test]
fn the_explain_cards_carry_the_screens_numbers() {
    let site = Site::new();
    let t = venue(&site);
    let r = site.run(super::explain, get(&at("/api/owner/ai/explain?screen=analytics&days=7&lang=sq")).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let a = analytics(&site, &t, 7);
    for c in v["cards"].as_array().unwrap() {
        equals_the_route(c, &a);
        assert_eq!(c["reworded"], false);
    }
    assert_eq!(v["cards"][1]["kind"], "hours", "{v}");
    let r = site.run(super::explain, get(&at("/api/owner/ai/explain?screen=orders")).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400);
    // The kitchen's staff do not read the owner's explanations.
    let cook = site.staff("alpha", &t, "k@x.test", "kitchen");
    let r = site.run(super::explain, get(&at("/api/owner/ai/explain?screen=kitchen")).bearer(&cook).on("alpha"), &[]);
    assert!(r.status_code() == 401 || r.status_code() == 403, "{} {}", r.status_code(), r.body_str());
}

#[test]
fn the_provider_setting_is_a_closed_word() {
    let site = Site::new();
    let t = venue(&site);
    assert_eq!(setting(&site, &t, "ai.provider", "openai").status_code(), 400);
    assert_eq!(setting(&site, &t, "ai.provider", "workers").status_code(), 200);
    assert_eq!(status(&site, &t)["mode"], "workers");
    // The meter is the hub's: the owner cannot reset it through settings.
    assert_eq!(setting(&site, &t, "ai.spent", "0:0").status_code(), 400);
}
