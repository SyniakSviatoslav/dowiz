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
