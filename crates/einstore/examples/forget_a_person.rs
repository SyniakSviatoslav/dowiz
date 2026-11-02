// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: a record can be erased without hiding that something was erased. The payload goes, its
//! id and place in the chain stay, the seals still hold, and a signed FORGOTTEN declares how many.
//! A tombstone nobody declared is reported.
//! Run: `cargo run --example forget_a_person`
//!
//! The nonce goes with the payload so that nobody can test a guess against the id that remains.
//! Whether this meets a legal duty to erase is not a question this library answers.
//! `AnyoneCanForge` is not a signature (`shared/mod.rs`).

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{forgotten_body, verify, Doc, DocError, Trust, FLAG_SHA256_IDS};
use einstore::evlog::Record;
use einstore::Store;
use shared::{AnyoneCanForge, Device, Sha256Ids, ERASED_NONCE, TOMBSTONE_BIT};

fn main() -> Result<(), DocError> {
    let mut old = Store::create_bytes(1 << 20)?;
    let mut till = Device::new(1);
    Doc::init_bytes(&mut old, FLAG_SHA256_IDS, &[till.key()])?;
    till.commit(&mut old, b"delivery to +355 00 000 0000", None)?;
    till.commit(&mut old, b"sold 1 cake", None)?;

    let erase = |r: &Record| r.payload.windows(4).any(|w| w == b"+355").then(|| [&ERASED_NONCE[..], &[TOMBSTONE_BIT]].concat());
    let mut fresh = Store::create_bytes(1 << 20)?;
    let erased = Doc::redact_into(&old, &mut fresh, &Sha256Ids, &erase)?;
    assert_eq!(erased.len(), 1);

    let r = verify(&fresh, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert_eq!((r.counters.redacted, r.counters.declared), (1, 0), "{:?}", r.counters);
    assert!(!r.holds(), "a tombstone nobody declared is reported");
    println!("erased, not declared: holds = {}", r.holds());

    // `subject` is a hash the application chooses. It must not be the person's identifier.
    till.commit(&mut fresh, &forgotten_body(&[0x5A; 32], &erased), None)?;
    let r = verify(&fresh, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert!(r.holds(), "{:?}", r.counters);
    println!("erased, declared:     redacted = {}, declared = {}", r.counters.redacted, r.counters.declared);
    Ok(())
}
