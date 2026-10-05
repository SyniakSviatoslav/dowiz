//! A personalised offer (W-SENSE row 7): labelled, the menu's own figure, no code, and only to the
//! guests in the taste segment who gave marketing consent on the campaign's channel.

use super::*;
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

#[test]
fn every_offer_is_labelled_in_its_language_and_carries_the_public_figure() {
    for (lang, word) in [("sq", "Ofertë e personalizuar"), ("en", "Personalised offer"), ("uk", "Персоналізована пропозиція"), ("ru", "Персонализированное предложение")] {
        let t = compose(lang, " Smoked eel ", 900, "ALL");
        assert!(t.starts_with(word), "{lang}: {t}");
        assert!(t.contains("Smoked eel -- 900 ALL"), "{t}");
    }
    assert_eq!(figure(1250, "EUR"), "12.50 EUR");
    assert_eq!(figure(5, "EUR"), "0.05 EUR");
    assert_eq!(figure(900, "ALL"), "900 ALL");
    assert_eq!(figure(900, "XYZ"), "900 XYZ", "an unknown code is printed as stored, never scaled");
}

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn order(site: &Site, dish: &str, phone: &str) -> Value {
    let body = json!({"items": [{"product_id": dish, "quantity": 2}], "contact": {"name": "Guest", "phone": phone},
        "fulfilment": {"kind": "pickup"}, "payment": "cash"});
    let r = site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}
fn key_of(site: &Site, phone: &str) -> String {
    crate::services::customers::handlers::customer_key(&crate::services::customers::handlers::signing_secret(&site.env()), phone)
}
fn consent(site: &Site, t: &str, key: &str) {
    act(site, t, key, json!({"state": "given", "evidence": "said yes at the counter", "lang": "sq"}));
}
fn act(site: &Site, t: &str, key: &str, body: Value) {
    let r = site.run(crate::services::customers::consent_routes::owner_act,
        post(&at(&format!("/api/owner/customers/{key}/consent")), &body).bearer(t).on("alpha"),
        &[("key", key)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}
fn define(site: &Site, t: &str, body: Value) -> crate::wire::Reply {
    site.run(super::super::handlers::define, post(&at("/api/owner/campaigns"), &body).bearer(t).on("alpha"), &[])
}
fn taste_offer(dish: &str, text: &str) -> Value {
    json!({"name": "Smoky lovers", "text": text,
           "segment": {"kind": "taste", "filter": {"key": "a:smoky", "min": 500}, "dish": dish, "lang": "en"},
           "template": {"name": "taste_offer", "lang": "en"}})
}

#[test]
fn a_taste_offer_reaches_only_consented_guests_in_the_segment_at_the_menu_figure() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(crate::owner::update_product,
        post(&at(&format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "sense": {"aroma": {"smoky": 3}}})).bearer(&t).on("alpha"),
        &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let (yes, no) = ("+355690000051", "+355690000052");
    let a = order(&site, &dish, yes);
    let b = order(&site, &dish, no);
    // THE PRICE IS NEVER CHANGED BY A SEGMENT: both guests paid the menu's figure, 2 x 900.
    assert_eq!((a["total"].clone(), b["total"].clone()), (json!(1800), json!(1800)), "{a} {b}");
    consent(&site, &t, &key_of(&site, yes)); // WhatsApp marketing consent for ONE of them
    // A third guest said yes and then took it back: a withdrawn permission is no permission.
    let gone = "+355690000053";
    order(&site, &dish, gone);
    consent(&site, &t, &key_of(&site, gone));
    act(&site, &t, &key_of(&site, gone), json!({"state": "withdrawn", "evidence": "asked to stop at the counter", "lang": "sq"}));
    // The owner's text and number are not the message: the server writes it from the catalogue.
    let r = define(&site, &t, taste_offer(&dish, "Only 100 lek for you!"));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let c = r.body_value()["campaign"].clone();
    let text = c["text"].as_str().unwrap();
    assert!(text.starts_with("Personalised offer: Futomaki -- 900 ALL"), "{text}");
    assert!(!text.contains("100 lek"), "the owner's number never reaches the guest: {text}");
    let id = c["id"].as_str().unwrap().to_string();
    let p = site.run(super::super::handlers::preview, post(&at(&format!("/api/owner/campaigns/{id}/preview")), &json!({})).bearer(&t).on("alpha"), &[("id", &id)]);
    assert_eq!(p.status_code(), 200, "{}", p.body_str());
    assert_eq!(p.body_value()["preview"]["count"], 1, "the guests without marketing consent (never given, withdrawn) are not reached: {}", p.body_str());
    // A segment nobody is in reaches nobody, consent or not.
    // (Edited in place: the test clock is one millisecond, and a campaign id is its instant.)
    let r = define(&site, &t, json!({"id": id, "name": "Sweet", "text": "x", "template": {"name": "taste_offer", "lang": "en"},
        "segment": {"kind": "taste", "filter": {"key": "t:sweet"}, "dish": dish, "lang": "en"}}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let id2 = r.body_value()["campaign"]["id"].as_str().unwrap().to_string();
    let p2 = site.run(super::super::handlers::preview, post(&at(&format!("/api/owner/campaigns/{id2}/preview")), &json!({})).bearer(&t).on("alpha"), &[("id", &id2)]);
    assert_eq!(p2.body_value()["preview"]["count"], 0, "{}", p2.body_str());
}

#[test]
fn a_taste_offer_with_a_code_a_price_an_allergen_or_an_unknown_dish_is_refused() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let mut coded = taste_offer(&dish, "x");
    coded["promo"] = json!("SMOKY10");
    let r = define(&site, &t, coded);
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    assert!(r.body_str().contains("no promo code"), "{}", r.body_str());
    let mut priced = taste_offer(&dish, "x");
    priced["segment"]["price"] = json!(100);
    assert_eq!(define(&site, &t, priced).status_code(), 400, "no price field anywhere in the shape");
    let mut priced2 = taste_offer(&dish, "x");
    priced2["price"] = json!(100);
    assert_eq!(define(&site, &t, priced2).status_code(), 400);
    let mut allergen = taste_offer(&dish, "x");
    allergen["segment"]["filter"]["key"] = json!("allergen:fish");
    assert_eq!(define(&site, &t, allergen).status_code(), 400, "allergens only filter for the guest, never target");
    assert_eq!(define(&site, &t, taste_offer("nope", "x")).status_code(), 400, "an unknown dish");
    let mut lang = taste_offer(&dish, "x");
    lang["segment"]["lang"] = json!("de");
    assert_eq!(define(&site, &t, lang).status_code(), 400);
    // Positive twin.
    assert_eq!(define(&site, &t, taste_offer(&dish, "x")).status_code(), 200);
}
