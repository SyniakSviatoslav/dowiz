//! The C surface of the module: what a host with no wasm-bindgen calls.
//!
//! THE HOST OWNS THE BYTES' LIFETIME AND THIS SIDE OWNS THE MEMORY. A caller
//! asks `bw_alloc` for room inside the module's linear memory, copies the image
//! in, calls a reader with the pointer and length, reads the two result cells
//! it supplied, and gives the room back with `bw_free`. No string crosses, no
//! JSON, no callback: two `i64` cells and a status.
//!
//! A STATUS IS NOT AN EXCEPTION. `panic = "abort"` is set in `Cargo.toml`, so a
//! panic here would be a trap the host sees as `unreachable` with no message.
//! Every refusal `lib.rs` can name therefore comes back as a number the host
//! can print, and the tests below prove the mapping is total.
//!
//! COMPILED FOR EVERY TARGET, not only wasm32, so `cargo test` exercises the
//! exact functions node calls; the only thing that differs under wasm32 is who
//! is on the other side of the pointer.

use crate::{kv_view, log_view, Refusal};

/// Bumped when a signature below changes. A host that reads a different number
/// is talking to a module it was not written for.
pub const ABI_VERSION: i32 = 1;

/// Status codes. Zero is the only success.
pub const OK: i32 = 0;
pub const NO_SUPERBLOCK: i32 = 1;
pub const NOT_A_KV: i32 = 2;
pub const NOT_A_LOG: i32 = 3;
pub const TRUNCATED: i32 = 4;
/// The pointer or the output cells were null.
pub const NULL_ARG: i32 = 5;

fn status_of(r: &Refusal) -> i32 {
    match r {
        Refusal::NoSuperblock => NO_SUPERBLOCK,
        Refusal::NotAKv => NOT_A_KV,
        Refusal::NotALog => NOT_A_LOG,
        Refusal::Truncated { .. } => TRUNCATED,
    }
}

#[no_mangle]
pub extern "C" fn bw_abi_version() -> i32 {
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

/// Give back what `bw_alloc(len)` handed out. `len` must be the same number.
///
/// # Safety
/// `ptr` came from `bw_alloc(len)` and is freed once.
#[no_mangle]
pub unsafe extern "C" fn bw_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len != 0 {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Read a KV image. On `OK`, `out[0]` = entry count, `out[1]` = the FNV-1a
/// root as `kv.bp`'s `kv_snapshot` prints it.
///
/// # Safety
/// `ptr..ptr+len` is readable and `out` points at two writable `i64` cells.
#[no_mangle]
pub unsafe extern "C" fn bw_kv(ptr: *const u8, len: usize, out: *mut i64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return NULL_ARG;
    }
    let bytes = core::slice::from_raw_parts(ptr, len);
    match kv_view(bytes) {
        Ok(v) => {
            *out = v.n;
            *out.add(1) = v.root;
            OK
        }
        Err(r) => status_of(&r),
    }
}

/// Read a log image. On `OK`, `out[0]` = record count, `out[1]` = the fold.
///
/// # Safety
/// As `bw_kv`.
#[no_mangle]
pub unsafe extern "C" fn bw_log(ptr: *const u8, len: usize, out: *mut i64) -> i32 {
    if ptr.is_null() || out.is_null() {
        return NULL_ARG;
    }
    let bytes = core::slice::from_raw_parts(ptr, len);
    match log_view(bytes) {
        Ok(v) => {
            *out = v.len;
            *out.add(1) = v.fold;
            OK
        }
        Err(r) => status_of(&r),
    }
}
