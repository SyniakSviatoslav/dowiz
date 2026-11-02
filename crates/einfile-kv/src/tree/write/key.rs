//! Keys BY REFERENCE for the path copy (W-ES-P19, step 1 of the many-writer pass): a slot's key
//! is either the bytes of an op (borrowed for the commit's whole life) or an address inside a
//! node the commit read -- never a `Vec<u8>` of its own. Before this a 4-key commit at 1e4 made
//! ~250 small allocations (32 per node read, one per op, one per piece's first key) and cost 42
//! us; by reference it costs 19 us (p19proto `P4_NOPUB` control, 2026-10-30). The bulk loader
//! (`build.rs`) keeps owned keys: `KeyBytes` lets both shapes through one `write_node`/`finish`.

use alloc::vec::Vec;
use core::cmp::Ordering;

use super::super::node::{cmp_at, key_at, ref_at, NodeRef};
use super::super::TreeError;
use crate::Store;

/// A key the writer holds without copying it.
#[derive(Clone, Copy)]
pub(crate) enum Key<'o> {
    /// `len` bytes at absolute byte address `addr` of the image (inside a node's key area).
    Img { addr: usize, len: usize },
    /// An op's key.
    Op(&'o [u8]),
}

impl Key<'_> {
    /// This key against `probe`, read from `st` where it lives there.
    pub(crate) fn cmp(&self, st: &Store, probe: &[u8]) -> Ordering {
        match *self {
            Key::Img { addr, len } => cmp_at(st, addr, len, probe),
            Key::Op(k) => k.cmp(probe),
        }
    }
}

/// What `write_node` needs of a key: its bytes appended to a buffer, its length back.
pub(crate) trait KeyBytes: Clone {
    fn push_to(&self, st: &Store, out: &mut Vec<u8>) -> usize;
}

impl KeyBytes for Vec<u8> {
    fn push_to(&self, _st: &Store, out: &mut Vec<u8>) -> usize {
        out.extend_from_slice(self);
        self.len()
    }
}

impl KeyBytes for Key<'_> {
    fn push_to(&self, st: &Store, out: &mut Vec<u8>) -> usize {
        match *self {
            Key::Op(k) => {
                out.extend_from_slice(k);
                k.len()
            }
            Key::Img { addr, len } => {
                // a cell at a time, never a byte at a time (node.rs `gather`'s lesson)
                let (mut b, end) = (addr, addr + len);
                while b < end {
                    let cell = st.cells.get(b / 8).copied().unwrap_or(0).to_le_bytes();
                    let take = (8 - b % 8).min(end - b);
                    out.extend_from_slice(cell.get(b % 8..b % 8 + take).unwrap_or_default());
                    b += take;
                }
                len
            }
        }
    }
}

/// Every slot of a node as (key by reference, absolute ref), the layout checked as `items` checks
/// it except the key ORDER (keys are not read here; `verify` and the reader's `search` hold the
/// order rule, and a commit over an unordered node is refused by `verify` on the next open).
pub(crate) fn slots<'o>(st: &Store, nd: &NodeRef) -> Result<Vec<(Key<'o>, usize)>, TreeError> {
    let mut out = Vec::with_capacity(nd.m);
    let base = (nd.obj + 2 + nd.karea) * 8;
    let mut used = 0usize;
    for i in 0..nd.m {
        let (addr, len) = key_at(st, nd, i)?;
        if addr != base + used {
            return Err(TreeError::Node { obj: nd.obj, why: "keys not packed in slot order" });
        }
        used += len;
        out.push((Key::Img { addr, len }, ref_at(st, nd, i)?));
    }
    if nd.len != nd.karea + used.div_ceil(8) {
        return Err(TreeError::Node { obj: nd.obj, why: "length is not exact" });
    }
    Ok(out)
}
