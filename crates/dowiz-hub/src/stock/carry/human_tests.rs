//! W-QTYHUB H3 (CONTRACT-QTY §1): 2.5 pieces received, 0.75 counted, 0.25
//! thrown away -- each booked whole on the shelf with its exact `uq` beside it,
//! the exact level never drifting, and every OLD record byte for byte.

use super::*;
use crate::stock::{decode, encode, WasteReason};

fn log() -> StockLog {
    StockLog::create_sized(256 * 1024).unwrap()
}
fn uq(u: i64) -> Meta {
    Meta { uq: Some(u), ..Meta::default() }
}
fn exact(log: &StockLog, item: &str) -> i64 {
    let (led, carry) = log.shelf_exact().unwrap();
    carry.exact(item, led.level(item).on_hand)
}
fn waste(item: &str) -> StockEvent {
    StockEvent::Wasted { item: item.into(), qty: 1, reason: WasteReason::Spoiled, by: "p_anna".into() }
}
fn count(item: &str) -> StockEvent {
    StockEvent::Stocktake { item: item.into(), observed: 0, stocktake_id: "st_1".into(), by: "p_anna".into() }
}

#[test]
fn two_and_a_half_pieces_received_twice_is_five_exactly() {
    let mut log = log();
    log.receive_with("nori", 0, &uq(2_500_000)).unwrap();
    assert_eq!(exact(&log, "nori"), 2_500_000, "the exact shelf is what was typed");
    assert_eq!(log.ledger().unwrap().level("nori").on_hand, 3, "booked whole, half up");
    log.receive_with("nori", 0, &uq(2_500_000)).unwrap();
    assert_eq!(log.ledger().unwrap().level("nori").on_hand, 5);
    assert_eq!(exact(&log, "nori"), 5_000_000);
    assert_eq!(log.shelf_exact().unwrap().1.of("nori"), 0, "no remainder left over");
    let raw = log.raw();
    assert!(raw[0].contains(r#""qty":3"#) && raw[0].contains(r#""uq":2500000"#), "{}", raw[0]);
    assert_eq!(log.verify_checkpoints(), Ok(0));
}

#[test]
fn a_count_of_three_quarters_and_a_quarter_wasted_leaves_half() {
    let mut log = log();
    log.receive_with("nori", 4, &Meta::default()).unwrap();
    log.append_with(&count("nori"), &uq(750_000)).unwrap();
    assert_eq!(exact(&log, "nori"), 750_000);
    assert_eq!(log.ledger().unwrap().level("nori").on_hand, 1);
    log.append_with(&waste("nori"), &uq(250_000)).unwrap();
    assert_eq!(exact(&log, "nori"), 500_000, "0.75 - 0.25");
    // A journal from genesis agrees with the tail fold.
    let j = log.journal().unwrap();
    assert_eq!(j.carry.exact("nori", j.ledger.level("nori").on_hand), 500_000);
}

#[test]
fn a_whole_count_clears_a_remainder_and_a_whole_move_writes_the_old_bytes() {
    let mut log = log();
    log.receive_with("nori", 0, &uq(2_500_000)).unwrap();
    log.append_with(&count("nori"), &uq(4_000_000)).unwrap();
    assert_eq!(exact(&log, "nori"), 4_000_000, "the count is the shelf, whatever the carry was");
    assert_eq!(log.shelf_exact().unwrap().1.of("nori"), 0);
    // Carry is zero now: a whole count, delivery and write-off carry no `uq`.
    log.append_with(&count("nori"), &uq(4_000_000)).unwrap();
    log.receive_with("nori", 0, &uq(10_000_000)).unwrap();
    log.append_with(&waste("nori"), &uq(2_000_000)).unwrap();
    let raw = log.raw();
    let n = raw.len();
    assert_eq!(raw[n - 3], r#"{"k":"stocktake","item":"nori","observed":4,"id":"st_1","by":"p_anna"}"#);
    assert_eq!(raw[n - 2], r#"{"k":"received","item":"nori","qty":10}"#);
    assert_eq!(raw[n - 1], r#"{"k":"wasted","item":"nori","qty":2,"reason":"spoiled","by":"p_anna"}"#);
    assert_eq!(exact(&log, "nori"), 12_000_000);
}

/// THE PINNED OLD RECORDS: what a live log already holds decodes to the same
/// values, folds the same, and moves no carry.
#[test]
fn old_records_decode_and_fold_exactly_as_before() {
    let old = [
        r#"{"k":"received","item":"salt","qty":2000,"at":1758600000000,"unit_cost":120,"per":1000}"#,
        r#"{"k":"wasted","item":"salt","qty":15,"reason":"dropped"}"#,
        r#"{"k":"stocktake","item":"salt","observed":1900,"id":"st_9","at":1758600000001,"session":"st_9","expected":1985}"#,
    ];
    assert_eq!(decode(old[0]), Some(StockEvent::Received { item: "salt".into(), qty: 2000 }));
    assert_eq!(decode(old[1]), Some(StockEvent::Wasted { item: "salt".into(), qty: 15, reason: WasteReason::Dropped, by: String::new() }));
    assert_eq!(
        decode(old[2]),
        Some(StockEvent::Stocktake { item: "salt".into(), observed: 1900, stocktake_id: "st_9".into(), by: String::new() })
    );
    assert_eq!(encode(&decode(old[0]).unwrap()), r#"{"k":"received","item":"salt","qty":2000}"#);
    let mut c = Carry::default();
    for r in old {
        c.apply(&decode(r).unwrap(), crate::minijson::int_field(r, "uq").filter(|u| *u > 0));
    }
    assert_eq!(c, Carry::default(), "no old record moves the carry");
}

#[test]
fn a_fractional_delivery_onto_an_owed_remainder_still_books_at_least_one() {
    let mut log = log();
    log.receive_with("salt", 10, &Meta::default()).unwrap();
    // 0.4 g drawn, booked 0: the carry owes +0.4.
    log.append_draws(&[Draw { item: "salt".into(), uq: 400_000, order_id: "o1".into(), via: None, stations: Vec::new() }]).unwrap();
    let before = exact(&log, "salt");
    assert_eq!(before, 9_600_000);
    log.receive_with("salt", 0, &uq(300_000)).unwrap();
    assert_eq!(exact(&log, "salt"), 9_900_000, "exact whatever was booked");
    assert_eq!(log.ledger().unwrap().level("salt").on_hand, 11, "a delivery books at least 1");
    log.set_checkpoint_every(1);
    log.append_with(&waste("salt"), &uq(900_000)).unwrap();
    assert_eq!(exact(&log, "salt"), 9_000_000);
    assert!(log.verify_checkpoints().unwrap() >= 1, "the carry rides the checkpoint");
}
