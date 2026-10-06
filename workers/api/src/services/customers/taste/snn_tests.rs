//! W-SNN row 3 and row 5, through the routes: the shadow never changes the guest's page, `off` runs
//! and counts nothing, an objection is never counted, the owner sees the per-venue agreement and
//! sets the switch (a wrong value is a 400 by name), and `/api/owner/health` carries it.

use super::*;
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use dowiz_hub::snn::shadow::Mode;
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn products() -> Vec<(String, String)> {
    let rows = [("maki", r#"{"taste":{"umami":4},"texture":{"crispy":2},"aroma":{}}"#, "rolls"), ("ramen", r#"{"taste":{"salty":4,"umami":5},"texture":{"tender":3},"aroma":{"smoky":2}}"#, "soups"),
        ("mochi", r#"{"taste":{"sweet":5},"texture":{"chewy":3},"aroma":{}}"#, "sweets"), ("tempura", r#"{"taste":{"salty":2},"texture":{"crispy":3},"aroma":{"toasty":2}}"#, "rolls")];
    rows.iter().map(|(id, s, c)| (id.to_string(), format!(r#"{{"id":"{id}","categoryId":"{c}","allergens":[],"sense":{{"v":1,{}}}}}"#, &s[1..s.len() - 1]))).collect()
}

fn profile() -> Profile {
    let mut p = Profile { v: 1, orders: 2, ..Profile::default() };
    p.sense = [("t:umami", 1600), ("x:crispy", 666), ("t:salty", 400)].iter().map(|(k, w)| (k.to_string(), *w)).collect();
    p.cats.insert("rolls".into(), 2000);
    p
}

#[test]
fn compare_shows_the_current_list_in_shadow_and_off_and_the_networks_only_when_on() {
    let (prods, p) = (products(), profile());
    let current = dowiz_hub::block::taste::top_k_json(&prods, &p.sense, 0, K);
    assert!(!current.is_empty());
    let (shown, o) = compare(Mode::Shadow, &prods, &p, K);
    assert_eq!(shown, current, "shadow shows the current ranker");
    assert!(matches!(o, Outcome::Compared { .. }), "{o:?}");
    assert_eq!(compare(Mode::Off, &prods, &p, K), (current.clone(), Outcome::Off));
    let (on, _) = compare(Mode::On, &prods, &p, K);
    let model = dowiz_hub::snn::shipped().unwrap();
    let menu = dowiz_hub::snn::Menu::from_products(&prods, 0);
    let g = dowiz_hub::snn::Guest::from_maps(&menu, &p.sense, &p.cats, &p.tags, &Default::default());
    assert_eq!(on, dowiz_hub::snn::rank(&model, &menu, &g, K, &[]), "on shows the network's list");
}

fn place(site: &Site, dish: &str, phone: &str) -> (String, String) {
    let body = json!({"items": [{"product_id": dish, "quantity": 2}], "contact": {"name": "Guest", "phone": phone},
        "fulfilment": {"kind": "pickup"}, "payment": "cash"});
    let r = site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    (r.body_value()["id"].as_str().unwrap().to_string(), r.body_value()["access_token"].as_str().unwrap().to_string())
}

fn guest_page(site: &Site, id: &str, token: &str) -> String {
    let r = site.run(super::super::super::taste_routes::guest_view, get(&at(&format!("/api/order/{id}/taste"))).bearer(token).on("alpha"), &[("id", id)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_str().to_string()
}

fn snn_view(site: &Site, owner: &str) -> Value {
    let r = site.run(super::super::super::snn_routes::owner_view, get(&at("/api/owner/snn?location_id=alpha")).bearer(owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn set_mode(site: &Site, owner: &str, mode: &str) -> crate::wire::Reply {
    site.run(super::super::super::snn_routes::owner_set, post(&at("/api/owner/snn"), &json!({"mode": mode})).bearer(owner).on("alpha"), &[])
}

#[test]
fn the_guest_page_is_the_same_bytes_with_the_shadow_on_or_off_and_only_the_venue_counts() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (id, token) = place(&site, &dish, "+355690000071");
    assert_eq!(snn_view(&site, &owner)["mode"], "shadow", "the default");
    let shadow_page = guest_page(&site, &id, &token);
    let v = snn_view(&site, &owner);
    assert_eq!((v["compared"].clone(), v["unusable"].clone(), v["model"].clone()), (json!(1), json!(0), json!(20261006)), "{v}");
    assert_eq!(set_mode(&site, &owner, "off").status_code(), 200);
    let off_page = guest_page(&site, &id, &token);
    // The one difference is the export listing what the shadow read HELD (Art. 15): remove it and
    // the bytes are the same, shadow or off.
    let mut off_v: Value = serde_json::from_str(&off_page).unwrap();
    let held = off_v["taste"].as_object_mut().unwrap().remove("held").expect("the export lists the held top-3s");
    assert_eq!((held["temporary"].clone(), held["deletedAt"].clone()), (json!(true), json!("next_order")), "{held}");
    assert!(held["network"].as_array().unwrap().iter().all(|r| r["name"] == "Futomaki"), "dishes by name: {held}");
    assert_eq!(off_v.to_string(), serde_json::from_str::<Value>(&shadow_page).unwrap().to_string(), "the same page, shadow or off");
    assert_eq!(snn_view(&site, &owner)["compared"], 1, "off ran nothing and counted nothing");
    assert_eq!(snn_view(&site, &owner)["mode"], "off");
    let bad = set_mode(&site, &owner, "maybe");
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    assert!(bad.body_str().contains("not one of shadow, on, off"), "{}", bad.body_str());
    assert_eq!(set_mode(&site, &owner, "shadow").status_code(), 200);
    // The guest objects: the page is read again, nothing is counted for them.
    let w = site.run(super::super::super::taste_routes::guest_withdraw, post(&at(&format!("/api/order/{id}/taste/withdraw")), &json!({})).bearer(&token).on("alpha"), &[("id", &id)]);
    assert_eq!(w.status_code(), 200, "{}", w.body_str());
    let _ = guest_page(&site, &id, &token);
    assert_eq!(snn_view(&site, &owner)["compared"], 1, "an objection is never counted");
    let h = site.run(crate::services::operations::health, get(&at("/api/owner/health")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(h.status_code(), 200, "{}", h.body_str());
    assert_eq!((h.body_value()["snn"]["mode"].clone(), h.body_value()["snn"]["compared"].clone()), (json!("shadow"), json!(1)));
}

#[test]
fn only_an_owner_reads_or_sets_the_switch() {
    let site = Site::new();
    let _ = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(super::super::super::snn_routes::owner_view, get(&at("/api/owner/snn?location_id=alpha")).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());
    let r = site.run(super::super::super::snn_routes::owner_set, post(&at("/api/owner/snn"), &json!({"mode": "on"})).on("alpha"), &[]);
    assert!(r.status_code() == 401 || r.status_code() == 400, "{} {}", r.status_code(), r.body_str());
}

fn owner_taste(site: &Site, owner: &str, phone: &str) -> Value {
    let key = super::super::super::handlers::customer_key(&super::super::super::handlers::signing_secret(&site.env()), phone);
    let r = site.run(super::super::super::taste_routes::owner_view, get(&at(&format!("/api/owner/customers/{key}/taste?location_id=alpha"))).bearer(owner).on("alpha"), &[("key", &key)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()["taste"].clone()
}

#[test]
fn the_next_order_is_checked_once_against_both_held_top3s_and_an_objection_stores_nothing() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let phone = "+355690000072";
    let (id, token) = place(&site, &dish, phone);
    let _ = guest_page(&site, &id, &token); // holds both top-3s beside the profile
    assert_eq!(snn_view(&site, &owner)["settled"], 0, "nothing settled before the next order");
    let _ = place(&site, &dish, phone);
    let v = snn_view(&site, &owner);
    assert_eq!(v["settled"], 1, "the next order checked the hold: {v}");
    assert_eq!(v["currentHitPm"], 0, "the current ranker had nothing (the dish declares no sense): {v}");
    let _ = place(&site, &dish, phone);
    assert_eq!(snn_view(&site, &owner)["settled"], 1, "the hold was cleared: a second order checks nothing");
    // The objection: the profile goes, and reading the page again stores nothing at all.
    let w = site.run(super::super::super::taste_routes::guest_withdraw, post(&at(&format!("/api/order/{id}/taste/withdraw")), &json!({})).bearer(&token).on("alpha"), &[("id", &id)]);
    assert_eq!(w.status_code(), 200, "{}", w.body_str());
    let _ = guest_page(&site, &id, &token);
    assert_eq!(owner_taste(&site, &owner, phone), Value::Null, "an objecting guest has no profile and no hold");
    let _ = place(&site, &dish, phone);
    assert_eq!(owner_taste(&site, &owner, phone), Value::Null);
    assert_eq!(snn_view(&site, &owner)["settled"], 1, "and is never counted");
}

#[test]
fn holding_never_creates_a_profile_and_a_same_hold_is_not_rewritten() {
    let h0 = dowiz_hub::snn::quality::Held::of(&[("a".into(), 9)], &[("b".into(), 9)], K, 1, 5);
    assert_eq!(held_record(None, h0.clone()), None, "no profile stored (an objection deleted it): nothing is written");
    let kept = held_record(Some(profile()), h0.clone()).expect("beside a profile, the hold is written");
    assert_eq!(kept.snn_held.as_ref(), Some(&h0));
    assert_eq!(held_record(Some(kept), h0), None, "the same hold is not written twice");
    let mut p = profile();
    let h = dowiz_hub::snn::quality::Held::of(&[("a".into(), 9)], &[("b".into(), 9)], K, 1, 5);
    assert!(hold_in(&mut p, h.clone()));
    assert!(!hold_in(&mut p, h), "the same hold is not written twice");
    let lines = vec![super::super::line_of(r#"{"id":"b","categoryId":"rolls"}"#, 1)];
    assert_eq!(settle_profile(&mut p, &lines), Some((false, true, 1)));
    assert_eq!(p.snn_held, None);
}

#[test]
fn the_export_lists_the_held_top3_by_name_as_a_temporary_check() {
    let h = dowiz_hub::snn::quality::Held::of(&[("a".into(), 9)], &[("b".into(), 9), ("z".into(), 1)], K, 1, 5);
    let names: std::collections::HashMap<String, Value> = [("a".to_string(), json!({"name": "Maki"})), ("b".to_string(), json!({"name": {"sq": "Supë", "en": "Soup"}}))].into_iter().collect();
    let v = held_view(&h, &names);
    assert_eq!(v["current"], json!([{"id": "a", "name": "Maki"}]));
    assert_eq!(v["network"][0]["id"], "b");
    assert!(v["network"][0]["name"].as_str().is_some_and(|n| !n.is_empty()), "a translated name gives one of its texts: {v}");
    assert_eq!(v["network"][1], json!({"id": "z", "name": ""}), "an unknown id is listed without a name, never dropped");
    assert_eq!((v["temporary"].clone(), v["deletedAt"].clone(), v["day"].clone()), (json!(true), json!("next_order"), json!(5)));
}
