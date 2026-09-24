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
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: None, wallet: None, now_ms: 5_000,
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
