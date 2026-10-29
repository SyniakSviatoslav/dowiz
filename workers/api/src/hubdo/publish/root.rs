//! THE PUBLISHED PRICE ROOT (LV-PCD-C, AC-LV-PCD-02; normative text
//! `docs/design/SPEC-PCD-2026-10-27.md`). Every publish names, in `manifest.json`, the RFC 9162
//! Merkle root over the menu's prices -- one leaf per product, `sha256(0x00 ‖ field(id) ‖
//! field(price i64 LE))`, in id byte order -- and the content-addressed list of those prices, so a
//! receipt line can carry a path to the root and anyone can climb it offline
//! (`public/store/verify.js`).
//!
//! This is the Worker's reader of the tree, written over `sha2` because the Worker does not link
//! `bebop-store` (the crate's own `merkle.rs` is the normative Rust); `root/tests.rs` pins it to the
//! gate fixture root of `crates/bebop-wasm/fixtures/verify.expected`, so the two cannot drift.
//!
//! THE SIGNATURE is ML-DSA-65 (`dowiz_core::pq::dsa`, FIPS 204, ACVP-gated), deterministic
//! (`rnd = 0^32`) over `signed_message`, with the key derived from the 32-byte seed in the
//! `VENUE_SIGN_SK` secret (64 hex). THE OPERATOR CREATES THAT SECRET (decision Q4, never rotated);
//! without it the manifest carries the root and no `pk_id`/`sig`, and `/verify` says "unsigned".

use base64::Engine;
use sha2::{Digest, Sha256};

pub(super) type Hash = [u8; 32];

fn sha(parts: &[&[u8]]) -> Hash {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

fn field(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}

pub(super) fn leaf(id: &str, price: i64) -> Hash {
    let mut d = Vec::with_capacity(id.len() + 24);
    field(&mut d, id.as_bytes());
    field(&mut d, &price.to_le_bytes());
    sha(&[&[0x00], &d])
}

/// MTH (RFC 9162 §2.1.1).
pub(super) fn mth(hs: &[Hash]) -> Hash {
    match hs.len() {
        0 => sha(&[]),
        1 => hs[0],
        n => {
            let mut k = 1;
            while k * 2 < n {
                k *= 2;
            }
            sha(&[&[0x01], &mth(&hs[..k]), &mth(&hs[k..])])
        }
    }
}

/// The menu's prices, `(id, price)` in id byte order: every `categories[].products[]` with a string
/// `id` and an integer `price`. `None` when an id repeats with a different price (one id, one price).
pub(super) fn prices(fragment: &str) -> Option<Vec<(String, i64)>> {
    let body: serde_json::Value = serde_json::from_str(fragment).ok()?;
    let mut v = Vec::new();
    for cat in body.get("categories").and_then(serde_json::Value::as_array).into_iter().flatten() {
        for p in cat.get("products").and_then(serde_json::Value::as_array).into_iter().flatten() {
            if let (Some(id), Some(price)) = (p.get("id").and_then(|x| x.as_str()), p.get("price").and_then(|x| x.as_i64())) {
                v.push((id.to_string(), price));
            }
        }
    }
    v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()).then(a.1.cmp(&b.1)));
    v.dedup();
    if v.windows(2).any(|w| w[0].0 == w[1].0) {
        return None;
    }
    Some(v)
}

pub(super) fn root_of(rows: &[(String, i64)]) -> Hash {
    mth(&rows.iter().map(|(id, p)| leaf(id, *p)).collect::<Vec<_>>())
}

/// The bytes the venue signs: byte-for-byte `bebop_store::merkle::signed_message`.
pub(super) fn signed_message(slug: &str, gen: i64, n: u64, root: &Hash) -> Vec<u8> {
    let mut v = vec![b'R'];
    field(&mut v, b"dowiz-price-root-v1");
    field(&mut v, slug.as_bytes());
    field(&mut v, &gen.to_le_bytes());
    field(&mut v, &n.to_le_bytes());
    field(&mut v, root);
    v
}

pub(super) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// `pk_id` = the first 8 bytes of sha256(pk), hex.
pub(super) fn pk_id(pk: &[u8]) -> String {
    hex(&sha(&[pk])[..8])
}

/// Sign `msg` with the key the 64-hex `seed` derives. `None` on a seed that is not 64 hex.
/// Returns `(pk, sig)`.
pub(super) fn sign(seed: &str, msg: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    use dowiz_core::pq::dsa;
    let s = seed.trim();
    if s.len() != 64 {
        return None;
    }
    let mut k = [0u8; dsa::SEEDBYTES];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    let (pk, sk) = dsa::keygen_bytes(&k);
    Some((pk.clone(), dsa::sign_internal_bytes(&sk, msg, &[0u8; dsa::RNDBYTES])))
}

/// The signature object's JSON: `{alg, pk, sig, msg}` (base64 standard; msg hex).
pub(super) fn sig_json(pk: &[u8], sig: &[u8], msg: &[u8]) -> String {
    let b = base64::engine::general_purpose::STANDARD;
    serde_json::json!({ "alg": "ML-DSA-65", "pk": b.encode(pk), "sig": b.encode(sig), "msg": hex(msg) }).to_string()
}

/// LV-PCD-C: what `manifest.json` says about the menu's prices (SPEC-PCD §4).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::hubdo) struct Stamp {
    /// The RFC 9162 root over the fragment's prices, hex.
    pub root: String,
    /// The leaf count.
    pub n: u64,
    /// `pk_id` and the signature object's key, when the venue holds `VENUE_SIGN_SK`.
    pub signed: Option<(String, String)>,
}

/// The manifest with the price root on it. Unsigned is said, not implied: `"sig": null`.
pub(in crate::hubdo) fn stamp(manifest: &str, s: &Stamp) -> String {
    let mut m: serde_json::Value = serde_json::from_str(manifest).unwrap_or_default();
    if let Some(o) = m.as_object_mut() {
        o.insert("root".into(), serde_json::json!(s.root));
        o.insert("n".into(), serde_json::json!(s.n));
        match &s.signed {
            Some((pk_id, sig)) => {
                o.insert("pk_id".into(), serde_json::json!(pk_id));
                o.insert("sig".into(), serde_json::json!(sig));
            }
            None => {
                o.insert("sig".into(), serde_json::Value::Null);
            }
        }
    }
    m.to_string()
}

/// The stamp for `fragment`, and the record's `signed` triple. The signature object is pushed onto
/// `set`; when `prev` signed this same message and that object is on the bucket, it is NAMED again
/// (empty bytes, `plan` skips it) instead of signed twice.
pub(super) fn stamp_for(
    fragment: &str,
    slug: &str,
    gen: i64,
    prev: &super::Published,
    everything: bool,
    seed: impl FnOnce() -> Option<String>,
    set: &mut Vec<super::Object>,
) -> (Option<Stamp>, Option<[String; 3]>) {
    let Some(rows) = prices(fragment) else { return (None, None) };
    let r = root_of(&rows);
    let msg = signed_message(slug, gen, rows.len() as u64, &r);
    let mk = crate::hubdo::menu::k64(&msg);
    let again = prev.signed.as_ref().filter(|s| s[0] == mk && !everything && prev.objects.get("sig") == Some(&s[1]));
    let signed = match again {
        Some(s) => {
            set.push(super::Object { name: "sig".into(), key: s[1].clone(), bytes: Vec::new(), content_type: super::JSON });
            Some(s.clone())
        }
        None => seed().and_then(|k| sign(&k, &msg)).map(|(pk, sig)| {
            let o = super::Object::json("sig", sig_json(&pk, &sig, &msg));
            let t = [mk.clone(), o.key.clone(), pk_id(&pk)];
            set.push(o);
            t
        }),
    };
    let st = Stamp { root: hex(&r), n: rows.len() as u64, signed: signed.as_ref().map(|s| (s[2].clone(), s[1].clone())) };
    (Some(st), signed)
}

#[cfg(test)]
pub(super) mod tests;
