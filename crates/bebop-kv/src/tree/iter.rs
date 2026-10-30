//! The ordered scan over a `TreeIn`: a cursor of (node, index) per level in a fixed array -- no Vec.
//! Seek is a lower-bound descent; `next` is idx + 1, and on a leaf's end it pops to the first
//! ancestor with a next child and descends that child's leftmost path.

use core::ops::Bound;

use super::node::{bytes_of, child_for, key_at, node_at, ref_at, search, val_at, NodeRef};
use super::TreeError;
use crate::bytes::Bytes;
use crate::zc::Image;

/// An ordered scan: a cursor of (node, index) per level. Yields `Err` once and stops on a refusal.
pub struct Iter<'t, 'k, C: Image> {
    pub(super) c: &'t C,
    pub(super) top: usize,
    pub(super) depth: usize,
    pub(super) s: [(NodeRef, usize); super::MAX_TREE_DEPTH],
    pub(super) hi: Bound<&'k [u8]>,
    pub(super) prefix: Option<&'k [u8]>,
    pub(super) done: bool,
    pub(super) err: Option<TreeError>,
}

/// The scan of the tree under (`top`, `depth`) in `c` from `lo` to `hi` (`TreeIn::range`; P8c:
/// `KvIn` and `MergeCursor` scan a v4 base through this without owning a `TreeIn`).
pub(crate) fn range_in<'t, 'k, C: Image>(c: &'t C, top: usize, depth: usize, lo: Bound<&[u8]>, hi: Bound<&'k [u8]>) -> Iter<'t, 'k, C> {
    let mut it = Iter { c, top, depth, s: [(NodeRef::default(), 0); super::MAX_TREE_DEPTH], hi, prefix: None, done: false, err: None };
    if let Err(e) = it.seek(lo) {
        it.err = Some(e);
    }
    it
}

impl<'t, C: Image> Iter<'t, '_, C> {
    fn set(&mut self, j: usize, v: (NodeRef, usize)) -> Result<(), TreeError> {
        *self.s.get_mut(j).ok_or(TreeError::Root { why: "DEPTH past MAX_TREE_DEPTH" })? = v;
        Ok(())
    }

    /// Descend to the first entry >= / > `lo`.
    pub(super) fn seek(&mut self, lo: Bound<&[u8]>) -> Result<(), TreeError> {
        let (c, d) = (self.c, self.depth);
        if d == 0 {
            self.done = true;
            return Ok(());
        }
        let mut obj = self.top;
        for j in 0..d {
            let nd = node_at(c, obj, j + 1 == d)?;
            let i = match (lo, nd.leaf) {
                (Bound::Unbounded, _) => 0,
                (Bound::Included(k) | Bound::Excluded(k), false) => child_for(c, &nd, k)?,
                (Bound::Included(k), true) => search(c, &nd, k)?.0,
                (Bound::Excluded(k), true) => {
                    let (i, eq) = search(c, &nd, k)?;
                    i + usize::from(eq)
                }
            };
            self.set(j, (nd, i))?;
            if !nd.leaf {
                obj = ref_at(c, &nd, i)?;
            }
        }
        Ok(())
    }

    /// The next entry, moving up past exhausted nodes and down the leftmost path.
    fn step(&mut self) -> Result<Option<(Bytes<'t>, Bytes<'t>)>, TreeError> {
        let (c, d) = (self.c, self.depth);
        loop {
            let (nd, i) = *self.s.get(d - 1).ok_or(TreeError::NotTree)?;
            if i < nd.m {
                self.set(d - 1, (nd, i + 1))?;
                let (ka, kl) = key_at(c, &nd, i)?;
                let (va, vl) = val_at(c, ref_at(c, &nd, i)?)?;
                return Ok(Some((bytes_of(c, ka, kl), bytes_of(c, va, vl))));
            }
            let mut j = d - 1;
            loop {
                if j == 0 {
                    return Ok(None);
                }
                j -= 1;
                let (p, pi) = *self.s.get(j).ok_or(TreeError::NotTree)?;
                if pi + 1 < p.m {
                    self.set(j, (p, pi + 1))?;
                    break;
                }
            }
            let (p, pi) = *self.s.get(j).ok_or(TreeError::NotTree)?;
            let mut obj = ref_at(c, &p, pi)?;
            for jj in j + 1..d {
                let nd = node_at(c, obj, jj + 1 == d)?;
                self.set(jj, (nd, 0))?;
                if !nd.leaf {
                    obj = ref_at(c, &nd, 0)?;
                }
            }
        }
    }
}

impl<'t, C: Image> Iterator for Iter<'t, '_, C> {
    type Item = Result<(Bytes<'t>, Bytes<'t>), TreeError>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(e) = self.err.take() {
            self.done = true;
            return Some(Err(e));
        }
        if self.done {
            return None;
        }
        match self.step() {
            Ok(Some((k, v))) => {
                let past = match self.hi {
                    Bound::Included(h) => k.as_bytes() > h,
                    Bound::Excluded(h) => k.as_bytes() >= h,
                    Bound::Unbounded => false,
                };
                if past || self.prefix.is_some_and(|p| !k.starts_with(p)) {
                    self.done = true;
                    return None;
                }
                Some(Ok((k, v)))
            }
            Ok(None) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}
