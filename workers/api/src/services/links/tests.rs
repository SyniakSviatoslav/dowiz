//! THE LINKS BETWEEN SYSTEMS, EACH DRIVEN FROM ITS PRODUCER TO ITS CONSUMER (lane W-INT1,
//! 2026-10-02). One test per row of `tools/integrations/matrix.json` that had no route test
//! reading the far end: the row's number is in the test's name, and the matrix names the test
//! as the row's proof (`tools/gates/integration-matrix.sh` refuses a TESTED-INMEM row whose test is
//! gone). Every step goes through the real handler the router would call, on the in-memory
//! platform (`edge::site`), and every assertion is on what the CONSUMER reads -- "the write
//! answered 200" proves nothing about the screen at the other end.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn id_of(r: &crate::wire::Reply) -> String {
    let v = r.body_value();
    ["id", "order_id", "orderId"]
        .iter()
        .find_map(|k| v[*k].as_str().or(v["order"][*k].as_str()).map(str::to_string))
        .unwrap_or_else(|| panic!("no id in {}", r.body_str()))
}

fn stored_order(site: &Site, slug: &str, id: &str) -> Value {
    let o = site.fold(slug, &format!("/fold/order?id={id}")).body_value();
    serde_json::from_str(o["order_json"].as_str().unwrap_or("{}")).unwrap_or_default()
}

fn act(site: &Site, slug: &str, owner: &str, id: &str, action: &str) {
    let r = site.run(
        crate::owner::order_action,
        post(&at(slug, &format!("/api/owner/orders/{id}/action")), &json!({"location_id": slug, "action": action}))
            .bearer(owner)
            .on(slug),
        &[("id", id)],
    );
    assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
}

/// The ids `/api/owner/health` lists under `kitchen.unseen`, as the console's health pane reads them.
fn unseen(site: &Site, owner: &str) -> Vec<String> {
    let h = site.run(crate::services::operations::health, get(&at("alpha", "/api/owner/health")).bearer(owner).on("alpha"), &[]);
    assert_eq!(h.status_code(), 200, "health: {}", h.body_str());
    let v = h.body_value();
    let list = v["kitchen"]["unseen"].as_array().unwrap_or_else(|| panic!("no kitchen.unseen in health: {v}"));
    list.iter().filter_map(|u| u["orderId"].as_str().map(str::to_string)).collect()
}

/// ROW 3. The kitchen's "seen" (`admin/kitchen.js` -> `POST /api/staff/orders/:id/kitchen-ack`)
/// is what the owner's health reads: a ticket older than `UNSEEN_AFTER_MS` with no ack is listed,
/// and the kitchen's ack takes it off -- through the order log, not through a flag on the side.
#[test]
fn link03_a_kitchen_ack_takes_the_ticket_off_the_owners_unseen_list() {
    let mut site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let cook = site.staff("alpha", &owner, "k@x.test", "kitchen");
    let id = id_of(&place_pickup(&site, "alpha", &dish, 1));
    let young = unseen(&site, &owner);
    assert!(!young.contains(&id), "a ticket placed this second is not late yet: {young:?}");

    site.now_ms += crate::command::kitchen_ack::UNSEEN_AFTER_MS + 1_000;
    let late = unseen(&site, &owner);
    assert!(late.contains(&id), "a late, unacknowledged ticket is on the owner's list: {late:?}");

    let ack = |site: &Site| {
        site.run(
            crate::services::orders::kitchen_ack::kitchen_ack,
            post(&at("alpha", &format!("/api/staff/orders/{id}/kitchen-ack")), &json!({"location_id": "alpha"})).bearer(&cook).on("alpha"),
            &[("id", &id)],
        )
    };
    let r = ack(&site);
    assert_eq!(r.status_code(), 200, "ack: {}", r.body_str());
    assert_eq!(r.body_value()["fresh"], json!(true), "{}", r.body_str());
    assert!(stored_order(&site, "alpha", &id).pointer("/kitchen/seen/at").is_some(), "the ack is on the order in the log");
    let after = unseen(&site, &owner);
    assert!(!after.contains(&id), "the owner still reads the acked ticket as unseen: {after:?}");
    // The twin: a second ack writes nothing and the answer says so.
    assert_eq!(ack(&site).body_value()["fresh"], json!(false));
}

fn login_courier(site: &Site, host: &str, body: Value) -> crate::wire::Reply {
    site.run(crate::accounts::courier_login, post(&at(host, "/api/courier/auth/login"), &body).on(host), &[])
}

/// ROW 15. The HOST decides the courier's venue (`accounts::venue_of_host`): the session minted on
/// alpha's subdomain is alpha's, the same courier on beta's subdomain is refused, and a body that
/// names another venue than the host is a contradiction, not a preference.
#[test]
fn link15_the_host_decides_which_venue_a_courier_signs_in_to() {
    let site = Site::new();
    let alpha = site.venue("alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    let (_, rider) = site.courier("alpha", &alpha, "+355695555555");
    let creds = json!({"phone": "+355695555555", "password": "courier-password-1"});

    let ok = login_courier(&site, "alpha", creds.clone());
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
    let v = ok.body_value();
    assert_eq!((v["courier"]["id"].as_str(), v["courier"]["locationId"].as_str()), (Some(rider.as_str()), Some("alpha")), "{v}");
    // The session works where it was minted: the courier's own task list answers.
    let jwt = v["jwt"].as_str().unwrap().to_string();
    let tasks = site.run(crate::courier::tasks, get(&at("alpha", "/api/courier/tasks")).bearer(&jwt).on("alpha"), &[]);
    assert_eq!(tasks.status_code(), 200, "{}", tasks.body_str());

    let elsewhere = login_courier(&site, "beta", creds.clone());
    assert_eq!(elsewhere.status_code(), 403, "alpha's courier signed in on beta's host: {}", elsewhere.body_str());
    let mut contradiction = creds.clone();
    contradiction["location_id"] = json!("beta");
    let r = login_courier(&site, "alpha", contradiction);
    assert_eq!(r.status_code(), 403, "a body naming beta on alpha's host: {}", r.body_str());
}

fn place_delivery(site: &Site, dish: &str) -> crate::wire::Reply {
    site.run(
        crate::storefront::place,
        post(
            &at("alpha", "/api/public/locations/alpha/orders"),
            &json!({
                "items": [{"product_id": dish, "quantity": 1}],
                "contact": {"name": "Guest", "phone": "+355690000002"},
                "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Tregtare 1, Durres", "lat_udeg": 41_312_000, "lon_udeg": 19_446_000}},
                "payment": "cash",
            }),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    )
}

/// The customer's own read of their order (`store/track.js` -> `GET /api/order/:id`).
fn customer_reads(site: &Site, id: &str, key: &str) -> Value {
    let r = site.run(crate::services::orders::read::order, get(&at("alpha", &format!("/api/order/{id}"))).bearer(key).on("alpha"), &[("id", id)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

/// ROW 17. A courier's position (`courier/app.js` -> `POST /api/courier/position`) reaches the
/// CUSTOMER'S tracking answer as `eta.courierAt`, once the courier carries their order -- and
/// not before (a courier's whereabouts on someone else's run are not the customer's to see).
#[test]
fn link17_a_couriers_position_reaches_the_customers_tracking_answer() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let (rider, rider_id) = site.courier("alpha", &owner, "+355691111111");
    let placed = place_delivery(&site, &dish);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = id_of(&placed);
    let key = placed.body_value()["access_token"].as_str().expect("the customer's order key").to_string();
    for a in ["confirm", "preparing", "ready"] {
        act(&site, "alpha", &owner, &id, a);
    }
    let r = site.run(crate::courier::shift, post(&at("alpha", "/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "shift: {}", r.body_str());
    let r = site.run(
        crate::owner::assign_courier,
        post(&at("alpha", &format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "assign: {}", r.body_str());
    let position = || {
        site.run(
            crate::courier::position,
            post(&at("alpha", "/api/courier/position"), &json!({"lat": 41.3231, "lon": 19.4414, "order_id": id})).bearer(&rider).on("alpha"),
            &[],
        )
    };
    let r = position();
    assert_eq!(r.status_code(), 200, "position: {}", r.body_str());
    let before = customer_reads(&site, &id, &key);
    // The estimate is there (the door has a pin -- without one there is no live estimate at all,
    // and a null `courierAt` would prove nothing); only the courier is withheld.
    assert!(before["eta"].is_object(), "no live estimate for a pinned delivery: {before}");
    assert!(before["eta"]["courierAt"].is_null(), "the courier is shown before they carry this order: {}", before["eta"]);

    let accepted = site.run(crate::courier::accept, post(&at("alpha", &format!("/api/courier/orders/{id}/accept")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]);
    assert_eq!(accepted.status_code(), 200, "accept: {}", accepted.body_str());
    let picked = site.run(crate::courier::pickup, post(&at("alpha", &format!("/api/courier/orders/{id}/pickup")), &json!({})).bearer(&rider).on("alpha"), &[("id", &id)]);
    assert_eq!(picked.status_code(), 200, "pickup: {}", picked.body_str());
    assert_eq!(position().status_code(), 200);
    let live = customer_reads(&site, &id, &key);
    assert_eq!(live["status"], "IN_DELIVERY", "{live}");
    let fix = &live["eta"]["courierAt"];
    assert_eq!((fix["latUdeg"].as_i64(), fix["lonUdeg"].as_i64()), (Some(41_323_100), Some(19_441_400)), "the customer's map has no courier: {}", live["eta"]);
    assert_eq!(live["eta"]["courierId"].as_str(), Some(rider_id.as_str()));
}

/// The dish as the console's recipe sheet reads it (`admin/menu.js storedBom` ->
/// `GET /api/owner/products?id=`).
fn owner_dish(site: &Site, owner: &str, dish: &str) -> Value {
    let r = site.run(
        crate::services::catalogue::import::bulk::owner_products,
        get(&at("alpha", &format!("/api/owner/products?id={dish}"))).bearer(owner).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    v["products"].as_array().and_then(|a| a.iter().find(|p| p["id"].as_str() == Some(dish)).cloned()).unwrap_or_else(|| panic!("{dish} not in {v}"))
}

fn supply(site: &Site, owner: &str, cost: i64) {
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("alpha", "/api/owner/supplies"), &json!({"id": "salmon", "name": "Salmon", "unit": "g", "costPerBasis": cost, "lowAt": 100})).bearer(owner).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "supply: {}", r.body_str());
}

/// ROW 21. A recipe saved on a dish (`POST /api/owner/products/:id` with `bom`) comes back to the
/// recipe sheet with its line named and costed FROM TODAY'S SUPPLY (`recipe::hydrate`): a supply
/// re-priced after the dish was saved moves the dish's cost on the next read, nothing re-saved.
#[test]
fn link21_a_saved_recipe_is_read_back_derived_from_todays_supplies() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    supply(&site, &owner, 180);
    let r = site.run(
        crate::owner::update_product,
        post(&at("alpha", &format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "bom": [{"supply": "salmon", "qty": 100}]})).bearer(&owner).on("alpha"),
        &[("id", &dish)],
    );
    assert_eq!(r.status_code(), 200, "recipe: {}", r.body_str());

    let p = owner_dish(&site, &owner, &dish);
    let line = &p["bom"][0];
    assert_eq!((line["supply"].as_str(), line["qty"].as_i64(), line["name"].as_str()), (Some("salmon"), Some(100), Some("Salmon")), "{p}");
    let first = p["cost"].as_i64().unwrap_or_else(|| panic!("no derived cost: {p}"));
    assert!(first > 0, "{p}");

    supply(&site, &owner, 360);
    let again = owner_dish(&site, &owner, &dish);
    assert_eq!(again["cost"].as_i64(), Some(first * 2), "the cost did not follow the supply: {again}");
}
