//! THE ONE CASE both builds are asked about, built from the hub's own API so
//! its provenance is code, not a hand-edited binary. `fixtures/decide/` holds
//! its bytes; `decide_agrees.rs` checks the committed files ARE what this
//! builds, and `BEBOP_WASM_BLESS=1` (re)writes them -- the only way they
//! change, and a change a reviewer sees as a diff of `*.json` and `*.delta`.
//!
//! Chosen to exercise what a byte-level disagreement would hide in: a
//! customer's own words with quotes and non-ASCII, a nested object the delta
//! recurses into, a non-order event in the middle of the order's history, a
//! delta already on the log (the fold, not just the snapshot), a second order,
//! a shelf the amendment re-reserves, a foreign-currency payment with a tip.

use dowiz_hub::stock::{StockEvent, StockLog};
use dowiz_hub::{EventKind, Hub};
use serde_json::json;

pub const NOW: i64 = 1_790_000_000_000;
pub const SEQ: u64 = 1_789_999_000_000;
pub const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/decide/");

pub fn dish(pid: &str, supply: &str, qty: i64) -> String {
    json!({ "id": pid, "bom": [{ "supply": supply, "qty": qty }] }).to_string()
}

fn boms() -> serde_json::Value {
    json!([["maki", dish("maki", "rice", 100)], ["beer", dish("beer", "keg", 1)], ["cake", dish("cake", "cake", 1)]])
}

/// r1 at table 7 (placed, then confirmed by a delta), r2 beside it, and an
/// owner's look at r1's contact in between.
pub fn amend_log() -> Vec<u8> {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r1 = json!({
        "id": "r1", "status": "PENDING", "location_id": "v1", "currency": "ALL",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
            {"product_id": "beer", "quantity": 1, "unit_price": 300, "name": "Birrë \"Korça\""}
        ],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "contact": {"name": "Ana Hoxha", "note": "pa qepë — \"shpejt\""},
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-000001", "customer_id": null
    });
    h.append(EventKind::Placed, "r1", &r1.to_string(), SEQ - 30, [0; 32]).unwrap();
    let r2 = json!({"id": "r2", "status": "PENDING", "location_id": "v1", "items": [], "total": 0});
    h.append(EventKind::Placed, "r2", &r2.to_string(), SEQ - 20, [0; 32]).unwrap();
    h.append(EventKind::Revealed, "r1", r#"{"who":"owner","at":1}"#, SEQ - 10, [0; 32]).unwrap();
    h.append(EventKind::Advanced, "r1", r#"{"status":"CONFIRMED","_d":true}"#, SEQ, [0; 32]).unwrap();
    h.to_bytes_trimmed()
}

/// Rice, a keg, ONE cake; r1's reservation as placement left it.
pub fn amend_stock() -> Vec<u8> {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append_all(&[
        StockEvent::Received { item: "rice".into(), qty: 1000 },
        StockEvent::Received { item: "keg".into(), qty: 10 },
        StockEvent::Received { item: "cake".into(), qty: 1 },
    ])
    .unwrap();
    let lines = vec![(dish("maki", "rice", 100), 2), (dish("beer", "keg", 1), 1)];
    s.append_all(&dowiz_hub::stock::reservations_for("r1", &lines)).unwrap();
    s.to_bytes_trimmed()
}

/// A cake added, the maki made three, the round moved to table 12.
pub fn amend_in() -> Vec<u8> {
    json!({
        "order_id": "r1", "location_id": "v1", "base_seq": SEQ, "by": "p1", "reason": null, "may_void": false,
        "ops": [
            {"op": "add", "line": {"product_id": "cake", "quantity": 1, "unit_price": 450, "name": "Tortë"}},
            {"op": "set_qty", "line": 0, "qty": 3},
            {"op": "table", "table": "12"}
        ],
        "boms": boms(), "now_ms": NOW
    })
    .to_string()
    .into_bytes()
}

/// The same tablet, one version behind: a stale edit, refused.
pub fn amend_stale_in() -> Vec<u8> {
    json!({
        "order_id": "r1", "location_id": "v1", "base_seq": SEQ - 30, "by": "p1", "reason": "mistake",
        "may_void": false, "ops": [{"op": "remove", "line": 1}], "boms": boms(), "now_ms": NOW
    })
    .to_string()
    .into_bytes()
}

/// r1 ready, 1500 lek.
pub fn pay_log() -> Vec<u8> {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r1 = json!({
        "id": "r1", "status": "READY", "location_id": "v1", "currency": "ALL",
        "items": [{"product_id": "maki", "quantity": 2, "unit_price": 750, "name": "Maki"}],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "payments": [{"by": "p2", "amount": 200, "method": "card", "currency": "ALL", "at": 1}]
    });
    h.append(EventKind::Placed, "r1", &r1.to_string(), SEQ, [0; 32]).unwrap();
    h.to_bytes_trimmed()
}

pub fn pay_room() -> Vec<u8> {
    br#"{"open_till":"main","venue_currency":"ALL"}"#.to_vec()
}

/// Ten euro in cash at 97.50, and a tip of 100 lek on top.
pub fn pay_in() -> Vec<u8> {
    json!({
        "order_id": "r1", "location_id": "v1", "amount": 1000, "method": "cash", "by": "p1",
        "currency": "EUR", "rate_ppm": 975_000, "tip": 100, "now_ms": NOW
    })
    .to_string()
    .into_bytes()
}

/// Every input file, by name.
pub fn inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("amend.log", amend_log()),
        ("amend.stock", amend_stock()),
        ("amend.in.json", amend_in()),
        ("amend_stale.in.json", amend_stale_in()),
        ("pay.log", pay_log()),
        ("pay.room.json", pay_room()),
        ("pay.in.json", pay_in()),
    ]
}

pub fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("fixtures/decide/{name}: {e} (BEBOP_WASM_BLESS=1 writes it)"))
}
