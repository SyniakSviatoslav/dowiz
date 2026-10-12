//! The delivery estimate through its route (W-COV C2): what a basket costs in minutes, the
//! journey on top for a delivery and none for a pickup, the queue counted from the orders still
//! in the kitchen -- and the refusals named rather than guessed around.

use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::{json, Value};

fn quote(site: &Site, body: Value) -> crate::wire::Reply {
    site.run(
        super::quote,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/public/locations/alpha/eta"), &body).on("alpha"),
        &[("slug", "alpha")],
    )
}

#[test]
fn a_basket_is_quoted_with_its_journey_and_the_kitchens_queue() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let delivery = quote(&site, json!({"items": [{"cookingMin": 12, "quantity": 2}], "distanceM": 3000}));
    assert_eq!(delivery.status_code(), 200, "{}", delivery.body_str());
    let d = delivery.body_value();
    assert!(d["parts"]["prepMin"].as_u64().unwrap() >= 12, "{d}");
    assert!(d["parts"]["travelMin"].as_u64().unwrap() > 0, "{d}");
    assert!(d["lowMin"].as_u64().unwrap() <= d["highMin"].as_u64().unwrap());
    assert_eq!((d["distanceM"].as_u64(), d["ordersAhead"].as_u64()), (Some(3000), Some(0)));

    let pickup = quote(&site, json!({"items": [{"cookingMin": 12, "quantity": 2}], "pickup": true})).body_value();
    assert_eq!(pickup["parts"]["travelMin"], 0, "{pickup}");
    assert!(pickup["highMin"].as_u64() < d["highMin"].as_u64(), "a pickup is quoted without the road: {pickup} vs {d}");

    // An order confirmed and in the kitchen is ahead of the next basket; a placed one is not yet.
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    assert_eq!(quote(&site, json!({"items": [{"id": dish}], "distanceM": 0})).body_value()["ordersAhead"], 0);
    let r = site.run(
        crate::owner::order_action,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/orders/{id}/action"), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let q = quote(&site, json!({"items": [{"id": dish}], "distanceM": 0})).body_value();
    assert_eq!(q["ordersAhead"], 1, "{q}");
    assert!(q["parts"]["queueMin"].as_u64().unwrap() > 0, "the queue costs minutes: {q}");

    // Refusals, each named.
    assert_eq!(quote(&site, json!({"items": []})).status_code(), 400, "an empty basket");
    assert_eq!(quote(&site, json!({"basket": []})).status_code(), 400, "no items field");
    let r = quote(&site, json!({"items": [{"cookingMin": 5}], "latUdeg": 41_323_100, "lonUdeg": 19_441_400}));
    assert_eq!(r.status_code(), 409, "a venue with no coordinates measures no distance: {}", r.body_str());
}

#[test]
fn a_venues_own_kitchen_settings_shape_the_profile_field_by_field() {
    let p = super::profile_of(&json!({"kitchenStations": 0, "courierSpeedMPerMin": 0, "pickupMin": 9}));
    let d = dowiz_kernel::eta::KitchenProfile::default_profile();
    assert_eq!((p.stations, p.courier_speed_m_per_min), (1, 1), "zero is floored to one, never a division by zero");
    assert_eq!(p.pickup_min, 9);
    assert_eq!(p.default_cooking_min, d.default_cooking_min, "an unset field keeps the default");
}

/// W-BN4B: A QUOTE IS ONE REQUEST TO THE VENUE'S OBJECT (and the registry's). Counted by the
/// test edge, both ways over the same venue: `quote_at` run by the Worker -- the route as it was
/// -- paid `/fold/products` and `/fold/orders` (every order folded to count the queue); the
/// route now hands the object the whole request. Same answer both ways, queue included.
#[test]
fn a_quote_costs_exactly_one_venue_object_request() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let r = site.run(
        crate::owner::order_action,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/orders/{id}/action"), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&owner).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let call = || post(&format!("https://alpha.{PLATFORM_HOST}/api/public/locations/alpha/eta"), &json!({"items": [{"id": dish}], "distanceM": 1200})).on("alpha");
    let paid = |f: &dyn Fn() -> crate::wire::Reply| {
        let from = crate::edge::mem::object_paths().len();
        let r = f();
        (crate::edge::mem::object_paths()[from..].to_vec(), r.body_value())
    };
    // The route as it was: the Worker resolves the slug and runs the body itself.
    let from_worker = |req: crate::wire::Call, ctx: crate::edge::Ctx<crate::Req>| async move {
        let place = crate::hubstore::Place::of_slug(&ctx, "alpha").await?;
        super::quote_at(place, req, ctx).await
    };
    let (before, was) = paid(&|| site.run(from_worker, call(), &[("slug", "alpha")]));
    let (after, is) = paid(&|| site.run(super::quote, call(), &[("slug", "alpha")]));
    eprintln!("COST quote before {before:?} after {after:?}");
    // The venue's own object only: the registry is the platform's (and W-CHEAP's slug memo may skip it).
    let venue = |p: &[String]| p.iter().filter(|p| crate::storefront::route_costs::is_venue(p)).cloned().collect::<Vec<_>>();
    assert_eq!(venue(&after), ["/fold/command"], "the quote is one turn (before: {before:?})");
    assert_eq!(venue(&before), ["/fold/products", "/fold/orders"], "what it was");
    assert_eq!(is, was, "the same quote both ways");
    assert_eq!(is["ordersAhead"], 1, "the queue is still counted: {is}");
}
