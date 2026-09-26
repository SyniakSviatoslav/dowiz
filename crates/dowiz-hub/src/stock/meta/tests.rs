//! I1 (research R2): a dated record, an undated old one, and the keys a
//! record carries besides its movement -- each read back through real bytes.

use super::*;
use crate::stock::{reservations_for, StockLedger, WasteReason};

const MAKI: &str = r#"{"id":"maki","bom":[{"supply":"rice","qty":100}]}"#;

fn recv(item: &str, qty: Qty) -> StockEvent {
    StockEvent::Received { item: item.into(), qty }
}
use crate::stock::Qty;

/// THE OLD-IMAGE ROUND TRIP. A log written with NO clock -- every live
/// venue's log before 2026-09-26 -- reloads, folds to the same ledger, and
/// reads with every key absent. Then the SAME log, with a clock set, keeps
/// trading: the new records carry `at`, the old ones still do not.
#[test]
fn an_undated_image_still_folds_and_new_records_are_dated() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&recv("rice", 1000)).unwrap();
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 2)])).unwrap();
    let old = log.to_bytes_trimmed();
    assert!(!String::from_utf8_lossy(&old).contains("\"at\""), "the fixture really is undated");

    let mut log = StockLog::load(&old).unwrap();
    let before = log.ledger().unwrap().items();
    assert!(log.raw().iter().all(|r| meta_of(r) == Meta::default()), "an old record has no keys");

    log.set_clock(1_790_000_000_000);
    log.append_all(&reservations_for("o2", &[(MAKI.into(), 1)])).unwrap();
    log.append(&recv("rice", 50)).unwrap();
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    let raw = back.raw();
    assert_eq!(raw.len(), 4);
    assert_eq!(meta_of(&raw[0]).at, None);
    assert_eq!(meta_of(&raw[1]).at, None);
    assert_eq!(meta_of(&raw[2]).at, Some(1_790_000_000_000), "the lifecycle's reservation is dated");
    assert_eq!(meta_of(&raw[3]).at, Some(1_790_000_000_000));
    // The shelf reads what it always read.
    let led = back.ledger().unwrap();
    assert_eq!(led.level("rice").on_hand, 1050);
    assert_eq!(led.level("rice").reserved, 300);
    assert_eq!(before, StockLedger::fold(&back.events()[..2]).unwrap().items());
}

/// Every key survives the bytes, and the shelf's decoder is not disturbed
/// by any of them -- a hostile supplier name included.
#[test]
fn every_key_round_trips_and_the_movement_is_untouched() {
    let meta = Meta {
        at: Some(7),
        lot: Some("L2609".into()),
        supplier: Some(r#"Fish "&" Co","qty":9999"#.into()),
        doc: Some("DN-17".into()),
        expiry: Some(20261003),
        unit_cost: Some(1200),
        per: Some(1000),
        session: Some("st_1".into()),
        expected: Some(40),
        value: Some(-12),
        by: Some("p_anna".into()),
    };
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append_with(&recv("salmon", 500), &meta).unwrap();
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(back.events(), vec![recv("salmon", 500)], "qty is 500, not the forged 9999");
    assert_eq!(meta_of(&back.raw()[0]), meta);
    // Twin: no meta writes exactly `encode`'s bytes.
    assert_eq!(with_meta(&encode(&recv("x", 1)), &Meta::default()), encode(&recv("x", 1)));
    // A blank string is not a key.
    let blank = Meta { lot: Some("  ".into()), ..Meta::default() };
    assert_eq!(with_meta("{}", &blank), "{}");
}

/// A signed write-off's own `by` and the meta signer do not collide.
#[test]
fn a_signer_key_does_not_shadow_the_events_own() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&recv("rice", 10)).unwrap();
    let w = StockEvent::Wasted { item: "rice".into(), qty: 1, reason: WasteReason::Spoiled, by: "p_cook".into() };
    log.append_with(&w, &Meta { by: Some("p_other".into()), ..Meta::default() }).unwrap();
    assert_eq!(log.events()[1], w);
    assert_eq!(meta_of(&log.raw()[1]).by.as_deref(), Some("p_other"));
}

/// The write door is the same door: unsigned is refused, a short shelf is
/// refused, and a refused batch writes nothing.
#[test]
fn append_with_keeps_every_refusal_of_append() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&recv("rice", 10)).unwrap();
    let unsigned = StockEvent::Wasted { item: "rice".into(), qty: 1, reason: WasteReason::Spoiled, by: "".into() };
    assert!(log.append_with(&unsigned, &Meta::at(1)).is_err());
    let too_much = StockEvent::Wasted { item: "rice".into(), qty: 16, reason: WasteReason::Spoiled, by: "p".into() };
    assert!(log.append_all_with(&[(recv("rice", 5), Meta::at(1)), (too_much, Meta::at(1))]).is_err());
    assert_eq!(log.len(), 1, "nothing of the refused batch was written");
    let ok = StockEvent::Wasted { item: "rice".into(), qty: 10, reason: WasteReason::Spoiled, by: "p".into() };
    log.append_all_with(&[(recv("rice", 5), Meta::at(1)), (ok, Meta::at(2))]).unwrap();
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 5);
}

/// Days are integers both ways, across a leap day and a year end.
#[test]
fn days_are_integers_both_ways() {
    assert_eq!(day_number(19700101), 0);
    assert_eq!(day_number(20261003) - day_number(20260926), 7);
    assert_eq!(day_number(20240301) - day_number(20240228), 2, "2024 is a leap year");
    assert_eq!(day_number(20270101) - day_number(20261231), 1);
    for d in [19700101, 20240229, 20261231, 20270101, 20991231] {
        assert_eq!(day_of_number(day_number(d)), d);
    }
    assert_eq!(day_of_local_ms(0), 19700101);
    assert_eq!(day_of_local_ms(86_400_000 * 20_000 + 5), day_of_number(20_000));
    assert_eq!(parse_day("2026-10-03"), Some(20261003));
    for bad in ["2026-02-30", "2026-13-01", "26-10-03", "2026/10/03", "", "1999-01-01"] {
        assert_eq!(parse_day(bad), None, "{bad}");
    }
    assert_eq!(show_day(20261003), "2026-10-03");
    assert!(valid_day(20261003) && !valid_day(20261000) && !valid_day(0));
}
