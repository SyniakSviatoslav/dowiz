//! W-NOM: a deletion through the object's turn -- only what the ledger knows
//! is written, twice is harmless, and the catalogue's absence is expected.
use super::super::turn::{run, StockTurnIn};
use super::*;
use std::collections::BTreeMap;

const NOW: i64 = 1_790_000_000_000;

fn body(v: Value) -> StockMoveIn {
    serde_json::from_value(v).unwrap()
}

fn turn(body: Value) -> StockTurnIn {
    // The catalogue has already let the supplies go: the list is empty.
    StockTurnIn { kind: "removed".into(), body, by: "p_owner".into(), now_ms: NOW, today: 20260928, supplies: BTreeMap::new() }
}

#[test]
fn the_plan_is_one_signed_removal_per_distinct_id() {
    let p = plan(&body(json!({ "item": "rice", "items": ["salmon", " rice ", "", "nori"] })), "p_owner", NOW).unwrap();
    let items: Vec<&str> = p.lines.iter().map(|(e, _)| e.item()).collect();
    assert_eq!(items, vec!["rice", "salmon", "nori"]);
    assert!(p.lines.iter().all(|(e, m)| matches!(e, StockEvent::Removed { by, .. } if by == "p_owner") && m.at == Some(NOW)));
    assert_eq!(plan(&body(json!({ "items": [" "] })), "p", NOW).unwrap_err().0, 400, "nothing named is a 400");
    let many: Vec<String> = (0..=MAX_IDS).map(|i| format!("s{i}")).collect();
    assert_eq!(plan(&body(json!({ "items": many })), "p", NOW).unwrap_err().0, 400);
    let exactly: Vec<String> = (0..MAX_IDS).map(|i| format!("s{i}")).collect();
    assert!(plan(&body(json!({ "items": exactly })), "p", NOW).is_ok(), "the limit itself fits");
}

#[test]
fn only_what_the_ledger_knows_is_written_and_twice_is_harmless() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 900 }).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 500 }).unwrap();
    let (shown, told) = run(&mut log, &turn(json!({ "items": ["rice", "ghost"] })), false).expect("not a 404: the catalogue is already empty");
    assert_eq!(shown["removed"], json!(["rice"]));
    assert_eq!(shown["unknown"], json!(["ghost"]));
    assert!(told.is_empty(), "a deletion tells no group anything: it is not a write-off");
    assert_eq!(log.len(), 3);
    let led = log.ledger().unwrap();
    assert_eq!(led.items().iter().map(|(i, _)| i.as_str()).collect::<Vec<_>>(), vec!["salmon"]);

    let (again, _) = run(&mut log, &turn(json!({ "items": ["rice"] })), false).unwrap();
    assert_eq!(again["removed"], json!([]), "deleting twice writes nothing the second time");
    assert_eq!(log.len(), 3);
}

/// Every other kind still names only supplies the catalogue holds.
#[test]
fn any_other_movement_of_an_unknown_supply_is_still_a_404() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let mut t = turn(json!({ "item": "rice", "qty": 5 }));
    t.kind = "received".into();
    assert_eq!(run(&mut log, &t, false).unwrap_err().0, 404);
    assert_eq!(log.len(), 0);
}
