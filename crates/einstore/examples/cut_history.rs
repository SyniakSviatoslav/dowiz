// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: history leaves a file only under a signed declaration. A cut moves the oldest records
//! out under a Merkle root. Until a device commits the TRUNCATE naming that root the file is
//! refused by name, and the old file, kept as the archive, recomputes the root.
//! Run: `cargo run --example cut_history`
//!
//! `AnyoneCanForge` is not a signature (`shared/mod.rs`).

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{verify, Doc, DocError, Ids, Trust, FLAG_SHA256_IDS};
use einstore::evlog::EvLog;
use einstore::Store;
use shared::{AnyoneCanForge, Device, Sha256Ids};

fn main() -> Result<(), DocError> {
    let mut old = Store::create_bytes(1 << 20)?;
    let mut till = Device::new(1);
    Doc::init_bytes(&mut old, FLAG_SHA256_IDS, &[till.key()])?;
    for day in 1..=6 {
        till.commit(&mut old, format!("day {day}").as_bytes(), None)?;
    }

    // Keep the two newest records; the four oldest leave under a segment.
    let mut fresh = Store::create_bytes(1 << 20)?;
    let segment = Doc::cut_into(&old, &mut fresh, 2, &Sha256Ids)?;

    let undeclared = verify(&fresh, &Sha256Ids, &AnyoneCanForge, Trust::Genesis);
    assert_eq!(undeclared, Err(DocError::CutUndeclared { root: segment.root }));
    println!("cut, not declared: refused");

    till.commit(&mut fresh, &segment.truncate_body(), None)?;
    let r = verify(&fresh, &Sha256Ids, &AnyoneCanForge, Trust::Genesis)?;
    assert!(r.holds(), "{:?}", r.counters);
    assert_eq!((r.counters.records, r.counters.cut), (3, 4), "{:?}", r.counters);
    println!("cut, declared: {} records kept, {} under the segment", r.counters.records, r.counters.cut);

    // The archive answers for what left: its ids, oldest first, give the segment's root.
    let mut archived = EvLog::walk_until_at(&old, Doc::open(&old)?.log, |_| false);
    archived.reverse();
    let ids: Vec<[u8; 32]> = archived[..segment.count() as usize].iter().map(|r| r.id).collect();
    assert_eq!(Sha256Ids.tree_root(&ids), Some(segment.root));
    println!("the archive recomputes the root");
    Ok(())
}
