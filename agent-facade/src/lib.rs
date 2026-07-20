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
    ///
    /// Item 31 §4.4 Phase-A cutover: parses with the kernel-owned `dowiz_kernel::json` primitive
    /// (bounded, degrade-closed, differentially proven vs serde_json) instead of `serde_json`.
    /// LLM-originated, bounded schema; a malformed arg is already fail-closed to `BadArg`.
    fn parse_arg(raw_arg: &str) -> Result<String, ToolError> {
        let v = dowiz_kernel::json::parse(raw_arg)
            .map_err(|_| ToolError::BadArg(raw_arg.to_string()))?;
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

// ─────────────────────────── WebFetchTool (P40 native browsing, R&D lane) ──────
//
// The concrete `ToolResource::WebFetch` implementation. OFF by default (the
// `web-fetch` Cargo feature, see Cargo.toml) — the network fetch (`ureq`) and
// the readable-text extraction (`dowiz_kernel::readability`, always compiled,
// pure `std`) both live here, never in the kernel, per the module firewall
// (`ports::tool`'s doc comment: "ZERO network / HTTP / JSON / serde" in the
// kernel; the concrete impl + framing live in downstream crates).
//
// Explicitly NOT interactive/JS-driven browsing — that class of capability
// stays an external tool behind its own port (e.g. `agent-browser`), never
// reimplemented here or in-kernel. See the native-agentic-browsing research
// this tool's scope is drawn directly from.
#[cfg(feature = "web-fetch")]
pub struct WebFetchTool {
    agent: ureq::Agent,
}

#[cfg(feature = "web-fetch")]
impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "web-fetch")]
impl WebFetchTool {
    /// Hard cap on response body bytes read — a page over this is truncated at
    /// the byte boundary before UTF-8 decode, never buffered unbounded. Sized
    /// generously for real article pages while refusing to let one fetch
    /// exhaust memory (the same degrade-closed-cap discipline as the kernel's
    /// `MAX_CHANNEL_EVENTS`/`MAX_HARMONIC_NODES` fixes).
    const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
    /// Per-request timeout — a hung remote server must not hang the agent turn.
    const TIMEOUT_S: u64 = 10;

    pub const SPEC: ToolSpec = ToolSpec {
        name: "web_fetch",
        description: "Fetch a URL (http/https only) and return its readable text \
                      content, with navigation/ads/boilerplate stripped. Read-only; \
                      does not execute JavaScript or render the page — static \
                      server-rendered content only.",
        arg_name: "url",
        scope: ToolScope {
            resource: ToolResource::WebFetch,
            action: ToolAction::Read,
        },
    };

    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(Self::TIMEOUT_S))
            .build();
        WebFetchTool { agent }
    }

    fn parse_arg(raw_arg: &str) -> Result<String, ToolError> {
        let v = dowiz_kernel::json::parse(raw_arg)
            .map_err(|_| ToolError::BadArg(raw_arg.to_string()))?;
        v.get("url")
            .and_then(|o| o.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ToolError::BadArg(raw_arg.to_string()))
    }

    /// Scheme allowlist — only `http`/`https`. Refuses `file://`, `ftp://`, and
    /// anything else BEFORE the request is ever made (the SSRF/local-file-read
    /// class of mistake is unrepresentable at the arg-validation step, not
    /// merely discouraged). Full SSRF hardening (blocking private/link-local
    /// IP ranges the DNS name resolves to) is NOT done here — named limitation,
    /// not a silent gap: this tool is R&D-lane and network-egress-bounded by
    /// its caller's deployment, the same posture `llm-adapters` already takes.
    fn validate_url(url: &str) -> Result<(), ToolError> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(())
        } else {
            Err(ToolError::BadArg(format!(
                "unsupported URL scheme (only http/https): {url}"
            )))
        }
    }
}

#[cfg(feature = "web-fetch")]
impl ToolPort for WebFetchTool {
    fn spec(&self) -> &ToolSpec {
        &Self::SPEC
    }

    fn invoke(&self, granted: ToolScope, inv: &ToolInvocation) -> Result<ToolOutput, ToolError> {
        // Fail-closed scope check FIRST — no network call happens on a denied grant.
        if granted != Self::SPEC.scope {
            return Err(ToolError::ScopeDenied);
        }
        let url = Self::parse_arg(&inv.raw_arg)?;
        Self::validate_url(&url)?;

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|_| ToolError::Unavailable)?;

        let mut body = Vec::new();
        {
            use std::io::Read as _;
            response
                .into_reader()
                .take(Self::MAX_BODY_BYTES)
                .read_to_end(&mut body)
                .map_err(|_| ToolError::Unavailable)?;
        }
        // Fail-open on non-UTF-8: lossy-decode rather than refuse the whole
        // fetch over a handful of bad bytes (real pages are near-universally
        // UTF-8; a strict reject here would be a worse failure mode than a
        // few replacement characters in boilerplate that gets scored out
        // anyway).
        let html = String::from_utf8_lossy(&body);

        let text = dowiz_kernel::readability::extract(&html);
        Ok(ToolOutput { content: text })
    }
}

#[cfg(all(test, feature = "web-fetch"))]
mod web_fetch_tests {
    use super::*;

    // No test here makes a real network call — CI must never depend on live
    // internet access. What IS verified without network: the fail-closed scope
    // check runs before any I/O, URL-scheme validation refuses non-http(s)
    // before any I/O, and arg parsing rejects malformed input before any I/O.
    // An end-to-end fetch-against-a-local-mock-server test is a named follow-up,
    // not silently skipped — see the tool's module doc.

    #[test]
    fn scope_denial_never_touches_network() {
        let tool = WebFetchTool::new();
        let wrong_scope = ToolScope {
            resource: ToolResource::OrderStatus, // NOT WebFetch
            action: ToolAction::Read,
        };
        let inv = ToolInvocation {
            tool_name: "web_fetch".to_string(),
            raw_arg: r#"{"url":"https://example.com"}"#.to_string(),
        };
        // If this ever reached the network it would hang/error differently in
        // a sandboxed CI runner; ScopeDenied must come back immediately.
        assert!(matches!(
            tool.invoke(wrong_scope, &inv),
            Err(ToolError::ScopeDenied)
        ));
    }

    #[test]
    fn rejects_non_http_schemes_before_any_fetch() {
        let tool = WebFetchTool::new();
        let granted = WebFetchTool::SPEC.scope;
        for bad_url in [
            "file:///etc/passwd",
            "ftp://x/y",
            "javascript:alert(1)",
            "data:text/html,x",
        ] {
            let inv = ToolInvocation {
                tool_name: "web_fetch".to_string(),
                raw_arg: format!(r#"{{"url":"{bad_url}"}}"#),
            };
            match tool.invoke(granted, &inv) {
                Err(ToolError::BadArg(_)) => {}
                other => panic!("expected BadArg for {bad_url}, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_malformed_json_arg() {
        let tool = WebFetchTool::new();
        let granted = WebFetchTool::SPEC.scope;
        let inv = ToolInvocation {
            tool_name: "web_fetch".to_string(),
            raw_arg: "not json at all".to_string(),
        };
        assert!(matches!(
            tool.invoke(granted, &inv),
            Err(ToolError::BadArg(_))
        ));
    }

    #[test]
    fn rejects_missing_url_field() {
        let tool = WebFetchTool::new();
        let granted = WebFetchTool::SPEC.scope;
        let inv = ToolInvocation {
            tool_name: "web_fetch".to_string(),
            raw_arg: r#"{"not_url":"https://example.com"}"#.to_string(),
        };
        assert!(matches!(
            tool.invoke(granted, &inv),
            Err(ToolError::BadArg(_))
        ));
    }

    #[test]
    fn spec_scope_is_webfetch_read_only() {
        assert_eq!(WebFetchTool::SPEC.scope.resource, ToolResource::WebFetch);
        assert_eq!(WebFetchTool::SPEC.scope.action, ToolAction::Read);
    }
}
