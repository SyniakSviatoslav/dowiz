//! agent-loop sibling-service binary (P40 DECART-B).
//!
//! Runs the bounded `AgentLoop` behind a localhost HTTP surface that
//! `native-spa-server` proxies `/api/agent` to. Reuses `main.rs`'s exact
//! `OllamaAdapter` wiring — the heavy `llm-adapters` dep lives ONLY in this
//! process, keeping the SPA binary zero-OCI (choice B).
//!
//! Live path precondition (OPS, not a code defect): a reachable Ollama daemon at
//! `$OLLAMA_BASE` (default `http://127.0.0.1:11434`). Without it, `AgentLoop`
//! fail-closes to `AssistantUnavailable` — a typed outcome, never a hang.

use agent_facade::{FixtureOrders, ReadOrderStatusTool, ToolAction, ToolResource, ToolScope};
use agent_loop::service;
use llm_adapters::OllamaAdapter;
use std::net::TcpListener;

/// Free localhost port for the sibling service (blueprint suggests 8771).
const DEFAULT_AGENT_PORT: u16 = 8771;

fn main() -> std::io::Result<()> {
    let base_url =
        std::env::var("OLLAMA_BASE").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let port: u16 = std::env::var("DOWIZ_AGENT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_AGENT_PORT);

    let backend = OllamaAdapter::new(&base_url);
    // Solo-offline fixture source (P40 DoD). A real deployment swaps in a real source.
    let orders = FixtureOrders::from_pairs(&[("ORD-42", "IN_DELIVERY")]);
    let tool = ReadOrderStatusTool::new(orders);
    let granted = ToolScope {
        resource: ToolResource::OrderStatus,
        action: ToolAction::Read,
    };

    let listener = TcpListener::bind(("127.0.0.1", port))?;
    eprintln!(
        "[agent-service] listening on 127.0.0.1:{port} (ollama base {base_url}) — POST /agent {{\"prompt\":...}}"
    );
    service::serve_forever(&listener, &backend, &tool, granted)
}
