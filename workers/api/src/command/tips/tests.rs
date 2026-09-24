//! The tip record (§7 item 8). THE CHECK: Σ over people == Σ `tip` over the
//! period's rounds, on a synthetic log whose payments are written by the REAL
//! `pay::decide` — so the test cannot agree with a fold that only agrees with
//! itself.

use super::*;
use crate::command::pay::{decide, PayIn, Room};
use crate::hubdo::OrderView;
use serde_json::json;

const T0: i64 = 1_790_000_000_000;
const ROOM: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn round(id: &str, total: i64, currency: Option<&str>) -> Value {
    let mut r = json!({
        "id": id, "status": "PENDING", "location_id": "v1",
        "items": [{"product_id": "maki", "quantity": 1, "unit_price": total, "name": "Maki"}],
        "subtotal": total, "discount": 0, "delivery_fee": 0, "tip": 0, "total": total,
        "fulfilment": {"kind": "dine_in", "table": "7"}
    });
    if let Some(c) = currency {
        r["currency"] = json!(c);
    }
    r
}

/// One payment through the real command; returns the order as it now is.
fn pay(order: &Value, amount: i64, tip: i64, by: &str, at: i64) -> Value {
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let id = order["id"].as_str().unwrap().to_string();
    let view = OrderView { order_id: id.clone(), kind: 3, seq: (at - 1) as u64, order_json: order.to_string() };
    let input = PayIn {
        order_id: id,
        location_id: "v1".into(),
        amount,
        method: "card".into(),
        by: by.into(),
        till_id: None,
        covers: None,
        currency: None,
        rate_ppm: None,
        tip: (tip > 0).then_some(tip),
        wallet: None,
        now_ms: at,
    };
    decide(&mut hub, Some(&view), &input, &ROOM).expect("the payment lands").0
}

fn sum(t: &[PersonTip]) -> i64 {
    t.iter().map(|p| p.amount).sum()
}

/// ≥5 orders, 3 people, 2 split payments: every tip goes to its taker and the
/// people's total is the rounds' total, to the minor unit.
#[test]
fn tips_by_person_sum_to_the_rounds_tips() {
    let r1 = pay(&round("r1", 1000, None), 1000, 100, "ana", T0 + 1);
    // Split: two payers on one round, each with a tip.
    let r2 = pay(&round("r2", 2000, None), 1200, 50, "ben", T0 + 2);
    let r2 = pay(&r2, 800, 30, "cleo", T0 + 3);
    let r3 = pay(&round("r3", 500, None), 500, 0, "ana", T0 + 4); // no tip
    let r4 = pay(&round("r4", 1500, None), 1500, 200, "ben", T0 + 5);
    // Split, the second payer tips and the first does not.
    let r5 = pay(&round("r5", 3000, None), 1000, 0, "cleo", T0 + 6);
    let r5 = pay(&r5, 2000, 70, "ana", T0 + 7);
    let orders = vec![r1, r2, r3, r4, r5];

    let t = tips_by_person(&orders, "ALL", T0, T0 + 10).unwrap();
    let rounds: i64 = orders.iter().map(|o| o["tip"].as_i64().unwrap()).sum();
    assert_eq!(rounds, 450, "the synthetic log's own tip total");
    assert_eq!(sum(&t), rounds, "Σ over people == Σ tip over the rounds");
    let by = |who: &str| t.iter().find(|p| p.by == who).map(|p| p.amount);
    assert_eq!((by("ana"), by("ben"), by("cleo")), (Some(170), Some(250), Some(30)));
    assert!(t.iter().all(|p| p.currency == "ALL"));
}

/// The period is the payment's instant, both ends included; its twin above
/// covers the whole period.
#[test]
fn a_tip_outside_the_period_is_not_counted() {
    let early = pay(&round("r1", 1000, None), 1000, 100, "ana", T0 - 1);
    let edge = pay(&round("r2", 1000, None), 1000, 40, "ana", T0);
    let late = pay(&round("r3", 1000, None), 1000, 60, "ben", T0 + 11);
    let t = tips_by_person(&[early, edge, late], "ALL", T0, T0 + 10).unwrap();
    assert_eq!(t, vec![PersonTip { by: "ana".into(), currency: "ALL".into(), amount: 40 }]);
}

#[test]
fn two_currencies_are_never_one_number() {
    let lek = pay(&round("r1", 1000, None), 1000, 100, "ana", T0 + 1);
    let eur = pay(&round("r2", 1000, Some("EUR")), 1000, 150, "ana", T0 + 2);
    let t = tips_by_person(&[lek, eur], "ALL", T0, T0 + 10).unwrap();
    assert_eq!(
        t,
        vec![
            PersonTip { by: "ana".into(), currency: "ALL".into(), amount: 100 },
            PersonTip { by: "ana".into(), currency: "EUR".into(), amount: 150 },
        ]
    );
}

/// A checkout tip has no taker: the round's `tip` is set with no `Paid.by`.
#[test]
fn an_online_checkout_tip_is_no_persons() {
    let mut online = round("r1", 1000, None);
    online["tip"] = json!(100);
    let t = tips_by_person(&[online], "ALL", T0, T0 + 10).unwrap();
    assert!(t.is_empty());
}

#[test]
fn an_overflowing_sum_is_refused_not_wrapped() {
    let big = |at: i64| json!({"payments": [{"by": "ana", "tip": i64::MAX, "at": at}]});
    assert!(tips_by_person(&[big(T0), big(T0 + 1)], "ALL", T0, T0 + 10).is_err());
    assert!(tips_by_person(&[big(T0)], "ALL", T0, T0 + 10).is_ok(), "one alone is fine");
}

/// Refund `order` through the REAL `refund::decide`: start, then (if `complete`)
/// the money handed back. Returns the order as the refund left it.
fn refund(order: &Value, complete: bool, at: i64) -> Value {
    use crate::command::refund::{decide as refund_decide, RefundIn};
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut shelf = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    let id = order["id"].as_str().unwrap().to_string();
    let step = |o: &Value, complete: bool, hub: &mut dowiz_hub::Hub, shelf: &mut dowiz_hub::stock::StockLog| {
        let view = OrderView { order_id: id.clone(), kind: 3, seq: (at - 1) as u64, order_json: o.to_string() };
        let input = RefundIn {
            order_id: id.clone(), location_id: "v1".into(), by: "owner".into(),
            reason: "customer_request".into(), complete, now_ms: at, at_door: false, note: None,
        };
        refund_decide(hub, shelf, Some(&view), &input, "ALL").expect("the refund lands").0
    };
    let started = step(order, false, &mut hub, &mut shelf);
    if complete { step(&started, true, &mut hub, &mut shelf) } else { started }
}

fn confirmed(id: &str, total: i64) -> Value {
    let mut r = round(id, total, None);
    r["status"] = json!("CONFIRMED");
    r
}

/// A FULL REFUND TAKES THE TIP BACK. The round was paid with a tip by the
/// real `pay::decide`, then refunded to COMPENSATED_REFUND by the real
/// `refund::decide` (which leaves `tip` on the order, law 3): nobody earned it.
/// Its twin: the other round, paid the same way and not refunded, still counts.
#[test]
fn a_refunded_rounds_tip_is_not_earned() {
    let kept = pay(&confirmed("r1", 1000), 1000, 100, "ana", T0 + 1);
    let paid = pay(&confirmed("r2", 1000), 1000, 150, "ana", T0 + 2);
    let gone = refund(&paid, true, T0 + 3);
    assert_eq!(gone["status"], json!("COMPENSATED_REFUND"));
    assert_eq!(gone["tip"], json!(150), "the refund does not rewrite the tip (law 3)");
    assert!(gone["refund"]["owed"].as_i64().unwrap() >= 1150, "the tip is in what was handed back");
    let t = tips_by_person(&[kept, gone], "ALL", T0, T0 + 10).unwrap();
    assert_eq!(t, vec![PersonTip { by: "ana".into(), currency: "ALL".into(), amount: 100 }]);
}

/// A refund STARTED but not handed back still counts, as the takings do.
#[test]
fn a_refund_in_flight_still_counts_its_tip() {
    let paid = pay(&confirmed("r1", 1000), 1000, 150, "ana", T0 + 1);
    let flying = refund(&paid, false, T0 + 2);
    assert_eq!(flying["status"], json!("REFUNDING"));
    let t = tips_by_person(&[flying], "ALL", T0, T0 + 10).unwrap();
    assert_eq!(t, vec![PersonTip { by: "ana".into(), currency: "ALL".into(), amount: 150 }]);
}

/// Rejected and cancelled rounds carry no tip for anyone either.
#[test]
fn a_rejected_or_cancelled_rounds_tip_is_not_earned() {
    for st in ["REJECTED", "CANCELLED"] {
        let mut o = pay(&confirmed("r1", 1000), 1000, 150, "ana", T0 + 1);
        o["status"] = json!(st);
        assert!(tips_by_person(&[o], "ALL", T0, T0 + 10).unwrap().is_empty(), "{st}");
    }
}
