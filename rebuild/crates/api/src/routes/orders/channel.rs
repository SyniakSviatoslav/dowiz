//! Strangler shim (STRUCTURE-UPGRADE A2) — channel attribution MOVED to the hub module
//! `crate::modules::channel_attribution` (with its module.toml manifest + write-only doctrine).
//!
//! This shim keeps every existing call site byte-identical: `routes/orders/mod.rs` still writes
//! `pub mod channel;` + `channel::channel_from_header(…)`, now resolving through this re-export.
//! No logic lives here — the code (and its tests) is in the module. Delete this file only when the
//! last call site is migrated to `crate::modules::channel_attribution::…` directly.
pub use crate::modules::channel_attribution::*;
