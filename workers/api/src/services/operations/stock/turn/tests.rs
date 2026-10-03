//! W0a: a movement as one turn -- the shelf changed, and what the groups are
//! told about it; a refusal changes nothing and tells nothing.
use super::*;
use dowiz_hub::stock::StockEvent;
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;
const TODAY: i64 = 20260926;

fn supplies() -> BTreeMap<String, SupplyIn> {
    supplies_of(vec![
        ("rice".into(), json!({ "name": "Rice", "unit": "g", "lowAt": 1000, "shelfDays": 3 }).to_string()),
        ("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string()),
    ])
}

fn input(kind: &str, body: Value) -> StockTurnIn {
    StockTurnIn { kind: kind.into(), body, by: "p_anna".into(), now_ms: NOW, today: TODAY, supplies: supplies(), currency: String::new() }
}

fn log() -> StockLog {
    StockLog::create_sized(64 * 1024).unwrap()
}

fn keys(t: &Told) -> Vec<&str> {
    t.iter().map(|(k, _)| *k).collect()
}

#[test]
fn a_supplys_words_and_thresholds_are_read_from_the_catalogue() {
    let s = supplies();
    assert_eq!(s["rice"], SupplyIn { name: "Rice".into(), unit: "g".into(), low_at: 1000, shelf_days: Some(3), record: None, linked: false });
    assert_eq!(s["salmon"].low_at, 0);
    let odd = supplies_of(vec![("x".into(), "not json".into())]);
    assert_eq!(odd["x"].name, "x", "an unreadable record is named by its id");
}

#[test]
fn a_delivery_is_written_and_tells_received_with_its_lot_and_date() {
    let mut l = log();
    let (shown, told) = run(&mut l, &input("received", json!({ "item": "rice", "qty": 5000, "lot": "L42" })), false).unwrap();
    assert_eq!(shown["kind"], "received");
    assert_eq!(l.len(), 1);
    assert_eq!(keys(&told), vec!["stock.received"]);
    assert_eq!(told[0].1, json!({ "name": "Rice", "qty": 5000, "unit": "g", "lot": "L42", "expiry": "2026-09-29" }), "shelfDays fills the date");
}

#[test]
fn a_write_off_tells_wasted_and_the_low_crossing_it_caused() {
    let mut l = log();
    run(&mut l, &input("received", json!({ "item": "rice", "qty": 1500 })), false).unwrap();
    let (_, told) = run(&mut l, &input("wasted", json!({ "item": "rice", "qty": 600, "reason": "spoiled" })), false).unwrap();
    assert_eq!(keys(&told), vec!["stock.wasted", "stock.low"]);
    assert_eq!(told[0].1, json!({ "name": "Rice", "qty": 600, "unit": "g", "reason": "spoiled" }));
    assert_eq!(told[1].1["items"][0]["on_hand"], 900);
    // ITS TWIN: already below, a second write-off does not say "low" again.
    let (_, told) = run(&mut l, &input("wasted", json!({ "item": "rice", "qty": 100, "reason": "spoiled" })), false).unwrap();
    assert_eq!(keys(&told), vec!["stock.wasted"]);
}

#[test]
fn a_count_tells_only_its_drift() {
    let mut l = log();
    run(&mut l, &input("received", json!({ "item": "rice", "qty": 3000 })), false).unwrap();
    run(&mut l, &input("received", json!({ "item": "salmon", "qty": 800 })), false).unwrap();
    let body = json!({ "lines": [{ "item": "rice", "observed": 2800 }, { "item": "salmon", "observed": 800 }] });
    let (_, told) = run(&mut l, &input("count", body), false).unwrap();
    assert_eq!(keys(&told), vec!["stocktake.variance"]);
    assert_eq!(told[0].1, json!({ "items": [{ "name": "Rice", "expected": 3000, "observed": 2800, "unit": "g" }] }));
    // A count that matches tells nothing.
    let (_, told) = run(&mut l, &input("stocktake", json!({ "item": "salmon", "observed": 800 })), false).unwrap();
    assert!(told.is_empty());
}

#[test]
fn the_first_movement_of_the_day_tells_the_lots_near_their_date() {
    let mut l = log();
    run(&mut l, &input("received", json!({ "item": "salmon", "qty": 800, "expiry": "2026-09-27" })), false).unwrap();
    run(&mut l, &input("received", json!({ "item": "rice", "qty": 5000, "expiry": "2026-12-01" })), false).unwrap();
    let (_, told) = run(&mut l, &input("received", json!({ "item": "rice", "qty": 10 })), true).unwrap();
    let exp = told.iter().find(|(k, _)| *k == "stock.expiring").expect("due today");
    assert_eq!(exp.1, json!({ "items": [{ "name": "Salmon", "qty": 800, "unit": "g", "expiry": "2026-09-27" }] }));
    // ITS TWIN: not due, not told.
    let (_, told) = run(&mut l, &input("received", json!({ "item": "rice", "qty": 10 })), false).unwrap();
    assert!(!keys(&told).contains(&"stock.expiring"));
}

#[test]
fn a_refusal_writes_nothing_and_tells_nothing() {
    let mut l = log();
    let e = run(&mut l, &input("wasted", json!({ "item": "rice", "qty": 5, "reason": "soggy" })), false).unwrap_err();
    assert_eq!(e.0, 400);
    let e = run(&mut l, &input("received", json!({ "item": "tuna", "qty": 5 })), false).unwrap_err();
    assert_eq!(e, (404, "not found: tuna".to_string()));
    let e = run(&mut l, &input("received", json!({ "item": "rice", "qty": 5, "by": "p_boss" })), false).unwrap_err();
    assert_eq!(e.0, 400, "a body cannot name its signer");
    assert_eq!(l.len(), 0);
    // Positive twin: the same delivery of a known supply is written.
    assert!(run(&mut l, &input("received", json!({ "item": "rice", "qty": 5 })), false).is_ok());
    assert_eq!(l.len(), 1);
    assert!(matches!(l.events()[0], StockEvent::Received { .. }));
}
