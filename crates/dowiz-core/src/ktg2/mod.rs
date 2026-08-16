//! KTG-2: a Rust-native, exokernel-managed 2-bit graph dataflow core.
//!
//! There are no matrix/tensor abstractions in this organ. Computation is a
//! graph of nodes and local 2×2 topologies. The exokernel leases resources;
//! policy and scheduling remain outside the privileged core.
//!
//! Layered modules:
//! - [`cell`]   — canonical 2-bit [`cell::State`] (three-valued, invalid code rejected).
//! - [`graph`]  — packed node-state store (4 states/byte).
//! - [`tile2x2`] — 2×2 systolic tile with node-gated datapath.
//! - [`telemetry`] — allocation-free counters.
//! - [`exokernel`] — resource leases (nodes/tiles/credits).
//! - [`fractal`] — fractal bit (ZERO = -64, cos/sin geometry).
//! - [`fractal_manchester`] — Fractal Manchester Architecture (transitions + optical transport).
//! - [`wire`] — FMA transport wired into the tile datapath (encode/round-trip/sync).

pub mod cell;
pub mod exokernel;
pub mod fractal;
pub mod fractal_manchester;
pub mod graph;
pub mod telemetry;
pub mod tile2x2;
pub mod wire;
