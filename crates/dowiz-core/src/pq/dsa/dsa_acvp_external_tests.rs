//! The EXTERNAL pure ML-DSA-65 group of NIST ACVP (W-MG-SIG): 30 sigGen and 15 sigVer vectors,
//! the group the seal's format stands on. `dsa_acvp_tests.rs` covers the internal interface only,
//! so "ACVP byte-exact" said nothing about `M'` until this file.
//!
//! `kat/acvp/sig-{gen,ver}-external.json` are `usnistgov/ACVP-Server`,
//! `gen-val/json-files/ML-DSA-{sigGen,sigVer}-FIPS204/internalProjection.json` as fetched
//! 2026-11-01 (vsId 42, isSample false), with every group dropped but ML-DSA-65 + external + pure:
//! sigGen tgId 3 (deterministic) and 15 (hedged), sigVer tgId 3. Nothing inside a group is edited.
//!
//! Only 3 of the 45 carry the empty context a seal uses, so each vector goes through
//! `external_message(ctx, ..)`, and the empty ones through `sign_pure_bytes` as well.

use alloc::vec::Vec;

use serde_json::Value;

use crate::pq::dsa::{external_message, sign_internal_bytes, sign_pure_bytes, verify_internal_bytes, verify_pure_bytes};

fn load(file: &str) -> Value {
    let p = alloc::format!("{}/src/pq/kat/acvp/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&p).expect("vendored ACVP file")).expect("ACVP json")
}

fn hex(v: &Value, field: &str) -> Vec<u8> {
    let s = v[field].as_str().unwrap_or_else(|| panic!("no field {field}"));
    assert_eq!(s.len() % 2, 0, "{field}");
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex")).collect()
}

/// `(rnd, test)` of every case, proved to be the group this file says it is.
fn cases(file: &str, deterministic_rule: bool) -> Vec<([u8; 32], Value)> {
    let mut out = Vec::new();
    for g in load(file)["testGroups"].as_array().expect("groups") {
        assert_eq!(g["parameterSet"], "ML-DSA-65");
        assert_eq!(g["signatureInterface"], "external");
        assert_eq!(g["preHash"], "pure");
        assert_eq!(g["externalMu"], false);
        for t in g["tests"].as_array().expect("tests") {
            let mut rnd = [0u8; 32];
            if deterministic_rule && g["deterministic"] == false {
                rnd.copy_from_slice(&hex(t, "rnd"));
            }
            out.push((rnd, t.clone()));
        }
    }
    out
}

#[test]
fn acvp_external_siggen_is_byte_exact() {
    let all = cases("sig-gen-external.json", true);
    assert_eq!(all.len(), 30, "the group is 15 deterministic + 15 hedged");
    let mut empty = 0;
    for (rnd, t) in &all {
        let (id, sk, pk) = (&t["tcId"], hex(t, "sk"), hex(t, "pk"));
        let (msg, ctx, want) = (hex(t, "message"), hex(t, "context"), hex(t, "signature"));
        let m = external_message(&ctx, &msg);
        assert_eq!(sign_internal_bytes(&sk, &m, rnd), want, "tcId {id}");
        assert!(verify_internal_bytes(&pk, &m, &want), "tcId {id}: the positive twin");
        assert!(!verify_internal_bytes(&pk, &msg, &want), "tcId {id}: not a signature over the bare message");
        if ctx.is_empty() {
            empty += 1;
            assert_eq!(sign_pure_bytes(&sk, &msg, rnd), want, "tcId {id}: sign_pure_bytes");
            assert!(verify_pure_bytes(&pk, &msg, &want), "tcId {id}: verify_pure_bytes");
        }
    }
    assert_eq!(empty, 2, "the empty-context vectors, or sign_pure_bytes met none");
}

#[test]
fn acvp_external_sigver_agrees_on_every_verdict() {
    let all = cases("sig-ver-external.json", false);
    assert_eq!(all.len(), 15);
    let (mut accepted, mut refused, mut empty) = (0, 0, 0);
    for (_, t) in &all {
        let (id, pk, msg, ctx, sig) = (&t["tcId"], hex(t, "pk"), hex(t, "message"), hex(t, "context"), hex(t, "signature"));
        let want = t["testPassed"].as_bool().expect("testPassed");
        assert_eq!(verify_internal_bytes(&pk, &external_message(&ctx, &msg), &sig), want, "tcId {id}: {}", t["reason"]);
        if ctx.is_empty() {
            empty += 1;
            assert_eq!(verify_pure_bytes(&pk, &msg, &sig), want, "tcId {id}: verify_pure_bytes");
        }
        if want {
            accepted += 1;
        } else {
            refused += 1;
        }
    }
    assert!(accepted > 0 && refused > 0, "both verdicts met: {accepted} {refused}");
    assert_eq!(empty, 1, "the empty-context vector");
}

/// `M'` itself, and the ceiling of a context, pinned apart from the signature.
#[test]
fn external_message_is_the_fips_prefix() {
    assert_eq!(external_message(b"", b"m"), [0u8, 0, b'm']);
    assert_eq!(external_message(b"ab", b"m"), [0u8, 2, b'a', b'b', b'm']);
    assert_eq!(external_message(&[7u8; 255], b"").len(), 257);
}

#[test]
#[should_panic(expected = "|ctx| <= 255")]
fn a_context_over_255_bytes_is_refused() {
    let _ = external_message(&[0u8; 256], b"m");
}
