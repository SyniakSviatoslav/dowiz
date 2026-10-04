//! THE TILL LINK END TO END, natively (W-COV C2): the owner configures it through the route, the
//! venue's timer fires a tick, the poll logs in to a fake ebills.al (the front page's XSRF cookie,
//! then the one POST), reads the floor, the sale list, each sale and the till's menu, and the
//! venue's object imports what it read. Each refusal the platform can give is obeyed: a lapsed
//! session is re-established once, a second factor halts the link, a page instead of data is named.

use crate::ebills::tests::{bill_listed, course, wrapped};
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};
use std::cell::Cell;
use std::rc::Rc;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn json_reply(v: &Value) -> worker::Result<Reply> {
    let mut r = Reply::from_json(v)?;
    r.headers_mut().set("content-type", "application/json")?;
    Ok(r)
}

/// ebills.al as measured (EBILLS blueprint §1): `logins` counts the POSTs, `lapse` answers the
/// next data read 401 once (a session that ran out), `mfa` makes the login ask for a second factor.
#[derive(Clone, Default)]
struct Till {
    logins: Rc<Cell<u32>>,
    lapse: Rc<Cell<bool>>,
    mfa: bool,
}

impl Till {
    fn serve(&self) {
        let t = self.clone();
        answer_outbound(move |c: &Call| {
            let url = c.url()?;
            assert_eq!(url.host_str(), Some("www.ebills.al"), "only ebills is called: {url}");
            let cookie = c.headers().get("cookie")?.unwrap_or_default();
            match (c.method(), url.path()) {
                (worker::Method::Get, "/") => {
                    let mut r = Reply::ok("<html></html>")?;
                    r.headers_mut().set("content-type", "text/html")?;
                    r.headers_mut().append("set-cookie", "XSRF-TOKEN=x1; Path=/")?;
                    Ok(r)
                }
                (worker::Method::Post, "/api/authentication") => {
                    t.logins.set(t.logins.get() + 1);
                    assert_eq!(c.headers().get("x-xsrf-token")?.as_deref(), Some("x1"), "the front page's token is sent back");
                    let body = String::from_utf8_lossy(&c.body_bytes()).into_owned();
                    assert!(body.contains("username=till%40x.test") || body.contains("username=till@x.test"), "{body}");
                    if t.mfa {
                        return Ok(json_reply(&json!({"mfaRequired": true}))?.with_status(401));
                    }
                    let mut r = json_reply(&json!({}))?;
                    r.headers_mut().append("set-cookie", "JSESSIONID=s1; Path=/; HttpOnly")?;
                    r.headers_mut().set("x-tenant-identifier", "t-1")?;
                    Ok(r)
                }
                (worker::Method::Get, p) => {
                    assert!(cookie.contains("JSESSIONID=s1"), "a data read carries the session: {cookie}");
                    assert_eq!(c.headers().get("x-tenant-identifier")?.as_deref(), Some("t-1"));
                    if t.lapse.replace(false) {
                        return Ok(json_reply(&json!({"title": "Unauthorized"}))?.with_status(401));
                    }
                    match p {
                        "/api/sale-units-tables" => json_reply(&json!([
                            {"id": 12, "identifier": "12", "type": "TABLE", "status": "OCCUPIED", "pointOfSaleId": 1, "orderTotal": 500.0},
                            {"id": 14, "identifier": "14", "type": "TABLE", "status": "ACTIVE", "pointOfSaleId": 1, "orderTotal": null}
                        ])),
                        "/api/sales" => json_reply(&json!({"sales": [bill_listed(), course()], "total": 660.0})),
                        "/api/sales/8602" => {
                            let mut r = Reply::ok(&wrapped(course()))?;
                            r.headers_mut().set("content-type", "application/json")?;
                            Ok(r)
                        }
                        "/api/sales/8601" => {
                            let mut r = Reply::ok(&wrapped(bill_listed()))?;
                            r.headers_mut().set("content-type", "application/json")?;
                            Ok(r)
                        }
                        "/api/item-in-sales" => json_reply(&json!([{"id": 74, "itemCode": "56", "item": "korca", "price": 250.0, "vat": "VAT_20"}])),
                        p if p.starts_with("/api/sales/") => Ok(Reply::error("not found", 404)?),
                        other => panic!("an unexpected read: {other}"),
                    }
                }
                (m, p) => panic!("an unexpected call: {m:?} {p}"),
            }
        });
    }
}

fn configure(site: &Site, t: &str) {
    let r = site.run(
        crate::ebills::routes::config,
        post(&at("/api/owner/ebills/config"), &json!({"enabled": true, "pos_id": 1, "user": "till@x.test", "password": "pw"})).bearer(t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(!r.body_str().contains("\"pw\""), "the password is answered nowhere: {}", r.body_str());
}

fn status(site: &Site, t: &str) -> Value {
    let r = site.run(crate::ebills::routes::status, get(&at("/api/owner/ebills")).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn tick(site: &Site, at_ms: i64) {
    block_on(super::tick_venue(&site.env(), "alpha", at_ms));
}

#[test]
fn a_configured_link_logs_in_reads_the_till_and_the_venue_imports_it() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    // INSIDE THE HOURS: alpha filed none, so it polls in the storefront's 11:00-23:00 (W-LOOP,
    // `ebills::cadence`); T0 is 23:13 in Tirane, closed, and a closed venue's floor is not read.
    let noon = site.now_ms + 13 * 3_600_000;
    // Not configured: a tick reads nothing.
    tick(&site, noon);
    assert!(sent().is_empty(), "no link, no call");
    configure(&site, &t);
    let s = status(&site, &t);
    assert_eq!((s["config"]["enabled"].as_bool(), s["config"]["secret_set"].as_bool()), (Some(true), Some(true)), "{s}");
    assert_eq!(s["state"]["session_live"], false);

    let till = Till::default();
    till.serve();
    tick(&site, noon);
    assert_eq!(till.logins.get(), 1, "one login per firing");
    let s = status(&site, &t);
    assert_eq!(s["state"]["session_live"], true, "the session was written back: {s}");
    assert!(s["state"]["last_error"].is_null(), "{s}");
    assert!(s["state"]["last_sales_ms"].as_i64().unwrap() > 0, "the list was read: {s}");
    // The walk starts below the list (the ids it hides are leads) and works up under a budget,
    // so the first firing moves the watermark into the listed range without promising the top.
    assert!(s["state"]["watermark"].as_i64().unwrap() > 0, "the walk moved: {s}");
    assert_eq!(s["health"]["unmatched"], 1, "the till's dish waits for the owner's crosswalk: {s}");
    assert!(!s["floor"].is_null(), "the floor was read: {s}");
    assert!(s.to_string().contains("korca"), "the till's menu is offered for the crosswalk: {s}");
    let paths: Vec<String> = sent().iter().map(|c| c.url().unwrap().path().to_string()).collect();
    assert!(paths.iter().all(|p| !p.contains("..")), "{paths:?}");

    // The next firing reuses the session: no second login; nothing new, nothing written twice.
    let before = status(&site, &t)["state"].clone();
    tick(&site, noon + 6 * 60_000);
    assert_eq!(till.logins.get(), 1, "a live session is reused");
    let after = status(&site, &t)["state"].clone();
    assert_eq!((after["placed"].clone(), after["paid"].clone()), (before["placed"].clone(), before["paid"].clone()), "{after}");

    // The session lapses: re-established ONCE, and the read retried.
    till.lapse.set(true);
    tick(&site, noon + 12 * 60_000);
    assert_eq!(till.logins.get(), 2, "a refused session logs in again once");
    assert!(status(&site, &t)["state"]["last_error"].is_null());
}

#[test]
fn a_second_factor_halts_the_link_and_says_why() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    configure(&site, &t);
    let till = Till { mfa: true, ..Till::default() };
    till.serve();
    tick(&site, site.now_ms);
    let s = status(&site, &t);
    assert_eq!(s["state"]["halted"], true, "{s}");
    assert!(s["state"]["last_error"].to_string().contains("second factor"), "{s}");
    // Halted: the next firing does not knock again.
    let n = sent().len();
    tick(&site, site.now_ms + 10 * 60_000);
    assert_eq!(sent().len(), n, "a halted link calls nobody");
    // Another venue's owner neither reads nor reconfigures alpha's link.
    let (beta, _) = open_venue(&site, "beta", "b@x.test");
    let r = site.run(crate::ebills::routes::status, get(&at("/api/owner/ebills?location_id=alpha")).bearer(&beta).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

#[test]
fn a_page_instead_of_data_is_named_and_counted_as_a_failure() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    configure(&site, &t);
    answer_outbound(|c: &Call| {
        let mut r = Reply::ok("<html>maintenance</html>")?;
        r.headers_mut().set("content-type", "text/html")?;
        if c.url()?.path() == "/" {
            r.headers_mut().append("set-cookie", "XSRF-TOKEN=x1")?;
        }
        if c.url()?.path() == "/api/authentication" {
            r.headers_mut().append("set-cookie", "JSESSIONID=s1")?;
            r.headers_mut().set("x-tenant-identifier", "t-1")?;
        }
        Ok(r)
    });
    tick(&site, site.now_ms);
    let s = status(&site, &t);
    assert_eq!(s["state"]["failures"], 1, "{s}");
    assert!(s["state"]["last_error"].to_string().contains("a page, not data"), "{s}");
    assert_eq!(s["state"]["halted"], false, "a page is worth trying again, after the backoff");
}

/// W-INT2 #13: the till link through the DAG path -- saving the link arms the venue's alarm, the
/// runner's job (`cron::run`, what the alarm asks `cron~alpha` for) ticks the poll against the
/// fake till, and the imported course is an order in the OWNER's list.
#[test]
fn the_alarm_path_imports_a_till_sale_into_the_owners_orders() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let host = site.world.host("alpha");
    assert!(host.alarm.get().is_none(), "no link, nothing to wake for");
    configure(&site, &t);
    let at = host.alarm.get().expect("saving the link armed the alarm");
    let till = Till::default();
    till.serve();
    let uuid = "00000000-0000-4000-8000-000000008602"; // `ebills::tests::course()`
    let owner_sees = |site: &Site| {
        let r = site.run(crate::owner::orders, get(&at_path("/api/owner/orders")).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        r.body_str().contains(uuid)
    };
    assert!(!owner_sees(&site), "imported before any tick");
    // Three firings, six minutes apart: the walk reaches the listed course within its budget.
    for n in 0..3 {
        block_on(crate::cron::run(&site.env(), "alpha", at + n * 6 * 60_000));
    }
    assert!(till.logins.get() >= 1, "the runner never reached the till");
    assert!(owner_sees(&site), "the till's course is not in the owner's orders: {}", status(&site, &t));
}

fn at_path(path: &str) -> String {
    at(path)
}
