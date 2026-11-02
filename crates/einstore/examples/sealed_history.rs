// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE (level 0): an edit under a seal is seen. Each device commit is one record and one seal
//! over the device's chain tip, so `verify` reports a changed record whether or not the forger
//! repaired the crc.
//! Run: `cargo run --example sealed_history [out.ein]`
//!
//! `AnyoneCanForge` is not a signature (`shared/mod.rs`). What this example proves is the layout
//! and the counters. With a real ML-DSA-65 key behind the same trait a seal also says who signed.

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{verify, Doc, DocError, Trust, FLAG_SHA256_IDS};
use einstore::evlog::EvLog;
use einstore::Store;
use shared::{AnyoneCanForge, Device, Sha256Ids};

fn main() -> Result<(), DocError> {
    let mut st = Store::create_bytes(1 << 20)?;
    let mut till = Device::new(1);
    Doc::init_bytes(&mut st, FLAG_SHA256_IDS, &[till.key()])?;
    till.commit(&mut st, b"sold 2 espresso", None)?;
    till.commit(&mut st, b"sold 1 cake", None)?;
    till.commit(&mut st, b"refund 1 cake", None)?;

    let clean = verify(&st, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert!(clean.holds(), "{:?}", clean.counters);
    println!("clean file:  {:?}", clean.counters);

    let bytes = st.to_bytes_trimmed();
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(&path, &bytes).expect("the path is writable");
        println!("wrote {path}; read it with tools/verify/ein-verify.mjs");
    }

    // A forger who does not know the format changes a payload cell: the crc refuses the file.
    let mut edited = Store::from_bytes(&bytes);
    let newest = EvLog::objs_at(&edited, Doc::open(&edited)?.log)[0];
    let first_payload_cell = 16; // 12 header cells, 4 for the actor
    edited.put_cell(newest, first_payload_cell, edited.get(newest, first_payload_cell) ^ 1);
    match verify(&edited, &Sha256Ids, &AnyoneCanForge, Trust::Genesis) {
        Err(DocError::BadCrc(b)) => println!("edit, crc left alone: refused, {b:?}"),
        other => panic!("expected a crc refusal, got {other:?}"),
    }

    // A forger who knows the format re-seals the crc: the record no longer matches its id.
    edited.seal(newest);
    let forged = verify(&edited, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert_eq!(forged.counters.broken, 1, "{:?}", forged.counters);
    assert!(!forged.holds());
    println!("edit, crc repaired:  broken = {}, holds = {}", forged.counters.broken, forged.holds());
    Ok(())
}
