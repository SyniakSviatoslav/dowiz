//! The bebop image format, read to one number, on any target.
//!
//! Two families, the two the product stores (`crates/dowiz-hub`): a KV image
//! (`kv.bp` / `bebop_store::kv`) and a chained event log (`bebop_store::evlog`).
//! Each is read to a count and a fold that every other reader of the same
//! bytes can be asked for:
//!
//!   KV  : `n` from the root, `root` = FNV-1a 64 over `len||key||len||value`
//!         frames in key order -- byte-for-byte `kv.bp`'s `kv_snapshot`,
//!         `bebop_store::kv::Kv::snapshot_root_u64` and dowiz-core's
//!         `InMemoryStore::snapshot_root`, printed as the two's-complement i64
//!         that `kv.bin h` prints.
//!   LOG : `len` from the root, `fold` = FNV-1a 64 over
//!         `seq||payload_len||payload` frames OLDEST FIRST. Defined here (the
//!         evlog family had no scalar fold before); `oracle.py` mirrors it.
//!
//! EVERYTHING REFUSES, NOTHING PANICS. The bytes arrive from a network or a
//! file and one flipped bit must come back as a `Refusal` the host can print,
//! never as a trap. `bebop-store` already bounds every claim against the image
//! (its `obj_cells`, `follow`, `Kv::load` returning `None`); this layer turns
//! those answers into a typed refusal and adds the one check a scalar reader
//! needs: a log whose root claims more records than its chain delivers is
//! `Truncated`, exactly as `dowiz_hub::Hub::load` refuses it.

pub mod abi;
/// W-ES-P8D: KV v4, the B+tree, an allocation-free walk of the image (`kv_view` routes version 4 here).
pub mod kv4;
/// DG9: the columnar block, decoded and re-encoded. Compiled only for tests and under
/// `--cfg bw_block` (gate.sh's separate module, which exports `bw_block`): measured 2026-09-30,
/// merely compiling it into the Worker's module moved `bytes.baseline` 31626 -> 31769.
#[cfg(any(test, bw_block))]
pub mod block;
/// BN3: the block read IN PLACE (DG7's `view` twin) and the published menu's price lookup;
/// gated exactly like `block` (`bw_view` / `bw_menu` live in the same separate module).
#[cfg(any(test, bw_block))]
pub mod block_view;
pub mod nodekey;
/// DG5: a log image's projection memo, read (`bw_proj`, exported under the `proj` feature).
pub mod proj;
/// LV-PCD-C: a receipt line's price certificate, climbed (`bw_verify`, `bw_qr`; `--features verify`).
#[cfg(any(test, feature = "verify"))]
pub mod verify;
#[cfg(feature = "decide")]
pub mod decide;
#[cfg(feature = "decide")]
pub mod decide_abi;
/// Sheaf S2: the kitchen tablet's writer over `dowiz_hub::glue::tablet` (`--features tablet`).
#[cfg(feature = "tablet")]
pub mod tablet;

use bebop_store::evlog::EvLog;
use bebop_store::kv::{Kv, FNV_OFFSET, FNV_PRIME};
use bebop_store::Store;

/// Why an image was not read. Each variant is one status code in `abi.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Neither superblock is valid, or its arithmetic does not fit the image.
    NoSuperblock,
    /// The root is not a KV root, or a slice in it does not fit its blob.
    NotAKv,
    /// The root is not a log root.
    NotALog,
    /// The log's root claims `claimed` records; the chain delivers `chained`
    /// (`None` = the chain never ends).
    Truncated { claimed: usize, chained: Option<usize> },
    /// An object's payload does not match its header crc (W-CRC): `obj` is its cell.
    BadCrc { obj: usize },
    /// The KV root names a version this reader does not know (W-ES-P8A; v4 reads since W-ES-P8D).
    UnknownVersion,
    /// The last commit's range does not hash to its anchor and every object in it checks
    /// (W-ES-P10, FORMAT §7 phase P2; an object that fails is `BadCrc` naming it).
    LastCommit,
}

/// A KV image, folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvView {
    pub n: i64,
    pub root: i64,
}

/// A log image, folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogView {
    pub len: i64,
    pub fold: i64,
}

pub(crate) fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// The store, or the refusal that stands for "no valid superblock fits here", or the last
/// commit's refusal (W-ES-P10, FORMAT §7 phase P2): a torn anchor is never fallen back past here.
///
/// `last_commit` is a SEPARATE SHORT COPY of `bebop_image::verify::last_commit_in` for the reason
/// `Store::pick` is one of `pick_checked_in` (FORMAT §3.1): this module's byte ratchet only falls,
/// and the library rule is generic over `Cells` (`crc32_in` / `bebop_crc::cells_by`), which this
/// module does not otherwise link; `Store::check_obj` and `crc32_cells` it already does. The copy
/// is held equal to the rule on every fixture by `tests/parity.rs`
/// `the_last_commit_copy_equals_the_library_rule`.
fn open(bytes: &[u8]) -> Result<Store, Refusal> {
    let st = Store::from_bytes(bytes);
    let Some(sb) = st.pick() else {
        return Err(Refusal::NoSuperblock);
    };
    last_commit(&st, &sb)?;
    Ok(st)
}

/// FORMAT §7 phase P2 over a `Store`: `Ok` when the anchor holds or is absent; a torn anchor
/// names the first object of `[mark, arena_used)` whose crc fails, else `LastCommit`. `pub` only
/// so `tests/parity.rs` can hold it against the library rule; it is not an export.
#[doc(hidden)]
pub fn last_commit(st: &Store, sb: &bebop_store::Superblock) -> Result<(), Refusal> {
    let a = st.cells.get(sb.at + bebop_store::SB_ANC).copied().unwrap_or(0) as u64;
    if a == 0 {
        return Ok(());
    }
    let (mark, used) = ((a >> 32) as usize, sb.arena_used as usize);
    if mark < bebop_store::ARENA || mark > used || used > st.cells.len() {
        return Err(Refusal::LastCommit);
    }
    if bebop_store::crc32_cells(&st.cells, mark, used - mark) == a as u32 {
        return Ok(());
    }
    let mut i = mark;
    while i < used {
        if !st.obj_crc_ok(i) {
            return Err(Refusal::BadCrc { obj: i });
        }
        i = i.saturating_add(2).saturating_add(st.obj_len(i).max(0) as usize);
    }
    Err(Refusal::LastCommit)
}

/// Read a KV image to its count and root.
pub fn kv_view(bytes: &[u8]) -> Result<KvView, Refusal> {
    let st = open(bytes)?;
    // v4 (W-ES-P8D): the root says so; `Kv::load_checked` in this build (no `tree` feature) would
    // refuse it as UnknownVersion, so the version is asked first and v4 goes to `kv4`.
    if Kv::version(&st) == bebop_store::kv::tree::VERSION_TREE {
        if let Some(root) = st.root() {
            return kv4::kv4_view(&st, bytes, root);
        }
    }
    let kv = Kv::load_checked(&st).map_err(|e| match e {
        bebop_store::verify::KvError::BadCrc(b) => Refusal::BadCrc { obj: b.obj },
        // Spelled as a compare, not a third arm: measured 2026-10-29, the plain three-arm match
        // is a br_table and costs 21 bytes more of the ratchet (46000 vs 45979).
        e => if e == bebop_store::verify::KvError::NotKv { Refusal::NotAKv } else { Refusal::UnknownVersion },
    })?;
    Ok(KvView {
        n: kv.entries.len() as i64,
        root: kv.snapshot_root_u64() as i64,
    })
}

/// Read a log image to its count and fold.
pub fn log_view(bytes: &[u8]) -> Result<LogView, Refusal> {
    let st = open(bytes)?;
    // A KV root is 5 cells and names four arrays; a log root is 7 or 8 cells
    // whose cell 1 is the newest record. The digest in the object header is
    // what says which; the layout digest for a log root is pinned in evlog.rs.
    let root = st.root().ok_or(Refusal::NotALog)?;
    if st.obj_digest(root) != bebop_store::evlog::DIGEST_EVLOG_ROOT {
        return Err(Refusal::NotALog);
    }
    let claimed = EvLog::len(&st);
    // The count and every record's crc in one walk (W-CRC), as `dowiz_hub::chain_is_whole`.
    let chained = EvLog::chain_crc(&st).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
    if chained != Some(claimed) {
        return Err(Refusal::Truncated { claimed, chained });
    }
    let mut records = EvLog::walk(&st);
    records.reverse();
    let mut h = FNV_OFFSET;
    for r in &records {
        h = fnv1a(h, &r.actor_seq.to_le_bytes());
        h = fnv1a(h, &(r.payload.len() as u64).to_le_bytes());
        h = fnv1a(h, &r.payload);
    }
    Ok(LogView { len: claimed as i64, fold: h as i64 })
}
