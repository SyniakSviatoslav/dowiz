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
