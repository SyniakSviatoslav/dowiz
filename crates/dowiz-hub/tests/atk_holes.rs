//! W-MG-ATK: THE HOLES. Each test states the WANTED assertion: the forged file must not hold.
//! RED ON PURPOSE until `verify` closes it; none of these may be `#[ignore]`d into a green suite.
//! Every forgery below is written with no private key of the file (`atk_forge`), `Trust::Only`,
//! real ML-DSA-65. The first assertion of each test is the control: what the file says unforged.
mod atk_forge;

#[path = "atk_holes/forger.rs"]
mod forger;
#[path = "atk_holes/segments.rs"]
mod segments;
#[path = "atk_holes/unbound.rs"]
mod unbound;

use atk_forge::*;
use dowiz_hub::doc_seal::{DeviceKey, SealedHub};
use dowiz_hub::EventKind;

pub const MOVE: &str = r#"{"item":"salmon","delta":-1,"unit":"portion"}"#;

pub fn key(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}

/// `n` plain orders of `key` from tag `tag`.
pub fn orders(sh: &mut SealedHub, key: &DeviceKey, tag: &str, n: u8) {
    (0..n).for_each(|i| drop(sh.append(EventKind::Placed, &format!("{tag}{i}"), MOVE, key, i as i64 + 1).unwrap()));
}

/// The wanted assertion, one line: the counters are printed above it.
pub fn must_not_hold(name: &str, bytes: &[u8], roots: &[Id]) {
    match only(name, bytes, roots) {
        Ok(r) => assert!(!r.holds(), "HOLE {name}: a forger with no key got holds=true: {:?}", r.counters),
        Err(_) => {}
    }
}
