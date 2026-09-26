//! I5: a prep batch -- raw onto the board, cleaned or cooked off it -- as a
//! measurement, or as stock moving from one supply into another.

use super::meta::Meta;
use super::*;

fn prep(qty: Qty, out: Qty, into: Option<&str>, by: &str) -> StockEvent {
    StockEvent::Produced {
        item: "salmon".into(), qty, out, stage: PrepStage::Clean, into: into.map(str::to_string), by: by.into(),
    }
}

/// A MEASUREMENT moves nothing: the shelf is still counted gross.
#[test]
fn a_measurement_moves_nothing() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 5000 }).unwrap();
    log.append(&prep(5000, 2750, None, "p_cook")).unwrap();
    log.append(&prep(9999, 1, Some("salmon"), "p_cook")).unwrap();
    assert_eq!(log.ledger().unwrap().level("salmon").on_hand, 5000, "into itself is a measurement too");
}

/// Moving stock: the input loses `qty`, the output gains `out` and is counted.
/// Refused past the unspoken-for shelf, as waste is; its twin fits exactly.
#[test]
fn a_prep_moves_stock_and_is_refused_past_the_shelf() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 5000 }).unwrap();
    log.append(&StockEvent::Reserved { item: "salmon".into(), qty: 1000, order_id: "o1".into() }).unwrap();
    assert!(matches!(log.append(&prep(4001, 2000, Some("fillet"), "p")), Err(StockError::OutOfStock { .. })));
    assert_eq!(log.len(), 2, "the refusal wrote nothing");
    log.append(&prep(4000, 2200, Some("fillet"), "p")).unwrap();
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel { on_hand: 1000, reserved: 1000 });
    assert_eq!(led.level("fillet").on_hand, 2200);
    assert!(led.is_counted("fillet"));
}

/// Signed, positive, and an output below zero is not an output.
#[test]
fn a_prep_is_signed_and_its_numbers_are_numbers() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    assert!(matches!(log.append(&prep(10, 5, None, " ")), Err(StockError::Unsigned)));
    assert!(matches!(log.append(&prep(0, 5, None, "p")), Err(StockError::NotPositive { .. })));
    assert!(matches!(log.append(&prep(10, -1, None, "p")), Err(StockError::NotPositive { .. })));
    log.append(&prep(10, 0, None, "p")).expect("a total loss is a measurement");
    assert_eq!(signer(&log.events()[1]), Some("p"));
}

/// The record survives the bytes, both forms, and a hostile name.
#[test]
fn a_prep_round_trips() {
    for ev in [
        prep(5000, 2750, None, "p_cook"),
        StockEvent::Produced {
            item: r#"rice","qty":1"#.into(), qty: 500, out: 1100, stage: PrepStage::Cook,
            into: Some("rice-cooked".into()), by: "p".into(),
        },
    ] {
        assert_eq!(decode(&encode(&ev)).as_ref(), Some(&ev));
    }
    assert_eq!(decode(r#"{"k":"produced","item":"x","qty":1,"out":1,"stage":"fry","into":"","by":"p"}"#), None);
    assert_eq!(PrepStage::from_str("cook"), Some(PrepStage::Cook));
}

/// VALUE MOVES WITH THE FOOD: 5 kg of whole fish at 1,000/kg become 2.5 kg of
/// fillet worth the same 5,000 -- 2,000/kg -- and the whole fish's pool is empty.
#[test]
fn a_fillet_costs_what_the_whole_fish_did() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 5000, &Meta { unit_cost: Some(1000), per: Some(1000), ..Meta::default() }).unwrap();
    log.append(&prep(5000, 2500, Some("fillet"), "p")).unwrap();
    let book = log.cost_book();
    assert_eq!(book.wac("fillet", 1000), Some(2000));
    assert_eq!(book.value_of("fillet", 2500), Some(5000));
    assert_eq!(book.wac("salmon", 1000), Some(1000), "the drawn pool keeps its last average");
    // Twin: an unpriced input gives the output no invented cost.
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 5000 }).unwrap();
    log.append(&prep(5000, 2500, Some("fillet"), "p")).unwrap();
    assert_eq!(log.cost_book().wac("fillet", 1000), None);
}
