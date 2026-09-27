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

/// W-AUDIT F9 (2026-09-27): the newest ORDER event's seq, without a fold;
/// audit records on the same id do not count, and an unknown id is 0.
#[test]
fn latest_seq_is_the_newest_order_events_seq() {
    let mut h = crate::Hub::create_sized(64 * 1024).unwrap();
    assert_eq!(super::latest_seq(&h, "r1"), 0);
    let r = serde_json::json!({"id": "r1", "status": "PENDING", "location_id": "v1"}).to_string();
    h.append(crate::EventKind::Placed, "r1", &r, 100, [0; 32]).unwrap();
    h.append(crate::EventKind::Noted, "r1", "{}", 250, [0; 32]).unwrap();
    h.append(crate::EventKind::Revealed, "r1", "{}", 900, [0; 32]).unwrap();
    h.append(crate::EventKind::Placed, "r2", &r, 700, [0; 32]).unwrap();
    assert_eq!(super::latest_seq(&h, "r1"), 250);
    assert_eq!(super::latest_seq(&h, "r2"), 700);
}
