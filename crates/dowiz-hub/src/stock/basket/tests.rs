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
