//! The delta WRITER (W-DELTA): records + a new v3 root, the compaction trigger, and the
//! superseded-cell count. The format and the readers are in `../delta.rs`.

use super::{chain_in, version_in, Appended, Op, Rec, Why, HEAD, OP_PUT, OP_REMOVE, ROOT_V3, VERSION_DELTA, MAX_DELTAS, DEAD_DIV};
use crate::kv::{DIGEST_ARR_I64, VERSION};
use crate::{obj_cells_in, Store, StoreError};

/// The base entry under `key`: (key bytes, value bytes) lengths, by binary search over the
/// sorted base index. For the writer's superseded estimate only -- a miss costs nothing.
fn base_lens(st: &Store, root: usize, key: &[u8]) -> Option<(usize, usize)> {
    let (kidx, kblob, vidx) = (st.follow(root, 1)?, st.follow(root, 2)?, st.follow(root, 3)?);
    let n = st.get(root, 0).max(0) as usize;
    let n = n.min(st.obj_cells(kidx) / 2).min(st.obj_cells(vidx) / 2);
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (ko, kl) = (st.get(kidx, 2 * mid).max(0) as usize, st.get(kidx, 2 * mid + 1).max(0) as usize);
        let k: Vec<u8> = (0..kl.min(1 << 16)).map(|j| ko.saturating_add(j)).map(|b| (st.get(kblob, b >> 3) >> (8 * (b & 7))) as u8).collect();
        match k.as_slice().cmp(key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return Some((kl, st.get(vidx, 2 * mid + 1).max(0) as usize)),
        }
    }
    None
}

/// Cells a record for `op` takes, header included.
fn rec_cells(op: &Op) -> usize {
    let (k, v) = match op {
        Op::Put(k, v) => (k.len(), v.len()),
        Op::Remove(k) => (k.len(), 0),
    };
    2 + HEAD + k.div_ceil(8) + v.div_ceil(8)
}

/// Superseded cells the ops make dead: the newest earlier record of the same key (exact), else
/// the base entry's value cells (+ key and index cells for a remove: an estimate -- those cells
/// stay inside the base blobs until a compaction).
fn dead_of(st: &Store, root: usize, chain: &[Rec], ops: &[Op]) -> i64 {
    let mut dead = 0i64;
    for (i, op) in ops.iter().enumerate() {
        let (key, remove) = match op {
            Op::Put(k, _) => (k.as_bytes(), false),
            Op::Remove(k) => (k.as_bytes(), true),
        };
        let earlier = ops[..i].iter().rev().find(|o| matches!(o, Op::Put(k, _) | Op::Remove(k) if k.as_bytes() == key));
        if let Some(o) = earlier {
            dead += rec_cells(o) as i64;
        } else if let Some(r) = chain.iter().find(|r| r.key(st) == key) {
            dead += 2 + obj_cells_in(st, r.obj) as i64;
        } else if let Some((kl, vl)) = base_lens(st, root, key) {
            dead += vl.div_ceil(8) as i64 + if remove { kl.div_ceil(8) as i64 + 4 } else { 0 };
        }
    }
    dead
}

/// Write `bytes` packed eight to a cell from payload cell `at` of `obj`.
fn write_packed(st: &mut Store, obj: usize, at: usize, bytes: &[u8]) {
    for (c, chunk) in bytes.chunks(8).enumerate() {
        let mut w = [0u8; 8];
        w[..chunk.len()].copy_from_slice(chunk);
        st.put_cell(obj, at + c, i64::from_le_bytes(w));
    }
}

/// `append_delta` with the trigger's two numbers given (tests move them).
pub fn append_delta_with(st: &mut Store, ops: &[Op], max_deltas: usize, dead_div: i64) -> Result<Appended, StoreError> {
    let root = st.root().ok_or(StoreError::NoSuperblock)?;
    let ver = version_in(st, root);
    if ver > VERSION_DELTA {
        return Err(StoreError::Corrupt("KV root names a version this code does not know"));
    }
    if ver < VERSION {
        return Ok(Appended::Compact(Why::V1));
    }
    let chain = chain_in(st, root).ok_or(StoreError::Corrupt("the KV delta chain does not hold"))?;
    if chain.len() + ops.len() > max_deltas {
        return Ok(Appended::Compact(Why::Deltas));
    }
    let sb = st.pick().ok_or(StoreError::NoSuperblock)?;
    let dead = 2 + obj_cells_in(st, root) as i64 + dead_of(st, root, &chain, ops);
    let alloc: i64 = ops.iter().map(|o| rec_cells(o) as i64).sum::<i64>() + 2 + ROOT_V3 as i64;
    let (live, sup) = (sb.live_cells + alloc - dead, sb.superseded_cells + dead);
    if sup.saturating_mul(dead_div) > live {
        return Ok(Appended::Compact(Why::Dead));
    }
    let arrays: Vec<usize> = (1..=4).filter_map(|i| st.follow(root, i)).collect();
    if arrays.len() != 4 {
        return Err(StoreError::Corrupt("KV root names no arrays"));
    }
    let (n, root_dig) = (st.get(root, 0), st.obj_digest(root));
    let mut tx = st.begin()?;
    let mut prev = chain.first().map(|r| r.obj);
    for op in ops {
        let (k, v, code) = match op {
            Op::Put(k, v) => (k.as_bytes(), *v, OP_PUT),
            Op::Remove(k) => (k.as_bytes(), &[][..], OP_REMOVE),
        };
        let len = HEAD + k.len().div_ceil(8) + v.len().div_ceil(8);
        let rec = match st.alloc(&mut tx, len as i64, DIGEST_ARR_I64) {
            Ok(r) => r,
            Err(StoreError::ArenaFull { .. }) => return Ok(Appended::Compact(Why::Full)),
            Err(e) => return Err(e),
        };
        match prev {
            Some(p) => st.link(rec, 0, p),
            None => st.put_cell(rec, 0, 0),
        }
        st.put_cell(rec, 1, code);
        st.put_cell(rec, 2, k.len() as i64);
        st.put_cell(rec, 3, v.len() as i64);
        write_packed(st, rec, HEAD, k);
        write_packed(st, rec, HEAD + k.len().div_ceil(8), v);
        st.seal(rec);
        prev = Some(rec);
    }
    let new_root = match st.alloc(&mut tx, ROOT_V3 as i64, root_dig) {
        Ok(r) => r,
        Err(StoreError::ArenaFull { .. }) => return Ok(Appended::Compact(Why::Full)),
        Err(e) => return Err(e),
    };
    st.put_cell(new_root, 0, n);
    for (i, a) in arrays.iter().enumerate() {
        st.link(new_root, i + 1, *a);
    }
    st.put_cell(new_root, 5, VERSION_DELTA);
    match prev {
        Some(p) => st.link(new_root, 6, p),
        None => st.put_cell(new_root, 6, 0),
    }
    st.put_cell(new_root, 7, (chain.len() + ops.len()) as i64);
    st.seal(new_root);
    tx.sup_delta += dead;
    Ok(Appended::Delta(st.commit_bytes(&tx, new_root)))
}

/// Append `ops` as delta records and a new root, or say why the caller must compact instead.
/// The caller's in-memory `Kv` is the truth either way; this only decides how it is written.
pub fn append_delta(st: &mut Store, ops: &[Op]) -> Result<Appended, StoreError> {
    append_delta_with(st, ops, MAX_DELTAS, DEAD_DIV)
}

