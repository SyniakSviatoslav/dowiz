//! held-handle shim — re-exports from `dowiz_core::p2p_delivery`.
//!
//! The implementation lives in `dowiz-core`. This file stays so that
//! `use crate::p2p_delivery::...` resolves unchanged in kernel code.
//! Kernel-specific extensions (if any) are added below the re-export.

pub use dowiz_core::p2p_delivery::*;
