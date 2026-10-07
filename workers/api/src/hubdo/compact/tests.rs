//! The v2 pin and the compaction route, with a stand-in compactor: this tree has no v3 writer
//! (W-DELTA's `Catalog::compact` is wired by main), so "v3" here is bytes starting `V3`, and
//! compacting one turns the prefix into `V2`. What is under test is the ROUTE: the pin, the
//! generation-guarded write, the per-image answer, the fan-out and who may call it.

use super::super::host::mem::Harness;
use super::FAKE;
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::wire::Call;
use worker::Method;

fn fake(_id: &str, bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    Ok(bytes.strip_prefix(b"V3").map(|rest| [b"V2".as_slice(), rest].concat()))
}

fn with_fake<R>(f: impl FnOnce() -> R) -> R {
    FAKE.with(|c| c.set(Some(fake)));
    let r = f();
    FAKE.with(|c| c.set(None));
    r
}

fn compact(h: &Harness, query: &str) -> serde_json::Value {
    let r = h.call(Call::new(&format!("https://hub/fold/compact{query}"), Method::Post).unwrap());
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

#[test]
fn compaction_rewrites_a_v3_catalogue_and_the_pin_keeps_later_writes_v2_until_lifted() {
    with_fake(|| {
        let h = Harness::new();
        assert_eq!(h.put("catalog", 0, b"V3menu").status_code(), 200, "unpinned: stored as sent");
        assert_eq!(h.cold().get("/img/catalog").body(), b"V3menu");

        let v = compact(&h, "");
        assert_eq!(v["pinned"], true);
        assert_eq!(v["images"][0]["compacted"], true, "{v}");
        assert_eq!(v["images"][0]["generation"], 2);
        let cold = h.cold().get("/img/catalog");
        assert_eq!((Harness::gen_of(&cold), cold.body()), (2, b"V2menu".as_slice()));

        // PINNED: a Worker that still writes v3 is stored v2.
        assert_eq!(h.put("catalog", 2, b"V3more").status_code(), 200);
        assert_eq!(h.cold().get("/img/catalog").body(), b"V2more");
        // Only KV images: `settings` is not one, and is stored as sent.
        assert_eq!(h.put("settings", 0, b"V3s").status_code(), 200);
        assert_eq!(h.cold().get("/img/settings").body(), b"V3s");

        // Twin: a second compaction finds v2 and writes nothing.
        let v = compact(&h, "");
        assert_eq!((v["images"][0]["compacted"].clone(), v["images"][0]["generation"].clone()), (false.into(), 3.into()));

        // Lifted: written as sent again, and nothing is compacted by the lift.
        assert_eq!(compact(&h, "?pin=0")["images"], serde_json::json!([]));
        assert_eq!(h.put("catalog", 3, b"V3back").status_code(), 200);
        assert_eq!(h.cold().get("/img/catalog").body(), b"V3back");
    });
}

#[test]
fn a_venue_without_a_catalogue_answers_absent() {
    let h = Harness::new();
    let v = compact(&h, "");
    assert_eq!(v["images"][0]["absent"], true, "{v}");
}

/// THE FAN-OUT: every venue's object, administrators only, and a failure is a 502 naming it.
#[test]
fn the_platform_compacts_every_venue_for_an_administrator_and_refuses_an_owner() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    let url = format!("https://{PLATFORM_HOST}/api/platform/compact");
    let body = serde_json::json!({});

    let r = site.run(super::fan::platform_compact, post(&url, &body).bearer(&owner), &[]);
    assert_eq!(r.status_code(), 404, "an owner is not an administrator: {}", r.body_str());

    let r = site.run(super::fan::platform_compact, post(&url, &body).bearer(&site.admin_token()), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["failed"], 0);
    let venues: Vec<String> = v["venues"].as_array().unwrap().iter().map(|x| x["venue"].as_str().unwrap().to_string()).collect();
    assert_eq!(venues.len(), 3, "alpha, beta and the unnamed object: {venues:?}");
    assert!(venues.iter().all(|n| site.world.host(n).kv.borrow().contains_key(super::PIN_KEY)), "every object pinned");

    // One venue's storage cannot answer: 502, and that venue is named.
    let sick = venues.iter().find(|n| n.as_str() != crate::hubstore::UNNAMED_VENUE).unwrap().clone();
    site.world.restart(&sick);
    site.world.host(&sick).reads_fail.set(true);
    let r = site.run(super::fan::platform_compact, post(&url, &body).bearer(&site.admin_token()), &[]);
    assert_eq!(r.status_code(), 502, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["failed"], 1);
    assert!(v["venues"].as_array().unwrap().iter().any(|x| x["venue"] == sick.as_str() && x["status"] != 200));
}
