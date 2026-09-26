//! P1-4: food back from a door. Resold, it is back on the shelf; wasted, the
//! shelf does not move (`Consumed` already took it); the choice is visible in
//! the stock log alone; and a log written before `returned` existed folds as
//! it did.

use super::*;

fn ret(item: &str, qty: Qty, resell: bool, chosen_by: &str) -> StockEvent {
    StockEvent::Returned {
        item: item.into(), qty, order_id: "o1".into(), resell,
        by: "courier-7".into(), chosen_by: chosen_by.into(),
    }
}

/// Rice 1000; o1 reserved and consumed 200 at PREPARING, went out, came back.
fn cooked_and_back() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append_all(&[
        StockEvent::Received { item: "rice".into(), qty: 1000 },
        StockEvent::Reserved { item: "rice".into(), qty: 200, order_id: "o1".into() },
        StockEvent::Consumed { item: "rice".into(), qty: 200, order_id: "o1".into() },
    ])
    .unwrap();
    log
}

#[test]
fn resold_food_goes_back_on_the_shelf() {
    let mut log = cooked_and_back();
    log.append(&ret("rice", 200, true, "owner1")).expect("lands");
    assert_eq!(log.ledger().unwrap().level("rice"), StockLevel { on_hand: 1000, reserved: 0 });
}

#[test]
fn wasted_food_does_not_come_off_the_shelf_twice() {
    let mut log = cooked_and_back();
    log.append(&ret("rice", 200, false, "owner1")).expect("lands");
    assert_eq!(log.ledger().unwrap().level("rice"), StockLevel { on_hand: 800, reserved: 0 });
    // Twin: the blueprint's literal `Wasted` would have taken it off again.
    let mut lit = cooked_and_back();
    lit.append(&StockEvent::Wasted { item: "rice".into(), qty: 200, reason: WasteReason::Returned, by: "courier-7".into() })
        .unwrap();
    assert_eq!(lit.ledger().unwrap().level("rice").on_hand, 600, "the double count this variant avoids");
}

#[test]
fn a_waste_choice_lands_even_when_the_rest_of_the_shelf_is_promised() {
    let mut log = cooked_and_back();
    // Everything left is reserved by o2: a `Wasted` would be refused.
    log.append(&StockEvent::Reserved { item: "rice".into(), qty: 800, order_id: "o2".into() }).unwrap();
    log.append(&ret("rice", 200, false, "owner1")).expect("waste of consumed food is not a draw on the shelf");
}

#[test]
fn the_choice_is_signed_and_a_quantity() {
    let mut log = cooked_and_back();
    assert!(matches!(log.append(&ret("rice", 200, true, " ")), Err(StockError::Unsigned)));
    assert!(matches!(log.append(&ret("rice", 0, true, "owner1")), Err(StockError::NotPositive { .. })));
    assert!(log.append(&ret("rice", 1, true, "owner1")).is_ok());
    assert_eq!(signer(&ret("rice", 1, true, "owner1")), Some("owner1"));
}

#[test]
fn the_lines_and_the_marker_come_from_the_log() {
    let mut log = cooked_and_back();
    log.append_all(&[
        StockEvent::Received { item: "nori".into(), qty: 1 },
        StockEvent::Reserved { item: "nori".into(), qty: 2, order_id: "o1".into() },
    ])
    .unwrap_err(); // reserve beyond a counted shelf: refused, nothing written
    log.append(&StockEvent::Received { item: "nori".into(), qty: 10 }).unwrap();
    log.append(&StockEvent::Reserved { item: "nori".into(), qty: 2, order_id: "o1".into() }).unwrap();
    log.append(&StockEvent::Consumed { item: "nori".into(), qty: 2, order_id: "o1".into() }).unwrap();
    assert_eq!(returned_lines(&log, "o1"), (false, vec![("rice".into(), 200), ("nori".into(), 2)]));
    assert_eq!(returned_lines(&log, "o2"), (false, vec![]), "another order's lines are not these");
    log.append(&ret("rice", 200, false, "owner1")).unwrap();
    assert!(returned_lines(&log, "o1").0, "chosen");
    assert!(!returned_lines(&log, "o2").0);
}

#[test]
fn a_returned_record_round_trips() {
    for r in [true, false] {
        let ev = ret("nori \"x\"", 3, r, "owner1");
        assert_eq!(decode(&encode(&ev)), Some(ev));
    }
    assert_eq!(decode(r#"{"k":"returned","item":"x","qty":1,"order":"o","resell":2,"by":"c","chosen_by":"p"}"#), None);
    assert_eq!(decode(r#"{"k":"returned","item":"x","qty":1,"resell":1,"by":"c","chosen_by":"p"}"#), None, "no order");
}

/// OLD BYTES: the literal records of a log from before `returned`, reloaded.
#[test]
fn a_log_written_before_returned_existed_folds_unchanged() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for rec in [
        r#"{"k":"received","item":"rice","qty":1000}"#,
        r#"{"k":"reserved","item":"rice","qty":200,"order":"o1"}"#,
        r#"{"k":"consumed","item":"rice","qty":200,"order":"o1"}"#,
        r#"{"k":"wasted","item":"rice","qty":10,"reason":"spoiled"}"#,
        r#"{"k":"served","item":"rice","qty":5,"order":"ebills:u1"}"#,
    ] {
        log.write_payload(rec.as_bytes().to_vec()).unwrap();
    }
    let old = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(old.ledger().unwrap().level("rice"), StockLevel { on_hand: 785, reserved: 0 });
    let mut grown = old;
    grown.append(&ret("rice", 200, true, "owner1")).unwrap();
    let back = StockLog::load(&grown.to_bytes_trimmed()).unwrap();
    assert_eq!(back.ledger().unwrap().level("rice"), StockLevel { on_hand: 985, reserved: 0 });
    assert!(returned_lines(&back, "o1").0);
}
