//! NIST ACVP FIPS 203 ML-KEM-768 byte-exact gate (P91.2).
//!
//! Vectors: `src/pq/kat/acvp/mlkem768-{keygen,encapdecap}.json`, the ML-KEM-768
//! test groups of ACVP-Server `internalProjection.json` (provenance in
//! `README-mlkem768.md`). Every vector in both files is run:
//!
//!   * keyGen  AFT (tg 2, 25)  — `keygen_internal(d, z)` → ek AND dk byte-exact.
//!   * encapsulation AFT (tg 2, 25) — `encaps_internal(ek, m)` → c AND k byte-exact,
//!     and additionally `decaps_internal(dk, c)` → k (the group carries dk).
//!   * decapsulation VAL (tg 5, 10) — `decaps_internal(dk, c)` → k byte-exact,
//!     incl. the "modified ciphertext" cases whose k is the implicit-rejection
//!     key J(z ‖ c).
//!   * decapsulationKeyCheck VAL (tg 9, 10) — `dk_check(dk)` == testPassed.
//!   * encapsulationKeyCheck VAL (tg 10, 10) — `ek_check(ek)` == testPassed.
//!
//! A run that skips anything is a failure: each group asserts it ran exactly
//! as many vectors as the file holds AND the pinned count, so an emptied or
//! re-filtered file goes red instead of passing on zero vectors. Failures are
//! collected and the panic names every failing tcId.

#![allow(non_snake_case)]

use std::sync::OnceLock;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::pq::kem::{decaps_internal, dk_check, ek_check, encaps_internal, keygen_internal};

const KAT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/pq/kat/acvp/");

#[derive(serde::Deserialize)]
struct AcvpFile {
    testGroups: Vec<TestGroup>,
}

#[derive(serde::Deserialize)]
struct TestGroup {
    tgId: u32,
    testType: String,
    parameterSet: String,
    #[serde(default)]
    function: Option<String>,
    tests: Vec<AcvpTest>,
}

#[derive(serde::Deserialize)]
struct AcvpTest {
    tcId: u32,
    #[serde(default)]
    deferred: bool,
    d: Option<String>,
    z: Option<String>,
    ek: Option<String>,
    dk: Option<String>,
    m: Option<String>,
    c: Option<String>,
    k: Option<String>,
    testPassed: Option<bool>,
    reason: Option<String>,
}

fn load(name: &str) -> AcvpFile {
    let path = format!("{KAT_DIR}{name}");
    let s = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn keygen_file() -> &'static AcvpFile {
    static F: OnceLock<AcvpFile> = OnceLock::new();
    F.get_or_init(|| load("mlkem768-keygen.json"))
}

fn encapdecap_file() -> &'static AcvpFile {
    static F: OnceLock<AcvpFile> = OnceLock::new();
    F.get_or_init(|| load("mlkem768-encapdecap.json"))
}

fn hex(s: &Option<String>, what: &str, tc: u32) -> Vec<u8> {
    let s = s.as_deref().unwrap_or_else(|| panic!("tcId {tc}: field {what} missing"));
    assert!(s.len() % 2 == 0, "tcId {tc}: odd-length hex in {what}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn arr32(v: &[u8], what: &str, tc: u32) -> [u8; 32] {
    v.try_into().unwrap_or_else(|_| panic!("tcId {tc}: {what} is {} bytes, want 32", v.len()))
}

fn first_diff(got: &[u8], want: &[u8]) -> String {
    if got.len() != want.len() {
        return format!("len {} != {}", got.len(), want.len());
    }
    let i = got.iter().zip(want).position(|(a, b)| a != b).unwrap_or(0);
    format!("first diff at byte {i}: got {:02x} want {:02x}", got[i], want[i])
}

/// The single ML-KEM-768 group of `file` whose (testType, function) match.
/// Refuses if it is missing, duplicated, or of another parameter set.
fn group<'a>(file: &'a AcvpFile, test_type: &str, function: Option<&str>) -> &'a TestGroup {
    let hits: Vec<&TestGroup> = file
        .testGroups
        .iter()
        .filter(|g| g.testType == test_type && g.function.as_deref() == function)
        .collect();
    assert_eq!(hits.len(), 1, "want exactly one {test_type}/{function:?} group");
    assert_eq!(hits[0].parameterSet, "ML-KEM-768", "tg {} is not ML-KEM-768", hits[0].tgId);
    hits[0]
}

/// Runs `check` over every test of `g`; asserts the run count equals the
/// group's size and `pinned`, and panics naming every failing tcId.
fn run_group(g: &TestGroup, pinned: usize, check: impl Fn(&AcvpTest) -> Result<(), String>) {
    let mut ran = 0usize;
    let mut fails: Vec<String> = Vec::new();
    for t in &g.tests {
        assert!(!t.deferred, "tcId {}: deferred vector in an internalProjection", t.tcId);
        ran += 1;
        if let Err(e) = check(t) {
            fails.push(format!("tcId {} ({}): {e}", t.tcId, t.reason.as_deref().unwrap_or("-")));
        }
    }
    assert_eq!(ran, g.tests.len(), "tg {}: ran {ran} of {}", g.tgId, g.tests.len());
    assert_eq!(ran, pinned, "tg {}: file holds {ran} vectors, gate pins {pinned}", g.tgId);
    assert!(
        fails.is_empty(),
        "tg {}: {} of {ran} vectors FAILED:\n{}",
        g.tgId,
        fails.len(),
        fails.join("\n")
    );
}

fn expect(got: &[u8], want: &[u8], what: &str) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what} mismatch, {}", first_diff(got, want)))
    }
}

#[test]
fn kem_acvp_files_hold_only_the_five_768_groups() {
    let kg = keygen_file();
    let ed = encapdecap_file();
    assert_eq!(kg.testGroups.len(), 1, "keygen file: want 1 group");
    assert_eq!(ed.testGroups.len(), 4, "encapdecap file: want 4 groups");
    let total: usize = kg.testGroups.iter().chain(&ed.testGroups).map(|g| g.tests.len()).sum();
    assert_eq!(total, 80, "25 keyGen + 25 encaps + 10 decaps + 10 dk-check + 10 ek-check");
}

#[test]
fn kem_acvp_keygen() {
    run_group(group(keygen_file(), "AFT", None), 25, |t| {
        let tc = t.tcId;
        let d = arr32(&hex(&t.d, "d", tc), "d", tc);
        let z = arr32(&hex(&t.z, "z", tc), "z", tc);
        let (ek, dk) = keygen_internal(&d, &z);
        expect(&ek, &hex(&t.ek, "ek", tc), "ek")?;
        expect(&dk, &hex(&t.dk, "dk", tc), "dk")
    });
}

#[test]
fn kem_acvp_encapsulation() {
    run_group(group(encapdecap_file(), "AFT", Some("encapsulation")), 25, |t| {
        let tc = t.tcId;
        let m = arr32(&hex(&t.m, "m", tc), "m", tc);
        let want_c = hex(&t.c, "c", tc);
        let want_k = hex(&t.k, "k", tc);
        let (c, k) = encaps_internal(&hex(&t.ek, "ek", tc), &m);
        expect(&c, &want_c, "c")?;
        expect(&k, &want_k, "k")?;
        let k2 = decaps_internal(&hex(&t.dk, "dk", tc), &want_c).map_err(|e| format!("decaps refused: {e:?}"))?;
        expect(&k2, &want_k, "decaps k")
    });
}

#[test]
fn kem_acvp_decapsulation_incl_implicit_rejection() {
    let g = group(encapdecap_file(), "VAL", Some("decapsulation"));
    // The group must actually exercise implicit rejection, not only valid cts.
    let rejects = g.tests.iter().filter(|t| t.reason.as_deref() == Some("modified ciphertext")).count();
    assert!(rejects > 0, "decapsulation group has no modified-ciphertext vectors");
    run_group(g, 10, |t| {
        let tc = t.tcId;
        let k = decaps_internal(&hex(&t.dk, "dk", tc), &hex(&t.c, "c", tc))
            .map_err(|e| format!("decaps refused: {e:?}"))?;
        expect(&k, &hex(&t.k, "k", tc), "k")
    });
}

#[test]
fn kem_acvp_decapsulation_key_check() {
    let g = group(encapdecap_file(), "VAL", Some("decapsulationKeyCheck"));
    let refusals = g.tests.iter().filter(|t| t.testPassed == Some(false)).count();
    assert!(refusals > 0 && refusals < g.tests.len(), "dk-check group needs both outcomes");
    run_group(g, 10, |t| {
        let want = t.testPassed.expect("testPassed missing");
        let got = dk_check(&hex(&t.dk, "dk", t.tcId));
        if got == want { Ok(()) } else { Err(format!("dk_check = {got}, want {want}")) }
    });
}

#[test]
fn kem_acvp_encapsulation_key_check() {
    let g = group(encapdecap_file(), "VAL", Some("encapsulationKeyCheck"));
    let refusals = g.tests.iter().filter(|t| t.testPassed == Some(false)).count();
    assert!(refusals > 0 && refusals < g.tests.len(), "ek-check group needs both outcomes");
    run_group(g, 10, |t| {
        let want = t.testPassed.expect("testPassed missing");
        let got = ek_check(&hex(&t.ek, "ek", t.tcId));
        if got == want { Ok(()) } else { Err(format!("ek_check = {got}, want {want}")) }
    });
}
