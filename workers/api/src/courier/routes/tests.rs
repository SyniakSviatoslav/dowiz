//! A delivery through the courier routes (W-COV C2): invited and claimed, on shift, assigned by
//! the owner, accepted, picked up, delivered with the cash counted -- and a courier of another
//! venue refused at every step.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
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

fn id_of(r: &crate::wire::Reply) -> String {
    let v = r.body_value();
    ["id", "order_id", "orderId"]
        .iter()
        .find_map(|k| v[*k].as_str().or(v["order"][*k].as_str()).map(str::to_string))
        .unwrap_or_else(|| panic!("no id in {}", r.body_str()))
}

fn status(site: &Site, slug: &str, id: &str) -> String {
    let o = site.fold(slug, &format!("/fold/order?id={id}")).body_value();
    let v: Value = serde_json::from_str(o["order_json"].as_str().unwrap_or("{}")).unwrap_or_default();
    v["status"].as_str().unwrap_or("").to_string()
}

fn act(site: &Site, slug: &str, owner: &str, id: &str, action: &str) -> crate::wire::Reply {
    site.run(
        crate::owner::order_action,
        post(&at(slug, &format!("/api/owner/orders/{id}/action")), &json!({"location_id": slug, "action": action}))
            .bearer(owner)
            .on(slug),
        &[("id", id)],
    )
}

#[test]
fn a_delivery_runs_from_assignment_to_the_door_and_the_cash_is_counted() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (rider, rider_id) = site.courier("alpha", &owner, "+355691111111");

    let placed = place_delivery(&site, "alpha", &dish);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = id_of(&placed);
    for a in ["confirm", "preparing", "ready"] {
        assert_eq!(act(&site, "alpha", &owner, &id, a).status_code(), 200, "{a}");
    }

    let r = site.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "shift: {}", r.body_str());

    let r = site.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id}))
            .bearer(&owner)
            .on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "assign: {}", r.body_str());

    let tasks = site.run(crate::courier::tasks, get(&at("alpha", "/api/courier/tasks")).bearer(&rider).on("alpha"), &[]);
    assert_eq!(tasks.status_code(), 200, "{}", tasks.body_str());
    assert!(tasks.body_str().contains(&id), "the rider sees the order: {}", tasks.body_str());

    for (route, want) in [("accept", None), ("pickup", Some("IN_DELIVERY"))] {
        let h = match route {
            "accept" => site.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
            _ => site.run(crate::courier::pickup, post(&at("alpha", &format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]),
        };
        assert_eq!(h.status_code(), 200, "{route}: {}", h.body_str());
        if let Some(s) = want {
            assert_eq!(status(&site, "alpha", &id), s, "{route}");
        }
    }
    let r = site.run(
        crate::courier::deliver,
        post(&at("alpha", &format!("/api/courier/orders/{id}/deliver")), &json!({"cash_collected": 900})).bearer(&rider).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "deliver: {}", r.body_str());
    assert_eq!(status(&site, "alpha", &id), "DELIVERED");

    let e = site.run(crate::courier::earnings, get(&at("alpha", "/api/courier/earnings")).bearer(&rider).on("alpha"), &[]);
    assert_eq!(e.status_code(), 200, "{}", e.body_str());
}

#[test]
fn a_courier_of_another_venue_cannot_see_take_or_move_the_order() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let other_owner = site.venue("beta", "b@x.test");
    let (stranger, stranger_id) = site.courier("beta", &other_owner, "+355692222222");
    let id = id_of(&place_delivery(&site, "alpha", &dish));
    assert_eq!(act(&site, "alpha", &owner, &id, "confirm").status_code(), 200);

    // alpha's owner cannot assign beta's courier.
    let r = site.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": stranger_id}))
            .bearer(&owner)
            .on("alpha"),
        &[("id", &id)],
    );
    assert!(r.status_code() >= 400, "another venue's courier was assigned: {}", r.body_str());

    // The stranger, on alpha's host, takes nothing: the token names beta.
    for (name, r) in [
        ("tasks", site.run(crate::courier::tasks, get(&at("alpha", "/api/courier/tasks")).bearer(&stranger).on("alpha"), &[])),
        ("accept", site.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&stranger).on("alpha"), &[("id", &id)])),
    ] {
        assert!(!r.body_str().contains("Rruga Tregtare"), "{name} leaked alpha's address: {}", r.body_str());
    }
    assert_eq!(status(&site, "alpha", &id), "CONFIRMED", "nothing moved");
}

#[test]
fn without_a_courier_token_every_courier_route_is_401() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let r = site.run(crate::courier::tasks, get(&at("alpha", "/api/courier/tasks")).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());
    let r = site.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());
    let owner = site.login("alpha", "a@x.test");
    let r = site.run(crate::courier::tasks, get(&at("alpha", "/api/courier/tasks")).bearer(&owner).on("alpha"), &[]);
    assert!(r.status_code() == 401 || r.status_code() == 403, "an owner token is not a courier's: {}", r.body_str());
}

/// A delivery to IN_DELIVERY, with the rider holding it: (owner, rider jwt, rider id, order id).
fn out_for_delivery(site: &Site) -> (String, String, String, String) {
    let (owner, dish) = open_venue(site, "alpha", "a@x.test");
    let (rider, rider_id) = site.courier("alpha", &owner, "+355691111111");
    let id = id_of(&place_delivery(site, "alpha", &dish));
    for a in ["confirm", "preparing", "ready"] {
        assert_eq!(act(site, "alpha", &owner, &id, a).status_code(), 200, "{a}");
    }
    site.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    site.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    site.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]);
    site.run(crate::courier::pickup, post(&at("alpha", &format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]);
    assert_eq!(status(site, "alpha", &id), "IN_DELIVERY");
    (owner, rider, rider_id, id)
}

#[test]
fn a_refusal_at_the_door_comes_back_and_the_owner_chooses_what_happens_to_the_food() {
    let site = Site::new();
    let (owner, rider, _, id) = out_for_delivery(&site);
    let r = site.run(
        crate::courier::door::refused,
        post(&at("alpha", &format!("/api/courier/orders/{id}/refused")), &json!({"note": "nobody home"})).bearer(&rider).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // Cash on delivery, nothing collected: no money to hand back, so both refund
    // edges run in this turn (command/refund.rs, "Nothing to hand back").
    assert_eq!(status(&site, "alpha", &id), "COMPENSATED_REFUND");
    let manager = site.staff("alpha", &owner, "m@x.test", "counter-manager");
    let returned = |choice: &str| {
        site.run(
            crate::services::orders::refund::returned,
            post(&at("alpha", &format!("/api/staff/orders/{id}/returned")), &json!({"location_id": "alpha", "choice": choice})).bearer(&manager).on("alpha"),
            &[("id", &id)],
        )
    };
    assert_eq!(returned("eat-it").status_code(), 400);
    // This venue has no recipes, so the kitchen drew no stock for the order and
    // there is no food to account for: every choice is refused, and says why.
    for choice in ["waste", "resell"] {
        let r = returned(choice);
        assert_eq!(r.status_code(), 409, "{choice}: {}", r.body_str());
        assert!(r.body_str().contains("took nothing"), "{choice}: {}", r.body_str());
    }
}

#[test]
fn a_rider_position_history_and_the_owners_view_of_the_rider() {
    let site = Site::new();
    let (owner, rider, rider_id, id) = out_for_delivery(&site);
    let r = site.run(
        crate::courier::position,
        post(&at("alpha", "/api/courier/position"), &json!({"lat": 41.3231, "lon": 19.4414, "order_id": id})).bearer(&rider).on("alpha"),
        &[],
    );
    assert!(r.status_code() < 500, "{}", r.body_str());
    let bad = site.run(crate::courier::position, post(&at("alpha", "/api/courier/position"), &json!({"lat": 41.0})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(bad.status_code(), 400);
    site.run(
        crate::courier::deliver,
        post(&at("alpha", &format!("/api/courier/orders/{id}/deliver")), &json!({"cash_collected": 900})).bearer(&rider).on("alpha"),
        &[("id", &id)],
    );
    let h = site.run(crate::services::courier::history::courier_history, get(&at("alpha", "/api/courier/history")).bearer(&rider).on("alpha"), &[]);
    assert_eq!(h.status_code(), 200, "{}", h.body_str());
    assert!(h.body_str().contains(&id), "{}", h.body_str());
    let list = site.run(crate::services::courier::console::couriers, get(&at("alpha", "/api/owner/couriers")).bearer(&owner).on("alpha"), &[]);
    assert!(list.body_str().contains(&rider_id), "{}", list.body_str());
    let d = site.run(
        crate::services::courier::console::courier_detail,
        get(&at("alpha", &format!("/api/owner/couriers/{rider_id}"))).bearer(&owner).on("alpha"),
        &[("id", &rider_id)],
    );
    assert_eq!(d.status_code(), 200, "{}", d.body_str());
    let b = site.venue("beta", "b@x.test");
    let d = site.run(
        crate::services::courier::console::courier_detail,
        get(&at("alpha", &format!("/api/owner/couriers/{rider_id}?location_id=alpha"))).bearer(&b).on("alpha"),
        &[("id", &rider_id)],
    );
    assert!(d.status_code() >= 400, "{}", d.body_str());
}

#[test]
fn a_switched_off_rider_cannot_sign_in_and_an_open_invite_can_be_withdrawn() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (_, rider_id) = site.courier("alpha", &owner, "+355694444444");
    let r = site.run(
        crate::services::courier::hiring::set_courier_active,
        post(&at("alpha", &format!("/api/owner/couriers/{rider_id}/active")), &json!({"active": false})).bearer(&owner).on("alpha"),
        &[("id", &rider_id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let login = site.run(
        crate::accounts::courier_login,
        post(&at("alpha", "/api/courier/auth/login"), &json!({"phone": "+355694444444", "password": "courier-password-1"})).on("alpha"),
        &[],
    );
    assert!(login.status_code() == 401 || login.status_code() == 403, "{}", login.body_str());
    let inv = site.run(
        crate::services::courier::hiring::invite_courier,
        post(&at("alpha", "/api/owner/couriers/invite"), &json!({"phone": "+355695555555", "name": "New"})).bearer(&owner).on("alpha"),
        &[],
    );
    assert_eq!(inv.status_code(), 200);
    let crew = crate::edge::mem::block_on(crate::identity_store::couriers(&site.env())).unwrap();
    let inv_id = crew.lookup(&crate::identity_store::invite_by_phone(&crate::auth::sha256_hex("+355695555555"))).expect("invite");
    let r = site.run(
        crate::services::courier::hiring::uninvite_courier,
        post(&at("alpha", &format!("/api/owner/couriers/{inv_id}/uninvite")), &json!({})).bearer(&owner).on("alpha"),
        &[("id", &inv_id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let code = inv.body_value()["code"].as_str().unwrap().to_string();
    let claim = site.run(
        crate::accounts::courier_claim,
        post(&at("alpha", "/api/courier/auth/claim"), &json!({"phone": "+355695555555", "code": code, "password": "courier-password-1"})).on("alpha"),
        &[],
    );
    assert_eq!(claim.status_code(), 400, "a withdrawn invite cannot be claimed: {}", claim.body_str());
}
