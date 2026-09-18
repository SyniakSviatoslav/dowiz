//! held-handle shim — re-exports from `dowiz_core::reservation`.
//!
//! The implementation lives in `dowiz-core`, beside the money and order-machine
//! laws it is built on. This file exists so `use crate::reservation::...` and
//! `dowiz_kernel::reservation::...` resolve unchanged, exactly as `money.rs` does.

pub use dowiz_core::reservation::*;
