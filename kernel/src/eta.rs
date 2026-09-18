//! held-handle shim — re-exports from `dowiz_core::eta`.
//!
//! The implementation lives in `dowiz-core`, beside the money and order laws it
//! is built on. This file exists so `dowiz_kernel::eta::...` resolves, exactly
//! as `money.rs` does.

pub use dowiz_core::eta::*;
