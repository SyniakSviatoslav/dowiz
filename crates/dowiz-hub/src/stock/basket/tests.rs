//! R2 at the sale: a batch cooked ahead is reserved first; without one the
//! order books exactly what it booked before this row.

use super::*;
use crate::prep::for_ledger;
use crate::prep::tests::{kitchen, PHILADELPHIA};
use crate::stock::act::tests::{act, stocked_log};
use crate::stock::{draws_for, StockEvent, StockLog};

fn look(k: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| k.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

fn reserved(log: &StockLog, order: &str) -> Vec<(String, i64)> {
    log.events()
        .into_iter()
        .filter_map(|e| match e {
            StockEvent::Reserved { item, qty, order_id } if order_id == order => Some((item, qty)),
            _ => None,
        })
        .collect()
}

fn sell(log: &mut StockLog, order: &str, n: i64) {
    let k = kitchen();
    let dish = for_ledger(&look(&k), PHILADELPHIA).0;
    log.append_draws(&draws_for(order, &[(dish, n)])).unwrap();
}

#[test]
fn nothing_cooked_ahead_books_what_the_raw_expansion_booked() {
    let k = kitchen();
    let with_tree = for_ledger(&look(&k), PHILADELPHIA).0;
    assert!(with_tree.contains(r#""tree""#));
    let mut v: serde_json::Value = serde_json::from_str(&with_tree).unwrap();
    v.as_object_mut().unwrap().remove("tree");
    let (mut a, mut b) = (stocked_log(), stocked_log());
    a.append_draws(&draws_for("o1", &[(with_tree, 3)])).unwrap();
    b.append_draws(&draws_for("o1", &[(v.to_string(), 3)])).unwrap();
    assert_eq!(a.events(), b.events(), "a tree with no batch ready changes no record");
    assert_eq!(reserved(&a, "o1"), vec![("rice-dry".into(), 186), ("salt".into(), 2), ("sugar".into(), 7), ("vinegar".into(), 37)]);
}

#[test]
fn a_ready_batch_is_reserved_first() {
    let k = kitchen();
    let mut log = stocked_log();
    log.cook(&act("rice-seasoned", 2100, 2100), &look(&k)).unwrap();
    sell(&mut log, "o1", 2);
    assert_eq!(reserved(&log, "o1"), vec![("rice-seasoned".into(), 260)], "two rolls, 130 g each, from the pot");
}

#[test]
fn a_short_batch_is_used_up_and_the_rest_comes_from_raw() {
    let k = kitchen();
    let mut log = stocked_log();
    log.cook(&act("rice-seasoned", 100, 100), &look(&k)).unwrap();
    sell(&mut log, "o1", 1);
    let r = reserved(&log, "o1");
    assert_eq!(r.iter().find(|(i, _)| i == "rice-seasoned"), Some(&("rice-seasoned".to_string(), 100)));
    // 30 g from the card: 14.29 g of dry rice.
    assert_eq!(r.iter().find(|(i, _)| i == "rice-dry"), Some(&("rice-dry".to_string(), 14)));
    // The next order finds the pot empty: all raw again, never refused.
    sell(&mut log, "o2", 1);
    assert!(reserved(&log, "o2").iter().all(|(i, _)| i != "rice-seasoned"));
    assert_eq!(log.ledger().unwrap().available("rice-seasoned"), 0);
}

/// W-PF3 T1: the share the shelf gave, and a dish's cost at that share.
#[test]
fn the_shelf_share_prices_the_batch_at_its_own_average() {
    let k = kitchen();
    let dish = for_ledger(&look(&k), PHILADELPHIA).0;
    let one = of(&[(dish.clone(), 1)]).unwrap();
    // Nothing ready: no share, and the caller prices the raw bom as before.
    let mut log = stocked_log();
    let (_, _, shelf) = log.append_draws_split(&draws_for("o0", &[(dish.clone(), 1)])).unwrap();
    assert!(shelf.is_empty());
    assert_eq!(one.portion_cost(&shelf, &log.cost_book()), None);
    // A whole batch ready: the dish's 130 g all come from it, at ITS average.
    log.cook(&act("rice-seasoned", 2100, 2100), &look(&k)).unwrap();
    let book = log.cost_book();
    let (_, _, shelf) = log.append_draws_split(&draws_for("o1", &[(dish.clone(), 1)])).unwrap();
    assert_eq!(shelf.get("rice-seasoned").map(|r| r.num == r.den), Some(true), "the batch gave all of it");
    let batch = i128::from(book.avg_micro("rice-seasoned").unwrap());
    let m = i128::from(MICRO);
    assert_eq!(one.portion_cost(&shelf, &book), Some(Some(((batch * 130 + m / 2) / m) as i64)));
    // 100 g ready for 130: the shelf gives 100 g plus the half-unit a whole
    // booking of 100 absorbs (`ready_micro`), and the rest from raw.
    let mut log = stocked_log();
    log.cook(&act("rice-seasoned", 100, 100), &look(&k)).unwrap();
    let (_, _, shelf) = log.append_draws_split(&draws_for("o2", &[(dish, 1)])).unwrap();
    assert_eq!(shelf.get("rice-seasoned"), Some(&Rat::new(100_499_999, 130_000_000)));
}

#[test]
fn a_batch_nobody_priced_is_not_a_cost() {
    let k = kitchen();
    let dish = for_ledger(&look(&k), PHILADELPHIA).0;
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for item in ["rice-dry", "vinegar", "salt", "sugar"] {
        log.append(&StockEvent::Received { item: item.into(), qty: 10_000 }).unwrap();
    }
    log.cook(&act("rice-seasoned", 2100, 2100), &look(&k)).unwrap();
    let book = log.cost_book();
    let (_, _, shelf) = log.append_draws_split(&draws_for("o1", &[(dish.clone(), 1)])).unwrap();
    assert_eq!(of(&[(dish, 1)]).unwrap().portion_cost(&shelf, &book), Some(None), "a partial sum is never a cost");
}
