//! The guest's taste through the routes (W-MR0 MR8, operator ruling 2026-10-04: automatic, objection
//! not consent): an order with a phone keeps a profile; no phone, nothing; the guest's own view; the
//! objection (the order page's tap, or `taste_off` from the phone) deletes it and stops the next order;
//! a vector after an objection is a 400; a malformed vector is a 400.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn place(site: &Site, dish: &str, phone: &str, extra: Value) -> crate::wire::Reply {
    let mut body = json!({"items": [{"product_id": dish, "quantity": 2}], "contact": {"name": "Guest", "phone": phone},
        "fulfilment": {"kind": "pickup"}, "payment": "cash"});
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        body[k] = v;
    }
    site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")])
}
fn key_of(site: &Site, phone: &str) -> String {
    super::super::handlers::customer_key(&super::super::handlers::signing_secret(&site.env()), phone)
}
fn owner_view(site: &Site, owner: &str, key: &str) -> Value {
    let r = site.run(super::owner_view, get(&at(&format!("/api/owner/customers/{key}/taste?location_id=alpha"))).bearer(owner).on("alpha"), &[("key", key)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}
fn sync() -> Value {
    json!({"v": 1, "tags": {"salmon": 1000}, "cats": {"rolls": 400}})
}
#[test]
fn an_order_with_a_phone_keeps_a_profile_without_any_box_and_without_a_phone_nothing() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let phone = "+355690000032";
    let r = place(&site, &dish, phone, json!({"taste_sync": sync()}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = owner_view(&site, &owner, &key_of(&site, phone));
    assert_eq!(v["taste"]["orders"], 1, "{v}");
    assert_eq!(v["taste"]["portions"], 2);
    assert_eq!(v["taste"]["segment"], "new");
    assert_eq!(v["taste"]["device"]["tags"]["salmon"], 1000);
    assert_eq!(place(&site, &dish, phone, json!({})).status_code(), 200, "the vector is optional");
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"]["orders"], 2);
    let seg = site.run(super::owner_segments, get(&at("/api/owner/customers/taste/segments?location_id=alpha")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(seg.body_value()["segments"]["regular"], 1, "{}", seg.body_str());
    // No phone: an order needs none, and there is nobody to keep a taste under.
    assert_eq!(place(&site, &dish, "", json!({"taste_sync": sync()})).status_code(), 200);
    // A malformed vector is refused by name, phone or not.
    let bad = place(&site, &dish, phone, json!({"taste_sync": {"v": 1, "tags": {"salmon": 5000}}}));
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    let extra = place(&site, &dish, phone, json!({"taste_sync": {"v": 1, "events": []}}));
    assert_eq!(extra.status_code(), 400, "the closed shape: {}", extra.body_str());
    // The checkbox field of the withdrawn design is not a field any more.
    assert_eq!(place(&site, &dish, phone, json!({"personalisation": {"wording": "x"}})).status_code(), 400);
}

#[test]
fn the_guest_sees_it_and_one_tap_objects_deletes_and_stops_the_next_order() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let phone = "+355690000033";
    let r = place(&site, &dish, phone, json!({"taste_sync": sync()}));
    let (id, token) = (r.body_value()["id"].as_str().unwrap().to_string(), r.body_value()["access_token"].as_str().unwrap().to_string());
    let g = site.run(super::guest_view, get(&at(&format!("/api/order/{id}/taste"))).bearer(&token).on("alpha"), &[("id", &id)]);
    assert_eq!(g.status_code(), 200, "{}", g.body_str());
    assert_eq!((g.body_value()["objected"].clone(), g.body_value()["taste"]["orders"].clone()), (json!(false), json!(1)));
    let other = site.run(super::guest_view, get(&at(&format!("/api/order/{id}/taste"))).on("alpha"), &[("id", &id)]);
    assert_eq!(other.status_code(), 401, "no link, no view");
    let w = site.run(super::guest_withdraw, post(&at(&format!("/api/order/{id}/taste/withdraw")), &json!({})).bearer(&token).on("alpha"), &[("id", &id)]);
    assert_eq!(w.status_code(), 200, "{}", w.body_str());
    assert_eq!((w.body_value()["objected"].clone(), w.body_value()["deleted"].clone()), (json!(true), json!(true)));
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"], Value::Null);
    assert_eq!(place(&site, &dish, phone, json!({})).status_code(), 200);
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"], Value::Null, "after the objection an order keeps nothing");
    let again = place(&site, &dish, phone, json!({"taste_sync": sync()}));
    assert_eq!(again.status_code(), 400, "and the vector is refused: {}", again.body_str());
    assert!(again.body_str().contains("turned personalisation off"), "{}", again.body_str());
}

#[test]
fn taste_off_from_the_phone_files_the_objection_and_deletes_with_no_order_link() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let phone = "+355690000034";
    assert_eq!(place(&site, &dish, phone, json!({"taste_sync": sync()})).status_code(), 200);
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"]["orders"], 1);
    let r = place(&site, &dish, phone, json!({"taste_off": true, "taste_sync": sync()}));
    assert_eq!(r.status_code(), 200, "the objection wins over a vector in the same body: {}", r.body_str());
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"], Value::Null, "deleted");
    assert_eq!(place(&site, &dish, phone, json!({})).status_code(), 200);
    assert_eq!(owner_view(&site, &owner, &key_of(&site, phone))["taste"], Value::Null, "and stopped");
    // Somebody else is untouched.
    assert_eq!(place(&site, &dish, "+355690000035", json!({})).status_code(), 200);
    assert_eq!(owner_view(&site, &owner, &key_of(&site, "+355690000035"))["taste"]["orders"], 1);
}

/// PURE: the judge of the placement's fields.
#[test]
fn the_judge_drops_what_has_no_phone_and_lets_the_objection_win() {
    let v: crate::services::customers::taste::SyncIn = serde_json::from_value(sync()).unwrap();
    let p = super::judge(None, true, Some(&v), 5).unwrap();
    assert!(p.key.is_none() && p.objection.is_none() && p.sync.is_none());
    let p = super::judge(Some("k".into()), true, Some(&v), 5).unwrap();
    assert!(p.sync.is_none() && p.objection.as_ref().is_some_and(|a| a.method == dowiz_hub::consent::Method::Objection));
    let p = super::judge(Some("k".into()), false, Some(&v), 5).unwrap();
    assert!(p.objection.is_none() && p.sync.is_some());
}

/// The owner's forget reaches the taste image (hubdo/forget.rs step 1b): the profile goes with the card.
#[test]
fn the_owners_forget_takes_the_taste_profile() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let phone = "+355690000036";
    let r = place(&site, &dish, phone, json!({"taste_sync": sync()}));
    let id = r.body_value()["id"].as_str().unwrap().to_string();
    let key = key_of(&site, phone);
    assert_eq!(owner_view(&site, &owner, &key)["taste"]["orders"], 1);
    let rej = site.run(
        crate::owner::order_action,
        post(&at(&format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "reject", "reason": "closed"})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(rej.status_code(), 200, "{}", rej.body_str());
    let f = site.run(
        crate::services::customers::forget::forget_customer,
        post(&at(&format!("/api/owner/customers/{key}/forget")), &json!({"reason": "asked by email", "lang": "en"})).bearer(&owner).on("alpha"),
        &[("key", &key)],
    );
    assert_eq!(f.status_code(), 200, "{}", f.body_str());
    assert_eq!(owner_view(&site, &owner, &key)["taste"], Value::Null, "the profile went with the card");
}

/// W-TASTE2 row 2 through the routes: a dish on sale with a declared taste, ordered with a phone,
/// makes the order page's "For you" name the dishes the taste block ranks for that guest -- never
/// the one just ordered, never one the card's allergens rule out -- and the objection empties it.
#[test]
fn the_order_page_for_you_comes_from_the_server_profile_and_stops_at_the_objection() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let smoky = json!({"taste": {"salty": 3}, "texture": {"crispy": 3}, "aroma": {"smoky": 3}});
    let edit = |id: &str, body: Value| {
        let r = site.run(crate::owner::update_product, post(&at(&format!("/api/owner/products/{id}")), &body).bearer(&owner).on("alpha"), &[("id", id)]);
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
    };
    edit(&dish, json!({"location_id": "alpha", "sense": smoky}));
    let cat = site.run(crate::catalog_edit::set_category, post(&at("/api/owner/categories"), &json!({"location_id": "alpha", "name": "Hot"})).bearer(&owner).on("alpha"), &[]);
    let cat = cat.body_value()["id"].as_str().unwrap().to_string();
    let mut made = Vec::new();
    for (name, allergens) in [("Ebi tempura", json!([])), ("Salmon aburi", json!(["fish"])), ("Mochi", json!([]))] {
        let r = site.run(crate::catalog_edit::create_product, post(&at("/api/owner/products"), &json!({"location_id": "alpha", "category_id": cat, "name": name, "price": 700})).bearer(&owner).on("alpha"), &[]);
        let id = r.body_value()["id"].as_str().unwrap_or_else(|| panic!("{}", r.body_str())).to_string();
        let sense = match name {
            "Mochi" => json!({"taste": {"sweet": 5}, "texture": {"soft": 3}}),
            "Salmon aburi" => json!({"taste": {"salty": 1}, "texture": {"crispy": 3}, "aroma": {"smoky": 3}}),
            _ => smoky.clone(),
        };
        edit(&id, json!({"location_id": "alpha", "allergens": allergens, "available": true, "sense": sense}));
        made.push(id);
    }
    let phone = "+355690000041";
    let r = place(&site, &dish, phone, json!({}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let (id, token) = (r.body_value()["id"].as_str().unwrap().to_string(), r.body_value()["access_token"].as_str().unwrap().to_string());
    let fy = |tok: Option<&str>| {
        let mut c = get(&at(&format!("/api/order/{id}/taste/for-you"))).on("alpha");
        if let Some(t) = tok {
            c = c.bearer(t);
        }
        site.run(super::for_you, c, &[("id", &id)])
    };
    let v = fy(Some(&token));
    assert_eq!(v.status_code(), 200, "{}", v.body_str());
    let v = v.body_value();
    let ids: Vec<&str> = v["items"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap()).collect();
    assert_eq!(v["state"], "shown", "{v}");
    assert_eq!(ids.first().copied(), Some(made[0].as_str()), "the smoky dish not yet ordered comes first: {v}");
    assert!(!ids.contains(&dish.as_str()), "never the dish just ordered: {v}");
    assert!(!ids.contains(&made[2].as_str()), "a dish sharing no taste is not suggested: {v}");
    assert!(!v.to_string().contains("700"), "no amount anywhere in the answer: {v}");
    assert_eq!(fy(None).status_code(), 401, "no link, no answer");
    // A fish allergy on the guest's card takes the fish dish out before any ranking.
    let key = key_of(&site, phone);
    assert!(ids.contains(&made[1].as_str()), "before the card says fish, the fish dish is there: {v}");
    let card = site.run(crate::services::customers::record_routes::put_record, post(&at(&format!("/api/owner/customers/{key}/record?location_id=alpha")), &json!({"allergens": ["fish"]})).bearer(&owner).on("alpha"), &[("key", &key)]);
    assert_eq!(card.status_code(), 200, "{}", card.body_str());
    let v = fy(Some(&token)).body_value();
    assert!(!v.to_string().contains(&made[1]), "the card's allergen is left out: {v}");
    assert!(v.to_string().contains(&made[0]), "and the rest stays: {v}");
    // The objection: nothing, and it says so.
    let w = site.run(super::guest_withdraw, post(&at(&format!("/api/order/{id}/taste/withdraw")), &json!({})).bearer(&token).on("alpha"), &[("id", &id)]);
    assert_eq!(w.status_code(), 200, "{}", w.body_str());
    let v = fy(Some(&token)).body_value();
    assert_eq!((v["state"].clone(), v["items"].as_array().map(Vec::len)), (json!("off"), Some(0)), "{v}");
    // An order with no phone has no profile: 404, the page shows nothing.
    let anon = place(&site, &dish, "", json!({}));
    let (aid, atok) = (anon.body_value()["id"].as_str().unwrap().to_string(), anon.body_value()["access_token"].as_str().unwrap().to_string());
    let r = site.run(super::for_you, get(&at(&format!("/api/order/{aid}/taste/for-you"))).bearer(&atok).on("alpha"), &[("id", &aid)]);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
}

/// W-TASTE2 S7a through the health route: the radius between the phone's vector and the venue's
/// profile is a figure on the owner's health page, `acts: false`, and an empty venue compares 0.
#[test]
fn the_health_page_shows_the_phone_venue_radius_and_it_acts_on_nothing() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let health = || {
        let r = site.run(crate::services::operations::health, get(&at("/api/owner/health")).bearer(&owner).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        r.body_value()["sheaf"].clone()
    };
    let empty = health();
    assert_eq!((empty["taste"]["compared"].clone(), empty["taste"]["acts"].clone()), (json!(0), json!(false)), "{empty}");
    let r = site.run(crate::owner::update_product, post(&at(&format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "sense": {"aroma": {"smoky": 3}}})).bearer(&owner).on("alpha"), &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // The phone says sweet, the venue's fold says smoky: they disagree on everything.
    let sync = json!({"v": 1, "sense": {"t:sweet": 1000}});
    assert_eq!(place(&site, &dish, "+355690000042", json!({"taste_sync": sync})).status_code(), 200);
    let s = health();
    assert_eq!((s["taste"]["compared"].clone(), s["taste"]["maxPm"].clone(), s["taste"]["overHalf"].clone()), (json!(1), json!(1000), json!(1)), "{s}");
    assert_eq!(s["taste"]["acts"], false);
}
