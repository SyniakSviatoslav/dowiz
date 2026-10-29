//! THE MIGRATION LAWS OF THE ORDER LOG (W-P0-LOG, R-BINREC P0, 2026-10-29).
//!
//! `tools/gates/migration-law.sh` fingerprints every order-log format (`log.kinds`, `log.frame`,
//! `log.order`, `log.delta`, `log.<kind>`) and refuses a change that does not name a test here.
//! These are the v1 laws: what an image written TODAY holds must read back the same tomorrow.
//! The fixtures are FROZEN bytes, in the shapes today's writers produce -- never generate them
//! from the current code, or the test agrees with whatever the code becomes.
//!
//! A later version of one format adds a field and ships its own test with both laws
//! (Delta o Sigma = id on these old records; readers agree, decode(old) == decode(encode(decode(old))))
//! -- see `stock/migration_tests.rs` for the shape.

use super::{decode, EventKind};
use crate::Hub;
use bebop_store::evlog::{EvLog, Record};
use serde_json::{json, Value};

const TIP: &str = "5be0c1a2d3e4f5061728394a5b6c7d8e9fa0b1c2d3e4f5061728394a5b6c7d8e";

/// One frozen record per kind: `(kind byte, order id, payload text)`, as written today.
fn fixtures() -> Vec<(u8, &'static str, String)> {
    vec![
        (1, "ord_1", r#"{"id":"ord_1","customer_id":null,"status":"PENDING","items":[{"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":900}],"subtotal":1800,"total":1650,"created_at_ms":1789000000000,"channel":"web","cash_pay_with":null,"discount":150,"promo":{"code":"HELLO","discount":150}}"#.into()),
        (2, "ord_1", r#"{"status":"CONFIRMED","rejection_reason":null,"_d":true}"#.into()),
        (3, "ord_1", r#"{"payment_status":"paid","payments":[{"method":"cash","amount":1650,"currency":"ALL","at":1789000060000,"by":"u1"}],"_d":true}"#.into()),
        (4, "cust:k1", r#"{"by":"u1","at":1789000070000,"reason":"call back about the order","id":"k1"}"#.into()),
        (5, "ord_1", r#"{"kitchen":{"seen":true,"at":1789000080000,"by":"u2"},"_d":true}"#.into()),
        (6, "", format!("tip={TIP} events=3 bytes=65536")),
        (7, "ord_1", r#"{"items":[{"product_id":"p1","modifier_ids":["m1"],"quantity":1,"unit_price":900}],"subtotal":900,"total":900,"amended":[{"by":"u1","at":1789000090000,"reason":"guest changed mind"}],"_d":true}"#.into()),
        (8, "cust:k1", r#"{"records":2,"redacted":["ord_1"],"by":"u1","at":1789000100000,"reason":"erasure request"}"#.into()),
    ]
}

fn frame(kind: u8, id: &str, body: &str) -> Vec<u8> {
    let mut p = vec![kind, id.len() as u8];
    p.extend_from_slice(id.as_bytes());
    p.extend_from_slice(body.as_bytes());
    p
}

fn record(payload: Vec<u8>) -> Record {
    Record { id: [0u8; 32], prev: [0u8; 32], actor_pubkey: [0u8; 32], actor_seq: 7, payload }
}

/// Base law for `log.kinds`, `log.frame` and every `log.<kind>` at v1: the frozen bytes decode to
/// the kind, id and text they hold; `Hub::append` of that event writes exactly the frozen bytes
/// (re-encoding is a fixed point); a redacted copy (high bit) still names its kind; and every
/// JSON payload is still an object.
#[test]
fn every_log_kind_round_trips() {
    let all = fixtures();
    assert_eq!(all.len(), 8, "one fixture per EventKind");
    let mut hub = Hub::create_sized(128 * 1024).unwrap();
    for (seq, (byte, id, body)) in all.iter().enumerate() {
        let old = frame(*byte, id, body);
        let ev = decode(&record(old.clone())).unwrap_or_else(|| panic!("frozen kind {byte} no longer decodes"));
        assert_eq!(ev.kind as u8, *byte, "kind byte {byte} changed meaning");
        assert_eq!((ev.order_id.as_str(), ev.order_json.as_str()), (*id, body.as_str()));
        assert_eq!(ev.seq, 7);

        let mut red = old.clone();
        red[0] |= crate::forget::REDACTED_BIT;
        assert_eq!(decode(&record(red)).map(|e| e.kind), Some(ev.kind), "redacted {byte} lost its kind");

        hub.append(ev.kind, &ev.order_id, &ev.order_json, seq as u64 + 1, [0u8; 32]).unwrap();
        let walked = EvLog::walk(&hub.store);
        assert_eq!(walked[0].payload, old, "re-encoding kind {byte} is not a fixed point");
        assert_eq!(decode(&walked[0]).map(|e| (e.kind, e.order_id, e.order_json)), Some((ev.kind, ev.order_id, ev.order_json)));

        if ev.kind != EventKind::Checkpoint {
            assert!(serde_json::from_str::<Value>(body).map(|v| v.is_object()).unwrap_or(false), "kind {byte}: {body}");
        }
    }
    assert_eq!(EventKind::from_u8(9), None, "a ninth kind is a new format: register it");
}

/// Base law for `log.order` v1: the kernel writes the frozen snapshot's keys in the frozen order,
/// and reads an old snapshot back (`apply_event_logic` decodes, steps, re-encodes) keeping every
/// field but the status it was asked to move.
#[test]
fn the_kernel_snapshot_round_trips() {
    let keys = |v: &Value| v.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default();
    let items = r#"[{"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":900}]"#;
    let placed = dowiz_core::json_api::place_order_at("ord_1".into(), None, items, 1789000000000, Some("web".into())).unwrap();
    let placed: Value = serde_json::from_str(&placed).unwrap();
    let old: Value = serde_json::from_str(
        r#"{"id":"ord_1","customer_id":null,"status":"PENDING","items":[{"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":900}],"subtotal":1800,"total":1800,"created_at_ms":1789000000000,"channel":"web","cash_pay_with":null}"#,
    )
    .unwrap();
    assert_eq!(placed, old, "the kernel no longer writes the v1 snapshot");
    assert_eq!(keys(&placed), keys(&old), "key order");
    assert_eq!(keys(&placed["items"][0]), keys(&old["items"][0]), "item key order");

    let next: Value = serde_json::from_str(&dowiz_core::json_api::apply_event_logic(&old.to_string(), "CONFIRMED").unwrap()).unwrap();
    let mut expect = old.clone();
    expect["status"] = json!("CONFIRMED");
    assert_eq!(next, expect, "an old snapshot read and written again lost or changed a field");
}

/// Base law for `log.delta` v1: a frozen history -- snapshot, then `_d` deltas, one with a `_x`
/// drop list -- still folds to the order it always folded to, and a newer unmarked snapshot
/// still REPLACES the state.
#[test]
fn old_deltas_still_fold() {
    let history = [
        r#"{"id":"ord_1","status":"PENDING","total":1800,"note":"no wasabi"}"#,
        r#"{"status":"CONFIRMED","_d":true}"#,
        r#"{"kitchen":{"seen":true},"_x":["note"],"_d":true}"#,
    ];
    let folded = crate::room::delta::fold(history.iter().copied());
    assert_eq!(folded, json!({"id":"ord_1","status":"CONFIRMED","total":1800,"kitchen":{"seen":true}}));
    let replaced = crate::room::delta::fold(history.iter().copied().chain([r#"{"id":"ord_1","status":"READY"}"#]));
    assert_eq!(replaced, json!({"id":"ord_1","status":"READY"}), "an unmarked payload is a snapshot");
}
