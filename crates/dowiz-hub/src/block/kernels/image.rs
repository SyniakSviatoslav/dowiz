//! THE SALES IMAGE READ IN PLACE (lane W-BLOCKVIEW; AC-LV-COL-05).
//!
//! `pack::days_in` + `full_sum_by` CHECK every block (crc + layout), then COPY every column of the
//! span out (`decode`, then `widen` copies again), then fold through a `BTreeMap` entry per row.
//! A year (365 blocks, 14,600 rows, ~694 KB) was ~2.4 ms in release, ~70% of it crc + copy. Here:
//!
//!   * `Index::open(bytes)` -- the FULL check, the SAFE DEFAULT for untrusted bytes: every block
//!     through `decode::check` (crc, descriptors, offsets, padding -- exactly what `split` refuses),
//!     then every row as `Pack::decode` reads it -- two `names` blocks first, each day block one day
//!     after the last, dishes strictly ascending and named -- refusing with ITS refusal, in ITS order
//!     (rows walked in place, never copied). It keeps each day's column ranges.
//!   * `Index::view(bytes)` -- for a holder that already opened THESE bytes (one `Index` per image
//!     generation, as `bebop_store::kv::zc`'s `Checked`): no crc, O(blocks) -- the length and every
//!     block's stored crc trailer must equal what `open` saw, else `Mismatch`. An `Index` only comes
//!     out of a successful `open`, and every range it holds lies inside a buffer of that length, so a
//!     wrong `Index` gives a wrong answer at worst, never a panic.
//!   * `Day` -- one checked day block's columns as BORROWED little-endian bytes (`ColRef`); nothing
//!     is copied. `Day::new` checks one loose block (the hot tail's, encoded).
//!   * `full_sum_by_days` / `run_plan_days` -- the kernels over borrowed columns: the same answers
//!     and the same refusals (first overflow in block-then-row order) as `full_sum_by` / `run_plan`,
//!     summed into `Groups` (a sorted run with a next-slot hint). A reader: the bytes do not change.

use std::collections::BTreeMap;
use super::super::decode::{check, Layout};
use super::super::{schema, ty, Refusal, CRC};
use super::pack::block_len;
use super::{plan_of, topk, Op, PlanRow};
#[path = "scan.rs"]
mod scan;
pub use scan::{sum_into_cols, Groups};

const NCOLS: usize = 7; // `schema::SALES`'s columns

/// One day block's place in the image: its day, rows and column ranges `(offset, len, type)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DayAt {
    day: i32,
    n: usize,
    cols: [(usize, usize, u8); NCOLS],
}

/// What `Index::open` learned about some bytes. Fields private: only `open` makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    len: usize,
    /// `(offset, stored value)` of every block's crc trailer, in image order.
    crcs: Vec<(usize, u32)>,
    days: Vec<DayAt>,
}

/// A checked block's day place: its day (first row's), rows and absolute column ranges.
fn place(b: &[u8], at: usize, l: &Layout) -> DayAt {
    let mut cols = [(0, 0, 0); NCOLS];
    for (c, (&(off, len), spec)) in cols.iter_mut().zip(l.cols.iter().zip(&l.known.cols)) {
        *c = (at + off, len, spec.typ);
    }
    DayAt { day: le_i32(&b[at + l.cols[0].0..]) as i32, n: l.n as usize, cols }
}

/// The day of one loose checked sales block (`Day::new`).
fn day_at(b: &[u8]) -> Result<DayAt, Refusal> {
    let l = check(b)?;
    if l.known.schema != &schema::SALES || l.n == 0 || l.cols.len() != NCOLS {
        return Err(Refusal::Mismatch("day blocks are one day each, ascending"));
    }
    Ok(place(b, 0, &l))
}

/// Day block `l` at `at` as `Pack::decode` reads it, refused with ITS words in ITS order: a sales
/// block, not empty, every row of one day after `prev`, dishes strictly ascending, every dish one
/// of `dishes` (sorted). Read in place: the rows are walked, never copied.
fn day_in(b: &[u8], at: usize, l: &Layout, dishes: &[i64], prev: Option<i32>) -> Result<DayAt, Refusal> {
    if l.known.schema != &schema::SALES {
        return Err(Refusal::Mismatch("not a sales block"));
    }
    if l.n == 0 {
        return Err(Refusal::Mismatch("an empty day block"));
    }
    let d = place(b, at, l);
    let ((doff, dlen, _), (koff, klen, _)) = (d.cols[0], d.cols[1]);
    let keys = || b[koff..koff + klen].chunks_exact(8).map(|w| i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
    let one_day = b[doff..doff + dlen].chunks_exact(4).all(|w| le_i32(w) as i32 == d.day);
    let asc = keys().zip(keys().skip(1)).all(|(x, y)| x < y);
    if !one_day || prev.is_some_and(|p| p >= d.day) || !asc {
        return Err(Refusal::Mismatch("day blocks are one day each, ascending, one row per dish"));
    }
    let mut j = 0;
    for k in keys() {
        j += dishes[j..].partition_point(|x| *x < k);
        if dishes.get(j) != Some(&k) {
            return Err(Refusal::Mismatch("a row names a dish the image does not"));
        }
    }
    Ok(d)
}

impl Index {
    /// The full check. See the module. No bytes is an empty image. Every block is checked FIRST
    /// (as `split`), then read as `Pack::decode` reads it, so a refusal is the one it gives.
    pub fn open(bytes: &[u8]) -> Result<Index, Refusal> {
        let mut blocks = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            let rest = &bytes[at..];
            let len = block_len(rest).filter(|l| *l <= rest.len()).ok_or(Refusal::TooShort { len: rest.len() })?;
            blocks.push((at, check(&rest[..len])?));
            at += len;
        }
        let ends = blocks.iter().skip(1).map(|(a, _)| *a).chain((!blocks.is_empty()).then_some(bytes.len()));
        // Every end is a checked block's (>= its header + crc); `checked_sub` holds it, never trusts it.
        let trailer = |e: usize| e.checked_sub(CRC).map(|t| (t, le_i32(&bytes[t..]) as u32)).ok_or(Refusal::TooShort { len: e });
        let crcs = ends.map(trailer).collect::<Result<Vec<_>, _>>()?;
        let mut idx = Index { len: bytes.len(), crcs, days: Vec::new() };
        let mut it = blocks.iter();
        let mut dishes = match (it.next(), it.next()) {
            (None, _) => return Ok(idx),
            (Some((_, f)), Some((at, d))) if f.known.schema == &schema::NAMES && d.known.schema == &schema::NAMES => {
                let (off, len) = d.cols[0];
                bytes[at + off..at + off + len].chunks_exact(8).map(|w| i64::from_le_bytes(w.try_into().unwrap_or([0; 8]))).collect::<Vec<i64>>()
            }
            _ => return Err(Refusal::Mismatch("a sales image opens with two names blocks")),
        };
        dishes.sort_unstable();
        for (at, l) in it {
            let d = day_in(bytes, *at, l, &dishes, idx.days.last().map(|p| p.day))?;
            idx.days.push(d);
        }
        Ok(idx)
    }

    /// THESE bytes, already opened: no crc. See the module.
    pub fn view<'a>(&'a self, bytes: &'a [u8]) -> Result<Image<'a>, Refusal> {
        if bytes.len() != self.len || self.crcs.iter().any(|&(at, c)| le_i32(&bytes[at..at + CRC]) as u32 != c) {
            return Err(Refusal::Mismatch("an index of other bytes"));
        }
        Ok(Image { bytes, idx: self })
    }

    /// Day blocks held.
    pub fn days(&self) -> usize {
        self.days.len()
    }
}

/// An opened image: its bytes and its index.
#[derive(Debug, Clone, Copy)]
pub struct Image<'a> {
    bytes: &'a [u8],
    idx: &'a Index,
}

impl<'a> Image<'a> {
    /// The day blocks of `from..=to` (`yyyymmdd`), in day order -- what `days_in` returns, read in place.
    pub fn days(&self, from: i32, to: i32) -> Vec<Day<'a>> {
        let d = &self.idx.days;
        let (s, e) = (d.partition_point(|x| x.day < from), d.partition_point(|x| x.day <= to));
        d[s..e.max(s)].iter().map(|at| Day { bytes: self.bytes, at: *at }).collect()
    }
}

/// One checked `sales` day block, its columns borrowed.
#[derive(Debug, Clone, Copy)]
pub struct Day<'a> {
    bytes: &'a [u8],
    at: DayAt,
}

impl<'a> Day<'a> {
    /// One loose block, CHECKED (`decode::check`): a `sales` block with at least one row.
    pub fn new(bytes: &'a [u8]) -> Result<Day<'a>, Refusal> {
        Ok(Day { bytes, at: day_at(bytes)? })
    }
    pub fn day(&self) -> i32 {
        self.at.day
    }
    pub fn n(&self) -> usize {
        self.at.n
    }
    /// Column `i`, borrowed; `None` past the schema.
    pub fn col(&self, i: usize) -> Option<ColRef<'a>> {
        let &(off, len, typ) = self.at.cols.get(i)?;
        let b = &self.bytes[off..off + len];
        Some(match typ {
            ty::I64 | ty::CSR_VAL => ColRef::I64(b),
            ty::I32 => ColRef::I32(b),
            ty::BYTES => ColRef::U8(b),
            _ => ColRef::U32(b),
        })
    }
}

pub(super) fn le_i32(w: &[u8]) -> i64 {
    i64::from(i32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}

/// A column's little-endian bytes and how a value widens to i64 (as `widen` does).
#[derive(Debug, Clone, Copy)]
pub enum ColRef<'a> {
    I64(&'a [u8]),
    I32(&'a [u8]),
    U32(&'a [u8]),
    U8(&'a [u8]),
}

/// `full_sum_by` over day blocks read in place: the same groups and the same refusals.
pub fn full_sum_by_days(days: &[Day], key: usize, val: usize, filter: Option<(usize, Op, i64)>) -> Result<BTreeMap<i64, i64>, Refusal> {
    let name = schema::of(&schema::SALES).and_then(|k| k.cols.get(val)).map_or("-", |c| c.name);
    let mut acc = Groups::default();
    for d in days {
        let (Some(kc), Some(vc)) = (d.col(key), d.col(val)) else { return Err(Refusal::Mismatch("plan names no such column")) };
        let mask = match filter {
            Some((c, op, rhs)) => Some((d.col(c).ok_or(Refusal::Mismatch("plan names no such column"))?, op, rhs)),
            None => None,
        };
        sum_into_cols(&mut acc, kc, vc, mask, name)?;
    }
    Ok(acc.into_map())
}

/// `run_plan` over day blocks read in place.
pub fn run_plan_days(days: &[Day], plan: &[PlanRow]) -> Result<Vec<(i64, i64)>, Refusal> {
    let (filter, key, val, k) = plan_of(plan)?;
    let groups = full_sum_by_days(days, key, val, filter)?;
    Ok(match k {
        Some(k) => topk(&groups, k),
        None => groups.into_iter().collect(),
    })
}
