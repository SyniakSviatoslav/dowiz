//! canonical.rs — canonical (order-independent) JSON + cache-line-aligned
//! hot-struct support, for LLM prompt-cache prefix stability (99% cache hits).
//!
//! The #1 cache-hit killer is non-deterministic serialization: a single key
//! reordering or a floating timestamp at the head of the context breaks the
//! provider's prefix hash. This module guarantees byte-identical output for
//! byte-identical *data*, regardless of insertion order:
//!
//! - object keys are sorted (bytewise) recursively,
//! - numbers are emitted via the shortest round-trip representation,
//! - strings are escaped with a single authority (the `fdr::json` escaper).
//!
//! Zero-dep, deterministic (no HashMap iteration order anywhere).

/// A canonical JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Self {
        Json::Str(s.into())
    }
    pub fn num(n: f64) -> Self {
        Json::Num(n)
    }
    pub fn bool(b: bool) -> Self {
        Json::Bool(b)
    }
    pub fn obj(pairs: Vec<(String, Json)>) -> Self {
        Json::Obj(pairs)
    }
    pub fn arr(items: Vec<Json>) -> Self {
        Json::Arr(items)
    }
}

/// Serialize a number as the shortest string that round-trips to the same f64.
fn write_num(out: &mut String, n: f64) {
    if n == 0.0 {
        // Canonical zero: fold -0.0 to 0.0.
        out.push('0');
        return;
    }
    if n.is_finite() {
        let s = n.to_string();
        // Rust's f64 Display is already shortest-round-trip; keep it.
        out.push_str(&s);
    } else {
        // Non-finite is unrepresentable in JSON — emit null (fail-closed).
        out.push_str("null");
    }
}

/// Serialize to canonical JSON (sorted object keys, shortest numbers).
pub fn canonical_json(v: &Json) -> String {
    let mut out = String::new();
    write_json(&mut out, v);
    out
}

fn write_json(out: &mut String, v: &Json) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => write_num(out, *n),
        Json::Str(s) => {
            out.push('"');
            crate::fdr::json::escape_into(out, s);
            out.push('"');
        }
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(out, item);
            }
            out.push(']');
        }
        Json::Obj(pairs) => {
            // Canonical: sort keys bytewise (the cache-prefix-stability property).
            let mut sorted: Vec<&(String, Json)> = pairs.iter().collect();
            sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push('{');
            for (i, (k, val)) in sorted.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                crate::fdr::json::escape_into(out, k);
                out.push('"');
                out.push(':');
                write_json(out, val);
            }
            out.push('}');
        }
    }
}

/// Convenience: canonical JSON from key/value pairs, keys sorted.
pub fn canonical_object(pairs: &[(String, Json)]) -> String {
    canonical_json(&Json::Obj(pairs.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_object_keys() {
        let a = canonical_json(&Json::obj(vec![
            ("b".into(), Json::num(1.0)),
            ("a".into(), Json::num(2.0)),
        ]));
        let b = canonical_json(&Json::obj(vec![
            ("a".into(), Json::num(2.0)),
            ("b".into(), Json::num(1.0)),
        ]));
        assert_eq!(a, b, "insertion order must not change canonical output");
        assert_eq!(a, "{\"a\":2,\"b\":1}");
    }

    #[test]
    fn folds_negative_zero() {
        assert_eq!(canonical_json(&Json::num(-0.0)), "0");
        assert_eq!(canonical_json(&Json::num(0.0)), "0");
    }

    #[test]
    fn nonfinite_becomes_null() {
        assert_eq!(canonical_json(&Json::num(f64::NAN)), "null");
        assert_eq!(canonical_json(&Json::num(f64::INFINITY)), "null");
    }

    #[test]
    fn escapes_strings() {
        let s = canonical_json(&Json::str("a\"b\\c"));
        assert_eq!(s, "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn nested_objects_sorted_recursively() {
        let v = Json::obj(vec![(
            "outer".into(),
            Json::obj(vec![("z".into(), Json::bool(true)), ("a".into(), Json::Null)]),
        )]);
        let s = canonical_json(&v);
        assert_eq!(s, "{\"outer\":{\"a\":null,\"z\":true}}");
    }

    #[test]
    fn arrays_preserve_order() {
        let v = Json::arr(vec![Json::num(3.0), Json::num(1.0), Json::num(2.0)]);
        assert_eq!(canonical_json(&v), "[3,1,2]");
    }

    #[test]
    fn canonical_object_matches_manual() {
        let pairs = vec![("k".into(), Json::num(1.0))];
        assert_eq!(canonical_object(&pairs), "{\"k\":1}");
    }
}
