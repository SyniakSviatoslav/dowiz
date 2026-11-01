//! THE WALK BUDGET (W-MG-HOLES2): what one `verify` may read, in cells. A reader opens files from
//! strangers, and objects are data, so they can overlap or be named twice: a KEYS table with 24 000
//! rows on one 64 KiB KEY, a chain of SEGs each claiming the rest of the file, a ring of two SEALs
//! under every HEADS row. Each read alone is bounded by the image; what was not bounded is the
//! sum (`atk2_dos`: 1678x the file). The record walk has carried this rule since W-AUDIT S2.
//!
//! THE RULE: objects of an honest file are DISJOINT, so everything it reads, each object once, sums
//! to at most the image's cells, and no honest file can run this out. A file that does has named
//! something twice or laid objects over each other, and is refused BEFORE the allocation.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeSet;
use crate::Store;
use super::DocError;

pub(crate) struct Budget {
    left: usize,
}

impl Budget {
    pub(crate) fn of(st: &Store) -> Self {
        Budget { left: st.cells.len() }
    }

    /// Pay for the object at `obj`, header included.
    pub(crate) fn spend(&mut self, st: &Store, obj: usize) -> Result<(), DocError> {
        let cost = st.obj_cells(obj).saturating_add(2);
        if cost > self.left {
            return Err(DocError::Corrupt("the walks read more cells than the image holds: objects overlap"));
        }
        self.left -= cost;
        Ok(())
    }
}

/// A set built by inserting. `.collect()` into a set sorts and bulk-loads: 6.5 KB of the reader's
/// wasm (measured), for files where it wins nothing.
pub(crate) fn set_of(it: impl Iterator<Item = [u8; 32]>) -> BTreeSet<[u8; 32]> {
    let mut s = BTreeSet::new();
    for x in it {
        s.insert(x);
    }
    s
}

/// An offset as a key of the one map type the reader's wasm already pays for (`verify.rs`).
pub(crate) fn at_key(at: usize) -> [u8; 32] {
    let mut k = [0u8; 32];
    k[..8].copy_from_slice(&(at as u64).to_le_bytes());
    k
}
