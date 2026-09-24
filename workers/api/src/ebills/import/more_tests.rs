//! The import's later rules: a void gives the shelf back, and the first
//! run's leads (split from `tests.rs` for the file-size ratchet).

use super::super::map::to_order;
use super::tests::{bill, sale, Line, Venue, LOC, T0};
use super::*;
use serde_json::json;

/// A VOID PUTS THE SHELF BACK: the voided course's `Served` draw is reversed
/// by `Unserved`, exactly once -- the same void imported again moves nothing.
#[test]
fn a_void_puts_back_what_its_sale_served_once() {
    let mut v = Venue::new();
    v.map.push(("56".into(), "p-roll".into()));
    v.stock.append(&StockEvent::Received { item: "salmon".into(), qty: 200 }).unwrap();
    let course = sale(8717, Some("10"), Some(5685), false, &[Line("56", "Sake roll", 2, 250)], 0, 1);
    let mut void = sale(8718, Some("10"), Some(5686), false, &[Line("56", "Sake roll", 2, 250)], 60, 1);
    void.changed_status = Some("CANCELLED".into());
    void.modified = Some(super::super::wire::Modified { id: 8717, uuid: course.uuid.clone() });
    if let Some(r) = void.sale_records.as_mut() {
        (r[0].amount, r[0].total_value) = (-2.0, -500.0);
    }
    void.total_value = -500.0;
    v.import(&[Mapped::Order { sale_id: 8717, envelope: to_order(&course, LOC).unwrap() }], T0);
    assert_eq!(v.stock.ledger().unwrap().level("salmon").on_hand, 120, "2 x 40 g served");
    let x = Mapped::Order { sale_id: 8718, envelope: to_order(&void, LOC).unwrap() };
    v.import(std::slice::from_ref(&x), T0 + 60_000);
    let led = v.stock.ledger().unwrap();
    assert_eq!(led.level("salmon").on_hand, 200, "the void put it back");
    assert!(led.served_of(&format!("ebills:{}", course.uuid)).is_empty());
    let len = v.stock.len();
    v.import(std::slice::from_ref(&x), T0 + 120_000);
    assert_eq!(v.stock.len(), len, "never twice");
}

/// THE FIRST RUN'S LEADS: a course from before the window is placed only
/// when a window bill claims it, and paid by it; an unclaimed lead is never
/// placed and is let go after two days.
#[test]
fn a_lead_is_placed_only_when_a_bill_claims_it() {
    let mut v = Venue::new();
    let lead = |id: i64, table: &str, secs: i64| Mapped::Lead { sale_id: id, envelope: to_order(&sale(id, Some(table), Some(id), false, &[Line("1", "x", 1, 600)], secs, 1), LOC).unwrap() };
    let out = v.import(&[lead(8660, "10", 0), lead(8661, "4", 10)], T0);
    assert_eq!((out.placed, out.leads.len()), (0, 2), "leads wait, unplaced");
    assert_eq!(v.orders(), 0);
    let out = v.import(&[bill(8664, "10", &[Line("1", "x", 1, 600)], 100)], T0 + 200_000);
    assert_eq!((out.placed, out.paid, out.pending.len(), out.leads.len()), (1, 1, 0, 1));
    assert_eq!(v.order("ebills:00000000-0000-4000-8000-000000008660")["bill"]["total"], json!(600));
    let out = v.import(&[], T0 + PENDING_FOR_MS + 1);
    assert_eq!((out.placed, out.leads.len(), out.refused.len()), (0, 0, 0), "an unclaimed lead goes quietly, never placed");
}
