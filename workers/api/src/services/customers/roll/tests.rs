//! A table is not a customer (the live walk, 2026-09-24: every room round
//! rolled into ONE "—" row, 8 orders / 7300 ALL). The fold runs with the REAL
//! `customer_key`, because the defect lives in the key: no digits is one
//! constant key per venue, so a pass-through key would never have seen it.

use super::*;
use crate::services::customers::handlers::customer_key;
use serde_json::json;

const SECRET: &[u8] = b"test-secret";

/// A round as `public/room/open.js` places it: the table's, no person.
fn round(at: i64, total: i64) -> Value {
    json!({
        "contact": { "name": "", "phone": "" }, "fulfilment": { "kind": "dine_in", "table": "7" },
        "created_at_ms": at, "total": total, "tip": 0, "status": "DELIVERED"
    })
}

fn guest(phone: &str, name: &str, at: i64, total: i64) -> Value {
    json!({ "contact": { "name": name, "phone": phone }, "created_at_ms": at, "total": total, "tip": 0, "status": "DELIVERED" })
}

fn rolled(os: &[Value]) -> Vec<Row> {
    roll(os, |p| customer_key(SECRET, p), |k| k.to_string(), |n| n.to_string(), |p| p.to_string(), Sort::Recent)
}

#[test]
fn rounds_without_a_phone_are_no_customer() {
    let os: Vec<Value> = (0..8).map(|i| round(10 + i, 900)).collect();
    assert!(rolled(&os).is_empty(), "eight table rounds are not one person");
    // Nor is a phone made of nothing but punctuation or the access prefix.
    for p in ["   ", "-", "+", "00", "( )"] {
        assert!(rolled(&[guest(p, "X", 1, 100)]).is_empty(), "{p:?}");
    }
}

/// THE POSITIVE TWIN: the same log with two real guests in it keeps both, with
/// only their own orders and spend; the rounds beside them change nothing.
#[test]
fn a_guest_with_a_phone_is_still_a_customer_beside_the_rounds() {
    let mut os: Vec<Value> = (0..8).map(|i| round(10 + i, 900)).collect();
    os.push(guest("+355691234567", "Ana", 50, 1500));
    os.push(guest("00355691234567", "Ana", 60, 500));
    os.push(guest("0691111111", "Ben", 40, 700));
    let r = rolled(&os);
    assert_eq!(r.len(), 2);
    assert_eq!((r[0].name.as_str(), r[0].orders, r[0].spent), ("Ana", 2, 2000));
    assert_eq!((r[1].name.as_str(), r[1].orders, r[1].spent), ("Ben", 1, 700));
}

#[test]
fn names_a_person_is_about_digits() {
    for p in ["", " ", "-", "00", "+ ()"] {
        assert!(!names_a_person(p), "{p:?}");
    }
    for p in ["1", "+355691234567", "069 123 4567", "0042"] {
        assert!(names_a_person(p), "{p:?}");
    }
}
