//! `Kv::decode`, and its blob copy A CELL AT A TIME (W-KVDEC, 2026-10-07).
//!
//! MEASURED before this (kv/decode/tests.rs `bench_decode_dubin`, native release, one A78
//! core, a 548,420-byte catalogue of 178 entries): `decode` 998 us of `Catalog::load`'s
//! 2,083 us -- a fifth of the Free plan's 10 ms CPU cap, at every one of the Worker's
//! `Catalog::load` sites. The cost was `blob_byte`: a version branch, a bounds-checked
//! cell load and a shift FOR EVERY BYTE, collected through an iterator. A v2 cell IS eight
//! packed little-endian bytes, so the copy below takes the cells as one slice and appends
//! `to_le_bytes` of each: the version branch runs once per entry, the bounds check once
//! per slice.
//!
//! NOTHING ABOUT WHAT IS READ CHANGED: the same checks in the same order refuse the same
//! images (every count, offset and length is still a claim, bounded before it is used),
//! and the bytes are the bytes `blob_byte` returned -- decode/tests.rs runs the old copy
//! through `decode_with` beside the new one and compares, corrupt images included.

use super::{delta, Kv};
use crate::Store;

impl Kv {
    /// Read all entries out of a store, believing every payload cell: the CRC is checked
    /// by `load` / `load_checked` (verify.rs, W-CRC) before this runs, and nothing else calls it.
    ///
    /// EVERY NUMBER IN HERE CAME OUT OF THE IMAGE, and an image arrives over a
    /// network from a Durable Object. The count, the four offsets and the four
    /// lengths are all claims; none of them was checked, and one flipped bit in
    /// a key's length -- 9 with bit 33 set is 8589934601 -- turned the collect
    /// below into `memory allocation of 8589934601 bytes failed`. An allocation
    /// that large does not return an error: the process ABORTS. On a Worker
    /// that is the isolate, for every tenant sharing it, from one corrupt byte.
    ///
    /// So a slice that does not fit the array it names is not a slice, and an
    /// image holding one is not a KV image. `None` here is `HubError::NotAHub`
    /// at the caller -- a refusal it can report, which is the answer a caller
    /// can act on. Truncating to what fits would be the other failure this
    /// crate keeps finding: a shorter image that still looks valid.
    pub(crate) fn decode(st: &Store) -> Option<Kv> {
        Self::decode_with(st, blob_bytes)
    }

    /// `decode` with the blob copy passed in: the tests run the old byte-at-a-time copy
    /// through the SAME checks and compare. `copy(st, blob, ver, off, len)` is called only
    /// with a byte range `fits` has bounded by the blob.
    pub(super) fn decode_with(st: &Store, copy: fn(&Store, usize, i64, usize, usize) -> Option<Vec<u8>>) -> Option<Kv> {
        let root = st.root()?;
        let ver = Self::version(st);
        if ver > delta::VERSION_DELTA {
            return None;
        }
        // v3 (W-DELTA): the base below, then the chain replayed. The chain is walked FIRST so
        // a chain that does not hold refuses the image before anything is decoded.
        let chain = delta::chain_in(st, root)?;
        // A blob of c cells holds c bytes in v1 and 8c in v2; every slice below is a
        // BYTE slice and is bounded by this.
        let bytes_of = |cells: usize| -> Option<usize> {
            if ver >= 2 { cells.checked_mul(8) } else { Some(cells) }
        };
        let n = st.get(root, 0);
        let kidx = st.follow(root, 1)?;
        let kblob = st.follow(root, 2)?;
        let vidx = st.follow(root, 3)?;
        let vblob = st.follow(root, 4)?;
        // Each entry owns two cells in each index, so the count is bounded by
        // the index arrays that are really there -- not by the root's word.
        let (kidx_cells, vidx_cells) = (st.obj_cells(kidx), st.obj_cells(vidx));
        let (kblob_cells, vblob_cells) = (bytes_of(st.obj_cells(kblob))?, bytes_of(st.obj_cells(vblob))?);
        if n < 0 {
            return None;
        }
        let n = n as usize;
        if n.checked_mul(2)? > kidx_cells.min(vidx_cells) {
            return None;
        }
        let fits = |off: i64, len: i64, cells: usize| -> Option<usize> {
            if off < 0 || len < 0 {
                return None;
            }
            let end = (off as usize).checked_add(len as usize)?;
            if end > cells {
                return None;
            }
            Some(len as usize)
        };
        let mut entries = Vec::with_capacity(n);
        // EVERY ENTRY OWNS ITS OWN BYTES, so the entries together cannot be
        // larger than the two blobs (W-AUDIT S2, 2026-09-27). Each entry was
        // bounded by its blob; n entries that all name the whole blob read
        // n × blob, and a crafted index made one `load` allocate quadratically
        // in the image. An index that over-claims the blobs is refused.
        let mut budget = kblob_cells.checked_add(vblob_cells)?;
        for i in 0..n {
            let ko = st.get(kidx, 2 * i);
            let kl = st.get(kidx, 2 * i + 1);
            let vo = st.get(vidx, 2 * i);
            let vl = st.get(vidx, 2 * i + 1);
            let kl = fits(ko, kl, kblob_cells)?;
            let vl = fits(vo, vl, vblob_cells)?;
            budget = budget.checked_sub(kl.checked_add(vl)?)?;
            let (ko, vo) = (ko as usize, vo as usize);
            // KEYS ARE UTF-8 BYTES AND MUST BE DECODED AS UTF-8. This read
            // `(byte as char)`, which is not a decode at all -- in Rust that
            // maps a `u8` to the code point of the same value, which is exactly
            // Latin-1. The write side has always been `k.as_bytes()`, so every
            // non-ASCII key was stored correctly and read back wrong, and the
            // wrong string was then written back as ITS OWN UTF-8 -- so the
            // damage COMPOUNDED on every round trip: `ujë` became `ujÃ«`, then
            // `ujÃÂ«`, then `ujÃÂÃÂ«`, and the dish it identified became a
            // new dish each time. It reached production as duplicate products on
            // an Albanian menu, which is the whole product's alphabet.
            //
            // Lossy rather than strict: a key that is already damaged must still
            // be readable, or this fix would make an affected image unopenable
            // instead of repairable.
            let k = key_of(copy(st, kblob, ver, ko, kl)?);
            let v = copy(st, vblob, ver, vo, vl)?;
            entries.push((k, v));
        }
        // A chain merges into SORTED keys only (zc/overlay.rs says why); none is written onto
        // any other base, so an image that has one is refused rather than guessed at.
        if !chain.is_empty() && !entries.windows(2).all(|w| w[0].0 < w[1].0) {
            return None;
        }
        let mut kv = Kv { entries };
        kv.replay(st, &chain);
        Some(kv)
    }
}

/// Bytes `off .. off + len` of a blob in version `ver`, copied a cell at a time.
///
/// v2+: cells `off/8 ..= (off+len-1)/8`, each as its eight little-endian bytes, the first
/// cell entered at `off % 8` and the last one cut at the end of the range. v1: one byte
/// per cell, the low byte (`as u8`, exactly what `blob_byte` did).
///
/// `None` only if the cells are not in the store -- which `decode` already rules out:
/// `fits` bounds the range by `obj_cells`, and that is clamped to the cells that exist.
/// It is a refusal rather than the zeros `Store::get` would have read past the end.
fn blob_bytes(st: &Store, blob: usize, ver: i64, off: usize, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    if len == 0 {
        return Some(out);
    }
    let base = blob.checked_add(2)?;
    if ver >= 2 {
        let (first, last) = (off >> 3, (off + len - 1) >> 3);
        let cells = st.cells.get(base.checked_add(first)?..=base.checked_add(last)?)?;
        // Each cell's eight little-endian bytes appended, the first one entered at
        // `off % 8`, then the tail cut: one pass. Same speed as resize + copy_within
        // (bench_decode_dubin, 2026-10-09) and 514 B less wasm.
        let skip = off & 7;
        out.reserve_exact(len + 8);
        for (i, c) in cells.iter().enumerate() {
            out.extend_from_slice(&c.to_le_bytes()[if i == 0 { skip } else { 0 }..]);
        }
        out.truncate(len);
    } else {
        let at = base.checked_add(off)?;
        let cells = st.cells.get(at..at.checked_add(len)?)?;
        out.extend(cells.iter().map(|&c| c as u8));
    }
    Some(out)
}

/// A key's bytes as a `String`, lossy exactly as `String::from_utf8_lossy` is, but without
/// a second copy when the bytes are already UTF-8 (every key a writer here produces).
/// Kept over a plain `from_utf8_lossy` (823 B less wasm) on purpose: the Worker is wasm32
/// too, and there the copy costs CPU under the 10 ms cap on every catalog load.
fn key_of(kb: Vec<u8>) -> String {
    String::from_utf8(kb).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

#[cfg(test)]
mod tests;
