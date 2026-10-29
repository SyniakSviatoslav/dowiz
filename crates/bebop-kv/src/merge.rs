//! `Staged`: a snapshot as a write transaction sees it -- the batch first, then the image
//! (W-ES-P3, R-EMBED-STORE §3.4 read-your-writes; AC-ES-10). No lock, no device: a reader over
//! any `Image` (a `View`, a `Store`, the library's snapshot) and a `Batch`, both borrowed.
//!
//! The merged order is key order: the base is ascending (`KvIn::open` checked it, or the holder
//! trusted it -- `open_lazy`), the batch is a BTreeMap, and on an equal key the batch wins. The
//! fold is `Kv::snapshot_root_u64`'s, over the merged entries, so a holder's `root()` with
//! pending edits answers what a decoded `Kv` with those edits applied would.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::iter::Peekable;
use core::ops::Bound;

use crate::batch::{empty_range, Batch, Range};
use crate::bytes::Bytes;
use crate::verify::KvError;
use crate::zc::{Image, KvIn};
use crate::FNV_OFFSET;

/// A reader over `base` with `batch`'s writes on top.
pub struct Staged<'a, C: Image> {
    pub base: &'a KvIn<C>,
    pub batch: &'a Batch,
}

impl<'a, C: Image> Staged<'a, C> {
    pub fn new(base: &'a KvIn<C>, batch: &'a Batch) -> Staged<'a, C> {
        Staged { base, batch }
    }

    /// The batch's answer if it has one, else the image's.
    pub fn get(&self, key: &[u8]) -> Result<Option<Bytes<'a>>, KvError> {
        let (base, batch): (&'a KvIn<C>, &'a Batch) = (self.base, self.batch);
        match batch.get(key) {
            Some(Some(v)) => Ok(Some(Bytes::borrowed(v))),
            Some(None) => Ok(None),
            None => Ok(base.get(key)?.map(Bytes::from)),
        }
    }

    pub fn contains(&self, key: &[u8]) -> Result<bool, KvError> {
        Ok(self.get(key)?.is_some())
    }

    /// Entries the merged map holds: O(|batch| log n).
    pub fn len(&self) -> Result<usize, KvError> {
        let mut n = self.base.len();
        for (k, v) in self.batch_entries() {
            let had = self.base.get(k)?.is_some();
            match (had, v.is_some()) {
                (false, true) => n += 1,
                (true, false) => n = n.saturating_sub(1),
                _ => {}
            }
        }
        Ok(n)
    }

    pub fn is_empty(&self) -> Result<bool, KvError> {
        Ok(self.len()? == 0)
    }

    fn batch_entries(&self) -> impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)> {
        let batch: &'a Batch = self.batch;
        batch.range(Bound::Unbounded, Bound::Unbounded).into_iter().flatten().map(|(k, v)| (k.as_slice(), v.as_deref()))
    }

    /// The merged entries with `lo <= key <= hi` (as the bounds say), in key order.
    pub fn cursor(&self, lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> Result<MergeCursor<'a, C>, KvError> {
        let (base, batch): (&'a KvIn<C>, &'a Batch) = (self.base, self.batch);
        if empty_range(lo, hi) {
            return Ok(MergeCursor::empty(base));
        }
        let start = match lo {
            Bound::Included(k) | Bound::Excluded(k) => base.seek(k)?,
            Bound::Unbounded => 0,
        };
        Ok(MergeCursor {
            base,
            i: start,
            sorted: base.sorted(),
            lo: own(lo),
            hi: own(hi),
            batch: batch.range(lo, hi).map(Iterator::peekable),
            head: None,
            base_done: false,
            failed: false,
        })
    }

    /// Every merged entry, in key order (what a compaction writes).
    pub fn entries(&self) -> Result<Vec<(Bytes<'a>, Bytes<'a>)>, KvError> {
        self.cursor(Bound::Unbounded, Bound::Unbounded)?.collect()
    }

    /// FNV-1a 64 over `len||key||len||value` of the merged entries: `Kv::snapshot_root_u64`.
    pub fn fold_root(&self) -> Result<u64, KvError> {
        let mut h = FNV_OFFSET;
        for e in self.cursor(Bound::Unbounded, Bound::Unbounded)? {
            let (k, v) = e?;
            h = crate::fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = crate::fnv1a(h, &k);
            h = crate::fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = crate::fnv1a(h, &v);
        }
        Ok(h)
    }
}

fn own(b: Bound<&[u8]>) -> Bound<Vec<u8>> {
    match b {
        Bound::Included(k) => Bound::Included(k.to_vec()),
        Bound::Excluded(k) => Bound::Excluded(k.to_vec()),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// A merge of the base's range and the batch's, in key order. An `Err` ends it.
pub struct MergeCursor<'a, C: Image> {
    base: &'a KvIn<C>,
    i: usize,
    sorted: bool,
    lo: Bound<Vec<u8>>,
    hi: Bound<Vec<u8>>,
    batch: Option<Peekable<Range<'a>>>,
    head: Option<(Bytes<'a>, Bytes<'a>)>,
    base_done: bool,
    failed: bool,
}

impl<'a, C: Image> MergeCursor<'a, C> {
    fn empty(base: &'a KvIn<C>) -> MergeCursor<'a, C> {
        MergeCursor {
            base,
            i: 0,
            sorted: true,
            lo: Bound::Unbounded,
            hi: Bound::Unbounded,
            batch: None,
            head: None,
            base_done: true,
            failed: false,
        }
    }

    fn below_lo(&self, k: &[u8]) -> bool {
        match &self.lo {
            Bound::Included(l) => k < l.as_slice(),
            Bound::Excluded(l) => k <= l.as_slice(),
            Bound::Unbounded => false,
        }
    }

    fn above_hi(&self, k: &[u8]) -> bool {
        match &self.hi {
            Bound::Included(h) => k > h.as_slice(),
            Bound::Excluded(h) => k >= h.as_slice(),
            Bound::Unbounded => false,
        }
    }

    /// The next base entry inside the bounds. On a sorted base the walk stops at the first key
    /// past `hi`; on an unsorted one (no writer here makes one) it filters the whole stored order.
    fn next_base(&mut self) -> Result<Option<(Bytes<'a>, Bytes<'a>)>, KvError> {
        let base: &'a KvIn<C> = self.base;
        while self.i < base.len() {
            let i = self.i;
            self.i += 1;
            let k = base.key(i)?;
            if self.below_lo(&k) {
                continue;
            }
            if self.above_hi(&k) {
                if self.sorted {
                    self.i = base.len();
                    return Ok(None);
                }
                continue;
            }
            let v = base.value(i)?;
            return Ok(Some((Bytes::from(k), Bytes::from(v))));
        }
        Ok(None)
    }

    /// Take the batch's next entry: `Some(put)` or `None` for a tombstone.
    fn take_batch(&mut self) -> Option<Option<(Bytes<'a>, Bytes<'a>)>> {
        let (k, v) = self.batch.as_mut()?.next()?;
        Some(v.as_deref().map(|v| (Bytes::borrowed(k.as_slice()), Bytes::borrowed(v))))
    }
}

impl<'a, C: Image> Iterator for MergeCursor<'a, C> {
    type Item = Result<(Bytes<'a>, Bytes<'a>), KvError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.failed {
                return None;
            }
            if self.head.is_none() && !self.base_done {
                match self.next_base() {
                    Ok(Some(e)) => self.head = Some(e),
                    Ok(None) => self.base_done = true,
                    Err(e) => {
                        self.failed = true;
                        return Some(Err(e));
                    }
                }
            }
            let bk: Option<&'a [u8]> = self.batch.as_mut().and_then(|b| b.peek().map(|&(k, _)| k.as_slice()));
            let order = match (&self.head, bk) {
                (None, None) => return None,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some((hk, _)), Some(bk)) => hk.as_bytes().cmp(bk),
            };
            match order {
                Ordering::Less => return self.head.take().map(Ok),
                Ordering::Equal => {
                    self.head = None;
                    if let Some(Some(e)) = self.take_batch() {
                        return Some(Ok(e));
                    }
                }
                Ordering::Greater => {
                    if let Some(Some(e)) = self.take_batch() {
                        return Some(Ok(e));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
