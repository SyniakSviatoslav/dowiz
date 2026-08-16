//! §H toy-pilot inference arc — std host shim.
//!
//! The pure no_std inference modules (`fixed`, `oracle`, `simd_i8`, `spec`,
//! `workspace` + `model_version_hash`) live in `dowiz_core::inference`. The
//! `golden` checksum guard stays here: its production path writes typed alarms
//! through the std `fdr::RingHandle` (file-backed ring), which the no_std core
//! cannot hold.

pub use dowiz_core::inference::*;

pub mod golden;
