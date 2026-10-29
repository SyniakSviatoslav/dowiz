//! es-bench as a library, so the integration tests (tests/) drive the same adapters the binary does.
pub mod bebop;
pub mod data;
pub mod engine;
pub mod gate;
#[cfg(feature = "competitors")]
pub mod lmdb_e;
#[cfg(feature = "competitors")]
pub mod redb_e;
#[cfg(feature = "competitors")]
pub mod sqlite_e;
pub mod work;
