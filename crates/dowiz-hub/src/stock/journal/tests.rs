//! The one pass agrees with the three walks it replaces, and values each row
//! at the average OF ITS OWN MOMENT.

use super::*;
use crate::stock::meta::Meta;
use crate::stock::{reservations_for, settle, PrepStage, WasteReason};

const MAKI: &str = r#"{"id":"maki","bom":[{"supply":"rice","qty":100}]}"#;

fn at_price(cost: i64) -> Meta {
    Meta { unit_cost: Some(cost), per: Some(1000), supplier: Some("Agro".into()), ..Meta::default() }
}
fn waste(item: &str, qty: Qty) -> StockEvent {
    StockEvent::Wasted { item: item.into(), qty, reason: WasteReason::Spoiled, by: "p".into() }
}

/// A history with every kind in it: the journal's shelf equals `ledger()`,
/// its book equals `cost_book()`, row for row the same events as `events()`.
#[test]
fn the_one_pass_equals_the_walks_it_replaces() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("rice", 2000, &at_price(1000)).unwrap();
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 3)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap();
    log.append(&waste("rice", 100)).unwrap();
    log.receive_with("rice", 1000, &at_price(1300)).unwrap();
    log.append(&StockEvent::Stocktake { item: "rice".into(), observed: 2500, stocktake_id: "s".into(), by: "p".into() })
        .unwrap();
    log.append(&StockEvent::Produced {
        item: "rice".into(), qty: 500, out: 1100, stage: PrepStage::Cook, into: Some("rice-cooked".into()), by: "p".into(),
    })
    .unwrap();
    let j = log.journal().unwrap();
    assert_eq!(j.ledger.items(), log.ledger().unwrap().items());
    assert_eq!(j.book, log.cost_book());
    assert_eq!(j.entries.iter().map(|e| e.ev.clone()).collect::<Vec<_>>(), log.events());
    assert_eq!(j.ledger.level("rice-cooked").on_hand, 1100);
    assert_eq!(j.ledger.level("rice").on_hand, 2000);
}

/// Each row's value is read BEFORE it moves the book: the write-off after a
/// 1,000/kg delivery is worth 1,000/kg even though a dearer one came later.
#[test]
fn a_row_is_valued_at_its_own_moment() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("rice", 1000, &at_price(1000)).unwrap();
    log.append(&waste("rice", 100)).unwrap();
    log.receive_with("rice", 900, &at_price(2000)).unwrap();
    log.append(&waste("rice", 100)).unwrap();
    let j = log.journal().unwrap();
    let v: Vec<Option<i64>> = j.entries.iter().map(|e| e.value).collect();
    // 1000 g at 1000/kg = 1000; 100 g = 100; 900 g at 2000/kg = 1800;
    // then the pool is 900 g @1 + 900 g @2 = 1.5/g, so 100 g = 150.
    assert_eq!(v, vec![Some(1000), Some(100), Some(1800), Some(150)]);
    // A stored value wins over the re-fold (the write door stamped it).
    let mut log2 = StockLog::create_sized(64 * 1024).unwrap();
    log2.receive_with("rice", 1000, &at_price(1000)).unwrap();
    log2.append_with(&waste("rice", 100), &Meta { value: Some(77), ..Meta::default() }).unwrap();
    assert_eq!(log2.journal().unwrap().entries[1].value, Some(77));
}

/// A count's `expected` is the stored one when there is one, else the fold's
/// `before`; its value is the drift at the average, signed.
#[test]
fn a_count_carries_its_drift_and_its_value() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("tuna", 1000, &Meta { unit_cost: Some(3000), per: Some(1000), ..Meta::default() }).unwrap();
    let count = |o| StockEvent::Stocktake { item: "tuna".into(), observed: o, stocktake_id: "s1".into(), by: "p".into() };
    // An OLD count: no `expected` key -- derived from the fold.
    log.append(&count(900)).unwrap();
    // A new count with its `expected` stored.
    log.append_with(&count(950), &Meta { expected: Some(900), session: Some("s2".into()), ..Meta::default() }).unwrap();
    let j = log.journal().unwrap();
    assert_eq!((j.entries[1].expected(), j.entries[1].value), (1000, Some(-300)), "100 g short at 3/g");
    assert_eq!((j.entries[2].expected(), j.entries[2].value), (900, Some(150)), "50 g over");
    // Unpriced: no value, never a zero that looks like one.
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 10 }).unwrap();
    log.append(&waste("nori", 1)).unwrap();
    assert_eq!(log.journal().unwrap().entries[1].value, None);
}

/// Half up, and a basis that is not one is no price.
#[test]
fn priced_rounds_half_up_and_refuses_a_zero_basis() {
    assert_eq!(priced(333, 1000, 1000), Some(333));
    assert_eq!(priced(1, 1, 2), Some(1));
    assert_eq!(priced(1, 1, 3), Some(0));
    assert_eq!(priced(10, 5, 0), None);
}

/// An unreadable record is skipped, as `events()` skips it.
#[test]
fn an_unreadable_record_is_skipped() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.write_payload(b"not json".to_vec()).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 5 }).unwrap();
    let j = log.journal().unwrap();
    assert_eq!(j.entries.len(), 1);
    assert_eq!(j.entries[0].seq, 0);
}
