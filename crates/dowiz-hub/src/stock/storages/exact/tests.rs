//! FRACTIONAL TRANSFERS (2026-10-17): every storage holds an EXACT level in
//! millionths; a transfer of 0.25 changes both storages by exactly 0.25; the
//! storages sum to the item's exact shelf after EVERY record; an old
//! checkpoint reads with every total unchanged; a retried transfer moves once.

use super::super::*;
use super::{decimal, valid_move_id};
use crate::stock::journal::Journal;
use crate::stock::meta::Meta;
use crate::stock::{draws_for, settle, StockError, StockEvent, StockLog, WasteReason};

const M: i64 = 1_000_000;

fn into(store: &str) -> Meta {
    Meta { store: Some(store.into()), ..Meta::default() }
}
fn frac(store: &str, uq: i64) -> Meta {
    Meta { uq: Some(uq), ..into(store) }
}
fn mv(item: &str, uq: i64, from: &str, to: &str) -> Moved {
    Moved::new(item, uq, from, to, "p1")
}
fn exact(j: &Journal, item: &str) -> Vec<(String, i64)> {
    j.stores.exact_of(item, &j.ledger, &j.carry)
}
fn v(list: &[(&str, i64)]) -> Vec<(String, i64)> {
    list.iter().map(|(s, q)| (s.to_string(), *q)).collect()
}
fn total(j: &Journal, item: &str) -> i64 {
    j.carry.exact(item, j.ledger.level(item).on_hand)
}

/// THE INVARIANT after every record, folded step by step from genesis: for
/// every item, Σ storages' exact == on_hand x 10^6 - carry, and the whole
/// levels still sum to `on_hand`. And the checkpoint fold agrees with genesis.
fn assert_exact_sums(log: &StockLog) {
    let mut j = Journal::default();
    for (n, rec) in log.raw().iter().enumerate() {
        j.step(rec).unwrap();
        for (item, l) in j.ledger.items() {
            let sum: i64 = exact(&j, &item).iter().map(|x| x.1).sum();
            assert_eq!(sum, total(&j, &item), "record {n}: {item}'s storages sum to {sum}, the exact shelf is {}", total(&j, &item));
            let whole: i64 = j.stores.of(&item, &j.ledger).iter().map(|x| x.1).sum();
            assert_eq!(whole, l.on_hand, "record {n}: {item}'s whole levels");
        }
    }
    let now = log.journal_now().unwrap();
    assert_eq!(now.stores, j.stores, "the checkpoint fold lost a storage's fraction");
}

#[test]
fn decimals_and_move_ids() {
    assert_eq!(decimal(750_000), "0.75");
    assert_eq!(decimal(-50_000), "-0.05");
    assert_eq!(decimal(7 * M), "7");
    assert_eq!(decimal(773_810), "0.77381");
    assert_eq!(decimal(0), "0");
    assert!(valid_move_id("mv-7f3a_9") && !valid_move_id("a b") && !valid_move_id("") && !valid_move_id(&"x".repeat(65)));
}

#[test]
fn a_fraction_moves_exactly_and_the_total_never_moves() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 1000, &into(FREEZER)).unwrap();
    log.receive_with("nori", 3, &into("bar")).unwrap();
    assert!(log.move_stock(&mv("salmon", 250_000, FREEZER, "kitchen")).unwrap());
    assert!(log.move_stock(&mv("nori", M / 2, "bar", "kitchen")).unwrap());
    let j = log.journal().unwrap();
    assert_eq!(exact(&j, "salmon"), v(&[(FREEZER, 999_750_000), ("kitchen", 250_000)]));
    assert_eq!(exact(&j, "nori"), v(&[("bar", 2_500_000), ("kitchen", 500_000)]));
    assert_eq!((total(&j, "salmon"), total(&j, "nori")), (1000 * M, 3 * M));
    // And back, to the millionth: the kitchen is empty again.
    log.move_stock(&mv("salmon", 250_000, "kitchen", FREEZER)).unwrap();
    assert_eq!(exact(&log.journal().unwrap(), "salmon"), v(&[(FREEZER, 1000 * M)]));
    assert_exact_sums(&log);
}

#[test]
fn more_than_the_storage_holds_exactly_is_refused_by_name() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 2, &into(FREEZER)).unwrap();
    log.move_stock(&mv("salmon", M / 2, FREEZER, "kitchen")).unwrap();
    let n = log.len();
    let e = log.move_stock(&mv("salmon", M / 2 + 1, "kitchen", "bar")).unwrap_err();
    assert_eq!(e.to_string(), "salmon: kitchen holds only 0.5");
    assert!(matches!(log.move_stock(&mv("salmon", 0, "kitchen", "bar")), Err(StockError::NotPositive { .. })));
    assert_eq!(log.len(), n, "a refused transfer wrote something");
    // TWIN: exactly what it holds.
    log.move_stock(&mv("salmon", M / 2, "kitchen", "bar")).unwrap();
    assert_eq!(exact(&log.journal().unwrap(), "salmon"), v(&[("bar", M / 2), (FREEZER, 3 * M / 2)]));
    assert_exact_sums(&log);
}

#[test]
fn a_retried_transfer_is_written_once() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 2, &into(FREEZER)).unwrap();
    let once = Moved { id: Some("mv-1".into()), ..mv("salmon", 250_000, FREEZER, "kitchen") };
    assert!(log.move_stock(&once).unwrap(), "written");
    let n = log.len();
    assert!(!log.move_stock(&once).unwrap(), "a retry is answered, not written");
    assert_eq!(log.len(), n);
    let other = Moved { uq: M, ..once.clone() };
    assert!(log.move_stock(&other).unwrap_err().to_string().contains("mv-1"));
    assert!(log.move_stock(&Moved { id: Some("a b".into()), ..once.clone() }).is_err());
    assert_eq!(log.len(), n);
    assert_eq!(exact(&log.journal().unwrap(), "salmon"), v(&[(FREEZER, 1_750_000), ("kitchen", 250_000)]));
    // TWIN: another id moves again.
    log.move_stock(&Moved { id: Some("mv-2".into()), ..once }).unwrap();
    assert_eq!(exact(&log.journal().unwrap(), "salmon"), v(&[(FREEZER, 1_500_000), ("kitchen", 500_000)]));
}

/// A whole transfer on an item with no fraction writes the very bytes
/// written before fractions moved (the fixture's record below).
#[test]
fn a_whole_transfer_writes_the_old_bytes() {
    let m = Moved { qty: 1, ..mv("salmon", M, FREEZER, "kitchen") };
    assert_eq!(m.body(), r#"{"item":"salmon","qty":1,"from":"freezer","to":"kitchen","by":"p1"}"#);
    let f = Moved { qty: 0, ..mv("salmon", 250_000, FREEZER, "kitchen") };
    assert_eq!(f.body(), r#"{"item":"salmon","qty":0,"uq":250000,"from":"freezer","to":"kitchen","by":"p1"}"#);
    assert_eq!(Moved::of(&format!(r#"{{"k":"note","note":"moved",{}}}"#, &f.body()[1..f.body().len() - 1])), Some(f));
}

/// Deliveries, write-offs and counts that NAME a storage keep its exact
/// level; a fractional sale (a semi-finished card's 0.3 g) draws from the
/// home. Invariant after every record, through checkpoints every 3.
#[test]
fn every_record_keeps_the_storages_exact() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(3);
    log.receive_with("salt", 3, &frac(FREEZER, 2_500_000)).unwrap();
    log.receive_with("salt", 1, &frac("bar", 750_000)).unwrap();
    let j = log.journal().unwrap();
    assert_eq!(exact(&j, "salt"), v(&[("bar", 750_000), (FREEZER, 2_500_000)]), "the delivery elsewhere kept the freezer's half");
    let waste = StockEvent::Wasted { item: "salt".into(), qty: 1, reason: WasteReason::Spoiled, by: "p".into() };
    log.append_with(&waste, &frac(FREEZER, 500_000)).unwrap();
    log.move_stock(&mv("salt", 1_250_000, FREEZER, "kitchen")).unwrap();
    let plate = r#"{"id":"p","bom":[{"supply":"salt","uq":300000}]}"#;
    for o in ["o1", "o2", "o3"] {
        log.append_draws(&draws_for(o, &[(plate.into(), 1)])).unwrap();
        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, o, o != "o2")).unwrap();
    }
    let j = log.journal().unwrap();
    // 0.75 bar, 2.5 - 0.5 - 1.25 = 0.75 freezer, 1.25 - 0.6 (two sold, one released) kitchen.
    assert_eq!(exact(&j, "salt"), v(&[("bar", 750_000), (FREEZER, 750_000), ("kitchen", 650_000)]));
    // A count of the bar at 0.5: the freezer and the kitchen keep their fractions.
    let count = StockEvent::Stocktake { item: "salt".into(), observed: 2, stocktake_id: "c1".into(), by: "p".into() };
    log.append_with(&count, &frac("bar", 1_900_000)).unwrap();
    let j = log.journal().unwrap();
    assert_eq!(exact(&j, "salt"), v(&[("bar", 500_000), (FREEZER, 750_000), ("kitchen", 650_000)]));
    assert_eq!(total(&j, "salt"), 1_900_000);
    assert!(!j.stores.frac.is_empty() && log.verify_checkpoints().unwrap() > 2);
    assert_exact_sums(&log);
}

/// THE OLD FORMAT, VERBATIM: these records -- the checkpoint included -- were
/// printed by the code BEFORE this file (2026-10-17, a lane's run of the
/// unchanged tree): a 2.5 delivery into the freezer, a whole one into the
/// kitchen, a whole transfer, a 0.3 fractional sale, a checkpoint with its
/// carry (`X`) and storages (`P`), and a write-off after it.
const OLD: [&str; 8] = [
    r#"{"k":"received","item":"salmon","qty":3,"at":1790000000000,"uq":2500000,"store":"freezer"}"#,
    r#"{"k":"received","item":"salmon","qty":2,"at":1790000000000,"store":"kitchen"}"#,
    r#"{"k":"received","item":"rice","qty":1000,"at":1790000000000}"#,
    r#"{"k":"note","note":"moved","item":"salmon","qty":1,"from":"freezer","to":"kitchen","by":"p1","at":1790000000000}"#,
    r#"{"k":"reserved","item":"rice","qty":0,"order":"o1","at":1790000000000,"uq":300000}"#,
    r#"{"k":"consumed","item":"rice","qty":0,"order":"o1","at":1790000000000}"#,
    "{\"k\":\"checkpoint\",\"v\":1,\"at\":1790000000000}\n L 2 6:salmon 5 0 4:rice 1000 0 O 0 S 0 C 2 6:salmon 4:rice B 2 6:salmon 0 0 - 4:rice 0 0 - T 3 6:salmon 2:#0 - - - 1790000000000 - - 3 3 0 6:salmon 2:#1 - - - 1790000000000 - - 2 2 1 4:rice 2:#2 - - - 1790000000000 - - 1000 1000 2 J 5 1790000000000 0 0 X 2 4:rice 300000 6:salmon 500000 0 P 3 4:rice 7:kitchen 1000 6:salmon 7:freezer 2 6:salmon 7:kitchen 3 2 4:rice 7:kitchen 6:salmon 7:kitchen",
    r#"{"k":"wasted","item":"salmon","qty":1,"reason":"spoiled","by":"p1","at":1790000000000,"store":"kitchen"}"#,
];

#[test]
fn an_old_checkpoint_reads_with_totals_unchanged_then_moves_a_fraction_and_reloads() {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(usize::MAX);
    for rec in OLD {
        log.write_payload(rec.as_bytes().to_vec()).unwrap();
    }
    // The totals the OLD code printed for this log: on_hand 4 and 1000,
    // carry 0.5 and 0.3 -- exact 3.5 and 999.7; whole levels 2 + 2.
    let now = log.journal_now().unwrap();
    assert_eq!(now.seen, 6, "the old checkpoint was read as the base (5 rows in it, one after)");
    assert_eq!((total(&now, "salmon"), total(&now, "rice")), (3_500_000, 999_700_000));
    assert_eq!(now.stores.of("salmon", &now.ledger), v(&[(FREEZER, 2), ("kitchen", 2)]), "old whole levels unchanged");
    // Old whole levels are exact with zero fraction; the item's carry stays
    // the item's, with its home (the kitchen): no total changed.
    assert_eq!(exact(&now, "salmon"), v(&[(FREEZER, 2 * M), ("kitchen", 1_500_000)]));
    assert_eq!(exact(&now, "rice"), v(&[("kitchen", 999_700_000)]));
    // From genesis the same totals (the fraction pinned where it was received).
    let all = log.journal().unwrap();
    assert_eq!((total(&all, "salmon"), total(&all, "rice")), (3_500_000, 999_700_000));
    assert_eq!(exact(&all, "salmon").iter().map(|x| x.1).sum::<i64>(), 3_500_000);
    assert_eq!(log.verify_checkpoints().unwrap(), 1, "the old checkpoint still verifies");
    // A fractional transfer on top of it: 0.25 from the kitchen to the bar.
    log.set_checkpoint_every(2);
    log.move_stock(&mv("salmon", 250_000, "kitchen", "bar")).unwrap();
    log.checkpoint_now().unwrap();
    let moved = log.journal_now().unwrap();
    assert_eq!(exact(&moved, "salmon"), v(&[("bar", 250_000), (FREEZER, 2 * M), ("kitchen", 1_250_000)]));
    assert_eq!(total(&moved, "salmon"), 3_500_000);
    // Reloaded from its bytes: the same levels, through the NEW checkpoint (`F`).
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    let again = back.journal_now().unwrap();
    assert_eq!(exact(&again, "salmon"), exact(&moved, "salmon"));
    assert_eq!(again.stores, moved.stores);
    assert_eq!(total(&again, "rice"), 999_700_000);
}
