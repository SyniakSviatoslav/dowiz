//! ports/mod.rs — external capability ports (the seams where the kernel meets the outside world
//! without importing it). Each submodule is a `pub mod` with a one-line doc, matching the
//! `retrieval/mod.rs` / `isolation/mod.rs` convention in this crate.

/// `LlmBackend` port — pluggable local/managed LLM backend trait + value types (zero HTTP/serde).
pub mod llm;

/// `AgentBridge` port (B1) — hybrid-signed, fail-closed agent-admission seam.
pub mod agent;

/// P49 — per-order customer identity (capability grant, privacy-minimal, no
/// device/personal data). Reuses the proto-cap signing *convention* (domain-
/// separated SHA3 commitment) over existing kernel math (geo/kalman/rng).
pub mod customer;
