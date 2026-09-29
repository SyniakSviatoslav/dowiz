//! R2: the production act on the operator's example -- the raw items leave
//! by the card, the batch lands weighed with what it cost, the loss shows.

use super::*;
use crate::stock::{decode, encode};
use crate::prep::tests::kitchen;
use crate::stock::cost::Price;

fn look(k: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| k.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

pub(crate) fn act(prep: &str, planned: Qty, out: Qty) -> Act {
    Act { prep: prep.into(), planned, out, act: format!("pa_{prep}_{planned}"), by: "cook-1".into(), lot: None, expiry: None }
}

fn price(per_kg: i64) -> Price {
    Price { unit_cost: per_kg, per: 1000, supplier: None, doc: None }
}

/// A shelf with every raw item of the kitchen delivered and priced.
pub(crate) fn stocked_log() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for (item, per_kg) in [("rice-dry", 200), ("vinegar", 300), ("salt", 50), ("sugar", 150)] {
        log.receive_priced(item, 10_000, &price(per_kg)).unwrap();
    }
    log
}

#[test]
fn a_batch_takes_its_card_off_the_shelf_and_lands_weighed() {
    let k = kitchen();
    let mut log = stocked_log();
    log.cook(&act("mitsukan", 1000, 1000), &look(&k)).unwrap();
    let done = log.cook(&act("rice-seasoned", 2100, 2050), &look(&k)).unwrap();
    // Rice seasoned: dry rice 1000, water untracked, mitsukan 250 FROM THE SHELF.
    assert_eq!(done.inputs, vec![("mitsukan".into(), 250, 250 * MICRO), ("rice-dry".into(), 1000, 1000 * MICRO)]);
    assert_eq!(done.gross, 2350, "1000 + 1100 + 250 g went in");
    let led = log.ledger().unwrap();
    assert_eq!((led.level("rice-seasoned").on_hand, led.is_counted("rice-seasoned")), (2050, true), "weighed: counted");
    assert_eq!(led.level("mitsukan").on_hand, 750);
    assert_eq!(led.level("rice-dry").on_hand, 9000);
    assert_eq!(led.level("vinegar").on_hand, 10_000 - 800, "only the mitsukan batch took vinegar");
    assert_eq!(led.level("water").on_hand, 0, "water is never drawn");
    // Mitsukan cost 800 x 0.30 + 50 x 0.05 + 150 x 0.15 = 240 + 2.5 + 22.5 = 265;
    // the rice: 1000 x 0.20 + 250 g of mitsukan at 0.265 = 200 + 66.25 -> 266.
    assert_eq!(done.value, Some(266));
    let events = log.events();
    let made = events.iter().rev().find(|e| matches!(e, StockEvent::Made { .. })).unwrap();
    assert_eq!(made, &StockEvent::Made { item: "rice-seasoned".into(), qty: 2050, planned: 2100, gross: 2350, act: "pa_rice-seasoned_2100".into(), by: "cook-1".into() });
    // Its average is what went in over what came out: 266 / 2050 g -> per kg 130.
    assert_eq!(log.cost_book().wac("rice-seasoned", 1000), Some(130));
}

#[test]
fn an_act_that_is_not_one_writes_nothing() {
    let k = kitchen();
    let mut log = stocked_log();
    let before = log.len();
    assert!(log.cook(&act("salt", 10, 10), &look(&k)).is_err(), "a raw item has no card");
    assert!(log.cook(&act("ghost", 10, 10), &look(&k)).is_err());
    assert!(log.cook(&act("mitsukan", 0, 10), &look(&k)).is_err());
    assert!(log.cook(&act("mitsukan", 10, 0), &look(&k)).is_err());
    let mut anon = act("mitsukan", 10, 10);
    anon.by = " ".into();
    assert!(matches!(log.cook(&anon, &look(&k)), Err(StockError::Unsigned)));
    assert_eq!(log.len(), before);
    // The twin: a real one writes its three inputs and the batch.
    log.cook(&act("mitsukan", 10, 10), &look(&k)).unwrap();
    assert_eq!(log.len(), before + 4);
}

#[test]
fn a_fraction_of_a_gram_is_carried_not_lost() {
    let k = kitchen();
    let mut log = stocked_log();
    // 10 g of mitsukan takes 0.5 g of salt: a hundred batches take 50 g, not 100 or 0.
    for _ in 0..100 {
        log.cook(&act("mitsukan", 10, 10), &look(&k)).unwrap();
    }
    assert_eq!(log.ledger().unwrap().level("salt").on_hand, 10_000 - 50);
}

#[test]
fn an_old_log_reads_the_same_and_the_new_records_round_trip() {
    let ev = StockEvent::Cooked { item: "salt".into(), qty: 1, into: "mitsukan".into(), act: "pa".into(), by: "c".into() };
    assert_eq!(decode(&encode(&ev)), Some(ev));
    let ev = StockEvent::Made { item: "mitsukan".into(), qty: 990, planned: 1000, gross: 1000, act: "pa".into(), by: "c".into() };
    assert_eq!(decode(&encode(&ev)), Some(ev));
}
