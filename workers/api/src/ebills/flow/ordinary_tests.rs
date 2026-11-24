//! A TILL SALE IS AN ORDINARY ORDER (operator 2026-10-19: "замовлення з ebills
//! ... були в системі як звичайні замовлення автоматично, зокрема в історії та
//! аналітиці"). The sale goes in the way production takes it -- the owner's
//! link saved, the venue's alarm, the poll against the fake till, the import in
//! the venue's object -- and every reader of "an order" is then asked through
//! its own route: the owner's list, today's tiles, the analytics pane (channel,
//! dish, hour, kind), and the same numbers after the night's rotation moved the
//! sale into an archive, plus the archive itself on the history screen.

use super::{at, configure, Till};
use crate::edge::mem::block_on;
use crate::edge::site::{get, post, As, Site};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

const DAY: i64 = 86_400_000;
/// `ebills::tests::course()`: 2026-09-21T09:01:25.857Z, table 12, 2 x korca at 250.
const SOLD_AT: i64 = 1_789_981_285_857;
const ORDER: &str = "ebills:00000000-0000-4000-8000-000000008602";

/// The venue after the alarm path imported the till's course.
fn imported() -> Site {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    configure(&site, &t);
    let alarm = site.world.host("alpha").alarm.get().expect("saving the link armed the alarm");
    Till::default().serve();
    for n in 0..3 {
        block_on(crate::cron::run(&site.env(), "alpha", alarm + n * 6 * 60_000));
    }
    site
}

fn owner_get<F, Fut>(site: &Site, handler: F, path: &str) -> Value
where
    F: FnOnce(crate::wire::Call, crate::edge::Ctx<crate::Req>) -> Fut,
    Fut: std::future::Future<Output = worker::Result<crate::wire::Reply>>,
{
    let t = site.login("alpha", "a@x.test");
    let r = site.run(handler, get(&at(path)).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{path}: {}", r.body_str());
    r.body_value()
}

use crate::owner::{dashboard, orders};
use crate::services::analytics::handler::analytics;
use crate::services::operations::history;

/// The till's channel row, and the till's dish, as the analytics pane reads them.
fn till_numbers(a: &Value) -> (Value, Value, Value) {
    let ch = a["channels"].as_array().into_iter().flatten().find(|c| c["channel"] == "ebills").cloned().unwrap_or(Value::Null);
    let dish = a["topProducts"].as_array().into_iter().flatten().find(|d| d["id"] == "ebills:56").cloned().unwrap_or(Value::Null);
    (json!([ch["orders"], ch["revenue"]]), json!([dish["name"], dish["quantity"], dish["revenue"]]), json!([a["dineIn"], a["orders"], a["revenue"]]))
}

#[test]
fn an_imported_till_sale_is_an_order_in_the_list_the_tiles_and_the_analytics() {
    let mut site = imported();
    site.now_ms = SOLD_AT + 3 * 3_600_000;
    let list = owner_get(&site, orders, "/api/owner/orders?location_id=alpha");
    let o = list["orders"].as_array().unwrap().iter().find(|o| o["id"] == ORDER).cloned().expect("the till's course is in the owner's orders");
    assert_eq!((o["status"].as_str(), o["channel"].as_str(), o["total"].as_i64()), (Some("PICKED_UP"), Some("ebills"), Some(500)), "{o}");
    let tiles = owner_get(&site, dashboard, "/api/owner/dashboard?location_id=alpha");
    assert_eq!((tiles["todayOrders"].as_i64(), tiles["todayRevenue"].as_i64()), (Some(1), Some(500)), "today's tiles count the till: {tiles}");

    let a = owner_get(&site, analytics, "/api/owner/analytics?location_id=alpha&v=2&days=7");
    let (ch, dish, sums) = till_numbers(&a);
    assert_eq!(ch, json!([1, 500]), "the channel mix: {}", a["channels"]);
    assert_eq!(sums, json!([1, 1, 500]), "a table order, one order, 500 taken");
    assert_eq!(dish, json!(["korca", 2, 500]), "the till's dish by the name it was sold under: {}", a["topProducts"]);
    let hour = crate::services::analytics::cube::hour_of(dowiz_hub::tz::DEFAULT, SOLD_AT);
    assert_eq!(a["hourRevenue"][hour], 500, "by the venue's hour");
}

#[test]
fn an_imported_till_sale_stays_in_history_and_analytics_after_the_rotation() {
    let mut site = imported();
    let now = SOLD_AT + 40 * DAY;
    site.now_ms = now;
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let v = block_on(crate::hubstore::rotate(&place, now)).unwrap();
    assert_eq!(v["rotated"].as_bool(), Some(true), "{v}");
    let t = site.login("alpha", "a@x.test");
    let r = site.run(crate::services::analytics::handler::history, post(&at("/api/owner/analytics/history"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());

    // The live list no longer holds it; the history screen's archive does.
    let list = owner_get(&site, orders, "/api/owner/orders?location_id=alpha");
    assert!(!list.to_string().contains(ORDER), "rotated out of the hot log");
    let h = owner_get(&site, history, "/api/owner/history?location_id=alpha");
    let archive = h["archives"][0].as_str().expect("an archive").to_string();
    let rows = owner_get(&site, history, &format!("/api/owner/history?location_id=alpha&archive={archive}"));
    assert!(rows["orders"].as_array().unwrap().iter().any(|o| o["id"] == ORDER && o["channel"] == "ebills"), "{rows}");

    let a = owner_get(&site, analytics, "/api/owner/analytics?location_id=alpha&v=2&from=2026-09-21&to=2026-09-21");
    let (ch, _, sums) = till_numbers(&a);
    assert_eq!((ch, sums), (json!([1, 500]), json!([1, 1, 500])), "the archived day still counts the till: {a}");
    assert_eq!(a["history"]["archivedDays"].as_i64(), Some(1), "{}", a["history"]);
}

/// THE CROSSWALK THROUGH THE VENUE'S OWN OBJECT: the imported course's till code 56
/// ("korca", 2 x 250) is no dish's, so the kitchen's margin names it unmatched; the
/// owner links it to Futomaki through `POST /api/owner/ebills/map`, and the same
/// kitchen answer then counts both portions under Futomaki and no longer lists 56.
/// The link lives in the object's till table (`K_MAP`) and the kitchen fold reads it
/// there, so this walks the wiring no unit test of the fold's arithmetic reaches.
#[test]
fn a_till_code_the_owner_links_counts_under_its_dish_in_the_kitchen_margin() {
    let mut site = imported();
    site.now_ms = SOLD_AT + 3 * 3_600_000;
    let kitchen = |site: &Site| owner_get(site, crate::services::analytics::kitchen::kitchen, "/api/owner/analytics/kitchen?location_id=alpha&v=2&days=7");
    // `menu.today` carries ids, no names; the venue has one dish, Futomaki.
    let futomaki = |k: &Value| match k["menu"]["today"].as_array().unwrap().as_slice() {
        [d] => d["id"].as_str().map(str::to_string),
        _ => None,
    };
    let row = |k: &Value, id: &str| k["menu"]["dishes"].as_array().unwrap().iter().find(|d| d["id"] == id).map(|d| (d["sold"].as_i64(), d["attributed"].as_i64(), d["revenue"].as_i64()));
    let codes = |k: &Value| k["menu"]["unmatched"].as_array().unwrap().iter().map(|u| u["code"].as_str().unwrap().to_string()).collect::<Vec<_>>();

    let before = kitchen(&site);
    let dish = futomaki(&before).unwrap_or_else(|| panic!("the venue's dish is on the menu: {}", before["menu"]));
    assert_eq!(codes(&before), ["56"], "not linked yet, the till code waits: {}", before["menu"]["unmatched"]);
    assert_eq!(row(&before, &dish), None, "and Futomaki sold nothing: {}", before["menu"]["dishes"]);
    assert_eq!(row(&before, "ebills:56"), Some((Some(2), Some(0), Some(500))), "the till line is its own row: {}", before["menu"]["dishes"]);

    let t = site.login("alpha", "a@x.test");
    let r = site.run(crate::ebills::routes::map, post(&at("/api/owner/ebills/map"), &json!({"code": "56", "product_id": dish})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());

    let after = kitchen(&site);
    assert_eq!(row(&after, &dish), Some((Some(2), Some(2), Some(500))), "both portions under Futomaki, counted by the link: {}", after["menu"]["dishes"]);
    assert!(codes(&after).is_empty(), "56 is no longer unmatched: {}", after["menu"]["unmatched"]);
    assert_eq!(row(&after, "ebills:56"), None, "nor its own row: {}", after["menu"]["dishes"]);
}
