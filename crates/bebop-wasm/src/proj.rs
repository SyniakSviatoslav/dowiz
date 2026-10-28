//! The projection reader (DG5, SPEC-BEBOP-DAG-RUNTIME-2026-09-28 §6): a log image that
//! carries a memo in its projection table (superblock cell 5), read to three numbers --
//! the record count and the log fold (exactly `log_view`'s) and the memo's value, read
//! through a BORROWED `bebop_store::View` (no copy of the image for the memo).
//!
//! It REFUSES, never panics: a valid superblock whose cells 13-14 are not zero (RT S-1: a
//! format this reader does not know), an image with no memo, and a memo that no longer
//! describes the log (its input generation, tip or count moved -- `proj::is_current`).
//! The memo's value is NOT compared to the fold here: that is the gate's comparison, and a
//! reader that refused a disagreeing memo would hide which side disagreed.
//!
//! `bw_proj` is a wasm export only under the `proj` feature (Cargo.toml), built apart by
//! gate.sh, so the Worker's reader module -- and `bytes.baseline` -- do not carry it.

use crate::{log_view, Refusal};
use bebop_store::proj::{is_current, lookup, projtab};
use bebop_store::{get_in, sb_valid_in, Cells, View, SB_A, SB_B};
/// Kinds 3/4 (LV-INC-A) are compiled only where they are read -- the `proj` module and the native
/// tests: in the Worker's reader module the extra code moved inlining and cost 131 bytes
/// (46067 -> 46198, MEASURED against a HEAD control build) for functions nothing there calls.
#[cfg(any(test, feature = "proj"))]
use kinds::{blob_view, zset_view};

/// A valid superblock has non-zero cells 13-14 (RT S-1).
pub const S1_BROKEN: i32 = 7;
/// No projection table, or its first row does not lead to a checked memo.
pub const NO_MEMO: i32 = 8;
/// The memo describes an input that is not the image's any more.
pub const STALE_MEMO: i32 = 9;
/// LV-INC-A: a kind 3/4 memo whose value is not the digest of its own OUT (a lying memo).
pub const BAD_MEMO: i32 = 11;

/// A log image with its memo, read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjView {
    pub n: i64,
    pub fold: i64,
    pub memo: i64,
}

/// RT S-1: every VALID superblock keeps cells 13 and 14 at zero.
pub fn s1_holds(bytes: &[u8]) -> bool {
    let v = View::new(bytes);
    [SB_A, SB_B].iter().all(|&at| !sb_valid_in(&v, at) || (v.cell_at(at + 13) == 0 && v.cell_at(at + 14) == 0))
}

fn status_of(r: Refusal) -> i32 {
    match r {
        Refusal::NoSuperblock => crate::abi::NO_SUPERBLOCK,
        Refusal::NotAKv => crate::abi::NOT_A_KV,
        Refusal::NotALog => crate::abi::NOT_A_LOG,
        Refusal::Truncated { .. } => crate::abi::TRUNCATED,
        Refusal::BadCrc { .. } => crate::abi::BAD_CRC,
    }
}

/// Read a log image and its first memo; a status code on refusal.
pub fn proj_view(bytes: &[u8]) -> Result<ProjView, i32> {
    if !s1_holds(bytes) {
        return Err(S1_BROKEN);
    }
    let v = View::new(bytes);
    // LV-INC-A: the first memo's kind decides the input; kind 4's is a KV, not a log.
    let first = projtab(&v).and_then(|t| lookup(&v, get_in(&v, t, 1)));
    #[cfg(any(test, feature = "proj"))]
    if let Some(p) = first.filter(|p| p.kind == bebop_store::proj::KIND_ZSET) {
        return zset_view(bytes, &v, &p);
    }
    let log = log_view(bytes).map_err(status_of)?;
    let p = first.ok_or(NO_MEMO)?;
    #[cfg(any(test, feature = "proj"))]
    if p.kind == bebop_store::proj::KIND_BLOB {
        return blob_view(bytes, &v, &p);
    }
    if !is_current(&v, &p) {
        return Err(STALE_MEMO);
    }
    Ok(ProjView { n: log.len, fold: log.fold, memo: p.value })
}

/// Three cells into `out`: record count, fold, memo value.
///
/// # Safety
/// `ptr..ptr+len` is readable and `out` points at three writable i64 cells.
#[cfg_attr(feature = "proj", no_mangle)]
pub unsafe extern "C" fn bw_proj(ptr: *const u8, len: usize, out: *mut i64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return crate::abi::NULL_ARG;
    }
    match proj_view(core::slice::from_raw_parts(ptr, len)) {
        Ok(p) => {
            *out = p.n;
            *out.add(1) = p.fold;
            *out.add(2) = p.memo;
            crate::abi::OK
        }
        Err(s) => s,
    }
}

#[cfg(any(test, feature = "proj"))]
mod kinds;

#[cfg(test)]
mod tests;
