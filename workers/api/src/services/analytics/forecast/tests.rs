//! The prep list over a small sushi venue: the forecast from the hot log and
//! the cube, through the recipes into ПФ and raw, the shelf read, tonight's
//! bookings on top, and "learning" when the history is short.

use super::expand::{components, needs, raw_by_day, use_until};
use super::plan::{bookings, cast, hhmm, windows};
use super::*;
use crate::services::analytics::cube;
use dowiz_hub::stock::{StockEvent, StockLog};

/// 2026-09-22 12:00 UTC, a Tuesday: 14:00 in Tirana.
const NOON: i64 = 1_790_078_400_000;
const DAY: i64 = 86_400_000;
const NO_COVERS: &dyn Fn(i64, i64) -> i64 = &|_, _| 0;
const NO_CUBE: &dyn Fn(i64, i64) -> std::result::Result<BTreeMap<i64, DayCube>, String> = &|_, _| Ok(BTreeMap::new());

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}

/// The venue of `kitchen/prep_tests.rs`: maki (raw rice), philadelphia (the
/// seasoned rice ПФ, whose card holds the mitsukan ПФ), open 11:00-23:00.
fn venue(hours: bool) -> dowiz_hub::catalog::Catalog {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    let day = r#"[{"open":660,"close":1380}]"#;
    let h = if hours { format!(r#","hours":[{d},{d},{d},{d},{d},{d},{d}]"#, d = day) } else { String::new() };
    cat.set_location(&format!(r#"{{"id":"v","tz":"Europe/Tirane"{h}}}"#));
    let raw = |id: &str, unit: &str| json!({ "id": id, "name": id, "unit": unit, "kind": "food_ingredient" }).to_string();
    cat.set_supply("vinegar", &raw("vinegar", "ml"));
    cat.set_supply("salt", &raw("salt", "g"));
    cat.set_supply("sugar", &raw("sugar", "g"));
    cat.set_supply("rice-dry", &raw("rice-dry", "g"));
    cat.set_supply("water", r#"{"id":"water","name":"Water","unit":"ml","kind":"food_ingredient","untracked":true}"#);
    cat.set_supply("mitsukan", r#"{"id":"mitsukan","name":"Mitsukan","unit":"g","kind":"prep","card":{"lines":[{"item":"vinegar","qty":800},{"item":"salt","qty":50},{"item":"sugar","qty":150}],"yield":1000}}"#);
    cat.set_supply("rice-seasoned", r#"{"id":"rice-seasoned","name":"Rice seasoned","unit":"g","kind":"prep","card":{"lines":[{"item":"rice-dry","qty":1000},{"item":"water","qty":1100},{"item":"mitsukan","qty":250}],"yield":2100}}"#);
    cat.set_product("philadelphia", r#"{"id":"philadelphia","name":"Philadelphia","price":650,"bom":[{"supply":"rice-seasoned","qty":130}]}"#);
    cat.set_product("maki", r#"{"id":"maki","name":"Maki","price":300,"bom":[{"supply":"rice-dry","qty":90}]}"#);
    cat.set_product("cola", r#"{"id":"cola","name":"Cola","price":200}"#);
    cat
}

fn order(i: usize, at: i64, dishes: &[(&str, i64)], status: &str) -> crate::hubdo::OrderView {
    let items: Vec<Value> = dishes.iter().map(|(d, q)| json!({ "product_id": d, "quantity": q, "unit_price": 500 })).collect();
    let o = json!({ "id": format!("o{i}"), "location_id": "v", "created_at_ms": at, "total": 1000, "tip": 0, "status": status, "items": items });
    crate::hubdo::OrderView { order_id: format!("o{i}"), kind: 1, seq: i as u64, order_json: o.to_string() }
}

/// `per_week[k]` orders of one philadelphia and one maki, on the same
/// weekday `k + 1` weeks before NOON (oldest last in the slice is fine).
fn weeks(per_week: &[usize]) -> Vec<crate::hubdo::OrderView> {
    let mut out = Vec::new();
    for (k, n) in per_week.iter().enumerate() {
        for j in 0..*n {
            out.push(order(out.len(), NOON - 7 * (k as i64 + 1) * DAY + j as i64, &[("philadelphia", 1), ("maki", 1)], "DELIVERED"));
        }
    }
    out
}

fn shelf(ready: i64) -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    if ready > 0 {
        log.append(&StockEvent::Received { item: "rice-seasoned".into(), qty: ready }).unwrap();
    }
    log
}

fn answer(listed: Vec<crate::hubdo::OrderView>, cat: &dowiz_hub::catalog::Catalog, ready: i64, day: Option<&str>, covers: Covers) -> Value {
    answer_with(listed, cat, &shelf(ready), "v", NOON, day, covers, NO_CUBE).expect("an answer")
}

#[test]
fn the_day_is_today_to_six_days_ahead_and_nothing_else() {
    let z = tirane();
    let today = days(z, NOON, None).unwrap().0;
    assert_eq!(days(z, NOON, None), Ok((today, today)));
    assert_eq!(days(z, NOON, Some("2026-09-28")), Ok((today, today + 6)));
    assert!(days(z, NOON, Some("2026-09-29")).is_err(), "seven days ahead");
    assert!(days(z, NOON, Some("2026-09-21")).is_err(), "yesterday");
    assert!(days(z, NOON, Some("tuesday")).unwrap_err().contains("not a date"));
    assert_eq!(days(z, NOON, Some("  ")), Ok((today, today)), "blank is today");
}

/// THE STRICT QUERY: a key the route does not read is named.
#[test]
fn a_query_key_the_route_does_not_read_is_refused() {
    assert_eq!(unknown_key(["day", "location_id"].into_iter()), None);
    assert_eq!(unknown_key(["day", "dya"].into_iter()), Some("dya"));
    assert_eq!(unknown_key(std::iter::empty()), None);
}

#[test]
fn the_history_adds_the_cube_and_the_hot_log_and_stops_before_today() {
    let z = tirane();
    let listed = weeks(&[2, 3]);
    let all = cube::fold_orders(&crate::services::orders::mine::of_venue(listed, "v"), z, NOON);
    let (d1, d2) = (*all.keys().next().unwrap(), *all.keys().nth(1).unwrap());
    let cold: BTreeMap<i64, DayCube> = all.iter().take(1).map(|(k, v)| (*k, v.clone())).collect();
    let mut hot: BTreeMap<i64, DayCube> = all.iter().skip(1).map(|(k, v)| (*k, v.clone())).collect();
    let refused = cube::fold_orders(&[serde_json::from_str(&order(99, NOON - 7 * DAY, &[("maki", 5)], "REJECTED").order_json).unwrap()], z, NOON);
    hot.get_mut(&d2).unwrap().add(&refused[&d2]);
    let today = days(z, NOON, None).unwrap().0;
    let h = history_of(&cold, &hot, today);
    assert_eq!(h.days.len(), 2);
    assert_eq!(h.days[&day_number(d1)].orders, 3, "the cube's day (two weeks back)");
    let last = &h.days[&day_number(d2)];
    assert_eq!((last.orders, last.dishes["maki"], last.bands.iter().sum::<i64>()), (2, 2, 3), "a refused order is placed, never accepted, never a dish");
    assert!(history_of(&cold, &hot, day_number(d2)).days.len() == 1, "a day at or after as_of is not history");
}

/// The acceptance of the prep list: four weeks of ten orders, 500 g of the
/// seasoned rice ready -> 1 300 g needed, 800 g to make, from its card.
#[test]
fn the_prep_list_is_the_forecast_through_the_recipes_minus_the_shelf() {
    let cat = venue(true);
    let v = answer(weeks(&[10, 10, 10, 10]), &cat, 500, None, NO_COVERS);
    assert_eq!(v["contract"], "kitchen.prep_forecast.v1");
    assert_eq!((v["day"].as_str(), v["today"].as_str(), v["weekday"].as_i64()), (Some("2026-09-22"), Some("2026-09-22"), Some(1)));
    assert_eq!(v["orders"], json!({ "value": 10, "learning": false, "weeks": 4, "method": "median" }));
    assert_eq!((v["portions"]["value"].as_i64(), v["portions"]["learning"].as_bool()), (Some(20), Some(false)));
    let dish = |id: &str| v["dishes"].as_array().unwrap().iter().find(|d| d["id"] == id).cloned().unwrap();
    assert_eq!((dish("philadelphia")["value"].as_i64(), dish("philadelphia")["name"].as_str()), (Some(10), Some("Philadelphia")));
    let p = &v["preps"][0];
    assert_eq!((p["id"].as_str(), p["need"].as_i64(), p["onHand"].as_i64(), p["make"].as_i64()), (Some("rice-seasoned"), Some(1300), Some(500), Some(800)));
    let from: Vec<(String, i64)> = p["from"].as_array().unwrap().iter().map(|x| (x["id"].as_str().unwrap().to_string(), x["qty"].as_i64().unwrap())).collect();
    // 800 g of a 2 100 g batch: 381 g of rice and 95.2 g of mitsukan, whose card is 80 % vinegar.
    assert_eq!(from, vec![("rice-dry".into(), 381), ("salt".into(), 5), ("sugar".into(), 14), ("vinegar".into(), 76)]);
    // Raw for the whole day: ten philadelphia through both cards + ten maki at 90 g.
    let raw: BTreeMap<String, i64> = v["raw"].as_array().unwrap().iter().map(|x| (x["id"].as_str().unwrap().to_string(), x["qty"].as_i64().unwrap())).collect();
    assert_eq!(raw, BTreeMap::from([("rice-dry".into(), 619 + 900), ("salt".into(), 8), ("sugar".into(), 23), ("vinegar".into(), 124)]));
    assert!(!raw.contains_key("water"), "untracked");
    // The twin: a shelf that holds enough makes nothing.
    let full = answer(weeks(&[10, 10, 10, 10]), &cat, 5000, None, NO_COVERS);
    assert_eq!((full["preps"][0]["make"].as_i64(), full["preps"][0]["from"].as_array().map(Vec::len)), (Some(0), Some(0)));
}

#[test]
fn bookings_add_their_guests_at_the_usual_basket() {
    let cat = venue(true);
    let four: Covers = &|from, to| if to - from >= 23 * 60 { 4 } else { 0 };
    let v = answer(weeks(&[10, 10, 10, 10]), &cat, 500, None, four);
    assert_eq!(v["bookings"], json!({ "covers": 4, "portions": 8 }), "4 guests x (1 philadelphia + 1 maki per order)");
    assert_eq!(v["preps"][0]["need"].as_i64(), Some(14 * 130));
    let h = history_of(&BTreeMap::new(), &cube::fold_orders(&crate::services::orders::mine::of_venue(weeks(&[10, 10, 10, 10]), "v"), tirane(), NOON), days(tirane(), NOON, None).unwrap().0);
    let today = days(tirane(), NOON, None).unwrap().0;
    let c = cast(&h, today, today, false);
    assert!(bookings(&h, &c, today, today, 0).is_empty(), "the twin: no guests, nothing added");
}

/// THE "LEARNING" RULE, end to end: two weeks give no number anywhere.
#[test]
fn two_weeks_of_history_say_learning_and_list_nothing_to_make() {
    let cat = venue(true);
    let v = answer(weeks(&[10, 10]), &cat, 0, None, NO_COVERS);
    assert_eq!((v["portions"]["value"].clone(), v["portions"]["learning"].as_bool(), v["portions"]["weeks"].as_i64()), (Value::Null, Some(true), Some(2)));
    assert!(v["dishes"].as_array().unwrap().iter().all(|d| d["learning"] == true && d["value"].is_null()));
    assert_eq!((v["preps"].as_array().map(Vec::len), v["raw"].as_array().map(Vec::len)), (Some(0), Some(0)));
    assert_eq!(v["samples"].as_array().map(Vec::len), Some(2));
}

#[test]
fn the_error_is_measured_and_shown_beside_the_naive_guess() {
    let cat = venue(true);
    let v = answer(weeks(&[9, 12, 10, 14, 8, 11, 10]), &cat, 0, None, NO_COVERS);
    let p = &v["portions"];
    assert_eq!(p["checkedDays"].as_i64(), Some(28));
    assert!(p["offBy"].as_i64().is_some() && p["naiveOffBy"].as_i64().is_some(), "{p}");
    let today = days(tirane(), NOON, None).unwrap().0;
    let h = history_with(weeks(&[9, 12, 10, 14, 8, 11, 10]), "v", tirane(), NOON, today, NO_CUBE).0;
    let e = fc::backtest(&h, fc::first_day(&h, today).unwrap(), today, &|d: &fc::Day| d.portions());
    assert_eq!((p["offBy"].as_i64(), p["masePm"].as_i64()), (e.off_by(), e.mase_pm()), "the JSON is the hub's measurement");
    let s: Vec<i64> = v["samples"].as_array().unwrap().iter().map(|s| s["orders"].as_i64().unwrap()).collect();
    assert_eq!(s, vec![10, 11, 8, 14, 10, 12, 9], "oldest first");
    assert_eq!(v["orders"]["value"].as_i64(), Some(10), "the median of the samples the answer lists");
}

#[test]
fn the_bands_follow_the_hours_and_say_when_there_are_none() {
    let v = answer(weeks(&[10, 10, 10]), &venue(true), 0, None, NO_COVERS);
    assert_eq!(v["hours"], json!({ "known": true, "open": true }));
    let b: Vec<(String, String, Option<i64>)> = v["bands"].as_array().unwrap().iter().map(|b| (b["from"].as_str().unwrap().into(), b["to"].as_str().unwrap().into(), b["value"].as_i64())).collect();
    assert_eq!(b, vec![("11:00".into(), "14:00".into(), Some(0)), ("14:00".into(), "18:00".into(), Some(10)), ("18:00".into(), "23:00".into(), Some(0))], "the orders came at 14:00 local");
    let none = answer(weeks(&[10, 10, 10]), &venue(false), 0, None, NO_COVERS);
    assert_eq!(none["hours"]["known"], false);
    assert_eq!(none["bands"].as_array().map(Vec::len), Some(3), "the clock's bands");
    assert_eq!((windows(None, 0), hhmm(1560), hhmm(660)), (None, "02:00".to_string(), "11:00".to_string()));
}

#[test]
fn a_dish_without_a_recipe_is_named_and_takes_nothing() {
    let cat = venue(true);
    let n = needs(&BTreeMap::from([("cola".to_string(), 5), ("maki".to_string(), 2), ("gone".to_string(), 1)]), &cat);
    assert_eq!(n.unmodelled, vec!["cola".to_string(), "gone".to_string()]);
    assert_eq!(n.raw, BTreeMap::from([("rice-dry".to_string(), 180)]));
    assert!(n.preps.is_empty() && n.refused.is_empty());
    assert!(components("nope", 10, &cat).is_err(), "an unknown ПФ is refused, not skipped");
}

/// P7's table: the forecast's raw use per day, and the use until a date.
#[test]
fn the_daily_raw_use_feeds_the_expiry_check() {
    let cat = venue(true);
    let today = days(tirane(), NOON, None).unwrap().0;
    let h = history_with(weeks(&[10, 10, 10, 10]), "v", tirane(), NOON, today, NO_CUBE).0;
    let t = raw_by_day(&h, today, &cat, 2).expect("four weeks: a number");
    assert_eq!(t["rice-dry"], vec![1519, 0, 0], "Tuesdays only");
    assert_eq!(t["salt"].len(), 3, "every item a recipe reaches is in the table");
    let expiry = |n: i64| dowiz_hub::stock::meta::day_of_number(today + n);
    assert_eq!(use_until(&t, today, "rice-dry", expiry(1)), Some(1519));
    assert_eq!(use_until(&t, today, "rice-dry", expiry(-1)), Some(0), "already past");
    assert_eq!(use_until(&t, today, "cola", expiry(1)), None, "no recipe reaches it: nothing is claimed");
    let young = history_with(weeks(&[10, 10]), "v", tirane(), NOON, today, NO_CUBE).0;
    assert_eq!(raw_by_day(&young, today, &cat, 2), None, "learning: no table");
}

/// THE DOOR: the handler asks the kitchen's numbers family (the access
/// table's row is held by `access/tests.rs` against the real door).
#[test]
fn the_route_asks_the_numbers_door() {
    let src = include_str!("../forecast.rs");
    assert!(src.contains("guard::staff_venue_as(&req, &ctx, &guard::NUMBERS)"));
    let row = crate::services::identity::staff::access::ROUTES.iter().find(|r| r.1 == "/api/staff/kitchen/prep").expect("a row");
    assert_eq!((row.0, row.2, row.3), ("get", crate::services::identity::staff::access::Door::Staff(&crate::services::identity::staff::guard::NUMBERS), crate::services::identity::staff::access::Kitchen::Read));
}
