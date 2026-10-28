//! Z-SETS: THE ONE DELTA CURRENCY (LV-INC-A, R-BEBOP-LEVERS-2026-10-27 §2.4, operator Q2).
//!
//! A Z-set is a finite map `row -> weight` (weights in i64, zero weights absent): DBSP's
//! collection type (Budiu et al., PVLDB 16(7) 2023). An insertion is +1, a deletion -1, and a
//! change is both. Integration is `add`, so an incremental step `acc + Q(delta)` equals the
//! full recomputation `Q(base + delta)` whenever `Q` is LINEAR (`Q(a + b) = Q a + Q b`).
//!
//! NOTHING NEW ON DISK. The image already holds Z-set deltas:
//!   * a KV v3 delta record (kv/delta.rs) is one: `put k v` = `+(k, v) - (k, old)` and
//!     `remove k` = `-(k, old)`, where `old` is what the image said just before the record
//!     (the older records, then the base) -- `from_kv_chain`;
//!   * the event log's records after a tip are a delta of weight +1 each -- `from_log_tail`.
//!
//! A row on disk is fixed-width i64 cells. A KV row becomes TWO cells, the K64 node keys of
//! `'O' ‖ field(key)` and `'O' ‖ field(value)` (nodekey.rs, TAG_OBJECT) -- `kv_row_cells`.
//! `digest` is what every reader re-derives (FORMAT §9.3, PROJ kind 4):
//!   FNV-1a 64 over the rows in ascending (signed i64, lexicographic) order, each row as
//!   `c0 LE8 ‖ .. ‖ c(w-1) LE8 ‖ weight LE8`.
use crate::evlog::Record;
use crate::kv::delta::{chain_in, Rec, OP_PUT};
use crate::kv::{FNV_OFFSET, FNV_PRIME};
use crate::nodekey::{Frame, TAG_OBJECT};
use crate::{follow_in, get_in, obj_cells_in, Cells};

/// A finite map row -> non-zero weight, kept sorted by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZSet<R: Ord + Clone> {
    rows: Vec<(R, i64)>,
}

impl<R: Ord + Clone> Default for ZSet<R> {
    fn default() -> Self {
        ZSet { rows: Vec::new() }
    }
}

impl<R: Ord + Clone> ZSet<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// One row with weight `w` (empty when `w` is 0).
    pub fn single(r: R, w: i64) -> Self {
        Self::from_pairs(vec![(r, w)])
    }

    /// Any pairs, consolidated: equal rows summed, zero weights dropped.
    pub fn from_pairs(mut v: Vec<(R, i64)>) -> Self {
        v.sort_by(|a, b| a.0.cmp(&b.0));
        let mut rows: Vec<(R, i64)> = Vec::with_capacity(v.len());
        for (r, w) in v {
            match rows.last_mut() {
                Some(last) if last.0 == r => last.1 = last.1.wrapping_add(w),
                _ => rows.push((r, w)),
            }
        }
        rows.retain(|p| p.1 != 0);
        ZSet { rows }
    }

    /// `self + other`: a sorted merge, zero sums dropped.
    pub fn add(&self, other: &Self) -> Self {
        let (a, b) = (&self.rows, &other.rows);
        let (mut i, mut j) = (0, 0);
        let mut rows = Vec::with_capacity(a.len() + b.len());
        while i < a.len() || j < b.len() {
            let ord = match (a.get(i), b.get(j)) {
                (Some(x), Some(y)) => x.0.cmp(&y.0),
                (Some(_), None) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Greater,
            };
            match ord {
                std::cmp::Ordering::Less => {
                    rows.push(a[i].clone());
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    rows.push(b[j].clone());
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    let w = a[i].1.wrapping_add(b[j].1);
                    if w != 0 {
                        rows.push((a[i].0.clone(), w));
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
        ZSet { rows }
    }

    /// `-self`: every weight negated.
    pub fn neg(&self) -> Self {
        ZSet { rows: self.rows.iter().map(|(r, w)| (r.clone(), w.wrapping_neg())).collect() }
    }

    /// `self - other`.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub fn is_zero(&self) -> bool {
        self.rows.is_empty()
    }

    /// The rows and their weights, ascending by row.
    pub fn rows(&self) -> &[(R, i64)] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The weight of `r` (0 when absent).
    pub fn weight(&self, r: &R) -> i64 {
        self.rows.binary_search_by(|p| p.0.cmp(r)).map_or(0, |i| self.rows[i].1)
    }

    /// A LINEAR map applied row by row: `f` sends a row to another row, weights carried.
    pub fn map<S: Ord + Clone>(&self, f: impl Fn(&R) -> S) -> ZSet<S> {
        ZSet::from_pairs(self.rows.iter().map(|(r, w)| (f(r), *w)).collect())
    }
}

/// A KV row: (key bytes, value bytes).
pub type KvRow = (Vec<u8>, Vec<u8>);

/// The two cells a KV row is on disk: K64('O' ‖ field(key)), K64('O' ‖ field(value)).
pub fn kv_row_cells(r: &KvRow) -> Vec<i64> {
    vec![Frame::new(TAG_OBJECT).bytes(&r.0).k64(), Frame::new(TAG_OBJECT).bytes(&r.1).k64()]
}

/// The digest every reader re-derives (module header).
pub fn digest(z: &ZSet<Vec<i64>>) -> i64 {
    let mut h = FNV_OFFSET;
    let mut eat = |v: i64| {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
    };
    for (r, w) in z.rows() {
        r.iter().for_each(|&c| eat(c));
        eat(*w);
    }
    h as i64
}

/// The event log's records after a tip, each as its content id with weight +1.
pub fn from_log_tail(recs: &[Record]) -> ZSet<[u8; 32]> {
    ZSet::from_pairs(recs.iter().map(|r| (r.id, 1)).collect())
}

/// `len` bytes from BYTE `off` of `obj`'s payload -- packed eight to a cell, or one per cell
/// when `per8` is false (v1) -- bounded by the object.
fn bytes_of<C: Cells + ?Sized>(c: &C, obj: usize, off: usize, len: usize, per8: bool) -> Option<Vec<u8>> {
    let cap = if per8 { obj_cells_in(c, obj).checked_mul(8)? } else { obj_cells_in(c, obj) };
    if off.checked_add(len)? > cap {
        return None;
    }
    Some(if per8 {
        (off..off + len).map(|b| (get_in(c, obj, b >> 3) >> (8 * (b & 7))) as u8).collect()
    } else {
        (off..off + len).map(|b| get_in(c, obj, b) as u8).collect()
    })
}

/// The base's slice `i` of one index/blob pair (keys: a[0], a[1]; values: a[2], a[3]), bounded by its blob.
fn base_slice<C: Cells + ?Sized>(c: &C, idx: usize, blob: usize, i: usize, per8: bool) -> Option<Vec<u8>> {
    let (off, len) = (get_in(c, idx, 2 * i), get_in(c, idx, 2 * i + 1));
    if off < 0 || len < 0 {
        return None;
    }
    bytes_of(c, blob, off as usize, len as usize, per8)
}

/// The base's entry `i`: (key, value), every claim bounded by its blob.
fn base_entry<C: Cells + ?Sized>(c: &C, a: [usize; 4], i: usize, per8: bool) -> Option<KvRow> {
    Some((base_slice(c, a[0], a[1], i, per8)?, base_slice(c, a[2], a[3], i, per8)?))
}

/// The base of the KV root at `root`: its arrays, entry count and packing.
fn base_of<C: Cells + ?Sized>(c: &C, root: usize) -> Option<([usize; 4], usize, bool)> {
    let a = [follow_in(c, root, 1)?, follow_in(c, root, 2)?, follow_in(c, root, 3)?, follow_in(c, root, 4)?];
    let n = get_in(c, root, 0);
    if n < 0 || (n as usize).checked_mul(2)? > obj_cells_in(c, a[0]).min(obj_cells_in(c, a[2])) {
        return None;
    }
    let ver = crate::kv::delta::version_in(c, root);
    Some((a, n as usize, ver >= 2))
}

/// The base entries of a KV root, each with weight +1. `None` = not a KV base.
pub fn from_kv_base<C: Cells + ?Sized>(c: &C, root: usize) -> Option<ZSet<KvRow>> {
    let (a, n, per8) = base_of(c, root)?;
    let rows = (0..n).map(|i| base_entry(c, a, i, per8).map(|r| (r, 1))).collect::<Option<Vec<_>>>()?;
    Some(ZSet::from_pairs(rows))
}

/// The base's value under `key`, by binary search over its sorted keys (every writer sorts;
/// `KvIn::open` checks it).
fn base_get<C: Cells + ?Sized>(c: &C, root: usize, key: &[u8]) -> Option<Vec<u8>> {
    let (a, n, per8) = base_of(c, root)?;
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        // KEYS ONLY while searching; the value is decoded once, on the match (MEASURED: decoding each
        // probe's 2.5 KB value was most of a one-price step, ~57-69 of ~110-121 us).
        let k = base_slice(c, a[0], a[1], mid, per8)?;
        match k.as_slice().cmp(key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return base_slice(c, a[2], a[3], mid, per8),
        }
    }
    None
}

/// What the image said under `key` just before chain record `chain[i]` (chain newest first):
/// the newest OLDER record that names the key, else the base.
fn value_before<C: Cells + ?Sized>(c: &C, root: usize, chain: &[Rec], i: usize, key: &[u8]) -> Option<Vec<u8>> {
    match chain[i + 1..].iter().find(|r| r.key(c) == key) {
        Some(r) if r.op == OP_PUT => Some(r.value(c)),
        Some(_) => None,
        None => base_get(c, root, key),
    }
}

/// The Z-set of the NEWEST `k` records of the KV root's delta chain. `None` = the chain does
/// not hold, or `k` is longer than it.
pub fn from_kv_chain<C: Cells + ?Sized>(c: &C, root: usize, k: usize) -> Option<ZSet<KvRow>> {
    let chain = chain_in(c, root)?;
    if k > chain.len() {
        return None;
    }
    let mut pairs = Vec::new();
    for i in 0..k {
        let r = &chain[i];
        let key = r.key(c);
        if let Some(old) = value_before(c, root, &chain, i, &key) {
            pairs.push(((key.clone(), old), -1));
        }
        if r.op == OP_PUT {
            pairs.push(((key, r.value(c)), 1));
        }
    }
    Some(ZSet::from_pairs(pairs))
}

/// The whole KV as a Z-set: the base plus every delta record (`integrate`).
pub fn from_kv<C: Cells + ?Sized>(c: &C, root: usize) -> Option<ZSet<KvRow>> {
    let d = chain_in(c, root)?.len();
    Some(from_kv_base(c, root)?.add(&from_kv_chain(c, root, d)?))
}

#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod lean_tests;
