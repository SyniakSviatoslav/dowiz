// SPDX-License-Identifier: MIT OR Apache-2.0
//! A C ABI over the bebop KV database (W-ES-P6, R-EMBED-STORE §5 row P6).
//!
//! Every function returns an `i32` status: [`OK`] (0) or a refusal code. On a refusal the
//! human-readable reason is kept per thread and read with [`bebop_last_error`]. Results come
//! back through out-pointers. Nothing here panics across the boundary: a panic is caught and
//! reported as [`PANIC`].
//!
//! # Handles and their order
//!
//! * `BebopDb` — one database ([`bebop_db_open`], [`bebop_db_open_bytes`],
//!   [`bebop_db_create_mem`]); closed with [`bebop_db_close`].
//! * `BebopRead` — a snapshot of one generation ([`bebop_read_begin`]); ended with
//!   [`bebop_read_end`].
//! * `BebopWrite` — the single write transaction ([`bebop_write_begin`]); consumed by
//!   [`bebop_write_commit`], [`bebop_write_stage`] or [`bebop_write_abort`].
//! * `BebopScan` — a key-ordered cursor over a `BebopRead` ([`bebop_read_scan`],
//!   [`bebop_read_prefix`]); freed with [`bebop_scan_free`].
//!
//! A child borrows its parent. Closing a parent while a child is still open is refused with
//! [`BUSY`] instead of leaving the child dangling: close scans before their read, and reads and
//! writes before their database.
//!
//! # Bytes
//!
//! Keys and values cross as `(pointer, length)`. A null pointer is accepted only with length 0.
//! A value returned by a `get` is a fresh copy the caller frees with [`bebop_free`]. A key or
//! value returned by [`bebop_scan_next`] is borrowed from the cursor and valid until the next
//! call on that cursor or its free.
//!
//! # Status codes
//!
//! The codes shared with `bebop-wasm`'s reader ABI keep their numbers there (1, 2, 5, 10, 12), so
//! the same image refuses with the same number through either surface. The rest start at 40.

use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::ops::Bound;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

use bebop_io::{Bytes, Db, Detached, Durability, Options, ReadTxn, Refusal, Scan, Verify, WriteTxn};

#[cfg(all(unix, not(target_arch = "wasm32")))]
use bebop_io::FileDevice;

/// Bumped when a signature below changes.
pub const ABI_VERSION: i32 = 1;

/// Success.
pub const OK: i32 = 0;
/// No superblock could be picked, or the live one was refused structurally (`Refusal::Pick`).
pub const NO_SUPERBLOCK: i32 = 1;
/// No KV root (`Refusal::NotKv`).
pub const NOT_A_KV: i32 = 2;
/// A required pointer was null (or a null slice had a non-zero length).
pub const NULL_ARG: i32 = 5;
/// An object failed its crc (`Refusal::BadCrc`).
pub const BAD_CRC: i32 = 10;
/// The KV root names a version this library does not know (`Refusal::UnknownVersion`).
pub const UNKNOWN_VERSION: i32 = 12;
/// The newest generation failed and so did the fallback (`Refusal::Recover`).
pub const RECOVER: i32 = 40;
/// The last commit failed its proof and falling back was not allowed (`Refusal::LastCommit`).
pub const LAST_COMMIT: i32 = 41;
/// An object met walking the whole arena is cut short or fails its crc (`Refusal::ObjectBad`).
pub const OBJECT_BAD: i32 = 42;
/// A cell the write path was about to trust disagrees with the image (`Refusal::Corrupt`).
pub const CORRUPT: i32 = 43;
pub const KEY_TOO_LONG: i32 = 44;
pub const VALUE_TOO_LONG: i32 = 45;
pub const ARENA_FULL: i32 = 46;
/// A format limit (`Refusal::Limit`).
pub const LIMIT: i32 = 47;
pub const TOO_SMALL: i32 = 48;
/// A write transaction is already open on this database.
pub const WRITER_BUSY: i32 = 49;
/// Another process holds the store's lock.
pub const LOCKED: i32 = 50;
/// An earlier write failed; nothing more is written until the database is reopened.
pub const POISONED: i32 = 51;
/// Create on a device that already holds a KV image.
pub const EXISTS: i32 = 52;
/// The device said no (`Refusal::Io`).
pub const IO: i32 = 53;
/// The commit asked for more durability than the device gives.
pub const DURABILITY: i32 = 54;
/// The staged image cannot be flushed over the durable one (`Refusal::Plan`).
pub const PLAN: i32 = 55;
/// An argument out of range: an unknown durability or verify depth, a path that is not UTF-8.
pub const BAD_ARG: i32 = 56;
/// A handle still has open children (see the module docs).
pub const BUSY: i32 = 57;
/// This build cannot do that (a file database on a non-unix target).
pub const UNSUPPORTED: i32 = 58;
/// A panic was caught at the boundary. The handle involved should be treated as unusable.
pub const PANIC: i32 = 59;
/// A refusal newer than this ABI (`Refusal` is `#[non_exhaustive]`); the message names it.
pub const OTHER: i32 = 60;

/// `bebop_db_open` flag: create the file (refused with [`EXISTS`] if it already holds a KV image).
pub const OPEN_CREATE: u32 = 1;

/// The status code of a refusal. Total: an unknown future variant is [`OTHER`].
pub fn code_of(r: &Refusal) -> i32 {
    match r {
        Refusal::Pick(_) => NO_SUPERBLOCK,
        Refusal::Recover { .. } => RECOVER,
        Refusal::LastCommit(_) => LAST_COMMIT,
        Refusal::BadCrc(_) => BAD_CRC,
        Refusal::ObjectBad { .. } => OBJECT_BAD,
        Refusal::NotKv => NOT_A_KV,
        Refusal::UnknownVersion(_) => UNKNOWN_VERSION,
        Refusal::Corrupt(_) => CORRUPT,
        Refusal::KeyTooLong { .. } => KEY_TOO_LONG,
        Refusal::ValueTooLong { .. } => VALUE_TOO_LONG,
        Refusal::ArenaFull { .. } => ARENA_FULL,
        Refusal::Limit { .. } => LIMIT,
        Refusal::TooSmall { .. } => TOO_SMALL,
        Refusal::WriterBusy => WRITER_BUSY,
        Refusal::Locked { .. } => LOCKED,
        Refusal::Poisoned => POISONED,
        Refusal::Exists => EXISTS,
        Refusal::Io { .. } => IO,
        Refusal::Durability { .. } => DURABILITY,
        Refusal::Plan(_) => PLAN,
        _ => OTHER,
    }
}

/// The name of a status code, as a static NUL-terminated string (`"UNKNOWN"` for none).
#[no_mangle]
pub extern "C" fn bebop_status_name(code: i32) -> *const c_char {
    let s: &'static [u8] = match code {
        OK => b"OK\0",
        NO_SUPERBLOCK => b"NO_SUPERBLOCK\0",
        NOT_A_KV => b"NOT_A_KV\0",
        NULL_ARG => b"NULL_ARG\0",
        BAD_CRC => b"BAD_CRC\0",
        UNKNOWN_VERSION => b"UNKNOWN_VERSION\0",
        RECOVER => b"RECOVER\0",
        LAST_COMMIT => b"LAST_COMMIT\0",
        OBJECT_BAD => b"OBJECT_BAD\0",
        CORRUPT => b"CORRUPT\0",
        KEY_TOO_LONG => b"KEY_TOO_LONG\0",
        VALUE_TOO_LONG => b"VALUE_TOO_LONG\0",
        ARENA_FULL => b"ARENA_FULL\0",
        LIMIT => b"LIMIT\0",
        TOO_SMALL => b"TOO_SMALL\0",
        WRITER_BUSY => b"WRITER_BUSY\0",
        LOCKED => b"LOCKED\0",
        POISONED => b"POISONED\0",
        EXISTS => b"EXISTS\0",
        IO => b"IO\0",
        DURABILITY => b"DURABILITY\0",
        PLAN => b"PLAN\0",
        BAD_ARG => b"BAD_ARG\0",
        BUSY => b"BUSY\0",
        UNSUPPORTED => b"UNSUPPORTED\0",
        PANIC => b"PANIC\0",
        OTHER => b"OTHER\0",
        _ => b"UNKNOWN\0",
    };
    s.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn bebop_abi_version() -> i32 {
    ABI_VERSION
}

// ── errors ──────────────────────────────────────────────────────────────────────────────────

thread_local! {
    static LAST: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Why a call failed: a refusal of the library, or one of this layer.
enum Fail {
    Refused(Refusal),
    Here(i32, &'static str),
}

impl From<Refusal> for Fail {
    fn from(r: Refusal) -> Fail {
        Fail::Refused(r)
    }
}

type Res = Result<(), Fail>;

/// Run `f`, catching a panic; keep the message of a failure for `bebop_last_error`.
fn run(f: impl FnOnce() -> Res) -> i32 {
    let (code, msg) = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => return OK,
        Ok(Err(Fail::Refused(r))) => (code_of(&r), format!("{r}")),
        Ok(Err(Fail::Here(c, m))) => (c, m.to_string()),
        Err(p) => {
            let m = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".to_string());
            (PANIC, m)
        }
    };
    LAST.with(|l| *l.borrow_mut() = msg);
    code
}

/// Copy this thread's last failure message into `buf` (at most `cap` bytes, no NUL added) and
/// return its full length. `buf` may be null to ask only for the length.
///
/// # Safety
/// `buf..buf+cap` is writable when `buf` is not null.
#[no_mangle]
pub unsafe extern "C" fn bebop_last_error(buf: *mut u8, cap: usize) -> usize {
    LAST.with(|l| {
        let s = l.borrow();
        if !buf.is_null() {
            let n = s.len().min(cap);
            std::ptr::copy_nonoverlapping(s.as_ptr(), buf, n);
        }
        s.len()
    })
}

// ── small helpers ───────────────────────────────────────────────────────────────────────────

fn null() -> Fail {
    Fail::Here(NULL_ARG, "a required pointer was null")
}

unsafe fn slice<'a>(p: *const u8, n: usize) -> Result<&'a [u8], Fail> {
    if p.is_null() {
        if n == 0 {
            return Ok(&[]);
        }
        return Err(null());
    }
    Ok(std::slice::from_raw_parts(p, n))
}

unsafe fn put<T>(out: *mut T, v: T) -> Res {
    if out.is_null() {
        return Err(null());
    }
    out.write(v);
    Ok(())
}

unsafe fn handle<'a, T>(p: *const T) -> Result<&'a T, Fail> {
    p.as_ref().ok_or_else(null)
}

fn durability(d: i32) -> Result<Durability, Fail> {
    match d {
        0 => Ok(Durability::None),
        1 => Ok(Durability::Data),
        2 => Ok(Durability::Full),
        _ => Err(Fail::Here(BAD_ARG, "durability must be 0 (None), 1 (Data) or 2 (Full)")),
    }
}

fn verify(v: i32) -> Result<Verify, Fail> {
    match v {
        0 => Ok(Verify::Anchor),
        1 => Ok(Verify::Reachable),
        2 => Ok(Verify::WholeArena),
        _ => Err(Fail::Here(BAD_ARG, "verify must be 0 (Anchor), 1 (Reachable) or 2 (WholeArena)")),
    }
}

/// Hand `v` to the caller as `(ptr, len)`; freed with `bebop_free(ptr, len)`. Empty = null.
unsafe fn give(v: Vec<u8>, ptr: *mut *mut u8, len: *mut usize) -> Res {
    if ptr.is_null() || len.is_null() {
        return Err(null());
    }
    let n = v.len();
    let p = if n == 0 { std::ptr::null_mut() } else { Box::into_raw(v.into_boxed_slice()) as *mut u8 };
    ptr.write(p);
    len.write(n);
    Ok(())
}

/// Free a buffer this library handed out (`bebop_read_get`, `bebop_write_get`,
/// `bebop_db_snapshot`). `len` must be the length it came with. Null is a no-op.
///
/// # Safety
/// `ptr` came from this library with this `len` and is freed once.
#[no_mangle]
pub unsafe extern "C" fn bebop_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len != 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
    }
}

// ── the database ────────────────────────────────────────────────────────────────────────────

enum AnyDb {
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    File(Db<FileDevice>),
    Mem(Db<Detached>),
}

/// One database. Opaque to C.
pub struct BebopDb {
    db: AnyDb,
    /// Open reads and writes borrowing this handle.
    children: AtomicUsize,
}

macro_rules! with_db {
    ($h:expr, $d:ident => $e:expr) => {
        match &$h.db {
            #[cfg(all(unix, not(target_arch = "wasm32")))]
            AnyDb::File($d) => $e,
            AnyDb::Mem($d) => $e,
        }
    };
}

fn boxed(db: AnyDb) -> *mut BebopDb {
    Box::into_raw(Box::new(BebopDb { db, children: AtomicUsize::new(0) }))
}

/// Open (or with [`OPEN_CREATE`], create) the store file at `path` (NUL-terminated UTF-8).
/// `verify` is the depth proved on open: 0 Anchor, 1 Reachable, 2 WholeArena.
///
/// # Safety
/// `path` is a NUL-terminated string; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_open(path: *const c_char, flags: u32, verify_depth: i32, out: *mut *mut BebopDb) -> i32 {
    run(|| {
        if path.is_null() || out.is_null() {
            return Err(null());
        }
        let p = CStr::from_ptr(path).to_str().map_err(|_| Fail::Here(BAD_ARG, "path is not UTF-8"))?;
        let opts = Options::default().verify(verify(verify_depth)?);
        #[cfg(all(unix, not(target_arch = "wasm32")))]
        {
            let db = if flags & OPEN_CREATE != 0 { Db::create_path(p, opts)? } else { Db::open_path(p, opts)? };
            put(out, boxed(AnyDb::File(db)))
        }
        #[cfg(not(all(unix, not(target_arch = "wasm32"))))]
        {
            let _ = (p, opts, flags);
            Err(Fail::Here(UNSUPPORTED, "file databases need a unix target"))
        }
    })
}

/// Adopt an image held in memory (copied). A torn last commit is REFUSED, not fallen back past
/// (`Options::detached`): bytes handed over whole are expected whole. Commits on this database
/// change only memory; read the bytes back with [`bebop_db_snapshot`].
///
/// # Safety
/// `ptr..ptr+len` is readable; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_open_bytes(ptr: *const u8, len: usize, verify_depth: i32, out: *mut *mut BebopDb) -> i32 {
    run(|| {
        let bytes = slice(ptr, len)?;
        if out.is_null() {
            return Err(null());
        }
        let db = Db::from_bytes(bytes, Options::detached().verify(verify(verify_depth)?))?;
        put(out, boxed(AnyDb::Mem(db)))
    })
}

/// A fresh, empty in-memory database.
///
/// # Safety
/// `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_create_mem(out: *mut *mut BebopDb) -> i32 {
    run(|| {
        if out.is_null() {
            return Err(null());
        }
        let db = Db::create_detached(Options::detached())?;
        put(out, boxed(AnyDb::Mem(db)))
    })
}

/// Close a database. [`BUSY`] (and nothing freed) while a read or write is open on it.
/// Null is a no-op.
///
/// # Safety
/// `db` came from an open call and is closed once.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_close(db: *mut BebopDb) -> i32 {
    run(|| {
        if db.is_null() {
            return Ok(());
        }
        if (*db).children.load(Ordering::SeqCst) != 0 {
            return Err(Fail::Here(BUSY, "the database still has an open read or write"));
        }
        drop(Box::from_raw(db));
        Ok(())
    })
}

/// Flush every staged commit with durability `d` (0 None, 1 Data, 2 Full); `out_gen` = the
/// generation now durable.
///
/// # Safety
/// `db` is open; `out_gen` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_flush(db: *const BebopDb, d: i32, out_gen: *mut i64) -> i32 {
    run(|| {
        let h = handle(db)?;
        let d = durability(d)?;
        let g = with_db!(h, x => x.flush(d)?);
        put(out_gen, g.get())
    })
}

/// The generation the device holds durably.
///
/// # Safety
/// `db` is open; `out_gen` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_durable(db: *const BebopDb, out_gen: *mut i64) -> i32 {
    run(|| {
        let h = handle(db)?;
        let g = with_db!(h, x => x.durable());
        put(out_gen, g.get())
    })
}

/// Prove the published generation to `depth` (0 Anchor, 1 Reachable, 2 WholeArena);
/// `out_gen` = the generation proved.
///
/// # Safety
/// `db` is open; `out_gen` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_verify(db: *const BebopDb, depth: i32, out_gen: *mut i64) -> i32 {
    run(|| {
        let h = handle(db)?;
        let depth = verify(depth)?;
        let r = with_db!(h, x => x.verify(depth)?);
        put(out_gen, r.generation.get())
    })
}

/// Rewrite the live entries into a fresh image and publish it (a file: `<path>.compact`, then
/// an atomic rename). `out_gen` = the new generation.
///
/// # Safety
/// `db` is open; `out_gen` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_compact(db: *const BebopDb, out_gen: *mut i64) -> i32 {
    run(|| {
        let h = handle(db)?;
        let g = with_db!(h, x => x.compact_fresh()?);
        put(out_gen, g.get())
    })
}

/// The published generation's trimmed image bytes (a copy; free with [`bebop_free`]).
///
/// # Safety
/// `db` is open; `out_ptr`, `out_len` are writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_snapshot(db: *const BebopDb, out_ptr: *mut *mut u8, out_len: *mut usize) -> i32 {
    run(|| {
        let h = handle(db)?;
        let v = with_db!(h, x => x.snapshot_bytes());
        give(v, out_ptr, out_len)
    })
}

/// 1 if a write failed since open (nothing more is written), else 0. -1 for a null handle.
///
/// # Safety
/// `db` is open or null.
#[no_mangle]
pub unsafe extern "C" fn bebop_db_poisoned(db: *const BebopDb) -> i32 {
    match db.as_ref() {
        None => -1,
        Some(h) => with_db!(h, x => x.poisoned() as i32),
    }
}

// ── read transactions ───────────────────────────────────────────────────────────────────────

/// A snapshot of one generation. Opaque to C.
pub struct BebopRead {
    /// `'static` is a lie the counters make safe: `bebop_db_close` refuses while this exists.
    txn: ReadTxn<'static>,
    db: *const BebopDb,
    scans: AtomicUsize,
}

/// Begin a read of the published generation. Never refused by a poisoned writer.
///
/// # Safety
/// `db` is open; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_begin(db: *const BebopDb, out: *mut *mut BebopRead) -> i32 {
    run(|| {
        let h = handle(db)?;
        if out.is_null() {
            return Err(null());
        }
        let txn: ReadTxn<'_> = with_db!(h, x => x.begin_read());
        // SAFETY: the read borrows `*db`, which `bebop_db_close` will not free while
        // `children` counts this read.
        let txn: ReadTxn<'static> = std::mem::transmute::<ReadTxn<'_>, ReadTxn<'static>>(txn);
        h.children.fetch_add(1, Ordering::SeqCst);
        put(out, Box::into_raw(Box::new(BebopRead { txn, db, scans: AtomicUsize::new(0) })))
    })
}

/// End a read. [`BUSY`] (and nothing freed) while a scan of it is open. Null is a no-op.
///
/// # Safety
/// `r` came from `bebop_read_begin` and is ended once.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_end(r: *mut BebopRead) -> i32 {
    run(|| {
        if r.is_null() {
            return Ok(());
        }
        if (*r).scans.load(Ordering::SeqCst) != 0 {
            return Err(Fail::Here(BUSY, "the read still has an open scan"));
        }
        let b = Box::from_raw(r);
        let db = b.db;
        drop(b);
        (*db).children.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    })
}

unsafe fn give_found(v: Option<Bytes<'_>>, found: *mut i32, ptr: *mut *mut u8, len: *mut usize) -> Res {
    put(found, v.is_some() as i32)?;
    give(v.map(|b| b.into_owned()).unwrap_or_default(), ptr, len)
}

/// Look `key` up. `out_found` = 1 and the value copied to `(out_ptr, out_len)` (free with
/// [`bebop_free`]), or `out_found` = 0 and an empty buffer.
///
/// # Safety
/// `r` is open; the key slice is readable; the outs are writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_get(
    r: *const BebopRead,
    key: *const u8,
    key_len: usize,
    out_found: *mut i32,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        let h = handle(r)?;
        let k = slice(key, key_len)?;
        give_found(h.txn.get(k)?, out_found, out_ptr, out_len)
    })
}

/// The number of entries in the snapshot.
///
/// # Safety
/// `r` is open; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_len(r: *const BebopRead, out: *mut u64) -> i32 {
    run(|| {
        let h = handle(r)?;
        put(out, h.txn.len() as u64)
    })
}

/// The snapshot's generation.
///
/// # Safety
/// `r` is open; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_generation(r: *const BebopRead, out: *mut i64) -> i32 {
    run(|| {
        let h = handle(r)?;
        put(out, h.txn.generation().get())
    })
}

/// The FNV-1a 64 fold over the entries (`Kv::snapshot_root_u64`), as the signed `i64` every
/// other reader of the format prints.
///
/// # Safety
/// `r` is open; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_fold_root(r: *const BebopRead, out: *mut i64) -> i32 {
    run(|| {
        let h = handle(r)?;
        put(out, h.txn.fold_root()? as i64)
    })
}

// ── scans ───────────────────────────────────────────────────────────────────────────────────

/// A key-ordered cursor. Opaque to C.
pub struct BebopScan {
    /// Declared first, so it drops before `scan`; both borrow `*read`, never each other.
    cur: Option<(Bytes<'static>, Bytes<'static>)>,
    scan: Scan<'static>,
    read: *const BebopRead,
}

unsafe fn open_scan(h: &BebopRead, r: *const BebopRead, scan: Scan<'_>, out: *mut *mut BebopScan) -> Res {
    // SAFETY: the scan borrows `h.txn`; `bebop_read_end` refuses while `scans` counts it.
    let scan: Scan<'static> = std::mem::transmute::<Scan<'_>, Scan<'static>>(scan);
    h.scans.fetch_add(1, Ordering::SeqCst);
    put(out, Box::into_raw(Box::new(BebopScan { cur: None, scan, read: r })))
}

/// Scan keys in `[lo, hi)` in key order. A null `lo` (or `hi`) is unbounded on that side.
///
/// # Safety
/// `r` is open; the bound slices are readable when not null; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_scan(
    r: *const BebopRead,
    lo: *const u8,
    lo_len: usize,
    hi: *const u8,
    hi_len: usize,
    out: *mut *mut BebopScan,
) -> i32 {
    run(|| {
        let h = handle(r)?;
        if out.is_null() {
            return Err(null());
        }
        let lo = if lo.is_null() { Bound::Unbounded } else { Bound::Included(slice(lo, lo_len)?) };
        let hi = if hi.is_null() { Bound::Unbounded } else { Bound::Excluded(slice(hi, hi_len)?) };
        let s = h.txn.scan(lo, hi)?;
        open_scan(h, r, s, out)
    })
}

/// Scan every key that starts with `prefix`, in key order.
///
/// # Safety
/// `r` is open; the prefix slice is readable; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_read_prefix(r: *const BebopRead, prefix: *const u8, prefix_len: usize, out: *mut *mut BebopScan) -> i32 {
    run(|| {
        let h = handle(r)?;
        if out.is_null() {
            return Err(null());
        }
        let p = slice(prefix, prefix_len)?;
        let s = h.txn.prefix(p)?;
        open_scan(h, r, s, out)
    })
}

/// Step the cursor. `out_has` = 1 with the entry in the four outs, or 0 at the end. The
/// pointers stay valid until the next call on this cursor or its free; copy what you keep.
///
/// # Safety
/// `s` is open; the outs are writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_scan_next(
    s: *mut BebopScan,
    out_has: *mut i32,
    key: *mut *const u8,
    key_len: *mut usize,
    val: *mut *const u8,
    val_len: *mut usize,
) -> i32 {
    run(|| {
        let h = s.as_mut().ok_or_else(null)?;
        if out_has.is_null() || key.is_null() || key_len.is_null() || val.is_null() || val_len.is_null() {
            return Err(null());
        }
        h.cur = None;
        match h.scan.next() {
            None => {
                out_has.write(0);
                key.write(std::ptr::null());
                key_len.write(0);
                val.write(std::ptr::null());
                val_len.write(0);
            }
            Some(item) => {
                let (k, v) = item?;
                out_has.write(1);
                key.write(k.as_ptr());
                key_len.write(k.len());
                val.write(v.as_ptr());
                val_len.write(v.len());
                h.cur = Some((k, v));
            }
        }
        Ok(())
    })
}

/// Free a cursor. Null is a no-op.
///
/// # Safety
/// `s` came from a scan call and is freed once.
#[no_mangle]
pub unsafe extern "C" fn bebop_scan_free(s: *mut BebopScan) {
    if s.is_null() {
        return;
    }
    let b = Box::from_raw(s);
    let read = b.read;
    drop(b);
    (*read).scans.fetch_sub(1, Ordering::SeqCst);
}

// ── write transactions ──────────────────────────────────────────────────────────────────────

enum AnyWrite {
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    File(WriteTxn<'static, FileDevice>),
    Mem(WriteTxn<'static, Detached>),
}

/// The write transaction. Opaque to C.
pub struct BebopWrite {
    txn: AnyWrite,
    db: *const BebopDb,
}

macro_rules! with_w {
    ($w:expr, $t:ident => $e:expr) => {
        match $w {
            #[cfg(all(unix, not(target_arch = "wasm32")))]
            AnyWrite::File($t) => $e,
            AnyWrite::Mem($t) => $e,
        }
    };
}

/// Begin the one write transaction: [`WRITER_BUSY`] if one is open, [`POISONED`] after a
/// failed write.
///
/// # Safety
/// `db` is open; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_begin(db: *const BebopDb, out: *mut *mut BebopWrite) -> i32 {
    run(|| {
        let h = handle(db)?;
        if out.is_null() {
            return Err(null());
        }
        // SAFETY (both arms): the txn borrows `*db`; `bebop_db_close` refuses while counted.
        let txn = match &h.db {
            #[cfg(all(unix, not(target_arch = "wasm32")))]
            AnyDb::File(x) => AnyWrite::File(std::mem::transmute::<WriteTxn<'_, FileDevice>, WriteTxn<'static, FileDevice>>(x.begin_write()?)),
            AnyDb::Mem(x) => AnyWrite::Mem(std::mem::transmute::<WriteTxn<'_, Detached>, WriteTxn<'static, Detached>>(x.begin_write()?)),
        };
        h.children.fetch_add(1, Ordering::SeqCst);
        put(out, Box::into_raw(Box::new(BebopWrite { txn, db })))
    })
}

/// Stage `key = value`. Limits are checked HERE ([`KEY_TOO_LONG`], [`VALUE_TOO_LONG`]), not at
/// commit.
///
/// # Safety
/// `w` is open; the slices are readable.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_put(w: *mut BebopWrite, key: *const u8, key_len: usize, val: *const u8, val_len: usize) -> i32 {
    run(|| {
        let h = w.as_mut().ok_or_else(null)?;
        let k = slice(key, key_len)?;
        let v = slice(val, val_len)?;
        with_w!(&mut h.txn, t => t.put(k, v)?);
        Ok(())
    })
}

/// Stage the removal of `key`; `out_was` (may be null) = 1 if the key was visible to this
/// transaction.
///
/// # Safety
/// `w` is open; the slice is readable; `out_was` is writable or null.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_remove(w: *mut BebopWrite, key: *const u8, key_len: usize, out_was: *mut i32) -> i32 {
    run(|| {
        let h = w.as_mut().ok_or_else(null)?;
        let k = slice(key, key_len)?;
        let was = with_w!(&mut h.txn, t => t.remove(k)?);
        if !out_was.is_null() {
            out_was.write(was as i32);
        }
        Ok(())
    })
}

/// Look `key` up as this transaction sees it (its own writes first). Outs as [`bebop_read_get`].
///
/// # Safety
/// `w` is open; the key slice is readable; the outs are writable.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_get(
    w: *const BebopWrite,
    key: *const u8,
    key_len: usize,
    out_found: *mut i32,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    run(|| {
        let h = handle(w)?;
        let k = slice(key, key_len)?;
        let v = with_w!(&h.txn, t => t.get(k)?.map(|b| b.into_owned()));
        put(out_found, v.is_some() as i32)?;
        give(v.unwrap_or_default(), out_ptr, out_len)
    })
}

fn take_write(w: *mut BebopWrite) -> Result<AnyWrite, Fail> {
    if w.is_null() {
        return Err(null());
    }
    // SAFETY: `w` came from `bebop_write_begin` and is consumed once (the caller's contract).
    let b = unsafe { Box::from_raw(w) };
    let BebopWrite { txn, db } = *b;
    // The txn is still alive here; it is consumed by the caller of `take_write` before `db`
    // could be closed, so the count can drop now.
    unsafe { (*db).children.fetch_sub(1, Ordering::SeqCst) };
    Ok(txn)
}

/// Commit with durability `d` (0 None, 1 Data, 2 Full); `out_gen` (may be null) = the new
/// generation (unchanged for an empty transaction). The handle is consumed whatever the status.
///
/// # Safety
/// `w` came from `bebop_write_begin` and is not used again; `out_gen` is writable or null.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_commit(w: *mut BebopWrite, d: i32, out_gen: *mut i64) -> i32 {
    run(|| {
        let d = match durability(d) {
            Ok(d) => d,
            Err(e) => {
                drop(take_write(w)?);
                return Err(e);
            }
        };
        let g = with_w!(take_write(w)?, t => t.commit(d)?);
        if !out_gen.is_null() {
            out_gen.write(g.get());
        }
        Ok(())
    })
}

/// Stage the commit in memory and publish it to readers WITHOUT flushing it (group commit:
/// several stages, then one [`bebop_db_flush`]). Consumes the handle.
///
/// # Safety
/// As [`bebop_write_commit`].
#[no_mangle]
pub unsafe extern "C" fn bebop_write_stage(w: *mut BebopWrite, out_gen: *mut i64) -> i32 {
    run(|| {
        let g = with_w!(take_write(w)?, t => t.stage()?);
        if !out_gen.is_null() {
            out_gen.write(g.get());
        }
        Ok(())
    })
}

/// Drop the writes; nothing touched the image. Null is a no-op.
///
/// # Safety
/// `w` came from `bebop_write_begin` and is not used again.
#[no_mangle]
pub unsafe extern "C" fn bebop_write_abort(w: *mut BebopWrite) {
    if let Ok(t) = take_write(w) {
        with_w!(t, t => t.abort());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    unsafe fn get(r: *const BebopRead, k: &[u8]) -> Option<Vec<u8>> {
        let (mut f, mut p, mut n) = (0, null_mut(), 0);
        assert_eq!(bebop_read_get(r, k.as_ptr(), k.len(), &mut f, &mut p, &mut n), OK);
        // An empty value comes back as (null, 0): never build a slice from it.
        let v = (f == 1).then(|| if n == 0 { vec![] } else { std::slice::from_raw_parts(p, n).to_vec() });
        bebop_free(p, n);
        v
    }

    #[test]
    fn mem_roundtrip_scan_and_busy() {
        unsafe {
            let mut db = null_mut();
            assert_eq!(bebop_db_create_mem(&mut db), OK);
            let mut w = null_mut();
            assert_eq!(bebop_write_begin(db, &mut w), OK);
            let mut w2 = null_mut();
            assert_eq!(bebop_write_begin(db, &mut w2), WRITER_BUSY);
            for (k, v) in [(&b"b"[..], &b"2"[..]), (b"a", b"1"), (b"c", b"")] {
                assert_eq!(bebop_write_put(w, k.as_ptr(), k.len(), v.as_ptr(), v.len()), OK);
            }
            let mut g = 0;
            assert_eq!(bebop_write_commit(w, 0, &mut g), OK);
            let mut r = null_mut();
            assert_eq!(bebop_read_begin(db, &mut r), OK);
            assert_eq!(bebop_db_close(db), BUSY);
            assert_eq!(get(r, b"a"), Some(b"1".to_vec()));
            assert_eq!(get(r, b"c"), Some(vec![]));
            assert_eq!(get(r, b"z"), None);
            let mut s = null_mut();
            assert_eq!(bebop_read_scan(r, null(), 0, null(), 0, &mut s), OK);
            assert_eq!(bebop_read_end(r), BUSY);
            let mut keys = vec![];
            loop {
                let (mut has, mut kp, mut kl, mut vp, mut vl) = (0, null(), 0, null(), 0);
                assert_eq!(bebop_scan_next(s, &mut has, &mut kp, &mut kl, &mut vp, &mut vl), OK);
                if has == 0 {
                    break;
                }
                keys.push(if kl == 0 { vec![] } else { std::slice::from_raw_parts(kp, kl).to_vec() });
            }
            assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
            bebop_scan_free(s);
            assert_eq!(bebop_read_end(r), OK);
            assert_eq!(bebop_db_close(db), OK);
        }
    }

    #[test]
    fn key_too_long_is_refused_by_number_and_message() {
        unsafe {
            let mut db = null_mut();
            assert_eq!(bebop_db_create_mem(&mut db), OK);
            let mut w = null_mut();
            assert_eq!(bebop_write_begin(db, &mut w), OK);
            let k = vec![b'k'; 65_536];
            assert_eq!(bebop_write_put(w, k.as_ptr(), k.len(), null(), 0), KEY_TOO_LONG);
            let n = bebop_last_error(null_mut(), 0);
            let mut buf = vec![0u8; n];
            bebop_last_error(buf.as_mut_ptr(), n);
            assert!(String::from_utf8(buf).unwrap().contains("KeyTooLong"));
            bebop_write_abort(w);
            assert_eq!(bebop_db_close(db), OK);
        }
    }

    #[test]
    fn null_and_bad_args() {
        unsafe {
            let mut db = null_mut();
            assert_eq!(bebop_db_open_bytes(null(), 3, 0, &mut db), NULL_ARG);
            assert_eq!(bebop_db_create_mem(&mut db), OK);
            let mut g = 0;
            assert_eq!(bebop_db_flush(db, 7, &mut g), BAD_ARG);
            assert_eq!(bebop_db_verify(db, 9, &mut g), BAD_ARG);
            assert_eq!(bebop_db_close(db), OK);
        }
    }
}
