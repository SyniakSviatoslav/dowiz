//! The native reader of the verify gate (`verify.sh`): every fixture certificate reads to its line in
//! `fixtures/verify.expected`, and the fixtures' root is the root `bebop_store::merkle` builds for the
//! catalogue `oracle_verify.py --make` cut them from -- two independent constructions, one root.

use super::*;
use bebop_store::merkle::{price_leaves, root, Cert};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/fixtures/verify/{name}.cert", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

#[test]
fn verify_fixtures_read_to_expected() {
    let want = std::fs::read_to_string(format!("{}/fixtures/verify.expected", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let names = ["good", "path", "price", "short"];
    let got: Vec<String> = names.iter().map(|n| line(n, &fixture(n))).collect();
    assert_eq!(got.join("\n"), want.trim_end(), "the native reader disagrees with verify.expected");
}

#[test]
fn the_fixtures_root_is_the_catalogues() {
    let rows: Vec<(Vec<u8>, i64)> = (0..165).map(|i| (format!("p_{i:03}").into_bytes(), 100 + 7 * i as i64)).collect();
    let l = price_leaves(&rows).unwrap();
    let hs: Vec<_> = l.iter().map(|x| x.2).collect();
    assert_eq!(verify_cert(&fixture("good")), (crate::abi::OK, root(&hs)));
    assert_eq!(Cert::of(&l, 7).unwrap().encode(), fixture("good"), "the oracle's frame is byte-for-byte Rust's");
}

#[test]
fn bw_verify_writes_the_climbed_root_and_refuses_null() {
    let g = fixture("good");
    let mut out = [9u8; 32];
    let s = unsafe { bw_verify(g.as_ptr(), g.len(), out.as_mut_ptr()) };
    assert_eq!((s, out), verify_cert(&g));
    let p = fixture("path");
    assert_eq!(unsafe { bw_verify(p.as_ptr(), p.len(), out.as_mut_ptr()) }, MISMATCH);
    assert_eq!(unsafe { bw_verify(core::ptr::null(), 0, out.as_mut_ptr()) }, crate::abi::NULL_ARG);
}
