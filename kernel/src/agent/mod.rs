//! agent — the bounded, fail-closed AgentLoop executor (WAVE P40).
//!
//! The loop drives a pluggable reasoning seam (`AgentReasoner`) through a bounded
//! plan→act→observe cycle. Every tool call is routed through the kernel's EXISTING
//! capability firewall — [`crate::ports::tool::ToolPort`] / [`crate::ports::tool::SkillRegistry`]
//! (the closed tool authority, writes UNREPRESENTABLE) and [`crate::ports::mcp::McpPort`]
//! (the capability-scoped, verify_chain-gated tool boundary). The loop does NOT
//! invent a second tool registry: tool discovery/activation and the capability gate
//! are reused verbatim from those ports.
//!
//! # Fail-closed contract (the three hard branches)
//! 1. **No tool runs without a verified capability.** The capability grant is a
//!    [`crate::ports::mcp::GrantSet`] already derived (in production) from
//!    `verify_chain`; the loop hands every tool call to `McpPort::call_tool`, which
//!    returns [`crate::ports::mcp::McpServeError::ScopeDenied`] BEFORE the tool body
//!    executes when the grant does not cover the tool's declared scope. There is no
//!    code path that invokes a `ToolPort` outside that gate.
//! 2. **Unknown tool rejected.** A tool name not present in the registry resolves to
//!    `None` and yields [`crate::ports::mcp::McpServeError::UnknownTool`] — never a
//!    default tool, never a fuzzy match. Logged as a malformed call, never a crash.
//! 3. **The loop terminates on budget exhaustion.** Each iteration debits a
//!    [`crate::token_bucket::TokenBucket`] (the kernel's degrade-closed budget
//!    primitive, reused — not re-invented). When the bucket lacks the unit, the loop
//!    returns [`LoopOutcome::AssistantUnavailable`] with the partial log attached.
//!    Together with [`MAX_AGENT_ITERATIONS`] this bounds worst-case wall time to a
//!    closed form (no supervisor, no silent retry-forever — repetition is only
//!    model-driven and capped).
//!
//! The kernel build stays serde/network-free: this module imports only `ports::tool`,
//! `ports::mcp`, and `token_bucket` — all plain-value, zero-dep.

#[path = "loop.rs"]
pub mod r#loop;
pub mod model_pair;
pub mod model_registry;
