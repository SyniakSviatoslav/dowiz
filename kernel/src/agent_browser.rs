//! held-handle shim — re-exports from `dowiz_core::agent_browser`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::agent_browser::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::agent_browser::*;
