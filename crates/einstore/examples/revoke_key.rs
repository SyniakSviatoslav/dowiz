// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE (level 0): keys are records. A device key is admitted by a record another admitted key
//! wrote, and a revoke after seq k makes every seal of that key past k invalid.
//! Run: `cargo run --example revoke_key`
//!
//! `AnyoneCanForge` is not a signature (`shared/mod.rs`).

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{verify, Doc, DocError, Key, KeyOp, Trust, FLAG_SHA256_IDS};
use einstore::Store;
use shared::{AnyoneCanForge, Device, Sha256Ids};

fn main() -> Result<(), DocError> {
    let mut st = Store::create_bytes(1 << 20)?;
    let mut owner = Device::new(1);
    let mut tablet = Device::new(2);
    Doc::init_bytes(&mut st, FLAG_SHA256_IDS, &[owner.key()])?;

    let admit = KeyOp::Admit(Key { valid_from: 1, ..tablet.key() });
    owner.commit(&mut st, &admit.body(), Some(&admit))?;
    tablet.commit(&mut st, b"tablet record 1", None)?;
    tablet.commit(&mut st, b"tablet record 2", None)?;
    let r = verify(&st, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad, r.admitted), (3, 0, 2), "{r:?}");
    println!("admitted: seals_ok = {}, seals_bad = {}", r.counters.seals_ok, r.counters.seals_bad);

    // The tablet is lost after its first record.
    let id = tablet.key().actor_id;
    let revoke = KeyOp::Revoke { actor_id: id, after_seq: 1, tip: Doc::tip_at(&st, &id, 1).expect("the tablet's first record") };
    owner.commit(&mut st, &revoke.body(), Some(&revoke))?;
    let r = verify(&st, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad), (3, 1), "{r:?}");
    assert!(!r.holds(), "the file now names what the lost key wrote past its revoke");
    println!("revoked:  seals_ok = {}, seals_bad = {}", r.counters.seals_ok, r.counters.seals_bad);

    // The writer refuses the revoked key too.
    assert_eq!(tablet.commit(&mut st, b"tablet record 3", None), Err(DocError::Revoked { after_seq: 1, seq: 3 }));
    println!("the writer refuses the revoked key");
    Ok(())
}
