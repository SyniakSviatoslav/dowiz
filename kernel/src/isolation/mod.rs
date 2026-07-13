//! Isolation tiering — host-capability probes and adapter registration gates.
//!
//! Submodules:
//! - [`microvm`]: fail-closed host-capability probe (DK-06 / MV-04). A node
//!   without KVM refuses `native-process` adapters instead of running them
//!   unsandboxed.

pub mod microvm;
