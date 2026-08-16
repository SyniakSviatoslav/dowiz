//! agent — the bounded, fail-closed AgentLoop executor (std host shim).
//!
//! The pure no_std loop + model registry live in `dowiz_core::agent`.
//! `model_pair` stays here: it calls the drift gate from `crate::hydra`
//! (the full supervisor, whose `FileEventStore` is std-fs).

pub use dowiz_core::agent::*;

pub mod model_pair;
