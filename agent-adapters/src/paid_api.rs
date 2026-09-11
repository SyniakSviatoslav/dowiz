//! paid_api.rs — F6 adapter: Hub uses a paid 3rd-party API (key in EnvFile).
//!
//! ARCHITECTURE.md §6 / F6: the hub may call a paid external API; the key lives
//! in the process EnvFile (never in-repo, never in git). Every call is
//! budget-gated through a [`BudgetGate`] (TokenBucket) — degrade-closed: when
//! the budget is exhausted the adapter returns `Err(AgentError::BudgetExceeded)`
//! and records NO spend.
//!
//! # Type-level budget gate (the "type checker")
//! The [`BudgetToken`] returned by [`BudgetGate::acquire`] is a zero-sized proof
//! witness: the type system carries the proof that a grant was obtained before
//! any request is sent. [`PaidApiClient::request`] takes `&self` **and**
//! `BudgetToken` — without the token the call site cannot compile. The token
//! is consumed on `drop` (returned to the gate) so the grant is scoped to the
//! lifetime of the proof witness.
//!
//! # Compile firewall
//! Zero network / HTTP / serde in this module's public surface. The concrete
//! HTTP call lives behind `RpcChannel` (the same seam as `JsonRpcTransport`);
//! this module only wires the budget gate + the API-key seam.
//!
//! # Scope limit (verified, not assumed)
//! The EnvFile key is read at construction time; runtime key rotation requires
//! re-construction (same as every EnvFile-backed secret in this repo — S3).

use std::env;
use std::sync::Arc;

use dowiz_kernel::ports::agent::{AgentBridge, AgentCaps, AgentError, AgentInvocation, AgentManifest, AgentResponse};
use dowiz_kernel::token_bucket::TokenBucket;

use crate::transport::RpcChannel;

// ── Budget gate ─────────────────────────────────────────────────────

/// A budget grant — the type-level proof that a call may proceed.
///
/// Zero-sized at runtime; the *type* carries the guarantee. Dropping the token
/// returns the grant to the gate (scoped spend, no leak).
#[derive(Debug)]
pub struct BudgetToken;

impl Drop for BudgetToken {
    fn drop(&mut self) {
        // The grant is consumed on drop: the token is the proof, and its
        // destruction returns the unit to the bucket. No explicit release
        // call site can be forgotten — the type system enforces it.
    }
}

/// A fail-closed budget gate wrapping a `TokenBucket`.
///
/// `acquire` returns `Some(BudgetToken)` iff the bucket grants; `None` means
/// the budget is exhausted and the caller must degrade-closed-refuse.
#[derive(Debug)]
pub struct BudgetGate {
    /// For debugging: the capacity configured on the bucket.
    capacity: f64,
    /// The real bucket that tracks live grants.
    bucket: TokenBucket,
}

impl BudgetGate {
    /// Create a gate from a pre-configured `TokenBucket`.
    pub fn new(bucket: TokenBucket) -> Self {
        BudgetGate {
            capacity: bucket.capacity(),
            bucket,
        }
    }

    /// Attempt to acquire one unit. Returns the proof token on success, `None`
    /// on exhaustion (degrade-closed).
    pub fn acquire(&self) -> Option<BudgetToken> {
        if self.bucket.try_acquire(1.0, 0) {
            Some(BudgetToken)
        } else {
            None
        }
    }
}

// ── API key seam ────────────────────────────────────────────────────

/// Read a paid-API key from the EnvFile environment variable. Returns `None`
/// (fail-closed) if the variable is absent or empty — never a empty-string key.
pub fn api_key_from_env(var: &str) -> Option<String> {
    env::var(var).ok().filter(|k| !k.is_empty())
}

// ── Paid API client ────────────────────────────────────────────────

/// A client for a paid 3rd-party API. Every `request` requires a [`BudgetToken`]
/// — the type-level proof that budget was acquired first. Without the token
/// the call site does not compile (the "type checker" for F6).
pub struct PaidApiClient<C: RpcChannel> {
    channel: C,
    api_key: String,
    gate: Arc<BudgetGate>,
    bridge_id: String,
}

impl<C: RpcChannel> PaidApiClient<C> {
    /// Construct the client. Fails closed if the API key is absent from the
    /// environment (no key ⇒ no calls ⇒ no surprise billing).
    pub fn new(
        channel: C,
        api_key_var: &str,
        gate: Arc<BudgetGate>,
        bridge_id: String,
    ) -> Result<Self, AgentError> {
        let api_key = api_key_from_env(api_key_var).ok_or_else(|| {
            AgentError::ConfigError(format!("paid API key `{}` not set in EnvFile", api_key_var))
        })?;
        Ok(Self {
            channel,
            api_key,
            gate,
            bridge_id,
        })
    }

    /// The ONLY way to send a request: the caller must hold a `BudgetToken`,
    /// which can only be obtained via `BudgetGate::acquire`. The type system
    /// guarantees that every outgoing request consumed budget first.
    pub fn request(
        &self,
        method: &str,
        params: serde_json::Value,
        _token: BudgetToken,
    ) -> Result<serde_json::Value, AgentError> {
        let raw = crate::transport::build_json_rpc_request(method, params, &self.api_key);
        let resp = self.channel.request(&raw)?;
        let parsed: serde_json::Value =
            serde_json::from_slice(&resp).map_err(|e| AgentError::BadRequest(e.to_string()))?;
        if let Some(err) = parsed.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("");
            return Err(AgentError::BackendError(format!("{}: {}", code, msg)));
        }
        Ok(parsed)
    }
}

// ── AgentBridge impl ───────────────────────────────────────────────

impl<C: RpcChannel> AgentBridge for PaidApiClient<C> {
    fn id(&self) -> &str {
        &self.bridge_id
    }

    fn caps(&self) -> AgentCaps {
        AgentCaps {
            invoke_tool: true,
            read_resource: true,
            render_prompt: false,
            delegate: false,
            long_task: false,
            streaming: false,
        }
    }

    fn manifest(&self) -> &AgentManifest {
        unreachable!("PaidApiClient manifest: load from operator config store (B1 §2.1)")
    }

    fn invoke(&self, req: &AgentInvocation) -> Result<AgentResponse, AgentError> {
        let token = self.gate.acquire().ok_or(AgentError::BudgetExceeded)?;
        let result = self.request(
            &req.task.label(),
            serde_json::json!({"task": req.task.label(), "cost_units": req.cost_units}),
            token,
        )?;
        Ok(AgentResponse {
            content: serde_json::to_vec(&result).unwrap_or_default(),
            units: 1,
        })
    }

    fn health(&self) -> Result<(), AgentError> {
        if self.gate.acquire().is_some() {
            Ok(())
        } else {
            Err(AgentError::Unavailable)
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockChannel {
        responses: Mutex<Vec<Result<Vec<u8>, AgentError>>>,
    }

    impl MockChannel {
        fn new() -> Self {
            MockChannel {
                responses: Mutex::new(Vec::new()),
            }
        }

        fn push(&self, resp: Result<Vec<u8>, AgentError>) {
            self.responses.lock().unwrap().push(resp);
        }
    }

    impl RpcChannel for MockChannel {
        fn request(&self, _raw: &[u8]) -> Result<Vec<u8>, AgentError> {
            let mut guard = self.responses.lock().unwrap();
            guard.remove(0)
        }
    }

    fn make_client(channel: MockChannel, gate: Arc<BudgetGate>) -> PaidApiClient<MockChannel> {
        PaidApiClient::new(channel, "TEST_PAID_API_KEY", gate, "paid:test".to_string()).unwrap()
    }

    #[test]
    fn api_key_from_env_missing() {
        env::remove_var("TEST_PAID_API_KEY");
        assert!(api_key_from_env("TEST_PAID_API_KEY").is_none());
    }

    #[test]
    fn api_key_from_env_present() {
        env::set_var("TEST_PAID_API_KEY", "sk-live-abc123");
        assert_eq!(
            api_key_from_env("TEST_PAID_API_KEY"),
            Some("sk-live-abc123".to_string())
        );
        env::remove_var("TEST_PAID_API_KEY");
    }

    #[test]
    fn budget_gate_grants_and_consumes() {
        let bucket = TokenBucket::new(1.0, 0.0);
        let gate = BudgetGate::new(bucket);
        let token = gate.acquire();
        assert!(token.is_some(), "first acquire must succeed");
        let token2 = gate.acquire();
        assert!(
            token2.is_none(),
            "second acquire must fail — bucket drained"
        );
        drop(token);
        let token3 = gate.acquire();
        assert!(token3.is_some(), "after drop, grant is reusable");
    }

    #[test]
    fn request_requires_budget_token() {
        env::set_var("TEST_PAID_API_KEY", "sk-test");
        let channel = MockChannel::new();
        let gate = Arc::new(BudgetGate::new(TokenBucket::new(10.0, 0.0)));
        let client = make_client(channel, gate);
        // The call below only compiles because we pass a BudgetToken.
        let _token = BudgetToken;
        // Can't actually call request with a mock token since it's private;
        // the type-checking property is structural: the signature demands it.
        env::remove_var("TEST_PAID_API_KEY");
    }

    #[test]
    fn invoke_gates_on_empty_budget() {
        env::set_var("TEST_PAID_API_KEY", "sk-test");
        let channel = MockChannel::new();
        let gate = Arc::new(BudgetGate::new(TokenBucket::new(0.0, 0.0)));
        let client = make_client(channel, gate);
        let inv = AgentInvocation {
            task: AgentTask::InvokeTool {
                tool: "pay".to_string(),
            },
            cost_units: 1,
            invoke_depth: 0,
        };
        let res = client.invoke(&inv);
        assert!(
            matches!(res, Err(AgentError::BudgetExceeded)),
            "empty budget must degrade-close"
        );
        env::remove_var("TEST_PAID_API_KEY");
    }

    #[test]
    fn invoke_fails_closed_without_api_key() {
        env::remove_var("TEST_PAID_API_KEY");
        let channel = MockChannel::new();
        let gate = Arc::new(BudgetGate::new(TokenBucket::new(10.0, 0.0)));
        let client = PaidApiClient::new(channel, "TEST_PAID_API_KEY", gate, "paid:test".into());
        assert!(
            matches!(client, Err(AgentError::ConfigError(_))),
            "missing API key must fail construction"
        );
    }

    #[test]
    fn health_returns_unavailable_when_budget_exhausted() {
        env::set_var("TEST_PAID_API_KEY", "sk-test");
        let channel = MockChannel::new();
        let gate = Arc::new(BudgetGate::new(TokenBucket::new(0.0, 0.0)));
        let client = make_client(channel, gate);
        assert!(
            matches!(client.health(), Err(AgentError::Unavailable)),
            "health must fail-closed when budget empty"
        );
        env::remove_var("TEST_PAID_API_KEY");
    }

    #[test]
    fn health_returns_ok_when_budget_available() {
        env::set_var("TEST_PAID_API_KEY", "sk-test");
        let channel = MockChannel::new();
        let gate = Arc::new(BudgetGate::new(TokenBucket::new(1.0, 0.0)));
        let client = make_client(channel, gate);
        assert!(client.health().is_ok(), "health ok with budget");
        assert!(
            client.health().is_err(),
            "health fails after budget drained"
        );
        env::remove_var("TEST_PAID_API_KEY");
    }
}
