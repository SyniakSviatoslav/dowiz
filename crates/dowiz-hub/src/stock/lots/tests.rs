//! I6: first-expiry-first-out, an explicit pick, and lots that always add up
//! to the shelf.

use crate::stock::meta::Meta;
use crate::stock::{reservations_for, settle, PrepStage, Qty, StockEvent, StockLog, WasteReason};

const ROLL: &str = r#"{"id":"r","bom":[{"supply":"salmon","qty":100}]}"#;

fn lot(code: &str, expiry: i64) -> Meta {
    Meta { lot: Some(code.into()), expiry: Some(expiry), supplier: Some("Sea".into()), ..Meta::default() }
}
fn left(log: &StockLog, item: &str) -> Vec<(String, Qty)> {
    log.journal().unwrap().lots.of(item).iter().map(|l| (l.code.clone(), l.left)).collect()
}
fn waste(qty: Qty, lot: Option<&str>) -> (StockEvent, Meta) {
    (
        StockEvent::Wasted { item: "salmon".into(), qty, reason: WasteReason::Spoiled, by: "p".into() },
        Meta { lot: lot.map(str::to_string), ..Meta::default() },
    )
}

/// The box that expires first is used first, whatever order they came in.
#[test]
fn the_first_to_expire_is_the_first_used() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 1000, &lot("L-late", 20261010)).unwrap();
    log.receive_with("salmon", 500, &lot("L-soon", 20261001)).unwrap();
    log.append_all(&reservations_for("o1", &[(ROLL.into(), 6)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap();
    assert_eq!(left(&log, "salmon"), vec![("L-late".to_string(), 900)], "600 g: all of L-soon, then 100 of L-late");
}

/// An explicit pick beats FEFO; a pick naming no open lot falls back to it.
#[test]
fn a_named_lot_is_drawn_first() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 1000, &lot("A", 20261001)).unwrap();
    log.receive_with("salmon", 1000, &lot("B", 20261005)).unwrap();
    let (ev, m) = waste(300, Some("B"));
    log.append_with(&ev, &m).unwrap();
    assert_eq!(left(&log, "salmon"), vec![("A".into(), 1000), ("B".into(), 700)]);
    let (ev, m) = waste(100, Some("nope"));
    log.append_with(&ev, &m).unwrap();
    assert_eq!(left(&log, "salmon"), vec![("A".into(), 900), ("B".into(), 700)]);
}

/// A count shrinks the first-expiring lots and a surplus lands unlabelled; a
/// delivery with no code gets one minted from its position.
#[test]
fn a_count_lands_on_what_should_have_been_used() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 1000, &lot("A", 20261001)).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 400 }).unwrap();
    let count = |o| StockEvent::Stocktake { item: "salmon".into(), observed: o, stocktake_id: "s".into(), by: "p".into() };
    log.append(&count(1100)).unwrap();
    assert_eq!(left(&log, "salmon"), vec![("A".into(), 700), ("#1".into(), 400)], "300 short, off the dated lot first");
    log.append(&count(1200)).unwrap();
    assert_eq!(left(&log, "salmon"), vec![("A".into(), 700), ("#1".into(), 400), ("-".into(), 100)]);
}

/// THE LOTS ADD UP TO THE SHELF after every record of a mixed history,
/// including a served dish that drives the level below zero.
#[test]
fn the_lots_always_add_up_to_the_shelf() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 300, &lot("A", 20261001)).unwrap();
    log.append(&StockEvent::Served { item: "salmon".into(), qty: 500, order_id: "t1".into() }).unwrap();
    log.receive_with("salmon", 1000, &lot("B", 20261009)).unwrap();
    log.append(&StockEvent::Unserved { item: "salmon".into(), qty: 100, order_id: "t1".into() }).unwrap();
    let (ev, m) = waste(50, None);
    log.append_with(&ev, &m).unwrap();
    let j = log.journal().unwrap();
    let on_hand = j.ledger.level("salmon").on_hand;
    let sum: Qty = j.lots.of("salmon").iter().map(|l| l.left).sum();
    assert_eq!((on_hand, sum), (850, 850));
    assert_eq!(left(&log, "salmon"), vec![("B".into(), 750), ("-".into(), 100)], "the dated lot is binned first");
}

/// Prep: the input's lot is drawn and the output opens a lot named after it.
#[test]
fn a_prep_carries_its_lot_forward() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("salmon", 5000, &lot("L7", 20261001)).unwrap();
    let prep = StockEvent::Produced {
        item: "salmon".into(), qty: 5000, out: 2750, stage: PrepStage::Clean, into: Some("fillet".into()), by: "p".into(),
    };
    log.append_with(&prep, &Meta { lot: Some("L7".into()), ..Meta::default() }).unwrap();
    let j = log.journal().unwrap();
    assert!(j.lots.of("salmon").is_empty());
    let f = j.lots.of("fillet");
    assert_eq!((f[0].code.as_str(), f[0].left, f[0].expiry), ("L7>fillet", 2750, Some(20261001)));
    assert_eq!(j.lots.open().len(), 1);
}
