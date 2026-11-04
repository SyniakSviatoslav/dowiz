//! THE REAL MONTH THROUGH THE REAL ALARM TURN, natively (W-EBILLS-POISON,
//! 2026-11-04). The production incident: the first-run history import of a
//! real till poisoned the Worker's isolate ("Module terminated" on every
//! later request). This drives the venue object's OWN alarm turn
//! (`timer_alarm_in`: outbox drain, the poll, the fiscal rail, the next
//! alarm) against the till's REAL answers read from files a read-only fetch
//! left on disk (`fetch-till.mjs`, `live.rs`'s layout), with the venue's
//! REAL menu when given, turn after turn until the month is read through --
//! and counts the till requests each turn made. A Rust panic anywhere on
//! that path fails this test with its backtrace; nothing it prints is a
//! body (a sale carries a waiter's name and the signing certificate).
//!
//!   EBILLS_LIVE_DIR=<dir> [EBILLS_MENU=<public menu json>] [EBILLS_TURNS=<max>]
//!   cargo test --lib ebills::poll::flow_tests::month -- --ignored --nocapture

use super::{at, configure, status};
use crate::edge::mem::{answer_outbound, block_on, object_requests, sent};
use crate::edge::site::{post, As, Site};
use crate::storefront::route_tests::open_venue;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

const VENUE: &str = "alpha";

/// `{"status", "body"}` as the fetch left it, or `None` when the file is not there.
fn answer(dir: &str, name: &str) -> Option<(u16, String)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/{name}")).ok()?).ok()?;
    Some((v["status"].as_u64()? as u16, v["body"].as_str()?.to_string()))
}

fn reply(status: u16, body: &str) -> worker::Result<Reply> {
    let mut r = Reply::ok(body)?.with_status(status);
    r.headers_mut().set("content-type", "application/json")?;
    Ok(r)
}

/// ebills.al, answering from the files: the front page's cookie, the login,
/// then the allow-listed reads exactly as the real till answered them.
fn serve_the_files(dir: String) {
    answer_outbound(move |c: &Call| {
        let url = c.url()?;
        assert_eq!(url.host_str(), Some("www.ebills.al"), "only ebills is called: {url}");
        let p = url.path().to_string();
        let q = url.query().unwrap_or_default().to_string();
        match (c.method(), p.as_str()) {
            (worker::Method::Get, "/") => {
                let mut r = Reply::ok("<html></html>")?;
                r.headers_mut().set("content-type", "text/html")?;
                r.headers_mut().append("set-cookie", "XSRF-TOKEN=x1; Path=/")?;
                Ok(r)
            }
            (worker::Method::Post, "/api/authentication") => {
                let mut r = reply(200, "{}")?;
                r.headers_mut().append("set-cookie", "JSESSIONID=s1; Path=/; HttpOnly")?;
                r.headers_mut().set("x-tenant-identifier", "t-1")?;
                Ok(r)
            }
            (worker::Method::Get, "/api/sales") => {
                let name = if q.contains("sort=id,asc") { "oldest.json" } else { "list.json" };
                let (s, b) = answer(&dir, name).unwrap_or_else(|| panic!("{dir}/{name} is missing"));
                reply(s, &b)
            }
            (worker::Method::Get, "/api/item-in-sales") => {
                let (s, b) = answer(&dir, "items.json").expect("items.json");
                reply(s, &b)
            }
            (worker::Method::Get, "/api/sale-units-tables") => {
                let (s, b) = answer(&dir, "tables.json").expect("tables.json");
                reply(s, &b)
            }
            (worker::Method::Get, p) if p.starts_with("/api/sales/") => match answer(&dir, &format!("sale-{}.json", &p["/api/sales/".len()..])) {
                Some((s, b)) => reply(s, &b),
                None => Ok(Reply::error("not found", 404)?),
            },
            (m, p) => panic!("an unexpected call: {m:?} {p}"),
        }
    });
}

/// The venue's real catalogue through the owner's routes: every category and dish of the
/// public menu, on sale, so the import's NAME MATCH runs against what production had.
fn seed_menu(site: &Site, t: &str, menu: &str) -> usize {
    let m: Value = serde_json::from_str(&std::fs::read_to_string(menu).expect("the menu file")).expect("the menu parses");
    let mut dishes = 0;
    for c in m["categories"].as_array().into_iter().flatten() {
        let r = site.run(
            crate::catalog_edit::set_category,
            post(&at("/api/owner/categories"), &json!({"location_id": VENUE, "name": c["name"]})).bearer(t).on(VENUE),
            &[],
        );
        assert_eq!(r.status_code(), 200, "category: {}", r.body_str());
        let cat = r.body_value()["id"].as_str().expect("category id").to_string();
        for p in c["products"].as_array().into_iter().flatten() {
            let r = site.run(
                crate::catalog_edit::create_product,
                post(&at("/api/owner/products"), &json!({"location_id": VENUE, "category_id": cat, "name": p["name"], "price": p["price"]}))
                    .bearer(t)
                    .on(VENUE),
                &[],
            );
            assert_eq!(r.status_code(), 200, "product: {}", r.body_str());
            let dish = r.body_value()["id"].as_str().expect("dish id").to_string();
            let r = site.run(
                crate::owner::update_product,
                post(&at(&format!("/api/owner/products/{dish}")), &json!({"location_id": VENUE, "allergens": [], "available": true})).bearer(t).on(VENUE),
                &[("id", &dish)],
            );
            assert_eq!(r.status_code(), 200, "on sale: {}", r.body_str());
            dishes += 1;
        }
    }
    dishes
}

/// One alarm turn of the venue's object, as production runs it (`Env::Own`
/// inside): the till requests and the billed object requests it made.
fn turn(site: &Site, at_ms: i64) -> (usize, usize) {
    let (reads, objects) = (sent().len(), object_requests());
    block_on(site.object(VENUE).timer_alarm_in(&site.env(), at_ms)).expect("the alarm ran");
    (sent().len() - reads, object_requests() - objects)
}

#[test]
#[ignore]
fn the_real_months_answers_through_the_alarm_turn_until_read_through() {
    let Ok(dir) = std::env::var("EBILLS_LIVE_DIR") else { return };
    let max_turns: i64 = std::env::var("EBILLS_TURNS").ok().and_then(|s| s.parse().ok()).unwrap_or(400);
    let site = Site::new();
    let (t, _) = open_venue(&site, VENUE, "a@x.test");
    if let Ok(menu) = std::env::var("EBILLS_MENU") {
        println!("menu: {} dishes seeded through the owner's routes", seed_menu(&site, &t, &menu));
    }
    configure(&site, &t);
    serve_the_files(dir);

    // Inside the polling hours (`cadence`): T0 is 23:13 in Tirane; noon the next day is open.
    let noon = site.now_ms + 13 * 3_600_000;
    let (mut most, mut busiest, mut turns_with_reads, mut total_reads) = (0usize, 0i64, 0, 0usize);
    let mut n = 0;
    let state = loop {
        let at = noon + n * 60_000;
        let (reads, objects) = turn(&site, at);
        total_reads += reads;
        if reads > most {
            (most, busiest) = (reads, n);
        }
        let s = status(&site, &t);
        let st = &s["state"];
        if reads > 0 {
            turns_with_reads += 1;
            println!(
                "turn {n}: {reads} till requests, {objects} object requests; watermark {} backlog {} placed {} noted {} paid {} refused {} failures {} halted {}",
                st["watermark"], st["backlog"], st["placed"], st["noted"], st["paid"], st["refused"].as_array().map_or(0, Vec::len), st["failures"], st["halted"]
            );
        }
        if reads > 0 && st["backlog"] == false {
            break s;
        }
        n += 1;
        assert!(n < max_turns, "the month was not read through in {max_turns} turns: {s}");
    };
    println!("month: read through in {} turns ({turns_with_reads} with reads), {total_reads} till requests, the busiest turn ({busiest}) made {most}", n + 1);

    // THE DAILY RE-READ (`poll::reread`) and the re-check passes after it: a day on, noon again.
    let tomorrow = noon + 24 * 3_600_000;
    for k in 0..40 {
        let (reads, objects) = turn(&site, tomorrow + k * 60_000);
        if reads > most {
            (most, busiest) = (reads, 1_000 + k);
        }
        let s = status(&site, &t);
        if reads > 0 {
            println!("day 2 turn {k}: {reads} till requests, {objects} object requests; recheck {:?} failures {}", s["state"]["recheck"], s["state"]["failures"]);
        }
    }
    let s = status(&site, &t);
    println!(
        "end: watermark {} placed {} noted {} paid {} refused {} pending bills {} failures {} halted {}; busiest turn {busiest} made {most} till requests",
        s["state"]["watermark"], s["state"]["placed"], s["state"]["noted"], s["state"]["paid"],
        s["state"]["refused"].as_array().map_or(0, Vec::len), s["state"]["pending"].as_array().map_or(0, Vec::len), s["state"]["failures"], s["state"]["halted"]
    );
    let _ = state;
    assert_eq!(s["state"]["halted"], false, "{s}");
}
