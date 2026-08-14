//! dowiz-core — the `no_std` core of dowiz.
//!
//! Zero dependencies, pure `core::` + `alloc::` (no std). The modules extracted
//! from `dowiz-kernel` for native/kernel-space embedding:
//! - [`constants`] — branchless constants (PI, TAU, PHI, GOLDEN_ANGLE, …).
//! - [`lut`] — O(1) branchless lookup tables (`Lut<K,V,N>`, `BinaryLut`).
//! - [`sanitize`] — fail-closed f64/f32 boundary sanitizers.
//! - [`stem`] — 50-language light stemmer.
//! - [`eigen`] — eigenvalue/eigenvector data primitive.
//! - [`math`] — bit-exact f64 `sqrt`/`fma`/`hypot` + ~1-ULP transcendentals.
//! - [`trig`], [`complex`], [`modular`], [`fft`], [`spherical`] — geometry layer.
//! - [`hypervector`], [`rng`], [`squash`] — VSA / PRNG / RLE+delta compression.
//!
//! This crate compiles under `#![no_std]`; `dowiz-kernel` depends on it and
//! re-exports the modules so `crate::constants::…` / `crate::stem::…` /
//! `crate::eigen::…` keep working unchanged.
#![cfg_attr(not(test), no_std)]
// The test harness (`#[test]`) needs std; link it only under `cfg(test)`.
#[cfg(test)]
extern crate std;

// squash + pixel_snapshot use Vec/String → alloc is needed for the no_std core.
#[macro_use]
extern crate alloc;

pub mod arena;
pub mod complex;
pub mod constants;
pub mod csr;
pub mod dflash;
pub mod eigen;
pub mod fft;
pub mod harmonic;
pub mod householder;
pub mod hypervector;
pub mod lut;
pub mod mat;
pub mod math;
pub mod modular;
pub mod oil_motion;
pub mod penecho;
pub mod rng;
pub mod sanitize;
pub mod scenario;
pub mod sort;
pub mod span;
pub mod spectral;
pub mod spherical;
pub mod squash;
pub mod stem;
pub mod trig;
pub mod verify_retrieval;
pub mod dsu;
pub mod impedance;
pub mod noether;
pub mod absorbing;
