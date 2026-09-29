//! Projection nodes in the store: the memo, `fold_step`, and when a memo is refused
//! (SPEC-BEBOP-DAG-RUNTIME-2026-09-28 §6; the Rust twin of `bebop-lang/selfhost/prelude/proj.bp`).
//!
//! ```text
//! superblock cell 5  PROJTAB   (0 = no table; every image before DG5 reads 0 here)
//! PROJTAB  [m, (key64, ref PROJ, kind) x m]          sorted by key64
//! PROJ     [key64, kind, code, fmt, input_gen, tip0, tip1, ref OUT, fn_key]    9 cells
//! OUT      [value, count]
//! ```
//! A PROJ lives in the SAME image as its input, reached only through the table: an image
//! rebuilt from its records (`dowiz_hub::Hub::redact`) carries no memo, which -- not the
//! tip -- is what makes the tree's own redaction safe (it keeps every id; the tip stays).
//!
//! CURRENT AT: `input_gen` = the generation in the INPUT ROOT's h1 (low 32), not the
//! image's (a memo write is itself a commit; only an append or put moves the root); `tip`
//! = the newest record's id0/id1 (kind 1) or the root's offset and h1 (kind 2); `count` =
//! the root's cell 0. HIT: code and all three match. A log memo whose tip is still in the
//! chain is EXTENDED one `log_step` per newer record (S-2); anything else refolds whole.
//! A memo is never trusted past its CRC. NOT SEEN, and said: an in-place rewrite keeping
//! every id at the same generation -- no writer in the tree does that.
use crate::evlog::{EvLog, Record};
use crate::kv::{Kv, FNV_OFFSET, FNV_PRIME};
use crate::nodekey::{Frame, TAG_COMPILE, TAG_PROJECTION};
use crate::{follow_in, get_in, obj_crc_ok_in, obj_digest_in, obj_len_in, pick_in, root_in, Cells, Store, StoreError};

/// Superblock cell naming the projection table (was `layout_table`; no writer ever set it).
pub const SB_PROJTAB: usize = 5;
/// Commit anchor `(mark << 32) | crc32(cells[mark..used))`: bebop's `st_commit` writes it, this crate 0.
pub const SB_ANC: usize = 9;
/// Kind 1: an event-log fold, extended per record (S-2).
pub const KIND_LOG: i64 = 1;
/// Kind 2: a KV snapshot, memoised per generation and re-derived whole (S-3).
pub const KIND_KV: i64 = 2;
/// The PROJ layout's own version, payload cell 3.
pub const FMT: i64 = 1;
pub const LAYOUT_PROJTAB: &str = "PROJTAB{i64,[i64,ref PROJ,i64]}";
pub const LAYOUT_PROJ: &str = "PROJ{i64,i64,i64,i64,i64,i64,i64,ref OUT,i64}";
pub const LAYOUT_OUT: &str = "OUT{i64,i64}";
/// `st_digest(LAYOUT_*)` -- pinned; `tests::digests_are_st_digest_of_their_layouts` re-derives them.
pub const DIGEST_PROJTAB: i64 = 2699843079;
pub const DIGEST_PROJ: i64 = 313398166;
pub const DIGEST_OUT: i64 = 514297014;
const PROJ_CELLS: i64 = 9;
const OUT_CELLS: i64 = 2;

/// How a read was answered. `Refold(why)` = a memo existed and was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How { Hit, Step(usize), Full, Refold(&'static str) }

/// One memo, as read (and CRC-checked) out of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proj {
    pub at: usize,
    pub out: usize,
    pub key: i64,
    pub kind: i64,
    pub code: i64,
    pub input_gen: i64,
    pub tip: [i64; 2],
    pub value: i64,
    pub count: i64,
    pub fn_key: i64,
}

/// This writer's `code_version` (RT §2.3 field 1): differs from any bebop binary's, so a memo one
/// of them wrote is never used by the other (K-4: the code is in the key).
pub fn rust_code() -> i64 {
    Frame::new(TAG_COMPILE).bytes(b"crates/bebop-store/src/proj.rs fold v1").k64()
}

/// A projection fn's key. STAND-IN until DG4 gives every fn a compile-node K64: the K64 of
/// a 'C' frame holding the fn's name. bebop builds the same frame (`proj.bp` `pj_fn_key`).
pub fn fn_key(name: &str) -> i64 {
    Frame::new(TAG_COMPILE).bytes(name.as_bytes()).k64()
}

/// The table key: K64 of `'P' ‖ code ‖ fn_key ‖ [input 0 ‖ gen 0] ‖ [kind]`. The input is
/// the image's one data root; its generation is NOT in the index key (the key would move on
/// every append and no memo could ever be found) -- it is recorded in the entry instead.
pub fn proj_key(code: i64, fn_key: i64, kind: i64) -> i64 {
    let mut f = Frame::new(TAG_PROJECTION);
    f.i64(code).i64(fn_key);
    let at = f.open();
    f.i64(0).i64(0);
    f.close(at);
    let at = f.open();
    f.i64(kind);
    f.close(at);
    f.k64()
}

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// `fold_step` (S-2): ONE frame of the four-reader LOG fold -- FNV-1a 64 over
/// `seq(8) ‖ payload_len(8) ‖ payload`, little-endian (crates/bebop-wasm/src/lib.rs).
pub fn log_step(h: u64, r: &Record) -> u64 {
    let h = fnv(h, &r.actor_seq.to_le_bytes());
    let h = fnv(h, &(r.payload.len() as u64).to_le_bytes());
    fnv(h, &r.payload)
}

/// The whole LOG fold, oldest first, and how many records it read.
pub fn log_fold_full(st: &Store) -> (i64, usize) {
    let mut rs = EvLog::walk(st);
    rs.reverse();
    (rs.iter().fold(FNV_OFFSET, log_step) as i64, rs.len())
}

/// The live projection table, if the live superblock names one that is really a table.
pub fn projtab<C: Cells + ?Sized>(c: &C) -> Option<usize> {
    let sb = pick_in(c)?;
    let t = c.cell_at(sb.at + SB_PROJTAB);
    if t < 1024 || t as usize + 2 > c.n_cells() {
        return None;
    }
    let t = t as usize;
    (obj_digest_in(c, t) == DIGEST_PROJTAB && obj_crc_ok_in(c, t)).then_some(t)
}

/// The entry for `key`, CRC-checked all the way to its OUT; `None` = no usable memo.
/// A table or memo that is not what it claims reads as absent, and the caller refolds.
pub fn lookup<C: Cells + ?Sized>(c: &C, key: i64) -> Option<Proj> {
    lookup_why(c, key).ok()
}

fn lookup_why<C: Cells + ?Sized>(c: &C, key: i64) -> Result<Proj, &'static str> {
    let t = projtab(c).ok_or("none")?;
    let m = get_in(c, t, 0);
    if m < 0 || 1 + 3 * m > obj_len_in(c, t) {
        return Err("crc");
    }
    let i = (0..m as usize).find(|&i| get_in(c, t, 1 + 3 * i) == key).ok_or("none")?;
    let at = follow_in(c, t, 2 + 3 * i).ok_or("crc")?;
    let ok = |o: usize, d: i64, n: i64| obj_digest_in(c, o) == d && obj_len_in(c, o) == n && obj_crc_ok_in(c, o);
    if !ok(at, DIGEST_PROJ, PROJ_CELLS) || get_in(c, at, 0) != key || get_in(c, at, 3) != FMT {
        return Err("crc");
    }
    let out = follow_in(c, at, 7).ok_or("crc")?;
    if !ok(out, DIGEST_OUT, OUT_CELLS) {
        return Err("crc");
    }
    let g = |i| get_in(c, at, i);
    Ok(Proj {
        at,
        out,
        key,
        kind: g(1),
        code: g(2),
        input_gen: g(4),
        tip: [g(5), g(6)],
        value: get_in(c, out, 0),
        count: get_in(c, out, 1),
        fn_key: g(8),
    })
}

/// The input's state for `kind`: (input_gen, tip) -- see the module header.
fn input_state<C: Cells + ?Sized>(c: &C, kind: i64) -> Option<(i64, [i64; 2])> {
    let root = root_in(c)?;
    let h1 = c.cell_at(root + 1);
    let tip = if kind == KIND_LOG {
        match follow_in(c, root, 1) {
            Some(last) => [get_in(c, last, 3), get_in(c, last, 4)],
            None => [0, 0],
        }
    } else {
        [root as i64, h1]
    };
    Some((h1 & 0xFFFF_FFFF, tip))
}

/// Does the memo describe the image's input as it is now (input_gen, tip, count)? The code
/// is NOT checked: a reader of a memo another program wrote (the four-reader gate) asks this.
pub fn is_current<C: Cells + ?Sized>(c: &C, p: &Proj) -> bool {
    let n = root_in(c).map(|r| get_in(c, r, 0));
    input_state(c, p.kind).is_some_and(|(gen, tip)| p.input_gen == gen && p.tip == tip && Some(p.count) == n)
}

/// The HIT path alone, over any `Cells` -- a `View` answers it without copying the image.
pub fn read_hit<C: Cells + ?Sized>(c: &C, key: i64, code: i64) -> Option<i64> {
    let p = lookup(c, key)?;
    (p.code == code && is_current(c, &p)).then_some(p.value)
}

/// Kind 1: the event log's fold, memoised and extended (RT §6.2).
pub fn eval_log(st: &mut Store, code: i64, fn_key: i64) -> Result<(i64, How), StoreError> {
    let key = proj_key(code, fn_key, KIND_LOG);
    let (gen, tip) = input_state(st, KIND_LOG).ok_or(StoreError::Corrupt("no log root"))?;
    let n = EvLog::len(st) as i64;
    let (value, count, how) = match lookup_why(st, key) {
        Err("none") => {
            let (v, c) = log_fold_full(st);
            (v, c as i64, How::Full)
        }
        Err(why) => refold(st, why),
        Ok(p) if p.code != code => refold(st, "code"),
        Ok(p) if p.input_gen == gen && p.tip == tip && p.count == n => return Ok((p.value, How::Hit)),
        Ok(p) => {
            // Walk from the newest record back to the memo's tip: O(k) records, oldest last.
            let newer = EvLog::walk_until(st, |r| tip16(r) == p.tip);
            let found = newer.last().is_some_and(|r| tip16(r) == p.tip);
            let k = newer.len().saturating_sub(1);
            if !found || p.count + k as i64 != n {
                refold(st, "tip")
            } else {
                let v = newer[..k].iter().rev().fold(p.value as u64, log_step) as i64;
                (v, n, How::Step(k))
            }
        }
    };
    put(st, key, KIND_LOG, code, gen, tip, value, count, fn_key)?;
    Ok((value, how))
}

fn refold(st: &Store, why: &'static str) -> (i64, i64, How) {
    let (v, c) = log_fold_full(st);
    (v, c as i64, How::Refold(why))
}

fn tip16(r: &Record) -> [i64; 2] {
    let w = |q: usize| i64::from_le_bytes(r.id[q * 8..q * 8 + 8].try_into().unwrap_or([0; 8]));
    [w(0), w(1)]
}

/// Kind 2: the KV snapshot root, memoised per generation, never extended (S-3).
pub fn eval_kv(st: &mut Store, code: i64, fn_key: i64) -> Result<(i64, How), StoreError> {
    let key = proj_key(code, fn_key, KIND_KV);
    let (gen, tip) = input_state(st, KIND_KV).ok_or(StoreError::Corrupt("no kv root"))?;
    let whole = |st: &Store| Kv::load(st).map(|kv| kv.snapshot_root_u64() as i64);
    let (value, how) = match lookup_why(st, key) {
        Err("none") => (whole(st), How::Full),
        Err(why) => (whole(st), How::Refold(why)),
        Ok(p) if p.code != code => (whole(st), How::Refold("code")),
        Ok(p) if p.input_gen == gen && p.tip == tip => return Ok((p.value, How::Hit)),
        Ok(p) if p.input_gen != gen => (whole(st), How::Refold("gen")),
        Ok(_) => (whole(st), How::Refold("tip")),
    };
    let value = value.ok_or(StoreError::Corrupt("the KV root does not load"))?;
    let n = Kv::load(st).map_or(0, |kv| kv.entries.len() as i64);
    put(st, key, KIND_KV, code, gen, tip, value, n, fn_key)?;
    Ok((value, how))
}

/// Write the memo: OUT, PROJ, and a new sorted table, in ONE commit that keeps the data root.
#[allow(clippy::too_many_arguments)]
fn put(st: &mut Store, key: i64, kind: i64, code: i64, gen: i64, tip: [i64; 2], value: i64, count: i64, fn_key: i64) -> Result<(), StoreError> {
    let root = root_in(st).ok_or(StoreError::NoSuperblock)?;
    let old = projtab(st);
    let mut rows: Vec<(i64, usize, i64)> = Vec::new();
    if let Some(t) = old {
        let m = st.get(t, 0).max(0) as usize;
        for i in 0..m.min(st.obj_cells(t).saturating_sub(1) / 3) {
            let k = st.get(t, 1 + 3 * i);
            if let Some(p) = st.follow(t, 2 + 3 * i) {
                // A row of the same fn and kind under another code is a previous deploy's.
                let same = st.get(p, 8) == fn_key && st.get(p, 1) == kind;
                if k != key && !same {
                    rows.push((k, p, st.get(t, 3 + 3 * i)));
                }
            }
        }
    }
    let mut tx = st.begin()?;
    let out = st.alloc(&mut tx, OUT_CELLS, DIGEST_OUT)?;
    st.put_cell(out, 0, value);
    st.put_cell(out, 1, count);
    st.seal(out);
    let p = st.alloc(&mut tx, PROJ_CELLS, DIGEST_PROJ)?;
    for (i, v) in [key, kind, code, FMT, gen, tip[0], tip[1], 0, fn_key].into_iter().enumerate() {
        st.put_cell(p, i, v);
    }
    st.link(p, 7, out);
    st.seal(p);
    rows.push((key, p, kind));
    rows.sort_by_key(|r| r.0);
    let t = st.alloc(&mut tx, 1 + 3 * rows.len() as i64, DIGEST_PROJTAB)?;
    st.put_cell(t, 0, rows.len() as i64);
    for (i, &(k, at, kd)) in rows.iter().enumerate() {
        st.put_cell(t, 1 + 3 * i, k);
        st.link(t, 2 + 3 * i, at);
        st.put_cell(t, 3 + 3 * i, kd);
    }
    st.seal(t);
    if let Some(ot) = old {
        tx.sup_delta += 2 + st.obj_len(ot);
    }
    st.commit_bytes_projtab(&tx, root, t as i64);
    Ok(())
}

#[cfg(test)]
mod tests;
