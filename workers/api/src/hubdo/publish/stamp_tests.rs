//! T-LV-PCD-02: the price root and its signature in `manifest.json` (LV-PCD-C), through the real
//! `HubImages` over memory, as `publish/tests.rs` drives the rest of the publish.

use super::tests::{manifest, published_venue};
use crate::hubdo::menu::tests::catalog;
use serde_json::Value;

/// T-LV-PCD-02: the manifest names the price root over the fragment's prices. Without the venue's
/// key it says so (`"sig": null`, no `pk_id`); a price edit moves the root.
#[test]
fn root_in_manifest() {
    super::root::tests::TEST_SEED.with(|s| *s.borrow_mut() = None);
    let (h, b) = published_venue();
    let m = manifest(&b);
    let root = m["root"].as_str().expect("a root").to_string();
    assert_eq!(root.len(), 64);
    assert_eq!(m["n"], 3, "three dishes, three leaves");
    assert_eq!(m["sig"], Value::Null, "unsigned is said");
    assert!(m.get("pk_id").is_none());
    b.drain();
    assert_eq!(h.put("catalog", 1, &catalog(950).to_bytes().unwrap()).status_code(), 200);
    assert_ne!(manifest(&b)["root"].as_str(), Some(root.as_str()), "a price edit moves the root");
}

/// With a TEST key (cfg(test) only) the manifest names `pk_id` and a signature object that verifies
/// over the signed message of exactly this root, and the record keeps what it signed.
#[test]
fn signed_root_in_manifest() {
    use base64::Engine;
    super::root::tests::TEST_SEED.with(|s| *s.borrow_mut() = Some(super::root::tests::SEED.to_string()));
    let (h, b) = published_venue();
    let m = manifest(&b);
    let key = format!("v/dubin/{}", m["sig"].as_str().expect("a signature object"));
    let obj = b.puts.borrow().iter().find(|p| p.key == key).map(|p| p.bytes.clone()).expect("the signature object was written");
    let o: Value = serde_json::from_slice(&obj).unwrap();
    let d = |k: &str| base64::engine::general_purpose::STANDARD.decode(o[k].as_str().unwrap()).unwrap();
    let (pk, sig) = (d("pk"), d("sig"));
    assert_eq!(m["pk_id"], super::root::pk_id(&pk));
    let mut r = [0u8; 32];
    for (i, x) in r.iter_mut().enumerate() {
        *x = u8::from_str_radix(&m["root"].as_str().unwrap()[2 * i..2 * i + 2], 16).unwrap();
    }
    let msg = super::root::signed_message("dubin", m["gens"]["catalog"].as_i64().unwrap(), 3, &r);
    assert_eq!(o["msg"], super::root::hex(&msg));
    assert!(dowiz_core::pq::dsa::verify_internal_bytes(&pk, &msg, &sig), "the signature verifies");
    // the record keeps the signed triple, so an unchanged message is named again, not re-signed
    let rec = h.get("/fold/publish").body_value();
    assert!(rec["published"]["signed"].is_array(), "the record keeps what it signed");
    super::root::tests::TEST_SEED.with(|s| *s.borrow_mut() = None);
}
