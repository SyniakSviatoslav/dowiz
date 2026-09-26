//! The kitchen report, end to end over a real log and real order JSON: by day,
//! by dish, per ingredient, and days of cover. Every number traced by hand.

use super::*;
use crate::services::analytics::kitchen::{sales, shelf};
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{StockEvent, StockLog, WasteReason};
use sales::DishLine;

fn world() -> (HashMap<String, Dish>, HashMap<String, Supply>, Window) {
    let sup = |id: &str, list| Supply { id: id.into(), name: id.to_uppercase(), unit: "g".into(), basis: 100, list_cost: list, clean_pm: 550, cook_pm: 1000 };
    let dishes = HashMap::from([(
        "sake".to_string(),
        Dish { id: "sake".into(), name: "Sake".into(), lines: vec![DishLine { supply: "salmon".into(), qty: 100, gross_g: Some(100), net_g: Some(55), out_g: Some(55) }] },
    )]);
    let supplies = HashMap::from([("salmon".to_string(), sup("salmon", Some(250)))]);
    (dishes, supplies, Window { starts: vec![0, 1000], end: 2000, days: vec![20260925, 20260926] })
}

#[test]
fn a_day_a_dish_and_an_ingredient_add_up() {
    let (dishes, supplies, w) = world();
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(10);
    log.receive_with("salmon", 1000, &Meta { unit_cost: Some(2000), per: Some(1000), ..Meta::default() }).unwrap();
    log.set_clock(1500);
    log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 50, reason: WasteReason::Dropped, by: "p".into() }).unwrap();
    let orders = vec![
        serde_json::json!({ "id": "o1", "created_at_ms": 100, "status": "DELIVERED", "items": [{ "product_id": "sake", "quantity": 2, "unit_price": 1000 }] }),
        serde_json::json!({ "id": "o2", "created_at_ms": 1100, "status": "DELIVERED", "items": [{ "product_id": "sake", "quantity": 1, "unit_price": 1000 }] }),
    ];
    let j = log.journal().unwrap();
    let s = sales::fold(&orders, &dishes, &w);
    let sh = shelf::fold(&j.entries, &supplies, &s.placed_at, &w);
    let r = report(&s, &sh, &dishes, &supplies, &j, &w);

    // A portion is 100 g at the AVERAGE 2/g (not the 2.5/g list): 200.
    let d = &r["dishes"][0];
    assert_eq!((d["sold"].clone(), d["revenue"].clone(), d["portionCost"].clone(), d["cogs"].clone()), (json!(3), json!(3000), json!(200), json!(600)));
    assert_eq!((d["margin"].clone(), d["marginPortion"].clone(), d["foodCostPm"].clone()), (json!(2400), json!(800), json!(200)));
    assert_eq!(r["byDay"][0], json!({ "day": "2026-09-25", "orders": 1, "revenue": 2000, "cogs": 400, "foodCostPm": 200, "waste": 0, "received": 2000 }));
    assert_eq!((r["byDay"][1]["cogs"].clone(), r["byDay"][1]["waste"].clone()), (json!(200), json!(100)));
    let i = &r["ingredients"][0];
    assert_eq!((i["used"].clone(), i["grossG"].clone(), i["netG"].clone(), i["cleanLossG"].clone()), (json!(300), json!(300), json!(165), json!(135)));
    assert_eq!((i["cost"].clone(), i["wasted"].clone(), i["wastedValue"].clone(), i["received"].clone()), (json!(600), json!(50), json!(100), json!(1000)));
    // 300 g over 2 days = 150 a day; 950 on the shelf = 6 days; no reorder.
    assert_eq!((i["adu"].clone(), i["daysCover"].clone(), i["reorder"].clone()), (json!(150), json!(6), Value::Null));
    let t = &r["totals"];
    assert_eq!((t["revenue"].clone(), t["cogs"].clone(), t["foodCostPm"].clone(), t["wasteValue"].clone()), (json!(3000), json!(600), json!(200), json!(100)));
    assert_eq!(r["waste"], json!([{ "reason": "dropped", "rows": 1, "value": 100 }]));
    assert_eq!((r["from"].clone(), r["to"].clone()), (json!("2026-09-25"), json!("2026-09-26")));
}

/// Days of cover: rounded-up daily use, a hint only under three days, and
/// nothing for an ingredient nobody counted (its zero is unknown).
#[test]
fn days_of_cover_and_the_reorder_hint() {
    assert_eq!(cover(950, 300, 2, true), (Some(150), Some(6), None));
    assert_eq!(cover(200, 300, 2, true), (Some(150), Some(1), Some(850)), "7 days of 150, minus the 200 there");
    assert_eq!(cover(0, 301, 2, true), (Some(151), Some(0), Some(1057)));
    assert_eq!(cover(200, 300, 2, false), (Some(150), None, None), "uncounted: no cover claimed");
    assert_eq!(cover(200, 0, 2, true), (None, None, None), "unused: nothing to say");
}

/// WHAT ONE REQUEST COSTS, measured natively (run with `--ignored
/// --nocapture`): a year-sized stock log and a busy month of orders through
/// the same three folds the route runs. A native debug build, so an order of
/// magnitude for the Worker, not its number.
#[test]
#[ignore]
fn measure_a_kitchen_request() {
    let (dishes, supplies, _) = world();
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(10);
    log.receive_with("salmon", 1_000_000, &Meta { unit_cost: Some(2000), per: Some(1000), ..Meta::default() }).unwrap();
    let roll = r#"{"bom":[{"supply":"salmon","qty":100}]}"#.to_string();
    let mut orders = Vec::new();
    for i in 0..2_500 {
        let id = format!("o{i}");
        log.append_all(&dowiz_hub::stock::reservations_for(&id, &[(roll.clone(), 1)])).unwrap();
        let led = log.ledger().unwrap();
        log.append_all(&dowiz_hub::stock::settle(&led, &id, true)).unwrap();
        orders.push(serde_json::json!({ "id": id, "created_at_ms": 100 + i, "status": "DELIVERED",
            "items": [{ "product_id": "sake", "quantity": 1, "unit_price": 1000 }] }));
    }
    let w = Window { starts: (0..30).map(|d| d * 1000).collect(), end: 30_000, days: (0..30).map(|d| 20260901 + d).collect() };
    let tl = std::time::Instant::now();
    let led = log.ledger().unwrap();
    println!("baseline: ledger() alone -- what every placement already folds -- {:?} ({} items)", tl.elapsed(), led.items().len());
    let t0 = std::time::Instant::now();
    let j = log.journal().unwrap();
    let t1 = std::time::Instant::now();
    let s = sales::fold(&orders, &dishes, &w);
    let sh = shelf::fold(&j.entries, &supplies, &s.placed_at, &w);
    let r = report(&s, &sh, &dishes, &supplies, &j, &w);
    let t2 = std::time::Instant::now();
    assert_eq!(r["dishes"][0]["sold"], json!(2500));
    println!(
        "kitchen request: {} stock records folded in {:?}; {} orders reported in {:?}",
        j.entries.len(), t1 - t0, orders.len(), t2 - t1
    );
}
