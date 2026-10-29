//! bebop-store: THE FACADE (W-ES-P1, R-EMBED-STORE §5 row P1, 2026-10-29).
//!
//! The library is four crates now -- `bebop-image` (the format: cells, superblocks, PartTab,
//! objects, crc, pick/verify; no_std), `bebop-kv` (the KV layout and its readers; no_std),
//! `bebop-log` (the chained event log; no_std) and `bebop-io` (Device, the commit plan, Writer,
//! the enumerated crash harness). This crate re-exports them under the paths every caller
//! already used (`bebop_store::Store`, `bebop_store::kv::Kv`, `bebop_store::evlog::EvLog`,
//! `bebop_store::verify::KvError`, ...), so dowiz-hub, bebop-wasm and kernel did not change in
//! the commit that split it. It still holds the DAG runtime (`proj`, `zset`, `nodekey`,
//! `merkle`; operator Q6: they belong to bebop-lang, not to the library) and the tests that need
//! more than one of the four crates.
//!
//! RE-EXPORT, NEVER WRAP: a wrapper is code the wasm reader would carry (R-EMBED-STORE R2: +507 B
//! once from delegating through a trait); a `pub use` is a name.

pub use bebop_image::*;
pub use bebop_kv as kv;
pub use bebop_log as evlog;
pub use bebop_io as iodev;
pub use bebop_io::sim;

/// The format's names and limits (bebop-image). Mounted here as well, so the tests that need the
/// KV and PROJ layouts next to it (T-ES-02/03/08) keep their place.
pub mod format {
    pub use bebop_image::format::*;
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

/// The crc check on load, all three halves (W-CRC): the object (bebop-image), the KV
/// (bebop-kv), the log and its mark (bebop-log).
pub mod verify {
    pub use bebop_image::verify::BadCrc;
    pub use bebop_kv::verify::KvError;
    pub use bebop_log::verify::{ChainMark, ChainScan};
    #[cfg(test)]
    #[allow(unused_imports)]
    use crate::{crc32_cells, evlog::{EvLog, Record}, kv::Kv, Store, StoreError};
    #[cfg(test)]
    mod tests;
}

pub mod nodekey;
pub mod proj;
/// LV-INC-A: the Z-set, the one delta currency (R-BEBOP-LEVERS §2.4).
pub mod zset;
/// LV-PCD-C: the published root and its inclusion proof (R-BEBOP-LEVERS §4.4).
pub mod merkle;

#[cfg(test)]
mod view_tests;
