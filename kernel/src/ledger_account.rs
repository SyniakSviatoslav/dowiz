//! held-handle shim — re-exports from `dowiz_core::ledger_account`.
//!
//! The implementation lives in `dowiz-core`, beside the money and order-machine
//! laws it is built on. This file exists so `use crate::ledger_account::...` and
//! `dowiz_kernel::ledger_account::...` resolve unchanged, exactly as `money.rs` does.

pub use dowiz_core::ledger_account::*;
