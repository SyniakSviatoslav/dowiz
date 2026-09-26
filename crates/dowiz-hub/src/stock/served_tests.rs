//! BLUEPRINT-EBILLS §6.4: a dish the till already served is RECORDED, never
//! refused for the shelf -- and a stock log written before `served` existed
//! folds to exactly the ledger it always did.

use super::*;

fn served(item: &str, qty: Qty, order: &str) -> StockEvent {
    StockEvent::Served { item: item.into(), qty, order_id: order.into() }
}

/// The dish was eaten. The shelf said 1; the ledger now says -1 and names it.
#[test]
fn a_served_dish_past_an_empty_shelf_is_recorded_and_goes_negative() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    log.append(&served("salmon", 2, "ebills:u1")).expect("served is never refused for the shelf");
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel { on_hand: -1, reserved: 0 });
    assert_eq!(short(&led), vec![("salmon".to_string(), -1)]);
    // THE 86 STILL HOLDS: nothing is available to a delivery order.
    assert!(matches!(
        led.decide(&StockEvent::Reserved { item: "salmon".into(), qty: 1, order_id: "o2".into() }),
        Err(StockError::OutOfStock { .. })
    ));
    // A delivery settles it.
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 5 }).unwrap();
    let led = log.ledger().unwrap();
    assert_eq!(led.available("salmon"), 4);
    assert!(short(&led).is_empty());
}

/// Its positive twin: with stock on the shelf it is an ordinary subtraction,
/// and it holds nothing -- there is no reservation to settle or strand.
#[test]
fn a_served_dish_on_a_stocked_shelf_subtracts_and_holds_nothing() {
    let led = StockLedger::fold(&[
        StockEvent::Received { item: "rice".into(), qty: 100 },
        served("rice", 30, "ebills:u2"),
    ])
    .unwrap();
    assert_eq!(led.level("rice"), StockLevel { on_hand: 70, reserved: 0 });
    assert!(led.stranded().is_empty());
    assert!(settle(&led, "ebills:u2", true).is_empty());
}

/// Only the arithmetic refuses it: a quantity that is not one, or an overflow.
#[test]
fn a_served_quantity_must_be_a_quantity() {
    let led = StockLedger::default();
    for q in [0, -3] {
        assert!(matches!(led.decide(&served("x", q, "o")), Err(StockError::NotPositive { .. })));
    }
    // Counted, so the shelf moves; an uncounted item's never does.
    let count = StockEvent::Stocktake { item: "x".into(), observed: 0, stocktake_id: "st".into(), by: "m".into() };
    let deep = StockLedger::fold(&[count, served("x", i64::MAX, "o")]).unwrap();
    assert!(matches!(deep.decide(&served("x", 2, "o")), Err(StockError::Overflow)));
    assert!(led.decide(&served("x", 1, "o")).is_ok());
}

#[test]
fn a_served_record_round_trips() {
    let ev = served("nori \"x\"", 3, "ebills:0000");
    assert_eq!(decode(&encode(&ev)), Some(ev));
    assert_eq!(decode(r#"{"k":"served","item":"x","qty":1}"#), None, "no order, no record");
}

/// OLD BYTES, NOT A FRESH ENCODE: the literal records every venue's stock log
/// held before this variant existed, laid down with `write_payload` and
/// reloaded from the image. The fold must be what it was, and a `served`
/// appended afterwards must fold on top of it.
#[test]
fn a_log_written_before_served_existed_folds_unchanged() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for rec in [
        r#"{"k":"received","item":"salmon","qty":500}"#,
        r#"{"k":"reserved","item":"salmon","qty":50,"order":"o1"}"#,
        r#"{"k":"consumed","item":"salmon","qty":50,"order":"o1"}"#,
        r#"{"k":"reserved","item":"salmon","qty":20,"order":"o2"}"#,
        r#"{"k":"wasted","item":"salmon","qty":10,"reason":"spoiled"}"#,
        r#"{"k":"stocktake","item":"rice","observed":40,"id":"st1"}"#,
    ] {
        log.write_payload(rec.as_bytes().to_vec()).unwrap();
    }
    let old = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    let led = old.ledger().expect("the old history folds");
    assert_eq!(led.level("salmon"), StockLevel { on_hand: 440, reserved: 20 });
    assert_eq!(led.level("rice"), StockLevel { on_hand: 40, reserved: 0 });
    assert_eq!(old.events().len(), 6);

    let mut grown = old;
    grown.append(&served("salmon", 5, "ebills:u3")).unwrap();
    let back = StockLog::load(&grown.to_bytes_trimmed()).unwrap();
    assert_eq!(back.ledger().unwrap().level("salmon"), StockLevel { on_hand: 435, reserved: 20 });
}

fn unserved(item: &str, qty: Qty, order: &str) -> StockEvent {
    StockEvent::Unserved { item: item.into(), qty, order_id: order.into() }
}

/// A VOID PUTS BACK exactly what its sale served -- once. A second reversal,
/// a larger one, or one for an order that served nothing is refused.
#[test]
fn a_void_puts_back_what_was_served_and_never_twice() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    log.append(&served("salmon", 80, "ebills:a")).unwrap();
    let led = log.ledger().unwrap();
    assert_eq!(led.served_of("ebills:a"), vec![("salmon".to_string(), 80)]);
    assert!(matches!(led.decide(&unserved("salmon", 81, "ebills:a")), Err(StockError::Linkage(_))), "more than served");
    assert!(matches!(led.decide(&unserved("salmon", 1, "ebills:b")), Err(StockError::Linkage(_))), "nothing served");
    log.append(&unserved("salmon", 80, "ebills:a")).expect("its own draw goes back");
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salmon").on_hand, 100);
    assert!(led.served_of("ebills:a").is_empty());
    assert!(log.append(&unserved("salmon", 80, "ebills:a")).is_err(), "never twice");
    assert_eq!(decode(&encode(&unserved("x", 2, "o"))), Some(unserved("x", 2, "o")));
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(back.ledger().unwrap().level("salmon").on_hand, 100, "survives the image");
}
