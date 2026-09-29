//! No drift over a thousand sales, the zero line that must be a record, a
//! cancel that undoes its fraction, and every old record folding to carry 0.

use super::*;
use crate::stock::{settle, StockLedger};

const SALT: i64 = 773_810; // 0.773 810 g per Philadelphia (prep::tests)

fn log() -> StockLog {
    let mut log = StockLog::create_sized(256 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salt".into(), qty: 2000 }).unwrap();
    log
}
fn draw(i: usize) -> Vec<Draw> {
    vec![Draw { item: "salt".into(), uq: SALT, order_id: format!("o{i}") }]
}

#[test]
fn a_thousand_sales_book_the_exact_total_not_a_thousand_grams() {
    let mut log = log();
    for i in 0..1000 {
        log.append_draws(&draw(i)).unwrap();
        // Half of them cook, so the carry survives `consumed` too.
        if i % 2 == 0 {
            let led = log.ledger().unwrap();
            log.append_all(&settle(&led, &format!("o{i}"), true)).unwrap();
        }
    }
    let led = log.ledger().unwrap();
    let l = led.level("salt");
    // 1000 × 0.773 810 = 773.81 g -> 774 booked (500 consumed, 500 still held).
    assert_eq!(2000 - l.on_hand + l.reserved, 774, "{l:?}");
    let carry = log.fold_tail(false, false).unwrap().2;
    assert!(carry.of("salt").abs() <= MICRO / 2, "bounded: {}", carry.of("salt"));
    assert_eq!(carry.of("salt"), 1000 * SALT - 774 * MICRO);
}

#[test]
fn a_draw_that_rounds_to_zero_is_still_a_record_and_still_settles() {
    let mut log = log();
    log.append_draws(&draw(1)).unwrap(); // 0.77 -> 1
    log.append_draws(&draw(2)).unwrap(); // 1.55 -> 2: +1
    log.append_draws(&draw(3)).unwrap(); // 2.32 -> 2: +0
    let evs = log.events();
    let booked: Vec<Qty> = evs.iter().filter_map(|e| match e { StockEvent::Reserved { qty, .. } => Some(*qty), _ => None }).collect();
    assert_eq!(booked, vec![1, 1, 0]);
    assert!(log.raw()[3].contains(r#""uq":773810"#), "{}", log.raw()[3]);
    // The zero hold cooks and cancels like any other.
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o3", true)).unwrap();
    log.append_draws(&draw(4)).unwrap(); // 3.10 -> 3: +1
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salt").reserved, 3);
    // Without the zero record the carry would forget o3 and book 0 for ever.
    assert_eq!(led.stranded().iter().filter(|(o, _, _)| o == "o3").count(), 0);
}

#[test]
fn a_cancel_undoes_its_own_fraction_exactly() {
    let mut log = log();
    log.append_draws(&draw(1)).unwrap();
    let c = log.fold_tail(false, false).unwrap().2;
    assert_eq!(c.of("salt"), SALT - MICRO);
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", false)).unwrap();
    let c = log.fold_tail(false, false).unwrap().2;
    assert_eq!(c.of("salt"), 0, "released: nothing owed either way");
    assert_eq!(log.ledger().unwrap().level("salt").reserved, 0);
    // Twin: a cook keeps it.
    log.append_draws(&draw(2)).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o2", true)).unwrap();
    assert_eq!(log.fold_tail(false, false).unwrap().2.of("salt"), SALT - MICRO);
}

#[test]
fn whole_lines_and_every_old_record_leave_the_carry_at_zero() {
    let mut log = log();
    log.append(&StockEvent::Reserved { item: "salt".into(), qty: 50, order_id: "o1".into() }).unwrap();
    log.append_draws(&[Draw { item: "salt".into(), uq: 100 * MICRO, order_id: "o2".into() }]).unwrap();
    let c = log.fold_tail(false, false).unwrap().2;
    assert_eq!(c, Carry::default(), "no fractional draw: both maps empty, so old checkpoints keep their bytes");
    assert!(!log.raw()[2].contains("uq"), "a whole draw writes no uq: {}", log.raw()[2]);
    assert_eq!(log.ledger().unwrap().level("salt").reserved, 150);
    let d = draws_for("o3", &[(r#"{"bom":[{"supply":"salt","qty":2},{"supply":"x","uq":1500000}]}"#.into(), 3), (r#"{"bom":[{"supply":"x","qty":1}]}"#.into(), 1)]);
    assert_eq!(d, vec![Draw { item: "salt".into(), uq: 6 * MICRO, order_id: "o3".into() }, Draw { item: "x".into(), uq: 5_500_000, order_id: "o3".into() }]);
    assert!(log.append_draws(&[Draw { item: "salt".into(), uq: 0, order_id: "o4".into() }]).is_err(), "a zero draw is not a draw");
}

#[test]
fn the_carry_survives_a_checkpoint_and_a_short_shelf_still_refuses() {
    let mut log = log();
    log.set_checkpoint_every(4);
    for i in 0..12 {
        log.append_draws(&draw(i)).unwrap();
    }
    assert!(log.verify_checkpoints().unwrap() >= 2, "checkpoints were written and hold");
    let through = log.fold_tail(false, false).unwrap().2;
    let mut genesis = Carry::default();
    for rec in log.raw() {
        if let Some(ev) = crate::stock::decode(&rec) {
            genesis.apply(&ev, crate::minijson::int_field(&rec, "uq"));
        }
    }
    assert_eq!(through, genesis, "fold = checkpoint + tail");
    assert_eq!(log.ledger().unwrap(), StockLedger::fold(&log.events()).unwrap());
    // The shelf still decides: 2000 g on hand, a draw of 3 kg is refused whole.
    assert!(log.append_draws(&[Draw { item: "salt".into(), uq: 3000 * MICRO, order_id: "big".into() }]).is_err());
}

/// A till import's sales go through the same door as `served`, and a void
/// (`unserved`) gives the fraction back exactly.
#[test]
fn served_draws_carry_too_and_a_void_undoes_the_fraction() {
    let mut log = log();
    for i in 0..3 {
        log.append_served_draws(&draw(i)).unwrap();
    }
    let led = log.ledger().unwrap();
    assert_eq!(led.level("salt").on_hand, 2000 - 2, "3 x 0.77 = 2.32 -> 2 whole grams sold");
    assert_eq!(led.served_of("o2"), vec![("salt".to_string(), 0)], "the zero line (o0 1, o1 1, o2 0) is a served line too");
    let c = log.fold_tail(false, false).unwrap().2;
    assert_eq!(c.of("salt"), 3 * SALT - 2 * MICRO);
    // The till voids o1 (which booked 1 g): back exactly.
    for (item, qty) in led.served_of("o1") {
        log.append(&StockEvent::Unserved { item, qty, order_id: "o1".into() }).unwrap();
    }
    let c = log.fold_tail(false, false).unwrap().2;
    assert_eq!(c.of("salt"), 2 * SALT - MICRO, "o0 and o2 remain: 1.55 booked as 1");
    assert_eq!(log.ledger().unwrap().level("salt").on_hand, 2000 - 1);
}
