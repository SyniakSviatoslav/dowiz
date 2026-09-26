//! The log's side over a window: dated by the record, else by its order.

use super::*;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{PrepStage, StockLog, WasteReason};

fn w() -> Window {
    Window { starts: vec![1000, 2000], end: 3000, days: vec![20260925, 20260926] }
}
fn supplies() -> HashMap<String, Supply> {
    HashMap::from([("salmon".to_string(), Supply {
        id: "salmon".into(), name: "Salmon".into(), unit: "g".into(), basis: 100, list_cost: None, clean_pm: 580, cook_pm: 1000,
    })])
}

#[test]
fn each_record_lands_on_its_day_and_undated_ones_are_counted_not_guessed() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap(); // undated
    log.set_clock(1500);
    log.receive_with("salmon", 1000, &Meta { unit_cost: Some(3000), per: Some(1000), supplier: Some("Sea".into()), ..Meta::default() }).unwrap();
    log.append_all(&dowiz_hub::stock::reservations_for("o1", &[(r#"{"bom":[{"supply":"salmon","qty":200}]}"#.into(), 1)])).unwrap();
    let led = log.ledger().unwrap();
    // The consumption carries no clock of its own here: it is dated by o1.
    let mut undated = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    undated.append_all(&dowiz_hub::stock::settle(&led, "o1", true)).unwrap();
    let mut log = undated;
    log.set_clock(2500);
    log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 100, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    log.append(&StockEvent::Produced { item: "salmon".into(), qty: 500, out: 280, stage: PrepStage::Clean, into: None, by: "p".into() }).unwrap();
    log.append_with(
        &StockEvent::Stocktake { item: "salmon".into(), observed: 700, stocktake_id: "s".into(), by: "p".into() },
        &Meta { expected: Some(800), ..Meta::default() },
    )
    .unwrap();
    let j = log.journal().unwrap();
    let placed = HashMap::from([("o1".to_string(), 2100i64)]);
    let s = fold(&j.entries, &supplies(), &placed, &w());
    assert_eq!(s.undated, 1, "the first delivery predates dates");
    let m = &s.moved["salmon"];
    assert_eq!((m.received, m.received_value, m.drawn, m.by_day_drawn.clone()), (1000, 3000, 200, vec![0, 200]));
    assert_eq!((m.wasted, m.drift, m.prep_in, m.prep_out), (100, -100, 500, 280));
    assert_eq!(s.waste["spoiled"], (1, 300 * 100 / 100));
    assert_eq!((s.received_by_day.clone(), s.waste_by_day.clone()), (vec![3000, 0], vec![0, s.waste["spoiled"].1]));
    let y = &s.yields[0];
    assert_eq!((y["measuredPm"].clone(), y["expectedPm"].clone(), y["diffPm"].clone()), (json!(560), json!(580), json!(-20)));
    assert_eq!(s.prices["salmon"][0]["perBasis"], json!(300));
    assert_eq!(s.prices["salmon"][0]["day"], json!("2026-09-25"));
}

#[test]
fn outside_the_window_is_left_out() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(5000);
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 10 }).unwrap();
    let j = log.journal().unwrap();
    let s = fold(&j.entries, &supplies(), &HashMap::new(), &w());
    assert!(s.moved.is_empty() && s.undated == 0);
}
