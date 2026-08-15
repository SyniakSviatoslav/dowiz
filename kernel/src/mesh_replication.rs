//! held-handle shim — re-exports from `dowiz_core::mesh_replication`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::mesh_replication::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::mesh_replication::*;
