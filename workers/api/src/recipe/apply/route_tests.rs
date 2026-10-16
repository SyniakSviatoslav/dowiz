//! A REFUSED RECIPE LINE THROUGH THE DISH SHEET'S ROUTE (2026-10-16): every
//! `set_bom` refusal reaches the owner as a 400 with `{error: "<supply>:
//! <reason>"}` -- it was a bare 500 for all but "unknown supply" -- and the
//! refused save writes nothing.

use crate::edge::mem::block_on;
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// The venue with one supply, its owner's token and its dish.
fn venue(site: &Site) -> (String, String) {
    let (t, dish) = open_venue(site, "alpha", "alpha@example.com");
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    block_on(crate::hubstore::with_catalog(&place, |c| {
        c.set_supply("salmon", &json!({ "id": "salmon", "name": "Salmon", "unit": "g", "kind": "food_ingredient", "costPerBasis": 240 }).to_string());
        Ok(())
    }))
    .expect("supply");
    (t, dish)
}

fn stored(site: &Site, dish: &str) -> Value {
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    let j = block_on(crate::hubstore::with_catalog(&place, |c| Ok(c.product(dish)))).expect("read").expect("dish");
    serde_json::from_str(&j).expect("json")
}

fn save(site: &Site, t: &str, dish: &str, bom: Value) -> crate::wire::Reply {
    let call = post(&at(&format!("/api/owner/products/{dish}")), &json!({ "location_id": "alpha", "bom": bom })).bearer(t).on("alpha");
    site.run(crate::owner::update_product, call, &[("id", dish)])
}

#[test]
fn a_refused_line_is_a_400_naming_the_supply_and_writes_nothing() {
    let site = Site::new();
    let (t, dish) = venue(&site);
    let r = save(&site, &t, &dish, json!([{ "supply": "salmon", "qty": 35 }]));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let before = stored(&site, &dish);
    for (bom, why) in [
        (json!([{ "supply": "salmon", "qty": 0 }]), "more than 0"),
        (json!([{ "supply": "salmon", "qty": 100_001 }]), "at most 100000"),
        (json!([{ "supply": "salmon", "qty": 60_000 }, { "supply": "salmon", "qty": 40_001 }]), "100000"),
        (json!([{ "supply": "salmon", "qty": 100, "net": 101 }]), "more than the gross"),
        (json!([{ "supply": "salmon", "qty": 100, "out": -1 }]), "out is 0 to"),
    ] {
        let r = save(&site, &t, &dish, bom.clone());
        assert_eq!(r.status_code(), 400, "{bom}: {}", r.body_str());
        let e = r.body_value()["error"].as_str().unwrap_or_default().to_string();
        assert!(e.starts_with("salmon: ") && e.contains(why), "{bom}: {e:?}");
        assert_eq!(stored(&site, &dish), before, "{bom}: nothing written");
    }
    // Too many decimals: the body's own reader refuses it with a 400 too.
    let r = save(&site, &t, &dish, json!([{ "supply": "salmon", "qty": "0.0001" }]));
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    assert_eq!(stored(&site, &dish), before);
    // TWIN: the same supply at a good quantity lands.
    let r = save(&site, &t, &dish, json!([{ "supply": "salmon", "qty": 50 }]));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(stored(&site, &dish)["bom"], json!([{ "supply": "salmon", "qty": 50 }]));
}

/// ITEM 7, THE EXACT REPRO: `POST /api/owner/products/:id?location_id=alpha`
/// with `{"bom": [...]}` answered `400 bad request body: missing field
/// location_id`. And the query and the body naming different venues is a 400.
#[test]
fn the_venue_may_ride_in_the_query_and_a_mismatch_is_refused() {
    let site = Site::new();
    let (t, dish) = venue(&site);
    let call = |q: &str, body: Value| post(&at(&format!("/api/owner/products/{dish}{q}")), &body).bearer(&t).on("alpha");
    let r = site.run(crate::owner::update_product, call("?location_id=alpha", json!({ "bom": [{ "supply": "salmon", "qty": 35 }] })), &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(stored(&site, &dish)["bom"], json!([{ "supply": "salmon", "qty": 35 }]));
    let r = site.run(crate::owner::update_product, call("?location_id=beta", json!({ "location_id": "alpha", "bom": [{ "supply": "salmon", "qty": 40 }] })), &[("id", &dish)]);
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    assert!(r.body_str().contains("location_id in the query and the body differ"), "{}", r.body_str());
    assert_eq!(stored(&site, &dish)["bom"], json!([{ "supply": "salmon", "qty": 35 }]), "nothing written");
}
