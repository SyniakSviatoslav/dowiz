//! P12: storages sum to the shelf after every record, a transfer never makes
//! or destroys stock, a sale draws from where the delivery went, and a
//! storage with stock in it is never archived.

use super::*;
use crate::stock::meta::Meta;
use crate::stock::oldimage_tests::{fold_text, old_history};
use crate::stock::journal::Journal;
use crate::stock::{reservations_for, settle, PrepStage, StockError, StockEvent, StockLog, WasteReason};

const ROLL: &str = r#"{"id":"r","bom":[{"supply":"salmon","qty":100},{"supply":"rice","qty":50}]}"#;

fn into(store: &str) -> Meta {
    Meta { store: Some(store.into()), ..Meta::default() }
}
fn mv(item: &str, qty: Qty, from: &str, to: &str) -> Moved {
    Moved { item: item.into(), qty, from: from.into(), to: to.into(), by: "p1".into() }
}
fn sell(log: &mut StockLog, order: &str, portions: i64) {
    log.append_all(&reservations_for(order, &[(ROLL.into(), portions)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, order, true)).unwrap();
}
fn levels(log: &StockLog, item: &str) -> Vec<(String, Qty)> {
    let j = log.journal().unwrap();
    j.stores.of(item, &j.ledger)
}
fn s(v: &[(&str, Qty)]) -> Vec<(String, Qty)> {
    v.iter().map(|(a, b)| (a.to_string(), *b)).collect()
}

/// THE INVARIANT, record by record: for every item, the storages sum to the
/// ledger's `on_hand`. Folded step by step over the raw log.
fn assert_sums(log: &StockLog) {
    let mut j = Journal::default();
    for (n, rec) in log.raw().iter().enumerate() {
        j.step(rec).unwrap();
        for (item, l) in j.ledger.items() {
            let sum: Qty = j.stores.of(&item, &j.ledger).iter().map(|x| x.1).sum();
            assert_eq!(sum, l.on_hand, "record {n}: {item}'s storages sum to {sum}, the shelf says {}", l.on_hand);
        }
    }
}

/// A log with no storage key is the default storage's, item by item, and the
/// golden old image still folds and writes byte-equal (see `oldimage_tests`).
#[test]
fn an_old_log_is_all_in_the_default_storage() {
    let log = old_history();
    let j = log.journal().unwrap();
    assert!(!j.stores.is_used());
    for (item, l) in j.ledger.items() {
        let want = if l.on_hand == 0 { vec![] } else { vec![(DEFAULT.to_string(), l.on_hand)] };
        assert_eq!(j.stores.of(&item, &j.ledger), want, "{item}");
        assert_eq!(j.stores.level(&item, "bar", &j.ledger), 0);
    }
    // The fold through the newest checkpoint agrees with the one from genesis.
    assert_eq!(fold_text(&log.journal_now().unwrap()), fold_text(&j));
}

/// A transfer moves stock between storages and changes NOTHING else: the
/// ledger, the cost book, the lots and the carry are byte-equal before and
/// after, and the item's storages sum to the same total.
#[test]
fn a_transfer_never_makes_or_destroys_stock() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 3000, &Meta { unit_cost: Some(2000), per: Some(1000), ..into(FREEZER) }).unwrap();
    log.receive_with("rice", 5000, &into("kitchen")).unwrap();
    let before = log.journal().unwrap();
    log.move_stock(&mv("salmon", 500, FREEZER, "kitchen")).unwrap();
    let after = log.journal().unwrap();
    assert_eq!(fold_text(&after), fold_text(&before), "a transfer moved the shelf, the cost, a lot or the carry");
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 2500), ("kitchen", 500)]));
    let total = |j: &Journal| j.stores.of("salmon", &j.ledger).iter().map(|x| x.1).sum::<Qty>();
    assert_eq!(total(&after), total(&before), "the sum over storages moved");
    // More than the storage holds is refused, nothing written.
    let n = log.len();
    assert!(matches!(log.move_stock(&mv("salmon", 501, "kitchen", "bar")), Err(StockError::OutOfStock { available: 500, .. })));
    // Same storage twice, an unknown storage, unsigned, not a quantity.
    assert!(log.move_stock(&mv("salmon", 1, "kitchen", "kitchen")).is_err());
    assert!(log.move_stock(&mv("salmon", 1, "kitchen", "cellar")).is_err());
    assert!(matches!(log.move_stock(&Moved { by: " ".into(), ..mv("salmon", 1, "kitchen", "bar") }), Err(StockError::Unsigned)));
    assert!(matches!(log.move_stock(&mv("salmon", 0, "kitchen", "bar")), Err(StockError::NotPositive { .. })));
    assert_eq!(log.len(), n, "a refused transfer wrote something");
    assert_sums(&log);
}

/// Poster's rule: a sale with no storage of its own draws from the storage
/// that LAST RECEIVED the item -- a transfer's destination included.
#[test]
fn a_sale_draws_from_the_storage_that_last_received() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 2000, &into(FREEZER)).unwrap();
    log.receive_with("rice", 5000, &into("kitchen")).unwrap();
    sell(&mut log, "o1", 2);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1800)]), "the freezer received the salmon last");
    log.move_stock(&mv("salmon", 600, FREEZER, "kitchen")).unwrap();
    sell(&mut log, "o2", 1);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1200), ("kitchen", 500)]), "after the transfer the kitchen is where it was received");
    assert_eq!(levels(&log, "rice"), s(&[("kitchen", 4850)]));
    // A draw that names its storage takes from there.
    let waste = StockEvent::Wasted { item: "salmon".into(), qty: 200, reason: WasteReason::Spoiled, by: "p".into() };
    log.append_with(&waste, &into(FREEZER)).unwrap();
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1000), ("kitchen", 500)]));
    assert_sums(&log);
}

/// A storage with stock in it cannot be archived; emptied, it can. The
/// default storage never can. Archived, it receives no transfer.
#[test]
fn a_storage_with_stock_is_never_archived() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.put_storage(&Storage { id: "cellar".into(), name: "Cellar".into(), archived: false }).unwrap();
    log.receive_with("wine", 12, &into("cellar")).unwrap();
    let n = log.len();
    let archive = Storage { id: "cellar".into(), name: "Cellar".into(), archived: true };
    assert!(log.put_storage(&archive).is_err(), "archived with 12 of wine in it");
    assert_eq!(log.len(), n);
    log.move_stock(&mv("wine", 12, "cellar", "bar")).unwrap();
    log.put_storage(&archive).unwrap();
    assert!(log.storages().iter().any(|s| s.id == "cellar" && s.archived));
    assert!(log.move_stock(&mv("wine", 1, "bar", "cellar")).is_err(), "an archived storage received a transfer");
    assert!(log.put_storage(&Storage { id: DEFAULT.into(), name: "K".into(), archived: true }).is_err());
    // Renamed, the id stays.
    log.put_storage(&Storage { id: "bar".into(), name: "Bar & lounge".into(), archived: false }).unwrap();
    assert_eq!(log.storages().iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["kitchen", "bar", "freezer", "cellar"]);
    assert!(log.put_storage(&Storage { id: "Bad Id".into(), name: "x".into(), archived: false }).is_err());
}

/// A count in one storage sets THAT storage, and the shelf's total is the
/// counted storage plus the others (the write door sends the total).
#[test]
fn a_count_lands_in_its_storage() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 1000, &into(FREEZER)).unwrap();
    log.move_stock(&mv("salmon", 400, FREEZER, "kitchen")).unwrap();
    // Counted 350 in the kitchen: the total the ledger is told is 600 + 350.
    let count = StockEvent::Stocktake { item: "salmon".into(), observed: 950, stocktake_id: "c1".into(), by: "p".into() };
    log.append_with(&count, &into("kitchen")).unwrap();
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 600), ("kitchen", 350)]));
    assert_sums(&log);
}

/// An uncounted item drawn negative, then delivered to ANOTHER storage: the
/// first delivery's forgiveness raises the negative storage to zero, it does
/// not inflate the receiving one. Prep into a fillet lands where it was cut.
#[test]
fn a_first_delivery_and_prep_keep_the_sum() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("rice", 100, &into("kitchen")).unwrap();
    sell(&mut log, "o1", 1); // salmon uncounted: -100 in the default storage
    assert_eq!(levels(&log, "salmon"), s(&[("kitchen", -100)]));
    log.receive_with("salmon", 2000, &into(FREEZER)).unwrap();
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 2000)]), "the delivery is 2000, and the uncounted use is forgiven");
    log.move_stock(&mv("salmon", 1000, FREEZER, "kitchen")).unwrap();
    let cut = StockEvent::Produced { item: "salmon".into(), qty: 1000, out: 600, stage: PrepStage::Clean, into: Some("fillet".into()), by: "p".into() };
    log.append_with(&cut, &into("kitchen")).unwrap();
    assert_eq!(levels(&log, "fillet"), s(&[("kitchen", 600)]));
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1000)]));
    log.append(&StockEvent::Removed { item: "fillet".into(), by: "p".into() }).unwrap();
    assert_eq!(levels(&log, "fillet"), vec![]);
    assert_sums(&log);
}

/// THE CHECKPOINT CARRIES THE STORAGES: fold(all) == fold(checkpoint + tail),
/// and `verify_checkpoints` re-folds every stored state from genesis.
#[test]
fn checkpoints_carry_the_storages() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_checkpoint_every(3);
    for k in 0..12 {
        log.receive_with("salmon", 1000, &into(if k % 2 == 0 { FREEZER } else { "bar" })).unwrap();
        log.receive_with("rice", 500, &Meta::default()).unwrap();
        log.move_stock(&mv("salmon", 300, if k % 2 == 0 { FREEZER } else { "bar" }, "kitchen")).unwrap();
        sell(&mut log, &format!("o{k}"), 1);
    }
    assert!(log.verify_checkpoints().unwrap() > 3);
    let all = log.journal().unwrap();
    let now = log.journal_now().unwrap();
    assert_eq!(now.stores, all.stores, "the checkpoint lost a storage");
    assert!(all.stores.is_used());
    assert_sums(&log);
}
