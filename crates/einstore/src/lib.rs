//! einstore: THE FACADE (W-ES-P1, R-EMBED-STORE §5 row P1, 2026-10-29).
//!
//! The library is four crates now -- `einfile-image` (the format: cells, superblocks, PartTab,
//! objects, crc, pick/verify; no_std), `einfile-kv` (the KV layout and its readers; no_std),
//! `einfile-log` (the chained event log; no_std) and `einfile-io` (Device, the commit plan, Writer,
//! the enumerated crash harness). This crate re-exports them under the paths every caller
//! already used (`einstore::Store`, `einstore::kv::Kv`, `einstore::evlog::EvLog`,
//! `einstore::verify::KvError`, ...), so dowiz-hub, dowiz-hub-wasm and kernel did not change in
//! the commit that split it. It still holds the DAG runtime (`proj`, `zset`, `nodekey`,
//! `merkle`; operator Q6: they belong to bebop-lang, not to the library) and the tests that need
//! more than one of the four crates.
//!
//! RE-EXPORT, NEVER WRAP: a wrapper is code the wasm reader would carry (R-EMBED-STORE R2: +507 B
//! once from delegating through a trait); a `pub use` is a name.

pub use einfile_image::*;
pub use einfile_kv as kv;
pub use einfile_log as evlog;
pub use einfile_io as iodev;
pub use einfile_io::sim;
/// W-ES-P3: the library API (R-EMBED-STORE §3.2) under the facade's name, so a caller writes
/// `einstore::{Db, ReadTxn, WriteTxn, Refusal}` as it writes `einstore::Store`.
pub use einfile_io::{
    Batch, Bytes, Db, Detached, Durability, Generation, LimitError, Limits, OnTorn, Options, ReadTxn, Refusal, Report, Scan,
    SnapImage, Verify, WriteTxn,
};

/// The format's names and limits (einfile-image). Mounted here as well, so the tests that need the
/// KV and PROJ layouts next to it (T-ES-02/03/08) keep their place.
pub mod format {
    pub use einfile_image::format::*;
    #[cfg(test)]
    #[allow(unused_imports)]
    use crate::{crc32_in, Cells, Store, Superblock, ARENA, MAGIC, SB_A, SB_B};
    #[cfg(test)]
    mod tests;
    #[cfg(test)]
    mod catalogue_tests;
    #[cfg(test)]
    mod catalogue_kv_tests;
}

/// The crc check on load, all three halves (W-CRC): the object (einfile-image), the KV
/// (einfile-kv), the log and its mark (einfile-log).
pub mod verify {
    pub use einfile_image::verify::{check_obj_in, last_commit_in, BadCrc, LastCommit};
    pub use einfile_kv::verify::KvError;
    pub use einfile_log::verify::{ChainMark, ChainScan};
    #[cfg(test)]
    #[allow(unused_imports)]
    use crate::{crc32_cells, evlog::{EvLog, Record}, kv::Kv, Store, StoreError};
    #[cfg(test)]
    mod tests;
}

pub mod nodekey;
mod sha_fast;
pub mod proj;
/// LV-INC-A: the Z-set, the one delta currency (R-BEBOP-LEVERS §2.4).
pub mod zset;
/// LV-PCD-C: the published root and its inclusion proof (R-BEBOP-LEVERS §4.4).
pub mod merkle;

#[cfg(test)]
mod view_tests;

/// README.md's quickstart, compiled and run by `cargo test --doc`. `doctest` is set only there, so
/// no build of this crate carries this item.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctest;
