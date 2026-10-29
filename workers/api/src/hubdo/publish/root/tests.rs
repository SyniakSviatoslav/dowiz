//! T-LV-PCD-02 (Worker half): the Worker's tree is the gate's tree, and the signature verifies.

use super::*;
use std::cell::RefCell;

thread_local! {
    /// The TEST key's seed (cfg(test) only; never a production key): what `VENUE_SIGN_SK` holds live.
    pub(in crate::hubdo::publish) static TEST_SEED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A TEST seed, 32 bytes of 0x5a. Never a production key (operator decision Q4).
pub(in crate::hubdo::publish) const SEED: &str = "5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";

/// The catalogue `crates/bebop-wasm/oracle_verify.py --make` cuts the gate fixtures from; its root is
/// the `good` line of `crates/bebop-wasm/fixtures/verify.expected` (and bebop_store::merkle's).
#[test]
fn the_workers_root_is_the_gate_fixtures_root() {
    let rows: Vec<(String, i64)> = (0..165).map(|i| (format!("p_{i:03}"), 100 + 7 * i as i64)).collect();
    assert_eq!(hex(&root_of(&rows)), "9d64ce278dfc1b4282ef478ea3361e6f9ded6cfc8ac8717d3d2c234e8481879b");
    assert_eq!(hex(&mth(&[])), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn prices_come_from_the_fragment_once_per_id() {
    let ok = r#"{"categories":[{"products":[{"id":"b","price":2},{"id":"a","price":1}]},{"products":[{"id":"a","price":1}]}]}"#;
    assert_eq!(prices(ok), Some(vec![("a".into(), 1), ("b".into(), 2)]), "a dish in two categories is one leaf");
    let two = r#"{"categories":[{"products":[{"id":"a","price":1},{"id":"a","price":2}]}]}"#;
    assert_eq!(prices(two), None, "one id, two prices: no root");
}

#[test]
fn the_signature_verifies_and_binds_the_message() {
    use dowiz_core::pq::dsa;
    let msg = signed_message("dubin", 4, 165, &[7; 32]);
    let t = std::time::Instant::now();
    let (pk, sig) = sign(SEED, &msg).expect("a 64-hex seed");
    println!("bench sign_ms_dsa65 (keygen + sign, native debug) = {:.1}", t.elapsed().as_secs_f64() * 1e3);
    assert_eq!(pk.len(), dsa::PUBLICKEYBYTES);
    assert!(dsa::verify_internal_bytes(&pk, &msg, &sig), "positive twin");
    let other = signed_message("dubin", 5, 165, &[7; 32]);
    assert!(!dsa::verify_internal_bytes(&pk, &other, &sig), "another generation is another message");
    assert_eq!(sign(SEED, &msg).unwrap().1, sig, "deterministic: rnd = 0");
    assert_eq!(pk_id(&pk).len(), 16);
    assert!(sign("not hex", &msg).is_none());
    assert!(sign(&SEED[..62], &msg).is_none(), "a short seed is refused, not padded");
}
