//! `GET /api/order/:id`, through the route seam (W-COV C2): the customer with the key minted at
//! placement, the venue's owner, and nobody else -- and the kitchen's cost stripped for the guest.

use crate::edge::site::{get, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};

fn url(id: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}/api/order/{id}")
}

#[test]
fn an_order_is_read_by_its_key_and_its_venue_and_by_nobody_else() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let v = place_pickup(&site, "alpha", &dish, 1).body_value();
    let (id, key) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap().to_string());

    let r = site.run(super::order, get(&url(&id)).bearer(&key).on("alpha"), &[("id", &id)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["id"], id.as_str());
    assert_eq!(r.headers().get("cache-control").unwrap().as_deref(), Some("private, no-store"));

    let r = site.run(super::order, get(&url(&id)).bearer(&t).on("alpha"), &[("id", &id)]);
    assert_eq!(r.status_code(), 200, "the venue's owner reads it: {}", r.body_str());

    let anon = site.run(super::order, get(&url(&id)).on("alpha"), &[("id", &id)]);
    assert_eq!(anon.status_code(), 401, "{}", anon.body_str());

    // Another order's key does not open this one.
    let other = place_pickup(&site, "alpha", &dish, 1).body_value();
    let r = site.run(super::order, get(&url(&id)).bearer(other["access_token"].as_str().unwrap()).on("alpha"), &[("id", &id)]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());

    // Beta's owner, on alpha's host: not this venue's.
    let b = site.venue("beta", "b@x.test");
    let r = site.run(super::order, get(&url(&id)).bearer(&b).on("alpha"), &[("id", &id)]);
    assert!(r.status_code() == 401 || r.status_code() == 404, "{}", r.body_str());

    let r = site.run(super::order, get(&url("nope")).bearer(&t).on("alpha"), &[("id", "nope")]);
    assert_eq!(r.status_code(), 404);
    let r = site.run(super::order, get(&url(&id)).bearer(&key).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400, "no id in the path");
}
