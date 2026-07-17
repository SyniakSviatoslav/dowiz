//! native-ser — pure-std serialization with EDGE adapters.
//!
//! Replaces the deleted `tools/telemetry/ser.py` (stdlib-only) with a pure-Rust
//! port. The contract is byte-identical to the Python:
//!   * `wire_f64` / `unwire_f64`   — canonical native encode/decode (raw LE f64).
//!   * `native_of` / `obj_of`     — typed obj <-> native f64 array (schema-ordered).
//!   * `json_edge` / `from_json`  — EDGE adapters (JSON text only where required).
//!   * `default_edge`             — live-read the `TELEMETRY_SER` env selector.
//!
//! No serde, no external crates. The canonical form is the kernel C-ABI
//! `field_metrics` wire shape (no parser, no allocator) — the speed floor.

mod json;

pub use json::Json;

/// Canonical: raw little-endian f64 array (no length prefix; caller knows N).
pub fn wire_f64(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for &v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Canonical: decode a raw LE f64 byte block back to f64s.
/// `n` = number of f64s; defaults to byte-length / 8.
pub fn unwire_f64(b: &[u8], n: Option<usize>) -> Vec<f64> {
    let n = n.unwrap_or(b.len() / 8);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let off = i * 8;
        if off + 8 > b.len() {
            break;
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&b[off..off + 8]);
        out.push(f64::from_le_bytes(buf));
    }
    out
}

/// Encode a numeric dict into a native f64 array following `schema` (ordered keys).
/// Labels (str/bool) stay at the EDGE, not in the native array.
pub fn native_of(schema: &[&str], obj: &std::collections::HashMap<String, f64>) -> Vec<u8> {
    let vals: Vec<f64> = schema.iter().map(|k| obj.get(*k).copied().unwrap_or(0.0)).collect();
    wire_f64(&vals)
}

/// Decode a native f64 array back to a dict under `schema`.
pub fn obj_of(schema: &[&str], b: &[u8]) -> std::collections::HashMap<String, f64> {
    let vals = unwire_f64(b, Some(schema.len()));
    let mut m = std::collections::HashMap::new();
    for (k, v) in schema.iter().zip(vals.iter()) {
        m.insert((*k).to_string(), *v);
    }
    m
}

/// EDGE: produce JSON text (Gatus body, grep-able ledgers, Telegram).
pub fn json_edge(obj: &Json) -> String {
    obj.to_string()
}

/// EDGE: JSON text -> obj.
pub fn from_json(s: &str) -> Result<Json, String> {
    json::parse(s)
}

/// Live-read the edge selection (callers can flip without re-import).
pub fn default_edge() -> String {
    let f = std::env::var("TELEMETRY_SER").unwrap_or_else(|_| "json".to_string()).to_lowercase();
    if f == "json" || f == "native" {
        f
    } else {
        "json".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_roundtrip_is_identity() {
        let v = [3.14f64, -2.5, 0.0, 1e9, -0.0];
        let b = wire_f64(&v);
        assert_eq!(b.len(), v.len() * 8);
        let back = unwire_f64(&b, None);
        for (a, c) in v.iter().zip(back.iter()) {
            assert_eq!(a.to_bits(), c.to_bits(), "f64 bit-identical round-trip");
        }
    }

    #[test]
    fn native_of_obj_of_roundtrip() {
        let schema = ["disk_pct", "load1", "mem_pct"];
        let mut obj = std::collections::HashMap::new();
        obj.insert("disk_pct".to_string(), 81.0);
        obj.insert("load1".to_string(), 0.12);
        obj.insert("mem_pct".to_string(), 9.0);
        let b = native_of(&schema, &obj);
        let back = obj_of(&schema, &b);
        assert_eq!(back.get("disk_pct").copied().unwrap_or(-1.0), 81.0);
        assert_eq!(back.get("load1").copied().unwrap_or(-1.0), 0.12);
        assert_eq!(back.get("mem_pct").copied().unwrap_or(-1.0), 9.0);
    }

    #[test]
    fn json_edge_roundtrip() {
        let j = Json::Obj(vec![
            ("disk_pct".into(), Json::Num(81.0)),
            ("load1".into(), Json::Num(0.12)),
        ]);
        let s = json_edge(&j);
        let parsed = from_json(&s).expect("parse");
        assert_eq!(parsed, j);
    }

    #[test]
    fn default_edge_validates_env() {
        std::env::set_var("TELEMETRY_SER", "bogus");
        assert_eq!(default_edge(), "json");
        std::env::set_var("TELEMETRY_SER", "native");
        assert_eq!(default_edge(), "native");
        std::env::remove_var("TELEMETRY_SER");
    }
}
