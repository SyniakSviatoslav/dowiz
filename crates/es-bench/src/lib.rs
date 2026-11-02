//! es-bench as a library, so the integration tests (tests/) drive the same adapters the binary does.
/// The reader and format text an archive embeds (needs the DOC API behind `mldsa`).
#[cfg(feature = "mldsa")]
pub mod archive;
pub mod einfile;
/// The v4 (B+tree) adapter through `Db` (W-ES-P8C).
pub mod einfile4;
pub mod data;
pub mod engine;
pub mod gate;
#[cfg(feature = "competitors")]
pub mod lmdb_e;
#[cfg(feature = "competitors")]
pub mod redb_e;
#[cfg(feature = "competitors")]
pub mod sqlite_e;
/// The `# machine` header of every `full` run.
pub mod machine;
/// The real ML-DSA-65 behind the seams: the one file that names the provider.
#[cfg(feature = "mldsa")]
pub mod mldsa;
/// The rows only a signed file has.
#[cfg(feature = "mldsa")]
pub mod signed;
pub mod work;
