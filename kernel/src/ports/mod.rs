//! ports/mod.rs — external capability ports (the seams where the kernel meets the outside world
//! without importing it). Each submodule is a `pub mod` with a one-line doc, matching the
//! `retrieval/mod.rs` / `isolation/mod.rs` convention in this crate.

/// `LlmBackend` port — pluggable local/managed LLM backend trait + value types (zero HTTP/serde).
pub mod llm;

/// `AgentBridge` port (B1) — mandatorily hybrid-signed, fail-closed agent-admission seam
/// + the `AgentManifest` canonical TLV + the admission path (zero HTTP/serde/wasmtime).
pub mod agent;
