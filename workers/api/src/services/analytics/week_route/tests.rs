//! The week's badge through the routes: a guest's plates reach the public
//! answer at the threshold, a TEST order's never do, and the owner's
//! analytics pane carries the same number for the same dish.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::services::analytics::week_top::THRESHOLD;
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn week(site: &Site) -> serde_json::Value {
    let r = site.run(super::week, get(&at("/api/public/locations/alpha/menu/week")).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

#[test]
fn the_badge_counts_real_plates_from_the_threshold_and_never_test_ones() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    assert_eq!(week(&site)["dishes"], json!([]), "no orders, no badge");
    // A TEST order of many plates lifts nothing.
    let r = site.run(
        crate::storefront::place,
        post(&at("/api/public/locations/alpha/orders"), &json!({"items": [{"product_id": dish, "quantity": THRESHOLD + 5}],
            "contact": {"name": "LIVE-wk guest", "phone": "+355690000019"}, "fulfilment": {"kind": "pickup"}, "payment": "cash"})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(week(&site)["dishes"], json!([]), "a TEST order never wears the badge");
    // One plate short of the threshold: still nothing.
    assert_eq!(place_pickup(&site, "alpha", &dish, THRESHOLD - 1).status_code(), 200);
    assert_eq!(week(&site)["dishes"], json!([]));
    assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    let w = week(&site);
    assert_eq!(w["dishes"], json!([{ "id": dish, "n": THRESHOLD }]), "{w}");
    assert_eq!(w["contract"], "menu.week-top.v1");
    // THE PANE AND THE BADGE: the owner's analytics (v2) carry the same n, and the test plates beside it.
    let a = site.run(crate::services::analytics::analytics, get(&at("/api/owner/analytics?location_id=alpha&v=2")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(a.status_code(), 200, "{}", a.body_str());
    let wt = &a.body_value()["weekTop"];
    let row = wt["dishes"].as_array().and_then(|d| d.iter().find(|x| x["id"] == json!(dish)).cloned()).unwrap_or_else(|| panic!("{wt}"));
    assert_eq!((row["n"].as_i64(), row["test"].as_i64(), row["badge"].as_bool()), (Some(THRESHOLD), Some(THRESHOLD + 5), Some(true)), "{wt}");
    assert_eq!((wt["from"].clone(), wt["to"].clone()), (w["from"].clone(), w["to"].clone()));
}
