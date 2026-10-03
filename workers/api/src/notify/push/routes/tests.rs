//! THE WHOLE PATH IN MEMORY (tested in-memory, NOT proven live -- the live
//! proof is `tools/live-proof/probes/feature-push.mjs`): three phones subscribe
//! through the routes with their own tokens, an order is placed, confirmed,
//! assigned and finished through the real handlers and the venue's real
//! object, the real drain sends through a fake push service, and each
//! delivered body is DECRYPTED with that phone's private key.
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::notify::push::{ece, subs};
use crate::storefront::route_tests::{open_venue, place_pickup};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::{json, Value};
use std::rc::Rc;

/// A TEST VAPID pair (the RFC 8291 sender key), never the live one.
const TEST_D: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
const TEST_PUB: &str = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn site() -> Site {
    let mut s = Site::with_secrets(&[("VAPID_PRIVATE_KEY", TEST_D)]);
    Rc::get_mut(&mut s.world).expect("the world is not shared yet").vars.insert("VAPID_PUBLIC_KEY".into(), TEST_PUB.into());
    s
}

/// A phone: its private key, and what its browser would post.
struct Phone {
    sk: [u8; 32],
    auth: [u8; 16],
    endpoint: String,
}

impl Phone {
    fn new(n: u8, host: &str) -> Self {
        Phone { sk: [n; 32], auth: [n.wrapping_add(100); 16], endpoint: format!("https://{host}/send/phone{n}") }
    }
    fn body(&self, lang: &str) -> Value {
        let pk = p256::SecretKey::from_slice(&self.sk).unwrap().public_key().to_encoded_point(false);
        json!({ "endpoint": self.endpoint, "expirationTime": null, "keys": { "p256dh": B64.encode(pk.as_bytes()), "auth": B64.encode(self.auth) }, "lang": lang })
    }
    /// Every message the fake push service received for this phone, opened.
    fn inbox(&self) -> Vec<Value> {
        sent()
            .into_iter()
            .filter(|c| c.url().map(|u| u.as_str() == self.endpoint).unwrap_or(false))
            .map(|c| serde_json::from_slice(&ece::open(&c.body_bytes(), &self.sk, &self.auth)).unwrap())
            .collect()
    }
}

fn subscribe(site: &Site, slug: &str, token: &str, body: &Value) -> crate::wire::Reply {
    site.run(super::subscribe, post(&at(slug, "/api/push/subscribe"), body).bearer(token).on(slug), &[])
}

fn act(site: &Site, slug: &str, owner: &str, id: &str, action: &str) {
    let r = site.run(
        crate::owner::order_action,
        post(&at(slug, &format!("/api/owner/orders/{id}/action")), &json!({"location_id": slug, "action": action})).bearer(owner).on(slug),
        &[("id", id)],
    );
    assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
}

/// The venue's drain, as its alarm runs it; answers what is still waiting.
fn drain(site: &Site, slug: &str) -> Vec<crate::outbox::Entry> {
    let env = site.env();
    block_on(crate::outbox::drain_venue(&env, slug, site.now_ms + 60_000));
    let place = crate::hubstore::Place { ns: env.durable_object("HUB").unwrap(), venue: slug.into() };
    block_on(crate::outbox::waiting(&place)).unwrap().into_iter().filter(|e| e.kind == crate::notify::push::plan::KIND).collect()
}

fn records(site: &Site, slug: &str, kind: &str) -> Vec<String> {
    let env = site.env();
    let place = crate::hubstore::Place { ns: env.durable_object("HUB").unwrap(), venue: slug.into() };
    block_on(crate::hubstore::load_table(&place, subs::IMAGE_PUSH, subs::PUSH_BYTES)).unwrap().table.all(kind).into_iter().map(|(id, _)| id).collect()
}

/// What a phone heard, sorted: the test world's clock does not move, so the
/// drain's oldest-first order is the entries' id order here.
fn heard(p: &Phone) -> Vec<String> {
    let mut v: Vec<String> = p.inbox().iter().map(|m| m["body"].as_str().unwrap().to_string()).collect();
    v.sort();
    v
}

fn sorted(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

fn created() -> crate::wire::Reply {
    crate::wire::Reply::empty().unwrap().with_status(201)
}

#[test]
fn the_key_route_serves_the_public_key_in_force() {
    let s = site();
    let r = s.run(super::key, get(&at("alpha", "/api/push/key")), &[]);
    assert_eq!(r.body_value()["key"], TEST_PUB);
    let plain = Site::new();
    let r = plain.run(super::key, get(&at("alpha", "/api/push/key")), &[]);
    assert_eq!(r.body_value()["key"], crate::notify::push::vapid::PUBLIC_KEY);
}

#[test]
fn staff_customer_and_courier_each_hear_their_own_and_the_message_opens_on_their_phone() {
    let s = site();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let (rider, rider_id) = s.courier("alpha", &owner, "+355691111111");
    answer_outbound(|_| Ok(created()));

    let staff_phone = Phone::new(1, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", &owner, &staff_phone.body("uk")).status_code(), 200);
    let rider_phone = Phone::new(2, "updates.push.services.mozilla.com");
    assert_eq!(subscribe(&s, "alpha", &rider, &rider_phone.body("sq")).status_code(), 200);

    let placed = place_pickup(&s, "alpha", &dish, 1);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = placed.body_value()["id"].as_str().or(placed.body_value()["order"]["id"].as_str()).unwrap().to_string();
    let token = placed.body_value()["access_token"].as_str().expect("the customer's token").to_string();
    let guest_phone = Phone::new(3, "web.push.apple.com");
    let r = subscribe(&s, "alpha", &token, &guest_phone.body("ru"));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());

    assert!(drain(&s, "alpha").is_empty(), "every push entry was delivered");
    let got = staff_phone.inbox();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["body"], "Нове замовлення");
    assert_eq!(got[0]["title"], format!("Замовлення #{}", &id[..8]));

    act(&s, "alpha", &owner, &id, "confirm");
    drain(&s, "alpha");
    let got = guest_phone.inbox();
    assert_eq!(got.len(), 1, "the guest hears CONFIRMED");
    assert_eq!(got[0]["body"], "Подтверждён");
    assert_eq!(got[0]["url"], format!("/?order={id}"));
    assert!(rider_phone.inbox().is_empty(), "the courier was not handed it yet");

    // VAPID: every request carried a token for its own push service, signed by our key.
    for c in sent() {
        let a = c.headers().get("authorization").unwrap().unwrap();
        assert!(a.starts_with("vapid t=") && a.ends_with(&format!(", k={TEST_PUB}")), "{a}");
        assert_eq!(c.headers().get("content-encoding").unwrap().as_deref(), Some("aes128gcm"));
        assert!(c.headers().get("ttl").unwrap().is_some());
    }
    let _ = (rider, rider_id);
}

fn place_delivery(site: &Site, slug: &str, dish: &str) -> crate::wire::Reply {
    site.run(
        crate::storefront::place,
        post(
            &at(slug, &format!("/api/public/locations/{slug}/orders")),
            &json!({
                "items": [{"product_id": dish, "quantity": 1}],
                "contact": {"name": "Guest", "phone": "+355690000002"},
                "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Tregtare 1, Durres"}},
                "payment": "cash",
            }),
        )
        .on(slug),
        &[("slug", slug)],
    )
}

#[test]
fn a_courier_hears_the_order_handed_to_them_and_the_customer_hears_the_road() {
    let s = site();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let (rider, rider_id) = s.courier("alpha", &owner, "+355691111111");
    let (other, _) = s.courier("alpha", &owner, "+355691111112");
    answer_outbound(|_| Ok(created()));
    let rider_phone = Phone::new(10, "updates.push.services.mozilla.com");
    let other_phone = Phone::new(11, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", &rider, &rider_phone.body("sq")).status_code(), 200);
    assert_eq!(subscribe(&s, "alpha", &other, &other_phone.body("en")).status_code(), 200);

    let placed = place_delivery(&s, "alpha", &dish);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = placed.body_value()["id"].as_str().or(placed.body_value()["order"]["id"].as_str()).unwrap().to_string();
    let guest = Phone::new(12, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", placed.body_value()["access_token"].as_str().unwrap(), &guest.body("en")).status_code(), 200);
    for a in ["confirm", "preparing"] {
        act(&s, "alpha", &owner, &id, a);
    }
    let r = s.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "shift: {}", r.body_str());
    let r = s.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "assign: {}", r.body_str());
    act(&s, "alpha", &owner, &id, "ready");
    // The courier's own taps go through the generic append: the customer still hears them.
    for route in ["accept", "pickup"] {
        let h = match route {
            "accept" => s.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
            _ => s.run(crate::courier::pickup, post(&at("alpha", &format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
        };
        assert_eq!(h.status_code(), 200, "{route}: {}", h.body_str());
    }
    let r = s.run(crate::courier::deliver, post(&at("alpha", &format!("/api/courier/orders/{id}/deliver")), &json!({"cash_collected": 900})).bearer(&rider).on("alpha"), &[("id", &id)]);
    assert_eq!(r.status_code(), 200, "deliver: {}", r.body_str());
    drain(&s, "alpha");

    assert_eq!(heard(&rider_phone), sorted(&["Ju është caktuar një porosi", "Gati për t'u marrë"]));
    assert!(other_phone.inbox().is_empty(), "the other courier was not handed it");
    assert_eq!(heard(&guest), sorted(&["Confirmed", "Being prepared", "Ready", "On the way", "Delivered"]));
    assert!(records(&s, "alpha", subs::K_CUSTOMER).is_empty());
}

#[test]
fn a_gone_phone_is_dropped_and_forgotten_and_the_end_of_the_order_forgets_the_guest() {
    let s = site();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let placed = place_pickup(&s, "alpha", &dish, 1);
    let id = placed.body_value()["id"].as_str().or(placed.body_value()["order"]["id"].as_str()).unwrap().to_string();
    let token = placed.body_value()["access_token"].as_str().unwrap().to_string();
    let guest = Phone::new(4, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", &token, &guest.body("en")).status_code(), 200);
    let staff = Phone::new(5, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", &owner, &staff.body("en")).status_code(), 200);
    assert_eq!(records(&s, "alpha", subs::K_STAFF).len(), 1);

    // The owner's phone uninstalled the app: its push service answers 410.
    let gone = staff.endpoint.clone();
    answer_outbound(move |c| Ok(if c.url().unwrap().as_str() == gone { created().with_status(410) } else { created() }));
    act(&s, "alpha", &owner, &id, "confirm");
    drain(&s, "alpha");
    assert_eq!(guest.inbox().len(), 1);

    // A second order: the staff device is still there, so the 410 lands now.
    let _ = place_pickup(&s, "alpha", &dish, 1);
    drain(&s, "alpha");
    assert!(records(&s, "alpha", subs::K_STAFF).is_empty(), "the 410 removed the owner's dead phone");

    for a in ["preparing", "ready"] {
        act(&s, "alpha", &owner, &id, a);
    }
    assert_eq!(records(&s, "alpha", subs::K_CUSTOMER).len(), 1, "still live: still subscribed");
    act(&s, "alpha", &owner, &id, "collected");
    assert!(records(&s, "alpha", subs::K_CUSTOMER).is_empty(), "the order ended: the guest's device is forgotten");
    drain(&s, "alpha");
    assert_eq!(heard(&guest), sorted(&["Confirmed", "Being prepared", "Ready", "Collected"]), "and the last word was still delivered");
}

#[test]
fn subscribing_needs_a_token_a_push_service_and_a_clean_body() {
    let s = site();
    let (owner, _) = open_venue(&s, "alpha", "a@x.test");
    let ok = Phone::new(6, "fcm.googleapis.com").body("en");
    let r = s.run(super::subscribe, post(&at("alpha", "/api/push/subscribe"), &ok).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "no token: {}", r.body_str());
    let evil = Phone::new(6, "evil.example").body("en");
    assert_eq!(subscribe(&s, "alpha", &owner, &evil).status_code(), 400);
    let mut extra = ok.clone();
    extra["venue"] = json!("beta");
    assert_eq!(subscribe(&s, "alpha", &owner, &extra).status_code(), 400, "the body never names a venue");
    assert_eq!(subscribe(&s, "alpha", &owner, &ok).status_code(), 200);
}

#[test]
fn state_and_unsubscribe_answer_for_this_device_only() {
    let s = site();
    let (owner, _) = open_venue(&s, "alpha", "a@x.test");
    let p = Phone::new(7, "fcm.googleapis.com");
    let state = |s: &Site| s.run(super::state, post(&at("alpha", "/api/push/state"), &json!({"endpoint": p.endpoint})).bearer(&owner).on("alpha"), &[]).body_value()["on"].clone();
    assert_eq!(state(&s), json!(false));
    assert_eq!(subscribe(&s, "alpha", &owner, &p.body("en")).status_code(), 200);
    assert_eq!(state(&s), json!(true));
    let r = s.run(super::unsubscribe, post(&at("alpha", "/api/push/unsubscribe"), &json!({"endpoint": p.endpoint})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.body_value()["removed"], json!(true));
    assert_eq!(state(&s), json!(false));
}

#[test]
fn another_venues_owner_subscribes_to_their_own_venue_not_this_one() {
    let s = site();
    let (_alpha_owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let beta_owner = s.venue("beta", "b@x.test");
    let p = Phone::new(8, "fcm.googleapis.com");
    // Beta's owner, at alpha's host: the record lands in BETA's image.
    assert_eq!(subscribe(&s, "alpha", &beta_owner, &p.body("en")).status_code(), 200);
    assert!(records(&s, "alpha", subs::K_STAFF).is_empty());
    assert_eq!(records(&s, "beta", subs::K_STAFF).len(), 1);
    answer_outbound(|_| Ok(created()));
    let _ = place_pickup(&s, "alpha", &dish, 1);
    drain(&s, "alpha");
    assert!(p.inbox().is_empty(), "alpha's order never reaches beta's owner");
}

#[test]
fn without_the_vapid_secret_nothing_is_sent_and_the_entry_waits_loudly() {
    let s = Site::new();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let p = Phone::new(9, "fcm.googleapis.com");
    assert_eq!(subscribe(&s, "alpha", &owner, &p.body("en")).status_code(), 200);
    answer_outbound(|_| Ok(created()));
    let _ = place_pickup(&s, "alpha", &dish, 1);
    let waiting = drain(&s, "alpha");
    assert_eq!(waiting.len(), 1, "the entry waits, no try spent");
    assert_eq!(waiting[0].tries, 0);
    assert!(p.inbox().is_empty());
    // and the line the drain says out loud (`outbox.abandoned`) names the missing secret
    let mut rail = crate::notify::push::rail::Rail::default();
    assert_eq!(block_on(rail.send(&s.env(), &waiting[0], s.now_ms)), crate::notify::push::rail::Push::Wait);
    assert!(rail.said.iter().any(|l| l.contains("VAPID_PRIVATE_KEY is not set")), "{:?}", rail.said);
}
