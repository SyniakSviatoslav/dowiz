//! dowiz-core — the `no_std` core of dowiz.
//!
//! Zero dependencies, pure `core::` (no alloc, no std). The first modules to be
//! extracted from `dowiz-kernel` for native/kernel-space embedding:
//! - [`constants`] — branchless constants (PI, TAU, PHI, GOLDEN_ANGLE, …).
//! - [`lut`] — O(1) branchless lookup tables (`Lut<K,V,N>`, `BinaryLut`).
//!
//! This crate compiles under `#![no_std]`; `dowiz-kernel` depends on it and
//! re-exports both modules so `crate::constants::…` / `crate::lut::…` keep
//! working unchanged.
#![cfg_attr(not(test), no_std)]
// The test harness (`#[test]`) needs std; link it only under `cfg(test)`.
#[cfg(test)]
extern crate std;

pub mod constants;
pub mod hypervector;
pub mod lut;
pub mod rng;
