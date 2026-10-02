//! The night and the minute, run natively against a world of venues (W-COV C2): the nightly
//! re-arms an alarm that was lost while work was due, the venue's runner drains its outbox, and
//! the cloud nightly copies a configured venue to its bucket and writes the witness census to
//! the platform object -- a second night contradicts nothing.

use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Reply;
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn place(site: &Site, venue: &str) -> crate::hubstore::Place {
    crate::hubstore::Place::of_authorised(&site.ctx(&[]), venue).unwrap()
}

fn setting(site: &Site, t: &str, key: &str, value: &str) {
    let r = site.run(crate::services::venue::settings::set_setting, post(&at("/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{key}: {}", r.body_str());
}

#[test]
fn a_lost_alarm_is_rearmed_by_the_nightly_and_the_runner_drains_the_venue() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    setting(&site, &t, "notify.telegram.token", "123:abc");
    let now = site.now_ms;
    let e = crate::outbox::Entry::new("w-1:telegram".into(), "telegram", "-4242".into(), "a test line".into(), now);
    block_on(crate::outbox::enqueue(&place(&site, "alpha"), &[e])).unwrap();
    let host = site.world.host("alpha");
    assert!(host.alarm.get().is_some(), "a write with work due arms the alarm");

    // The alarm is lost; the nightly finds work due and re-arms it. Beta has nothing due.
    host.alarm.set(None);
    block_on(crate::cron::nightly(&site.env(), now));
    assert!(host.alarm.get().is_some_and(|a| a <= now + 60_000), "{:?}", host.alarm.get());
    assert!(site.world.host("beta").alarm.get().is_none(), "nothing due, nothing set");

    // The alarm fires: the runner's jobs run for this venue and the message goes out once.
    answer_outbound(|_| Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}})));
    block_on(crate::cron::run(&site.env(), "alpha", now + 60_000));
    let to: Vec<String> = sent()
        .iter()
        .filter(|c| c.url().unwrap().path().ends_with("/sendMessage"))
        .map(|c| serde_json::from_slice::<serde_json::Value>(&c.body_bytes()).unwrap()["chat_id"].to_string())
        .collect();
    assert_eq!(to.len(), 1, "{to:?}");
    assert!(to[0].contains("-4242"));
    assert!(block_on(crate::outbox::waiting(&place(&site, "alpha"))).unwrap().is_empty(), "the drained entry is gone");
    // A second run sends nothing more.
    block_on(crate::cron::run(&site.env(), "alpha", now + 120_000));
    assert_eq!(sent().iter().filter(|c| c.url().unwrap().path().ends_with("/sendMessage")).count(), 1);
}

#[test]
fn the_night_copies_a_configured_venue_and_its_witness_agrees_with_the_next_night() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    for _ in 0..3 {
        assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    }
    for (k, v) in [("cloud.s3.endpoint", "https://s3.example"), ("cloud.s3.bucket", "alpha-copies"), ("cloud.s3.key", "AKIA"), ("cloud.s3.secret", "shh")] {
        setting(&site, &t, k, v);
    }
    answer_outbound(|_| {
        let mut r = Reply::empty().unwrap();
        r.headers_mut().set("etag", "\"e1\"").unwrap();
        Ok(r)
    });
    let night = site.now_ms + 3_600_000;
    block_on(crate::cloud::nightly(&site.env(), night));
    let puts: Vec<String> = sent().iter().filter(|c| c.method() == worker::Method::Put).map(|c| c.url().unwrap().to_string()).collect();
    assert!(!puts.is_empty() && puts.iter().all(|u| u.contains("alpha-copies")), "only alpha has a bucket: {puts:?}");

    let ns = site.env().durable_object("HUB").unwrap();
    let census = block_on(crate::witness::last(&ns, "alpha")).unwrap().expect("alpha's census was written");
    assert_eq!(census.venue, "alpha");
    assert!(census.records >= 3, "{census:?}");
    assert!(block_on(crate::witness::last(&ns, "beta")).unwrap().is_some(), "every venue is counted, bucket or not");

    // One more order and a second night: the census grows, nothing is contradicted.
    assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    block_on(crate::cloud::nightly(&site.env(), night + 86_400_000));
    let next = block_on(crate::witness::last(&ns, "alpha")).unwrap().unwrap();
    assert!(next.records > census.records, "{next:?} vs {census:?}");
}

#[test]
fn finished_history_past_thirty_days_leaves_the_hot_log_for_an_archive_the_witness_then_checks() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let done = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let live = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let r = site.run(
        crate::owner::order_action,
        post(&at(&format!("/api/owner/orders/{done}/action")), &json!({"location_id": "alpha", "action": "reject"})).bearer(&t).on("alpha"),
        &[("id", &done)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let p = place(&site, "alpha");

    // Inside thirty days nothing moves.
    let soon = block_on(crate::hubstore::rotate(&p, site.now_ms + 86_400_000)).unwrap();
    assert_ne!(soon["rotated"], true, "{soon}");

    // Past it: the rejected order goes to the archive, the pending one stays hot.
    let later = site.now_ms + crate::hubstore::HOT_KEEP_MS + 86_400_000;
    let v = block_on(crate::hubstore::rotate(&p, later)).unwrap();
    assert_eq!(v["rotated"], true, "{v}");
    assert_eq!((v["ordersMoved"].as_u64(), v["ordersKept"].as_u64()), (Some(1), Some(1)), "{v}");
    assert!(v["eventsAfter"].as_u64() < v["eventsBefore"].as_u64(), "{v}");
    let archive = v["archive"].as_str().unwrap().to_string();
    let hot: Vec<String> = block_on(crate::hubstore::orders(&p)).unwrap().into_iter().map(|o| o.order_id).collect();
    assert!(hot.contains(&live) && !hot.contains(&done), "{hot:?}");
    let (records, tip) = block_on(crate::hubstore::archive_seal(&p, &archive)).unwrap().expect("the archive is readable");
    assert!(records > 0 && tip.is_some());
    assert!(block_on(crate::hubstore::archive_holds(&p, &archive, tip.as_deref().unwrap())).unwrap());
    assert!(!block_on(crate::hubstore::archive_holds(&p, &archive, "00")).unwrap());
    assert_eq!(block_on(crate::hubstore::archive_seal(&p, "not-an-archive")).unwrap(), None);

    // Two nights later: the witness seals the archive once and nothing is contradicted.
    answer_outbound(|_| Reply::from_json(&json!({})));
    block_on(crate::cloud::nightly(&site.env(), later + 86_400_000));
    block_on(crate::cloud::nightly(&site.env(), later + 2 * 86_400_000));
    let ns = site.env().durable_object("HUB").unwrap();
    let census = block_on(crate::witness::last(&ns, "alpha")).unwrap().expect("census");
    assert_eq!(census.archived(), records, "{census:?}");
}
