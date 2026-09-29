use super::*;

fn recv(item: &str, q: Qty) -> StockEvent {
    StockEvent::Received { item: item.into(), qty: q }
}
fn res(item: &str, q: Qty, o: &str) -> StockEvent {
    StockEvent::Reserved { item: item.into(), qty: q, order_id: o.into() }
}
fn con(item: &str, q: Qty, o: &str) -> StockEvent {
    StockEvent::Consumed { item: item.into(), qty: q, order_id: o.into() }
}
fn rel(item: &str, q: Qty, o: &str) -> StockEvent {
    StockEvent::Released { item: item.into(), qty: q, order_id: o.into() }
}

#[test]
fn the_happy_path_conserves() {
    let led = StockLedger::fold(&[
        recv("salmon", 1000),
        res("salmon", 200, "ord_1"),
        con("salmon", 200, "ord_1"),
    ])
    .expect("fold");
    assert_eq!(led.level("salmon"), StockLevel { on_hand: 800, reserved: 0 });
    assert_eq!(led.available("salmon"), 800);
    assert!(led.stranded().is_empty());
}

/// I1, and the sentence §4 builds the whole design around: the refusal IS
/// the automatic stop-listing. Nothing sets a flag; there is nothing to race.
#[test]
fn reserving_more_than_is_available_is_refused_and_that_is_the_86() {
    let led = StockLedger::fold(&[recv("salmon", 100), res("salmon", 60, "ord_1")]).unwrap();
    assert_eq!(led.available("salmon"), 40);
    match led.decide(&res("salmon", 41, "ord_2")) {
        Err(StockError::OutOfStock { wanted, available, .. }) => {
            assert_eq!((wanted, available), (41, 40));
        }
        other => panic!("expected OutOfStock, got {other:?}"),
    }
    // Exactly what is left is fine; one more is not.
    assert!(led.decide(&res("salmon", 40, "ord_2")).is_ok());
}

/// A never-counted item is not an empty one -- and every order still
/// takes its ingredients (operator, 2026-09-26). The reservation and the
/// consumption are recorded, `on_hand` goes NEGATIVE (what was used since
/// nobody looked: "needs a count"), and nothing is refused -- until the
/// first count or delivery, from which point the 86 is exactly as strict
/// as above.
#[test]
fn an_uncounted_item_never_refuses_but_every_order_takes_it_and_the_first_count_arms_it() {
    let led = StockLedger::fold(&[res("nori", 5, "ord_1"), con("nori", 5, "ord_1")]).unwrap();
    assert!(!led.is_counted("nori"));
    assert_eq!(led.level("nori"), StockLevel { on_hand: -5, reserved: 0 }, "the order took 5: needs a count");
    assert!(led.decide(&res("nori", 1_000, "ord_2")).is_ok(), "an uncounted level is not a measurement");
    assert!(led.decide(&StockEvent::Wasted {
        item: "nori".into(), qty: 3, reason: WasteReason::Spoiled, by: "p".into()
    }).is_ok(), "nor does it refuse a write-off");
    assert!(led.stranded().is_empty(), "the reservation was settled, not left behind");

    let count = StockEvent::Stocktake {
        item: "nori".into(), observed: 20, stocktake_id: "st_1".into(), by: "mgr1".into(),
    };
    let led = StockLedger::fold(&[res("nori", 5, "ord_1"), con("nori", 5, "ord_1"), count]).unwrap();
    assert!(led.is_counted("nori"));
    assert_eq!(led.level("nori").on_hand, 20, "the count is the basis, not 20 minus earlier uncounted use");
    assert!(led.decide(&res("nori", 21, "ord_2")).is_err(), "counted: the 86 is armed");
    assert!(led.decide(&res("nori", 20, "ord_2")).is_ok());

    // A delivery arms it too, AT WHAT CAME IN: the uncounted use before it
    // is not a debt the delivery pays.
    let led = StockLedger::fold(&[res("rice", 7, "o0"), con("rice", 7, "o0"), recv("rice", 10)]).unwrap();
    assert_eq!(led.level("rice").on_hand, 10);
    assert!(led.is_counted("rice") && led.decide(&res("rice", 11, "ord_3")).is_err());
    // Twin: once counted, a draw below zero is the ledger's own number again.
    let led = StockLedger::fold(&[recv("rice", 10), recv("rice", 5)]).unwrap();
    assert_eq!(led.level("rice").on_hand, 15, "only the FIRST delivery re-bases");
}

/// Once an order holds its portion, moving it on is never refused for
/// stock: here a till sale (`Served`) took the shelf below what the order
/// holds, and the order still cooks. The shelf goes negative -- a count is
/// owed -- rather than a courier being stopped at IN_DELIVERY.
#[test]
fn a_held_order_always_cooks_even_when_the_shelf_moved_under_it() {
    let served = StockEvent::Served { item: "tuna".into(), qty: 5, order_id: "till_1".into() };
    let led = StockLedger::fold(&[recv("tuna", 10), res("tuna", 10, "ord_1"), served]).unwrap();
    assert_eq!(led.level("tuna").on_hand, 5);
    assert!(led.decide(&con("tuna", 10, "ord_1")).is_ok());
    let led = StockLedger::fold(&[
        recv("tuna", 10), res("tuna", 10, "ord_1"),
        StockEvent::Served { item: "tuna".into(), qty: 5, order_id: "till_1".into() },
        con("tuna", 10, "ord_1"),
    ]).unwrap();
    assert_eq!(led.level("tuna"), StockLevel { on_hand: -5, reserved: 0 });
    // A NEW order is still refused: the 86 is armed for what was counted.
    assert!(led.decide(&res("tuna", 1, "ord_2")).is_err());
}

/// Two orders cannot be promised the same portion. This is the oversell the
/// design exists to make structurally impossible.
#[test]
fn the_same_portion_cannot_be_promised_twice() {
    let led = StockLedger::fold(&[recv("uni", 2), res("uni", 2, "ord_1")]).unwrap();
    assert_eq!(led.available("uni"), 0, "all of it is spoken for");
    assert_eq!(led.level("uni").on_hand, 2, "and it is still on the shelf");
    assert!(led.decide(&res("uni", 1, "ord_2")).is_err());
}

/// I1 non-negativity, across every event that subtracts.
#[test]
fn nothing_can_drive_a_level_negative() {
    let led = StockLedger::fold(&[recv("rice", 10)]).unwrap();
    assert!(led.decide(&StockEvent::Wasted {
        item: "rice".into(), qty: 11, reason: WasteReason::Spoiled, by: "mgr1".into()
    }).is_err());
    // And waste cannot eat a reservation: that portion is owed to somebody.
    let led = StockLedger::fold(&[recv("rice", 10), res("rice", 8, "ord_1")]).unwrap();
    assert!(led.decide(&StockEvent::Wasted {
        item: "rice".into(), qty: 3, reason: WasteReason::Dropped, by: "mgr1".into()
    }).is_err(), "only 2 are unspoken for");
    assert!(led.decide(&StockEvent::Wasted {
        item: "rice".into(), qty: 2, reason: WasteReason::Dropped, by: "mgr1".into()
    }).is_ok());
}

/// §4: "Always > 0".
#[test]
fn a_quantity_that_is_not_a_quantity_is_refused() {
    let led = StockLedger::default();
    for q in [0, -1, i64::MIN] {
        assert!(matches!(led.decide(&recv("x", q)), Err(StockError::NotPositive { .. })));
    }
    // An observed count of zero is legitimate: a shelf can be empty.
    assert!(led.decide(&StockEvent::Stocktake {
        item: "x".into(), observed: 0, stocktake_id: "s1".into(), by: "mgr1".into()
    }).is_ok());
    assert!(led.decide(&StockEvent::Stocktake {
        item: "x".into(), observed: -1, stocktake_id: "s1".into(), by: "mgr1".into()
    }).is_err());
}

/// I3: exactly one of Consumed or Released, never both, never neither.
#[test]
fn a_reservation_is_matched_exactly_once() {
    // Released, then consumed: refused.
    let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1"), rel("a", 4, "o1")]).unwrap();
    assert!(matches!(led.decide(&con("a", 4, "o1")), Err(StockError::Linkage(_))));
    assert_eq!(led.level("a"), StockLevel { on_hand: 10, reserved: 0 });

    // Consumed, then released: also refused.
    let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1"), con("a", 4, "o1")]).unwrap();
    assert!(matches!(led.decide(&rel("a", 4, "o1")), Err(StockError::Linkage(_))));

    // Consuming more than was reserved is refused.
    let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1")]).unwrap();
    assert!(matches!(led.decide(&con("a", 5, "o1")), Err(StockError::Linkage(_))));

    // And an order nobody reserved for cannot consume at all.
    assert!(matches!(led.decide(&con("a", 1, "ghost")), Err(StockError::Linkage(_))));
}

/// A reservation that is never matched is stock the venue thinks it owes to
/// an order that ended -- the slow leak that makes a kitchen believe it is
/// out of something it has.
#[test]
fn stranded_reservations_are_visible() {
    let led = StockLedger::fold(&[
        recv("a", 10), res("a", 3, "o1"), res("a", 2, "o2"), rel("a", 2, "o2"),
    ])
    .unwrap();
    assert_eq!(led.stranded(), vec![("o1".to_string(), "a".to_string(), 3)]);
    assert_eq!(led.available("a"), 7);
}

/// I2: the basis resets at a count, and reservations survive it.
#[test]
fn a_stocktake_resets_the_basis_and_keeps_promises() {
    let led = StockLedger::fold(&[
        recv("tuna", 50),
        res("tuna", 10, "o1"),
        // Somebody counted and there are only 30.
        StockEvent::Stocktake { item: "tuna".into(), observed: 30, stocktake_id: "s1".into(), by: "mgr1".into() },
    ])
    .unwrap();
    assert_eq!(led.level("tuna"), StockLevel { on_hand: 30, reserved: 10 });
    assert_eq!(led.available("tuna"), 20);
    // The open order can still be fulfilled.
    assert!(led.decide(&con("tuna", 10, "o1")).is_ok());
}

/// A count below what is already promised cannot be right, and accepting it
/// would make the ledger claim it owes more than it has.
#[test]
fn a_count_below_what_is_reserved_is_refused() {
    let led = StockLedger::fold(&[recv("tuna", 50), res("tuna", 20, "o1")]).unwrap();
    assert!(led.decide(&StockEvent::Stocktake {
        item: "tuna".into(), observed: 5, stocktake_id: "s1".into(), by: "mgr1".into()
    }).is_err());
    assert!(led.decide(&StockEvent::Stocktake {
        item: "tuna".into(), observed: 20, stocktake_id: "s1".into(), by: "mgr1".into()
    }).is_ok());
}

/// I4: the same sequence folds to the same projection, and the item order
/// is stable, so two processes agree byte for byte.
#[test]
fn the_fold_is_deterministic() {
    let evs = vec![
        recv("z", 5), recv("a", 3), res("z", 2, "o1"), recv("m", 7), con("z", 2, "o1"),
    ];
    let a = StockLedger::fold(&evs).unwrap();
    let b = StockLedger::fold(&evs).unwrap();
    assert_eq!(a.items(), b.items());
    assert_eq!(
        a.items().iter().map(|(i, _)| i.as_str()).collect::<Vec<_>>(),
        vec!["a", "m", "z"],
        "items come out sorted, so two folds serialise identically"
    );
}

/// The events survive the round trip that a restart is.
#[test]
fn every_event_round_trips() {
    let evs = vec![
        recv("salmon", 100),
        res("salmon", 5, "ord_1"),
        con("salmon", 5, "ord_1"),
        rel("rice", 2, "ord_2"),
        StockEvent::Wasted { item: "rice".into(), qty: 1, reason: WasteReason::Spoiled, by: "mgr1".into() },
        StockEvent::Stocktake { item: "nori".into(), observed: 42, stocktake_id: "s1".into(), by: "mgr1".into() },
    ];
    for ev in &evs {
        assert_eq!(decode(&encode(ev)).as_ref(), Some(ev), "{ev:?}");
    }
    assert_eq!(decode("not json"), None);
    assert_eq!(decode(r#"{"k":"nonsense","item":"x"}"#), None);
}

/// An item name with a quote cannot forge a second field.
#[test]
fn a_hostile_item_name_cannot_forge_a_record() {
    let ev = recv(r#"x","qty":9999"#, 1);
    let back = decode(&encode(&ev)).expect("decode");
    match back {
        StockEvent::Received { qty, .. } => assert_eq!(qty, 1, "the quantity must not move"),
        other => panic!("{other:?}"),
    }
}

/// Checked arithmetic, not wrapping. A receipt that would overflow is
/// refused rather than turning a full shelf into a negative one.
#[test]
fn overflow_is_refused_not_wrapped() {
    let led = StockLedger::fold(&[recv("x", i64::MAX)]).unwrap();
    assert!(matches!(led.decide(&recv("x", 1)), Err(StockError::Overflow)));
}
