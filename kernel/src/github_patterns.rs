//! held-handle shim — re-exports from `dowiz_core::github_patterns`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::github_patterns::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::github_patterns::*;
