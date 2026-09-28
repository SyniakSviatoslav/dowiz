//! W-NOM (2026-09-28): an ingredient DELETED from the nomenclature. The log
//! keeps its history (append-only); every fold treats the item as gone.

use super::meta::Meta;
use super::*;

fn gone(item: &str) -> StockEvent {
    StockEvent::Removed { item: item.into(), by: "p_owner".into() }
}

fn priced(log: &mut StockLog, item: &str, qty: Qty, unit_cost: i64, lot: &str) {
    let m = Meta { unit_cost: Some(unit_cost), per: Some(1000), lot: Some(lot.into()), expiry: Some(20261010), ..Meta::default() };
    log.receive_with(item, qty, &m).unwrap();
}

/// The deletion drops the shelf, the count mark, the holds, the lots and the
/// cost pool of THAT item only -- and the history before it stays in the log.
#[test]
fn a_removed_item_is_gone_from_every_fold_and_its_neighbours_are_untouched() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    priced(&mut log, "salmon", 5000, 2000, "L1");
    priced(&mut log, "rice", 9000, 300, "L2");
    log.append(&StockEvent::Reserved { item: "salmon".into(), qty: 200, order_id: "o1".into() }).unwrap();
    log.append(&StockEvent::Reserved { item: "rice".into(), qty: 90, order_id: "o1".into() }).unwrap();
    let before = log.len();
    log.append(&gone("salmon")).unwrap();
    assert_eq!(log.len(), before + 1, "the deletion is a record: the log only grows");

    let led = log.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel::default());
    assert!(!led.is_counted("salmon"), "a re-created salmon starts uncounted, not with the old shelf");
    assert!(led.items().iter().all(|(i, _)| i != "salmon"), "not listed");
    assert_eq!(led.stranded(), vec![("o1".to_string(), "rice".to_string(), 90)], "the salmon hold went with it; rice's stayed");
    assert_eq!(settle(&led, "o1", true).len(), 1, "the open order settles only what still exists");

    let j = log.journal().unwrap();
    assert!(j.lots.of("salmon").is_empty(), "no lot of a deleted item");
    assert_eq!(j.lots.of("rice").len(), 1);
    assert_eq!(j.book.wac("salmon", 1000), None, "no price survives the deletion");
    assert_eq!(j.book.wac("rice", 1000), Some(300));
    assert_eq!(led, j.ledger, "the journal and the shelf agree");
    assert!(j.entries.iter().any(|e| matches!(e.ev, StockEvent::Received { ref item, .. } if item == "salmon")), "history stays");

    // A supply re-created under the same id starts from nothing.
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    assert_eq!(log.ledger().unwrap().level("salmon").on_hand, 100);
}

/// Deleting is not a write-off: it has no quantity, moves no waste, and the
/// order lifecycle keeps working for the orders that held it.
#[test]
fn a_deletion_is_not_a_write_off_and_orders_still_move() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 50 }).unwrap();
    log.append(&StockEvent::Reserved { item: "nori".into(), qty: 5, order_id: "o1".into() }).unwrap();
    log.append(&gone("nori")).unwrap();
    // The order is cooked: nothing of nori is held any more, so nothing is consumed or refused.
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap();
    assert!(log.ledger().unwrap().stranded().is_empty());
    let j = log.journal().unwrap();
    let e = j.entries.iter().find(|e| matches!(e.ev, StockEvent::Removed { .. })).expect("the deletion row");
    assert_eq!(e.value, None, "a deletion carries no value: it is not waste");
}

/// Signed like a write-off; idempotent (a second deletion of nothing is fine);
/// the record round-trips; the checkpoint fold equals the fold from genesis.
#[test]
fn a_deletion_is_signed_idempotent_round_trips_and_checkpoints() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    assert!(matches!(log.append(&StockEvent::Removed { item: "x".into(), by: " ".into() }), Err(StockError::Unsigned)));
    assert_eq!(log.len(), 0, "an unsigned deletion wrote nothing");
    log.append(&gone("never-seen")).unwrap();
    log.append(&gone("never-seen")).unwrap();
    assert_eq!(decode(&encode(&gone("a\"b"))), Some(gone("a\"b")));
    assert_eq!(signer(&gone("a")), Some("p_owner"));

    log.set_checkpoint_every(3);
    log.append(&StockEvent::Received { item: "tuna".into(), qty: 10 }).unwrap();
    log.append(&gone("tuna")).unwrap();
    log.append(&StockEvent::Received { item: "eel".into(), qty: 4 }).unwrap();
    log.append(&gone("eel")).unwrap();
    assert!(log.verify_checkpoints().unwrap() >= 1);
    assert_eq!(log.ledger().unwrap(), StockLedger::fold(&log.events()).unwrap());
    assert_eq!(log.ledger().unwrap().items(), vec![]);
}
