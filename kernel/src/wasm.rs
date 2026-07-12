//! wasm-bindgen glue exposing the kernel to the web as JS-callable JSON functions.
//!
//! Every function takes and returns plain JSON `String`s (or a `u64`), so the
//! web layer never has to deal with Rust/WASM struct layout. The kernel's
//! domain types (`Order`, `OrderItem`, `ChannelEvent`, `OrderStatus`) do NOT
//! derive `serde` (they live in `domain.rs`/`analytics.rs` which must stay
//! untouched), so this module defines its own serde shapes for the wire format
//! and maps them onto the canonical kernel types.
//!
//! Design note: the `#[wasm_bindgen]` entry points are thin wrappers that call
//! into plain `Result<String, String>` logic functions. Keeping the logic free
//! of `JsValue` means the JSON round-trips are unit-testable on the native host
//! target (where there is no JS engine), as well as under wasm.
//!
//! Exposed surface:
//!   * `place_order_js(customer_id, items_json, channel) -> Order JSON`
//!   * `apply_event_js(order_json, next_status) -> updated Order JSON (or error str)`
//!   * `channel_ledger_js(events_json) -> {orders_by_channel, funnel, anomalies} JSON`
//!   * `reduce_anomalies_js(events_json) -> anomaly count (u64)`
//!
//! Pure std + serde_json only. No float on money. No courier scoring anywhere.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::analytics::{reduce_anomalies, ChannelEvent, ChannelLedger};
use crate::domain::{apply_event, place_order, Order, OrderItem};
use crate::order_machine::{OrderStatus, TransitionError};

/// Monotonic id / timestamp source for `place_order_js` (the JS signature does
/// not supply an `id` or `created_at_ms`). Deterministic and order-preserving.
static ORDER_SEQ: AtomicU64 = AtomicU64::new(0);

// ── Wire (JSON) shapes — these are the only structs serde touches ──

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

#[derive(Deserialize)]
struct EventIn {
    order_id: String,
    channel: String,
    status: String,
    at_ms: i64,
}

#[derive(Serialize)]
struct LedgerOut {
    /// channel -> distinct order count
    orders_by_channel: Vec<(String, u64)>,
    /// channel -> [[status, count], ...] funnel stages in enum order
    funnel: HashMap<String, Vec<(String, u64)>>,
    /// number of orders whose status sequence contained an illegal transition
    anomalies: u64,
}

// ── Internal mapping helpers (free of wasm_bindgen so they are unit-testable
//    on the native target and reused by every exported function) ──

fn item_to_domain(i: ItemInput) -> OrderItem {
    OrderItem {
        product_id: i.product_id,
        modifier_ids: i.modifier_ids,
        quantity: i.quantity,
        unit_price: i.unit_price,
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
    let items = o.items.into_iter().map(item_to_domain).collect();
    Ok(Order {
        id: o.id,
        customer_id: o.customer_id,
        status,
        items,
        subtotal: o.subtotal,
        total: o.total,
        created_at_ms: o.created_at_ms,
        channel: o.channel,
        cash_pay_with: o.cash_pay_with,
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

// ── Pure logic (testable on native host) ──

fn place_order_logic(
    customer_id: Option<String>,
    items_json: &str,
    channel: Option<String>,
) -> Result<String, String> {
    let items_in: Vec<ItemInput> = serde_json::from_str(items_json).map_err(|e| e.to_string())?;
    let items: Vec<OrderItem> = items_in.into_iter().map(item_to_domain).collect();

    let seq = ORDER_SEQ.fetch_add(1, Ordering::SeqCst);
    let id = format!("ord_{}", seq);
    let created_at_ms = seq as i64;

    let order = place_order(
        id,
        customer_id,
        items,
        created_at_ms,
        channel,
        None, // cash_pay_with is not part of the JS placement surface
    )
    .map_err(status_err)?;

    let out = order_to_out(&order);
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

fn apply_event_logic(order_json: &str, next_status: &str) -> Result<String, String> {
    let parsed: OrderIn = serde_json::from_str(order_json).map_err(|e| e.to_string())?;
    let order = order_from_in(parsed)?;

    let next = OrderStatus::from_str(next_status)
        .ok_or_else(|| format!("unknown OrderStatus: {}", next_status))?;

    let updated = apply_event(&order, next).map_err(status_err)?;
    let out = order_to_out(&updated);
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

fn channel_ledger_logic(events_json: &str) -> Result<String, String> {
    let events: Vec<EventIn> = serde_json::from_str(events_json).map_err(|e| e.to_string())?;

    let mut ledger = ChannelLedger::new();
    // Owned copy of the (order_id, status, at_ms) stream for the anomaly reducer.
    let mut reduce_input: Vec<(String, OrderStatus, i64)> = Vec::with_capacity(events.len());

    for e in events {
        let status = OrderStatus::from_str(&e.status)
            .ok_or_else(|| format!("unknown OrderStatus: {}", e.status))?;
        // ChannelEvent borrows `&'static str`, so leak the owned strings into
        // static storage for the lifetime of this call. This is the documented
        // boundary tradeoff for reusing the unmodified `ChannelEvent` type.
        let oid: &'static str = Box::leak(e.order_id.clone().into_boxed_str());
        let ch: &'static str = Box::leak(e.channel.clone().into_boxed_str());
        ledger.ingest(ChannelEvent {
            order_id: oid,
            channel: ch,
            status,
            at_ms: e.at_ms,
        });
        reduce_input.push((e.order_id, status, e.at_ms));
    }

    let by_channel = ledger.orders_by_channel();
    let mut funnel: HashMap<String, Vec<(String, u64)>> = HashMap::new();
    for (channel, _count) in &by_channel {
        let stages = ledger
            .funnel(channel)
            .into_iter()
            .map(|(s, c)| (s.as_str().to_string(), c))
            .collect();
        funnel.insert(channel.clone(), stages);
    }

    let out = LedgerOut {
        orders_by_channel: by_channel,
        funnel,
        anomalies: reduce_anomalies(&reduce_input),
    };
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

fn reduce_anomalies_logic(events_json: &str) -> Result<u64, String> {
    let events: Vec<EventIn> = serde_json::from_str(events_json).map_err(|e| e.to_string())?;
    let mut input: Vec<(String, OrderStatus, i64)> = Vec::with_capacity(events.len());
    for e in events {
        let status = OrderStatus::from_str(&e.status)
            .ok_or_else(|| format!("unknown OrderStatus: {}", e.status))?;
        input.push((e.order_id, status, e.at_ms));
    }
    Ok(reduce_anomalies(&input))
}

// ── Exported JS-callable functions (thin JsValue wrappers) ──

/// Create a new `Pending` order from a JSON item list.
///
/// `items_json` is a JSON array of
/// `{product_id, modifier_ids: [], quantity: i64, unit_price: i64}`.
/// Returns the created `Order` serialized to JSON.
#[wasm_bindgen]
pub fn place_order_js(
    customer_id: Option<String>,
    items_json: String,
    channel: Option<String>,
) -> Result<String, JsValue> {
    place_order_logic(customer_id, &items_json, channel).map_err(|e| JsValue::from_str(&e))
}

/// Advance an order one step. `next_status` is the status name (e.g. "CONFIRMED").
/// Returns the updated order JSON, or a `JsValue` error string on an illegal
/// transition (same status / illegal edge / scaffold disabled).
#[wasm_bindgen]
pub fn apply_event_js(order_json: String, next_status: String) -> Result<String, JsValue> {
    apply_event_logic(&order_json, &next_status).map_err(|e| JsValue::from_str(&e))
}

/// Ingest a batch of channel events and return aggregated attribution + anomaly
/// counts as JSON: `{orders_by_channel: [[channel,count]...], funnel: {channel:
/// [[status,count]...]}, anomalies: u64}`.
///
/// `events_json` is an array of `{order_id, channel, status, at_ms}`.
/// `status` is the status name string.
#[wasm_bindgen]
pub fn channel_ledger_js(events_json: String) -> Result<String, JsValue> {
    channel_ledger_logic(&events_json).map_err(|e| JsValue::from_str(&e))
}

/// Reduce a raw `(order_id, status, at_ms)` event stream to an anomaly count
/// (`u64`). `events_json` is an array of `{order_id, channel, status, at_ms}`
/// (the `channel` field is accepted but ignored by the reducer).
#[wasm_bindgen]
pub fn reduce_anomalies_js(events_json: String) -> Result<u64, JsValue> {
    reduce_anomalies_logic(&events_json).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ITEMS: &str = r#"[
        {"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":500},
        {"product_id":"p2","modifier_ids":[],"quantity":1,"unit_price":300}
    ]"#;

    #[test]
    fn place_order_round_trip() {
        let json = place_order_logic(Some("c1".into()), SAMPLE_ITEMS, Some("web".into()))
            .expect("place_order_logic ok");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["status"], "PENDING");
        assert_eq!(v["subtotal"], 2 * 500 + 300);
        assert_eq!(v["total"], 2 * 500 + 300); // provisional until tax/fee
        assert_eq!(v["customer_id"], "c1");
        assert_eq!(v["channel"], "web");
        assert_eq!(v["items"].as_array().unwrap().len(), 2);
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
    fn channel_ledger_output_shape() {
        let events = r#"[
            {"order_id":"a1","channel":"tiktok","status":"PENDING","at_ms":1},
            {"order_id":"a2","channel":"tiktok","status":"DELIVERED","at_ms":2},
            {"order_id":"b1","channel":"ig","status":"REJECTED","at_ms":3}
        ]"#;
        let json = channel_ledger_logic(events).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        // orders_by_channel sums: tiktok=2, ig=1
        let obc = &v["orders_by_channel"];
        assert_eq!(obc[0][0], "tiktok");
        assert_eq!(obc[0][1], 2);
        assert_eq!(obc[1][0], "ig");
        assert_eq!(obc[1][1], 1);
        // funnel keyed by channel, each a list of [status, count]
        let tk = &v["funnel"]["tiktok"];
        assert_eq!(tk[0][0], "PENDING"); // Pending stage first in enum order
        assert_eq!(tk[0][1], 1);
        // anomalies: all three orders are single observations with no transition
        // steps, so the reducer finds zero illegal sequences.
        assert_eq!(v["anomalies"], 0);
    }

    #[test]
    fn reduce_anomalies_direct() {
        let events = r#"[
            {"order_id":"o1","channel":"x","status":"PENDING","at_ms":1},
            {"order_id":"o1","channel":"x","status":"CONFIRMED","at_ms":2},
            {"order_id":"o2","channel":"x","status":"PENDING","at_ms":1},
            {"order_id":"o2","channel":"x","status":"DELIVERED","at_ms":2}
        ]"#;
        let n = reduce_anomalies_logic(events).unwrap();
        assert_eq!(n, 1); // o2 is the single illegal sequence
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
