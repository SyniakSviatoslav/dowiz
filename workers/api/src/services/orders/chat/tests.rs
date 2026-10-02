//! The courier chat through its routes (W-URGENT 2026-10-02): the kitchen and a
//! different courier get 403; the customer and the assigned courier post and
//! read; the owner reads and cannot post; after the order is over posting is
//! refused; forgetting the customer erases the thread; a line reaches the two
//! parties' sockets and nobody else's.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST, T0};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// A delivery order on alpha: (order id, the customer's key).
fn place_delivery(site: &Site, dish: &str, phone: &str) -> (String, String) {
    let r = site.run(
        crate::storefront::place,
        post(
            &at("/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "Ana Hoxha", "phone": phone},
                    "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Tregtare 5"}}, "payment": "cash"}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap().to_string())
}

fn act(site: &Site, owner: &str, id: &str, body: Value) {
    let r = site.run(
        crate::owner::order_action,
        post(&at(&format!("/api/owner/orders/{id}/action")), &body).bearer(owner).on("alpha"),
        &[("id", id)],
    );
    assert_eq!(r.status_code(), 200, "{body}: {}", r.body_str());
}

fn assign(site: &Site, owner: &str, id: &str, courier_id: &str) {
    let r = site.run(
        crate::owner::assign_courier,
        post(&at(&format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": courier_id})).bearer(owner).on("alpha"),
        &[("id", id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

/// The assigned courier carries the order to its end: accept, pick up (CONFIRMED
/// goes straight to IN_DELIVERY), deliver. The one legal exit from CONFIRMED --
/// the FSM lets only a PENDING order be cancelled.
fn finish(site: &Site, rider: &str, id: &str) {
    for (name, r) in [
        ("accept", site.run(crate::courier::accept, post(&at(&format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(rider).on("alpha"), &[("id", id)])),
        ("pickup", site.run(crate::courier::pickup, post(&at(&format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(rider).on("alpha"), &[("id", id)])),
        ("deliver", site.run(crate::courier::deliver, post(&at(&format!("/api/courier/orders/{id}/deliver")), &json!({"cash_collected": 900})).bearer(rider).on("alpha"), &[("id", id)])),
    ] {
        assert_eq!(r.status_code(), 200, "{name}: {}", r.body_str());
    }
}

fn read(site: &Site, id: &str, tok: &str, since: i64) -> crate::wire::Reply {
    site.run(super::read, get(&at(&format!("/api/order/{id}/chat?since={since}"))).bearer(tok).on("alpha"), &[("id", id)])
}

fn say(site: &Site, id: &str, tok: &str, text: &str, client_id: &str) -> crate::wire::Reply {
    site.run(
        super::send,
        post(&at(&format!("/api/order/{id}/chat")), &json!({"text": text, "clientId": client_id})).bearer(tok).on("alpha"),
        &[("id", id)],
    )
}

/// A confirmed delivery with courier `rider` on it: (order id, customer key, rider token, rider id).
fn running(site: &Site, owner: &str, dish: &str) -> (String, String, String, String) {
    let (id, key) = place_delivery(site, dish, "+355691110001");
    act(site, owner, &id, json!({"location_id": "alpha", "action": "confirm"}));
    let (rider, rider_id) = site.courier("alpha", owner, "+355699990001");
    assign(site, owner, &id, &rider_id);
    (id, key, rider, rider_id)
}

#[test]
fn the_parties_talk_the_owner_reads_strangers_are_refused_and_the_end_of_the_order_closes_it() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (id, key) = place_delivery(&site, &dish, "+355691110001");
    act(&site, &owner, &id, json!({"location_id": "alpha", "action": "confirm"}));
    // Before a courier: readable, says "waiting", takes nothing.
    let r = read(&site, &id, &key, 0);
    assert_eq!((r.status_code(), r.body_value()["state"].as_str()), (200, Some("waiting")), "{}", r.body_str());
    assert_eq!(say(&site, &id, &key, "hello?", "c0").status_code(), 409);

    let (rider, rider_id) = site.courier("alpha", &owner, "+355699990001");
    let (other, _) = site.courier("alpha", &owner, "+355699990002");
    let kitchen = site.staff("alpha", &owner, "k@x.test", "kitchen");
    assign(&site, &owner, &id, &rider_id);

    // THE KITCHEN NEVER, ANOTHER COURIER NEVER, ANOTHER ORDER'S CUSTOMER IS NOT TOLD IT EXISTS.
    assert_eq!(read(&site, &id, &kitchen, 0).status_code(), 403);
    assert_eq!(say(&site, &id, &kitchen, "hi", "k1").status_code(), 403);
    assert_eq!(read(&site, &id, &other, 0).status_code(), 403);
    assert_eq!(say(&site, &id, &other, "hi", "o1").status_code(), 403);
    let (_, key2) = place_delivery(&site, &dish, "+355691110002");
    assert_eq!(read(&site, &id, &key2, 0).status_code(), 404);
    assert_eq!(say(&site, &id, &key2, "hi", "x1").status_code(), 404);
    let anon = site.run(super::read, get(&at(&format!("/api/order/{id}/chat"))).on("alpha"), &[("id", &id)]);
    assert_eq!(anon.status_code(), 401, "{}", anon.body_str());

    // THE CUSTOMER AND THE ASSIGNED COURIER POST AND READ; a resend is the same line.
    let r = say(&site, &id, &key, "  I am at the gate ", "c1");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["replayed"], false);
    let msg = r.body_value()["id"].as_str().unwrap().to_string();
    let again = say(&site, &id, &key, "I am at the gate", "c1");
    assert_eq!((again.body_value()["id"].as_str(), again.body_value()["replayed"].as_bool()), (Some(msg.as_str()), Some(true)));
    let r = say(&site, &id, &rider, "two minutes", "r1");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let seen = read(&site, &id, &rider, 0);
    let v = seen.body_value();
    let ms = v["messages"].as_array().unwrap();
    assert_eq!((v["state"].as_str(), v["me"].as_str(), ms.len()), (Some("open"), Some("COURIER"), 2), "{v}");
    assert_eq!((ms[0]["from"].as_str(), ms[0]["text"].as_str()), (Some("CUSTOMER"), Some("I am at the gate")), "trimmed, oldest first: {v}");
    assert_eq!(ms[1]["from"], "COURIER");
    let mine = read(&site, &id, &key, 0).body_value();
    assert_eq!((mine["me"].as_str(), mine["messages"].as_array().unwrap().len()), (Some("CUSTOMER"), 2));
    assert!(read(&site, &id, &key, T0).body_value()["messages"].as_array().unwrap().is_empty(), "since the last instant: nothing new");

    // THE OWNER READS EVERYTHING AND SAYS NOTHING.
    let o = read(&site, &id, &owner, 0);
    assert_eq!(o.status_code(), 200, "{}", o.body_str());
    assert_eq!((o.body_value()["me"].is_null(), o.body_value()["messages"].as_array().unwrap().len()), (true, 2));
    assert_eq!(say(&site, &id, &owner, "stop", "v1").status_code(), 403);
    // Another venue's owner: not even told it exists.
    let beta = site.venue("beta", "b@x.test");
    assert_eq!(read(&site, &id, &beta, 0).status_code(), 404);

    // WHAT IS REFUSED ABOUT A LINE: empty, too long, a field nobody declared, no client id.
    assert_eq!(say(&site, &id, &key, "   ", "e1").status_code(), 400);
    assert_eq!(say(&site, &id, &key, &"x".repeat(501), "e2").status_code(), 400);
    assert_eq!(say(&site, &id, &key, "ok", " ").status_code(), 400);
    let extra = site.run(
        super::send,
        post(&at(&format!("/api/order/{id}/chat")), &json!({"text": "hi", "clientId": "e3", "phone": "+355"})).bearer(&key).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(extra.status_code(), 400, "{}", extra.body_str());
    assert!(extra.body_str().contains("phone"), "the unknown field is named: {}", extra.body_str());
    assert_eq!(read(&site, &id, &owner, 0).body_value()["total"], 2, "the refusals wrote nothing");

    // THE ORDER OVER, THE CHAT IS READ-ONLY.
    finish(&site, &rider, &id);
    assert_eq!(say(&site, &id, &rider, "sorry", "r2").status_code(), 409);
    assert_eq!(say(&site, &id, &key, "never mind", "c9").status_code(), 409);
    let closed = read(&site, &id, &key, 0).body_value();
    assert_eq!((closed["state"].as_str(), closed["messages"].as_array().unwrap().len()), (Some("closed"), 2), "{closed}");
}

#[test]
fn thirty_lines_an_hour_per_side_and_the_thirty_first_is_refused() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (id, key, rider, _) = running(&site, &owner, &dish);
    for i in 0..30 {
        let r = say(&site, &id, &key, &format!("line {i}"), &format!("c{i}"));
        assert_eq!(r.status_code(), 200, "line {i}: {}", r.body_str());
    }
    assert_eq!(say(&site, &id, &key, "one more", "c30").status_code(), 429);
    assert_eq!(say(&site, &id, &rider, "the other side still speaks", "r0").status_code(), 200);
    assert_eq!(read(&site, &id, &owner, 0).body_value()["total"], 31);
}

#[test]
fn forgetting_the_customer_erases_the_thread() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (id, key, rider, _) = running(&site, &owner, &dish);
    assert_eq!(say(&site, &id, &key, "third floor", "c1").status_code(), 200);
    assert_eq!(say(&site, &id, &rider, "coming up", "r1").status_code(), 200);
    assert_eq!(read(&site, &id, &owner, 0).body_value()["total"], 2);
    finish(&site, &rider, &id);
    let list = site.run(crate::services::customers::handlers::customers, get(&at("/api/owner/customers")).bearer(&owner).on("alpha"), &[]);
    let customer_key = list.body_value()["customers"][0]["key"].as_str().expect("a customer card").to_string();
    let r = site.run(
        crate::services::customers::forget::forget_customer,
        post(&at(&format!("/api/owner/customers/{customer_key}/forget")), &json!({"reason": "asked by email", "lang": "en"})).bearer(&owner).on("alpha"),
        &[("key", &customer_key)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["threads"], 2, "both lines went with the person: {}", r.body_str());
    let after = read(&site, &id, &owner, 0);
    assert_eq!(after.status_code(), 200, "{}", after.body_str());
    assert_eq!(after.body_value()["total"], 0, "{}", after.body_str());
    assert!(!after.body_str().contains("third floor"));
}

#[test]
fn a_line_nudges_the_two_parties_sockets_and_nobody_elses() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (id, key, _rider, rider_id) = running(&site, &owner, &dish);
    let host = site.world.host("alpha");
    let customer = host.socket(&[&crate::hubdo::tag_order(&id)]);
    let courier = host.socket(&[crate::hubdo::TAG_COURIER, &crate::hubdo::tag_courier(&rider_id)]);
    let console = host.socket(&[crate::hubdo::TAG_CONSOLE]);
    let kitchen = host.socket(&[crate::services::orders::kitchen_ack::board::TAG_KITCHEN]);
    let other_order = host.socket(&[&crate::hubdo::tag_order("ord_other")]);
    assert_eq!(say(&site, &id, &key, "here", "c1").status_code(), 200);
    assert_eq!(customer.borrow().len(), 1, "{:?}", customer.borrow());
    assert_eq!(courier.borrow().len(), 1, "{:?}", courier.borrow());
    let frame: Value = serde_json::from_str(&customer.borrow()[0]).unwrap();
    assert_eq!((frame["t"].as_str(), frame["orderId"].as_str()), (Some("chat"), Some(id.as_str())));
    assert!(!customer.borrow()[0].contains("here"), "the frame carries no text: {}", customer.borrow()[0]);
    assert!(console.borrow().is_empty(), "the console hears nothing of it");
    assert!(kitchen.borrow().is_empty(), "the kitchen hears nothing of it");
    assert!(other_order.borrow().is_empty());
}
