//! ports/mod.rs — external capability ports (the seams where the kernel meets the outside world
//! without importing it). Each submodule is a `pub mod` with a one-line doc, matching the
//! `retrieval/mod.rs` / `isolation/mod.rs` convention in this crate.

/// `LlmBackend` port — pluggable local/managed LLM backend trait + value types (zero HTTP/serde).
pub mod llm;

/// `AgentBridge` port (B1) — hybrid-signed, fail-closed agent-admission seam.
pub mod agent;

/// `ToolPort` firewall (P40/P42) — the closed tool authority (writes
/// UNREPRESENTABLE) + the Skills-pattern discovery layer (P42).
pub mod tool;

/// MCP port + capability-scoped tool boundary (P42) — verify_chain-gated,
/// fail-closed, Skill-discovering. Typed; the stdio/JSON framing lives downstream.
pub mod mcp;
