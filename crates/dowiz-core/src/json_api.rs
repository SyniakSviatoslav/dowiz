//! `dowiz_core::json_api` — the kernel's JSON order bridge, now no_std.
//!
//! This is the load-bearing order boundary shared by BOTH the wasm JS surface and
//! the native HTTP adapter (P37 W37-1). Moved from `kernel/src/json_api.rs`
//! (wave-58): `serde`/`serde_json` were replaced by the hand-rolled `crate::json`
//! parser/serializer, so the order bridge is now no_std and serde-free. The
//! DEFAULT (no-feature) kernel build already pulled NONE of serde; this removes
//! serde_json from the `json-api` runtime graph too (it survives only as the
//! dev-dependency differential oracle).
//!
//! Byte-compatibility: `crate::json::Value::to_string()` emits the same compact
//! JSON as `serde_json::to_string` for these shapes — insertion-order objects,
//! minimal string escaping, `i64` without a decimal point, shortest-round-trip
//! floats. The old `#[serde(default)]` wire semantics are reproduced by the
//! manual field getters below (missing `Option`/`Vec` → `None`/empty; a missing
//! REQUIRED field → `Err`). `subtotal`/`total` are always recomputed from items
//! (Layer G money recompute), so their wire presence/absence is irrelevant to
//! correctness.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::domain::{apply_event, place_order, Order, OrderItem};
use crate::json::{parse, Value};
use crate::money::Currency;
use crate::order_machine::{OrderStatus, TransitionError};
use crate::vendor::VendorId;

/// Monotonic id / timestamp source (mirrors the wasm surface). Deterministic and
/// order-preserving; the HTTP adapter keys the volatile store on the returned id.
static ORDER_SEQ: AtomicU64 = AtomicU64::new(0);

// ── Field extraction helpers (reproduce the old `#[derive(Deserialize)]` shapes) ──

/// Required string field.
fn field_str(v: &Value, key: &str) -> Result<String, String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing or non-string field `{}`", key))
}

/// `Option<String>` with `#[serde(default)]`: missing or `null` → `None`.
fn field_opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(|s| s.to_string())
}

/// `Vec<String>` with `#[serde(default)]`: missing or `null` → empty.
fn field_str_vec(v: &Value, key: &str) -> Result<Vec<String>, String> {
    match v.get(key) {
        None => Ok(Vec::new()),
        Some(x) if x.is_null() => Ok(Vec::new()),
        Some(x) => {
            let arr = x
                .as_array()
                .ok_or_else(|| format!("`{}` must be an array", key))?;
            arr.iter()
                .map(|e| {
                    e.as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| format!("`{}` has a non-string element", key))
                })
                .collect()
        }
    }
}

/// Required `i64` field.
fn field_i64(v: &Value, key: &str) -> Result<i64, String> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("missing or non-integer field `{}`", key))
}

// ── Parse: wire JSON → domain ──

fn item_value_to_domain(v: &Value) -> Result<OrderItem, String> {
    Ok(OrderItem {
        product_id: field_str(v, "product_id")?,
        modifier_ids: field_str_vec(v, "modifier_ids")?,
        quantity: field_i64(v, "quantity")?,
        unit_price: field_i64(v, "unit_price")?,
        vendor_id: VendorId(0),
        currency: Currency::All,
    })
}

fn parse_items(json: &str) -> Result<Vec<OrderItem>, String> {
    let v = parse(json).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("items must be a JSON array")?;
    arr.iter().map(item_value_to_domain).collect()
}

fn order_from_json(json: &str) -> Result<Order, String> {
    let v = parse(json).map_err(|e| e.to_string())?;
    let status_str = field_str(&v, "status")?;
    let status = OrderStatus::from_str(&status_str)
        .ok_or_else(|| format!("unknown OrderStatus: {}", status_str))?;
    let items = v
        .get("items")
        .and_then(Value::as_array)
        .ok_or("`items` must be a JSON array")?
        .iter()
        .map(item_value_to_domain)
        .collect::<Result<Vec<_>, _>>()?;

    // V3 1.2 / 5.6 (ROUND-2 GAP-AUDIT, E1 forged-order-total): the `subtotal`
    // and `total` carried in the untrusted JSON are attacker-controlled and MUST
    // NOT be trusted. Recompute them server-authoritatively from the items
    // (Layer G money recompute) so a forged total cannot survive a fold. The
    // JSON values are dropped (and, unlike the old serde wire type, are not even
    // required to be present).
    let subtotal = Order::compute_subtotal(&items).map_err(|e| format!("order_from_in: {}", e))?;
    // Total is provisional (tax/fee not folded until a server estimate) — matching
    // place_order, which sets total = subtotal on creation.
    let total = subtotal;

    Ok(Order {
        id: field_str(&v, "id")?,
        customer_id: field_opt_str(&v, "customer_id"),
        status,
        items,
        subtotal,
        total,
        created_at_ms: field_i64(&v, "created_at_ms")?,
        channel: field_opt_str(&v, "channel"),
        cash_pay_with: field_opt_str(&v, "cash_pay_with"),
        // JS-boundary reconstruction: this path deserializes an order shape that
        // did not carry the trust flag → conservatively UNTRUSTED (fail-closed).
        price_trusted: false,
        ledger: Vec::new(),
    })
}

// ── Serialize: domain → wire JSON ──

fn item_to_value(i: &OrderItem) -> Value {
    Value::Object(vec![
        (String::from("product_id"), Value::from(i.product_id.clone())),
        (
            String::from("modifier_ids"),
            Value::Array(i.modifier_ids.iter().map(|m| Value::from(m.clone())).collect()),
        ),
        (String::from("quantity"), Value::from(i.quantity)),
        (String::from("unit_price"), Value::from(i.unit_price)),
    ])
}

fn order_to_value(o: &Order) -> Value {
    Value::Object(vec![
        (String::from("id"), Value::from(o.id.clone())),
        (String::from("customer_id"), Value::from(o.customer_id.clone())),
        (String::from("status"), Value::from(o.status.as_str().to_string())),
        (
            String::from("items"),
            Value::Array(o.items.iter().map(item_to_value).collect()),
        ),
        (String::from("subtotal"), Value::from(o.subtotal)),
        (String::from("total"), Value::from(o.total)),
        (String::from("created_at_ms"), Value::from(o.created_at_ms)),
        (String::from("channel"), Value::from(o.channel.clone())),
        (String::from("cash_pay_with"), Value::from(o.cash_pay_with.clone())),
    ])
}

fn status_err(e: TransitionError) -> String {
    // Mirror the oracle's error reporting: human-readable message string.
    e.message()
}

// ── Public entry points ──

/// Create a new `Pending` order from a JSON item list. The kernel authority for
/// BOTH the wasm surface and the HTTP adapter (P37 W37-1). Returns the created
/// [`Order`] serialized to JSON, or an error string (fail-closed on malformed
/// input / illegal quantity / price).
pub fn place_order_logic(
    customer_id: Option<String>,
    items_json: &str,
    channel: Option<String>,
) -> Result<String, String> {
    let items = parse_items(items_json)?;

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

    Ok(order_to_value(&order).to_string())
}

/// Advance an order one step along the kernel FSM. `next_status` is the status
/// name (e.g. "CONFIRMED"). Returns the updated order JSON, or an error string on
/// an illegal transition (same status / illegal edge / scaffold disabled). The
/// kernel recomputes `subtotal`/`total` from items — the wire total is never
/// trusted (V3 1.2).
pub fn apply_event_logic(order_json: &str, next_status: &str) -> Result<String, String> {
    let order = order_from_json(order_json)?;

    let next = OrderStatus::from_str(next_status)
        .ok_or_else(|| format!("unknown OrderStatus: {}", next_status))?;

    let updated = apply_event(&order, next).map_err(status_err)?;
    Ok(order_to_value(&updated).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ITEMS: &str = r#"[
        {"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":500},
        {"product_id":"p2","modifier_ids":[],"quantity":1,"unit_price":300}
    ]"#;

    /// Stamp forged `total`/`subtotal` onto a serialized order, for the tamper test.
    fn forge_totals(json: &str, forged: i64) -> String {
        let mut v = parse(json).unwrap();
        if let Value::Object(m) = &mut v {
            for (k, val) in m.iter_mut() {
                if k == "total" || k == "subtotal" {
                    *val = Value::Int(forged);
                }
            }
        }
        v.to_string()
    }

    #[test]
    fn apply_event_recomputes_forged_total_from_items() {
        // V3 1.2 / 5.6 (ROUND-2 GAP-AUDIT, E1 forged-order-total): an attacker
        // controls the `total`/`subtotal` fields in the JSON they hand to
        // apply_event_logic. The kernel must recompute them from the items, never
        // trust the wire value.
        let json = place_order_logic(Some("c1".into()), SAMPLE_ITEMS, Some("web".into()))
            .expect("place_order_logic ok");

        let forged = 9_999_999i64;
        let tampered = forge_totals(&json, forged);

        let updated = apply_event_logic(&tampered, "CONFIRMED").expect("apply_event_logic ok");
        let out = parse(&updated).unwrap();

        // True total = 2*500 + 300 = 1300 (provisional, no tax/fee folded).
        let expected = 2 * 500 + 300;
        assert_ne!(out.get("total").and_then(Value::as_i64), Some(forged), "forged total must NOT survive the fold");
        assert_eq!(out.get("total").and_then(Value::as_i64), Some(expected), "total must be recomputed from items");
        assert_ne!(out.get("subtotal").and_then(Value::as_i64), Some(forged), "forged subtotal must NOT survive");
        assert_eq!(out.get("subtotal").and_then(Value::as_i64), Some(expected));
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
        let v = parse(&confirmed).unwrap();
        assert_eq!(v.get("status").and_then(Value::as_str), Some("CONFIRMED"));

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
        let v = parse(&p).unwrap();
        assert_eq!(v.get("status").and_then(Value::as_str), Some("PREPARING"));
        assert_eq!(v.get("channel").and_then(Value::as_str), Some("app"));
        assert_eq!(v.get("customer_id").and_then(Value::as_str), Some("c9"));
    }
}
