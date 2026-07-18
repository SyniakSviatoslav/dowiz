//! P40 B-d live end-to-end proof (§3.4). `#[ignore]`d: requires a live Ollama
//! daemon with `llama3.1:8b` resident. Runs on demand (`cargo test -- --ignored`
//! from llm-adapters/) for a REAL proof; skips cleanly in offline CI (no fake-green).
//!
//! Exercises the full stack: `OllamaAdapter` (real `/api/show` tool_calling probe)
//! → `ReadOrderStatusTool<FixtureOrders>` → `AgentLoop::run`, and asserts the agent
//! DID call the tool (event sequence) — the "agent can DO something" falsifiable gate.

use agent_facade::{FixtureOrders, ReadOrderStatusTool, ToolAction, ToolResource, ToolScope};
use agent_loop::{AgentLoop, LoopEventKind, LoopOutcome, MAX_AGENT_ITERATIONS};
use llm_adapters::OllamaAdapter;

#[test]
#[ignore] // live-daemon proof; run with `cargo test -- --ignored`
fn agent_reads_order_status_end_to_end() {
    // Live adapter (General route → llama3.1:8b). The probe must report true.
    let backend = OllamaAdapter::new("http://127.0.0.1:11434");
    let caps = <OllamaAdapter as agent_facade::LlmBackend>::caps(&backend);
    assert!(
        caps.tool_calling,
        "live probe must report tool_calling=true for llama3.1:8b (got {:?})",
        caps
    );

    // Fixture source: ord-7 → IN_DELIVERY.
    let src = FixtureOrders::from_pairs(&[("ord-7", "IN_DELIVERY")]);
    let tool = ReadOrderStatusTool::new(src);
    let granted = ToolScope {
        resource: ToolResource::OrderStatus,
        action: ToolAction::Read,
    };

    let loop_ = AgentLoop::new(&backend, &tool, granted);
    let out = loop_.run("What is the status of order ord-7?");

    match out {
        LoopOutcome::Answer { log, .. } => {
            // The agent DID something: it parsed a tool call for read_order_status.
            let parsed = log.iter().any(|e| {
                matches!(
                    e.event,
                    LoopEventKind::ToolCallParsed { ref tool_name, .. }
                        if tool_name == "read_order_status"
                )
            });
            assert!(
                parsed,
                "agent must have called read_order_status, log={log:?}"
            );
            // And it observed a real result.
            let result = log
                .iter()
                .any(|e| matches!(e.event, LoopEventKind::ToolResult { .. }));
            assert!(result, "agent must have observed a ToolResult, log={log:?}");
        }
        LoopOutcome::ToolCallingUnsupported { .. } => {
            panic!("live probe said tool_calling=false — daemon/model mismatch");
        }
        other => panic!("expected Answer, got {other:?} (cap={MAX_AGENT_ITERATIONS})"),
    }
}
