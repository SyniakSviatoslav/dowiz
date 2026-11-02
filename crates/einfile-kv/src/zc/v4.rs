//! `KvIn` over a v4 image (W-ES-P8C): the constructor and the accessors `tree::kvin` reads
//! through. Split out of `zc.rs` (the 300-line rule); a child module, so it sees the fields.

use alloc::vec::Vec;

use super::{Image, KvIn};

impl<C: Image> KvIn<C> {
    /// A v4 reader over `c` (W-ES-P8C, `tree::kvin`): `n` entries, the tree under (top, depth).
    pub(crate) fn tree_parts(c: C, n: usize, top: usize, depth: usize) -> Self {
        let (chain, ov, pieces) = (Vec::new(), Vec::new(), Vec::new());
        let t4 = Some((top, depth));
        KvIn { c, ver: crate::tree::VERSION_TREE, n, kidx: 0, kblob: 0, vidx: 0, vblob: 0, kbytes: 0, vbytes: 0, sorted: true, chain, ov, pieces, mlen: 0, t4 }
    }

    /// (top, depth) when this is a v4 image. A compile-time `None` on wasm32 (`v4_here`).
    #[inline(always)]
    pub fn tree_head(&self) -> Option<(usize, usize)> {
        if crate::tree::v4_here() { self.t4 } else { None }
    }

    /// The image this reader reads.
    pub fn image(&self) -> &C {
        &self.c
    }
}
