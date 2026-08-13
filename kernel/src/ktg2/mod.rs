//! KTG-2: a Rust-native, exokernel-managed 2-bit graph dataflow core.
//!
//! There are no matrix/tensor abstractions in this organ. Computation is a
//! graph of nodes and local 2×2 topologies. The exokernel leases resources;
//! policy and scheduling remain outside the privileged core.

pub mod cell;
pub mod exokernel;
pub mod graph;
pub mod telemetry;
pub mod tile2x2;
