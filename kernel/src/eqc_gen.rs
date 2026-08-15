//! held-handle shim — re-exports from `dowiz_core::eqc_gen`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::eqc_gen::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::eqc_gen::*;
