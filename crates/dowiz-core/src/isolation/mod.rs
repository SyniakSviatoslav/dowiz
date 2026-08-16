//! Isolation tiering — host-capability probe seam and adapter-registration gates.
//!
//! Submodules:
//! - [`microvm`]: the pure `SandboxTier` / `AdapterRejected` value types and a
//!   fail-closed `register_adapter` seam. The std host probe (`/dev/kvm` +
//!   `/proc/cpuinfo`) stays in the kernel and is injected via `set_kvm_probe`.

pub mod microvm;
