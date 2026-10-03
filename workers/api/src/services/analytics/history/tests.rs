//! The owner's numbers over any period, pinned against the reference fold.

use super::*;
use crate::services::analytics::{cube, fold};
use serde_json::json;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}
/// 2026-09-22 12:00 UTC.
const NOON: i64 = 1_790_078_400_000;
const DAY: i64 = 86_400_000;

fn order(at: i64, total: i64, status: &str, kind: &str, dish: &str, q: i64, phone: &str) -> Value {
    json!({ "created_at_ms": at, "total": total, "tip": 0, "status": status, "fulfilment": { "kind": kind },
            "contact": { "phone": phone }, "items": [{ "product_id": dish, "quantity": q, "unit_price": total / q.max(1) }] })
}
fn no_name(id: &str) -> Value {
    json!(id)
}
fn week() -> Vec<Value> {
    vec![
        order(NOON, 1000, "DELIVERED", "delivery", "sake", 2, "+355691"),
        order(NOON - 3 * DAY, 3000, "PICKED_UP", "pickup", "maki", 3, "+355692"),
        order(NOON - 3 * DAY + 3_600_000, 5000, "REJECTED", "delivery", "sake", 1, "+355691"),
        order(NOON - 6 * DAY, 2000, "PICKED_UP", "dine_in", "uramaki", 1, ""),
        order(NOON - 7 * DAY, 9000, "DELIVERED", "delivery", "sake", 9, "+355693"), // the day before the window
    ]
}

/// THE CUBE'S NUMBERS ARE THE FOLD'S NUMBERS over the same orders: every v1
/// field of the pane, whichever side of the hot log a day is on.
#[test]
fn the_v1_fields_equal_the_reference_fold_whether_a_day_is_hot_or_archived() {
    let z = tirane();
    let os = week();
    let r = fold::fold(&os, z, &fold::day_starts(z, NOON, 7), NOON);
    let s = span(z, NOON, Some("7"), None, None).unwrap();
    let all = cube::fold_orders(&os, z, NOON);
    // Every split of the days between the cube and the hot log gives the same pane.
    for cut in [0usize, 2, 7] {
        let cold: BTreeMap<i64, cube::DayCube> = all.iter().take(cut).map(|(k, v)| (*k, v.clone())).collect();
        let hot: BTreeMap<i64, cube::DayCube> = all.iter().skip(cut).map(|(k, v)| (*k, v.clone())).collect();
        let v = report(z, s, &cold, &hot, &no_name);
        assert_eq!(v["orders"], r.orders, "cut {cut}");
        assert_eq!(v["revenue"], r.revenue);
        assert_eq!(v["rejected"], r.rejected);
        assert_eq!(v["averageOrder"], r.average_order);
        assert_eq!((v["delivery"].as_i64(), v["pickup"].as_i64(), v["dineIn"].as_i64()), (Some(r.delivery), Some(r.pickup), Some(r.dine_in)));
        assert_eq!(v["byHour"], json!(r.by_hour.to_vec()));
        assert_eq!(v["byChannel"], json!(r.by_channel));
        let days: Vec<(i64, i64, i64)> = v["byDay"].as_array().unwrap().iter().map(|d| (d["at"].as_i64().unwrap(), d["orders"].as_i64().unwrap(), d["revenue"].as_i64().unwrap())).collect();
        assert_eq!(days, r.by_day.iter().map(|d| (d.at, d.orders, d.revenue)).collect::<Vec<_>>());
        let top: Vec<(String, i64, i64)> = v["topProducts"].as_array().unwrap().iter().map(|d| (d["id"].as_str().unwrap().to_string(), d["quantity"].as_i64().unwrap(), d["revenue"].as_i64().unwrap())).collect();
        assert_eq!(top, r.top_products.iter().map(|d| (d.id.clone(), d.quantity, d.revenue)).collect::<Vec<_>>());
    }
}

#[test]
fn the_period_is_a_named_window_or_a_range_and_the_rest_is_refused() {
    let z = tirane();
    let s = span(z, NOON, Some("30"), None, None).unwrap();
    assert_eq!((s.n, s.days().1), (30, 20260922));
    assert_eq!(span(z, NOON, Some("365"), None, None).unwrap().n, 365);
    let r = span(z, NOON, None, Some("2026-01-01"), Some("2026-03-31")).unwrap();
    assert_eq!((r.n, r.days()), (90, (20260101, 20260331)));
    assert_eq!(span(z, NOON, None, Some("2025-09-22"), Some("2026-09-22")).unwrap().n, 366, "the longest allowed");
    assert!(span(z, NOON, None, Some("2025-09-21"), Some("2026-09-22")).unwrap_err().contains("366"));
    assert!(span(z, NOON, None, Some("2026-09-23"), Some("2026-09-22")).is_err());
    assert!(span(z, NOON, None, Some("2026-02-30"), None).is_err());
}

#[test]
fn the_previous_period_and_the_same_weekday_are_compared() {
    let z = tirane();
    let os = vec![
        order(NOON, 1000, "DELIVERED", "delivery", "sake", 1, ""),
        order(NOON - 7 * DAY, 400, "DELIVERED", "delivery", "sake", 1, ""),
        order(NOON - 14 * DAY, 800, "DELIVERED", "delivery", "sake", 1, ""),
        order(NOON - 21 * DAY, 1200, "DELIVERED", "delivery", "sake", 1, ""),
        order(NOON - 28 * DAY, 1600, "DELIVERED", "delivery", "sake", 1, ""),
    ];
    let s = span(z, NOON, Some("7"), None, None).unwrap();
    let v = report(z, s, &BTreeMap::new(), &cube::fold_orders(&os, z, NOON), &no_name);
    let prev = &v["compare"]["prev"];
    assert_eq!((prev["from"].as_str(), prev["to"].as_str()), (Some("2026-09-09"), Some("2026-09-15")));
    assert_eq!((prev["revenue"].as_i64(), prev["delta"]["revenue"].as_i64(), prev["deltaPm"]["revenue"].as_i64()), (Some(400), Some(600), Some(1500)));
    let wd = &v["compare"]["weekday"];
    assert_eq!((wd["revenue"].as_i64(), wd["lastWeek"]["revenue"].as_i64(), wd["lastWeek"]["day"].as_str()), (Some(1000), Some(400), Some("2026-09-15")));
    assert_eq!(wd["average"]["revenue"], 1000, "(400 + 800 + 1200 + 1600) / 4");
    assert_eq!(wd["weekday"], 1, "2026-09-22 is a Tuesday");
    assert_eq!(v["byWeekdayHour"][1][14], 1, "Tuesday, 14:00 local");
}

/// A PERIOD WITH NOTHING BEFORE IT says so with a zero and no ratio, never a
/// division by zero.
#[test]
fn a_first_period_has_no_ratio_against_nothing() {
    let z = tirane();
    let os = vec![order(NOON, 1000, "DELIVERED", "delivery", "sake", 1, "")];
    let v = report(z, span(z, NOON, None, None, None).unwrap(), &BTreeMap::new(), &cube::fold_orders(&os, z, NOON), &no_name);
    assert_eq!(v["compare"]["prev"]["revenue"], 0);
    assert!(v["compare"]["prev"]["deltaPm"]["revenue"].is_null());
}

#[test]
fn best_and_worst_hours_dish_trend_and_channel_money() {
    let z = tirane();
    let s = span(z, NOON, Some("30"), None, None).unwrap();
    let mut os: Vec<Value> = (0..4).map(|h| order(NOON + h * 3_600_000 - 4 * 3_600_000, 1000 * (h + 1), "DELIVERED", "delivery", "sake", 1, "")).collect();
    os.push(order(NOON - 40 * DAY, 700, "DELIVERED", "delivery", "sake", 1, "")); // the period before
    let mut till = order(NOON - DAY, 500, "DELIVERED", "pickup", "maki", 2, "");
    till["channel"] = json!("ebills");
    os.push(till);
    let v = report(z, s, &BTreeMap::new(), &cube::fold_orders(&os, z, NOON), &no_name);
    let hours = |k: &str| v["hours"][k].as_array().unwrap().iter().map(|h| h["hour"].as_i64().unwrap()).collect::<Vec<_>>();
    assert_eq!(hours("best"), vec![13, 12, 11], "by money");
    assert_eq!(hours("worst"), vec![14, 10], "the rest, least first; never a best hour again");
    let sake = v["trend"]["dishes"].as_array().unwrap().iter().find(|d| d["id"] == "sake").unwrap().clone();
    assert_eq!((sake["quantity"].as_i64(), sake["prevQuantity"].as_i64()), (Some(4), Some(1)));
    assert_eq!(sake["series"].as_array().unwrap().len(), 30, "one point a day up to a month");
    assert_eq!(sake["series"][29], 4);
    let ch: Vec<(String, i64, i64)> = v["channels"].as_array().unwrap().iter().map(|c| (c["channel"].as_str().unwrap().into(), c["orders"].as_i64().unwrap(), c["revenue"].as_i64().unwrap())).collect();
    assert_eq!(ch, vec![("storefront".into(), 4, 10000), ("ebills".into(), 1, 500)]);
    let y = report(z, span(z, NOON, Some("365"), None, None).unwrap(), &BTreeMap::new(), &BTreeMap::new(), &no_name);
    assert_eq!(y["trend"]["buckets"].as_array().unwrap().len(), 53, "a point a week over a year");
    assert_eq!(y["byDay"].as_array().unwrap().len(), 365);
}

/// COUNTS, NEVER PEOPLE: two orders by one phone make one repeat customer; a
/// refused order, a phone-less one and one outside the period do not count.
#[test]
fn repeat_customers_are_counts_over_accepted_orders_with_a_phone() {
    let z = tirane();
    let os = vec![
        order(NOON, 1000, "DELIVERED", "delivery", "sake", 1, "+355 69 111"),
        order(NOON - DAY, 1000, "DELIVERED", "delivery", "sake", 1, "00355 69 111"),
        order(NOON - DAY, 1000, "DELIVERED", "delivery", "sake", 1, "+355 69 222"),
        order(NOON - DAY, 1000, "REJECTED", "delivery", "sake", 1, "+355 69 222"),
        order(NOON - DAY, 1000, "DELIVERED", "delivery", "sake", 1, ""),
        order(NOON - 9 * DAY, 1000, "DELIVERED", "delivery", "sake", 1, "+355 69 222"),
    ];
    let r = repeat(&os, z, span(z, NOON, Some("7"), None, None).unwrap());
    assert_eq!((r["customers"].as_i64(), r["repeatCustomers"].as_i64()), (Some(2), Some(1)));
    assert_eq!((r["orders"].as_i64(), r["repeatOrders"].as_i64(), r["sharePm"].as_i64()), (Some(3), Some(2), Some(666)));
    assert!(r.to_string().find("355").is_none(), "no phone leaves the fold: {r}");
}

/// THE 10 MS BUDGET, MEASURED (W-HIST card): a year's read is the image's
/// range parse (the period, the one before it, four weeks) plus the report.
/// 365 days of a busy venue: 120 orders a day over 40 dishes. Run with
/// `cargo test --lib cpu_of_a_year -- --ignored --nocapture`; it prints µs
/// and the image's size, and fails only above a generous native ceiling.
#[test]
#[ignore]
fn cpu_of_a_year_read() {
    let z = tirane();
    let mut cube = cube::Cube::default();
    let mut parts = BTreeMap::new();
    for k in 0..730 {
        let d = dowiz_hub::stock::meta::day_of_number(dowiz_hub::stock::meta::day_number(20260922) - k);
        let mut r = cube::DayCube::new(d);
        r.o = 120;
        r.t = 120 * 1500;
        r.f = r.t;
        r.k = [80, 30, 10];
        for h in 10..23 {
            r.h[h] = 9;
            r.ht[h] = 9 * 1500;
        }
        r.c.insert("storefront".into(), [100, 150_000]);
        r.c.insert("ebills".into(), [20, 30_000]);
        for i in 0..40 {
            r.m.insert(format!("dish-{i:03}"), [3, 4500, 3, 1200]);
        }
        parts.insert(d, r);
    }
    cube.absorb("log@1", parts);
    let bytes = cube.encode();
    let s = span(z, NOON, Some("365"), None, None).unwrap();
    let t0 = std::time::Instant::now();
    let lo = dowiz_hub::stock::meta::day_of_number(s.prev().first.min(s.last() - 28));
    let rows = cube::Cube::decode_range(&bytes, lo, s.days().1).unwrap().rows;
    let parsed = t0.elapsed();
    let v = report(z, s, &rows, &BTreeMap::new(), &no_name);
    let all = t0.elapsed();
    // WARM: the object keeps the parse per image (`hubdo/cube.rs` `parsed`), so a
    // read after the first is the range copy and the report.
    let whole = cube::Cube::decode(&bytes).unwrap();
    let t1 = std::time::Instant::now();
    let warm_rows: BTreeMap<i64, cube::DayCube> = whole.rows.range(lo..=s.days().1).map(|(d, r)| (*d, r.clone())).collect();
    let _ = report(z, s, &warm_rows, &BTreeMap::new(), &no_name);
    let warm = t1.elapsed();
    println!("CPU-YEAR image={}B rows={} parse={}us parse+report={}us warm(range+report)={}us", bytes.len(), rows.len(), parsed.as_micros(), all.as_micros(), warm.as_micros());
    assert_eq!(v["orders"], 365 * 120);
    assert!(all.as_millis() < 2_000, "a year's read took {all:?}");
}
