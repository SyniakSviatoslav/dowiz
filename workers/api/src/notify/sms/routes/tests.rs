//! THE WHOLE PATH IN MEMORY (tested in-memory, NOT proven live -- the live
//! proof is `tools/live-proof/probes/feature-sms.mjs`, which needs the venue's
//! Android phone): the owner sets the gateway through the route, a guest ticks
//! the SMS box at checkout, the owner moves the order through the real
//! handlers and the venue's real object, and the real drain posts to a FAKE
//! SMSGate. Then a STOP, an unticked order, the budget, a wrong password.
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::notify::sms::config::SMSGATE_URL;
use crate::storefront::route_tests::open_venue;
use crate::wire::Reply;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use dowiz_hub::consent::sms_wordings::sms_wording_id;
use serde_json::{json, Value};

const GUEST: &str = "+355690000002";

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn q(path: &str) -> String {
    at("alpha", &format!("{path}?location_id=alpha"))
}

fn set(s: &Site, owner: &str, body: Value) -> Reply {
    s.run(super::set, post(&q("/api/owner/sms"), &body).bearer(owner).on("alpha"), &[])
}

fn card(s: &Site, owner: &str) -> Value {
    let r = s.run(super::status, get(&q("/api/owner/sms")).bearer(owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn place(s: &Site, dish: &str, sms: Option<Value>, phone: &str) -> Reply {
    let mut body = json!({
        "items": [{"product_id": dish, "quantity": 1}],
        "contact": {"name": "Arben Hoxha", "phone": phone},
        "fulfilment": {"kind": "pickup"},
        "payment": "cash",
    });
    if let Some(b) = sms {
        body["sms"] = b;
    }
    s.run(crate::storefront::place, post(&at("alpha", "/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")])
}

fn tick() -> Option<Value> {
    Some(json!({"order_status": true, "wording": sms_wording_id("en")}))
}

fn id_of(r: &Reply) -> String {
    let v = r.body_value();
    v["id"].as_str().or(v["order"]["id"].as_str()).unwrap_or_else(|| panic!("no id: {}", r.body_str())).to_string()
}

fn act(s: &Site, owner: &str, id: &str, action: &str) {
    let r = s.run(
        crate::owner::order_action,
        post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": action})).bearer(owner).on("alpha"),
        &[("id", id)],
    );
    assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
}

fn drain(s: &Site) {
    block_on(crate::outbox::drain_venue(&s.env(), "alpha", s.now_ms + 60_000));
}

/// Every text the fake SMSGate received: (recipient, text, authorization).
fn texts() -> Vec<(String, String, String)> {
    sent()
        .into_iter()
        .filter(|c| c.url().map(|u| u.as_str() == SMSGATE_URL).unwrap_or(false))
        .map(|c| {
            let b: Value = serde_json::from_slice(&c.body_bytes()).unwrap();
            let auth = c.headers().get("authorization").ok().flatten().unwrap_or_default();
            (b["phoneNumbers"][0].as_str().unwrap().to_string(), b["textMessage"]["text"].as_str().unwrap().to_string(), auth)
        })
        .collect()
}

fn accepted() -> Reply {
    Reply::from_json(&json!({"id": "m1", "state": "Pending", "deviceId": "d1", "recipients": []})).unwrap().with_status(202)
}

fn configured() -> (Site, String, String) {
    let s = Site::new();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let r = set(&s, &owner, json!({"on": true, "provider": "smsgate", "user": "GATEUSER", "secret": "gate-password-14"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(!r.body_str().contains("gate-password-14"), "the password never comes back: {}", r.body_str());
    (s, owner, dish)
}

#[test]
fn a_ticked_order_is_texted_through_the_venues_own_gateway_and_nothing_personal_goes() {
    let (s, owner, dish) = configured();
    answer_outbound(|_| Ok(accepted()));
    let box_ = s.run(super::box_for, get(&at("alpha", "/api/public/locations/alpha/sms")).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(box_.body_value()["on"], true);
    assert_eq!(box_.body_value()["wordings"].as_array().unwrap().len(), 4);

    let placed = place(&s, &dish, tick(), GUEST);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = id_of(&placed);
    for a in ["confirm", "preparing", "ready"] {
        act(&s, &owner, &id, a);
    }
    drain(&s);
    let got = texts();
    let short: String = id.chars().take(8).collect();
    let want_auth = format!("Basic {}", B64.encode("GATEUSER:gate-password-14"));
    assert_eq!(got.len(), 2, "CONFIRMED and READY (a pickup); PREPARING is not texted: {got:?}");
    for (to, text, auth) in &got {
        assert_eq!((to.as_str(), auth.as_str()), (GUEST, want_auth.as_str()));
        assert!(text.contains(&format!("#{short}")) && text.contains("STOP"), "{text}");
        assert!(!text.contains("Arben") && !text.contains("Futomaki") && !text.contains("900"), "nothing personal: {text}");
    }
    assert!(got[0].1.contains("Confirmed") && got[1].1.contains("Ready"), "{got:?}");
    let c = card(&s, &owner);
    assert_eq!((c["health"]["sent"].as_u64(), c["waiting"].as_u64()), (Some(2), Some(0)), "{c}");
    assert_eq!(c["config"]["secret_set"], true);
}

#[test]
fn no_tick_no_text_and_a_stop_wins_over_a_queued_text() {
    let (s, owner, dish) = configured();
    answer_outbound(|_| Ok(accepted()));
    let plain = place(&s, &dish, None, GUEST);
    act(&s, &owner, &id_of(&plain), "confirm");
    drain(&s);
    assert!(texts().is_empty(), "the box was not ticked");

    let ticked = place(&s, &dish, tick(), GUEST);
    let id = id_of(&ticked);
    act(&s, &owner, &id, "confirm");
    // The customer tells the venue STOP before the drain runs, in the national spelling.
    let r = s.run(super::stop, post(&q("/api/owner/sms/stop"), &json!({"phone": "069 000 0002"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    drain(&s);
    assert!(texts().is_empty(), "STOP filed after the tick must win: {:?}", texts());
    assert_eq!(card(&s, &owner)["health"]["stopped"].as_u64(), Some(1));
}

#[test]
fn switched_off_nothing_is_queued_and_the_box_is_hidden() {
    let s = Site::new();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    answer_outbound(|_| Ok(accepted()));
    let box_ = s.run(super::box_for, get(&at("alpha", "/api/public/locations/alpha/sms")).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(box_.body_value()["on"], false);
    let placed = place(&s, &dish, tick(), GUEST);
    act(&s, &owner, &id_of(&placed), "confirm");
    drain(&s);
    assert!(texts().is_empty());
    assert_eq!(card(&s, &owner)["waiting"].as_u64(), Some(0), "nothing queued for a venue with SMS off");
}

#[test]
fn a_tick_without_a_usable_number_is_refused_at_checkout() {
    let (s, _owner, dish) = configured();
    let r = place(&s, &dish, tick(), "12345678");
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    assert!(r.body_str().contains("full phone number"), "{}", r.body_str());
    let r = place(&s, &dish, Some(json!({"order_status": true, "wording": "0000000000000000"})), GUEST);
    assert_eq!(r.status_code(), 400, "an unknown sentence is no proof: {}", r.body_str());
    let r = place(&s, &dish, Some(json!({"order_status": true, "wording": sms_wording_id("en"), "marketing": true})), GUEST);
    assert_eq!(r.status_code(), 400, "a closed shape: {}", r.body_str());
}

#[test]
fn the_test_button_reports_the_providers_verdict() {
    let (s, owner, _) = configured();
    answer_outbound(|_| Ok(accepted()));
    let r = s.run(super::test, post(&q("/api/owner/sms/test"), &json!({"phone": "+355690000009"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!((r.status_code(), r.body_value()["ok"].clone()), (200, json!(true)), "{}", r.body_str());
    assert!(texts().iter().any(|(to, t, _)| to == "+355690000009" && t.contains("test OK")));

    answer_outbound(|_| Ok(Reply::from_json(&json!({"message": "Unauthorized"})).unwrap().with_status(401)));
    let r = s.run(super::test, post(&q("/api/owner/sms/test"), &json!({"phone": "+355690000009"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.body_value()["why"], "sms_why_auth", "{}", r.body_str());
    let r = s.run(super::test, post(&q("/api/owner/sms/test"), &json!({"phone": "069"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400);
}

#[test]
fn a_wrong_password_retries_then_says_so_and_the_daily_limit_holds() {
    let (s, owner, dish) = configured();
    answer_outbound(|_| Ok(Reply::from_json(&json!({"message": "Unauthorized"})).unwrap().with_status(401)));
    let placed = place(&s, &dish, tick(), GUEST);
    act(&s, &owner, &id_of(&placed), "confirm");
    drain(&s);
    let c = card(&s, &owner);
    assert_eq!(c["health"]["last_err"]["why"], "sms_why_auth", "{c}");
    assert_eq!(c["waiting"].as_u64(), Some(1), "an ordinary failure waits for its backoff");

    // A limit of one: the second text of the day is dropped, not sent.
    let r = set(&s, &owner, json!({"on": true, "provider": "smsgate", "user": "GATEUSER", "daily": 1}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    answer_outbound(|_| Ok(accepted()));
    let s2 = Site { world: s.world.clone(), now_ms: s.now_ms + 120_000 };
    drain(&s2);
    let p2 = place(&s, &dish, tick(), GUEST);
    act(&s, &owner, &id_of(&p2), "confirm");
    drain(&s2);
    let c = card(&s, &owner);
    assert_eq!((c["health"]["sent"].as_u64(), c["health"]["over_budget"].as_u64()), (Some(1), Some(1)), "{c}");
}

#[test]
fn a_bad_configuration_is_refused_while_the_owner_can_fix_it() {
    let s = Site::new();
    let (owner, _) = open_venue(&s, "alpha", "a@x.test");
    let r = set(&s, &owner, json!({"on": true, "provider": "smsgate", "user": "U"}));
    assert_eq!(r.status_code(), 400, "no password: {}", r.body_str());
    let r = set(&s, &owner, json!({"on": false, "provider": "smsgate", "url": "http://192.168.1.5:8080/message"}));
    assert_eq!(r.status_code(), 400, "plain http: {}", r.body_str());
    let r = set(&s, &owner, json!({"on": false, "provider": "smsgate", "token": "x"}));
    assert_eq!(r.status_code(), 400, "a closed shape: {}", r.body_str());
    let other = s.venue("beta", "b@x.test");
    let r = s.run(super::status, get(&q("/api/owner/sms")).bearer(&other).on("alpha"), &[]);
    assert_ne!(r.status_code(), 200, "another venue's owner cannot read this card");
}

#[test]
fn a_couriers_pickup_texts_on_the_way_through_the_generic_append() {
    let (s, owner, dish) = configured();
    let (rider, rider_id) = s.courier("alpha", &owner, "+355691111111");
    answer_outbound(|_| Ok(accepted()));
    let body = json!({
        "items": [{"product_id": dish, "quantity": 1}],
        "contact": {"name": "Guest", "phone": GUEST},
        "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Tregtare 1, Durres"}},
        "payment": "cash",
        "sms": {"order_status": true, "wording": sms_wording_id("sq")},
    });
    let placed = s.run(crate::storefront::place, post(&at("alpha", "/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = id_of(&placed);
    for a in ["confirm", "preparing"] {
        act(&s, &owner, &id, a);
    }
    let r = s.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "shift: {}", r.body_str());
    let r = s.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "assign: {}", r.body_str());
    act(&s, &owner, &id, "ready");
    for route in ["accept", "pickup"] {
        let h = match route {
            "accept" => s.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
            _ => s.run(crate::courier::pickup, post(&at("alpha", &format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
        };
        assert_eq!(h.status_code(), 200, "{route}: {}", h.body_str());
    }
    drain(&s);
    let got: Vec<String> = texts().into_iter().map(|(_, t, _)| t).collect();
    assert_eq!(got.len(), 2, "CONFIRMED and IN_DELIVERY; a delivery's READY is not texted: {got:?}");
    assert!(got[0].contains("U konfirmua") && got[1].contains("Ne rruge"), "{got:?}");
}
