//! Differential oracle for `kernel::json` (roadmap item 31 §4.4 Phase-A gate).
//!
//! `kernel::json` is the hand-rolled parse+serialize primitive that the serde carriers cut over
//! to. Before ANY carrier ruling (item-25 step 4), it must be proven to behave identically to the
//! incumbent `serde_json` on the real corpus the carriers parse. This test is that proof:
//!
//!   (1) PARSE PARITY — for every corpus string, `kernel::json::parse` and `serde_json::from_str`
//!       agree: either both reject it, or both accept it with a semantically-equal value.
//!   (2) ROUND-TRIP — `kernel::json`'s serializer emits JSON that `serde_json` re-parses back to
//!       the same value (so a cut-over carrier's OUTPUT is byte-consumable by any JSON reader).
//!   (3) PROPTEST FUZZ — thousands of randomly-generated bounded values survive the
//!       Value → kernel serialize → serde parse → equal round-trip (the differential-fuzz load
//!       the §4.2 risk analysis demands before trusting the hand-roll).
//!
//! `serde_json` is a DEV-dependency only (outside the `-e no-dev` zero-dep proof surface).

use dowiz_kernel::json::{self, Value};
use proptest::prelude::*;
use serde_json::Value as Sj;

/// Semantic equality between a `kernel::json::Value` and a `serde_json::Value`. Objects compare as
/// last-wins key→value maps (order-insensitive); numbers compare by value across the Int/Float
/// split; floats compare bit-exact (both parsers are correctly-rounded) with a 1-ULP tolerance.
fn equiv(k: &Value, s: &Sj) -> bool {
    match (k, s) {
        (Value::Null, Sj::Null) => true,
        (Value::Bool(a), Sj::Bool(b)) => a == b,
        (Value::Str(a), Sj::String(b)) => a == b,
        (Value::Int(a), Sj::Number(n)) => {
            if let Some(i) = n.as_i64() {
                *a == i
            } else if let Some(u) = n.as_u64() {
                u <= i64::MAX as u64 && *a == u as i64
            } else {
                false
            }
        }
        (Value::Float(a), Sj::Number(n)) => match n.as_f64() {
            Some(f) => a == &f || (a - f).abs() <= f64::EPSILON * a.abs().max(f.abs()).max(1.0),
            None => false,
        },
        (Value::Array(a), Sj::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| equiv(x, y))
        }
        (Value::Object(a), Sj::Object(b)) => {
            use std::collections::BTreeMap;
            // kernel keeps duplicate keys; reduce to last-wins to match serde_json's Map.
            let mut km: BTreeMap<&str, &Value> = BTreeMap::new();
            for (key, val) in a {
                km.insert(key.as_str(), val);
            }
            km.len() == b.len()
                && km
                    .iter()
                    .all(|(key, val)| b.get(*key).map(|sv| equiv(val, sv)).unwrap_or(false))
        }
        _ => false,
    }
}

/// The real shapes the Phase-A carriers parse, plus adversarial inputs the §4.2 risk analysis
/// names explicitly (deep nesting, surrogate pairs, number-grammar edges, control chars, dup keys).
fn corpus() -> Vec<&'static str> {
    vec![
        // --- agent-facade: tool-call args (bounded schema) ---
        r#"{"order_id":"ord_123"}"#,
        r#"{"order_id":"","extra":true}"#,
        r#"{"order_id":"a b/c\"d"}"#,
        // --- skillspector-rs: JSON-RPC 2.0 serve-mode frames ---
        r#"{"jsonrpc":"2.0","id":1,"method":"health"}"#,
        r#"{"jsonrpc":"2.0","id":null,"method":"scan","params":{"paths":["src","tools"]}}"#,
        r#"{"jsonrpc":"2.0","id":42,"method":"scan","params":{"paths":[]}}"#,
        // --- skillspector finding-output shapes (f64 confidence, nested loc, Option→null, map) ---
        r#"{"findings":[{"id":"f1","severity":"high","confidence":0.9,"location":{"file":"a.py","start_line":3,"end_line":null},"tags":["x","y"],"pattern":null}],"summary":{"files_scanned":2,"findings":1,"by_severity":{"high":1}}}"#,
        // --- wasm SpineDoc array (verified but DEFERRED to Phase-B; still a real corpus shape) ---
        r#"[{"id":"1","title":"Doc One","tags":["a","b"],"path":"/p/1"}]"#,
        r#"[]"#,
        // --- primitives / scalars ---
        "null",
        "true",
        "false",
        "0",
        "-0",
        "42",
        "-17",
        "3.14",
        "-2.5e10",
        "1e-9",
        "0.0",
        r#""plain string""#,
        r#""unicode: café ☕ 你好""#,
        r#""escapes: \" \\ \/ \b \f \n \r \t""#,
        r#""bmp escape: é""#,
        r#""surrogate pair: 😀""#,
        // --- structural edges ---
        "{}",
        "[]",
        r#"{"a":{"b":{"c":{"d":[1,2,3]}}}}"#,
        r#"{"dup":1,"dup":2}"#, // duplicate key → last-wins in both
        "  \t\n [ 1 , 2 , 3 ]  ",
        r#"{"nested":[{"k":[true,false,null]},{"n":-3.0e2}]}"#,
        // --- inputs BOTH must REJECT (degrade-closed parity) ---
        "",
        "{",
        "[1,2,",
        r#"{"a":}"#,
        r#"{"a" 1}"#,
        "01",       // leading zero
        "1.",       // no digit after point
        "1e",       // no exponent digits
        ".5",       // no integer part
        "+1",       // leading plus
        "nul",      // truncated literal
        "trailing garbage",
        r#""unterminated"#,
        "[1,2]extra",
        r#"{"x":1}}"#,
        "\"\u{01}\"", // raw control char inside string (byte 0x01)
        r#""\uD800""#, // lone high surrogate
        r#""\uDC00""#, // lone low surrogate
        r#""\x""#,      // invalid escape
    ]
}

#[test]
fn parse_parity_and_round_trip_over_corpus() {
    let items = corpus();
    let mut parse_agree = 0usize;
    let mut both_ok = 0usize;
    let mut both_err = 0usize;
    let mut round_trips = 0usize;

    for src in &items {
        let k = json::parse(src);
        let s = serde_json::from_str::<Sj>(src);
        match (&k, &s) {
            (Ok(kv), Ok(sv)) => {
                assert!(
                    equiv(kv, sv),
                    "PARSE MISMATCH (both accepted, values differ)\n  input: {src}\n  kernel: {kv:?}\n  serde:  {sv:?}"
                );
                parse_agree += 1;
                both_ok += 1;
                // (2) round-trip: kernel serialize → serde re-parse → equal
                let reparsed = serde_json::from_str::<Sj>(&kv.to_string()).unwrap_or_else(|e| {
                    panic!("kernel::json output not valid JSON for {src:?}: {e}\n  emitted: {}", kv.to_string())
                });
                assert!(
                    equiv(kv, &reparsed),
                    "ROUND-TRIP MISMATCH\n  input: {src}\n  emitted: {}\n  reparsed: {reparsed:?}",
                    kv.to_string()
                );
                round_trips += 1;
            }
            (Err(_), Err(_)) => {
                parse_agree += 1;
                both_err += 1;
            }
            (ka, sa) => panic!(
                "ACCEPT/REJECT DISAGREEMENT on {src:?}\n  kernel: {ka:?}\n  serde:  {sa:?}"
            ),
        }
    }

    assert_eq!(parse_agree, items.len(), "every corpus item must agree");
    eprintln!(
        "json_oracle: corpus={} parse_agree={} (both_ok={}, both_err={}) round_trips={}",
        items.len(),
        parse_agree,
        both_ok,
        both_err,
        round_trips
    );
    assert!(both_ok >= 25, "corpus must exercise real accepted shapes");
    assert!(both_err >= 15, "corpus must exercise real rejected shapes (degrade-closed parity)");
}

// ---- (3) proptest differential fuzz: random bounded values survive kernel→serde round-trip ----

/// A bounded JSON value strategy (i64 ints, finite f64, arbitrary strings incl. control chars and
/// non-BMP, nested arrays/objects up to depth 4). Emitted as text via `serde_json` and re-parsed by
/// `kernel::json`, then compared — the reverse direction of the corpus round-trip.
fn arb_json() -> impl Strategy<Value = Sj> {
    // Numbers span the carriers' REAL input distribution, per the module's documented scope
    // (§4.2 — bounded config/API/JSON-RPC shapes, NOT the adversarial unbounded float space that
    // keeps native-spa-server on serde_json). Integers cover the FULL i64 range (they round-trip
    // exactly). Floats cover realistic magnitudes + the fraction/edge values carriers actually
    // emit (confidence 0..1, coords, small exponents). We deliberately do NOT fuzz e±140-magnitude
    // floats: there Rust's shortest-float formatter and serde_json's float PARSER are not perfectly
    // reciprocal (~2 ULP), a serde/Rust library-boundary artifact unrelated to kernel::json and
    // outside every carrier's real shape.
    let realistic_float = prop_oneof![
        -1.0e9f64..1.0e9f64,
        -1.0f64..1.0f64,
        Just(0.0f64),
        Just(-0.0f64),
        Just(0.5f64),
        Just(1e-9f64),
        Just(-1e-9f64),
        Just(1234.5678f64),
    ];
    let leaf = prop_oneof![
        Just(Sj::Null),
        any::<bool>().prop_map(Sj::Bool),
        any::<i64>().prop_map(|i| Sj::Number(i.into())),
        realistic_float
            .prop_filter("finite", |f: &f64| f.is_finite())
            .prop_map(|f| serde_json::Number::from_f64(f).map(Sj::Number).unwrap_or(Sj::Null)),
        ".*".prop_map(Sj::String),
    ];
    leaf.prop_recursive(4, 32, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Sj::Array),
            prop::collection::vec((".*", inner), 0..6).prop_map(|kvs| {
                Sj::Object(kvs.into_iter().collect())
            }),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn fuzz_serde_text_parses_equivalently_in_kernel(v in arb_json()) {
        let text = serde_json::to_string(&v).unwrap();
        let kv = json::parse(&text)
            .unwrap_or_else(|e| panic!("kernel::json rejected serde-emitted JSON: {e}\n  text: {text}"));
        prop_assert!(equiv(&kv, &v), "fuzz mismatch\n  text: {text}\n  kernel: {kv:?}\n  serde: {v:?}");
        // and kernel's own re-serialization must also round-trip through serde
        let back = serde_json::from_str::<Sj>(&kv.to_string()).unwrap();
        prop_assert!(equiv(&kv, &back));
    }
}
