//! agent-adapters — concrete `AgentBridge` adapters for dowiz.
//!
//! Sibling of `llm-adapters`, same structure and the SAME composition discipline:
//! `AgentDispatcher<CachingBackend<McpServerBridge, S>>` mirrors
//! `Dispatcher<CachingBackend<OllamaAdapter, S>>`. One generic JSON-RPC 2.0 transport
//! (zero framework knowledge), one `AgentQuirks` per bridged framework, the reused
//! `Dispatcher` pattern (pre-acquire budget → typed refusal → `TrackRecord` harvest on
//! both poles), and per-agent cache partitioning (SH-2 default-deny).
//!
//! The ONE reference `Quirks` profile is the MCP-server bridge (`AgentQuirks::mcp_server`):
//! MCP's open-world string grammar NEVER enters the signed manifest — the bridge produces
//! a draft, the operator's keys sign the final closed-enum manifest, and an operator-
//! authored tool allowlist maps tool names to closed `(Resource, Action)` scopes
//! (unmapped tools are a fail-closed drop). The manifest is anchored to a `sha3_256` of
//! the canonical sorted tool set so post-admission drift is detectable.
//!
//! The Wasmtime fuel↔`TokenBucket` wiring lives in [`fuel`]; the real wasmtime backend is
//! behind the `wasmtime-fuel` feature (the default build uses a deterministic reference
//! `FuelMeter`). `FUEL_PER_UNIT` is a B4-pending placeholder (see the kernel constant).

pub mod cache;
pub mod dispatch;
pub mod fuel;
pub mod manifest;
pub mod mcp;
pub mod quirks;
pub mod transport;

pub use cache::{AgentCache, CacheProvisioner};
pub use dispatch::{AgentDispatchError, AgentDispatcher, HarvestSink, TrackRecord, VecHarvest};
pub use fuel::{DeterministicFuelMeter, FuelError, FuelMeter, FuelTrancheRunner, SliceOutcome};
pub use manifest::draft_manifest;
pub use mcp::McpServerBridge;
pub use quirks::{AgentQuirks, TransportKind};
pub use transport::{JsonRpcTransport, MockChannel, RpcChannel};

// Re-export the kernel agent-port surface so callers pick it from one crate (as
// llm-adapters re-exports `CachePolicy`).
pub use dowiz_kernel::ports::agent::{
    AgentBridge, AgentCaps, AgentError, AgentInvocation, AgentManifest, AgentResponse, AgentTask,
};
