//! Message threads through their routes (W-COV C2): a customer and the venue talk in the
//! order's thread, a resend is free, a READ receipt keeps no text, and nobody else reads it --
//! not another order's customer, not a courier, not a thread id that names nothing.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn say(site: &Site, id: &str, tok: &str, body: serde_json::Value) -> crate::wire::Reply {
    site.run(
        crate::social::send,
        post(&at(&format!("/api/public/locations/alpha/threads/{id}/messages")), &body).bearer(tok).on("alpha"),
        &[("slug", "alpha"), ("id", id)],
    )
}

fn read(site: &Site, id: &str, tok: &str, after: u64) -> crate::wire::Reply {
    site.run(
        crate::social::messages,
        get(&at(&format!("/api/public/locations/alpha/threads/{id}?after={after}"))).bearer(tok).on("alpha"),
        &[("slug", "alpha"), ("id", id)],
    )
}

#[test]
fn a_customer_and_the_venue_talk_in_the_orders_thread_and_nobody_else_does() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let v = place_pickup(&site, "alpha", &dish, 1).body_value();
    let (id, key) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap().to_string());
    let v2 = place_pickup(&site, "alpha", &dish, 1).body_value();
    let (id2, key2) = (v2["id"].as_str().unwrap().to_string(), v2["access_token"].as_str().unwrap().to_string());

    // The customer speaks; `from` in the body is ignored -- the token decides the side.
    let r = say(&site, &id, &key, json!({"from": "VENUE", "body": "no wasabi please", "clientId": "c1"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["replayed"], false);
    let msg = r.body_value()["id"].as_str().unwrap().to_string();
    // A resend with the same clientId writes nothing new.
    let again = say(&site, &id, &key, json!({"from": "CUSTOMER", "body": "no wasabi please", "clientId": "c1"}));
    assert_eq!((again.body_value()["id"].as_str(), again.body_value()["replayed"].as_bool()), (Some(msg.as_str()), Some(true)));
    assert_eq!(say(&site, &id, &key, json!({"from": "CUSTOMER", "body": "x", "clientId": " "})).status_code(), 400, "clientId is required");

    // The venue answers and reads everything.
    let r = say(&site, &id, &owner, json!({"from": "CUSTOMER", "body": "noted", "clientId": "v1"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let all = read(&site, &id, &owner, 0).body_value();
    let ms = all["messages"].as_array().unwrap().clone();
    assert_eq!(ms.len(), 2, "{all}");
    // Each side numbers its own lines, so both are seq 1; the side breaks the tie.
    let line = |from: &str| ms.iter().find(|m| m["from"] == from).and_then(|m| m["body"].as_str()).map(str::to_string);
    assert_eq!(line("CUSTOMER").as_deref(), Some("no wasabi please"), "{all}");
    assert_eq!(line("VENUE").as_deref(), Some("noted"), "{all}");
    assert_eq!(all["unread"]["CUSTOMER"], 1, "the customer has not read the venue's line: {all}");
    // `after` is a sequence cursor per side; the customer's seq 1 is behind it.
    let later = read(&site, &id, &key, 1).body_value();
    assert!(later["messages"].as_array().unwrap().iter().all(|m| m["seq"].as_u64().unwrap() > 1), "{later}");

    // A READ receipt keeps no text, whatever the body carried.
    let r = say(&site, &id, &key, json!({"from": "CUSTOMER", "kind": "READ", "readThrough": 1, "body": "x".repeat(4096), "clientId": "r1"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let all = read(&site, &id, &owner, 0).body_value();
    let receipt = all["messages"].as_array().unwrap().iter().find(|m| m["kind"] == "READ").cloned().expect("receipt");
    assert_eq!((receipt["body"].as_str(), receipt["readThrough"].as_u64()), (Some(""), Some(1)), "{receipt}");
    assert_eq!(all["unread"]["CUSTOMER"], 0, "{all}");

    // Another order's customer: not found, both reading and speaking; nothing written.
    assert_eq!(read(&site, &id, &key2, 0).status_code(), 404);
    assert_eq!(say(&site, &id, &key2, json!({"from": "CUSTOMER", "body": "hi", "clientId": "z"})).status_code(), 404);
    // A thread id that names no order and has no messages is not created by a typo.
    assert_eq!(say(&site, "ord_nothing", &owner, json!({"from": "VENUE", "body": "hi", "clientId": "t"})).status_code(), 404);
    // The second order's customer has their own, empty thread.
    let own = read(&site, &id2, &key2, 0);
    assert_eq!(own.status_code(), 200, "{}", own.body_str());
    assert!(own.body_value()["messages"].as_array().unwrap().is_empty());
    // No token at all.
    let anon = site.run(
        crate::social::messages,
        get(&at(&format!("/api/public/locations/alpha/threads/{id}"))).on("alpha"),
        &[("slug", "alpha"), ("id", &id)],
    );
    assert_eq!(anon.status_code(), 401, "{}", anon.body_str());
    assert_eq!(read(&site, &id, &owner, 0).body_value()["messages"].as_array().unwrap().len(), 3, "the refusals wrote nothing");

    // The venue's inbox names the thread; a courier reads nothing.
    let inbox = site.run(crate::social::inbox::list, get(&at("/api/owner/threads")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(inbox.status_code(), 200, "{}", inbox.body_str());
    assert!(inbox.body_str().contains(&id), "{}", inbox.body_str());
    let (rider, _) = site.courier("alpha", &owner, "+355691119999");
    assert_eq!(read(&site, &id, &rider, 0).status_code(), 403);
    let r = site.run(crate::social::inbox::list, get(&at("/api/owner/threads")).bearer(&key).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "a customer does not read the venue's inbox: {}", r.body_str());
}

#[test]
fn another_venues_owner_cannot_read_or_speak_in_this_venues_thread() {
    let site = Site::new();
    let (_owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (beta, _) = open_venue(&site, "beta", "b@x.test");
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    // Refused without confirming the thread exists (a 404 is the same answer as no thread).
    let r = read(&site, &id, &beta, 0);
    assert!([401, 403, 404].contains(&r.status_code()), "{}", r.body_str());
    let r = say(&site, &id, &beta, json!({"from": "VENUE", "body": "hi", "clientId": "b1"}));
    assert!([401, 403, 404].contains(&r.status_code()), "{}", r.body_str());
    assert!(!r.body_str().contains("hi"), "nothing echoed back");
}
