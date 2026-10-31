//! SPEC (W-MG-M5, R-MAINGOAL-A §2f, §5.2, §5.3): the object that makes an ARCHIVE file read
//! itself. It names two blobs -- the reader compiled to wasm32 (`crates/bebop-wasm-reader`)
//! and the format text -- and carries the reader's sha256 as the file's CLAIM about it.
//!
//! ```text
//! SPEC v1  {VERSION=1, ref WASM, ref TEXT, sha256 4 cells, abi}     8 cells
//! BLOB     {len, bytes packed 8 to a cell, little-endian}           1 + ceil(len / 8) cells
//! ```
//!
//! ── FIRST OBJECT, FORWARD REFS ──
//!
//! SPEC is THE FIRST OBJECT OF THE ARENA (cell `ARENA` = 1024) of a self-describing file, so a
//! host with no parser at all can find the reader: read h0 at cell 1024, its high 32 bits must be
//! `DIGEST_SPEC`; payload cell 1 (cell 1027) is an object-relative ref, `W = 1024 + ref` is the
//! WASM blob; cell `W + 2` is its byte length `L`; the bytes are `[(W + 3) * 8, (W + 3) * 8 + L)`.
//! That is `examples/read.mjs`'s whole bootstrap, and `Doc::spec_first` is the same six lines in
//! Rust so the two cannot drift. The refs point FORWARD (the blobs follow SPEC): refs that point
//! back are KV v4's rule (FORMAT §9.1), not the container's, and `Store::follow` accepts either.
//!
//! ── ARCHIVE LEVEL, NOT EVERY IMAGE (§2f) ──
//!
//! A working image keeps `DOC.SPEC = 0`. SPEC is written once, into a FRESH file:
//! `Doc::init_described_bytes` (a file self-describing from birth) refuses a store whose arena is
//! not empty, because "first object" is a promise to a host that cannot parse. An existing DOC
//! becomes an archive by SPEC-first + a copy (`copy.rs`, W-MG-M1B); the cost is one SPEC per
//! archive FILE: `2 + 8` cells for SPEC, `2 + 1 + ceil(len/8)` per blob -- measured by
//! `spec/tests.rs::the_cost_of_self_description_is_the_blobs_plus_three_headers`.
//!
//! ── TESTIMONY, NOT AUTHORITY (§5.3) ──
//!
//! The embedded reader runs in a sandbox with no I/O, and its sha256 is NAMED here and in the
//! format document; a host prints both and a person compares. The file vouching for its own
//! reader proves nothing by itself: the authority is the format text (`ref TEXT`, which a
//! stdlib-only reader such as `tools/verify/bebop-verify.py` follows with no wasm at all) and
//! the independent readers `conformance.sh` holds to one answer. A reader that returns the
//! SPEC must say so in words (`bebop-wasm-reader` `status.rs`), never imply more.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{EvLog, Store, StoreError, Tx, ARENA};
use super::keys::{stage_key, stage_keys, Key};
use super::seal::stage_heads;
use super::write::{stage_doc, Refs};
use super::{bytes_to_cells, cells_of, cells_to_bytes, cells_to_id, id_to_cells, Doc, DocError, FLAG_SELF_DESCRIBING};

pub const SPEC_VERSION: i64 = 1;
pub const LAYOUT_SPEC: &str = "SPEC{i64,ref BLOB,ref BLOB,i64,i64,i64,i64,i64}";
/// `st_digest(LAYOUT_SPEC)`: the low 32 bits of sha256, big-endian (`spec/tests.rs` re-derives).
pub const DIGEST_SPEC: i64 = 849619745;
pub const LAYOUT_BLOB: &str = "BLOB{i64,[i64]}";
pub const DIGEST_BLOB: i64 = 799774958;
pub const SPEC_CELLS: usize = 8;
const C_VERSION: usize = 0;
const C_WASM: usize = 1;
const C_TEXT: usize = 2;
const C_SHA: usize = 3;
const C_ABI: usize = 7;
/// The longest blob a SPEC may name: 16 MiB (the reader is ~65 KB, the format text ~40 KB), so
/// a damaged `len` never becomes an allocation.
pub const MAX_BLOB_BYTES: usize = 16 << 20;

/// What a writer embeds: the reader's bytes, the sha256 the writer computed over them (this
/// crate has no hash and does not check it), the format text, and the reader's ABI version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecIn<'a> {
    pub wasm: &'a [u8],
    pub wasm_sha256: [u8; 32],
    pub text: &'a [u8],
    pub abi: i64,
}

/// A SPEC as read: where it is, where its blobs are, what it claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecView {
    pub at: usize,
    pub wasm: Option<usize>,
    pub text: Option<usize>,
    pub wasm_sha256: [u8; 32],
    pub abi: i64,
}

/// One SPEC object; `None` for a wrong digest, version or length. The crc is the caller's
/// question (`Doc::spec` asks it).
pub fn read_spec(st: &Store, at: usize) -> Option<SpecView> {
    if st.obj_digest(at) != DIGEST_SPEC || st.obj_cells(at) != SPEC_CELLS || st.get(at, C_VERSION) != SPEC_VERSION {
        return None;
    }
    Some(SpecView {
        at,
        wasm: st.follow(at, C_WASM),
        text: st.follow(at, C_TEXT),
        wasm_sha256: cells_to_id(&cells_of(st, at, C_SHA, 4)),
        abi: st.get(at, C_ABI),
    })
}

/// A BLOB's bytes, bounded by its length claim, its cap and what the object really holds;
/// `None` for a wrong digest, a length past `MAX_BLOB_BYTES`, or a crc that fails.
pub fn read_blob(st: &Store, at: usize) -> Option<Vec<u8>> {
    if st.obj_digest(at) != DIGEST_BLOB || st.obj_cells(at) < 1 || st.check_obj(at).is_err() {
        return None;
    }
    let len = st.get(at, 0);
    if !(0..=MAX_BLOB_BYTES as i64).contains(&len) {
        return None;
    }
    let n = (len as usize).div_ceil(8);
    if st.obj_cells(at) != 1 + n {
        return None;
    }
    Some(cells_to_bytes(&cells_of(st, at, 1, n), len as usize))
}

pub(crate) fn stage_blob(st: &mut Store, tx: &mut Tx, bytes: &[u8]) -> Result<usize, StoreError> {
    let c = bytes_to_cells(bytes);
    let o = st.alloc(tx, (1 + c.len()) as i64, DIGEST_BLOB)?;
    st.put_cell(o, 0, bytes.len() as i64);
    for (i, v) in c.iter().enumerate() {
        st.put_cell(o, 1 + i, *v);
    }
    st.seal(o);
    Ok(o)
}

/// SPEC, then its two blobs, in that order -- so in a fresh store SPEC lands at `ARENA`.
pub(crate) fn stage_spec(st: &mut Store, tx: &mut Tx, s: &SpecIn<'_>) -> Result<usize, StoreError> {
    let o = st.alloc(tx, SPEC_CELLS as i64, DIGEST_SPEC)?;
    let wasm = stage_blob(st, tx, s.wasm)?;
    let text = stage_blob(st, tx, s.text)?;
    st.put_cell(o, C_VERSION, SPEC_VERSION);
    st.link(o, C_WASM, wasm);
    st.link(o, C_TEXT, text);
    for (i, c) in id_to_cells(&s.wasm_sha256).iter().enumerate() {
        st.put_cell(o, C_SHA + i, *c);
    }
    st.put_cell(o, C_ABI, s.abi);
    st.seal(o);
    Ok(o)
}

impl Doc {
    /// THE BOOTSTRAP, in Rust: is the first object of the arena a SPEC whose WASM blob reads?
    /// `Some(ARENA)` when it is -- the same rule `examples/read.mjs` applies to the bytes.
    pub fn spec_first(st: &Store) -> Option<usize> {
        let s = read_spec(st, ARENA)?;
        read_blob(st, s.wasm?).map(|_| ARENA)
    }

    /// The SPEC the DOC at the root names, its crc checked; `None` = a working image
    /// (`DOC.SPEC = 0`), or a root that is not a DOC.
    pub fn spec(st: &Store) -> Option<SpecView> {
        let at = Doc::open(st).ok()?.spec?;
        st.check_obj(at).ok()?;
        read_spec(st, at)
    }

    /// `init_bytes` for an ARCHIVE: SPEC and its blobs as the first objects of a FRESH store,
    /// then an empty v2 log, an empty HEADS, `genesis` as KEYS, and a DOC naming SPEC with
    /// `FLAG_SELF_DESCRIBING` set. Refused (`Corrupt`) when the arena is not empty: a SPEC
    /// that is not first is one a parserless host cannot find.
    pub fn init_described_bytes(st: &mut Store, flags: i64, genesis: &[Key], spec: &SpecIn<'_>) -> Result<i64, DocError> {
        let mut tx = st.begin()?;
        if st.root().is_some() || tx.cursor != ARENA as i64 {
            return Err(DocError::Corrupt("SPEC must be the first object: the store is not fresh"));
        }
        let spec = stage_spec(st, &mut tx, spec)?;
        // The M1 init, verbatim (`write.rs` `init_bytes`); duplicated rather than threaded through
        // a parameter so that file is not edited by this lane.
        let log = EvLog::stage_init_in(st, &mut tx)?;
        let mut rows = Vec::with_capacity(genesis.len());
        for k in genesis {
            let at = stage_key(st, &mut tx, &Key { admit_rec: None, ..k.clone() })?;
            rows.push((k.actor_id, at));
        }
        let keys = stage_keys(st, &mut tx, &mut rows)?;
        let heads = stage_heads(st, &mut tx, &mut Vec::new())?;
        let refs = Refs { log: Some(log), heads: Some(heads), keys: Some(keys), spec: Some(spec), ..Refs::default() };
        let doc = stage_doc(st, &mut tx, flags | FLAG_SELF_DESCRIBING, refs)?;
        Ok(st.commit_bytes(&tx, doc))
    }

    /// An ARCHIVE of the DOC at `src`: `spec` first in a FRESH `dst`, then `copy_bytes`' verbatim
    /// copy of records, keys, seals, anchors and segments, `FLAG_SELF_DESCRIBING` set. A `src`
    /// that already carries a SPEC stays refused: an archive is not archived again.
    pub fn archive_bytes(src: &Store, dst: &mut Store, spec: &SpecIn<'_>) -> Result<i64, DocError> {
        let mut tx = dst.begin()?;
        if dst.root().is_some() || tx.cursor != ARENA as i64 {
            return Err(DocError::Corrupt("SPEC must be the first object: the store is not fresh"));
        }
        let at = stage_spec(dst, &mut tx, spec)?;
        let doc = super::copy::stage_copy(src, dst, &mut tx, Some(at))?;
        Ok(dst.commit_bytes(&tx, doc))
    }
}

#[cfg(test)]
mod tests;
