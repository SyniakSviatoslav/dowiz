//! THE BAG INSERT IN MEMORY (tested in-memory, NOT proven live -- the live
//! proof is `tools/live-proof/probes/feature-bag-qr.mjs`, run by main on
//! qa-durres after deploy): the owner sets the welcome through the route, a
//! guest lands with `src=bag`, places through the real `storefront::place` and
//! the venue's real object, then tries again with the same phone.
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Reply;
use serde_json::{json, Value};

const GUEST: &str = "+355690000002";
/// The QR the card draws for `alpha` and campaign `spring`, byte for byte.
/// `workers/api/public/admin/bag-qr.test.mjs` DECODES this same file with an
/// independent decoder and demands exactly the landing URL; this test demands
/// the route still draws exactly this file. Together: the printed code opens
/// the venue's own host and nothing else.
const FIXTURE: &str = include_str!("qr-alpha-spring.svg");
const FIXTURE_PATH: &str = "src/services/loyalty/bag_routes/qr-alpha-spring.svg";

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn q(path: &str, extra: &str) -> String {
    at("alpha", &format!("{path}?location_id=alpha{extra}"))
}

fn set(s: &Site, owner: &str, body: Value) -> Reply {
    s.run(super::set, post(&q("/api/owner/bag", ""), &body).bearer(owner).on("alpha"), &[])
}

fn card(s: &Site, owner: &str, c: &str) -> Value {
    let r = s.run(super::card, get(&q("/api/owner/bag", &format!("&c={c}"))).bearer(owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn place(s: &Site, dish: &str, src: Option<Value>, phone: &str) -> Reply {
    let mut body = json!({
        "items": [{"product_id": dish, "quantity": 1}],
        "contact": {"name": "Arben Hoxha", "phone": phone},
        "fulfilment": {"kind": "pickup"},
        "payment": "cash",
    });
    if let Some(b) = src {
        body["src"] = b;
    }
    s.run(crate::storefront::place, post(&at("alpha", "/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")])
}

fn bag() -> Option<Value> {
    Some(json!({"src": "bag", "c": "spring"}))
}

fn public(s: &Site) -> Value {
    s.run(super::public, get(&at("alpha", "/api/public/locations/alpha/welcome")).on("alpha"), &[("slug", "alpha")]).body_value()
}

#[test]
fn a_bag_guest_gets_the_welcome_once_per_phone_and_the_card_counts_it() {
    let s = Site::new();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    assert_eq!(public(&s), json!({"on": false}), "no offer until the owner sets one");
    let r = set(&s, &owner, json!({"offer": {"kind": "fixed", "value": 300, "min": 0}, "commission_pct": "30"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(public(&s), json!({"on": true, "kind": "fixed", "value": 300, "min": 0}));

    let first = place(&s, &dish, bag(), GUEST);
    assert_eq!(first.status_code(), 200, "{}", first.body_str());
    let f = first.body_value();
    assert_eq!(f["welcome"], json!({"kind": "fixed", "discount": 300, "c": "spring"}), "{f}");
    assert_eq!(f["referral"], json!({"src": "bag", "c": "spring"}));
    assert_eq!(f["total"], 600);
    assert_eq!(f["items"][0]["unit_price"], 900, "the menu price is the menu price");

    // RED PROOF 6.1, through the real routes: the same phone, again.
    let second = place(&s, &dish, bag(), GUEST);
    assert_eq!(second.status_code(), 200, "the ORDER is placed: {}", second.body_str());
    let v = second.body_value();
    assert_eq!((v["welcome_refused"].as_str(), v.get("welcome")), (Some("used"), None), "{v}");
    assert_eq!(v["total"], 900);

    // TWIN: another phone from the bag is welcome; a guest who did not land from a bag gets nothing.
    let other = place(&s, &dish, bag(), "+355690000003").body_value();
    assert_eq!(other["total"], 600);
    let direct = place(&s, &dish, None, "+355690000004").body_value();
    assert!(direct.get("welcome").is_none() && direct.get("referral").is_none(), "{direct}");

    let c = card(&s, &owner, "spring");
    assert_eq!(c["url"], format!("https://alpha.{PLATFORM_HOST}/?src=bag&c=spring"));
    assert_eq!(c["stats"]["orders"], 3);
    assert_eq!(c["stats"]["guests"], 2);
    assert_eq!(c["stats"]["repeat"], 1);
    assert_eq!(c["stats"]["direct_total"], 600 + 900 + 600);
    assert_eq!(c["stats"]["saved"], (600 + 900 + 600) * 30 / 100);
    assert_eq!(c["stats"]["scans"], Value::Null);
    assert_eq!(c["commission_pct"], 30);
}

/// RED PROOF 6.4 (Rust half): the QR the route draws is the fixture the node
/// test decodes. `BAG_QR_BLESS=1` rewrites the fixture -- by hand, never in CI.
#[test]
fn the_printed_code_is_the_decoded_fixture() {
    let s = Site::new();
    let (owner, _) = open_venue(&s, "alpha", "a@x.test");
    let r = s.run(super::qr, get(&q("/api/owner/bag/qr.svg", "&c=spring")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    if std::env::var("BAG_QR_BLESS").as_deref() == Ok("1") {
        std::fs::write(FIXTURE_PATH, r.body_str()).unwrap();
    }
    assert_eq!(r.body_str(), FIXTURE, "the route's QR moved; the node decoder must re-check it");
    assert_eq!(card(&s, &owner, "spring")["svg"], FIXTURE);
}

/// Each refusal names itself; the twin is the happy path above.
#[test]
fn the_doors_refuse_what_they_cannot_keep() {
    let s = Site::new();
    let (owner, dish) = open_venue(&s, "alpha", "a@x.test");
    let bad = |b: Value| set(&s, &owner, b).status_code();
    assert_eq!(bad(json!({"offer": {"kind": "fixed", "value": 300}, "until": 1})), 400, "unknown field");
    assert_eq!(bad(json!({"offer": {"kind": "fixed", "value": 0}})), 400);
    assert_eq!(bad(json!({"offer": {"kind": "stamps"}})), 400, "no stamp card");
    assert_eq!(bad(json!({"offer": {"kind": "gift", "product": "nobody"}})), 400, "not on the menu");
    assert_eq!(bad(json!({"offer": {"kind": "fixed", "value": 300}, "commission_pct": "22.5"})), 400);
    assert_eq!(bad(json!({"offer": {"kind": "gift", "product": dish}})), 200);
    assert_eq!(public(&s)["gift"]["name"], "Futomaki");
    let r = s.run(super::card, get(&q("/api/owner/bag", "&c=Bad%20Word")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    let r = s.run(super::card, get(&q("/api/owner/bag", "")).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "no token, no card");
    assert_eq!(place(&s, &dish, Some(json!({"src": "wolt"})), GUEST).status_code(), 400, "a word this build does not know");
    assert_eq!(place(&s, &dish, Some(json!({"src": "bag", "fp": "x"})), GUEST).status_code(), 400, "strict");
    // The gift: on the house when it is in the basket.
    let g = place(&s, &dish, bag(), GUEST).body_value();
    assert_eq!((g["discount"].as_i64(), g["total"].as_i64()), (Some(900), Some(0)), "{g}");
}
