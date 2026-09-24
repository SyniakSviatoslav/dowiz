//! §2.10's CHECK, through the real log: two receipts at 1,000 and 1,200 per kg
//! give a WAC of 1,100; an order placed between them stamps the 1,000-cost;
//! a rebuild from the persisted bytes reproduces the stamp. Each refusal
//! beside its twin.

use super::*;
use crate::stock::{reservations_for, settle};

const KG: Qty = 1000;

fn price(c: i64) -> Price {
    Price { unit_cost: c, per: KG, supplier: Some("Fish & Co".into()), doc: Some("DN-17".into()) }
}
fn maki() -> Vec<BomLine> {
    vec![BomLine { supply: "rice".into(), qty: 100 }]
}
const MAKI: &str = r#"{"id":"maki","bom":[{"supply":"rice","qty":100}]}"#;

#[test]
fn two_receipts_average_and_the_order_between_keeps_its_stamp() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000));

    // An order placed between the receipts: 100 g at 1,000 per kg.
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 1)])).unwrap();
    let s = stamp(&log, &maki()).expect("rice has a cost");
    assert_eq!(s.cost, 100);

    log.receive_priced("rice", KG, &price(1200)).unwrap();
    assert_eq!(log.cost_book().wac("rice", KG), Some(1100), "the §2.10 CHECK");
    assert_eq!(stamp(&log, &maki()).unwrap().cost, 110, "a NEW order stamps the new average");

    // LAW 8: from the persisted bytes alone, the old stamp is reproduced.
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(rebuild(&back, &s, &maki()), Some(100));
    // Twin: the rebuild is not a constant -- at today's length it says today's.
    let now = Stamp { cost: 0, at: back.len() };
    assert_eq!(rebuild(&back, &now, &maki()), Some(110));
}

#[test]
fn the_shelf_reads_a_priced_receipt_as_the_plain_received_it_always_was() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", 500, &price(1000)).unwrap();
    assert_eq!(log.events(), vec![StockEvent::Received { item: "rice".into(), qty: 500 }]);
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 500);
    assert!(log.raw()[0].contains(r#""supplier":"Fish & Co""#) && log.raw()[0].contains(r#""doc":"DN-17""#));
}

#[test]
fn a_price_that_is_not_one_is_refused_and_nothing_is_written() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for bad in [Price { per: 0, ..price(1000) }, Price { per: -1, ..price(1000) }, price(-5)] {
        assert!(matches!(log.receive_priced("rice", KG, &bad), Err(StockError::NotPositive { .. })), "{bad:?}");
    }
    assert!(log.receive_priced("rice", 0, &price(1000)).is_err(), "the shelf still decides the quantity");
    assert_eq!(log.len(), 0);
    // Twin: a free delivery (cost 0) is a price.
    log.receive_priced("rice", KG, &price(0)).unwrap();
    assert_eq!((log.len(), log.cost_book().wac("rice", KG)), (1, Some(0)));
}

#[test]
fn a_dish_costs_only_when_every_supply_does() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 50 }).unwrap();
    let both = vec![BomLine { supply: "rice".into(), qty: 100 }, BomLine { supply: "nori".into(), qty: 2 }];
    assert_eq!(log.cost_book().dish_cost(&both), None, "nori was never priced");
    assert_eq!(log.cost_book().dish_cost(&[]), None, "no recipe, no cost");
    // Twin: once nori is priced, its unpriced 50 join at that price.
    log.receive_priced("nori", 50, &Price { unit_cost: 30, per: 1, supplier: None, doc: None }).unwrap();
    assert_eq!(log.cost_book().wac("nori", 1), Some(30));
    assert_eq!(log.cost_book().dish_cost(&both), Some(100 + 60));
}

#[test]
fn draws_leave_at_the_average_so_the_next_receipt_weighs_what_is_left() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 5)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap(); // 500 g consumed
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000), "a draw does not move the average");
    log.receive_priced("rice", KG, &price(1200)).unwrap();
    // (500 x 1000 + 1000 x 1200) / 1500 = 1133.3
    assert_eq!(log.cost_book().wac("rice", KG), Some(1133));
    // Twin: a reservation alone draws nothing -- the plain two-receipt 1,100.
    let mut r = StockLog::create_sized(64 * 1024).unwrap();
    r.receive_priced("rice", KG, &price(1000)).unwrap();
    r.append_all(&reservations_for("o1", &[(MAKI.into(), 5)])).unwrap();
    r.receive_priced("rice", KG, &price(1200)).unwrap();
    assert_eq!(r.cost_book().wac("rice", KG), Some(1100));
}

#[test]
fn a_shelf_drawn_to_zero_keeps_its_last_cost() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", 100, &price(1000)).unwrap();
    log.append(&StockEvent::Wasted { item: "rice".into(), qty: 100, reason: crate::stock::WasteReason::Spoiled, by: "p1".into() }).unwrap();
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 0);
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000));
    assert_eq!(StockLog::create_sized(64 * 1024).unwrap().cost_book().wac("rice", KG), None, "twin: never priced");
}
