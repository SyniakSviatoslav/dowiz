//! The kitchen printer through its routes (W-COV C2): a venue key, a job queued by a placement,
//! polled, fetched, acknowledged -- and a key of another venue that finds nothing.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Call;
use serde_json::json;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn key_for(site: &Site, owner: &str, slug: &str) -> String {
    let r = site.run(
        crate::services::identity::keys::create_api_key,
        post(&at(slug, "/api/owner/apikeys"), &json!({"label": "printer"})).bearer(owner).on(slug),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    ["key", "apiKey", "token", "secret"].iter().find_map(|k| v[*k].as_str()).unwrap().to_string()
}

#[test]
fn a_placed_order_prints_once_and_is_acknowledged() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("alpha", "/api/owner/settings"), &json!({"key": crate::print_rail::SETTING, "value": "epson-1"})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let key = key_for(&site, &t, "alpha");
    let poll = |k: &str| site.run(crate::services::orders::print::poll, post(&at("alpha", "/api/print/poll"), &json!({})).bearer(k), &[]);

    assert_eq!(poll(&key).body_value()["jobReady"], false, "nothing to print yet");
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let p = poll(&key);
    assert_eq!(p.body_value()["jobReady"], true, "{}", p.body_str());
    let token = p.body_value()["jobToken"].as_str().unwrap().to_string();

    let job = site.run(crate::services::orders::print::job, get(&at("alpha", &format!("/api/print/job/{token}"))).bearer(&key), &[("token", &token)]);
    assert_eq!(job.status_code(), 200, "{}", job.body_str());
    assert!(!job.body().is_empty());
    let jobs = site.run(crate::services::orders::print::jobs, get(&at("alpha", "/api/owner/print/jobs")).bearer(&t).on("alpha"), &[]);
    assert!(jobs.body_str().contains(&id), "{}", jobs.body_str());

    let ack = site.run(
        crate::services::orders::print::ack,
        Call::new(&at("alpha", &format!("/api/print/job/{token}?code=200")), worker::Method::Delete).unwrap().bearer(&key),
        &[("token", &token)],
    );
    assert_eq!(ack.status_code(), 200, "{}", ack.body_str());
    assert_eq!(poll(&key).body_value()["jobReady"], false, "an acknowledged job is not handed out again");

    // No key, a junk key, another venue's key: nothing.
    assert_eq!(site.run(crate::services::orders::print::poll, post(&at("alpha", "/api/print/poll"), &json!({})), &[]).status_code(), 401);
    assert_eq!(poll("dowiz_junk").status_code(), 401);
    let b = site.venue("beta", "b@x.test");
    let bkey = key_for(&site, &b, "beta");
    let other = site.run(crate::services::orders::print::job, get(&at("alpha", &format!("/api/print/job/{token}"))).bearer(&bkey), &[("token", &token)]);
    assert!(other.status_code() >= 400, "beta's printer fetched alpha's ticket: {}", other.body_str());
}
