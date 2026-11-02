// SPDX-License-Identifier: MIT OR Apache-2.0
//! NIST ACVP, ML-DSA-65, external pure: 30 sigGen and 15 sigVer vectors, shared with dowiz-core's
//! `dsa_acvp_external_tests.rs` (provenance in that header). Forty-two of them carry a non-empty
//! context this crate never uses and all of them a fixed secret key `DeviceKey` cannot be built
//! from, so the library is asked directly for the bytes, and `MlDsa65` for each empty-context case.

use einfile_log::doc::{Verifier as _, ALG_ML_DSA_65};
use einfile_sig_mldsa::{MlDsa65, PK_BYTES, SIG_BYTES};
use fips204::ml_dsa_65::{PrivateKey, PublicKey, SK_LEN};
use fips204::traits::{SerDes as _, Signer as _, Verifier as _};
use serde_json::Value;

fn tests_of(file: &str) -> Vec<(Value, Value)> {
    let p = format!("{}/../dowiz-core/src/pq/kat/acvp/{file}", env!("CARGO_MANIFEST_DIR"));
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&p).expect("vendored ACVP file")).expect("json");
    let mut out = Vec::new();
    for g in d["testGroups"].as_array().expect("groups") {
        assert_eq!((g["parameterSet"].as_str(), g["signatureInterface"].as_str(), g["preHash"].as_str()), (Some("ML-DSA-65"), Some("external"), Some("pure")));
        for t in g["tests"].as_array().expect("tests") {
            out.push((g.clone(), t.clone()));
        }
    }
    out
}

fn hex(v: &Value, field: &str) -> Vec<u8> {
    let s = v[field].as_str().unwrap_or_else(|| panic!("no field {field}"));
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex")).collect()
}

fn pk_of(t: &Value) -> PublicKey {
    let b: [u8; PK_BYTES] = hex(t, "pk").try_into().expect("pk length");
    PublicKey::try_from_bytes(b).expect("a vector's key parses")
}

fn sig_of(t: &Value) -> Result<[u8; SIG_BYTES], Vec<u8>> {
    hex(t, "signature").try_into()
}

#[test]
fn siggen_is_byte_exact() {
    let all = tests_of("sig-gen-external.json");
    assert_eq!(all.len(), 30);
    let mut empty = 0;
    for (g, t) in &all {
        let id = &t["tcId"];
        let sk: [u8; SK_LEN] = hex(t, "sk").try_into().expect("sk length");
        let sk = PrivateKey::try_from_bytes(sk).expect("a vector's key parses");
        let mut rnd = [0u8; 32];
        if g["deterministic"].as_bool() == Some(false) {
            rnd.copy_from_slice(&hex(t, "rnd"));
        }
        let (msg, ctx) = (hex(t, "message"), hex(t, "context"));
        let want = sig_of(t).expect("sig length");
        assert_eq!(sk.try_sign_with_seed(&rnd, &msg, &ctx).expect("signs"), want, "tcId {id}");
        assert!(pk_of(t).verify(&msg, &want, &ctx), "tcId {id}: the positive twin");
        if ctx.is_empty() {
            empty += 1;
            assert!(MlDsa65.verify(ALG_ML_DSA_65, &hex(t, "pk"), &msg, &want), "tcId {id}: MlDsa65");
        }
    }
    assert_eq!(empty, 2, "the empty-context vectors, or MlDsa65 met none");
}

#[test]
fn sigver_agrees_on_every_verdict() {
    let all = tests_of("sig-ver-external.json");
    assert_eq!(all.len(), 15);
    let (mut accepted, mut refused, mut empty) = (0, 0, 0);
    for (_, t) in &all {
        let (id, msg, ctx) = (&t["tcId"], hex(t, "message"), hex(t, "context"));
        let want = t["testPassed"].as_bool().expect("testPassed");
        let got = sig_of(t).is_ok_and(|sig| pk_of(t).verify(&msg, &sig, &ctx));
        assert_eq!(got, want, "tcId {id}: {}", t["reason"]);
        if ctx.is_empty() {
            empty += 1;
            assert_eq!(MlDsa65.verify(ALG_ML_DSA_65, &hex(t, "pk"), &msg, &hex(t, "signature")), want, "tcId {id}: MlDsa65");
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
