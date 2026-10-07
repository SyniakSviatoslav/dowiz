//! The drill over a bundle built from REAL images (the Worker test drives the real nightly
//! path; this one pins the loaders and the strict base64 without a Worker).
use super::*;
use crate::EventKind;

fn enc(b: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            s.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

fn entry(bytes: &[u8]) -> serde_json::Value {
    serde_json::json!({ "generation": 1, "bytes": bytes.len(), "sha256": hex(&Sha256::digest(bytes)), "image": enc(bytes) })
}

fn images() -> (Hub, Vec<u8>) {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for i in 0..3 {
        h.append(EventKind::Placed, &format!("ord_{i}"), r#"{"status":"PENDING"}"#, i, [0; 32]).unwrap();
    }
    let mut c = Catalog::create().unwrap();
    c.set_product("d1", r#"{"id":"d1","price":900}"#);
    (h, c.to_bytes().unwrap())
}

fn bundle(log: &[u8], catalog: &[u8]) -> Vec<u8> {
    serde_json::json!({ "format": FORMAT, "venue": "alpha", "taken_at_ms": 7,
        "images": { "log": entry(log), "catalog": entry(catalog) }, "archives": {} })
    .to_string()
    .into_bytes()
}

#[test]
fn strict_base64_reads_the_rfc_vectors_and_refuses_the_rest() {
    for (plain, coded) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
        assert_eq!(b64(coded).as_deref(), Some(plain.as_bytes()), "{coded}");
        assert_eq!(enc(plain.as_bytes()), coded);
    }
    for bad in ["Zg=", "Zg=a", "Z===", "Zm9v\n", "Zm=v"] {
        assert_eq!(b64(bad), None, "{bad:?}");
    }
}

#[test]
fn a_clean_copy_passes_and_its_tip_equals_the_witness() {
    let (h, cat) = images();
    let tip = h.tip().unwrap();
    let w = serde_json::json!({ "venue": "alpha", "tip": tip, "records": 3 }).to_string();
    let d = run(&bundle(&h.to_bytes_trimmed(), &cat), Some(w.as_bytes()));
    assert!(d.passed(), "{:?}", d.refused);
    assert_eq!((d.tip, d.chain.map(|c| c.records)), (Tip::Equal, Some(3)));
}

/// One named cell: a byte inside the catalogue's dish VALUE, with the manifest's sha256
/// recomputed so the digest passes and only the loader's crc can catch it.
#[test]
fn a_changed_catalogue_byte_with_a_fixed_digest_is_refused_by_the_crc() {
    let (h, mut cat) = images();
    let at = cat.windows(9).position(|w| w == b"price\":90").unwrap() + 8;
    cat[at] = b'1';
    let d = run(&bundle(&h.to_bytes_trimmed(), &cat), None);
    assert!(d.refused.iter().any(|r| r.starts_with("catalog: refused by its loader: BadCrc")), "{:?}", d.refused);
}

#[test]
fn a_witnessed_tip_the_copy_does_not_hold_is_refused() {
    let (h, cat) = images();
    let w = serde_json::json!({ "venue": "alpha", "tip": "ab".repeat(32) }).to_string();
    let d = run(&bundle(&h.to_bytes_trimmed(), &cat), Some(w.as_bytes()));
    assert_eq!(d.tip, Tip::Missing);
    assert!(!d.passed());
}
