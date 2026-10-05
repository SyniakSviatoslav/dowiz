//! The owner's segment builder through its route (W-SENSE row 7): guests who ordered a smoky dish
//! are counted, the plan names the dish, the closed shape refuses anything else, and only the
//! owner may ask.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}
fn place(site: &Site, dish: &str, phone: &str) {
    let body = json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "Guest", "phone": phone},
        "fulfilment": {"kind": "pickup"}, "payment": "cash"});
    let r = site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}
fn ask(site: &Site, t: &str, q: &str) -> crate::wire::Reply {
    site.run(super::builder, get(&at(&format!("/api/owner/customers/taste/builder?location_id=alpha&{q}"))).bearer(t).on("alpha"), &[])
}

#[test]
fn guests_who_order_a_smoky_dish_are_counted_and_the_plan_names_it() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(crate::owner::update_product,
        post(&at(&format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "sense": {"aroma": {"smoky": 3}, "texture": {"crispy": 2}}})).bearer(&t).on("alpha"),
        &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    place(&site, &dish, "+355690000041");
    place(&site, &dish, "+355690000042");
    let r = ask(&site, &t, "key=a:smoky&min=500");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["contract"], "customers.taste-builder.v1");
    assert_eq!(v["count"], 2, "{v}");
    assert_eq!(v["plan"]["dishes"][0]["id"], dish.as_str());
    assert_eq!(v["plan"]["dishes"][0]["share"], 1000);
    assert!(v["busiest"].is_string(), "the orders were filed with the venue's moment: {v}");
    assert_eq!(v["trend"].as_array().map(Vec::len), Some(6));
    assert!(v.to_string().find("+35569").is_none(), "never a person: {v}");
    // Not seen for 4 days: they ordered today.
    assert_eq!(ask(&site, &t, "key=a:smoky&not_seen_days=4").body_value()["count"], 0);
    // A key nobody has.
    assert_eq!(ask(&site, &t, "key=t:sweet").body_value()["count"], 0);
}

#[test]
fn the_closed_shape_refuses_allergens_health_and_unknown_words() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    for q in ["key=allergen:gluten", "key=t:richness", "key=a:smoky&health=diabetic", "key=a:smoky&min=x", "key=a:smoky&band=brunch", ""] {
        let r = ask(&site, &t, q);
        assert_eq!(r.status_code(), 400, "{q}: {}", r.body_str());
    }
    assert_eq!(ask(&site, &t, "key=x:creamy").status_code(), 200, "positive twin");
}

#[test]
fn another_venue_and_the_anonymous_cannot_ask() {
    let site = Site::new();
    open_venue(&site, "alpha", "a@x.test");
    let (other, _) = open_venue(&site, "beta", "b@x.test");
    assert!(ask(&site, &other, "key=a:smoky").status_code() >= 400);
    let anon = site.run(super::builder, get(&at("/api/owner/customers/taste/builder?location_id=alpha&key=a:smoky")).on("alpha"), &[]);
    assert!(anon.status_code() >= 400);
}

#[test]
fn the_plan_takes_strong_available_dishes_and_their_supplies() {
    let products = vec![
        ("d1".to_string(), json!({"name": "Smoked eel", "sense": {"v": 1, "aroma": {"smoky": 3}}, "bom": [{"supply": "s1", "qty": 80}]}).to_string()),
        ("d2".to_string(), json!({"name": "Faint", "sense": {"v": 1, "aroma": {"smoky": 1}}}).to_string()),
        ("d3".to_string(), json!({"name": "Off", "available": false, "sense": {"v": 1, "aroma": {"smoky": 3}}}).to_string()),
        ("d4".to_string(), json!({"name": "Nothing declared"}).to_string()),
    ];
    let p = super::plan("a:smoky", &products, |s| (s == "s1").then(|| "Eel fillet".to_string()));
    assert_eq!(p["dishes"].as_array().unwrap().len(), 1, "{p}");
    assert_eq!(p["dishes"][0]["id"], "d1");
    assert_eq!(p["supplies"], json!(["Eel fillet"]));
}
