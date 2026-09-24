//! `current` is the projection's answer for one id.

use super::*;
use crate::EventKind;

fn hub() -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.append(EventKind::Placed, "r1", r#"{"id":"r1","status":"PENDING","total":5}"#, 10, [0; 32]).unwrap();
    h.append(EventKind::Placed, "r2", r#"{"id":"r2","status":"PENDING"}"#, 11, [0; 32]).unwrap();
    h.append(EventKind::Revealed, "r1", r#"{"who":"owner"}"#, 12, [0; 32]).unwrap();
    h.append(EventKind::Advanced, "r1", r#"{"status":"CONFIRMED","_d":true}"#, 13, [0; 32]).unwrap();
    h
}

#[test]
fn the_fold_of_one_order_skips_other_orders_and_non_order_events() {
    let v = current(&hub(), "r1").expect("r1 is in the log");
    assert_eq!((v.kind, v.seq), (EventKind::Advanced as u8, 13), "the NEWEST order event, not the Revealed");
    let o: serde_json::Value = serde_json::from_str(&v.order_json).unwrap();
    assert_eq!(o, serde_json::json!({"id": "r1", "status": "CONFIRMED", "total": 5}));
    assert_eq!(current(&hub(), "r2").map(|v| v.seq), Some(11));
}

#[test]
fn an_order_the_log_does_not_hold_is_none() {
    assert_eq!(current(&hub(), "r9"), None);
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    h.append(EventKind::Placed, "r3", "not json", 1, [0; 32]).unwrap();
    assert_eq!(current(&h, "r3"), None, "a fold that is null is not an order");
}
