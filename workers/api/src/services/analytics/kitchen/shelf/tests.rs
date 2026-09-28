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

/// R7: the report read from the newest checkpoint older than its window is
/// the report read from the first record -- every number, and the undatable
/// rows behind the checkpoint counted once.
#[test]
fn a_report_from_a_checkpoint_is_the_report_from_genesis() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(5);
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 5000 }).unwrap(); // undated
    log.append_all(&dowiz_hub::stock::reservations_for("o0", &[(r#"{"bom":[{"supply":"salmon","qty":10}]}"#.into(), 1)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&dowiz_hub::stock::settle(&led, "o0", true)).unwrap(); // undated, dated by o0 if known
    for (i, at) in [100, 400, 900, 1200, 1700, 2200, 2600].into_iter().enumerate() {
        log.set_clock(at);
        log.receive_with("salmon", 100, &Meta { unit_cost: Some(3000 + i as i64), per: Some(1000), ..Meta::default() }).unwrap();
        log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 3, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    }
    assert!(log.verify_checkpoints().unwrap() >= 2);
    for placed in [HashMap::new(), HashMap::from([("o0".to_string(), 50i64)])] {
        let full = fold(&log.journal().unwrap().entries, &supplies(), &placed, &w());
        let j = log.journal_since(w().starts[0]).unwrap();
        assert!(j.entries.len() < log.journal().unwrap().entries.len(), "the report started at a checkpoint");
        assert_eq!(fold_journal(&j, &supplies(), &placed, &w()), full);
    }
    // Twin: from genesis nothing is carried, so the two folds are the same fold.
    let g = log.journal().unwrap();
    assert_eq!(fold_journal(&g, &supplies(), &HashMap::new(), &w()), fold(&g.entries, &supplies(), &HashMap::new(), &w()));
}

/// W-NOM: a deletion is not a movement of food -- it makes no row for the
/// item, not even an empty one, and is not counted as undated; its twin, a
/// dated delivery of another item beside it, is one.
#[test]
fn a_deletion_moves_nothing_in_the_numbers() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(1500);
    log.append(&StockEvent::Received { item: "rice".into(), qty: 10 }).unwrap();
    log.append(&StockEvent::Removed { item: "nori".into(), by: "p_owner".into() }).unwrap();
    let mut undated = StockLog::create_sized(64 * 1024).unwrap();
    undated.append(&StockEvent::Removed { item: "tuna".into(), by: "p_owner".into() }).unwrap();
    let j = log.journal().unwrap();
    let s = fold(&j.entries, &supplies(), &HashMap::new(), &w());
    assert!(!s.moved.contains_key("nori"));
    assert!(s.moved.contains_key("rice"));
    let s = fold(&undated.journal().unwrap().entries, &supplies(), &HashMap::new(), &w());
    assert_eq!((s.undated, s.moved.len()), (0, 0));
}
