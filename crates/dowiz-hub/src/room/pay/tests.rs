//! The payment over the hub's own image. The Worker's `command/pay/tests.rs`
//! holds G5's refusals against this same code; these pin the sum rule and
//! `paid_of`, which moved here from the Worker's sitting fold.

use super::*;
use crate::{EventKind, Hub};

const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn log() -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r = json!({"id": "r1", "status": "READY", "location_id": "v1", "tip": 0, "total": 1500});
    h.append(EventKind::Placed, "r1", &r.to_string(), 100, [0; 32]).unwrap();
    h
}

fn cash(amount: i64) -> PayIn {
    PayIn {
        order_id: "r1".into(), location_id: "v1".into(), amount, method: "cash".into(), by: "p1".into(),
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: None, wallet: None, base_seq: None, now_ms: 5_000,
    }
}

#[test]
fn the_bill_closes_at_its_total_and_not_one_unit_past_it() {
    let mut h = log();
    let cur = crate::room::view::current(&h, "r1");
    assert!(matches!(decide(&mut h, cur.as_ref(), &cash(1501), &OPEN), Err(Refused::Conflict(_))));
    assert_eq!(h.len(), 1, "a refusal appends nothing");
    let (order, body, _) = decide(&mut h, cur.as_ref(), &cash(1500), &OPEN).expect("exactly the bill");
    assert_eq!(order["payment_status"], json!("paid"));
    assert_eq!(paid_of(&order), 1500);
    assert_eq!(h.events()[0].kind, EventKind::Paid);
    assert!(body.contains("\"till_id\":\"main\""), "the object stamps the open till: {body}");
}

#[test]
fn paid_of_counts_the_tip_and_the_converted_amount() {
    let o = json!({"payments": [
        {"amount": 2000, "amount_in_order_currency": 1950, "currency": "EUR"},
        {"amount": 100, "tip": 50}
    ]});
    assert_eq!(paid_of(&o), 1950 + 150);
    assert_eq!(paid_of(&json!({})), 0);
}

fn round(status: &str, placed_by: &str) -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r = json!({"id": "r1", "status": status, "location_id": "v1", "tip": 0, "total": 1500, "placed_by": placed_by});
    h.append(EventKind::Placed, "r1", &r.to_string(), 100, [0; 32]).unwrap();
    h
}

/// D4 / D9 (G4): a guest's QR round nobody has confirmed is not yet the
/// venue's order. Paid now and rejected next, its money had no way back
/// (refund refuses PENDING, REJECTED -> REFUNDING is no edge).
#[test]
fn an_unconfirmed_guest_round_takes_no_payment() {
    let mut h = round("PENDING", "guest");
    let cur = crate::room::view::current(&h, "r1");
    let r = decide(&mut h, cur.as_ref(), &cash(1500), &OPEN);
    assert!(matches!(&r, Err(Refused::Conflict(m)) if m.contains("confirm")), "{r:?}");
    assert_eq!(h.len(), 1, "a refusal appends nothing");
}

/// The positive twins: the same guest round once a waiter confirmed it, and
/// a waiter's own PENDING round (pay-first at the bar), both take payment.
#[test]
fn a_confirmed_guest_round_and_a_staff_pending_round_take_payment() {
    for (status, by) in [("CONFIRMED", "guest"), ("PENDING", "p1")] {
        let mut h = round(status, by);
        let cur = crate::room::view::current(&h, "r1");
        let (o, _, _) = decide(&mut h, cur.as_ref(), &cash(1500), &OPEN).unwrap_or_else(|r| panic!("{status}/{by}: {r:?}"));
        assert_eq!(o["payment_status"], json!("paid"));
    }
}

/// D7 (G4): the same partial payment, taken twice from one screen's view of
/// the round (two waiters splitting, or an offline re-tap under a fresh key).
/// The second quotes a seq the round has moved past and is refused.
#[test]
fn a_payment_quoting_a_stale_seq_is_refused_so_700_is_not_taken_twice() {
    let mut h = log();
    let seen = crate::room::view::current(&h, "r1").unwrap().seq;
    let first = PayIn { base_seq: Some(seen), ..cash(700) };
    let cur = crate::room::view::current(&h, "r1");
    decide(&mut h, cur.as_ref(), &first, &OPEN).expect("the first 700 lands");
    let again = PayIn { base_seq: Some(seen), now_ms: 5_001, ..cash(700) };
    let cur = crate::room::view::current(&h, "r1");
    let r = decide(&mut h, cur.as_ref(), &again, &OPEN);
    assert!(matches!(&r, Err(Refused::Conflict(m)) if m.contains("changed")), "{r:?}");
    let o: Value = serde_json::from_str(&crate::room::view::current(&h, "r1").unwrap().order_json).unwrap();
    assert_eq!(paid_of(&o), 700, "one 700, not two");
}

/// The twin: quoting the round's CURRENT seq lands, and so does a payment
/// from a client that sends none (older than the rule).
#[test]
fn a_payment_quoting_the_current_seq_or_none_lands() {
    let mut h = log();
    let cur = crate::room::view::current(&h, "r1");
    let now = PayIn { base_seq: Some(cur.as_ref().unwrap().seq), ..cash(700) };
    let (_, _, seq) = decide(&mut h, cur.as_ref(), &now, &OPEN).expect("current seq");
    let next = PayIn { base_seq: Some(seq), now_ms: 5_001, ..cash(700) };
    let cur = crate::room::view::current(&h, "r1");
    decide(&mut h, cur.as_ref(), &next, &OPEN).expect("the seq the first answer carried");
    let cur = crate::room::view::current(&h, "r1");
    let (o, _, _) = decide(&mut h, cur.as_ref(), &PayIn { now_ms: 5_002, ..cash(100) }, &OPEN).expect("no base_seq");
    assert_eq!(o["payment_status"], json!("paid"));
}
