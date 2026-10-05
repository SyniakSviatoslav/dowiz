//! The `lost` block (A13, W-LOST): per dish per venue day, lek as integers,
//! the window respected, and a kitchen token sees counts without revenue.

use super::*;
use dowiz_hub::stock::refused::LostSale;

fn tirane() -> dowiz_hub::tz::Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("zone")
}
/// 2026-09-26 10:00 UTC = 12:00 in Tirane.
const NOW: i64 = 1_790_416_800_000;
const DAY: i64 = 86_400_000;

fn row(dish: &str, qty: i64, price: i64, at: i64) -> LostRow {
    LostRow { sale: LostSale { item: "salmon".into(), dish: dish.into(), qty, price }, at }
}

fn dishes() -> HashMap<String, Dish> {
    let d = |id: &str, name: &str| (id.to_string(), Dish { id: id.into(), name: name.into(), lines: vec![], leaves: vec![] });
    [d("maki", "Maki salmon"), d("roll", "Philadelphia")].into_iter().collect()
}

#[test]
fn orders_lost_per_dish_per_day_in_lek() {
    let w = super::super::window(tirane(), NOW, None, None, Some("2")).unwrap();
    let rows = vec![
        row("maki", 2, 700, NOW - DAY),          // yesterday
        row("maki", 1, 700, NOW),                // today
        row("roll", 1, 1000, NOW - 60_000),      // today
        row("roll", 1, 1000, NOW - 5 * DAY),     // outside the window
        row("ghost", 1, 300, NOW),               // a dish deleted since: its id
    ];
    let b = report(&rows, &dishes(), &w);
    assert_eq!((b["rows"].as_i64(), b["portions"].as_i64(), b["revenue"].as_i64()), (Some(4), Some(5), Some(2100 + 1000 + 300)));
    assert_eq!(b["contract"], "stock.refused.v1");
    assert_eq!(b["rateLimitedMinutes"], 10);
    let first = &b["dishes"][0];
    assert_eq!((first["id"].as_str(), first["name"].as_str(), first["rows"].as_i64(), first["revenue"].as_i64()), (Some("maki"), Some("Maki salmon"), Some(2), Some(2100)));
    assert_eq!(first["byDay"][0]["revenue"], 1400, "yesterday: 2 x 700");
    assert_eq!(first["byDay"][1]["revenue"], 700);
    assert_eq!(b["dishes"][2]["name"], "ghost", "a dish no longer on the menu keeps its id");
    assert_eq!(b["byDay"][0]["day"], show_day(w.days[0]));
    assert_eq!(b["byDay"][1]["rows"], 3);
    assert!(b["revenue"].is_i64(), "integer lek, never a float");
}

#[test]
fn no_refusal_is_an_empty_block_not_a_missing_one() {
    let w = super::super::window(tirane(), NOW, None, None, None).unwrap();
    let b = report(&[], &dishes(), &w);
    assert_eq!((b["rows"].as_i64(), b["revenue"].as_i64()), (Some(0), Some(0)));
    assert_eq!(b["byDay"].as_array().map(Vec::len), Some(7), "one entry per day of the window");
    assert!(b["dishes"].as_array().unwrap().is_empty());
}

#[test]
fn the_kitchen_reads_counts_and_never_the_revenue() {
    let w = super::super::window(tirane(), NOW, None, None, Some("1")).unwrap();
    let b = report(&[row("maki", 2, 700, NOW)], &dishes(), &w);
    let staff = crate::services::identity::staff::access::numbers_for_kitchen(json!({ "lost": b }));
    let s = staff["lost"].to_string();
    assert!(!s.contains("revenue") && !s.contains("1400"), "no lek reaches a kitchen token: {s}");
    assert_eq!(staff["lost"]["dishes"][0]["rows"], 1, "the count does");
}

#[test]
fn the_kitchen_answer_carries_the_block_from_the_stock_log() {
    let cat = dowiz_hub::catalog::Catalog::create().unwrap();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(16 * 1024).unwrap();
    stock.set_clock(NOW);
    stock.append_lost(&LostSale { item: "salmon".into(), dish: "maki".into(), qty: 1, price: 700 }).unwrap();
    let cold = |_: i64, _: i64| -> std::result::Result<std::collections::BTreeMap<i64, crate::services::analytics::cube::DayCube>, String> { Ok(std::collections::BTreeMap::new()) };
    let out = super::super::answer_with(vec![], &cat, &stock, "loc", NOW, (None, None, Some("1")), &cold).unwrap();
    assert_eq!(out["lost"]["rows"], 1);
    assert_eq!(out["lost"]["revenue"], 700);
}
