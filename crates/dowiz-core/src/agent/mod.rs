//! agent — the bounded, fail-closed AgentLoop executor (WAVE P40).
//!
//! The pure no_std loop (`loop`) + model registry (`model_registry`) live in
//! `dowiz_core::agent`. `model_pair` stays here: it calls the drift gate from
//! `crate::hydra` (the full supervisor, whose `FileEventStore` is std-fs).

pub mod r#loop;
pub mod model_registry;
