//! `dowiz_core::json_bridge` — the JSON bridges behind the wasm JS surface
//! (and reusable by any std host), now no_std and serde-free.
//!
//! Moved from `kernel/src/wasm.rs` (wave-59): the pure `*_logic` functions and
//! their parse/serialize helpers were the second-last `serde_json` consumer in
//! the kernel runtime graph. They now use the hand-rolled `crate::json`
//! parser/serializer. The `#[wasm_bindgen]` JS wrappers stay in the kernel and
//! call into these functions.
//!
//! JSON compatibility: outputs are *semantically* identical to the old
//! `serde_json` output (same shapes, same field sets). Two byte-level nuances
//! differ and are irrelevant to JSON consumers (`JSON.parse` and value-lookup):
//! (a) whole-number `f64` prints without a trailing `.0` (e.g. `5` not `5.0`),
//! (b) object key order is insertion order, not serde_json's BTreeMap-sorted
//! order. Both parse to identical values; no golden test pins these bytes.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::analytics::{reduce_anomalies, ChannelEvent, ChannelLedger};
use crate::harmonic::harmonic_centrality;
use crate::json::{parse, Value};
use crate::money::{estimate_order_total, FeeConfig, OrderTotalConfig};
use crate::order_machine::{fsm_graph_report, OrderStatus};
use crate::spectral::{
    algebraic_connectivity, classify_drift, eigenvalues, spectral_gap, spectral_radius, DriftClass,
};

// Resource caps on the untrusted-JSON trust boundary (round-2 gap-audit V3 1.1 / 1.7).
const MAX_CHANNEL_EVENTS: usize = 100_000;
const MAX_HARMONIC_NODES: usize = 50_000;

// ── Field extraction helpers (reproduce the old `#[derive(Deserialize)]` shapes) ──

fn field_str(v: &Value, key: &str) -> Result<String, String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing or non-string field `{}`", key))
}

fn field_i64(v: &Value, key: &str) -> Result<i64, String> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("missing or non-integer field `{}`", key))
}

fn field_f64(v: &Value, key: &str) -> Result<f64, String> {
    v.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing or non-number field `{}`", key))
}

fn field_bool(v: &Value, key: &str) -> Result<bool, String> {
    v.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing or non-bool field `{}`", key))
}

/// `Option<i64>` with `#[serde(default)]`: missing or `null` → `None`.
fn field_opt_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

/// `bool` with `#[serde(default)]`: missing → `false`.
fn field_bool_default(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

// ── Geo / spectral parse helpers (were `serde_json::Value` based) ──

fn parse_polyline(json: &str) -> Result<Vec<(f64, f64)>, String> {
    // Accepts [[lat,lng], ...] or [{"lat":..,"lng":..}, ...].
    let v = parse(json).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("polyline must be an array")?;
    let mut out = Vec::with_capacity(arr.len());
    for p in arr {
        if let Some(a) = p.as_array() {
            let lat = a.get(0).and_then(Value::as_f64).ok_or("bad lat")?;
            let lng = a.get(1).and_then(Value::as_f64).ok_or("bad lng")?;
            out.push((lat, lng));
        } else {
            let lat = p.get("lat").and_then(Value::as_f64).ok_or("bad lat")?;
            let lng = p.get("lng").and_then(Value::as_f64).ok_or("bad lng")?;
            out.push((lat, lng));
        }
    }
    Ok(out)
}

fn parse_pos(json: &str) -> Result<(f64, f64), String> {
    let v = parse(json).map_err(|e| e.to_string())?;
    if let Some(a) = v.as_array() {
        Ok((
            a.get(0).and_then(Value::as_f64).ok_or("bad lat")?,
            a.get(1).and_then(Value::as_f64).ok_or("bad lng")?,
        ))
    } else {
        Ok((
            v.get("lat").and_then(Value::as_f64).ok_or("bad lat")?,
            v.get("lng").and_then(Value::as_f64).ok_or("bad lng")?,
        ))
    }
}

fn parse_matrix(json: &str) -> Result<Vec<Vec<f64>>, String> {
    let v = parse(json).map_err(|e| format!("bad JSON: {e}"))?;
    let rows = v
        .as_array()
        .ok_or_else(|| "matrix must be a JSON array of rows".to_string())?;
    if rows.is_empty() {
        return Err("empty matrix".to_string());
    }
    let mut m = Vec::with_capacity(rows.len());
    let cols = rows[0]
        .as_array()
        .ok_or_else(|| "each row must be a JSON array".to_string())?
        .len();
    for (i, r) in rows.iter().enumerate() {
        let ra = r
            .as_array()
            .ok_or_else(|| format!("row {i} is not a JSON array"))?;
        if ra.len() != cols {
            return Err(format!("row {i} has {} cols, expected {cols}", ra.len()));
        }
        let row: Vec<f64> = ra
            .iter()
            .map(|x| x.as_f64().ok_or_else(|| format!("row {i} has a non-number")))
            .collect::<Result<_, _>>()?;
        m.push(row);
    }
    if m.len() != cols {
        return Err(format!("matrix is {}x{}, expected square", m.len(), cols));
    }
    Ok(m)
}

// ── Channel ledger / anomaly reduction ──

/// Parse one `{order_id, channel, status, at_ms}` event object.
fn parse_event(v: &Value) -> Result<(String, String, OrderStatus, i64), String> {
    let order_id = field_str(v, "order_id")?;
    let channel = field_str(v, "channel")?;
    let status_str = field_str(v, "status")?;
    let status = OrderStatus::from_str(&status_str)
        .ok_or_else(|| format!("unknown OrderStatus: {}", status_str))?;
    let at_ms = field_i64(v, "at_ms")?;
    Ok((order_id, channel, status, at_ms))
}

pub fn channel_ledger_logic(events_json: &str) -> Result<String, String> {
    let v = parse(events_json).map_err(|e| e.to_string())?;
    let events = v.as_array().ok_or("events must be a JSON array")?;
    // V3 1.1 (ROUND-2 GAP-AUDIT): cap the input before the per-event `Box::leak`.
    if events.len() > MAX_CHANNEL_EVENTS {
        return Err(format!(
            "channel_ledger: too many events ({} > {})",
            events.len(),
            MAX_CHANNEL_EVENTS
        ));
    }

    let mut ledger = ChannelLedger::new();
    let mut reduce_input: Vec<(String, OrderStatus, i64)> = Vec::with_capacity(events.len());

    for ev in events {
        let (order_id, channel, status, at_ms) = parse_event(ev)?;
        // ChannelEvent borrows `&'static str`, so leak the owned strings into
        // static storage for the lifetime of this call (documented boundary tradeoff).
        let oid: &'static str = Box::leak(order_id.clone().into_boxed_str());
        let ch: &'static str = Box::leak(channel.clone().into_boxed_str());
        ledger.ingest(ChannelEvent {
            order_id: oid,
            channel: ch,
            status,
            at_ms,
        });
        reduce_input.push((order_id, status, at_ms));
    }

    let by_channel = ledger.orders_by_channel();
    let mut funnel: BTreeMap<String, Vec<(String, u64)>> = BTreeMap::new();
    for (channel, _count) in &by_channel {
        let stages = ledger
            .funnel(channel)
            .into_iter()
            .map(|(s, c)| (s.as_str().to_string(), c))
            .collect();
        funnel.insert(channel.clone(), stages);
    }

    let orders_by_channel: Vec<Value> = by_channel
        .into_iter()
        .map(|(ch, c)| Value::Array(vec![Value::from(ch), Value::from(c as i64)]))
        .collect();
    let funnel_v: Vec<(String, Value)> = funnel
        .into_iter()
        .map(|(ch, stages)| {
            let stages_v: Vec<Value> = stages
                .into_iter()
                .map(|(s, c)| Value::Array(vec![Value::from(s), Value::from(c as i64)]))
                .collect();
            (ch, Value::Array(stages_v))
        })
        .collect();

    let out = Value::Object(vec![
        (String::from("orders_by_channel"), Value::Array(orders_by_channel)),
        (String::from("funnel"), Value::Object(funnel_v)),
        (String::from("anomalies"), Value::from(reduce_anomalies(&reduce_input) as i64)),
    ]);
    Ok(out.to_string())
}

pub fn reduce_anomalies_logic(events_json: &str) -> Result<u64, String> {
    let v = parse(events_json).map_err(|e| e.to_string())?;
    let events = v.as_array().ok_or("events must be a JSON array")?;
    let mut input: Vec<(String, OrderStatus, i64)> = Vec::with_capacity(events.len());
    for ev in events {
        let (order_id, _channel, status, at_ms) = parse_event(ev)?;
        input.push((order_id, status, at_ms));
    }
    Ok(reduce_anomalies(&input))
}

// ── Money: order-total mirror (RW-03) ──

pub fn estimate_order_total_logic(subtotal: i64, cfg_json: &str) -> Result<String, String> {
    let v = parse(cfg_json).map_err(|e| e.to_string())?;
    let cfg = OrderTotalConfig {
        fee: FeeConfig {
            is_pickup: field_bool(&v, "is_pickup")?,
            free_delivery_threshold: field_opt_i64(&v, "free_delivery_threshold"),
            delivery_fee_flat: field_opt_i64(&v, "delivery_fee_flat"),
            has_distance_tiers: field_bool_default(&v, "has_distance_tiers"),
        },
        tax_rate: field_f64(&v, "tax_rate")?,
        price_includes_tax: field_bool(&v, "price_includes_tax")?,
        min_order_value: field_opt_i64(&v, "min_order_value"),
    };
    let est = estimate_order_total(subtotal, &cfg);
    let out = Value::Object(vec![
        (String::from("fee_known"), Value::from(est.fee_known)),
        (String::from("delivery_fee"), Value::from(est.delivery_fee)),
        (String::from("tax_total"), Value::from(est.tax_total)),
        (String::from("total"), Value::from(est.total)),
        (String::from("min_not_met"), Value::from(est.min_not_met)),
    ]);
    Ok(out.to_string())
}

// ── FSM graph report (already serde-free) ──

pub fn fsm_graph_report_logic() -> Result<String, String> {
    Ok(fsm_graph_report().to_json())
}

// ── Geo / route kinematics surface (RW-06) ──

pub fn geo_haversine_logic(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, String> {
    Ok(crate::geo::haversine_meters(a_lat, a_lng, b_lat, b_lng).to_string())
}

pub fn geo_lerp_logic(
    a_lat: f64,
    a_lng: f64,
    b_lat: f64,
    b_lng: f64,
    t: f64,
) -> Result<String, String> {
    let (lat, lng) = crate::geo::lerp_lat_lng(a_lat, a_lng, b_lat, b_lng, t);
    Ok(pos_to_value(lat, lng).to_string())
}

fn pos_to_value(lat: f64, lng: f64) -> Value {
    Value::Object(vec![
        (String::from("lat"), Value::from(lat)),
        (String::from("lng"), Value::from(lng)),
    ])
}

pub fn geo_bearing_logic(a_lat: f64, a_lng: f64, b_lat: f64, b_lng: f64) -> Result<String, String> {
    Ok(crate::geo::bearing_deg(a_lat, a_lng, b_lat, b_lng).to_string())
}

pub fn geo_progress_logic(poly_json: &str, pos_lat: f64, pos_lng: f64) -> Result<String, String> {
    let poly = parse_polyline(poly_json)?;
    let r = crate::geo::progress_along_route(&poly, (pos_lat, pos_lng));
    let out = Value::Object(vec![
        (String::from("remaining_m"), Value::from(r.remaining_m)),
        (String::from("snapped"), pos_to_value(r.snapped.0, r.snapped.1)),
        (String::from("segment_index"), Value::from(r.segment_index as i64)),
    ]);
    Ok(out.to_string())
}

pub fn geo_progress_flat_logic(poly_json: &str, pos_lat: f64, pos_lng: f64) -> Result<String, String> {
    let poly = parse_polyline(poly_json)?;
    let r = crate::geo::progress_along_route(&poly, (pos_lat, pos_lng));
    Ok(format!(
        "[{},{},{},{}]",
        r.remaining_m, r.snapped.0, r.snapped.1, r.segment_index
    ))
}

pub fn geo_eta_logic(remaining_m: f64, total_m: f64, baseline_s: f64) -> Result<String, String> {
    Ok(crate::geo::eta_seconds(remaining_m, total_m, baseline_s).to_string())
}

pub fn geo_should_snap_logic(
    prev_json: &str,
    next_json: &str,
    threshold_m: f64,
) -> Result<String, String> {
    let prev = parse_pos(prev_json)?;
    let next = parse_pos(next_json)?;
    Ok(crate::geo::should_snap(prev, next, threshold_m).to_string())
}

pub fn geo_is_arriving_logic(remaining_m: f64, threshold_m: f64) -> Result<String, String> {
    Ok(crate::geo::is_arriving(remaining_m, threshold_m).to_string())
}

pub fn geo_point_in_polygon_logic(
    pt_lat: f64,
    pt_lng: f64,
    polygon_json: &str,
) -> Result<String, String> {
    let poly = parse_polyline(polygon_json)?;
    Ok(crate::geo::point_in_polygon(pt_lat, pt_lng, &poly).to_string())
}

pub fn geo_is_out_of_order_logic(last_ts: i64, ts: i64) -> Result<String, String> {
    // JS sends -1 to mean "no previous timestamp" (None).
    let last = if last_ts < 0 { None } else { Some(last_ts) };
    Ok(crate::geo::is_out_of_order(last, ts).to_string())
}

// ── Spectral-engine wasm surface (FE-07) ──

pub fn spectral_eigenvalues_logic(matrix_json: &str) -> Result<String, String> {
    let m = parse_matrix(matrix_json)?;
    let eigs = eigenvalues(&m);
    let out: Vec<Value> = eigs
        .iter()
        .map(|e| {
            Value::Object(vec![
                (String::from("re"), Value::from(e.re)),
                (String::from("im"), Value::from(e.im)),
            ])
        })
        .collect();
    Ok(Value::Array(out).to_string())
}

pub fn spectral_radius_logic(matrix_json: &str) -> Result<String, String> {
    Ok(spectral_radius(&parse_matrix(matrix_json)?).to_string())
}

pub fn spectral_gap_logic(matrix_json: &str) -> Result<String, String> {
    Ok(spectral_gap(&parse_matrix(matrix_json)?).to_string())
}

pub fn spectral_algebraic_connectivity_logic(adjacency_json: &str) -> Result<String, String> {
    Ok(algebraic_connectivity(&parse_matrix(adjacency_json)?).to_string())
}

pub fn spectral_classify_drift_logic(matrix_json: &str) -> Result<String, String> {
    Ok(match classify_drift(&parse_matrix(matrix_json)?) {
        DriftClass::Damped => "Damped",
        DriftClass::Resonant => "Resonant",
        DriftClass::Unstable => "Unstable",
    }
    .to_string())
}

pub fn spectral_flat_logic(matrix_json: &str) -> Result<String, String> {
    let m = parse_matrix(matrix_json)?;
    let mut eigs = eigenvalues(&m);
    // sort by descending modulus so the engine sees the dominant modes first.
    eigs.sort_by(|a, b| {
        b.abs()
            .partial_cmp(&a.abs())
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut out = vec![
        spectral_radius(&m),
        spectral_gap(&m),
        algebraic_connectivity(&m),
        classify_drift(&m).wire_code() as f64,
        eigs.len() as f64,
    ];
    for e in &eigs {
        out.push(e.re);
        out.push(e.im);
    }
    Ok(format!(
        "[{}]",
        out.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    ))
}

// ── Harmonic centrality (HK-05/HK-06) ──

pub fn harmonic_centrality_logic(n: usize, edges_json: &str) -> Result<String, String> {
    // V3 1.7 (ROUND-2 GAP-AUDIT): `n` is attacker-controlled; cap before alloc.
    if n == 0 || n > MAX_HARMONIC_NODES {
        return Err(format!(
            "harmonic_centrality: n out of range (0 < n <= {})",
            MAX_HARMONIC_NODES
        ));
    }
    let v = parse(edges_json).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("edges must be a JSON array")?;
    let edges: Vec<(usize, usize)> = arr
        .iter()
        .map(|p| {
            let a = p
                .as_array()
                .and_then(|a| a.get(0))
                .and_then(Value::as_i64)
                .ok_or("edge must be a [u,v] pair")?;
            let b = p
                .as_array()
                .and_then(|a| a.get(1))
                .and_then(Value::as_i64)
                .ok_or("edge must be a [u,v] pair")?;
            Ok((a as usize, b as usize))
        })
        .collect::<Result<_, String>>()?;
    let out = harmonic_centrality(n, &edges);
    let vv: Vec<Value> = out.iter().map(|x| Value::from(*x)).collect();
    Ok(Value::Array(vv).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_ledger_output_shape() {
        let events = r#"[
            {"order_id":"a1","channel":"tiktok","status":"PENDING","at_ms":1},
            {"order_id":"a2","channel":"tiktok","status":"DELIVERED","at_ms":2},
            {"order_id":"b1","channel":"ig","status":"REJECTED","at_ms":3}
        ]"#;
        let json = channel_ledger_logic(events).unwrap();
        let v = parse(&json).unwrap();
        let obc = v.get("orders_by_channel").and_then(Value::as_array).unwrap();
        assert_eq!(obc[0].as_array().unwrap()[0].as_str(), Some("tiktok"));
        assert_eq!(obc[0].as_array().unwrap()[1].as_i64(), Some(2));
        assert_eq!(obc[1].as_array().unwrap()[0].as_str(), Some("ig"));
        assert_eq!(obc[1].as_array().unwrap()[1].as_i64(), Some(1));
        let tk = v.get("funnel").and_then(|x| x.get("tiktok")).and_then(Value::as_array).unwrap();
        assert_eq!(tk[0].as_array().unwrap()[0].as_str(), Some("PENDING"));
        assert_eq!(tk[0].as_array().unwrap()[1].as_i64(), Some(1));
        assert_eq!(v.get("anomalies").and_then(Value::as_i64), Some(0));
    }

    #[test]
    fn channel_ledger_funnel_serialization_is_deterministic() {
        let events = r#"[
            {"order_id":"z1","channel":"zebra","status":"PENDING","at_ms":1},
            {"order_id":"a1","channel":"alpha","status":"PENDING","at_ms":2},
            {"order_id":"m1","channel":"mango","status":"PENDING","at_ms":3},
            {"order_id":"b1","channel":"beta","status":"PENDING","at_ms":4}
        ]"#;
        let a = channel_ledger_logic(events).unwrap();
        let b = channel_ledger_logic(events).unwrap();
        assert_eq!(a, b, "funnel JSON must be byte-identical across runs");
        let funnel_seg = a.split("\"funnel\":").nth(1).expect("funnel present");
        let positions: Vec<usize> = ["alpha", "beta", "mango", "zebra"]
            .iter()
            .map(|k| funnel_seg.find(&format!("\"{k}\"")).unwrap())
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "funnel keys must serialize in sorted order"
        );
    }

    #[test]
    fn channel_ledger_rejects_oversized_event_feed() {
        let mut events = String::from("[");
        for i in 0..(MAX_CHANNEL_EVENTS + 10) {
            if i > 0 {
                events.push(',');
            }
            events.push_str(&format!(
                "{{\"order_id\":\"o{i}\",\"channel\":\"c\",\"status\":\"PENDING\",\"at_ms\":{i}}}"
            ));
        }
        events.push(']');
        let res = channel_ledger_logic(&events);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("too many events"));
    }

    #[test]
    fn harmonic_centrality_rejects_out_of_range_n() {
        assert!(harmonic_centrality_logic(0, "[]").is_err());
        let huge = harmonic_centrality_logic(MAX_HARMONIC_NODES + 1, "[]");
        assert!(huge.is_err());
        assert!(huge.unwrap_err().contains("out of range"));
        let ok = harmonic_centrality_logic(3, "[[0,1],[1,2]]").unwrap();
        let v = parse(&ok).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
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

    fn est(subtotal: i64, cfg: &str) -> Value {
        let json = estimate_order_total_logic(subtotal, cfg).expect("estimate ok");
        parse(&json).unwrap()
    }

    #[test]
    fn estimate_flat_exclusive() {
        let v = est(1000, CFG_FLAT_EXCL);
        assert_eq!(v.get("fee_known").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("delivery_fee").and_then(Value::as_i64), Some(200));
        assert_eq!(v.get("tax_total").and_then(Value::as_i64), Some(200));
        assert_eq!(v.get("total").and_then(Value::as_i64), Some(1400));
    }
    #[test]
    fn estimate_free_threshold_boundary() {
        let v = est(2000, CFG_FREE_THR);
        assert_eq!(v.get("delivery_fee").and_then(Value::as_i64), Some(0));
        assert_eq!(v.get("tax_total").and_then(Value::as_i64), Some(200));
        assert_eq!(v.get("total").and_then(Value::as_i64), Some(2200));
    }
    #[test]
    fn estimate_pickup() {
        let v = est(1500, CFG_PICKUP);
        assert_eq!(v.get("delivery_fee").and_then(Value::as_i64), Some(0));
        assert_eq!(v.get("total").and_then(Value::as_i64), Some(1500 + 300));
    }
    #[test]
    fn estimate_distance_unknown() {
        let v = est(1000, CFG_DISTANCE);
        assert_eq!(v.get("fee_known").and_then(Value::as_bool), Some(false));
        assert_eq!(v.get("delivery_fee").map(Value::is_null), Some(true));
        assert_eq!(v.get("total").map(Value::is_null), Some(true));
    }
    #[test]
    fn estimate_min_not_met() {
        let v = est(400, CFG_MIN);
        assert_eq!(v.get("min_not_met").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("total").and_then(Value::as_i64), Some(400 + 200 + 80));
    }

    #[test]
    fn fsm_graph_report_js_shape() {
        let json = fsm_graph_report_logic().expect("report ok");
        let v = parse(&json).unwrap();
        assert_eq!(v.get("vertices").and_then(Value::as_i64), Some(12));
        assert_eq!(v.get("is_acyclic").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("cyclomatic").and_then(Value::as_i64), Some(4));
        assert_eq!(v.get("topological_len").and_then(Value::as_i64), Some(12));
        let rfp = v.get("reachable_from_pending").and_then(Value::as_i64).unwrap();
        assert_eq!(rfp & 1, 1);
    }

    const POLY: &str = "[[40.0,-3.0],[40.01,-3.0],[40.02,-3.0]]";

    #[test]
    fn geo_haversine_js_parity() {
        let d: f64 = geo_haversine_logic(51.5074, -0.1278, 48.8566, 2.3522)
            .unwrap()
            .parse()
            .unwrap();
        assert!((d - 343_000.0).abs() < 2_000.0, "London→Paris ≈ 343km");
    }

    #[test]
    fn geo_lerp_js_parity() {
        let j = geo_lerp_logic(0.0, 0.0, 10.0, 20.0, 0.5).unwrap();
        let v = parse(&j).unwrap();
        assert!((v.get("lat").and_then(Value::as_f64).unwrap() - 5.0).abs() < 1e-9);
        assert!((v.get("lng").and_then(Value::as_f64).unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn geo_progress_js_parity() {
        let j = geo_progress_logic(POLY, 40.005, -3.0).unwrap();
        let v = parse(&j).unwrap();
        assert_eq!(v.get("segment_index").and_then(Value::as_i64), Some(1));
        assert!((v.get("snapped").and_then(|x| x.get("lat")).and_then(Value::as_f64).unwrap() - 40.005).abs() < 1e-6);
        let rem = v.get("remaining_m").and_then(Value::as_f64).unwrap();
        assert!(rem > 1500.0 && rem < 1800.0, "remaining ≈ 1668 m, got {rem}");
    }

    #[test]
    fn geo_should_snap_js_parity() {
        let t = "[0.0,0.0]";
        let n = "[0.000045,0.0]";
        assert_eq!(geo_should_snap_logic(t, n, 500.0).unwrap(), "true");
        let far = "[0.01,0.0]";
        assert_eq!(geo_should_snap_logic(t, far, 500.0).unwrap(), "false");
    }

    #[test]
    fn geo_is_arriving_js_parity() {
        assert_eq!(geo_is_arriving_logic(120.0, crate::geo::ARRIVE_THRESHOLD_M).unwrap(), "true");
        assert_eq!(geo_is_arriving_logic(300.0, crate::geo::ARRIVE_THRESHOLD_M).unwrap(), "false");
    }

    #[test]
    fn geo_point_in_polygon_js_parity() {
        let sq = "[[0.0,0.0],[0.0,10.0],[10.0,10.0],[10.0,0.0]]";
        assert_eq!(geo_point_in_polygon_logic(5.0, 5.0, sq).unwrap(), "true");
        assert_eq!(geo_point_in_polygon_logic(15.0, 5.0, sq).unwrap(), "false");
    }

    #[test]
    fn geo_is_out_of_order_js_parity() {
        assert_eq!(geo_is_out_of_order_logic(-1, 100).unwrap(), "false");
        assert_eq!(geo_is_out_of_order_logic(100, 99).unwrap(), "true");
        assert_eq!(geo_is_out_of_order_logic(100, 101).unwrap(), "false");
    }

    #[test]
    fn spectral_eigenvalues_js_parity() {
        let j = spectral_eigenvalues_logic("[[0,1],[1,0]]").unwrap();
        let v = parse(&j).unwrap();
        let mut mods: Vec<f64> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let re = e.get("re").and_then(Value::as_f64).unwrap();
                let im = e.get("im").and_then(Value::as_f64).unwrap();
                crate::math::sqrt(crate::math::powi(re, 2) + crate::math::powi(im, 2))
            })
            .collect();
        mods.sort_by(|a, b| a.total_cmp(b));
        assert!((mods[0] - 1.0).abs() < 1e-6 && (mods[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn spectral_radius_and_gap_js_parity() {
        let rho: f64 = spectral_radius_logic("[[0.5,0.5],[0.5,0.5]]").unwrap().parse().unwrap();
        assert!((rho - 1.0).abs() < 1e-6);
        let gap: f64 = spectral_gap_logic("[[0.5,0.5],[0.5,0.5]]").unwrap().parse().unwrap();
        assert!((gap - 1.0).abs() < 1e-6);
        let gap0: f64 = spectral_gap_logic("[[0,1],[1,0]]").unwrap().parse().unwrap();
        assert!(gap0.abs() < 1e-6);
    }

    #[test]
    fn spectral_classify_drift_js_parity() {
        assert_eq!(spectral_classify_drift_logic("[[0.5,0],[0,0.3]]").unwrap(), "Damped");
        assert_eq!(spectral_classify_drift_logic("[[2,0],[0,1.5]]").unwrap(), "Unstable");
        assert_eq!(spectral_classify_drift_logic("[[0,1],[1,0]]").unwrap(), "Resonant");
    }

    #[test]
    fn spectral_surface_rejects_malformed_matrices() {
        assert!(spectral_radius_logic("[]").is_err());
        assert!(spectral_radius_logic("[[1,2,3],[4,5,6]]").is_err());
        assert!(spectral_radius_logic("not json").is_err());
        assert!(spectral_eigenvalues_logic("[[1,2],[3,\"x\"]]").is_err());
    }

    #[test]
    fn spectral_flat_js_matches_engine_contract() {
        let j = spectral_flat_logic("[[0,1],[1,0]]").unwrap();
        let body = j.trim_matches(|c| c == '[' || c == ']');
        let parts: Vec<f64> = body.split(',').map(|s| s.parse().unwrap()).collect();
        assert!(parts.len() >= 9);
        assert!((parts[0] - 1.0).abs() < 1e-6);
        assert!(parts[1].abs() < 1e-6);
        assert_eq!(parts[3] as i32, 1);
        assert_eq!(parts[4] as usize, 2);
        assert!((parts[5] - 1.0).abs() < 1e-6 && parts[6].abs() < 1e-6);
        assert!((parts[7] + 1.0).abs() < 1e-6 && parts[8].abs() < 1e-6);
    }
}
