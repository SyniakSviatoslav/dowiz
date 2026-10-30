// SPDX-License-Identifier: MIT OR Apache-2.0
//! The C surface: what a host with no wasm-bindgen calls (R-MAINGOAL-A §5.1, as far as today's
//! format reaches: open / kind / root / get / scan / log_len / log_at / verify / status_text).
//!
//! ONE IMAGE OPEN AT A TIME, held in this module (a wasm instance is one thread; a host that
//! wants two images instantiates twice). The host asks `bw_alloc` for room, copies the image in,
//! calls `bw_open`, and may `bw_free` the image at once: `Doc` owns copies. Every answer that is
//! bytes is written into a host-supplied buffer and its LENGTH returned; a buffer too small gets
//! the length needed with NOTHING written, so the host grows the buffer and asks again (a length
//! is never negative, so it is never mistaken for a status). Negative returns are `-status`
//! (status.rs).
//! A status is never an exception: `panic = "abort"` is set, so every refusal is a number.
//!
//! COMPILED FOR EVERY TARGET: `tests/conformance.rs` calls these very functions natively.

use core::cell::UnsafeCell;

use crate::status::*;
use crate::{open, Doc, Kind, Refusal};

/// Bumped when a signature below changes.
pub const ABI_VERSION: i32 = 1;
/// `bw_kind` for a log root; a KV root answers its version 1..=4.
pub const KIND_LOG: i32 = 16;

struct Slot(UnsafeCell<Option<Doc>>);
// One thread per wasm instance; natively the tests run this ABI serially (`--test-threads` is
// not set: the tests that touch the slot are in one `#[test]`, `the_abi_round_trips_natively`).
unsafe impl Sync for Slot {}
static SLOT: Slot = Slot(UnsafeCell::new(None));

fn slot() -> &'static mut Option<Doc> {
    // SAFETY: single-threaded by construction (see `Slot`); no reference escapes a call.
    unsafe { &mut *SLOT.0.get() }
}

/// The status a refusal is.
pub fn status_of(r: &Refusal) -> i32 {
    match r {
        Refusal::NoSuperblock => NO_SUPERBLOCK,
        Refusal::NotAKv => NOT_A_KV,
        Refusal::NotALog => NOT_A_LOG,
        Refusal::Truncated { .. } => TRUNCATED,
        Refusal::BadCrc { .. } => BAD_CRC,
        Refusal::UnknownVersion => UNKNOWN_VERSION,
        Refusal::LastCommit => LAST_COMMIT,
    }
}

#[no_mangle]
pub extern "C" fn bw_version() -> i32 {
    ABI_VERSION
}

/// Room for `len` bytes inside this module's memory. Null when `len` is zero.
#[no_mangle]
pub extern "C" fn bw_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return core::ptr::null_mut();
    }
    let mut v: Vec<u8> = Vec::with_capacity(len);
    let p = v.as_mut_ptr();
    core::mem::forget(v);
    p
}

/// Give back what `bw_alloc(len)` handed out, with the same `len`.
///
/// # Safety
/// `ptr` came from `bw_alloc(len)` and is freed once.
#[no_mangle]
pub unsafe extern "C" fn bw_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len != 0 {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Open the image at `ptr..ptr+len`. 0 = open; any other status = nothing is open.
///
/// # Safety
/// `ptr..ptr+len` is readable.
#[no_mangle]
pub unsafe extern "C" fn bw_open(ptr: *const u8, len: usize) -> i32 {
    *slot() = None;
    if ptr.is_null() {
        return NULL_ARG;
    }
    match open(core::slice::from_raw_parts(ptr, len)) {
        Ok(d) => {
            *slot() = Some(d);
            OK
        }
        Err(r) => status_of(&r),
    }
}

#[no_mangle]
pub extern "C" fn bw_close() {
    *slot() = None;
}

/// 0 when nothing is open; a KV root's version 1..=4; `KIND_LOG` for a log.
#[no_mangle]
pub extern "C" fn bw_kind() -> i32 {
    match slot().as_ref().map(|d| d.kind) {
        None => 0,
        Some(Kind::Kv { version }) => version as i32,
        Some(Kind::Log) => KIND_LOG,
    }
}

/// Entries of a KV or records of a log, or `-NOT_OPEN`.
#[no_mangle]
pub extern "C" fn bw_len() -> i64 {
    slot().as_ref().map_or(-(NOT_OPEN as i64), |d| d.len() as i64)
}

/// The family's fold into `*out`.
///
/// # Safety
/// `out` points at one writable `i64`.
#[no_mangle]
pub unsafe extern "C" fn bw_root(out: *mut i64) -> i32 {
    if out.is_null() {
        return NULL_ARG;
    }
    match slot().as_ref() {
        Some(d) => {
            *out = d.fold;
            OK
        }
        None => NOT_OPEN,
    }
}

/// Copy `src` into `out..out+cap` if it fits: its length either way (nothing written if not).
unsafe fn put(src: &[u8], out: *mut u8, cap: usize) -> i64 {
    if src.len() <= cap {
        core::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len());
    }
    src.len() as i64
}

/// The value under the key at `kptr..kptr+klen`, into `out..out+cap`: its length (written only
/// if it fits), or `-NOT_FOUND`, `-NOT_OPEN`, `-NOT_A_KV` (a log is open), `-NULL_ARG`.
///
/// # Safety
/// The key is readable and `out..out+cap` writable.
#[no_mangle]
pub unsafe extern "C" fn bw_get(kptr: *const u8, klen: usize, out: *mut u8, cap: usize) -> i64 {
    if kptr.is_null() || out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    if d.kind == Kind::Log {
        return -(NOT_A_KV as i64);
    }
    match d.get(core::slice::from_raw_parts(kptr, klen)) {
        Some(v) => put(v, out, cap),
        None => -(NOT_FOUND as i64),
    }
}

/// Entry `cursor` (0-based, key order) as the frame `klen8 || key || vlen8 || value` (the
/// bytes the fold hashes): its length, written if it fits; 0 past the last entry;
/// `-NOT_OPEN`, `-NOT_A_KV`, `-NULL_ARG`.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_scan_next(cursor: i64, out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    if d.kind == Kind::Log {
        return -(NOT_A_KV as i64);
    }
    let Some((k, v)) = usize::try_from(cursor).ok().and_then(|i| d.entries.get(i)) else { return 0 };
    let mut f = Vec::with_capacity(16 + k.len() + v.len());
    f.extend_from_slice(&(k.len() as u64).to_le_bytes());
    f.extend_from_slice(k);
    f.extend_from_slice(&(v.len() as u64).to_le_bytes());
    f.extend_from_slice(v);
    put(&f, out, cap)
}

/// Records of the open log, or `-NOT_A_LOG` (a KV is open), `-NOT_OPEN`.
#[no_mangle]
pub extern "C" fn bw_log_len() -> i64 {
    match slot().as_ref() {
        None => -(NOT_OPEN as i64),
        Some(d) if d.kind != Kind::Log => -(NOT_A_LOG as i64),
        Some(d) => d.records.len() as i64,
    }
}

/// Record `i` (0 = OLDEST) as `seq8 || plen8 || id32 || prev32 || actor_pubkey32 || payload`:
/// its length, written if it fits; `-OUT_OF_RANGE`, `-NOT_A_LOG`, `-NOT_OPEN`, `-NULL_ARG`.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_log_at(i: i64, out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    let Some(d) = slot().as_ref() else { return -(NOT_OPEN as i64) };
    if d.kind != Kind::Log {
        return -(NOT_A_LOG as i64);
    }
    let Some(r) = usize::try_from(i).ok().and_then(|i| d.records.get(i)) else { return -(OUT_OF_RANGE as i64) };
    let mut f = Vec::with_capacity(112 + r.payload.len());
    f.extend_from_slice(&r.actor_seq.to_le_bytes());
    f.extend_from_slice(&(r.payload.len() as u64).to_le_bytes());
    f.extend_from_slice(&r.id);
    f.extend_from_slice(&r.prev);
    f.extend_from_slice(&r.actor_pubkey);
    f.extend_from_slice(&r.payload);
    put(&f, out, cap)
}

/// `depth` 0 (Anchor) and 1 (Reachable): what `bw_open` already proved, so OK while an image
/// is open. 2 (WholeArena): every object of the arena up to `arena_used` hashes to its crc, or
/// `BAD_CRC`. `NOT_OPEN` when nothing is open.
#[no_mangle]
pub extern "C" fn bw_verify(depth: i32) -> i32 {
    let Some(d) = slot().as_ref() else { return NOT_OPEN };
    if depth < 2 {
        return OK;
    }
    d.verify_arena().map_or_else(|r| status_of(&r), |()| OK)
}

/// The line of docs/STATUS-CODES.md for `code` into `out..out+cap`: its length (written if it
/// fits); 0 for a code this reader never gives; `-NULL_ARG`.
///
/// # Safety
/// `out..out+cap` is writable.
#[no_mangle]
pub unsafe extern "C" fn bw_status_text(code: i32, out: *mut u8, cap: usize) -> i64 {
    if out.is_null() {
        return -(NULL_ARG as i64);
    }
    crate::status::text(code).map_or(0, |t| put(t.as_bytes(), out, cap))
}
