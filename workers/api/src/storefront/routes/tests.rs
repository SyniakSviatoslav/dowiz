//! A whole order through the routes (W-COV C2): the owner builds a menu and opens, a guest
//! places through the storefront, the owner sees and accepts it -- and the other venue's owner
//! can do none of that to it.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

/// A venue with one category, one dish at 900 and pickup open; returns (owner token, dish id).
pub(crate) fn open_venue(site: &Site, slug: &str, email: &str) -> (String, String) {
    let t = site.venue(slug, email);
    let r = site.run(
        crate::catalog_edit::set_category,
        post(&at(slug, "/api/owner/categories"), &json!({"location_id": slug, "name": "Rolls"})).bearer(&t).on(slug),
        &[],
    );
    assert_eq!(r.status_code(), 200, "category: {}", r.body_str());
    let cat = r.body_value()["id"].as_str().map(str::to_string).unwrap_or_else(|| panic!("category id: {}", r.body_str()));
    let r = site.run(
        crate::catalog_edit::create_product,
        post(
            &at(slug, "/api/owner/products"),
            &json!({"location_id": slug, "category_id": cat, "name": "Futomaki", "price": 900}),
        )
        .bearer(&t)
        .on(slug),
        &[],
    );
    assert_eq!(r.status_code(), 200, "product: {}", r.body_str());
    let dish = r.body_value()["id"].as_str().map(str::to_string).unwrap_or_else(|| panic!("dish id: {}", r.body_str()));
    // On sale only once its allergens are declared (an empty list is a declaration).
    let r = site.run(
        crate::owner::update_product,
        post(&at(slug, &format!("/api/owner/products/{dish}")), &json!({"location_id": slug, "allergens": [], "available": true}))
            .bearer(&t)
            .on(slug),
        &[("id", &dish)],
    );
    assert_eq!(r.status_code(), 200, "on sale: {}", r.body_str());
    let r = site.run(
        crate::owner::update_location,
        post(&at(slug, "/api/owner/location"), &json!({"location_id": slug, "status": "open", "pickup": true}))
            .bearer(&t)
            .on(slug),
        &[],
    );
    assert_eq!(r.status_code(), 200, "open: {}", r.body_str());
    (t, dish)
}

pub(crate) fn place_pickup(site: &Site, slug: &str, dish: &str, qty: i64) -> crate::wire::Reply {
    site.run(
        crate::storefront::place,
        post(
            &at(slug, &format!("/api/public/locations/{slug}/orders")),
            &json!({
                "items": [{"product_id": dish, "quantity": qty}],
                "contact": {"name": "Guest", "phone": "+355690000001"},
                "fulfilment": {"kind": "pickup"},
                "payment": "cash",
            }),
        )
        .on(slug),
        &[("slug", slug)],
    )
}

fn order_id(r: &crate::wire::Reply) -> String {
    let v = r.body_value();
    for k in ["id", "order_id", "orderId"] {
        if let Some(s) = v[k].as_str() {
            return s.to_string();
        }
        if let Some(s) = v["order"][k].as_str() {
            return s.to_string();
        }
    }
    panic!("no order id in {}", r.body_str())
}

#[test]
fn a_guest_order_reaches_the_owner_priced_by_the_catalogue_and_is_accepted() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let placed = place_pickup(&site, "alpha", &dish, 2);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = order_id(&placed);

    let r = site.run(crate::owner::orders, get(&at("alpha", "/api/owner/orders")).bearer(&t).on("alpha"), &[]);
    let list = r.body_value();
    let orders = list["orders"].as_array().cloned().unwrap_or_default();
    assert_eq!(orders.len(), 1, "{}", r.body_str());
    assert_eq!(orders[0]["id"], json!(id));
    assert_eq!(orders[0]["subtotal"], json!(1800), "2 x 900 from the catalogue, not from the basket");

    let r = site.run(
        crate::owner::order_action,
        post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "confirm"}))
            .bearer(&t)
            .on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let stored = site.fold("alpha", &format!("/fold/order?id={id}")).body_value();
    let state: Value = serde_json::from_str(stored["order_json"].as_str().unwrap_or("{}")).unwrap();
    assert_eq!(state["status"], "CONFIRMED", "{stored}");

    // The rest of a pickup's life, each step read back from the log.
    for (action, status) in [("preparing", "PREPARING"), ("ready", "READY"), ("collected", "PICKED_UP")] {
        let r = site.run(
            crate::owner::order_action,
            post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": action}))
                .bearer(&t)
                .on("alpha"),
            &[("id", &id)],
        );
        assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
        let stored = site.fold("alpha", &format!("/fold/order?id={id}")).body_value();
        let state: Value = serde_json::from_str(stored["order_json"].as_str().unwrap_or("{}")).unwrap();
        assert_eq!(state["status"], status, "{action}");
    }
    // A finished order cannot be cancelled: the kernel's refusal, not a 200.
    let r = site.run(
        crate::owner::order_action,
        post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "cancel"}))
            .bearer(&t)
            .on("alpha"),
        &[("id", &id)],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

#[test]
fn a_rejection_carries_its_reason_into_the_log() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = order_id(&place_pickup(&site, "alpha", &dish, 1));
    let r = site.run(
        crate::owner::order_action,
        post(
            &at("alpha", &format!("/api/owner/orders/{id}/action")),
            &json!({"location_id": "alpha", "action": "reject", "reason": "the kitchen is closed"}),
        )
        .bearer(&t)
        .on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let stored = site.fold("alpha", &format!("/fold/order?id={id}")).body_str();
    assert!(stored.contains("REJECTED") && stored.contains("the kitchen is closed"), "{stored}");
}

/// THE OTHER VENUE'S OWNER cannot move alpha's order: naming alpha is refused (not a member),
/// naming beta finds no such order -- and alpha's log does not move either way.
#[test]
fn another_venues_owner_cannot_act_on_the_order() {
    let site = Site::new();
    let (_t, dish) = open_venue(&site, "alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    let id = order_id(&place_pickup(&site, "alpha", &dish, 1));
    let before = site.fold("alpha", "/fold/generation").body_value();
    for loc in ["alpha", "beta"] {
        let r = site.run(
            crate::owner::order_action,
            post(&at("beta", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": loc, "action": "confirm"}))
                .bearer(&b)
                .on("beta"),
            &[("id", &id)],
        );
        assert!(r.status_code() >= 400, "beta's owner naming {loc} was answered {}: {}", r.status_code(), r.body_str());
    }
    assert_eq!(site.fold("alpha", "/fold/generation").body_value(), before, "alpha's log did not move");
}

#[test]
fn a_closed_venue_an_empty_basket_and_an_unknown_kind_are_refused_before_anything_is_written() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let url = at("alpha", "/api/public/locations/alpha/orders");
    let before = site.fold("alpha", "/fold/generation").body_value();
    let empty = site.run(
        crate::storefront::place,
        post(&url, &json!({"items": [], "contact": {"phone": ""}, "fulfilment": {"kind": "pickup"}})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(empty.status_code(), 400);
    let banana = site.run(
        crate::storefront::place,
        post(&url, &json!({"items": [{"product_id": "x", "quantity": 1}], "contact": {"phone": ""}, "fulfilment": {"kind": "banana"}}))
            .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(banana.status_code(), 400);
    let closed = site.run(
        crate::storefront::place,
        post(&url, &json!({"items": [{"product_id": "x", "quantity": 1}], "contact": {"phone": ""}, "fulfilment": {"kind": "pickup"}}))
            .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(closed.status_code(), 409, "a new venue is closed: {}", closed.body_str());
    assert_eq!(site.fold("alpha", "/fold/generation").body_value(), before, "no order was logged");
}
