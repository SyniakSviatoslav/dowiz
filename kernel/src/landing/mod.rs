//! `landing` — std host shim.
//!
//! The pure render-agnostic landing core (`journey` FSM, `form` validation,
//! `claim_client` port, constants, `SemanticScene` authoring) lives in
//! `dowiz_core::landing`. Only the full-wgpu hero-render test harness
//! (`tests.rs`, Lane B / O18a) stays here — it drives the actual `wgpu`
//! instance and cannot live in the no_std core.

pub use dowiz_core::landing::*;

#[cfg(test)]
mod tests;
