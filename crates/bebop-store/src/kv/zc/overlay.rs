//! THE DELTA CHAIN READ IN PLACE (W-DELTA, 2026-10-06; the format is in `kv/delta.rs`).
//!
//! A v3 image is a v2 base plus a chain of records. `KvIn` answers it without decoding the
//! base: `ov` holds the NEWEST op per key, sorted by key (at most `MAX_DELTAS` of them as the
//! writer builds it), and `merged` the order a decoded `Kv` would have -- base slots and
//! overlay slots interleaved by a two-pointer merge, removed keys dropped. Then:
//!
//!   * `get` -- the overlay by binary search first (a put answers its value straight from the
//!     record's cells, a remove answers `None`), else the base exactly as before;
//!   * `len` / `key(i)` / `value(i)` / `prefix_range` -- over the merged order, so every
//!     caller (CatalogView, `kv_prefix_in`, the fold) sees what `Kv::load` would decode.
//!
//! Without a chain `merged` stays EMPTY and index i is base i: a v1/v2 image costs nothing new.
//! A chain on a base whose keys are not strictly ascending is refused (`NotKv`), as by
//! `Kv::decode`: no writer here makes one, and merging into an unsorted order has no answer
//! both readers would agree on.

use std::borrow::Cow;
use std::cmp::Ordering;

use super::{delta, Image, KvError, KvIn, Rec};

/// Where merged entry i lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    Base(usize),
    Delta(usize),
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
    /// Build the merged order once the key order is known. See the module.
    pub(super) fn finish(&mut self) -> Result<(), KvError> {
        if self.ov.is_empty() {
            return Ok(());
        }
        if !self.sorted {
            return Err(KvError::NotKv);
        }
        let mut out = Vec::with_capacity(self.n + self.ov.len());
        let (mut i, mut j) = (0usize, 0usize);
        while i < self.n || j < self.ov.len() {
            let ord = if i == self.n {
                Ordering::Greater
            } else if j == self.ov.len() {
                Ordering::Less
            } else {
                self.cmp_key(i, &self.ov[j].0)?
            };
            let put = j < self.ov.len() && self.ov[j].1.op == delta::OP_PUT;
            match ord {
                Ordering::Less => {
                    out.push(Slot::Base(i));
                    i += 1;
                }
                Ordering::Greater => {
                    if put {
                        out.push(Slot::Delta(j));
                    }
                    j += 1;
                }
                Ordering::Equal => {
                    if put {
                        out.push(Slot::Delta(j));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
        self.merged = out;
        Ok(())
    }

    fn slot(&self, i: usize) -> Result<Slot, KvError> {
        if self.ov.is_empty() {
            return if i < self.n { Ok(Slot::Base(i)) } else { Err(KvError::NotKv) };
        }
        self.merged.get(i).copied().ok_or(KvError::NotKv)
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
        if self.ov.is_empty() { self.n } else { self.merged.len() }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Key `i` in the merged (sorted) order.
    pub fn key(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        match self.slot(i)? {
            Slot::Base(b) => self.base_key(b),
            Slot::Delta(j) => Ok(Cow::Borrowed(&self.ov[j].0)),
        }
    }

    /// Value `i` in the merged order.
    pub fn value(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        match self.slot(i)? {
            Slot::Base(b) => self.base_value(b),
            Slot::Delta(j) => Ok(self.rec_value(&self.ov[j].1)),
        }
    }

    /// The value under `key`: the overlay first, then O(log n) base keys and the one value.
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
    /// this is the base's lower-bound search; with one, a pass over the merged order (it is
    /// at most n + MAX_DELTAS entries, and only a v3 image pays it).
    pub fn prefix_range(&self, prefix: &[u8]) -> Result<Vec<usize>, KvError> {
        if self.ov.is_empty() {
            return self.base_prefix_range(prefix);
        }
        let mut out = Vec::new();
        for i in 0..self.merged.len() {
            if self.key(i)?.starts_with(prefix) {
                out.push(i);
            }
        }
        Ok(out)
    }
}
