//! NIST ACVP FIPS204 ML-DSA-65 byte-exact property-gate.
//!
//! Parses the vendored official NIST ACVP known-answer vectors
//! (`bebop2/core/kat/acvp/{key-gen,sig-gen,sig-ver}.json`, vsId 42,
//! revision FIPS204, isSample=false) — the canonical NIST ACVP-Server export
//! mirrored by RustCrypto/signatures `ml-dsa/tests` — and asserts byte-exact
//! agreement with the from-scratch implementation in `pq_dsa`:
//!
//!   * keyGen — `keygen_bytes(seed)` pk AND sk must equal the expected bytes.
//!   * sigGen — `sign_internal_bytes(sk, msg, rnd)` signature must equal expected.
//!     Groups without an `rnd` field are FIPS deterministic mode → rnd = 0^32.
//!   * sigVer — `verify_internal_bytes(pk, msg, signature)` must equal
//!     `testPassed` (valid / invalid incl. malformed + too-many-hints).
//!
//! Every ACVP test case is a discrete `#[test]` (one per tcId) so a failure
//! names the exact vector. The 2.5 MB JSON is parsed exactly once and cached.
//!
//! dev-dependencies `serde` (derive) + `serde_json` are used ONLY here (tests).

#![allow(dead_code)]

use std::sync::OnceLock;

use std::vec::Vec;

use crate::pq::dsa::{keygen_bytes, sign_internal_bytes, verify_internal_bytes, SEEDBYTES};

const KAT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/pq/kat/acvp/");

#[derive(serde::Deserialize)]
struct AcvpFile {
    testGroups: Vec<TestGroup>,
}

#[derive(serde::Deserialize)]
struct TestGroup {
    parameterSet: String,
    #[serde(default)]
    pk: Option<String>,
    #[serde(default)]
    sk: Option<String>,
    tests: Vec<AcvpTest>,
}

#[derive(serde::Deserialize)]
struct AcvpTest {
    tcId: u32,
    #[serde(default)]
    seed: Option<String>,
    #[serde(default)]
    pk: Option<String>,
    #[serde(default)]
    sk: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    rnd: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    testPassed: Option<bool>,
}

struct Files {
    kg: AcvpFile,
    sg: AcvpFile,
    sv: AcvpFile,
}

fn cached_files() -> &'static Files {
    static CACHE: OnceLock<Files> = OnceLock::new();
    CACHE.get_or_init(|| {
        let kg = std::fs::read_to_string(format!("{}key-gen.json", KAT_DIR)).unwrap();
        let sg = std::fs::read_to_string(format!("{}sig-gen.json", KAT_DIR)).unwrap();
        let sv = std::fs::read_to_string(format!("{}sig-ver.json", KAT_DIR)).unwrap();
        Files {
            kg: serde_json::from_str(&kg).unwrap(),
            sg: serde_json::from_str(&sg).unwrap(),
            sv: serde_json::from_str(&sv).unwrap(),
        }
    })
}

fn hex(s: &str) -> Vec<u8> {
    let s = s.trim();
    assert!(s.len() % 2 == 0, "odd hex len for {}", s);
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..bytes.len()).step_by(2) {
        let hi = (bytes[i] as char).to_digit(16).unwrap();
        let lo = (bytes[i + 1] as char).to_digit(16).unwrap();
        out.push((hi * 16 + lo) as u8);
    }
    out
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    if a.len() != b.len() {
        return format!("len mismatch: got {} want {}", a.len(), b.len());
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            let s = i.saturating_sub(8);
            let e = (i + 16).min(a.len());
            return format!(
                "first byte diff at offset {}: got {:02x} want {:02x} | window got={} want={}",
                i,
                a[i],
                b[i],
                a[s..e]
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(""),
                b[s..e]
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join("")
            );
        }
    }
    "no diff".to_string()
}

/// The 25 ML-DSA-65 keyGen test cases: (tcId, seed, exp_pk, exp_sk).
fn keygen_cases() -> Vec<(u32, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let f = cached_files();
    let mut out = Vec::new();
    for g in &f.kg.testGroups {
        if g.parameterSet != "ML-DSA-65" {
            continue;
        }
        for t in &g.tests {
            out.push((
                t.tcId,
                hex(t.seed.as_ref().unwrap()),
                hex(t.pk.as_ref().unwrap()),
                hex(t.sk.as_ref().unwrap()),
            ));
        }
    }
    out
}

/// The 20 ML-DSA-65 sigGen test cases: (tcId, sk, msg, rnd_or_none, exp_sig).
fn siggen_cases() -> Vec<(u32, Vec<u8>, Vec<u8>, Option<Vec<u8>>, Vec<u8>)> {
    let f = cached_files();
    let mut out = Vec::new();
    for g in &f.sg.testGroups {
        if g.parameterSet != "ML-DSA-65" {
            continue;
        }
        for t in &g.tests {
            let rnd = t.rnd.as_ref().map(|r| hex(r));
            out.push((
                t.tcId,
                hex(t.sk.as_ref().unwrap()),
                match t.message.as_ref() {
                    Some(m) => hex(m),
                    None => Vec::new(),
                },
                rnd,
                hex(t.signature.as_ref().unwrap()),
            ));
        }
    }
    out
}

/// The 15 ML-DSA-65 sigVer test cases: (tcId, pk, msg, sig, want_valid).
fn sigver_cases() -> Vec<(u32, Vec<u8>, Vec<u8>, Vec<u8>, bool)> {
    let f = cached_files();
    let mut out = Vec::new();
    for g in &f.sv.testGroups {
        if g.parameterSet != "ML-DSA-65" {
            continue;
        }
        let pk = hex(g.pk.as_ref().unwrap());
        for t in &g.tests {
            out.push((
                t.tcId,
                pk.clone(),
                match t.message.as_ref() {
                    Some(m) => hex(m),
                    None => Vec::new(),
                },
                hex(t.signature.as_ref().unwrap()),
                t.testPassed.unwrap(),
            ));
        }
    }
    out
}

#[cfg(test)]
mod generated {
    use super::*;

    // One discrete #[test] per ACVP tcId. Names are synthesized with the
    // dev-only `paste` crate (`paste::item!`), keeping the production crate
    // dependency-free.

    // --- keyGen (25) ---
    macro_rules! kg_case {
        ($($tc:literal),* $(,)?) => {
            $(
                paste::item! {
                    #[test]
                    fn [<acvp_keygen_ $tc>]() {
                        let cases = keygen_cases();
                        let (tc, seed, exp_pk, exp_sk) =
                            cases.into_iter().find(|c| c.0 == $tc).unwrap();
                        assert_eq!(seed.len(), SEEDBYTES, "seed len tcId {}", tc);
                        let mut s = [0u8; SEEDBYTES];
                        s.copy_from_slice(&seed);
                        let (pk, sk) = keygen_bytes(&s);
                        assert_eq!(
                            pk, exp_pk,
                            "keyGen pk mismatch tcId {} — {}",
                            tc, first_diff(&pk, &exp_pk)
                        );
                        assert_eq!(
                            sk, exp_sk,
                            "keyGen sk mismatch tcId {} — {}",
                            tc, first_diff(&sk, &exp_sk)
                        );
                    }
                }
            )*
        };
    }

    macro_rules! sg_case {
        ($($tc:literal),* $(,)?) => {
            $(
                paste::item! {
                    #[test]
                    fn [<acvp_siggen_ $tc>]() {
                        let cases = siggen_cases();
                        let (tc, sk, msg, rnd, exp_sig) =
                            cases.into_iter().find(|c| c.0 == $tc).unwrap();
                        let r = match rnd {
                            Some(v) => {
                                assert_eq!(v.len(), 32, "rnd len tcId {}", tc);
                                let mut b = [0u8; 32];
                                b.copy_from_slice(&v);
                                b
                            }
                            None => [0u8; 32],
                        };
                        let sig = sign_internal_bytes(&sk, &msg, &r);
                        assert_eq!(
                            sig, exp_sig,
                            "sigGen signature mismatch tcId {} — {}",
                            tc, first_diff(&sig, &exp_sig)
                        );
                    }
                }
            )*
        };
    }

    macro_rules! sv_case {
        ($($tc:literal),* $(,)?) => {
            $(
                paste::item! {
                    #[test]
                    fn [<acvp_sigver_ $tc>]() {
                        let cases = sigver_cases();
                        let (tc, pk, msg, sig, want) =
                            cases.into_iter().find(|c| c.0 == $tc).unwrap();
                        // `want` is taken directly from the NIST ACVP `testPassed`
                        // field (parsed by sigver_cases) — never a hand-hardcoded
                        // expectation, so a typo can't flip a valid vector to "fail".
                        let got = verify_internal_bytes(&pk, &msg, &sig);
                        assert_eq!(
                            got, want,
                            "sigVer mismatch tcId {} (ACVP testPassed={})",
                            tc, want
                        );
                    }
                }
            )*
        };
    }

    kg_case!(
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
        49, 50
    );

    sg_case!(21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40);

    sv_case!(16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30);
}

// ── Aggregate guards (prove the exact counts are exercised) ──────────────────
#[test]
fn acvp_mldsa65_keygen_count() {
    assert_eq!(
        keygen_cases().len(),
        25,
        "expected 25 ML-DSA-65 keyGen cases"
    );
}
#[test]
fn acvp_mldsa65_siggen_count() {
    assert_eq!(
        siggen_cases().len(),
        20,
        "expected 20 ML-DSA-65 sigGen cases"
    );
}
#[test]
fn acvp_mldsa65_sigver_count() {
    assert_eq!(
        sigver_cases().len(),
        15,
        "expected 15 ML-DSA-65 sigVer cases"
    );
}

#[cfg(test)]
mod diag {
    use super::*;
    use crate::pq::dsa::{unpack_pk_bytes, unpack_sig_bytes};

    fn hx(b: &[u8]) -> String {
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[test]
    fn diag_tc20() {
        let cases = sigver_cases();
        let cases: Vec<_> = cases.into_iter().collect();
        for tc in [20u32, 21u32] {
            let (_, pk, msg, sig, want) = cases.iter().find(|c| c.0 == tc).unwrap();
            std::eprintln!("=== tcId {} want={}", tc, want);
            let (c_tilde, _z, _h) = unpack_sig_bytes(sig).unwrap();
            std::eprintln!("ctilde={}", hx(c_tilde.as_slice()));
            let (rho, _t1) = unpack_pk_bytes(&pk);
            std::eprintln!("rho={}", hx(&rho));
            let got = verify_internal_bytes(&pk, &msg, &sig);
            std::eprintln!("got={} want={}", got, want);
            std::eprintln!("msg={}", hx(&msg));
        }
    }
}
