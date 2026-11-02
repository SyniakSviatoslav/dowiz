// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE (level 1), and its limit: a file can carry evidence from outside itself that a record
//! existed by some moment. An anchor nobody checked counts as nothing. This example talks to no
//! calendar and no blockchain, so the only verdict it can reach for its proof is `Unchecked`.
//! Run: `cargo run --example time_anchor`
//!
//! The MIT crates store a proof and never interpret it. A verifier supplies an `AnchorCheck`. An
//! OpenTimestamps proof says something only after its Bitcoin transaction has a block and the
//! auditor compares that block's header. Until then its right verdict is `Unchecked`.

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::anch::NoAdapters;
use einstore::evlog::doc::{check_anchors, AnchorCounts, Doc, DocError, FLAG_SHA256_IDS, KIND_OTS};
use einstore::Store;
use shared::Device;

fn main() -> Result<(), DocError> {
    let mut st = Store::create_bytes(1 << 20)?;
    let mut till = Device::new(1);
    Doc::init_bytes(&mut st, FLAG_SHA256_IDS, &[till.key()])?;
    till.commit(&mut st, b"closed the day", None)?;

    // In real use the proof is the `.ots` file for this digest. These bytes are a placeholder.
    let tip = till.tip;
    Doc::anchor_bytes(&mut st, KIND_OTS, &tip, 1_700_000_000_000, b"placeholder, not a proof")?;

    let checked = check_anchors(&st, &NoAdapters)?;
    let counts = AnchorCounts::of(&checked);
    assert_eq!((counts.anchors, counts.ok, counts.unchecked), (1, 0, 1), "{counts:?}");
    println!("over a record of this file, no adapter: {:?}", checked[0].verdict);

    // An anchor over nothing in the file binds nothing: bad before any adapter runs.
    Doc::anchor_bytes(&mut st, KIND_OTS, &[0xEE; 32], 1_700_000_000_000, b"placeholder")?;
    let counts = AnchorCounts::of(&check_anchors(&st, &NoAdapters)?);
    assert_eq!((counts.anchors, counts.ok, counts.bad), (2, 0, 1), "{counts:?}");
    println!("over no record of this file: bad = {}", counts.bad);
    Ok(())
}
