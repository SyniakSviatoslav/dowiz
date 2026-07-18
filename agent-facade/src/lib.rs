//! agent-facade — P40's compilation firewall + the one concrete tool.
//!
//! This is the ONLY crate in the agent lane that imports `dowiz_kernel`. It
//! re-exports exactly the two kernel port surfaces (`llm` + `tool`) and NOTHING
//! else of the kernel — no domain, no money, no `order_machine` in its public
//! API. Downstream `agent-loop` imports `agent-facade` and nothing kernel-shaped,
//! so `dowiz-kernel` appears in its graph only at depth 2, only via this crate
//! (the audited chokepoint — see `agent-loop`'s firewall test).
//!
//! # Reachability guarantee
//! A model's output can only select among `ToolPort` invocations; it cannot name
//! `decide`, `fold`, `apply_tax`, or any store. That is proven by `cargo tree`
//! + grep (committed as a test in `agent-loop`), not promised.

// Re-export the two port surfaces verbatim. Nothing else from the kernel leaks.
pub use dowiz_kernel::ports::llm::*;
pub use dowiz_kernel::ports::tool::*;

use std::collections::BTreeMap;

/// The P37 seam (soft dependency, named not built): where order status comes from.
/// Implemented by `FixtureOrders` now and (later, P37 landed) `HttpOrderStatusSource`
/// — same trait, inherits P37's capability-cert auth (NOT P40's job to build).
pub trait OrderStatusSource {
    /// Returns the canonical oracle string form (e.g. "PENDING" … "COMPENSATED_REFUND",
    /// the `order_machine.rs` vocabulary) or a typed error.
    fn status_of(&self, order_id: &str) -> Result<String, ToolError>;
}

/// Stub source for P40's DoD: a fixed map, solo-offline by construction.
/// LATER (P37 landed, separate PR): HttpOrderStatusSource { base_url }.
#[derive(Debug, Clone, Default)]
pub struct FixtureOrders(BTreeMap<String, String>);

impl FixtureOrders {
    /// Build from a list of `(order_id, canonical_status_string)` pairs.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        FixtureOrders(
            pairs
                .iter()
                .map(|(id, st)| (id.to_string(), st.to_string()))
                .collect(),
        )
    }
}

impl OrderStatusSource for FixtureOrders {
    fn status_of(&self, order_id: &str) -> Result<String, ToolError> {
        self.0
            .get(order_id)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(order_id.to_string()))
    }
}

/// The one tool. Wraps any `OrderStatusSource`; enforces scope + arg parsing.
/// `invoke` is fail-closed: scope-cover check runs BEFORE the source is touched
/// (proven by the adversarial test asserting zero source calls on `ScopeDenied`).
pub struct ReadOrderStatusTool<S: OrderStatusSource> {
    source: S,
}

impl<S: OrderStatusSource> ReadOrderStatusTool<S> {
    /// The static declaration handed to the model (verbatim contract).
    pub const SPEC: ToolSpec = ToolSpec {
        name: "read_order_status",
        description: "Read the lifecycle status of a delivery order by its id. \
                      Returns the canonical status vocabulary (e.g. PENDING, IN_DELIVERY, COMPENSATED_REFUND).",
        arg_name: "order_id",
        scope: ToolScope {
            resource: ToolResource::OrderStatus,
            action: ToolAction::Read,
        },
    };

    pub fn new(source: S) -> Self {
        ReadOrderStatusTool { source }
    }

    /// Parse `raw_arg` (the model's JSON, verbatim) into an `order_id` string.
    /// Never panics: malformed JSON / missing key ⇒ `ToolError::BadArg`.
    fn parse_arg(raw_arg: &str) -> Result<String, ToolError> {
        let v: serde_json::Value =
            serde_json::from_str(raw_arg).map_err(|_| ToolError::BadArg(raw_arg.to_string()))?;
        v.get("order_id")
            .and_then(|o| o.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ToolError::BadArg(raw_arg.to_string()))
    }
}

impl<S: OrderStatusSource> ToolPort for ReadOrderStatusTool<S> {
    fn spec(&self) -> &ToolSpec {
        &Self::SPEC
    }

    fn invoke(&self, granted: ToolScope, inv: &ToolInvocation) -> Result<ToolOutput, ToolError> {
        // Fail-closed: the granted scope must cover the tool's declared scope.
        // `ToolScope` is a struct of closed enums; coverage is exact-equality here
        // (one resource + one action). No coverage ⇒ refuse BEFORE touching the source.
        if granted != Self::SPEC.scope {
            return Err(ToolError::ScopeDenied);
        }
        let order_id = Self::parse_arg(&inv.raw_arg)?;
        let status = self.source.status_of(&order_id)?;
        Ok(ToolOutput {
            content: format!("order {} status: {}", order_id, status),
        })
    }
}
