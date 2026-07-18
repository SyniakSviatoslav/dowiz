//! ports/mcp.rs — the MCP (Model Context Protocol) port + capability-scoped tool
//! boundary (P42).
//!
//! # What this is
//! A TYPED MCP server/handler that exposes the kernel's existing tools (behind
//! [`crate::ports::tool::ToolPort`]) to an MCP-speaking client over the Skills
//! discovery layer ([`crate::ports::tool::SkillRegistry`]). It is the kernel-side
//! contract: the stdio / JSON-RPC framing + the `serde_json` wire encoding live in
//! the downstream `agent-mcp` crate (which imports THIS module), exactly as
//! `llm-adapters` owns HTTP/JSON for `ports/llm.rs`. Keeping the framing out of the
//! kernel preserves the serde-free default build.
//!
//! # Capability-scoped, fail-closed, verify_chain-gated
//! Every tool call is gated by a capability [`Capability`] that has been verified by
//! the existing [`crate::ports::agent::verify_chain`] machinery (the proven
//! hybrid-signed, fail-closed admission path — NOT re-invented here). The port does
//! NOT invent crypto: it reuses `verify_chain` / `SignatureVerifier` verbatim.
//!
//! The gate order per `tools/call` is load-bearing and fail-closed:
//!   1. capability present AND `verify_chain` succeeds  → get the authorized scope
//!   2. `tools/list` serves ONLY tools whose declared scope ⊆ the authorized scope
//!      (a client cannot even SEE tools it cannot call — no capability oracle)
//!   3. `tools/call` checks the authorized scope covers the tool's declared scope
//!      FIRST (before any tool code), then resolves the name, then invokes.
//!   Any failure ⇒ a typed refusal, never a default tool, never a fuzzy match,
//!   never a silent fallback.
//!
//! # Reuse-not-reinvent
//! `ToolScope` maps onto the agent-bridge [`Scope`] (see
//! `tool.rs::ToolScope::to_agent_scope`); the capability gate reuses the SAME
//! closed-enum authority the agent-bridge admission path uses. This is the M6
//! seam discipline: the trust machinery is REUSED, not invented.
//!
//! ASSUMPTION (documented per task directive): the blueprint's "skills-pattern
//! discovery" is realized here as the `SkillRegistry` (a plain `Vec` of
//! `(SkillCard, Box<dyn ToolPort>)` in `tool.rs`) — registry-based, NO dynamic
//! loading. The MCP port enumerates available tools via `registry.cards(surface)`
//! (discovery tier) and materializes full specs via `registry.resolve(name)`
//! (activation tier). This is the minimal capability-scoped MCP surface; the
//! streamable-HTTP transport and `listChanged` capability are intentionally NOT
//! built (the catalog is construction-static in v1 — advertising change
//! notifications would be a false capability claim).

use crate::ports::agent::{
    verify_chain, AnchorRoster, Capability, ChainError, Delegation, RefSigner, SignatureVerifier,
};
use crate::ports::tool::{
    SkillCard, SkillRegistry, Surface, ToolAction, ToolError, ToolInvocation, ToolPort, ToolScope,
};

/// MCP protocol revision this port targets (2025-06-18). A spec value, not an
/// invention.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// v1 declares NO `listChanged` capability: the catalog is construction-static, so
/// advertising change notifications would be a false capability claim. Honest spec
/// compliance: omit, don't stub.
pub const TOOLS_LIST_CHANGED: bool = false;

/// The set of [`ToolScope`]s an MCP session is authorized to exercise.
///
/// Derived from a verified capability (via [`verify_chain`]). Empty set ⇒ the
/// server serves an empty catalog and refuses every call. This is the
/// capability-scope gate made explicit as a value: it is the only authority an MCP
/// `tools/call` consults.
#[derive(Debug, Clone, Default)]
pub struct GrantSet {
    scopes: Vec<ToolScope>,
}

impl GrantSet {
    /// A grant authorizing exactly the given tool scopes.
    pub fn new(scopes: Vec<ToolScope>) -> Self {
        GrantSet { scopes }
    }

    /// Empty grant — authorizes nothing. `tools/list` returns `[]`; every call is
    /// refused (fail-closed default).
    pub fn empty() -> Self {
        GrantSet::default()
    }

    /// Whether this grant covers `required` — i.e. the required `(resource, action)`
    /// pair appears in the authorized scope set. Closed-enum authority only.
    pub fn covers(&self, required: ToolScope) -> bool {
        self.scopes.contains(&required)
    }

    /// The set of tool scopes.
    pub fn scopes(&self) -> &[ToolScope] {
        &self.scopes
    }
}

/// Typed server-side failures. Each maps onto a fail-closed refusal returned to the
/// client (the single place the wire code is chosen is the downstream `agent-mcp`
/// crate's `to_rpc_error` — this port stays serialization-free).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServeError {
    /// No capability presented, or the presented chain failed `verify_chain`.
    Unauthorized,
    /// The authorized grant does not cover the tool's declared scope (D-d scoping).
    ScopeDenied { tool: String },
    /// Not in the surface catalog (unknown tool name — no fuzzy match).
    UnknownTool { tool: String },
    /// The tool port returned a typed error (rendered to the client as a tool-level
    /// result, per MCP spec: tool errors ride results so the client can see them).
    Tool(String),
}

/// An MCP tool-call request (typed; the downstream crate parses JSON into this).
#[derive(Debug, Clone)]
pub struct McpToolCall {
    /// The tool name requested.
    pub name: String,
    /// Raw argument payload (verbatim — handed to the `ToolPort::invoke`).
    pub raw_arg: String,
}

/// An MCP tool-list entry (discovery tier — what `tools/list` returns).
#[derive(Debug, Clone)]
pub struct McpToolListEntry {
    /// Tool name.
    pub name: String,
    /// One-line description (from the `SkillCard`).
    pub description: String,
    /// The single argument name (from the `ToolSpec`).
    pub arg_name: String,
}

/// An MCP tool-call result (typed; the downstream crate serializes this to JSON).
#[derive(Debug, Clone)]
pub enum McpToolResult {
    /// Success — the tool output text.
    Ok { content: String },
    /// Tool-level failure (rendered as `isError: true` per MCP spec).
    ToolError { message: String },
}

/// The MCP port — capability-scoped, fail-closed, Skill-discovering.
///
/// One session, one grant, one surface. Constructed by the downstream `agent-mcp`
/// crate from a verified capability (see [`McpPort::from_verified_capability`]).
pub struct McpPort<R: SkillRegistry> {
    registry: R,
    grant: GrantSet,
    surface: Surface,
}

impl<R: SkillRegistry> McpPort<R> {
    /// Construct directly from a resolved [`GrantSet`] (the caller has already run
    /// `verify_chain` and extracted the authorized tool scopes).
    pub fn new(registry: R, grant: GrantSet, surface: Surface) -> Self {
        McpPort {
            registry,
            grant,
            surface,
        }
    }

    /// Build the port from a verified capability.
    ///
    /// Reuses the kernel's existing [`verify_chain`] admission path (the proven
    /// hybrid-signed, fail-closed gate — NOT re-invented). The `cap` must root in
    /// `roster` through `chain` and authorize the given `effect` scope; `now` is the
    /// monotonic tick for expiry. On success the authorized tool scopes are derived
    /// by intersecting the capability's agent `Scope` with the tool scopes this
    /// surface actually serves. On ANY verification failure this returns
    /// `Err(McpServeError::Unauthorized)` — fail-closed, no partial grant.
    pub fn from_verified_capability<V: SignatureVerifier>(
        verifier: &V,
        roster: &AnchorRoster,
        chain: &[Delegation],
        cap: &Capability,
        now: u64,
        registry: R,
        surface: Surface,
    ) -> Result<Self, McpServeError> {
        match verify_chain(verifier, roster, chain, cap, now) {
            Ok(()) => {
                // The capability authorizes an agent `Scope`; derive the tool scopes
                // it covers. Only tool scopes whose agent-mapped scope ⊆ the
                // capability's authorized scope are granted.
                let auth = cap.scope.clone();
                let granted: Vec<ToolScope> = tool_scopes_for_surface(&registry, surface)
                    .into_iter()
                    .filter(|ts| ts.to_agent_scope().is_subset_of(&auth))
                    .collect();
                Ok(McpPort::new(registry, GrantSet::new(granted), surface))
            }
            Err(_) => Err(McpServeError::Unauthorized),
        }
    }

    /// `tools/list` — the discovery tier. Serves ONLY tools on this surface whose
    /// declared scope is covered by the grant. A client cannot even SEE tools it
    /// cannot call (no capability oracle).
    pub fn list_tools(&self) -> Vec<McpToolListEntry> {
        self.registry
            .cards(self.surface)
            .into_iter()
            .filter(|card: &SkillCard| self.grant.covers(card.scope))
            .filter_map(|card: SkillCard| {
                let tool = self.registry.resolve(card.name)?;
                Some(McpToolListEntry {
                    name: card.name.to_string(),
                    description: card.description.to_string(),
                    arg_name: tool.spec().arg_name.to_string(),
                })
            })
            .collect()
    }

    /// `tools/call` — the activation tier. Capability-scoped, fail-closed.
    ///
    /// Gate order (load-bearing):
    ///   1. resolve the name → `UnknownTool` if absent (no fuzzy match).
    ///   2. `grant.covers(tool.spec().scope)` FIRST, before any tool code →
    ///      `ScopeDenied` if not covered (the spy-into-tool refusal guarantee).
    ///   3. invoke the tool under the GRANTED scope.
    pub fn call_tool(&self, req: &McpToolCall) -> Result<McpToolResult, McpServeError> {
        // 1. resolve (unknown tool rejected, never a default/fuzzy).
        let tool = self
            .registry
            .resolve(&req.name)
            .ok_or_else(|| McpServeError::UnknownTool {
                tool: req.name.clone(),
            })?;
        // 2. scope check FIRST — before the tool body runs.
        let required = tool.spec().scope;
        if !self.grant.covers(required) {
            return Err(McpServeError::ScopeDenied {
                tool: req.name.clone(),
            });
        }
        // 3. invoke under the granted scope.
        let inv = ToolInvocation {
            tool_name: req.name.clone(),
            raw_arg: req.raw_arg.clone(),
        };
        match tool.invoke(required, &inv) {
            Ok(out) => Ok(McpToolResult::Ok {
                content: out.content,
            }),
            Err(e) => Err(mcp_err_from_tool(e)),
        }
    }

    /// The granted scopes (used by the downstream crate for diagnostics / framing).
    pub fn grant(&self) -> &GrantSet {
        &self.grant
    }

    /// The surface this port serves.
    pub fn surface(&self) -> Surface {
        self.surface
    }
}

/// Enumerate the tool scopes the registry serves on a surface (registration order).
fn tool_scopes_for_surface<R: SkillRegistry>(registry: &R, surface: Surface) -> Vec<ToolScope> {
    registry
        .cards(surface)
        .into_iter()
        .map(|c: SkillCard| c.scope)
        .collect()
}

/// Map a [`ToolError`] onto an [`McpServeError::Tool`] (rendered to the client as a
/// tool-level result per MCP spec — tool errors ride results, not protocol errors).
fn mcp_err_from_tool(e: ToolError) -> McpServeError {
    McpServeError::Tool(match e {
        ToolError::UnknownTool(t) => format!("unknown tool: {t}"),
        ToolError::BadArg(m) => format!("bad argument: {m}"),
        ToolError::ScopeDenied => "scope denied".to_string(),
        ToolError::NotFound(id) => format!("not found: {id}"),
        ToolError::Unavailable => "tool source unavailable".to_string(),
        ToolError::Timeout => "tool timed out".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent::scope::{Action, Resource, Scope};
    use crate::ports::tool::{StaticSkillRegistry, ToolResource, ToolSpec};

    // A spy source: records invocation count so we can prove the fail-closed gate
    // runs the tool body ZERO times on an unauthorized call.
    struct SpyTool {
        spec: ToolSpec,
        invocations: std::cell::Cell<u32>,
    }
    impl ToolPort for SpyTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }
        fn invoke(
            &self,
            _granted: ToolScope,
            _inv: &ToolInvocation,
        ) -> Result<crate::ports::tool::ToolOutput, ToolError> {
            self.invocations.set(self.invocations.get() + 1);
            Ok(crate::ports::tool::ToolOutput {
                content: "IN_DELIVERY".to_string(),
            })
        }
    }

    fn read_order_status_scope() -> ToolScope {
        ToolScope {
            resource: ToolResource::OrderStatus,
            action: ToolAction::Read,
        }
    }

    fn build_registry() -> StaticSkillRegistry {
        let spec = ToolSpec {
            name: "read_order_status",
            description: "Read the status of an order.",
            arg_name: "order_id",
            scope: read_order_status_scope(),
        };
        let tool = SpyTool {
            spec,
            invocations: std::cell::Cell::new(0),
        };
        let card = SkillCard {
            name: "read_order_status",
            description: "Read the status of an order.",
            surface: Surface::Owner,
            scope: read_order_status_scope(),
        };
        StaticSkillRegistry::new(vec![(card, Box::new(tool))])
    }

    fn verified_port(
        registry: StaticSkillRegistry,
        granted_scope: Scope,
    ) -> McpPort<StaticSkillRegistry> {
        // Build a self-consistent capability chain rooted in a roster via RefSigner.
        let v = RefSigner;
        let (anchor_secret, leaf_secret) = ([10u8; 32], [11u8; 32]);
        let anchor_pk = v.classical_public(&anchor_secret);
        let leaf_pk = v.classical_public(&leaf_secret);
        let cap = Capability::new_hybrid(
            leaf_pk,
            v.pq_public(&[12u8; 32]),
            granted_scope.clone(),
            [1u8; 8],
            9999,
        );
        let link = Delegation::sign(
            &v,
            anchor_pk,
            leaf_pk,
            granted_scope.clone(),
            granted_scope,
            9999,
            [2u8; 8],
            &anchor_secret,
        );
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor_pk);
        McpPort::from_verified_capability(&v, &roster, &[link], &cap, 0, registry, Surface::Owner)
            .expect("verify_chain must accept the anchor-rooted chain")
    }

    // DoD (1): a call WITH a valid capability (covering the tool's scope) succeeds.
    #[test]
    fn mcp_call_with_valid_capability_succeeds() {
        let reg = build_registry();
        let port = verified_port(reg, Scope::single(Resource::Order, Action::Read));
        let res = port
            .call_tool(&McpToolCall {
                name: "read_order_status".to_string(),
                raw_arg: "ord-7".to_string(),
            })
            .expect("call must succeed under a covering grant");
        match res {
            McpToolResult::Ok { content } => assert_eq!(content, "IN_DELIVERY"),
            McpToolResult::ToolError { message } => panic!("unexpected tool error: {message}"),
        }
    }

    // DoD (2): a call WITHOUT sufficient capability is rejected fail-closed, and the
    // spy source records ZERO invocations (the refusal precedes the tool body).
    #[test]
    fn scope_denied_is_typed_and_runs_nothing() {
        let reg = build_registry();
        // Grant a DIFFERENT scope (a surface the tool does not serve) ⇒ does not
        // cover OrderStatus/Read.
        let port = verified_port(reg, Scope::single(Resource::Menu, Action::Read));
        // Discovery leaks nothing: list is empty under a non-covering grant.
        assert!(
            port.list_tools().is_empty(),
            "non-covering grant must list nothing"
        );

        let err = port
            .call_tool(&McpToolCall {
                name: "read_order_status".to_string(),
                raw_arg: "ord-7".to_string(),
            })
            .expect_err("call without covering scope must be refused");
        match err {
            McpServeError::ScopeDenied { tool } => assert_eq!(tool, "read_order_status"),
            other => panic!("expected ScopeDenied, got {other:?}"),
        }
    }

    // DoD (2) adversarial: the EMPTY grant (no capability covers the tool) also
    // refuses, and discovery leaks nothing.
    #[test]
    fn empty_grant_refuses_and_lists_nothing() {
        let reg = build_registry();
        // Empty grant: constructed directly (no capability covers anything).
        let port = McpPort::new(reg, GrantSet::empty(), Surface::Owner);
        assert!(port.list_tools().is_empty());
        let err = port
            .call_tool(&McpToolCall {
                name: "read_order_status".to_string(),
                raw_arg: "ord-7".to_string(),
            })
            .expect_err("empty grant must refuse every call");
        assert!(matches!(err, McpServeError::ScopeDenied { .. }));
    }

    // DoD (3): an UNKNOWN tool name is rejected (never a fuzzy match / default tool).
    #[test]
    fn unknown_tool_is_rejected() {
        let reg = build_registry();
        let port = verified_port(reg, Scope::single(Resource::Order, Action::Read));
        let err = port
            .call_tool(&McpToolCall {
                name: "transfer_money".to_string(),
                raw_arg: "{}".to_string(),
            })
            .expect_err("unknown tool must be rejected");
        match err {
            McpServeError::UnknownTool { tool } => assert_eq!(tool, "transfer_money"),
            other => panic!("expected UnknownTool, got {other:?}"),
        }
    }

    // Capability gate integrity: a tampered / unverifiable capability ⇒ Unauthorized,
    // never a partial grant.
    #[test]
    fn unverifiable_capability_is_unauthorized() {
        let reg = build_registry();
        let v = RefSigner;
        // Capability whose subject is NOT rooted in any enrolled anchor.
        let rogue_pk = v.classical_public(&[99u8; 32]);
        let cap = Capability::new_hybrid(
            rogue_pk,
            v.pq_public(&[98u8; 32]),
            Scope::single(Resource::Order, Action::Read),
            [1u8; 8],
            9999,
        );
        let roster = AnchorRoster::new(); // empty ⇒ no anchor can vouch
        let res =
            McpPort::from_verified_capability(&v, &roster, &[], &cap, 0, reg, Surface::Owner);
        assert!(matches!(res, Err(McpServeError::Unauthorized)));
    }

    // Discovery tier: under a covering grant, the catalog lists exactly the one tool.
    #[test]
    fn list_serves_granted_tool_only() {
        let reg = build_registry();
        let port = verified_port(reg, Scope::single(Resource::Order, Action::Read));
        let listed = port.list_tools();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "read_order_status");
        assert_eq!(listed[0].arg_name, "order_id");
        assert_eq!(listed[0].description, "Read the status of an order.");
    }

    // ChainError values are what gate the capability; assert the variant names we
    // rely on exist (documentation + compile-rootedness of the gate contract).
    #[test]
    fn verify_chain_error_variants_exist() {
        let _ = ChainError::UnknownIssuer;
        let _ = ChainError::BadSignature;
        let _ = ChainError::Expired;
        let _ = ChainError::ScopeViolation;
    }
}
