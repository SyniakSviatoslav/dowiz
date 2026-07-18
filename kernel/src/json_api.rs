//! `json_api` — the kernel's JSON string boundary, shared by BOTH the wasm JS
//! surface and the native HTTP adapter (P37 `native-spa-server`).
//!
//! This module is the load-bearing extraction target of BLUEPRINT-P37 (W37-1):
//! the order `place_order_logic` / `apply_event_logic` functions used to live
//! PRIVATE and `#[cfg(feature = "wasm")]`-gated inside `wasm.rs`. They are moved
//! here verbatim, made `pub`, and gated behind the new `json-api` feature. The
//! `wasm` feature now enables `json-api` (a superset), so the browser surface is
//! unchanged and the two surfaces share ONE JSON authority.
//!
//! CRITICAL SERDE-FREE DEFAULT DISCIPLINE: this entire module is
//! `#[cfg(feature = "json-api")]`. The DEFAULT (no-feature) kernel build pulls
//! NONE of serde / serde_json — the native rlib graph stays pure-`std`. The
//! `cargo tree -p dowiz-kernel --no-default-features -e no-dev | grep -c serde`
//! gate (documented at lib.rs:274) must remain `0`.

#![cfg(feature = "json-api")]

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::domain::{apply_event, place_order, Order, OrderItem};
use crate::order_machine::{OrderStatus, TransitionError};

/// Monotonic id / timestamp source (mirrors the wasm surface). Deterministic and
/// order-preserving; the HTTP adapter keys the volatile store on the returned id.
static ORDER_SEQ: AtomicU64 = AtomicU64::new(0);

// ── Wire (JSON) shapes — the ONLY structs serde touches here ────────────────

#[derive(Deserialize)]
struct ItemInput {
    product_id: String,
    #[serde(default)]
    modifier_ids: Vec<String>,
    quantity: i64,
    unit_price: i64,
}

#[derive(Serialize)]
struct ItemOut {
    product_id: String,
    modifier_ids: Vec<String>,
    quantity: i64,
    unit_price: i64,
}

#[derive(Deserialize)]
struct OrderIn {
    id: String,
    #[serde(default)]
    customer_id: Option<String>,
    status: String,
    items: Vec<ItemInput>,
    subtotal: i64,
    total: i64,
    created_at_ms: i64,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    cash_pay_with: Option<String>,
}

#[derive(Serialize)]
struct OrderOut {
    id: String,
    customer_id: Option<String>,
    status: String,
    items: Vec<ItemOut>,
    subtotal: i64,
    total: i64,
    created_at_ms: i64,
    channel: Option<String>,
    cash_pay_with: Option<String>,
}

// ── Internal mapping helpers (free of wasm_bindgen / JsValue) ───────────────

fn item_to_domain(i: ItemInput) -> OrderItem {
    OrderItem {
        product_id: i.product_id,
        modifier_ids: i.modifier_ids,
        quantity: i.quantity,
        unit_price: i.unit_price,
        vendor_id: VendorId(0),
        currency: Currency::All,
    }
}

fn item_from_domain(i: &OrderItem) -> ItemOut {
    ItemOut {
        product_id: i.product_id.clone(),
        modifier_ids: i.modifier_ids.clone(),
        quantity: i.quantity,
        unit_price: i.unit_price,
    }
}

fn order_from_in(o: OrderIn) -> Result<Order, String> {
    let status = OrderStatus::from_str(&o.status)
        .ok_or_else(|| format!("unknown OrderStatus: {}", o.status))?;
    let items: Vec<OrderItem> = o.items.into_iter().map(item_to_domain).collect();

    // V3 1.2 / 5.6 (ROUND-2 GAP-AUDIT, E1 forged-order-total): the `subtotal`
    // and `total` carried in the untrusted JSON are attacker-controlled and MUST
    // NOT be trusted. Recompute them server-authoritatively from the items
    // (Layer G money recompute) so a forged total cannot survive a fold. The
    // JSON values are dropped.
    let subtotal = Order::compute_subtotal(&items)
        .map_err(|e| format!("order_from_in: {}", e))?;
    // Total is provisional (tax/fee not folded until a server estimate) — matching
    // place_order, which sets total = subtotal on creation.
    let total = subtotal;

    Ok(Order {
        id: o.id,
        customer_id: o.customer_id,
        status,
        items,
        subtotal,
        total,
        created_at_ms: o.created_at_ms,
        channel: o.channel,
        cash_pay_with: o.cash_pay_with,
        // JS-boundary reconstruction: this path deserializes an order shape that
        // did not carry the trust flag → conservatively UNTRUSTED (fail-closed).
        price_trusted: false,
        ledger: Vec::new(),
    })
}

fn order_to_out(o: &Order) -> OrderOut {
    OrderOut {
        id: o.id.clone(),
        customer_id: o.customer_id.clone(),
        status: o.status.as_str().to_string(),
        items: o.items.iter().map(item_from_domain).collect(),
        subtotal: o.subtotal,
        total: o.total,
        created_at_ms: o.created_at_ms,
        channel: o.channel.clone(),
        cash_pay_with: o.cash_pay_with.clone(),
    }
}

fn status_err(e: TransitionError) -> String {
    // Mirror the oracle's error reporting: human-readable message string.
    e.message()
}

/// Create a new `Pending` order from a JSON item list. The kernel authority for
/// BOTH the wasm surface and the HTTP adapter (P37 W37-1). Returns the created
/// [`Order`] serialized to JSON, or an error string (fail-closed on malformed
/// input / illegal quantity / price).
pub fn place_order_logic(
    customer_id: Option<String>,
    items_json: &str,
    channel: Option<String>,
) -> Result<String, String> {
    let items_in: Vec<ItemInput> = serde_json::from_str(items_json).map_err(|e| e.to_string())?;
    let items: Vec<OrderItem> = items_in.into_iter().map(item_to_domain).collect();

    // V3 1.3 (ROUND-2 GAP-AUDIT): a negative quantity or unit price is malformed
    // input that would produce a negative/garbage order total. Refuse before any
    // domain mutation (fail-closed on the untrusted-JSON boundary).
    for it in &items {
        if it.quantity <= 0 {
            return Err(format!(
                "place_order: quantity must be >= 1, got {}",
                it.quantity
            ));
        }
        if it.unit_price < 0 {
            return Err(format!(
                "place_order: unit_price must be >= 0, got {}",
                it.unit_price
            ));
        }
    }

    let seq = ORDER_SEQ.fetch_add(1, Ordering::SeqCst);
    let id = format!("ord_{}", seq);
    let created_at_ms = seq as i64;

    let order = place_order(
        id,
        customer_id,
        items,
        created_at_ms,
        channel,
        None, // cash_pay_with is not part of the JSON placement surface
    )
    .map_err(status_err)?;

    let out = order_to_out(&order);
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

/// Advance an order one step along the kernel FSM. `next_status` is the status
/// name (e.g. "CONFIRMED"). Returns the updated order JSON, or an error string on
/// an illegal transition (same status / illegal edge / scaffold disabled). The
/// kernel recomputes `subtotal`/`total` from items — the wire total is never
/// trusted (V3 1.2).
pub fn apply_event_logic(order_json: &str, next_status: &str) -> Result<String, String> {
    let parsed: OrderIn = serde_json::from_str(order_json).map_err(|e| e.to_string())?;
    let order = order_from_in(parsed)?;

    let next = OrderStatus::from_str(next_status)
        .ok_or_else(|| format!("unknown OrderStatus: {}", next_status))?;

    let updated = apply_event(&order, next).map_err(status_err)?;
    let out = order_to_out(&updated);
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ITEMS: &str = r#"[
        {"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":500},
        {"product_id":"p2","modifier_ids":[],"quantity":1,"unit_price":300}
    ]"#;

    #[test]
    fn apply_event_recomputes_forged_total_from_items() {
        // V3 1.2 / 5.6 (ROUND-2 GAP-AUDIT, E1 forged-order-total): an attacker
        // controls the `total`/`subtotal` fields in the JSON they hand to
        // apply_event_logic. The kernel must recompute them from the items, never
        // trust the wire value.
        let json = place_order_logic(Some("c1".into()), SAMPLE_ITEMS, Some("web".into()))
            .expect("place_order_logic ok");

        // Tamper: stamp a forged total onto the serialized order.
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let forged = 9_999_999i64;
        v["total"] = serde_json::json!(forged);
        v["subtotal"] = serde_json::json!(forged);
        let tampered = serde_json::to_string(&v).unwrap();

        let updated = apply_event_logic(&tampered, "CONFIRMED")
            .expect("apply_event_logic ok");
        let out: serde_json::Value = serde_json::from_str(&updated).unwrap();

        // True total = 2*500 + 300 = 1300 (provisional, no tax/fee folded).
        let expected = 2 * 500 + 300;
        assert_ne!(out["total"], forged, "forged total must NOT survive the fold");
        assert_eq!(out["total"], expected, "total must be recomputed from items");
        assert_ne!(out["subtotal"], forged, "forged subtotal must NOT survive");
        assert_eq!(out["subtotal"], expected);
    }

    #[test]
    fn place_order_rejects_negative_quantity_and_price() {
        // V3 1.3 (ROUND-2 GAP-AUDIT): malformed input (negative qty / price) must
        // be refused fail-closed, not produce a negative/garbage total.
        let neg_qty = r#"[{"product_id":"p1","modifier_ids":[],"quantity":-2,"unit_price":500}]"#;
        let r1 = place_order_logic(None, neg_qty, None);
        assert!(r1.is_err(), "negative quantity must be refused");
        assert!(
            r1.unwrap_err().contains("quantity"),
            "error must name the quantity violation"
        );

        let neg_price = r#"[{"product_id":"p1","modifier_ids":[],"quantity":1,"unit_price":-500}]"#;
        let r2 = place_order_logic(None, neg_price, None);
        assert!(r2.is_err(), "negative unit_price must be refused");
        assert!(
            r2.unwrap_err().contains("unit_price"),
            "error must name the price violation"
        );

        // Zero quantity (no items) is also refused.
        let zero_qty = r#"[{"product_id":"p1","modifier_ids":[],"quantity":0,"unit_price":500}]"#;
        assert!(
            place_order_logic(None, zero_qty, None).is_err(),
            "zero quantity must be refused"
        );
    }

    #[test]
    fn apply_event_happy_and_illegal() {
        let created = place_order_logic(None, SAMPLE_ITEMS, None).unwrap();
        let confirmed = apply_event_logic(&created, "CONFIRMED").unwrap();
        let v: serde_json::Value = serde_json::from_str(&confirmed).unwrap();
        assert_eq!(v["status"], "CONFIRMED");

        // Pending -> Delivered is illegal; must surface an error string.
        let bad = apply_event_logic(&created, "DELIVERED");
        assert!(bad.is_err());
        let msg = bad.unwrap_err();
        assert!(
            msg.contains("Illegal"),
            "expected illegal-transition error, got: {msg}"
        );

        // Unknown status name rejected.
        let unknown = apply_event_logic(&confirmed, "NOPE");
        assert!(unknown.is_err());
    }

    #[test]
    fn round_trip_full_order_json() {
        // apply_event_logic must accept the exact JSON place_order_logic produced.
        let created =
            place_order_logic(Some("c9".into()), SAMPLE_ITEMS, Some("app".into())).unwrap();
        // advance Pending -> Confirmed -> Preparing
        let c = apply_event_logic(&created, "CONFIRMED").unwrap();
        let p = apply_event_logic(&c, "PREPARING").unwrap();
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(v["status"], "PREPARING");
        assert_eq!(v["channel"], "app");
        assert_eq!(v["customer_id"], "c9");
    }
}
