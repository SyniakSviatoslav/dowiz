//! wasm-bindgen glue exposing the kernel to the web as JS-callable JSON functions.
//!
//! This entire module is compiled ONLY under `#[cfg(feature = "wasm")]` (see the
//! inner attribute below and the gated `pub mod wasm;` in `lib.rs`). Native rlib
//! consumers build the kernel WITHOUT this module and therefore pull NONE of
//! wasm-bindgen / serde / serde_json / serde_yaml.
#![cfg(feature = "wasm")]
//!
//! Every function takes and returns plain JSON `String`s (or a `u64`), so the
//! web layer never has to deal with Rust/WASM struct layout.
//!
//! Design note: the `#[wasm_bindgen]` entry points are thin wrappers that call
//! into the no_std `dowiz_core::json_bridge` / `dowiz_core::json_api` logic
//! functions (migrated in waves 58–59, serde-free). The wrappers here only map
//! `String` errors onto `JsValue`; they hold no logic.

use wasm_bindgen::prelude::*;

// ── Order JSON authority lives in `dowiz_core::json_api` (P37 W37-1) ──

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
    crate::json_api::place_order_logic(customer_id, &items_json, channel)
        .map_err(|e| JsValue::from_str(&e))
}

/// Advance an order one step. `next_status` is the status name (e.g. "CONFIRMED").
/// Returns the updated order JSON, or a `JsValue` error string on an illegal
/// transition (same status / illegal edge / scaffold disabled).
#[wasm_bindgen]
pub fn apply_event_js(order_json: String, next_status: String) -> Result<String, JsValue> {
    crate::json_api::apply_event_logic(&order_json, &next_status).map_err(|e| JsValue::from_str(&e))
}

// ── Channel ledger / anomaly surface ──

/// Ingest a batch of channel events and return aggregated attribution + anomaly
/// counts as JSON: `{orders_by_channel: [[channel,count]...], funnel: {channel:
/// [[status,count]...]}, anomalies: u64}`.
///
/// `events_json` is an array of `{order_id, channel, status, at_ms}`.
#[wasm_bindgen]
pub fn channel_ledger_js(events_json: String) -> Result<String, JsValue> {
    crate::json_bridge::channel_ledger_logic(&events_json).map_err(|e| JsValue::from_str(&e))
}

/// Reduce a raw `(order_id, status, at_ms)` event stream to an anomaly count
/// (`u64`). `events_json` is an array of `{order_id, channel, status, at_ms}`
/// (the `channel` field is accepted but ignored by the reducer).
#[wasm_bindgen]
pub fn reduce_anomalies_js(events_json: String) -> Result<u64, JsValue> {
    crate::json_bridge::reduce_anomalies_logic(&events_json).map_err(|e| JsValue::from_str(&e))
}

/// Boot-time FSM drift gate (fail-closed) — mirrors [`crate::kernel_boot_verify_fsm`].
#[wasm_bindgen]
pub fn boot_verify_fsm_js() -> Result<String, JsValue> {
    match crate::kernel_boot_verify_fsm() {
        Ok(()) => Ok("OK".to_string()),
        Err(drift) => Err(JsValue::from_str(&format!("fsm boot drift: {drift}"))),
    }
}

// ── Money: order-total mirror (RW-03) ──

/// Compute the client-side order-total estimate.
/// `subtotal` and `fee` fields are integer minor units. `cfg_json` is a JSON
/// object with the fee/tax/min-order fields. Returns the estimate JSON.
#[wasm_bindgen]
pub fn estimate_order_total_js(subtotal: i64, cfg_json: String) -> Result<String, JsValue> {
    crate::json_bridge::estimate_order_total_logic(subtotal, &cfg_json)
        .map_err(|e| JsValue::from_str(&e))
}

/// Structural signature of the order-lifecycle FSM as JSON.
#[wasm_bindgen]
pub fn fsm_graph_report_js() -> Result<String, JsValue> {
    crate::json_bridge::fsm_graph_report_logic().map_err(|e| JsValue::from_str(&e))
}

// ── Geo / route kinematics surface (RW-06) ──

#[wasm_bindgen]
pub fn geo_haversine_js(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, JsValue> {
    crate::json_bridge::geo_haversine_logic(a_lat, a_lng, b_lat, b_lng)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_lerp_js(
    a_lat: f64,
    a_lng: f64,
    b_lat: f64,
    b_lng: f64,
    t: f64,
) -> Result<String, JsValue> {
    crate::json_bridge::geo_lerp_logic(a_lat, a_lng, b_lat, b_lng, t)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_bearing_js(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, JsValue> {
    crate::json_bridge::geo_bearing_logic(a_lat, a_lng, b_lat, b_lng)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_progress_js(poly_json: String, pos_lat: f64, pos_lng: f64) -> Result<String, JsValue> {
    crate::json_bridge::geo_progress_logic(&poly_json, pos_lat, pos_lng)
        .map_err(|e| JsValue::from_str(&e))
}

/// Flat bridge protocol for the engine (FE-06): `[remaining_m, snapped_lat,
/// snapped_lng, segment_index]` — no object keys, no serde on the engine side.
#[wasm_bindgen]
pub fn geo_progress_flat_js(
    poly_json: String,
    pos_lat: f64,
    pos_lng: f64,
) -> Result<String, JsValue> {
    crate::json_bridge::geo_progress_flat_logic(&poly_json, pos_lat, pos_lng)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_eta_js(remaining_m: f64, total_m: f64, baseline_s: f64) -> Result<String, JsValue> {
    crate::json_bridge::geo_eta_logic(remaining_m, total_m, baseline_s)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_should_snap_js(
    prev_json: String,
    next_json: String,
    threshold_m: f64,
) -> Result<String, JsValue> {
    crate::json_bridge::geo_should_snap_logic(&prev_json, &next_json, threshold_m)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_is_arriving_js(remaining_m: f64, threshold_m: f64) -> Result<String, JsValue> {
    crate::json_bridge::geo_is_arriving_logic(remaining_m, threshold_m)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_point_in_polygon_js(
    pt_lat: f64,
    pt_lng: f64,
    polygon_json: String,
) -> Result<String, JsValue> {
    crate::json_bridge::geo_point_in_polygon_logic(pt_lat, pt_lng, &polygon_json)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn geo_is_out_of_order_js(last_ts: i64, ts: i64) -> Result<String, JsValue> {
    crate::json_bridge::geo_is_out_of_order_logic(last_ts, ts).map_err(|e| JsValue::from_str(&e))
}

// ── Spectral-engine wasm surface (FE-07) ──

#[wasm_bindgen]
pub fn spectral_eigenvalues_js(matrix_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_eigenvalues_logic(&matrix_json).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn spectral_radius_js(matrix_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_radius_logic(&matrix_json).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn spectral_gap_js(matrix_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_gap_logic(&matrix_json).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn spectral_algebraic_connectivity_js(adjacency_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_algebraic_connectivity_logic(&adjacency_json)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn spectral_classify_drift_js(matrix_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_classify_drift_logic(&matrix_json)
        .map_err(|e| JsValue::from_str(&e))
}

/// Flat bridge protocol for the engine (FE-07, mirrors `geo_progress_flat_js`):
/// `[rho, gap, fiedler, drift_code, n, e1re, e1im, ...]`.
#[wasm_bindgen]
pub fn spectral_flat_js(matrix_json: String) -> Result<String, JsValue> {
    crate::json_bridge::spectral_flat_logic(&matrix_json).map_err(|e| JsValue::from_str(&e))
}

// ── Harmonic centrality (HK-05/HK-06) ──

/// Harmonic centrality H(v)=Σ 1/d(u,v) for every node `0..n` of an undirected
/// graph. `edges_json` is a JSON array of `[u, v]` pairs; `n` is the node count.
/// Returns a JSON array of length `n`.
#[wasm_bindgen]
pub fn harmonic_centrality_js(n: usize, edges_json: String) -> Result<String, JsValue> {
    crate::json_bridge::harmonic_centrality_logic(n, &edges_json)
        .map_err(|e| JsValue::from_str(&e))
}
