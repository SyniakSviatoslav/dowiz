//! held-handle shim — re-exports from `dowiz_core::impedance`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::impedance::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::impedance::*;
