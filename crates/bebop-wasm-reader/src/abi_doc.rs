// SPDX-License-Identifier: MIT OR Apache-2.0
//! The DOC calls of the C surface (W-MG-M5, ABI 2): `bw_heads`, `bw_counts`, `bw_spec`,
//! `bw_unchecked_text`. Same slot, same buffer rule as `abi.rs` (a buffer too small gets the
//! length, nothing written; a refusal is `-status`); split out so each file stays small.

use crate::abi::{put, slot, status_of, ANCHOR_ROW, COUNTS_CELLS, HEAD_ROW};
use crate::status::*;

/// The HEADS rows of a DOC, `HEAD_ROW` bytes each (`actor_id 32 || seq 8 || tip 32`), in table
/// order: the total length, written if it fits; 0 for a KV, a log, or a DOC with no row;
/// `-NOT_OPEN`, `-NULL_ARG`.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_heads(out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    let mut f = Vec::with_capacity(HEAD_ROW * d.heads.len());
    for h in &d.heads {
        f.extend_from_slice(&h.actor_id);
        f.extend_from_slice(&h.seq.to_le_bytes());
        f.extend_from_slice(&h.tip);
    }
    put(&f, out, cap)
}

/// A DOC's counters as `COUNTS_CELLS` little-endian i64 cells -- records, chained, legacy,
/// redacted, broken, seals, forks, heads, keys, admitted, checked, holds, segments, cut,
/// cut_redacted, declared (`DocCounts`): the length (128), written if it fits; `-NOT_A_DOC` for a KV or a log; `-status` when `doc::verify`
/// refuses; `-NOT_OPEN`, `-NULL_ARG`. Runs the verify if `bw_verify(1)` has not.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_counts(out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_mut() else { return -(NOT_OPEN as i64) };
    let c = match d.verify_doc() {
        None => return -(NOT_A_DOC as i64),
        Some(Err(r)) => return -(status_of(&r) as i64),
        Some(Ok(c)) => c,
    };
    let cells = [c.records, c.chained, c.legacy, c.redacted, c.broken, c.seals, c.forks, c.heads, c.keys, c.admitted, c.checked, c.holds, c.segments, c.cut, c.cut_redacted, c.declared];
    let mut f = Vec::with_capacity(8 * COUNTS_CELLS);
    for v in cells {
        f.extend_from_slice(&v.to_le_bytes());
    }
    put(&f, out, cap)
}

/// A self-describing DOC's SPEC as `abi8 || wasm_sha256 32 || format text`: the length,
/// written if it fits; 0 for an image with no SPEC (a working image, a KV, a log) or a SPEC
/// whose text blob does not read; `-NOT_OPEN`, `-NULL_ARG`. The sha256 is the FILE'S CLAIM
/// about the reader it embeds (status.rs `UNCHECKED`'s sibling: a host recomputes and compares
/// with the format document, never trusts the file about itself).
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_spec(out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    let (Some(s), Some(text)) = (d.spec, d.spec_text()) else { return 0 };
    let mut f = Vec::with_capacity(40 + text.len());
    f.extend_from_slice(&s.abi.to_le_bytes());
    f.extend_from_slice(&s.wasm_sha256);
    f.extend_from_slice(&text);
    put(&f, out, cap)
}

/// What a DOC's seal count means from this reader (`status::UNCHECKED`): its length, written
/// if it fits; `-NULL_ARG`. A host prints it beside `bw_counts`' `seals` whenever `checked` is 0.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_unchecked_text(out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    put(UNCHECKED.as_bytes(), out, cap)
}

/// A DOC's time anchors (FORMAT §9.4 ANCH), newest first, `ANCHOR_ROW` bytes each
/// (`kind 8 || over 32 || time_ms 8 || proof_len 8`): the total length, written if it fits; 0 for
/// a KV, a log, or a DOC with no anchor; `-NOT_OPEN`, `-NULL_ARG`. PRESENT, NEVER VERIFIED: this
/// reader has no adapter for any kind (TSA key, OTS calendar, Roughtime), so a row says only
/// that the file carries a proof blob of that kind and length, as `anch::read_anchors` read it.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_anchors(out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    let mut f = Vec::with_capacity(ANCHOR_ROW * d.anchors.len());
    for a in &d.anchors {
        f.extend_from_slice(&a.kind.to_le_bytes());
        f.extend_from_slice(&a.over);
        f.extend_from_slice(&a.time_ms.to_le_bytes());
        f.extend_from_slice(&(a.proof.len() as i64).to_le_bytes());
    }
    put(&f, out, cap)
}
