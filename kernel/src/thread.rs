//! held-handle shim — re-exports from `dowiz_core::thread`.
//!
//! The implementation lives in `dowiz-core`, beside the money and order-machine
//! laws it is built on. This file exists so `use crate::thread::...` and
//! `dowiz_kernel::thread::...` resolve unchanged, exactly as `money.rs` does.

pub use dowiz_core::thread::*;
