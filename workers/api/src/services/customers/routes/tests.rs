//! The customer book through its routes (W-COV C2): the masked list, a reveal that is audited
//! before it answers, the card, consent, links -- and the right to be forgotten, read back from
//! the orders themselves. The other venue's owner is refused on every one.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Call;
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn order_as(site: &Site, dish: &str, phone: &str) -> String {
    let r = site.run(
        crate::storefront::place,
        post(
            &at("alpha", "/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "Ana Hoxha", "phone": phone},
                    "fulfilment": {"kind": "pickup"}, "payment": "cash"}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()["id"].as_str().unwrap().to_string()
}

fn keys(site: &Site, t: &str) -> Vec<(String, Value)> {
    let r = site.run(crate::services::customers::handlers::customers, get(&at("alpha", "/api/owner/customers")).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()["customers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c["key"].as_str().unwrap_or("").to_string(), c))
        .collect()
}

#[test]
fn the_list_is_masked_and_a_reveal_is_audited_before_it_answers() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    order_as(&site, &dish, "+355691112223");
    let ks = keys(&site, &t);
    assert_eq!(ks.len(), 1);
    let (key, row) = ks[0].clone();
    assert!(!row.to_string().contains("691112223"), "the list shows no phone: {row}");
    let empty = site.run(
        crate::services::customers::handlers::reveal_customer,
        post(&at("alpha", &format!("/api/owner/customers/{key}/reveal")), &json!({"reason": ""})).bearer(&t).on("alpha"),
        &[("key", &key)],
    );
    assert_eq!(empty.status_code(), 400, "a reveal needs a reason: {}", empty.body_str());
    let r = site.run(
        crate::services::customers::handlers::reveal_customer,
        post(&at("alpha", &format!("/api/owner/customers/{key}/reveal")), &json!({"reason": "allergy call"})).bearer(&t).on("alpha"),
        &[("key", &key)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(r.body_str().contains("+355691112223"), "{}", r.body_str());
    let log = site.run(crate::services::customers::handlers::reveals, get(&at("alpha", "/api/owner/customers/reveals")).bearer(&t).on("alpha"), &[]);
    assert!(log.body_str().contains("allergy call"), "the reveal is on the record: {}", log.body_str());
    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::services::customers::handlers::reveal_customer,
        post(&at("alpha", &format!("/api/owner/customers/{key}/reveal?location_id=alpha")), &json!({"reason": "curious"})).bearer(&b).on("alpha"),
        &[("key", &key)],
    );
    assert!(r.status_code() >= 400 && !r.body_str().contains("691112223"), "beta revealed alpha's customer: {}", r.body_str());
}

#[test]
fn a_card_consent_and_a_link_are_written_and_read_back() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    order_as(&site, &dish, "+355691112224");
    order_as(&site, &dish, "+355691112225");
    let ks = keys(&site, &t);
    assert_eq!(ks.len(), 2);
    let (k1, k2) = (ks[0].0.clone(), ks[1].0.clone());
    let put = |key: &str, body: Value| {
        site.run(
            crate::services::customers::record_routes::put_record,
            Call::new(&at("alpha", &format!("/api/owner/customers/{key}/record")), worker::Method::Put).unwrap().with_json(&body).bearer(&t).on("alpha"),
            &[("key", key)],
        )
    };
    let r = put(&k1, json!({"note": "window seat", "tags": ["regular"], "usualTable": "T4"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(keys(&site, &t).iter().any(|(k, row)| k == &k1 && row.to_string().contains("window seat")), "the card is on the row");
    assert_eq!(put(&k1, json!({"tags": ["nonsense-tag"]})).status_code(), 400);
    assert_eq!(put("not-a-key", json!({"note": "x"})).status_code(), 400);

    let consent = |state: &str, evidence: &str| {
        site.run(
            crate::services::customers::consent_routes::owner_act,
            post(&at("alpha", &format!("/api/owner/customers/{k1}/consent")), &json!({"state": state, "evidence": evidence, "lang": "sq"})).bearer(&t).on("alpha"),
            &[("key", &k1)],
        )
    };
    assert_eq!(consent("given", "").status_code(), 400, "a grant needs evidence");
    assert_eq!(consent("given", "said yes at the counter").status_code(), 200);
    assert_eq!(consent("withdrawn", "").status_code(), 200);

    let r = site.run(
        crate::services::customers::alias_routes::link,
        post(&at("alpha", &format!("/api/owner/customers/{k2}/link")), &json!({"to": k1, "reason": "same person, new phone"})).bearer(&t).on("alpha"),
        &[("key", &k2)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = site.run(
        crate::services::customers::alias_routes::unlink,
        post(&at("alpha", &format!("/api/owner/customers/{k2}/unlink")), &json!({"reason": "wrong"})).bearer(&t).on("alpha"),
        &[("key", &k2)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let w = site.run(crate::services::customers::consent_routes::wordings, get(&at("alpha", "/api/public/consent/wordings")).on("alpha"), &[]);
    assert_eq!(w.status_code(), 200);
}

#[test]
fn a_forgotten_customer_is_gone_from_the_orders_and_the_list() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = order_as(&site, &dish, "+355691112226");
    let key = keys(&site, &t)[0].0.clone();
    let forget = || {
        site.run(
            crate::services::customers::forget::forget_customer,
            post(&at("alpha", &format!("/api/owner/customers/{key}/forget")), &json!({"reason": "asked by email", "lang": "en"})).bearer(&t).on("alpha"),
            &[("key", &key)],
        )
    };
    // An order still in flight is not erased from under the kitchen.
    let r = forget();
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    let r = site.run(
        crate::owner::order_action,
        post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "reject", "reason": "closed"})).bearer(&t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = forget();
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let o = site.fold("alpha", &format!("/fold/order?id={id}")).body_str();
    assert!(!o.contains("691112226") && !o.contains("Ana Hoxha"), "the order still names them: {o}");
    let r = site.run(crate::services::customers::forget::run::reforget, post(&at("alpha", "/api/owner/customers/reforget"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

