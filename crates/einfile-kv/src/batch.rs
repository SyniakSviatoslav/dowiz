//! `Batch`: the writes of a transaction that has not committed (W-ES-P3, R-EMBED-STORE §3.4).
//!
//! OWNED -- it borrows no database -- so a holder can keep pending edits across calls and hand
//! them to a write transaction later (`Db::begin_write_with`). This is what dowiz-hub's
//! `Catalog.dirty` and its in-memory `Kv` were together: the newest op per key, in key order.
//! Limits are checked at `put`/`remove`, so a refusal leaves the batch as it was.

use alloc::collections::btree_map;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::ops::Bound;

use crate::delta::KeyOp;
use crate::limits::{LimitError, Limits};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Batch {
    ops: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    limits: Limits,
}

/// The empty batch, for a reader with no writes of its own (`ReadTxn::scan`).
static EMPTY: Batch = Batch { ops: BTreeMap::new(), limits: Limits::FORMAT };

/// The batch's entries between two bounds, or none.
pub(crate) type Range<'a> = btree_map::Range<'a, Vec<u8>, Option<Vec<u8>>>;

impl Batch {
    pub fn new(limits: Limits) -> Batch {
        Batch { ops: BTreeMap::new(), limits: limits.lowered() }
    }

    /// A batch with nothing in it, borrowed for as long as anyone likes.
    pub fn empty() -> &'static Batch {
        &EMPTY
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Insert or overwrite.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), LimitError> {
        self.limits.check_key(key.len())?;
        self.limits.check_value(value.len())?;
        self.ops.insert(key.to_vec(), Some(value.to_vec()));
        Ok(())
    }

    /// Record a removal. Whether the key was there is a question for the reader the batch is
    /// staged over (`Staged::contains`), not for the batch.
    pub fn remove(&mut self, key: &[u8]) -> Result<(), LimitError> {
        self.limits.check_key(key.len())?;
        self.ops.insert(key.to_vec(), None);
        Ok(())
    }

    /// `None` = the batch says nothing about `key`; `Some(None)` = removed here.
    pub fn get(&self, key: &[u8]) -> Option<Option<&[u8]>> {
        self.ops.get(key).map(|v| v.as_deref())
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn clear(&mut self) {
        self.ops.clear();
    }

    /// The ops in key order, as the delta writer takes them.
    pub fn ops(&self) -> impl Iterator<Item = KeyOp<'_>> {
        self.ops.iter().map(|(k, v)| match v {
            Some(v) => KeyOp::Put(k, v),
            None => KeyOp::Remove(k),
        })
    }

    /// The entries between `lo` and `hi`; `None` for a range with nothing in it (BTreeMap would
    /// PANIC on `lo > hi` or on two equal excluded ends -- a scan's bounds come from callers).
    pub(crate) fn range(&self, lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> Option<Range<'_>> {
        if empty_range(lo, hi) {
            return None;
        }
        Some(self.ops.range::<[u8], _>((lo, hi)))
    }
}

/// Is the range `lo..hi` empty by its bounds alone?
pub(crate) fn empty_range(lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> bool {
    match (lo, hi) {
        (Bound::Included(a), Bound::Included(b)) => a > b,
        (Bound::Included(a) | Bound::Excluded(a), Bound::Included(b) | Bound::Excluded(b)) => a >= b,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
