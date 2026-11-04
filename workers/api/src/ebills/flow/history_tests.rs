//! THE FIRST RUN BACKFILLS THE MONTH (operator 2026-11-03: the till has thirty
//! days of history and the site showed none). Through the real path: the link
//! saved, the poll against the fake till whose month's oldest sale is forty
//! ids below anything the window lists, the walk a budget at a time, the
//! venue's object importing. The sale from a month back is the owner's order
//! ONCE, however many passes follow. Without `history.rs` the walk starts at
//! the window and that sale is a lead no bill claims -- never placed.

use super::{at, configure, status, tick, Till};
use crate::ebills::state::LEAD_IDS;
use crate::ebills::walk::BUDGET;
use crate::edge::mem::sent;
use crate::edge::site::{get, As, Site};
use crate::storefront::route_tests::open_venue;

/// The month's oldest listed sale; the counter sale is five ids after it
/// (`Till::history`, `counter_at`), far below the window's 8601..8602.
const OLDEST: i64 = 8520;
const ORDER: &str = "ebills:00000000-0000-4000-8000-000000008525";

/// Every id asked for by detail, in order.
fn details() -> Vec<i64> {
    sent().iter().filter_map(|c| c.url().ok()?.path().strip_prefix("/api/sales/")?.parse().ok()).collect()
}

/// `Path::Oldest`: the sales list, one row, ascending (the menu is read ascending too, and is not this).
fn history_reads() -> usize {
    sent().iter().map(|c| c.url().unwrap()).filter(|u| u.path() == "/api/sales" && u.query().is_some_and(|q| q.contains("size=1&sort=id,asc"))).count()
}

#[test]
fn a_first_run_starts_at_the_months_oldest_sale_and_imports_it_once() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    configure(&site, &t);
    Till { history: Some(OLDEST), ..Till::default() }.serve();
    let owner_has = |site: &Site| {
        let r = site.run(crate::owner::orders, get(&at("/api/owner/orders")).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        r.body_value()["orders"].as_array().map_or(0, |os| os.iter().filter(|o| o["id"] == ORDER).count())
    };
    // Inside the hours (`cadence`): T0 is 23:13 in Tirane; noon is open.
    let noon = site.now_ms + 13 * 3_600_000;
    tick(&site, noon);
    assert_eq!(history_reads(), 1, "the month's oldest sale was asked for: {:?}", sent().iter().map(|c| c.url().unwrap().to_string()).collect::<Vec<_>>());
    let d = details();
    assert_eq!(d.first(), Some(&(OLDEST - LEAD_IDS)), "the walk starts LEAD_IDS below the month's oldest sale: {d:?}");
    assert_eq!(d.len() as u32, BUDGET, "one firing reads its budget and no more: {d:?}");
    let s = status(&site, &t);
    assert_eq!(s["state"]["backlog"], true, "a month is more than one firing: {s}");
    assert_eq!(owner_has(&site), 0, "not reached yet");

    // The backlog re-arms a minute on; the third firing reaches the sale.
    for n in 1..=3 {
        tick(&site, noon + n * 60_000);
    }
    assert_eq!(owner_has(&site), 1, "the counter sale from a month back is the owner's order: {}", status(&site, &t));
    assert_eq!(history_reads(), 1, "the history is asked for on the first run only");

    // Every later pass over the same month places nothing again.
    let placed = status(&site, &t)["state"]["placed"].as_u64().unwrap();
    for n in 4..=10 {
        tick(&site, noon + n * 60_000);
    }
    assert_eq!(owner_has(&site), 1, "the same sale twice is the same order");
    let s = status(&site, &t);
    assert_eq!(s["state"]["backlog"], false, "the month is read through: {s}");
    assert!(s["state"]["placed"].as_u64().unwrap() >= placed, "{s}");
    assert_eq!(history_reads(), 1, "{s}");
}
