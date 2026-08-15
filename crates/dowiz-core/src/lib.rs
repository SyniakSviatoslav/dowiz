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
pub mod eqc_gen;
pub mod fft;
pub mod harmonic;
pub mod householder;
pub mod hypervector;
pub mod lut;
pub mod mat;
pub mod math;
pub mod messenger;
pub mod metrics;
pub mod modular;
pub mod needle2;
pub mod oil_motion;
pub mod penecho;
pub mod readability;
pub mod rng;
pub mod sanitize;
pub mod scenario;
pub mod semantic;
pub mod sort;
pub mod span;
pub mod spectral;
pub mod spherical;
pub mod spool;
pub mod squash;
pub mod stem;
pub mod swarm;
pub mod tri_state;
pub mod trig;
pub mod trigram;
pub mod verify_retrieval;
pub mod dsu;
pub mod impedance;
pub mod noether;
pub mod absorbing;
pub mod vendor;

// Re-export TriState at the crate root so `dowiz_core::TriState` resolves —
// the kernel mirrors this with `pub use dowiz_core::TriState;` at its root.
pub use tri_state::TriState;

// Mirror the kernel's crate-root re-exports so `crate::sanitize_f64` /
// `crate::sort_by_f64_desc` resolve identically in dowiz-core (they live in
// the `sanitize` / `sort` modules, but call sites spell them crate-root).
pub use sanitize::{sanitize_f32, sanitize_f64, sanitize_normalized};
pub use sort::{sort_by_f64_asc, sort_by_f64_desc};
pub mod landmark;
pub mod attention;
pub mod micrograd;
pub mod pixel_snapshot;
pub mod resonance;
pub mod pid;
pub mod intake;
pub mod money;
pub mod hex_util;
pub mod telemetry;
pub mod github_patterns;
pub mod incidence;
pub mod optical;
pub mod budget;
pub mod detection;
pub mod cgraph;

// --- wave 3: leaves + primitives ---
pub mod power_forecast;
pub mod crystal;
pub mod deploy_config;
pub mod support;
pub mod json;
pub mod spectral_cache;
pub mod delta;
pub mod geo;
pub mod online;
pub mod entropy_budget;
pub mod math_guard;
pub mod numerical_guard;
pub mod gboost;
pub mod glyph_dashboard;
pub mod neon;
pub mod spinlock;
pub mod parse;

// --- wave 4: dependency-clean leaves ---
pub mod cart;
pub mod event_log;
pub mod hypervector_index;
pub mod invert;
pub mod kalman;
pub mod laplacian_eqc_parity;
pub mod markov;
pub mod predictor;
pub mod router;
pub mod spectral_laplacian;
pub mod tensor;
/// Spectral Parsing: O(n⁰) paper extraction — raw OAI-PMH byte scanner (no XML
/// DOM), tensor ASCII storage, spectral decomposition search, harmonic ranking.
/// Extracted from the kernel; the network/process methods (`curl`, `Instant`) are
/// std-gated behind `feature = "std"` so the no_std core stays process-free.
pub mod spectral_parser;
pub mod weave;
pub mod checksum;

// --- wave 5: dependency-clean leaves ---
pub mod self_harness;
pub mod sha256_hw;
pub mod snapshot;
pub mod spine;
pub mod trading_intent;
pub mod trading_escrow;
pub mod clock_stabilizer;
pub mod moderation;
pub mod reverse_engineer;
pub mod workflow_gate;
pub mod canonical;
pub mod channel;
pub mod chunker;
pub mod hypergraph;
pub mod mesh_replication;
pub mod p2p_delivery;
pub mod spectral_graph;
pub mod cooperation_protocol;
pub mod parametric_spectral;
pub mod research;
pub mod research_ascii;
pub mod agent_browser;
pub mod self_reproduce;
pub mod three_phase;
pub mod evolution_go;
pub mod field_eigenmodes;
pub mod clock;
pub mod time_stabilizer;
pub mod order_machine;
pub mod vfs;
pub mod typed_metrics;
pub mod kthread;
pub mod kprocess;

// --- wave 6: batch-5 leaves (analytics / stats) ---
pub mod analytics;
pub mod stats;

// --- wave 7: leaves (determinism goldens) ---
pub mod determinism;

// --- wave 8: held-handle seams (fxhash / signal / proxy_redirect / ct_gate / academia / hw_profile / slot_arena) ---
pub mod fxhash;
pub mod signal;
pub mod proxy_redirect;
pub mod ct_gate;
pub mod academia;
pub mod hw_profile;
pub mod slot_arena;

// --- wave 8: post-quantum crypto primitives (FIPS 203/204) ---
pub mod pq;
pub mod crypto_signer;

// --- wave 9: P9 research leaves (oracle / causal inference) ---
pub mod oracle;
pub mod causal;
pub mod predict;
pub mod wave;
pub mod trinary;
pub mod fractal;
pub mod chronos_topology;
pub mod chronos;
pub mod code_oracle;
pub mod sys_dashboard;
pub mod gossip;
pub mod tracker;
