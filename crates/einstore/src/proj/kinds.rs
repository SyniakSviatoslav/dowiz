//! PROJ KINDS 3 AND 4 (LV-INC-A, R-BEBOP-LEVERS-2026-10-27 §2.4): a memo whose OUT is more
//! than one number. The PROJ object, its table and FMT = 1 are unchanged; each kind has its
//! own OUT layout, so a reader that knows only kinds 1/2 refuses these rows as "crc" and
//! refolds -- never misreads them.
//!
//! ```text
//! KIND 3 BLOB   input = the event log (tip = the newest record's id0/id1, kind 1's rule)
//!   OUTB [value, count, nbytes, bytes packed 8/cell LE]    3 + ceil(nbytes/8) cells
//!   value = FNV-1a 64 over the bytes; count = the log's record count
//!   step  = BlobFold::step(old bytes, records after the tip, oldest first)
//! KIND 4 ZSET   input = a KV (v1/v2/v3)
//!   OUTZ [value, count, m, w, (c0..c(w-1), weight) x m]   4 + m(w+1) cells, rows ascending
//!   value = zset::digest(rows); count = the KV root's cell 0
//!   tip0  = the newest delta record's offset (0 = no chain)
//!   tip1  = FINGERPRINT: FNV-1a 64 over (offset, h1) as LE8 of the four base arrays, then of
//!           every chain record from the OLDEST up to tip0
//!   step  = rows + Q(zset::from_kv_chain(records newer than tip0)), Q linear (ZLinear)
//! ```
//! THE FINGERPRINT is why kind 4 is safer than kind 2 (which keys on the root alone): a base
//! rewritten in place (`commit_into`) moves the arrays' offsets, and a record or array
//! re-sealed at the same generation moves its h1 (the crc is its high half) -- either way the
//! memo is refolded, not stepped. NOT SEEN, said: a crc32 collision on such a rewrite.
use super::{
    follow_in, get_in, input_state, log_step, lookup_why, obj_crc_ok_in, obj_digest_in, obj_len_in, proj_key,
    tip16, Cells, How, Out, Store, StoreError, DIGEST_OUT,
};
use crate::evlog::{EvLog, Record};
use crate::kv::delta::{chain_in, version_in, VERSION_DELTA};
use crate::kv::FNV_OFFSET;
use crate::zset::{self, KvRow, ZSet};

pub const KIND_BLOB: i64 = 3;
pub const KIND_ZSET: i64 = 4;
pub const LAYOUT_OUTB: &str = "OUTB{i64,i64,i64,[i64]}";
pub const LAYOUT_OUTZ: &str = "OUTZ{i64,i64,i64,i64,[i64]}";
/// `st_digest(LAYOUT_*)`, pinned; `kinds/tests.rs` re-derives them.
pub const DIGEST_OUTB: i64 = 4157677073;
pub const DIGEST_OUTZ: i64 = 85289679;
/// The widest row a reader accepts (a claim in the image; bounds the arithmetic).
pub const MAX_WIDTH: i64 = 64;

/// A fold over the event log whose result is bytes, extendable record by record:
/// `step(full(a), b) == full(a ++ b)` is the law, and `kinds/tests.rs` checks it.
pub trait BlobFold {
    fn full(&self, recs: &[Record]) -> Vec<u8>;
    fn step(&self, acc: &[u8], newer: &[Record]) -> Vec<u8>;
}

/// A LINEAR query over a KV's Z-set: `apply(a + b) == apply(a) + apply(b)`; rows of `width` cells.
pub trait ZLinear {
    fn width(&self) -> usize;
    fn apply(&self, d: &ZSet<KvRow>) -> ZSet<Vec<i64>>;
}

/// The identity query: each KV row as its two node-key cells (`zset::kv_row_cells`).
pub struct Identity;
impl ZLinear for Identity {
    fn width(&self) -> usize {
        2
    }
    fn apply(&self, d: &ZSet<KvRow>) -> ZSet<Vec<i64>> {
        d.map(zset::kv_row_cells)
    }
}

/// The fixture's blob fold: record count u64 ‖ payload bytes u64 ‖ the LOG fold u64, all LE.
pub struct LogSummary;
impl LogSummary {
    fn add(acc: [u64; 3], recs: &[Record]) -> Vec<u8> {
        let [mut n, mut b, mut h] = acc;
        for r in recs {
            n += 1;
            b += r.payload.len() as u64;
            h = log_step(h, r);
        }
        [n, b, h].iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}
impl BlobFold for LogSummary {
    fn full(&self, recs: &[Record]) -> Vec<u8> {
        Self::add([0, 0, FNV_OFFSET], recs)
    }
    fn step(&self, acc: &[u8], newer: &[Record]) -> Vec<u8> {
        let w = |i: usize| acc.get(i * 8..i * 8 + 8).map_or(0, |s| u64::from_le_bytes(s.try_into().unwrap_or([0; 8])));
        Self::add([w(0), w(1), w(2)], newer)
    }
}

fn fnv_cells(h: u64, cells: &[i64]) -> u64 {
    cells.iter().flat_map(|c| c.to_le_bytes()).fold(h, |h, b| (h ^ b as u64).wrapping_mul(crate::kv::FNV_PRIME))
}

/// FNV-1a 64 of a blob's bytes, as the i64 OUTB's value cell holds.
pub fn blob_value(bytes: &[u8]) -> i64 {
    bytes.iter().fold(FNV_OFFSET, |h, &b| (h ^ b as u64).wrapping_mul(crate::kv::FNV_PRIME)) as i64
}

/// Is `out` the OUT layout `kind` names, with its length rule and its crc?
pub(super) fn out_ok<C: Cells + ?Sized>(c: &C, kind: i64, out: usize) -> bool {
    let len = obj_len_in(c, out);
    let g = |i| get_in(c, out, i);
    let shape = match kind {
        1 | 2 => obj_digest_in(c, out) == DIGEST_OUT && len == 2,
        KIND_BLOB => obj_digest_in(c, out) == DIGEST_OUTB && len >= 3 && g(2) >= 0 && len == 3 + (g(2) + 7) / 8,
        KIND_ZSET => {
            let (m, w) = if len >= 4 { (g(2), g(3)) } else { (-1, 0) };
            obj_digest_in(c, out) == DIGEST_OUTZ
                && m >= 0
                && (1..=MAX_WIDTH).contains(&w)
                && m.checked_mul(w + 1).and_then(|x| x.checked_add(4)) == Some(len)
        }
        _ => false,
    };
    shape && (out + 2 + len as usize) <= c.n_cells() && obj_crc_ok_in(c, out)
}

/// The fingerprint of a KV root's base and of its chain records `oldest..=upto` (chain index;
/// `None` = the base alone). Module header.
fn fingerprint<C: Cells + ?Sized>(c: &C, root: usize, chain: &[crate::kv::delta::Rec], upto: Option<usize>) -> Option<i64> {
    let mut cells = Vec::new();
    for i in 1..=4 {
        let a = follow_in(c, root, i)?;
        cells.extend([a as i64, c.cell_at(a + 1)]);
    }
    if let Some(j) = upto {
        for r in chain[j..].iter().rev() {
            cells.extend([r.obj as i64, c.cell_at(r.obj + 1)]);
        }
    }
    Some(fnv_cells(FNV_OFFSET, &cells) as i64)
}

/// The KV versions kind 4 reads: v1-v3, the ones with a base and a delta chain. A v4 root
/// (W-ES-P9) keeps the TREE ref in cell 6 and its DEPTH in cell 7 -- the cells `chain_in` reads
/// as the newest record and the chain length -- so it is refused BY NAME before any walk
/// (`StoreError::Limit { what: KIND4_VERSIONS, .. }`), never handed to the chain reader.
pub const KIND4_VERSIONS: &str = "KV version kind 4 reads";

/// Refuse a KV root whose version kind 4 does not read (module header).
fn kind4_version<C: Cells + ?Sized>(c: &C, root: usize) -> Result<(), StoreError> {
    let v = version_in(c, root);
    if v > VERSION_DELTA {
        return Err(StoreError::Limit { what: KIND4_VERSIONS, got: v, max: VERSION_DELTA });
    }
    Ok(())
}

/// The kind-4 tip of the KV at `root`: [newest record offset or 0, fingerprint]. `None` on a
/// version kind 4 does not read (a v4 tree has no chain to step).
pub(super) fn kv_tip<C: Cells + ?Sized>(c: &C, root: usize) -> Option<[i64; 2]> {
    kind4_version(c, root).ok()?;
    let chain = chain_in(c, root)?;
    let fp = fingerprint(c, root, &chain, if chain.is_empty() { None } else { Some(0) })?;
    Some([chain.first().map_or(0, |r| r.obj as i64), fp])
}

/// The rows an OUTZ holds (`out_ok` has bounded them).
pub fn zset_of<C: Cells + ?Sized>(c: &C, out: usize) -> ZSet<Vec<i64>> {
    let (m, w) = (get_in(c, out, 2) as usize, get_in(c, out, 3) as usize);
    let row = |i: usize| -> (Vec<i64>, i64) {
        let at = 4 + i * (w + 1);
        ((0..w).map(|q| get_in(c, out, at + q)).collect(), get_in(c, out, at + w))
    };
    ZSet::from_pairs((0..m).map(row).collect())
}

/// The bytes an OUTB holds.
pub fn blob_of<C: Cells + ?Sized>(c: &C, out: usize) -> Vec<u8> {
    let n = get_in(c, out, 2).max(0) as usize;
    (0..n).map(|b| (get_in(c, out, 3 + (b >> 3)) >> (8 * (b & 7))) as u8).collect()
}

fn outz_cells(z: &ZSet<Vec<i64>>, w: usize, count: i64) -> Vec<i64> {
    let mut v = vec![zset::digest(z), count, z.len() as i64, w as i64];
    for (r, wt) in z.rows() {
        v.extend_from_slice(r);
        v.push(*wt);
    }
    v
}

fn outb_cells(b: &[u8], count: i64) -> Vec<i64> {
    let mut v = vec![blob_value(b), count, b.len() as i64];
    v.extend(b.chunks(8).map(|ch| {
        let mut w = [0u8; 8];
        w[..ch.len()].copy_from_slice(ch);
        i64::from_le_bytes(w)
    }));
    v
}

/// Kind 3: a blob fold over the event log, memoised and extended like kind 1.
pub fn eval_blob(st: &mut Store, code: i64, fn_key: i64, f: &dyn BlobFold) -> Result<(Vec<u8>, How), StoreError> {
    let key = proj_key(code, fn_key, KIND_BLOB);
    let (gen, tip) = input_state(st, KIND_BLOB).ok_or(StoreError::Corrupt("no log root"))?;
    let n = EvLog::len(st) as i64;
    let all = |st: &Store| {
        let mut rs = EvLog::walk(st);
        rs.reverse();
        f.full(&rs)
    };
    let (blob, how) = match lookup_why(st, key) {
        Err("none") => (all(st), How::Full),
        Err(why) => (all(st), How::Refold(why)),
        Ok(p) if p.code != code => (all(st), How::Refold("code")),
        Ok(p) if p.input_gen == gen && p.tip == tip && p.count == n => return Ok((blob_of(st, p.out), How::Hit)),
        Ok(p) => {
            let newer = EvLog::walk_until(st, |r| tip16(r) == p.tip);
            let found = newer.last().is_some_and(|r| tip16(r) == p.tip);
            let k = newer.len().saturating_sub(1);
            if !found || p.count + k as i64 != n {
                (all(st), How::Refold("tip"))
            } else {
                let mut tail = newer[..k].to_vec();
                tail.reverse();
                (f.step(&blob_of(st, p.out), &tail), How::Step(k))
            }
        }
    };
    super::write::put(st, key, KIND_BLOB, code, gen, tip, fn_key, Out::Cells(DIGEST_OUTB, &outb_cells(&blob, n)))?;
    Ok((blob, how))
}

/// Kind 4: a linear query's Z-set over a KV, memoised and stepped per delta record.
pub fn eval_zset(st: &mut Store, code: i64, fn_key: i64, q: &dyn ZLinear) -> Result<(ZSet<Vec<i64>>, How), StoreError> {
    let key = proj_key(code, fn_key, KIND_ZSET);
    let bad = StoreError::Corrupt("the KV does not hold");
    let root = crate::root_in(st).ok_or(StoreError::NoSuperblock)?;
    kind4_version(st, root)?;
    let (gen, tip) = input_state(st, KIND_ZSET).ok_or(StoreError::Corrupt("the KV delta chain does not hold"))?;
    let count = get_in(st, root, 0);
    let all = |st: &Store| zset::from_kv(st, root).map(|z| q.apply(&z));
    let (z, how) = match lookup_why(st, key) {
        Err("none") => (all(st), How::Full),
        Err(why) => (all(st), How::Refold(why)),
        Ok(p) if p.code != code => (all(st), How::Refold("code")),
        Ok(p) if get_in(st, p.out, 3) != q.width() as i64 => (all(st), How::Refold("width")),
        Ok(p) if p.input_gen == gen && p.tip == tip && p.count == count => return Ok((zset_of(st, p.out), How::Hit)),
        Ok(p) => {
            let chain = chain_in(st, root).ok_or(bad)?;
            let j = if p.tip[0] == 0 { Some(chain.len()) } else { chain.iter().position(|r| r.obj as i64 == p.tip[0]) };
            let upto = j.filter(|&j| j < chain.len());
            match j.filter(|_| fingerprint(st, root, &chain, upto) == Some(p.tip[1])) {
                Some(k) => (zset::from_kv_chain(st, root, k).map(|d| zset_of(st, p.out).add(&q.apply(&d))), How::Step(k)),
                None => (all(st), How::Refold("tip")),
            }
        }
    };
    let z = z.ok_or(StoreError::Corrupt("the KV does not decode"))?;
    super::write::put(st, key, KIND_ZSET, code, gen, tip, fn_key, Out::Cells(DIGEST_OUTZ, &outz_cells(&z, q.width(), count)))?;
    Ok((z, how))
}

#[cfg(test)]
mod tests;
