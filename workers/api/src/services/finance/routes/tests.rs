//! The money routes a second time through the real handlers: a form sent twice is one entry,
//! and the month the client described comes to its exact net.

use super::super::routes;
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::{json, Value};

fn at(slug: &str, p: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{p}")
}

fn add(site: &Site, owner: &str, slug: &str, body: Value) -> crate::wire::Reply {
    site.run(routes::add, post(&at(slug, &format!("/api/owner/finance/expenses?location_id={slug}")), &body).bearer(owner).on(slug), &[])
}

fn read(site: &Site, owner: &str, slug: &str, month: &str) -> Value {
    let r = site.run(routes::finance, get(&at(slug, &format!("/api/owner/finance?location_id={slug}&month={month}"))).bearer(owner).on(slug), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn fuel(id: Option<&str>, amount: i64) -> Value {
    let mut b = json!({"date": "2023-11-03", "name": "Бензин", "amount": amount, "kind": "other"});
    if let Some(id) = id {
        b["client_id"] = json!(id);
    }
    b
}

/// THE DOUBLE TAP: the same form twice is one row and one amount; the answer names the entry
/// already filed. A different form is a second entry, and so is a body with no form id.
#[test]
fn a_form_sent_twice_is_one_entry_and_another_form_is_another() {
    let site = Site::new();
    let (a, _) = open_venue(&site, "alpha", "a@x.test");
    let first = add(&site, &a, "alpha", fuel(Some("lk3x9q0ab1"), 500));
    assert_eq!(first.status_code(), 200, "{}", first.body_str());
    let v1 = first.body_value();
    assert_eq!((v1["id"].clone(), v1["repeated"].clone()), (json!("clk3x9q0ab1"), json!(false)));
    let again = add(&site, &a, "alpha", fuel(Some("lk3x9q0ab1"), 500)).body_value();
    assert_eq!((again["id"].clone(), again["repeated"].clone()), (json!("clk3x9q0ab1"), json!(true)));
    // A copy that says something else does not rewrite the entry: an edit has its own route.
    let other = add(&site, &a, "alpha", fuel(Some("lk3x9q0ab1"), 999)).body_value();
    assert_eq!(other["repeated"], true);
    let v = read(&site, &a, "alpha", "2023-11");
    assert_eq!(v["expenses"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!((v["other"]["total"].clone(), v["profit"]["minus"]["total"].clone()), (json!(500), json!(500)), "500 once, not 1000 and not 999");

    assert_eq!(add(&site, &a, "alpha", fuel(Some("zz77zz77"), 500)).body_value()["repeated"], false);
    assert_eq!(add(&site, &a, "alpha", fuel(None, 500)).status_code(), 200);
    assert_eq!(add(&site, &a, "alpha", fuel(None, 500)).status_code(), 200);
    let v = read(&site, &a, "alpha", "2023-11");
    assert_eq!((v["expenses"].as_array().unwrap().len(), v["other"]["total"].clone()), (4, json!(2000)), "{v}");
}

#[test]
fn a_form_id_that_is_not_letters_and_digits_is_refused_and_writes_nothing() {
    let site = Site::new();
    let (a, _) = open_venue(&site, "alpha", "a@x.test");
    let long = "a".repeat(41);
    for bad in ["", "short12", "UPPERCASE1", "has-dash-1", "../../etc1", "with space", "кирилиця1", long.as_str()] {
        let r = add(&site, &a, "alpha", fuel(Some(bad), 500));
        assert_eq!(r.status_code(), 400, "{bad:?}: {}", r.body_str());
    }
    assert_eq!(read(&site, &a, "alpha", "2023-11")["expenses"], json!([]));
    assert_eq!(add(&site, &a, "alpha", fuel(Some(&"a".repeat(40)), 500)).status_code(), 200);
    assert_eq!(add(&site, &a, "alpha", fuel(Some("abcd1234"), 500)).status_code(), 200);
}

/// One venue's form id is nothing to another venue: the same id there files that venue's own entry.
#[test]
fn the_same_form_id_in_two_venues_is_two_venues_entries() {
    let site = Site::new();
    let (a, _) = open_venue(&site, "alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    assert_eq!(add(&site, &a, "alpha", fuel(Some("sameid0001"), 500)).body_value()["repeated"], false);
    let r = add(&site, &b, "beta", fuel(Some("sameid0001"), 70));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["repeated"], false, "beta never filed this form");
    assert_eq!(read(&site, &a, "alpha", "2023-11")["other"]["total"], 500);
    assert_eq!(read(&site, &b, "beta", "2023-11")["other"]["total"], 70);
}

/// THE MONTH AS THE CLIENT DESCRIBED IT, through the real routes: sales in the plus; a priced
/// delivery, a tax, "Бензин 500", "Прикраси на Хеловін 3200" and an employee's hours in the
/// minus, each once.
#[test]
fn salaries_taxes_other_costs_and_purchases_are_all_in_the_minus_each_once() {
    use crate::services::operations::payroll::{month_route, set_entry, set_rate};
    let site = Site::new();
    let (a, dish) = open_venue(&site, "alpha", "a@x.test");
    let ok = |r: crate::wire::Reply| {
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        r
    };
    let order = ok(place_pickup(&site, "alpha", &dish, 2)).body_value()["id"].as_str().unwrap().to_string();
    ok(site.run(crate::owner::order_action, post(&at("alpha", &format!("/api/owner/orders/{order}/action")), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&a).on("alpha"), &[("id", &order)]));
    ok(site.run(crate::services::operations::supplies::set_supply, post(&at("alpha", "/api/owner/supplies"), &json!({"id": "salmon", "name": "salmon", "unit": "g"})).bearer(&a).on("alpha"), &[]));
    ok(site.run(crate::services::operations::stock::stock_move, post(&at("alpha", "/api/owner/stock/received"), &json!({"item": "salmon", "qty": 1000, "total": 1200})).bearer(&a).on("alpha"), &[("kind", "received")]));
    ok(add(&site, &a, "alpha", json!({"date": "2023-11-10", "name": "ПДВ", "amount": 700, "kind": "tax"})));
    ok(add(&site, &a, "alpha", json!({"date": "2023-11-03", "name": "Бензин", "amount": 500, "kind": "other"})));
    ok(add(&site, &a, "alpha", json!({"date": "2023-11-01", "name": "Прикраси на Хеловін", "amount": 3200, "kind": "other"})));
    let (_, ira) = site.staff_full("alpha", &a, "ira@x.test", "waiter");
    ok(site.run(set_rate, post(&at("alpha", "/api/owner/payroll/rate?location_id=alpha"), &json!({"staff_id": ira, "rate_minor": 250})).bearer(&a).on("alpha"), &[]));
    ok(site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry?location_id=alpha"), &json!({"staff_id": ira, "date": "2023-11-13", "minutes": 450})).bearer(&a).on("alpha"), &[]));

    let v = read(&site, &a, "alpha", "2023-11");
    let p = &v["profit"];
    let minus: Vec<(String, i64)> = p["minus"]["rows"].as_array().unwrap().iter().map(|r| (r["key"].as_str().unwrap().to_string(), r["amount"].as_i64().unwrap())).collect();
    assert_eq!(minus, vec![("purchases".to_string(), 1200), ("taxes".to_string(), 700), ("other".to_string(), 3700), ("payroll".to_string(), 1875)], "{p}");
    assert_eq!(p["plus"]["total"], 1800, "{p}");
    assert_eq!(p["minus"]["total"], 7475, "1200 + 700 + 3700 + 1875");
    assert_eq!(p["net"], -5675, "1800 - 7475");
    assert_eq!(p["minus"]["rows"][0]["undated"], 0, "every delivery here has its date");
    // The payroll row is the payroll tab's own total for the month.
    let pay = ok(site.run(month_route, get(&at("alpha", "/api/owner/payroll?location_id=alpha&month=2023-11")).bearer(&a).on("alpha"), &[])).body_value();
    assert_eq!(pay["totalMinor"], 1875);
    let names: Vec<&str> = v["expenses"].as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["ПДВ", "Бензин", "Прикраси на Хеловін"], "plain lines, newest first");
}
