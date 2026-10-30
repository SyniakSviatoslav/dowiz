//! The v4 READER: `TreeIn` over any `Image` -- get, ordered scans, the fold, and the full verify.
//!
//! `at` is O(1) (the root's own cells); `get` reads one node per level and the value; a scan is a
//! fixed `[(node, idx); MAX_TREE_DEPTH]` stack, no Vec (`iter.rs`; COW trees have no sibling links:
//! changing a leaf would force rewriting its predecessor).

use alloc::vec::Vec;
use core::ops::Bound;

use super::iter::Iter;
use super::node::{bytes_of, child_for, items, node_at, ref_at, search, val_at};
use super::{fnv1a, TreeError, MAX_TREE_DEPTH, ROOT_V4, VERSION_TREE};
use crate::bytes::Bytes;
use crate::zc::{check_obj_in, Image};
use crate::{get_in, obj_digest_in, obj_len_in, root_in, DIGEST_KV_ROOT, FNV_OFFSET};

/// `TreeIn::get` over the tree under (`top`, `depth`) in `c`: one node per level, then the value.
pub(crate) fn tree_get<'c, C: Image>(c: &'c C, top: usize, depth: usize, key: &[u8]) -> Result<Option<Bytes<'c>>, TreeError> {
    let mut obj = top;
    for level in (1..=depth).rev() {
        let nd = node_at(c, obj, level == 1)?;
        if level > 1 {
            obj = ref_at(c, &nd, child_for(c, &nd, key)?)?;
            continue;
        }
        let (i, eq) = search(c, &nd, key)?;
        if !eq {
            return Ok(None);
        }
        let (a, l) = val_at(c, ref_at(c, &nd, i)?)?;
        return Ok(Some(bytes_of(c, a, l)));
    }
    Ok(None)
}

/// A v4 tree opened in place at one KV root.
pub struct TreeIn<C: Image> {
    pub(super) c: C,
    root: usize,
    n: usize,
    pub(super) top: usize,
    pub(super) depth: usize,
}

impl<C: Image> TreeIn<C> {
    /// The tree the live superblock names.
    pub fn open(c: C) -> Result<Self, TreeError> {
        let r = root_in(&c).ok_or(TreeError::NotTree)?;
        Self::at(c, r)
    }

    /// The tree under the KV root at `root` -- the live one or ANY older generation's (they stay
    /// readable: a commit never writes below its mark). Checks the root's cells, nothing below.
    pub fn at(c: C, root: usize) -> Result<Self, TreeError> {
        if obj_digest_in(&c, root) != DIGEST_KV_ROOT {
            return Err(TreeError::NotTree);
        }
        let len = obj_len_in(&c, root) as usize;
        if len < 6 || root.checked_add(2 + len).is_none_or(|e| e > c.n_cells()) {
            return Err(TreeError::NotTree);
        }
        if get_in(&c, root, 5) != VERSION_TREE {
            return Err(TreeError::NotTree);
        }
        if len != ROOT_V4 {
            return Err(TreeError::Root { why: "a v4 root is exactly 8 cells" });
        }
        if (1..=4).any(|i| get_in(&c, root, i) != 0) {
            return Err(TreeError::Root { why: "cells 1-4 of a v4 root must be zero" });
        }
        let (n, depth) = (get_in(&c, root, 0), get_in(&c, root, 7));
        if n < 0 {
            return Err(TreeError::Root { why: "n is negative" });
        }
        if !(0..=MAX_TREE_DEPTH as i64).contains(&depth) {
            return Err(TreeError::Root { why: "DEPTH past MAX_TREE_DEPTH" });
        }
        let top = if depth == 0 {
            if get_in(&c, root, 6) != 0 || n != 0 {
                return Err(TreeError::Root { why: "DEPTH 0 is the empty map: no TREE, n = 0" });
            }
            0
        } else {
            super::node::back(&c, root, 6)?
        };
        Ok(TreeIn { c, root, n: n as usize, top, depth: depth as usize })
    }

    /// Entries, as the root CLAIMS (`verify` checks the claim).
    pub fn len(&self) -> usize {
        self.n
    }
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
    /// Levels below the KV root (0 = empty, 1 = one leaf).
    pub fn depth(&self) -> usize {
        self.depth
    }
    /// The KV root object this tree was opened at.
    pub fn root(&self) -> usize {
        self.root
    }
    /// The top node (0 for the empty map).
    pub fn top(&self) -> usize {
        self.top
    }
    pub fn image(&self) -> &C {
        &self.c
    }

    /// The value under `key`: one node per level, then the value object. No allocation where the
    /// image is bytes.
    pub fn get(&self, key: &[u8]) -> Result<Option<Bytes<'_>>, TreeError> {
        tree_get(&self.c, self.top, self.depth, key)
    }

    /// (top, depth): what `KvIn` keeps of an opened tree, next to its own image (P8c).
    pub(crate) fn head(&self) -> (usize, usize) {
        (self.top, self.depth)
    }

    /// The image back.
    pub fn into_image(self) -> C {
        self.c
    }

    /// The entries with `lo <= key <= hi` (by the bounds), in key order.
    pub fn range<'k>(&self, lo: Bound<&'k [u8]>, hi: Bound<&'k [u8]>) -> Iter<'_, 'k, C> {
        super::iter::range_in(&self.c, self.top, self.depth, lo, hi)
    }
    /// Every entry, in key order.
    pub fn iter(&self) -> Iter<'_, 'static, C> {
        self.range(Bound::Unbounded, Bound::Unbounded)
    }
    /// The entries whose key starts with `p`.
    pub fn prefix<'k>(&self, p: &'k [u8]) -> Iter<'_, 'k, C> {
        let mut it = self.range(Bound::Included(p), Bound::Unbounded);
        it.prefix = Some(p);
        it
    }

    /// `Kv::snapshot_root_u64` over this tree's entries, in key order.
    pub fn fold(&self) -> Result<u64, TreeError> {
        let mut h = FNV_OFFSET;
        for e in self.iter() {
            let (k, v) = e?;
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, &k);
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, &v);
        }
        Ok(h)
    }

    /// Every entry, owned.
    pub fn entries(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, TreeError> {
        self.iter().map(|e| e.map(|(k, v)| (k.into_owned(), v.into_owned()))).collect()
    }

    /// THE FULL CHECK (design §4 `Verify::Reachable`): the crc of the root and of every reachable
    /// node and value, every layout rule exactly, keys strictly ascending across the whole tree,
    /// each separator equal to its child's first key, no reachable one-child node, `n` counted.
    pub fn verify(&self) -> Result<(), TreeError> {
        check_obj_in(&self.c, self.root).map_err(TreeError::BadCrc)?;
        let (mut last, mut count) = (None, 0usize);
        if self.depth > 0 {
            self.walk(self.top, self.depth, None, &mut last, &mut count)?;
        }
        if count != self.n {
            return Err(TreeError::Count { claim: self.n as i64, got: count as i64 });
        }
        Ok(())
    }

    fn walk(&self, obj: usize, level: usize, sep: Option<&[u8]>, last: &mut Option<Vec<u8>>, count: &mut usize) -> Result<(), TreeError> {
        check_obj_in(&self.c, obj).map_err(TreeError::BadCrc)?;
        let nd = node_at(&self.c, obj, level == 1)?;
        if level > 1 && nd.m < 2 {
            return Err(TreeError::Node { obj, why: "a reachable node with one child" });
        }
        let its = items(&self.c, &nd)?;
        if sep.is_some_and(|s| its.first().is_none_or(|(k, _)| k.as_slice() != s)) {
            return Err(TreeError::Order { obj });
        }
        for (k, r) in its {
            if level > 1 {
                self.walk(r, level - 1, Some(&k), last, count)?;
                continue;
            }
            if last.as_ref().is_some_and(|l| *l >= k) {
                return Err(TreeError::Order { obj });
            }
            check_obj_in(&self.c, r).map_err(TreeError::BadCrc)?;
            val_at(&self.c, r)?;
            *last = Some(k);
            *count += 1;
        }
        Ok(())
    }
}
