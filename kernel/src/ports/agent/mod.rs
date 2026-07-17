//! ports/agent — the `AgentBridge` port (B1): a mandatorily hybrid-signed, fail-closed,
//! enumerable-only agent-admission seam that generalizes the proven `LlmBackend` /
//! `llm-adapters` pattern to arbitrary bridged agents (an MCP server, a LangGraph graph,
//! a bare binary).
//!
//! # Compile firewall (mirrors `ports/llm.rs:3-7`)
//! This module tree has ZERO network / HTTP / JSON / serde. It defines only the abstract
//! contract (`AgentBridge` trait), the plain value types the admission decision is made
//! from (`AgentManifest`, `Capability`, `SignedFrame`, `AnchorRoster`, `RevocationSet`,
//! …), the closed enums (`Resource`/`Action`/`AgentCaps`/`ValidationPolicy`/…), and the
//! admission logic. The concrete adapter crate (`agent-adapters`, repo root) owns all
//! transport/JSON and the Wasmtime embedding — exactly as `llm-adapters` owns HTTP/JSON
//! for `ports/llm.rs`. `cargo tree -p dowiz-kernel` must show no HTTP client / no
//! Wasmtime here (the migration-step-2 done-check).
//!
//! # Reuse-not-reinvent (the M6 seam)
//! Per B1's thesis, the trust machinery is REUSED, not invented: the verification is the
//! bebop2 `HybridGate::check` sequence (dual Ed25519 ⊕ ML-DSA-65 under an unrelaxable
//! `RequireBoth` floor) reproduced faithfully in [`admission::ReferenceHybridGate`], with
//! the real cryptographic primitive behind the [`cap::SignatureVerifier`] seam so a
//! production node injects the real bebop2 verifier without changing this port. See
//! `cap.rs`'s module doc for why this is a trait seam (the dowiz kernel does not link
//! bebop2 `proto-cap`, and B1 DoD item 7 defers the concrete cross-repo hard-link).

pub mod admission;
pub mod cap;
pub mod manifest;
pub mod scope;

pub use admission::{
    AdmissionError, AdmissionGate, AdmissionLimiter, AdmissionRecord, Admitter, ReferenceHybridGate,
    B1_NEW_SCOPES, DEFAULT_MAX_AGENT_DEPTH, FUEL_PER_UNIT, MAX_VERIFY_CHAIN_LINKS, TRANCHE_UNITS,
};
pub use cap::{
    pq_key_id, revocation_hash, verify_chain, AnchorRoster, Capability, ChainError, Delegation,
    HybridPolicy, NodeId, RefSigner, RevocationSet, SignatureVerifier, SignedFrame, ML_DSA_65_PK_LEN,
};
pub use manifest::{
    config_axis_domain, AgentCaps, AgentManifest, BudgetRequest, CostDenomination, ExecutionModel,
    ManifestParseError, QuirksProfile, ResourceNeed, ValidationPolicy,
};
pub use scope::{Action, RedLinePolicy, Resource, Scope};

/// Typed agent-bridge error (the `LlmError` generalization). `health()` and any probe
/// return a typed `Err` when the bridged agent is absent or refuses — never a mock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// Bridge endpoint/process is not reachable (health failed). Fail-closed.
    Unavailable,
    /// The requested capability is not offered by this bridge.
    Unsupported,
    /// Malformed request.
    BadRequest(String),
    /// Request timed out.
    Timeout,
    /// The requested tool name is not in the operator-authored allowlist (fail-closed
    /// drop — never a string passthrough).
    ToolNotAllowed(String),
    /// The live tool-map digest no longer matches the admitted digest (`listChanged` /
    /// server substitution — registry-poisoning class). Refuse and require re-admission.
    DigestDrift,
    /// A server-initiated `sampling/createMessage` or elicitation request was refused —
    /// an admitted agent must never drive the host's LLM (RC-2 control inversion).
    ControlInversionRefused,
    /// The per-agent budget envelope is exhausted (F2). Degrade-closed.
    BudgetExceeded,
    /// The invocation's delegation depth reached the granted cap (F10).
    DepthExceeded,
    /// A typed refusal with context.
    Refused(String),
}

/// The unit of work a bridged agent is asked to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTask {
    /// Execute a tool call (mapped through the operator allowlist to a closed scope).
    InvokeTool {
        /// The MCP tool name (validated against the allowlist).
        name: String,
        /// Opaque, already-validated arguments.
        args: Vec<u8>,
    },
    /// Serve a resource read (idempotent; cacheable under `Exact`).
    ReadResource {
        /// The resource uri.
        uri: String,
    },
    /// Serve a prompt template.
    RenderPrompt {
        /// The prompt name.
        name: String,
    },
}

impl AgentTask {
    /// The `TrackRecord.task` label (the `gov_route` fold key).
    pub fn label(&self) -> &'static str {
        match self {
            AgentTask::InvokeTool { .. } => "agent.invoke_tool",
            AgentTask::ReadResource { .. } => "agent.read_resource",
            AgentTask::RenderPrompt { .. } => "agent.render_prompt",
        }
    }
}

/// One bridged invocation. `invoke_depth` is the number of `(AgentBridge, InvokeAgent)`
/// links in the presented (already-`verify_chain`'d) delegation chain — the F10 depth is
/// cryptographically witnessed, not a mutable counter the agent reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInvocation {
    /// What to do.
    pub task: AgentTask,
    /// Estimated cost in budget units (pre-acquired before the call).
    pub cost_units: u64,
    /// Cryptographically-witnessed invocation depth (count of `InvokeAgent` links).
    pub invoke_depth: u8,
}

/// A bridged-invocation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponse {
    /// Opaque response bytes.
    pub content: Vec<u8>,
    /// Budget units actually consumed (what the `TrackRecord` row prices).
    pub units: u64,
}

/// The pluggable agent-bridge port. Implemented by `McpServerBridge` in the
/// `agent-adapters` crate. Mirrors `LlmBackend` (`id/caps/…/health`).
pub trait AgentBridge {
    /// Stable bridge id, e.g. `"mcp:<server-id>"`. Used in cache keys + telemetry rows.
    fn id(&self) -> &str;
    /// Fail-closed capability discovery.
    fn caps(&self) -> AgentCaps;
    /// The admitted manifest this bridge was constructed from.
    fn manifest(&self) -> &AgentManifest;
    /// Invoke the bridged agent. `Err` on any failure — never a mock response.
    fn invoke(&self, req: &AgentInvocation) -> Result<AgentResponse, AgentError>;
    /// Typed health probe. `Ok(())` iff the bridge is reachable.
    fn health(&self) -> Result<(), AgentError>;
}
