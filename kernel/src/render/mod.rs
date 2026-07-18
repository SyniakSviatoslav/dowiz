//! P38 O18a — graphics unlock (feature-gated GPU render backend).
//!
//! The canonical kernel is the bit-deterministic order/money authority and the
//! GPU is *presentation only* (BLUEPRINT-P38 §3.1 honesty split). This module
//! is the O18a unlock seam: it compiles to NOTHING in the default build and
//! carries a REAL headless wgpu bring-up only behind the additive `gpu` feature.
//!
//! Rollback discipline (BLUEPRINT-P38 §4.5): `--no-default-features`/no `gpu`
//! ⇒ zero GPU symbols; the feature is purely additive and guard-restorable.

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "gpu")]
pub use gpu::{init, GpuContext, GpuError};
