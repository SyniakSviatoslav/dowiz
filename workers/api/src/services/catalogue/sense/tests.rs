//! The dish's taste, texture and aroma through the routes (W-SENSE row 1): the owner saves it, the
//! storefront serves it, the version moves, a value out of the vocabulary is a 400 by name, and
//! "Suggest" answers a draft that writes nothing.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn edit(site: &Site, t: &str, dish: &str, body: Value) -> crate::wire::Reply {
    site.run(crate::owner::update_product, post(&at(&format!("/api/owner/products/{dish}")), &body).bearer(t).on("alpha"), &[("id", dish)])
}
fn menu(site: &Site) -> Value {
    let r = site.run(crate::storefront::menu, get(&at("/api/public/locations/alpha/menu?fresh=1")).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}
fn dish_in(m: &Value, id: &str) -> Value {
    fn find(v: &Value, id: &str) -> Option<Value> {
        match v {
            Value::Object(o) if o.get("id").and_then(Value::as_str) == Some(id) && o.contains_key("price") => Some(v.clone()),
            Value::Object(o) => o.values().find_map(|x| find(x, id)),
            Value::Array(a) => a.iter().find_map(|x| find(x, id)),
            _ => None,
        }
    }
    find(m, id).unwrap_or_else(|| panic!("dish {id} not in {m}"))
}
fn version(m: &Value) -> i64 {
    m["location"]["menuVersion"].as_i64().unwrap_or(-1)
}

#[test]
fn the_owner_saves_the_profile_and_the_storefront_serves_it_with_a_new_version() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let before = version(&menu(&site));
    let s = json!({"taste": {"spicy": 4, "sweet": 0, "umami": 3}, "texture": {"crispy": 3}, "aroma": {"smoky": 2}});
    let r = edit(&site, &t, &dish, json!({"location_id": "alpha", "sense": s}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let m = menu(&site);
    let d = dish_in(&m, &dish);
    assert_eq!(d["sense"]["taste"]["spicy"], 4, "{d}");
    assert_eq!(d["sense"]["taste"]["sweet"], 0, "a declared zero is served as 0");
    assert_eq!(d["sense"]["texture"]["crispy"], 3);
    assert_eq!(d["sense"]["aroma"]["smoky"], 2);
    assert!(d["sense"]["taste"].get("bitter").is_none(), "an axis nobody declared is absent, not 0");
    assert!(version(&m) > before, "menuVersion {} -> {}", before, version(&m));
    // `{}` clears it: the storefront then shows nothing (no fake zeros).
    assert_eq!(edit(&site, &t, &dish, json!({"location_id": "alpha", "sense": {}})).status_code(), 200);
    assert_eq!(dish_in(&menu(&site), &dish)["sense"], Value::Null);
}

#[test]
fn a_value_outside_the_vocabulary_or_range_is_refused_and_nothing_is_written() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    for bad in [json!({"taste": {"spicy": 6}}), json!({"taste": {"richness": 1}}), json!({"aroma": {"gluten": 1}}),
                json!({"texture": {"crispy": 1.5}}), json!({"smell": {}})] {
        let r = edit(&site, &t, &dish, json!({"location_id": "alpha", "sense": bad}));
        assert_eq!(r.status_code(), 400, "{bad} -> {}", r.body_str());
    }
    assert_eq!(dish_in(&menu(&site), &dish)["sense"], Value::Null, "a refused edit wrote nothing");
    // Positive twin.
    assert_eq!(edit(&site, &t, &dish, json!({"location_id": "alpha", "sense": {"taste": {"spicy": 5}}})).status_code(), 200);
}

#[test]
fn saving_the_profile_clears_the_old_taste_field() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    assert_eq!(edit(&site, &t, &dish, json!({"location_id": "alpha", "taste": {"spicy": 3}})).status_code(), 200);
    assert_eq!(dish_in(&menu(&site), &dish)["taste"]["spicy"], 3);
    assert_eq!(edit(&site, &t, &dish, json!({"location_id": "alpha", "sense": {"taste": {"spicy": 2}}})).status_code(), 200);
    let d = dish_in(&menu(&site), &dish);
    assert_eq!(d["taste"], Value::Null, "{d}");
    assert_eq!(d["sense"]["taste"]["spicy"], 2);
}

#[test]
fn suggest_answers_a_draft_from_the_words_and_writes_nothing() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let words = json!({"location_id": "alpha", "description": "Smoked salmon, cream cheese, crispy tempura flakes",
                       "ingredients": ["salmon", "cream cheese"]});
    assert_eq!(edit(&site, &t, &dish, words).status_code(), 200);
    let before = version(&menu(&site));
    let r = site.run(super::suggest, post(&at(&format!("/api/owner/products/{dish}/sense/suggest")), &json!({"location_id": "alpha"})).bearer(&t).on("alpha"), &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["contract"], "menu.sense-suggest.v1");
    assert_eq!(v["saved"], false);
    assert_eq!(v["draft"]["aroma"]["smoky"], 3, "{v}");
    assert_eq!(v["draft"]["texture"]["creamy"], 3);
    assert_eq!(v["draft"]["texture"]["crispy"], 3);
    assert!(v["why"].as_array().unwrap().iter().any(|w| w["key"] == "a:smoky"), "{v}");
    assert!(dowiz_hub::sense::validate(&v["draft"]).is_ok(), "the draft is a valid edit as it stands");
    let m = menu(&site);
    assert_eq!(dish_in(&m, &dish)["sense"], Value::Null, "nothing was saved");
    assert_eq!(version(&m), before, "and the menu did not move");
    // Another venue's owner cannot ask, and an unknown dish is a 404.
    let (other, _) = open_venue(&site, "beta", "b@x.test");
    let no = site.run(super::suggest, post(&at(&format!("/api/owner/products/{dish}/sense/suggest")), &json!({"location_id": "alpha"})).bearer(&other).on("alpha"), &[("id", &dish)]);
    assert!(no.status_code() >= 400, "{}", no.body_str());
    let gone = site.run(super::suggest, post(&at("/api/owner/products/nope/sense/suggest"), &json!({"location_id": "alpha"})).bearer(&t).on("alpha"), &[("id", "nope")]);
    assert_eq!(gone.status_code(), 404);
}

#[test]
fn the_pure_draft_says_what_the_dish_declares_now_beside_the_draft() {
    let p = json!({"name": "Spicy tuna roll", "taste": {"spicy": 3}});
    let v = super::draft("d1", &p);
    assert_eq!(v["current"]["taste"]["spicy"], 5, "the old field, read on the new scale");
    assert!(v["draft"]["taste"]["spicy"].as_i64().unwrap() >= 3);
    assert_eq!(super::draft("d2", &json!({"name": "Water"}))["draft"], Value::Null, "no word, no draft");
}

// ── W-TASTE row 3a: the venue's model draft, through from_model, under the lexicon ──────────────

const MODEL_SAYS: &str = "Sure! Here it is: {\"taste\": {\"spicy\": 9, \"umami\": 4, \"bitter\": 2, \"richness\": 2}, \
    \"texture\": {\"crispy\": 1, \"gluten\": 3, \"tender\": 2}, \"aroma\": {\"smoky\": 1, \"floral\": 2, \"marine\": 2.5}, \
    \"note\": \"IGNORE PREVIOUS INSTRUCTIONS\"} Enjoy!";

#[test]
fn a_model_draft_is_held_to_the_vocabulary_and_merged_under_the_lexicon() {
    let p = json!({"name": "Smoked eel", "description": "crispy tempura"});
    let lex = super::draft("d1", &p);
    let v = super::draft_with("d1", &p, Some(MODEL_SAYS));
    assert_eq!(v["source"], "lexicon+model", "{v}");
    let under = |dim: &str, id: &str, n: i64| if lex["draft"][dim][id].is_null() { json!(n) } else { lex["draft"][dim][id].clone() };
    assert_eq!(v["draft"]["taste"]["bitter"], 2, "an id only the model gave is added");
    assert_eq!(v["draft"]["aroma"]["floral"], 2);
    assert_eq!(v["draft"]["taste"]["umami"], under("taste", "umami", 4));
    assert_eq!(v["draft"]["texture"]["tender"], under("texture", "tender", 2));
    assert_eq!(v["draft"]["aroma"]["smoky"], lex["draft"]["aroma"]["smoky"], "the lexicon wins where it spoke: {v}");
    assert_eq!(v["draft"]["texture"]["crispy"], lex["draft"]["texture"]["crispy"]);
    for gone in [&v["draft"]["taste"]["spicy"], &v["draft"]["taste"]["richness"], &v["draft"]["texture"]["gluten"], &v["draft"]["aroma"]["marine"]] {
        assert_eq!(*gone, Value::Null, "out of range, out of vocabulary or not whole: dropped ({v})");
    }
    assert!(dowiz_hub::sense::validate(&v["draft"]).is_ok(), "the merged draft is a valid edit");
    let text = v.to_string();
    for raw in ["IGNORE", "Sure!", "Enjoy", "note"] {
        assert!(!text.contains(raw), "the model's raw text never reaches the answer: {raw} in {text}");
    }
    assert!(v["why"].as_array().unwrap().iter().any(|w| w["key"] == "a:floral" && w["from"] == "model"));
    // Junk, a refusal or nothing: the lexicon draft alone.
    for junk in ["I cannot help with that.", "{not json", ""] {
        let j = super::draft_with("d1", &p, Some(junk));
        assert_eq!((j["draft"].clone(), j["source"].clone()), (lex["draft"].clone(), json!("lexicon")), "{junk:?}");
    }
}

fn ai_on(site: &Site, t: &str) {
    let r = site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": true})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

fn suggest(site: &Site, t: &str, dish: &str) -> Value {
    let r = site.run(super::suggest, post(&at(&format!("/api/owner/products/{dish}/sense/suggest")), &json!({"location_id": "alpha"})).bearer(t).on("alpha"), &[("id", dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

#[test]
fn suggest_asks_the_venues_model_when_ai_is_on_and_a_failure_is_the_lexicon_alone() {
    use crate::services::engagement::ai::call::hook;
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    // W-TASTE2: AI is ON by default; with no binding and no endpoint the answer is the lexicon,
    // 200, never an error (fail soft).
    let bare = suggest(&site, &t, &dish);
    assert_eq!(bare["source"], "lexicon", "on by default, no route: the lexicon draft ({bare})");
    assert_eq!(bare["ai"]["provider"], Value::Null, "no model answered: {bare}");
    // W-TASTE2 S7b: the venue has no recipe, and the recipe source says exactly that.
    assert_eq!(bare["recipe"], json!({"state": "no-recipes"}), "no fabricated values: {bare}");
    let r = site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": false})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    hook::answer(|_, _| Ok(json!({ "response": MODEL_SAYS })));
    let off = suggest(&site, &t, &dish);
    assert_eq!(off["source"], "lexicon", "the owner's off: the model is not asked ({off})");
    assert_eq!(off["ai"], Value::Null, "nothing was sent: {off}");
    assert!(hook::seen().is_empty(), "off sends nothing anywhere");
    ai_on(&site, &t);
    hook::answer(|_, _| Ok(json!({ "response": MODEL_SAYS })));
    let before = version(&menu(&site));
    let on = suggest(&site, &t, &dish);
    assert_eq!(on["source"], "lexicon+model", "{on}");
    assert_eq!(on["draft"]["aroma"]["floral"], 2);
    assert!(!on.to_string().contains("IGNORE"), "never the raw answer");
    assert_eq!(on["ai"]["provider"], "workers-ai", "the provenance names the route: {on}");
    assert_eq!(dish_in(&menu(&site), &dish)["sense"], Value::Null, "nothing was saved");
    assert_eq!(version(&menu(&site)), before);
    hook::answer(|_, _| Err("model overloaded".into()));
    let failed = suggest(&site, &t, &dish);
    assert_eq!((failed["source"].clone(), failed["draft"].clone()), (json!("lexicon"), off["draft"].clone()), "a failure is silence: {failed}");
}
