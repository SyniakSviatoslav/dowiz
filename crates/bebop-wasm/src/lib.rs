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

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// The store, or the refusal that stands for "no valid superblock fits here".
fn open(bytes: &[u8]) -> Result<Store, Refusal> {
    let st = Store::from_bytes(bytes);
    if st.pick().is_none() {
        return Err(Refusal::NoSuperblock);
    }
    Ok(st)
}

/// Read a KV image to its count and root.
pub fn kv_view(bytes: &[u8]) -> Result<KvView, Refusal> {
    let st = open(bytes)?;
    let kv = Kv::load(&st).ok_or(Refusal::NotAKv)?;
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
    let chained = EvLog::chain_len(&st);
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
