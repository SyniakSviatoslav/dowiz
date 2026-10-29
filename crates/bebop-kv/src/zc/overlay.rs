//! THE DELTA CHAIN READ IN PLACE (W-DELTA, 2026-10-06; the format is in `kv/delta.rs`).
//!
//! A v3 image is a v2 base plus a chain of records. `KvIn` answers it without decoding the
//! base: `ov` holds the NEWEST op per key, sorted by key (at most `MAX_DELTAS` of them as the
//! writer builds it), and `pieces` the order a decoded `Kv` would have, as at most 2D+1 runs:
//! a run of consecutive base slots, or one overlay slot, each with its first merged index.
//! Then:
//!
//!   * `get` -- the overlay by binary search first (a put answers its value straight from the
//!     record's cells, a remove answers `None`), else the base exactly as before;
//!   * `len` / `key(i)` / `value(i)` / `prefix_range` -- over the merged order, so every
//!     caller (CatalogView, `kv_prefix_in`, the fold) sees what `Kv::load` would decode.
//!
//! O(D log n) PER OPEN, NOT O(n) (W-KVPERF 2026-10-28). This used to be a two-pointer merge
//! of all n base keys into an (n + D)-slot vector on EVERY `open`/`reopen`: 1.6 ms for one
//! delta at 1e5 x 256 B against 1.7 us without one, which is what `reopen` exists to avoid.
//! Now each overlay key is placed in the base by ONE binary search (the base is strictly
//! ascending: `open` checked it, and a chain on an unsorted base is refused below), and the
//! pieces between those places are counted, not listed. `key(i)` is a binary search over
//! the <= 2D+1 pieces.
//!
//! Without a chain `pieces` stays EMPTY and index i is base i: a v1/v2 image costs nothing new.
//! A chain on a base whose keys are not strictly ascending is refused (`NotKv`), as by
//! `Kv::decode`: no writer here makes one, and merging into an unsorted order has no answer
//! both readers would agree on.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::borrow::Cow;
use core::cmp::Ordering;

use super::{delta, Image, KvError, KvIn, Rec};

/// Where merged entry i lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    Base(usize),
    Delta(usize),
}

/// A run of the merged order starting at merged index `at`: base slots `b, b+1, ..` up to
/// the next piece's `at` (or `len`), or the one overlay slot `j`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Piece {
    at: usize,
    slot: Slot,
}

/// The newest op per key, sorted by key. `chain` is newest first, and a stable sort keeps
/// that order among equal keys, so `dedup` keeping the first keeps the newest.
pub(super) fn build<C: Image>(c: &C, chain: &[Rec]) -> Vec<(Vec<u8>, Rec)> {
    let mut ov: Vec<(Vec<u8>, Rec)> = chain.iter().map(|r| (r.key(c), *r)).collect();
    ov.sort_by(|a, b| a.0.cmp(&b.0));
    ov.dedup_by(|later, kept| later.0 == kept.0);
    ov
}

impl<C: Image> KvIn<C> {
    /// Place the overlay in the base once the key order is known. See the module.
    pub(super) fn finish(&mut self) -> Result<(), KvError> {
        if self.ov.is_empty() {
            return Ok(());
        }
        if !self.sorted {
            return Err(KvError::NotKv);
        }
        let mut pieces = Vec::with_capacity(2 * self.ov.len() + 1);
        let (mut at, mut b) = (0usize, 0usize);
        for (j, (k, r)) in self.ov.iter().enumerate() {
            // where `k` sits in the base: Ok = that slot holds it, Err = it goes before that slot
            let (pos, found) = self.lower_bound(k)?;
            if pos > b {
                pieces.push(Piece { at, slot: Slot::Base(b) });
                at += pos - b;
                b = pos;
            }
            if found {
                b += 1;
            }
            if r.op == delta::OP_PUT {
                pieces.push(Piece { at, slot: Slot::Delta(j) });
                at += 1;
            }
        }
        if b < self.n {
            pieces.push(Piece { at, slot: Slot::Base(b) });
            at += self.n - b;
        }
        self.pieces = pieces;
        self.mlen = at;
        Ok(())
    }

    /// The first base index whose key is not below `key`, and whether it equals `key`.
    /// Only on a sorted base (`finish` refuses the other).
    fn lower_bound(&self, key: &[u8]) -> Result<(usize, bool), KvError> {
        let (mut lo, mut hi) = (0usize, self.n);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.cmp_key(mid, key)? {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok((mid, true)),
            }
        }
        Ok((lo, false))
    }

    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    fn slot(&self, i: usize) -> Result<Slot, KvError> {
        if self.ov.is_empty() {
            return if i < self.n { Ok(Slot::Base(i)) } else { Err(KvError::NotKv) };
        }
        if i >= self.mlen {
            return Err(KvError::NotKv);
        }
        // the last piece starting at or before i; pieces[0].at == 0 whenever mlen > 0
        let p = self.pieces[self.pieces.partition_point(|p| p.at <= i) - 1];
        Ok(match p.slot {
            Slot::Base(b) => Slot::Base(b + (i - p.at)),
            d => d,
        })
    }

    /// A record's value: borrowed from the image when it is bytes, copied otherwise.
    fn rec_value(&self, r: &Rec) -> Cow<'_, [u8]> {
        let at = (r.obj + 2 + r.val_cell()).checked_mul(8);
        match at.and_then(|a| self.c.bytes_at(a, r.vlen)) {
            Some(b) => Cow::Borrowed(b),
            None => Cow::Owned(r.value(&self.c)),
        }
    }

    /// Entries a decoded `Kv` would hold.
    pub fn len(&self) -> usize {
        if self.ov.is_empty() { self.n } else { self.mlen }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Key `i` in the merged (sorted) order.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    pub fn key(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        match self.slot(i)? {
            Slot::Base(b) => self.base_key(b),
            Slot::Delta(j) => Ok(Cow::Borrowed(&self.ov[j].0)),
        }
    }

    /// Value `i` in the merged order.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    pub fn value(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        match self.slot(i)? {
            Slot::Base(b) => self.base_value(b),
            Slot::Delta(j) => Ok(self.rec_value(&self.ov[j].1)),
        }
    }

    /// The value under `key`: the overlay first, then O(log n) base keys and the one value.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
    pub fn get(&self, key: &[u8]) -> Result<Option<Cow<'_, [u8]>>, KvError> {
        if let Ok(j) = self.ov.binary_search_by(|(k, _)| k.as_slice().cmp(key)) {
            let r = &self.ov[j].1;
            return Ok(if r.op == delta::OP_PUT { Some(self.rec_value(r)) } else { None });
        }
        match self.find(key)? {
            Some(i) => self.base_value(i).map(Some),
            None => Ok(None),
        }
    }

    /// The merged indices of every key starting with `prefix`, in key order. Without a chain
    /// this is the base's lower-bound search; with one, the same search over the merged order
    /// (sorted: a strictly ascending base with the newest op per key folded in).
    pub fn prefix_range(&self, prefix: &[u8]) -> Result<Vec<usize>, KvError> {
        if self.ov.is_empty() {
            return self.base_prefix_range(prefix);
        }
        let (mut lo, mut hi) = (0usize, self.mlen);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if &*self.key(mid)? < prefix { lo = mid + 1 } else { hi = mid }
        }
        let mut out = Vec::new();
        while lo < self.mlen && self.key(lo)?.starts_with(prefix) {
            out.push(lo);
            lo += 1;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
