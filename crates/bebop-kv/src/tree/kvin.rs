//! P8c: the library's readers over a v4 image. `KvIn` (the zero-copy reader every caller holds)
//! and `Kv::load_checked` read a v4 KV root through the tree; a v1-v3 image is read exactly as
//! before.
//!
//! WHERE v4 IS READ, AND WHERE IT IS NOT (yet): only a native build with the cargo feature `tree`
//! (off by default; bebop-io's `tree` turns it on) reads it. Every other build keeps REFUSING it
//! (`UnknownVersion`, W-ES-P8A) until P8d teaches the wasm, python, bebop and C-ABI readers: all
//! five readers must answer one image the same way (W-ES-P4's conformance.sh). `v4_here()` is a
//! compile-time `false` there, so no v4 code reaches those builds.
//!
//! WHAT A v4 `KvIn` DOES NOT DO: the merged-index API (`key(i)`, `value(i)`, `seek`,
//! `prefix_range`). A COW tree keeps no subtree counts, so index i is an O(i) walk; it is refused
//! by name (`TreeError::Unindexed`) instead of being slow without saying so. Scans go through
//! `KvIn::scan` / `Staged::cursor`, which walk the tree in order.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Bound;

use super::iter::{range_in, Iter};
use super::read::{tree_get, TreeIn};
use super::{fnv1a, TreeError};
use crate::verify::KvError;
use crate::zc::{Image, KvIn};
use crate::{Kv, Store, FNV_OFFSET};

/// Does this build read v4? Only with the `tree` feature (off by default) and never on wasm32, so
/// the v4 arms compile out of every default and every wasm build (see the module).
#[inline(always)]
pub const fn v4_here() -> bool {
    cfg!(all(feature = "tree", not(target_arch = "wasm32")))
}

impl<C: Image> KvIn<C> {
    /// A v4 image opened at `root`. `full`: `TreeIn::verify` (every reachable crc, the order,
    /// the count) -- what `open` proves. Otherwise the root's own shape, O(1): `reopen` and
    /// `open_lazy`, whose contract (a wrong trust gives a wrong answer, never a panic) the tree
    /// reader keeps (every cell it touches is bounded).
    pub(crate) fn open_tree(c: C, root: usize, full: bool) -> Result<Self, KvError> {
        let t = TreeIn::at(c, root).map_err(KvError::Tree)?;
        if full {
            t.verify().map_err(KvError::Tree)?;
        }
        let n = t.len();
        let (top, depth) = t.head();
        Ok(KvIn::tree_parts(t.into_image(), n, top, depth))
    }

    /// v4 `get`: one node per level, then the value, borrowed where the image is bytes.
    pub(crate) fn tree_value(&self, top: usize, depth: usize, key: &[u8]) -> Result<Option<Cow<'_, [u8]>>, KvError> {
        tree_get(self.image(), top, depth, key).map(|o| o.map(crate::Bytes::into_cow)).map_err(KvError::Tree)
    }

    /// v4 fold: `Kv::snapshot_root_u64` over the entries in key order.
    pub(crate) fn tree_fold(&self, top: usize, depth: usize) -> Result<u64, KvError> {
        let mut h = FNV_OFFSET;
        for e in range_in(self.image(), top, depth, Bound::Unbounded, Bound::Unbounded) {
            let (k, v) = e.map_err(KvError::Tree)?;
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, &k);
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, &v);
        }
        Ok(h)
    }

    /// The ordered scan of a v4 image from `lo` to `hi`; `None` for a v1-v3 image (whose callers
    /// use the index API). `MergeCursor` walks a v4 base through this.
    pub fn tree_scan<'k>(&self, lo: Bound<&[u8]>, hi: Bound<&'k [u8]>) -> Option<Iter<'_, 'k, C>> {
        let (top, depth) = self.tree_head()?;
        Some(range_in(self.image(), top, depth, lo, hi))
    }
}

/// The index API on a v4 image: refused by name (see the module).
pub(crate) const UNINDEXED: KvError = KvError::Tree(TreeError::Unindexed);

/// `Kv::load_checked`'s v4 half: the full check (`TreeIn::verify`), then every entry. Keys are
/// decoded as `Kv::decode` decodes them (lossy UTF-8; no writer here makes another).
pub(crate) fn load_tree(st: &Store, root: usize) -> Result<Kv, KvError> {
    let t = TreeIn::at(st, root).map_err(KvError::Tree)?;
    t.verify().map_err(KvError::Tree)?;
    let mut entries = Vec::with_capacity(t.len());
    for e in t.iter() {
        let (k, v) = e.map_err(KvError::Tree)?;
        entries.push((String::from_utf8_lossy(&k).into_owned(), v.into_owned()));
    }
    Ok(Kv { entries })
}
