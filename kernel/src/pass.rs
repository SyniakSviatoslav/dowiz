//! held-handle shim — re-exports from `dowiz_core::pass`.
//!
//! The implementation lives in `dowiz-core`, beside the money and order-machine
//! laws it is built on. This file exists so `use crate::pass::...` and
//! `dowiz_kernel::pass::...` resolve unchanged, exactly as `money.rs` does.

pub use dowiz_core::pass::*;
