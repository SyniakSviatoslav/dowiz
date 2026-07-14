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
use crate::money::{estimate_order_total, FeeConfig, OrderTotalConfig};
use crate::order_machine::{fsm_graph_report, OrderStatus, TransitionError};

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

// ── Money: order-total mirror (RW-03) ────────────────────────────────────────
// 1:1 port of packages/ui/src/lib/money.ts. The SERVER (apps/api orders.ts fee
// ladder) stays authoritative for what is CHARGED; this mirror drives what the
// client SEES. All amounts are integer minor units.

#[derive(Deserialize)]
struct FeeConfigIn {
    is_pickup: bool,
    #[serde(default)]
    free_delivery_threshold: Option<i64>,
    #[serde(default)]
    delivery_fee_flat: Option<i64>,
    #[serde(default)]
    has_distance_tiers: bool,
}

#[derive(Deserialize)]
struct OrderTotalConfigIn {
    is_pickup: bool,
    #[serde(default)]
    free_delivery_threshold: Option<i64>,
    #[serde(default)]
    delivery_fee_flat: Option<i64>,
    #[serde(default)]
    has_distance_tiers: bool,
    tax_rate: f64,
    price_includes_tax: bool,
    #[serde(default)]
    min_order_value: Option<i64>,
}

#[derive(Serialize)]
struct EstimateOut {
    fee_known: bool,
    delivery_fee: Option<i64>,
    tax_total: i64,
    total: Option<i64>,
    min_not_met: bool,
}

fn order_total_cfg_from_in(c: &OrderTotalConfigIn) -> OrderTotalConfig {
    OrderTotalConfig {
        fee: FeeConfig {
            is_pickup: c.is_pickup,
            free_delivery_threshold: c.free_delivery_threshold,
            delivery_fee_flat: c.delivery_fee_flat,
            has_distance_tiers: c.has_distance_tiers,
        },
        tax_rate: c.tax_rate,
        price_includes_tax: c.price_includes_tax,
        min_order_value: c.min_order_value,
    }
}

/// Compute the client-side order-total estimate.
/// `subtotal` and `fee` fields are integer minor units. `cfg_json` is a JSON
/// object with the fee/tax/min-order fields (see `OrderTotalConfigIn`).
/// Returns `{fee_known, delivery_fee, tax_total, total, min_not_met}` JSON.
fn estimate_order_total_logic(subtotal: i64, cfg_json: &str) -> Result<String, String> {
    let cfg_in: OrderTotalConfigIn = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
    let cfg = order_total_cfg_from_in(&cfg_in);
    let est = estimate_order_total(subtotal, &cfg);
    let out = EstimateOut {
        fee_known: est.fee_known,
        delivery_fee: est.delivery_fee,
        tax_total: est.tax_total,
        total: est.total,
        min_not_met: est.min_not_met,
    };
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn estimate_order_total_js(subtotal: i64, cfg_json: String) -> Result<String, JsValue> {
    estimate_order_total_logic(subtotal, &cfg_json).map_err(|e| JsValue::from_str(&e))
}

/// Structural signature of the order-lifecycle FSM as JSON
/// (`{vertices,edges,is_acyclic,cyclomatic,spectral_radius,reachable_from_pending,
/// reachable_states,topological_len}`). Drift-telemetry surface: a silent change
/// to the transition table shifts a field and trips a regression gate.
///
/// Pure host-testable logic in `fsm_graph_report` (no `JsValue`); this is a thin
/// wrapper.
fn fsm_graph_report_logic() -> Result<String, String> {
    Ok(fsm_graph_report().to_json())
}

#[wasm_bindgen]
pub fn fsm_graph_report_js() -> Result<String, JsValue> {
    fsm_graph_report_logic().map_err(|e| JsValue::from_str(&e))
}

// ── Geo / route kinematics surface (RW-06) ──
// Exposes the kernel `geo` math to JS so the field-sim hook calls the kernel
// for every math step instead of keeping a parallel TS copy (`geo-anim.js`).
// Plain numeric / JSON in-out, matching the control-plane wrapper style.

#[derive(Serialize)]
struct PosOut {
    lat: f64,
    lng: f64,
}

#[derive(Serialize)]
struct ProgressOut {
    remaining_m: f64,
    snapped: PosOut,
    segment_index: usize,
}

fn parse_polyline(json: &str) -> Result<Vec<(f64, f64)>, String> {
    // Accepts [[lat,lng], ...] or [{"lat":..,"lng":..}, ...].
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("polyline must be an array")?;
    let mut out = Vec::with_capacity(arr.len());
    for p in arr {
        if let Some(a) = p.as_array() {
            let lat = a.get(0).and_then(|x| x.as_f64()).ok_or("bad lat")?;
            let lng = a.get(1).and_then(|x| x.as_f64()).ok_or("bad lng")?;
            out.push((lat, lng));
        } else {
            let lat = p.get("lat").and_then(|x| x.as_f64()).ok_or("bad lat")?;
            let lng = p.get("lng").and_then(|x| x.as_f64()).ok_or("bad lng")?;
            out.push((lat, lng));
        }
    }
    Ok(out)
}

fn parse_pos(json: &str) -> Result<(f64, f64), String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if let Some(a) = v.as_array() {
        Ok((
            a.get(0).and_then(|x| x.as_f64()).ok_or("bad lat")?,
            a.get(1).and_then(|x| x.as_f64()).ok_or("bad lng")?,
        ))
    } else {
        Ok((
            v.get("lat").and_then(|x| x.as_f64()).ok_or("bad lat")?,
            v.get("lng").and_then(|x| x.as_f64()).ok_or("bad lng")?,
        ))
    }
}

fn geo_haversine_logic(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, String> {
    Ok(crate::geo::haversine_meters(a_lat, a_lng, b_lat, b_lng).to_string())
}
#[wasm_bindgen]
pub fn geo_haversine_js(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, JsValue> {
    geo_haversine_logic(a_lat, a_lng, b_lat, b_lng).map_err(|e| JsValue::from_str(&e))
}

fn geo_lerp_logic(
    a_lat: f64,
    a_lng: f64,
    b_lat: f64,
    b_lng: f64,
    t: f64,
) -> Result<String, String> {
    let (lat, lng) = crate::geo::lerp_lat_lng(a_lat, a_lng, b_lat, b_lng, t);
    serde_json::to_string(&PosOut { lat, lng }).map_err(|e| e.to_string())
}
#[wasm_bindgen]
pub fn geo_lerp_js(
    a_lat: f64,
    a_lng: f64,
    b_lat: f64,
    b_lng: f64,
    t: f64,
) -> Result<String, JsValue> {
    geo_lerp_logic(a_lat, a_lng, b_lat, b_lng, t).map_err(|e| JsValue::from_str(&e))
}

fn geo_bearing_logic(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, String> {
    Ok(crate::geo::bearing_deg(a_lat, a_lng, b_lat, b_lng).to_string())
}
#[wasm_bindgen]
pub fn geo_bearing_js(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, JsValue> {
    geo_bearing_logic(a_lat, a_lng, b_lat, b_lng).map_err(|e| JsValue::from_str(&e))
}

fn geo_progress_logic(poly_json: &str, pos_lat: f64, pos_lng: f64) -> Result<String, String> {
    let poly = parse_polyline(poly_json)?;
    let r = crate::geo::progress_along_route(&poly, (pos_lat, pos_lng));
    let out = ProgressOut {
        remaining_m: r.remaining_m,
        snapped: PosOut {
            lat: r.snapped.0,
            lng: r.snapped.1,
        },
        segment_index: r.segment_index,
    };
    serde_json::to_string(&out).map_err(|e| e.to_string())
}
#[wasm_bindgen]
pub fn geo_progress_js(poly_json: String, pos_lat: f64, pos_lng: f64) -> Result<String, JsValue> {
    geo_progress_logic(&poly_json, pos_lat, pos_lng).map_err(|e| JsValue::from_str(&e))
}

fn geo_eta_logic(remaining_m: f64, total_m: f64, baseline_s: f64) -> Result<String, String> {
    Ok(crate::geo::eta_seconds(remaining_m, total_m, baseline_s).to_string())
}
#[wasm_bindgen]
pub fn geo_eta_js(remaining_m: f64, total_m: f64, baseline_s: f64) -> Result<String, JsValue> {
    geo_eta_logic(remaining_m, total_m, baseline_s).map_err(|e| JsValue::from_str(&e))
}

fn geo_should_snap_logic(
    prev_json: &str,
    next_json: &str,
    threshold_m: f64,
) -> Result<String, String> {
    let prev = parse_pos(prev_json)?;
    let next = parse_pos(next_json)?;
    Ok(crate::geo::should_snap(prev, next, threshold_m).to_string())
}
#[wasm_bindgen]
pub fn geo_should_snap_js(
    prev_json: String,
    next_json: String,
    threshold_m: f64,
) -> Result<String, JsValue> {
    geo_should_snap_logic(&prev_json, &next_json, threshold_m).map_err(|e| JsValue::from_str(&e))
}

fn geo_is_arriving_logic(remaining_m: f64, threshold_m: f64) -> Result<String, String> {
    Ok(crate::geo::is_arriving(remaining_m, threshold_m).to_string())
}
#[wasm_bindgen]
pub fn geo_is_arriving_js(remaining_m: f64, threshold_m: f64) -> Result<String, JsValue> {
    geo_is_arriving_logic(remaining_m, threshold_m).map_err(|e| JsValue::from_str(&e))
}

fn geo_point_in_polygon_logic(
    pt_lat: f64,
    pt_lng: f64,
    polygon_json: &str,
) -> Result<String, String> {
    let poly = parse_polyline(polygon_json)?;
    Ok(crate::geo::point_in_polygon(pt_lat, pt_lng, &poly).to_string())
}
#[wasm_bindgen]
pub fn geo_point_in_polygon_js(
    pt_lat: f64,
    pt_lng: f64,
    polygon_json: String,
) -> Result<String, JsValue> {
    geo_point_in_polygon_logic(pt_lat, pt_lng, &polygon_json).map_err(|e| JsValue::from_str(&e))
}

fn geo_is_out_of_order_logic(last_ts: i64, ts: i64) -> Result<String, String> {
    // JS sends -1 to mean "no previous timestamp" (None).
    let last = if last_ts < 0 { None } else { Some(last_ts) };
    Ok(crate::geo::is_out_of_order(last, ts).to_string())
}
#[wasm_bindgen]
pub fn geo_is_out_of_order_js(last_ts: i64, ts: i64) -> Result<String, JsValue> {
    geo_is_out_of_order_logic(last_ts, ts).map_err(|e| JsValue::from_str(&e))
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

    // ── RW-03: estimate_order_total_logic == packages/ui/src/lib/money.ts ──
    const CFG_FLAT_EXCL: &str = r#"{"is_pickup":false,"free_delivery_threshold":null,
        "delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":0.20,
        "price_includes_tax":false,"min_order_value":null}"#;
    const CFG_FREE_THR: &str = r#"{"is_pickup":false,"free_delivery_threshold":2000,
        "delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":0.10,
        "price_includes_tax":false,"min_order_value":null}"#;
    const CFG_PICKUP: &str = r#"{"is_pickup":true,"free_delivery_threshold":null,
        "delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":0.20,
        "price_includes_tax":false,"min_order_value":null}"#;
    const CFG_DISTANCE: &str = r#"{"is_pickup":false,"free_delivery_threshold":null,
        "delivery_fee_flat":200,"has_distance_tiers":true,"tax_rate":0.20,
        "price_includes_tax":false,"min_order_value":null}"#;
    const CFG_MIN: &str = r#"{"is_pickup":false,"free_delivery_threshold":null,
        "delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":0.20,
        "price_includes_tax":false,"min_order_value":500}"#;

    fn est(subtotal: i64, cfg: &str) -> serde_json::Value {
        let json = estimate_order_total_logic(subtotal, cfg).expect("estimate ok");
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn estimate_flat_exclusive() {
        let v = est(1000, CFG_FLAT_EXCL);
        assert_eq!(v["fee_known"], true);
        assert_eq!(v["delivery_fee"], 200);
        assert_eq!(v["tax_total"], 200);
        assert_eq!(v["total"], 1400);
    }
    #[test]
    fn estimate_free_threshold_boundary() {
        let v = est(2000, CFG_FREE_THR);
        assert_eq!(v["delivery_fee"], 0);
        assert_eq!(v["tax_total"], 200);
        assert_eq!(v["total"], 2200);
    }
    #[test]
    fn estimate_pickup() {
        let v = est(1500, CFG_PICKUP);
        assert_eq!(v["delivery_fee"], 0);
        assert_eq!(v["total"], 1500 + 300);
    }
    #[test]
    fn estimate_distance_unknown() {
        let v = est(1000, CFG_DISTANCE);
        assert_eq!(v["fee_known"], false);
        assert_eq!(v["delivery_fee"], serde_json::Value::Null);
        assert_eq!(v["total"], serde_json::Value::Null);
    }
    #[test]
    fn estimate_min_not_met() {
        let v = est(400, CFG_MIN);
        assert_eq!(v["min_not_met"], true);
        assert_eq!(v["total"], 400 + 200 + 80);
    }

    // ── GREEN: FSM graph-report surface emits a valid structural signature ──
    #[test]
    fn fsm_graph_report_js_shape() {
        let json = fsm_graph_report_logic().expect("report ok");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["vertices"], 10);
        assert_eq!(v["is_acyclic"], true);
        // Established structural facts (see order_machine green_cyclomatic_*
        // tests): 9 edges, 10 vertices, 2 undirected components (Scheduled is an
        // orphan) ⇒ μ = 9 − 10 + 2 = 1 (one undirected cycle), directed ρ = 0.
        assert_eq!(v["cyclomatic"], 1);
        assert_eq!(v["topological_len"], 10);
        // reachable_from_pending is a bitmask; Pending (bit 0) always set.
        assert_eq!(v["reachable_from_pending"].as_u64().unwrap() & 1, 1);
    }

    // ── G-geo: wasm geo surface matches kernel math (parity within tolerance) ──
    const POLY: &str = "[[40.0,-3.0],[40.01,-3.0],[40.02,-3.0]]"; // ~2×1° northward

    #[test]
    fn geo_haversine_js_parity() {
        let d: f64 = geo_haversine_logic(51.5074, -0.1278, 48.8566, 2.3522)
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            (d - 343_000.0).abs() < 2_000.0,
            "London→Paris ≈ 343km, got {d}"
        );
    }

    #[test]
    fn geo_lerp_js_parity() {
        let j = geo_lerp_logic(0.0, 0.0, 10.0, 20.0, 0.5).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert!((v["lat"].as_f64().unwrap() - 5.0).abs() < 1e-9);
        assert!((v["lng"].as_f64().unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn geo_bearing_js_parity() {
        let b: f64 = geo_bearing_logic(51.5074, -0.1278, 48.8566, 2.3522)
            .unwrap()
            .parse()
            .unwrap();
        assert!((b - 148.0).abs() < 3.0, "London→Paris ≈ 148°, got {b}");
    }

    #[test]
    fn geo_progress_js_parity() {
        let j = geo_progress_logic(POLY, 40.005, -3.0).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        // pos is midpoint of the first 0.01°-segment (≈1112 m), so it lands on
        // segment 0 (end-node index 1), snapped lat ≈ 40.005, remaining ≈ 0.75 of
        // the ~2224 m polyline ≈ 1668 m.
        assert_eq!(v["segment_index"], 1);
        assert!((v["snapped"]["lat"].as_f64().unwrap() - 40.005).abs() < 1e-6);
        let rem = v["remaining_m"].as_f64().unwrap();
        assert!(
            rem > 1500.0 && rem < 1800.0,
            "remaining ≈ 1668 m, got {rem}"
        );
    }

    #[test]
    fn geo_eta_js_parity() {
        let s: f64 = geo_eta_logic(1000.0, 2000.0, 400.0)
            .unwrap()
            .parse()
            .unwrap();
        assert!((s - 200.0).abs() < 1e-9, "eta 200s, got {s}");
        let fb: f64 = geo_eta_logic(1000.0, 0.0, 0.0).unwrap().parse().unwrap();
        assert!(
            (fb - 200.0).abs() < 1e-9,
            "eta fallback 5m/s → 200s, got {fb}"
        );
    }

    #[test]
    fn geo_should_snap_js_parity() {
        let t = "[0.0,0.0]";
        let n = "[0.000045,0.0]"; // ~5 m
        assert_eq!(geo_should_snap_logic(t, n, 500.0).unwrap(), "true");
        let far = "[0.01,0.0]"; // ~1.1 km
        assert_eq!(geo_should_snap_logic(t, far, 500.0).unwrap(), "false");
    }

    #[test]
    fn geo_is_arriving_js_parity() {
        assert_eq!(
            geo_is_arriving_logic(120.0, crate::geo::ARRIVE_THRESHOLD_M).unwrap(),
            "true"
        );
        assert_eq!(
            geo_is_arriving_logic(300.0, crate::geo::ARRIVE_THRESHOLD_M).unwrap(),
            "false"
        );
    }

    #[test]
    fn geo_point_in_polygon_js_parity() {
        let sq = "[[0.0,0.0],[0.0,10.0],[10.0,10.0],[10.0,0.0]]";
        assert_eq!(geo_point_in_polygon_logic(5.0, 5.0, sq).unwrap(), "true");
        assert_eq!(geo_point_in_polygon_logic(15.0, 5.0, sq).unwrap(), "false");
    }

    #[test]
    fn geo_is_out_of_order_js_parity() {
        assert_eq!(geo_is_out_of_order_logic(-1, 100).unwrap(), "false"); // first ping
        assert_eq!(geo_is_out_of_order_logic(100, 99).unwrap(), "true"); // older
        assert_eq!(geo_is_out_of_order_logic(100, 101).unwrap(), "false"); // newer
    }
}
