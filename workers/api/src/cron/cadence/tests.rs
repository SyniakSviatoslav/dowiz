//! W-LOOP row 1: THE MINUTE LOOP (docs/research/2026-10-03-hub-cost-recomputed.md §1.3). One venue's
//! alarm fired 1,449 times a day, 24 h: its till link was usable, it had no opening hours on file,
//! and `sched.is_empty() || is_open_at(..)` read "no hours" as "always open" -- so the link was
//! polled every minute, all night, with nothing to read. These tests run a whole venue day through
//! the REAL alarm against a till that never sells anything, and count the firings.

use crate::edge::mem::{answer_outbound, block_on};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

const DAY: i64 = 86_400_000;

fn json_reply(v: &Value) -> worker::Result<Reply> {
    let mut r = Reply::from_json(v)?;
    r.headers_mut().set("content-type", "application/json")?;
    Ok(r)
}

/// A till that is up and never sells anything: an empty floor, an empty list, an empty menu.
/// (Telegram, when a test also drains a message, accepts it.)
pub(crate) fn idle_till() {
    answer_outbound(|c: &Call| {
        let url = c.url()?;
        if url.host_str() == Some("api.telegram.org") {
            return Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}}));
        }
        assert_eq!(url.host_str(), Some("www.ebills.al"), "only ebills is called: {url}");
        match (c.method(), url.path()) {
            (worker::Method::Get, "/") => {
                let mut r = Reply::ok("<html></html>")?;
                r.headers_mut().set("content-type", "text/html")?;
                r.headers_mut().append("set-cookie", "XSRF-TOKEN=x1; Path=/")?;
                Ok(r)
            }
            (worker::Method::Post, "/api/authentication") => {
                let mut r = json_reply(&json!({}))?;
                r.headers_mut().append("set-cookie", "JSESSIONID=s1; Path=/; HttpOnly")?;
                r.headers_mut().set("x-tenant-identifier", "t-1")?;
                Ok(r)
            }
            (worker::Method::Get, "/api/sale-units-tables") => json_reply(&json!([])),
            (worker::Method::Get, "/api/sales") => json_reply(&json!({"sales": [], "total": 0.0})),
            (worker::Method::Get, "/api/item-in-sales") => json_reply(&json!([])),
            (m, p) => panic!("an unexpected call: {m:?} {p}"),
        }
    });
}

/// The owner saves a usable till link; the venue has NO opening hours on file.
pub(crate) fn link_without_hours(site: &Site, slug: &str) -> String {
    let (t, _) = open_venue(site, slug, &format!("{slug}@x.test"));
    let r = site.run(
        crate::ebills::routes::config,
        post(&format!("https://{slug}.{PLATFORM_HOST}/api/owner/ebills/config"), &json!({"enabled": true, "pos_id": 1, "user": "till@x.test", "password": "pw"}))
            .bearer(&t)
            .on(slug),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    t
}

/// Fire the venue's alarm, as the platform would, until a day has passed; the firings.
fn alarms_in_a_day(site: &Site, slug: &str) -> Vec<i64> {
    let host = site.world.host(slug);
    let start = host.alarm.get().expect("the saved link armed the alarm");
    let mut fired = Vec::new();
    while let Some(at) = host.alarm.get() {
        if at >= start + DAY {
            break;
        }
        host.alarm.set(None); // the platform deletes a fired alarm before it runs the handler
        block_on(site.object(slug).timer_alarm_in(&site.env(), at)).expect("the alarm ran");
        fired.push(at);
        assert!(fired.len() <= 3_000, "the alarm runs away");
    }
    fired
}

/// THE ROOT CAUSE, as a number: an idle link at a venue with no hours on file wakes the venue
/// every minute of the day. The bound is the backoff's (1, 2, 5, then 15 minutes inside the
/// storefront's fallback hours, the closed half-hour list outside them).
#[test]
fn an_idle_link_at_a_venue_with_no_hours_does_not_poll_every_minute() {
    let site = Site::new();
    link_without_hours(&site, "alpha");
    idle_till();
    let fired = alarms_in_a_day(&site, "alpha");
    eprintln!("alarms in one idle day, no hours on file: {}", fired.len());
    assert!(fired.len() <= 100, "{} alarms in one idle day (the minute loop is 1,440)", fired.len());
    let gaps: Vec<i64> = fired.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().filter(|g| **g <= 60_000).count() <= 4, "minute gaps: {gaps:?}");
}
