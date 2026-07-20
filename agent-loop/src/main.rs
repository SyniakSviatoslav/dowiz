//! agent-loop host binary (audit #12: the executor had zero callers + emitted 0 metrics).
//!
//! Runs one `AgentLoop` turn over a real `LlmBackend` (`OllamaAdapter`) with the single
//! `read_order_status` tool, then emits the designed `dowiz_agent_*` metric IDs into the
//! SHARED harvest ledger (`track_record.jsonl`) — the same row the `Dispatcher` writes, via
//! `llm_adapters::append_harvest` (one channel, no schema drift; AGENTS.md native-telemetry rule).
//!
//! This binary is the concrete caller the library was missing: it wires `AgentLoop` to a
//! backend, so the executor is no longer dead code, and it makes the agent's telemetry
//! observable (previously 0 of the 20 designed metric IDs emitted anything).

use agent_facade::{FixtureOrders, ReadOrderStatusTool};
use agent_loop::{AgentLoop, LoopEventKind, LoopLogEntry, LoopOutcome};
use llm_adapters::{append_harvest, OllamaAdapter, TrackRecord};

fn main() {
    // Base URL from env (default: local Ollama daemon, already running on this host).
    let base_url = std::env::var("OLLAMA_BASE").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let request = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "What is the status of order ORD-42?".into());

    let backend = OllamaAdapter::new(&base_url);
    // Solo-offline fixture source (P40 DoD). A real deployment swaps in HttpOrderStatusSource.
    let orders = FixtureOrders::from_pairs(&[("ORD-42", "IN_DELIVERY")]);
    let tool = ReadOrderStatusTool::new(orders);
    // Grant exactly the tool's declared scope (fail-closed enforcement lives in the tool impl).
    let granted = agent_facade::ToolScope {
        resource: agent_facade::ToolResource::OrderStatus,
        action: agent_facade::ToolAction::Read,
    };

    let loop_ = AgentLoop::new(&backend, &tool, granted);
    let outcome = loop_.run(&request);

    // Fold the event log into the designed agent metrics.
    let mut iterations: u64 = 0;
    let mut tool_calls: u64 = 0;
    let mut total_tokens: u64 = 0;
    let mut success = false;

    match &outcome {
        LoopOutcome::Answer { text, log } => {
            success = true;
            println!("ANSWER: {text}");
            fold_log(log, &mut iterations, &mut tool_calls, &mut total_tokens);
        }
        LoopOutcome::AssistantUnavailable { reason, log } => {
            eprintln!("ASSISTANT UNAVAILABLE: {reason}");
            fold_log(log, &mut iterations, &mut tool_calls, &mut total_tokens);
        }
        LoopOutcome::ToolCallingUnsupported { backend_id } => {
            eprintln!("TOOL-CALLING UNSUPPORTED on backend: {backend_id}");
        }
        LoopOutcome::IterationCapExceeded { log } => {
            eprintln!("ITERATION CAP EXCEEDED");
            fold_log(log, &mut iterations, &mut tool_calls, &mut total_tokens);
        }
    }

    // Emit the designed `dowiz_agent_*` metric IDs into the shared harvest ledger.
    // `task` is the metric ID gov_route folds by; `value` carries the measured count.
    emit("agent_loop", success, total_tokens, total_tokens as f64);
    emit(
        "dowiz_agent_iterations",
        success,
        iterations,
        iterations as f64,
    );
    emit(
        "dowiz_agent_tool_calls",
        success,
        tool_calls,
        tool_calls as f64,
    );

    println!(
        "\n[agent metrics] iterations={iterations} tool_calls={tool_calls} tokens={total_tokens} success={success}"
    );
    if !success {
        std::process::exit(1);
    }
}

/// Count iterations / tool calls / tokens from a loop event log.
fn fold_log(
    log: &[LoopLogEntry],
    iterations: &mut u64,
    tool_calls: &mut u64,
    total_tokens: &mut u64,
) {
    for entry in log {
        *iterations = (*iterations).max(entry.iteration as u64);
        match &entry.event {
            LoopEventKind::ToolCallParsed { .. }
            | LoopEventKind::ToolResult { .. }
            | LoopEventKind::ToolFailed { .. } => *tool_calls += 1,
            LoopEventKind::ModelReply {
                total_tokens: t, ..
            } => {
                *total_tokens += *t as u64;
            }
            LoopEventKind::ToolCallMalformed { .. } => {}
        }
    }
}
fn emit(task: &str, success: bool, tokens: u64, cost: f64) {
    let rec = TrackRecord {
        backend_id: "ollama".into(),
        model_id: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.1:8b".into()),
        total_tokens: tokens,
        ms: 0,
        task: task.into(),
        success,
        value: cost, // EV numerator carries the measured count for agent metrics
        cost,        // EV denominator mirrors tokens for local Ollama
    };
    append_harvest(&rec);
}
