//! held-handle shim — re-exports from `dowiz_core::cgraph`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::cgraph::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::cgraph::*;
