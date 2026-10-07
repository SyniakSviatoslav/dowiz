//! THE KV DELTA CHAIN (W-DELTA, R-BEBOPDB D.1 #3 / A.1 #2 / A.1 #5, 2026-10-06).
//!
//! Before this, every KV write rewrote all four arrays and the root into a fresh image
//! (`compacted_bytes_fit`): one changed price = 1.5-3.5 ms CPU and a new layout, so almost
//! every 96 KiB chunk of the Durable Object's copy differed and was written again. Now a
//! write APPENDS: one small record per changed key and a new root, and the image's bytes
//! change only in the superblock page and at the tail.
//!
//! ── VERSION 3, AND WHY A VERSION AND NOT AN OPTIONAL REF ──
//! A ref added to a v2 root would be invisible to every reader already deployed: it would
//! read the base and answer the OLD price, with nothing saying so. A version it does not
//! know is REFUSED (`Kv::decode`, `KvIn`, oracle.py status 3). kv.bp did NOT refuse (pj_kv_frame
//! read any version but 2 as one byte per cell), so every reader in the tree was taught v3 in the
//! same change: kv.bp `kv3_fold`, oracle.py, bebop-wasm through this crate. So:
//!
//!   v3 root, 8 cells, the SAME digest:
//!     {n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB, VERSION=3, ref DELTA, D}
//!   cells 0..4 are a v2 base (bytes packed eight to a cell); DELTA names the NEWEST record;
//!   D is how many records the chain holds -- a CLAIM: a walk that does not deliver exactly
//!   D records is a refusal (EvLog's rule for its count).
//!
//!   delta record (digest `arr i64`), payload:
//!     {ref NEXT (the next OLDER record, 0 = none), OP (1 put, 2 remove), KLEN, VLEN,
//!      key bytes packed ceil(KLEN/8) cells, value bytes packed ceil(VLEN/8) cells}
//!   its length must be EXACTLY 4 + ceil(KLEN/8) + ceil(VLEN/8); a remove has VLEN 0.
//!
//! READ = the base, then the records replayed OLDEST FIRST (a put inserts or overwrites,
//! a remove deletes; a remove of an absent key changes nothing). The crc of the root and of
//! every record is checked wherever the base's is (`Kv::load_checked`, `KvIn::open`).
//!
//! COMPACTION writes v2, byte for byte what it wrote before this module existed
//! (`golden_tests`), so after a compaction every reader that ever read a KV image reads it.
//! The writer compacts instead of appending when (`Appended::Compact` says why):
//!   * the chain would pass `MAX_DELTAS` records -- every read walks the chain;
//!   * superseded cells would pass a quarter of the live cells (`DEAD_DIV`);
//!   * the base is v1 (one byte per cell: no delta is written onto it);
//!   * the arena is full.

use super::Kv;
use crate::{follow_in, get_in, obj_cells_in, obj_len_in, Cells};

/// The version a root with a delta chain says.
pub const VERSION_DELTA: i64 = 3;
/// Cells in a v3 root.
pub(crate) const ROOT_V3: usize = 8;
/// Record ops.
pub const OP_PUT: i64 = 1;
pub const OP_REMOVE: i64 = 2;
/// The longest chain the writer builds before it compacts.
pub const MAX_DELTAS: usize = 32;
/// Compact when superseded cells would exceed live cells / DEAD_DIV.
pub const DEAD_DIV: i64 = 4;
/// Payload cells before a record's key: NEXT, OP, KLEN, VLEN.
const HEAD: usize = 4;

/// One change to append.
#[derive(Debug, Clone, Copy)]
pub enum Op<'a> {
    Put(&'a str, &'a [u8]),
    Remove(&'a str),
}

/// What `append_delta` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appended {
    /// The records and a new root were committed: the new generation.
    Delta(i64),
    /// Nothing was committed; the caller compacts. Why:
    Compact(Why),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Deltas,
    Dead,
    V1,
    Full,
}

/// One record found on a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rec {
    pub obj: usize,
    pub op: i64,
    pub klen: usize,
    pub vlen: usize,
}

impl Rec {
    /// Payload cell of the key's first byte.
    pub fn key_cell(&self) -> usize {
        HEAD
    }
    /// Payload cell of the value's first byte.
    pub fn val_cell(&self) -> usize {
        HEAD + self.klen.div_ceil(8)
    }
    /// The record's key bytes.
    pub fn key<C: Cells + ?Sized>(&self, c: &C) -> Vec<u8> {
        packed(c, self.obj, self.key_cell(), self.klen)
    }
    /// The record's value bytes (empty for a remove).
    pub fn value<C: Cells + ?Sized>(&self, c: &C) -> Vec<u8> {
        packed(c, self.obj, self.val_cell(), self.vlen)
    }
}

/// `len` bytes packed eight to a cell from payload cell `at` of `obj`.
fn packed<C: Cells + ?Sized>(c: &C, obj: usize, at: usize, len: usize) -> Vec<u8> {
    (0..len).map(|b| (get_in(c, obj, at + (b >> 3)) >> (8 * (b & 7))) as u8).collect()
}

/// The version a root says, read the way `Kv::version` reads it.
pub(crate) fn version_in<C: Cells + ?Sized>(c: &C, root: usize) -> i64 {
    if obj_cells_in(c, root) >= 6 && get_in(c, root, 5) > 0 { get_in(c, root, 5) } else { 1 }
}

/// The chain of a v3 root, NEWEST FIRST, every claim bounded by the image. `None` = not a KV
/// image: a record whose length is not exactly what its KLEN/VLEN say, an unknown op, a walk
/// that delivers more or fewer than D records, or records that together claim more cells than
/// the image holds (records are disjoint objects; this is also what ends a looping chain).
/// A v1/v2 root has an empty chain. The crc is NOT checked here (`check_chain`).
pub fn chain_in<C: Cells + ?Sized>(c: &C, root: usize) -> Option<Vec<Rec>> {
    if version_in(c, root) < VERSION_DELTA {
        return Some(Vec::new());
    }
    if obj_cells_in(c, root) < ROOT_V3 {
        return None;
    }
    let d = get_in(c, root, 7);
    if d < 0 {
        return None;
    }
    let d = d as usize;
    let mut budget = c.n_cells();
    let mut out = Vec::new();
    let mut cur = follow_in(c, root, 6);
    while let Some(obj) = cur {
        if out.len() >= d {
            return None;
        }
        let len = obj_cells_in(c, obj);
        if len as i64 != obj_len_in(c, obj) || len < HEAD {
            return None;
        }
        budget = budget.checked_sub(len + 2)?;
        let (op, kl, vl) = (get_in(c, obj, 1), get_in(c, obj, 2), get_in(c, obj, 3));
        if !(op == OP_PUT || op == OP_REMOVE) || kl < 0 || vl < 0 || (op == OP_REMOVE && vl != 0) {
            return None;
        }
        let (klen, vlen) = (kl as usize, vl as usize);
        if HEAD.checked_add(klen.div_ceil(8))?.checked_add(vlen.div_ceil(8))? != len {
            return None;
        }
        out.push(Rec { obj, op, klen, vlen });
        cur = follow_in(c, obj, 0);
    }
    if out.len() != d {
        return None;
    }
    Some(out)
}

impl Kv {
    /// Replay a chain (newest first, as `chain_in` gives it) onto decoded base entries.
    pub(crate) fn replay<C: Cells + ?Sized>(&mut self, c: &C, chain: &[Rec]) {
        for r in chain.iter().rev() {
            let k = String::from_utf8_lossy(&r.key(c)).into_owned();
            if r.op == OP_PUT {
                self.put(&k, &r.value(c));
            } else {
                self.remove(&k);
            }
        }
    }
}

/// The crc of every record on a chain (W-CRC covers the delta records too).
pub fn check_chain<C: super::zc::Image + ?Sized>(c: &C, chain: &[Rec]) -> Result<(), crate::BadCrc> {
    chain.iter().try_for_each(|r| super::zc::check_obj_in(c, r.obj))
}

/// The writer (split out of this file for the 300-line ratchet).
mod write;
pub use write::{append_delta, append_delta_with};

#[cfg(test)]
mod tests;
