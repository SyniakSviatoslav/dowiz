//! ports/tool.rs — the `ToolPort` firewall (P40's tool seam, defined HERE so the
//! parallel P40 `agent-loop` worktree and P42's `mcp` port can both reuse it) plus
//! the Skills-pattern discovery layer (P42 — `SkillCard` / `SkillRegistry` /
//! `StaticSkillRegistry`).
//!
//! # Compile firewall (mirrors `ports/llm.rs:3-7`)
//! ZERO network / HTTP / JSON / serde. This module defines ABSTRACT contracts
//! (traits + plain value types) only. The concrete tool impl + the JSON framing
//! live in downstream crates (`agent-facade` for the impl, the `agent-mcp` crate
//! for the stdio/JSON-RPC framing that drives [`crate::ports::mcp`]). A native rlib
//! consumer of the kernel pulls NONE of serde / serde_json.
//!
//! # Why this lives in the kernel ports layer, not a facade crate
//! The `ToolPort` trait is the closed authority the Agent-Skills boundary hangs off
//! of: writes are UNREPRESENTABLE because `ToolAction` has exactly one variant
//! (`Read`). P40's loop and P42's MCP server are TWO consumers of one contract, so
//! the trait must live where both reach it without importing each other — the ports
//! layer. Defining it here (instead of inside the `agent-loop` worktree) is the
//! file-ownership coordination for P40/P42 parallel execution: P42 owns `tool.rs` +
//! `mcp.rs`; P40 owns `agent/loop.rs` and imports `ToolPort` from here.

/// Closed resource enum. A tool target not listed here is UNREPRESENTABLE.
/// P40/P42 ship exactly one variant. Money/auth/RLS/migration resources are never
/// added (red-line — see `mcp.rs`'s reachability argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolResource {
    /// Read the lifecycle status of an order (the P40/P42 tool).
    OrderStatus,
}

/// Closed action enum. `Read` is the ONLY variant in P40/P42 — a mutating tool
/// invocation is not policy-forbidden, it is type-unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolAction {
    /// Read-only access to the tool's backing source.
    Read,
}

/// The capability scope a tool invocation executes under. Granted by the
/// composition layer (agent-facade / agent-mcp), checked fail-closed by the port
/// impl: the granted scope must cover the tool's declared scope or the invocation
/// is refused BEFORE the tool body runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolScope {
    /// What the tool targets.
    pub resource: ToolResource,
    /// What the tool does with it.
    pub action: ToolAction,
}

impl ToolScope {
    /// Map this kernel tool-scope onto the agent-bridge capability [`Scope`]
    /// (P40/P42 reuse the proven closed-enum authority from `ports::agent` for the
    /// verify_chain capability gate — we do NOT invent a second crypto/scope system).
    ///
    /// `ToolResource::OrderStatus` + `ToolAction::Read` ⇒ read access to the
    /// `Order` resource. A future tool that targets a RED-LINE resource would map
    /// onto a red-line `(Resource, Action)` pair and would be denied by the
    /// capability gate's red-line policy unless operator-allow-listed.
    pub fn to_agent_scope(&self) -> crate::ports::agent::Scope {
        use crate::ports::agent::{Action, Resource, Scope};
        match (self.resource, self.action) {
            (ToolResource::OrderStatus, ToolAction::Read) => {
                Scope::single(Resource::Order, Action::Read)
            }
        }
    }
}

/// Static declaration of one tool — what the model is told, verbatim.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    /// "`read_order_status`".
    pub name: &'static str,
    /// Natural-language contract handed to the model.
    pub description: &'static str,
    /// "`order_id`" — the single string argument the tool accepts.
    pub arg_name: &'static str,
    /// Declared capability requirement, checked against the grant.
    pub scope: ToolScope,
}

/// One parsed tool invocation (the model's ask, post-parse, pre-execution).
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// The tool name (resolved by the dispatcher).
    pub tool_name: String,
    /// The raw argument payload from the model, verbatim. The PORT IMPL parses it;
    /// the loop / MCP server never do.
    pub raw_arg: String,
}

/// Tool output — plain text handed back as the observation.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// The result text (e.g. an order status).
    pub content: String,
}

/// Typed tool failure. Every variant is a loop OBSERVATION or OUTCOME — never a
/// panic, never a silent retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Model asked for a tool that doesn't exist (no fuzzy match).
    UnknownTool(String),
    /// Arguments unparseable / missing the required arg.
    BadArg(String),
    /// Granted scope does not cover the tool's declared scope.
    ScopeDenied,
    /// Order id valid in form, absent in the source.
    NotFound(String),
    /// The tool's backing source is down (fail-closed; never a stub fallback).
    Unavailable,
    /// Tool execution exceeded the timeout.
    Timeout,
}

/// The tool port. Implemented in `agent-facade` (downstream crate); consumed by the
/// agent loop as `&dyn ToolPort` (P40) AND by the MCP server (P42) as `&dyn
/// ToolPort` — one door, one contract.
pub trait ToolPort {
    /// The static declaration (name, description, arg, declared scope).
    fn spec(&self) -> &ToolSpec;
    /// Invoke the tool under the `granted` scope. The impl MUST fail-closed (return
    /// [`ToolError::ScopeDenied]) when `granted` does not cover `spec().scope`.
    fn invoke(&self, granted: ToolScope, inv: &ToolInvocation) -> Result<ToolOutput, ToolError>;
}

// ───────────────────────────── Skills-pattern discovery (P42) ──────────────────
//
// The discoverability layer that lets the tool catalog grow WITHOUT growing every
// prompt: discovery (cards, cheap, always in context) is separated from activation
// (the full ToolSpec, materialized only for selected tools). Cards are what a
// context carries by default (~1 line each); the full spec is requested by name.

/// Which product surface a tool serves. Closed enum — the deterministic discovery
/// pre-filter (tier 3). A tool with no surface is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    /// The venue owner's operator tooling.
    Owner,
    /// Courier-facing tooling.
    Courier,
    /// Customer-facing tooling.
    Customer,
    /// Ops / observability tooling.
    Ops,
}

/// Discovery projection of a tool — the Skills-pattern "frontmatter".
/// This is what a context carries for EVERY tool; the full `ToolSpec` is
/// materialized per-request for SELECTED tools only (activation tier).
#[derive(Debug, Clone)]
pub struct SkillCard {
    /// == `ToolSpec.name`, the join key.
    pub name: &'static str,
    /// ≤ `MAX_CARD_DESCRIPTION_BYTES` — enforced at registry construction.
    pub description: &'static str,
    /// Which product surface this tool serves (deterministic pre-filter).
    pub surface: Surface,
    /// Declared capability requirement (reused verbatim from `ToolSpec`).
    pub scope: ToolScope,
}

/// The two-tier registry. Implemented (instantiated) by `agent-facade`; consumed by
/// the loop (P40) AND the MCP server (P42) — both through this trait, ONE catalog.
pub trait SkillRegistry {
    /// Discovery tier: cheap, surface-filtered. Order is stable (registration order).
    fn cards(&self, surface: Surface) -> Vec<SkillCard>;
    /// Activation tier: full tool, by card name. `None` = never registered (a card
    /// without a resolvable tool is a registry-construction error, caught at build).
    fn resolve(&self, name: &str) -> Option<&dyn ToolPort>;
}

/// Discovery-tier size cap. A description that cannot fit one line is doing
/// activation-tier work in the discovery tier — refused at registration.
pub const MAX_CARD_DESCRIPTION_BYTES: usize = 200;

/// Named growth trigger (NOT built now): when the surface-filtered card count first
/// exceeds this, a relevance-selection pass between tiers becomes a reviewed
/// follow-up design. Below it, all surface cards resolve (the contract holds).
pub const CARD_ROUTER_THRESHOLD: usize = 12;

/// Per-request ceiling on ACTIVATED tools (full specs). The context-economics
/// invariant: prompt tool-cost is O(min(N, this)), not O(N).
pub const MAX_ACTIVE_TOOLS: usize = 4;

/// The static catalog: `(SkillCard, tool)` pairs fixed at construction.
///
/// Construction PANICS on a malformed catalog (duplicate name, card/spec name
/// mismatch, description over the byte cap, card scope ≠ spec scope, or a card whose
/// tool cannot be resolved) — a malformed catalog is unrepresentable at runtime;
/// the panic surfaces at composition time, never mid-request (the Self-Termination
/// leg: unrepresentable-malformed-catalog).
pub struct StaticSkillRegistry {
    entries: Vec<(SkillCard, Box<dyn ToolPort>)>,
}

impl StaticSkillRegistry {
    /// Build the catalog. PANICS on any malformed entry (see struct doc).
    pub fn new(entries: Vec<(SkillCard, Box<dyn ToolPort>)>) -> Self {
        // Uniqueness of names.
        let mut seen = std::collections::HashSet::new();
        for (card, tool) in &entries {
            assert!(
                card.description.len() <= MAX_CARD_DESCRIPTION_BYTES,
                "SkillCard '{}' description exceeds MAX_CARD_DESCRIPTION_BYTES ({} > {})",
                card.name,
                card.description.len(),
                MAX_CARD_DESCRIPTION_BYTES
            );
            assert_eq!(
                card.name,
                tool.spec().name,
                "SkillCard name '{}' != ToolSpec name '{}'",
                card.name,
                tool.spec().name
            );
            assert_eq!(
                card.scope,
                tool.spec().scope,
                "SkillCard scope for '{}' != ToolSpec scope",
                card.name
            );
            assert!(
                seen.insert(card.name),
                "duplicate SkillCard name '{}' in StaticSkillRegistry",
                card.name
            );
        }
        StaticSkillRegistry { entries }
    }
}

impl SkillRegistry for StaticSkillRegistry {
    fn cards(&self, surface: Surface) -> Vec<SkillCard> {
        self.entries
            .iter()
            .filter(|(card, _)| card.surface == surface)
            .map(|(card, _)| card.clone())
            .collect()
    }

    fn resolve(&self, name: &str) -> Option<&dyn ToolPort> {
        self.entries
            .iter()
            .find(|(card, _)| card.name == name)
            .map(|(_, tool)| tool.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A toy tool for the registry adversaries.
    struct ToyTool {
        spec: ToolSpec,
    }
    impl ToolPort for ToyTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }
        fn invoke(
            &self,
            _granted: ToolScope,
            _inv: &ToolInvocation,
        ) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                content: "ok".to_string(),
            })
        }
    }

    fn card(
        name: &'static str,
        desc: &'static str,
        surface: Surface,
        scope: ToolScope,
    ) -> SkillCard {
        SkillCard {
            name,
            description: desc,
            surface,
            scope,
        }
    }

    fn order_status_scope() -> ToolScope {
        ToolScope {
            resource: ToolResource::OrderStatus,
            action: ToolAction::Read,
        }
    }

    #[test]
    fn registry_serves_one_card() {
        let spec = ToolSpec {
            name: "read_order_status",
            description: "Read the status of an order.",
            arg_name: "order_id",
            scope: order_status_scope(),
        };
        let tool = ToyTool { spec };
        let c = card(
            "read_order_status",
            "Read the status of an order.",
            Surface::Owner,
            order_status_scope(),
        );
        let reg = StaticSkillRegistry::new(vec![(c, Box::new(tool))]);
        let cards = reg.cards(Surface::Owner);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].name, "read_order_status");
        assert_eq!(cards[0].description, "Read the status of an order.");
        assert_eq!(cards[0].surface, Surface::Owner);
        assert!(reg.resolve("read_order_status").is_some());
    }

    #[test]
    #[should_panic(expected = "duplicate SkillCard name")]
    fn dup_name_panics() {
        let make = || ToyTool {
            spec: ToolSpec {
                name: "t",
                description: "d",
                arg_name: "a",
                scope: order_status_scope(),
            },
        };
        let c = card("t", "d", Surface::Owner, order_status_scope());
        let _ =
            StaticSkillRegistry::new(vec![(c.clone(), Box::new(make())), (c, Box::new(make()))]);
    }

    #[test]
    #[should_panic(expected = "exceeds MAX_CARD_DESCRIPTION_BYTES")]
    fn oversize_description_panics() {
        let tool = ToyTool {
            spec: ToolSpec {
                name: "t",
                description: "d",
                arg_name: "a",
                scope: order_status_scope(),
            },
        };
        // 201-byte description.
        let big: String = std::iter::repeat('x').take(201).collect();
        let c = card(
            "t",
            Box::leak(big.into_boxed_str()),
            Surface::Owner,
            order_status_scope(),
        );
        let _ = StaticSkillRegistry::new(vec![(c, Box::new(tool))]);
    }

    #[test]
    fn resolve_unknown_is_none() {
        let tool = ToyTool {
            spec: ToolSpec {
                name: "read_order_status",
                description: "d",
                arg_name: "a",
                scope: order_status_scope(),
            },
        };
        let c = card(
            "read_order_status",
            "d",
            Surface::Owner,
            order_status_scope(),
        );
        let reg = StaticSkillRegistry::new(vec![(c, Box::new(tool))]);
        // Never a default tool, never a fuzzy match.
        assert!(reg.resolve("transfer_money").is_none());
    }

    #[test]
    fn tool_scope_maps_to_agent_order_read() {
        let scope = order_status_scope();
        let a = scope.to_agent_scope();
        assert!(a.grants.contains(&(
            crate::ports::agent::Resource::Order,
            crate::ports::agent::Action::Read
        )));
    }
}
