use super::*;

#[test]
fn the_log_survives_the_byte_image() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 500 }).unwrap();
    log.append(&StockEvent::Reserved {
        item: "salmon".into(), qty: 50, order_id: "o1".into()
    }).unwrap();
    let bytes = log.to_bytes();

    let log = StockLog::load(&bytes).expect("load");
    assert_eq!(log.events().len(), 2);
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel { on_hand: 500, reserved: 50 });
    assert_eq!(led.available("salmon"), 450);
}

/// THE ORDER OF A FOLD IS NOT NEGOTIABLE. `EvLog::walk` returns newest
/// first; applying that order makes a reservation land before the delivery
/// that made it possible, and the ledger refuses its own history. This is
/// the regression test for exactly that -- it failed with
/// `OutOfStock { wanted: 50, available: 0 }` on a log holding 500.
#[test]
fn a_reloaded_log_replays_in_the_order_it_happened() {
    let mut log = StockLog::create().expect("create");
    for ev in [
        StockEvent::Received { item: "salmon".into(), qty: 500 },
        StockEvent::Reserved { item: "salmon".into(), qty: 50, order_id: "o1".into() },
        StockEvent::Consumed { item: "salmon".into(), qty: 50, order_id: "o1".into() },
        StockEvent::Received { item: "salmon".into(), qty: 100 },
    ] {
        log.append(&ev).expect("append");
    }
    let replayed = StockLog::load(&log.to_bytes()).expect("load");
    // Oldest first, as it happened.
    assert!(matches!(replayed.events()[0], StockEvent::Received { qty: 500, .. }));
    assert_eq!(replayed.ledger().unwrap().level("salmon"),
               StockLevel { on_hand: 550, reserved: 0 });
}

/// Nothing is written when the gate refuses, so a replay can never
/// reconstruct an impossible state.
#[test]
fn a_refused_event_does_not_reach_the_log() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "uni".into(), qty: 2 }).unwrap();
    assert!(log.append(&StockEvent::Reserved {
        item: "uni".into(), qty: 3, order_id: "o1".into()
    }).is_err());
    assert_eq!(log.events().len(), 1, "the refusal wrote nothing");
}

/// §4's "one commit, not two". A basket whose third line is short must
/// reserve NOTHING -- otherwise the first two are held by an order that was
/// never placed, and nothing will ever release them.
#[test]
fn a_batch_is_all_or_nothing() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "rice".into(), qty: 100 }).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 100 }).unwrap();
    log.append(&StockEvent::Received { item: "uni".into(), qty: 1 }).unwrap();
    let before = log.events().len();

    let batch = vec![
        StockEvent::Reserved { item: "rice".into(), qty: 10, order_id: "o1".into() },
        StockEvent::Reserved { item: "nori".into(), qty: 2, order_id: "o1".into() },
        StockEvent::Reserved { item: "uni".into(), qty: 5, order_id: "o1".into() },
    ];
    assert!(log.append_all(&batch).is_err(), "the third line is short");
    assert_eq!(log.events().len(), before, "and so NOTHING was reserved");
    assert_eq!(log.ledger().unwrap().level("rice").reserved, 0);
}

/// Two lines of ONE order competing for the same ingredient are caught by
/// the batch, not by the second one failing after the first was written.
#[test]
fn two_lines_of_one_order_are_decided_together() {
    let mut log = StockLog::create().expect("create");
    log.append(&StockEvent::Received { item: "uni".into(), qty: 3 }).unwrap();
    let batch = vec![
        StockEvent::Reserved { item: "uni".into(), qty: 2, order_id: "o1".into() },
        StockEvent::Reserved { item: "uni".into(), qty: 2, order_id: "o1".into() },
    ];
    assert!(log.append_all(&batch).is_err(), "4 wanted, 3 on the shelf");
    assert_eq!(log.ledger().unwrap().level("uni").reserved, 0);
}
