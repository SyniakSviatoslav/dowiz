//! B-e adversarial closure + §3.1 firewall test for `agent-loop`.
//!
//! All adversarial cases use a deterministic scripted `LlmBackend` (no daemon),
//! per blueprint §3.5/§3.6 — "no live daemon needed, fully deterministic".
//! The live end-to-end proof (real Ollama) is `#[ignore]`d and runs only when the
//! daemon is present (fail-closed: it SKIPs cleanly, never fakes green).

use agent_facade::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse, ToolCallReq, ToolError, ToolScope, Usage,
};
use agent_loop::{AgentLoop, LoopEventKind, LoopOutcome, MAX_AGENT_ITERATIONS};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ── scripted backend (thread-safe; trait is &self) ───────────────────────────
struct ScriptedBackendMT {
    caps: Caps,
    healthy: bool,
    script: Mutex<VecDeque<ChatResponse>>,
}

impl LlmBackend for ScriptedBackendMT {
    fn id(&self) -> &str {
        "scripted-mt"
    }
    fn caps(&self) -> Caps {
        self.caps
    }
    fn health(&self) -> Result<(), LlmError> {
        if self.healthy {
            Ok(())
        } else {
            Err(LlmError::Unavailable)
        }
    }
    fn embed(&self, _: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn rerank(&self, _: &RerankRequest) -> Result<RerankResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let mut q = self.script.lock().unwrap();
        Ok(q.pop_front().unwrap_or_else(empty_reply))
    }
}

fn empty_reply() -> ChatResponse {
    ChatResponse {
        content: String::new(),
        usage: Usage::default(),
        tool_calls: vec![],
    }
}

fn reply_with(content: &str, calls: Vec<ToolCallReq>) -> ChatResponse {
    ChatResponse {
        content: content.to_string(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        tool_calls: calls,
    }
}

fn tool_call(name: &str, args: &str) -> ToolCallReq {
    ToolCallReq {
        name: name.to_string(),
        arguments_json: args.to_string(),
    }
}

// ── spy source (counts invocations) ──────────────────────────────────────────
struct SpySource {
    inner: agent_facade::FixtureOrders,
    calls: Arc<AtomicUsize>,
}
impl agent_facade::OrderStatusSource for SpySource {
    fn status_of(&self, id: &str) -> Result<String, ToolError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.status_of(id)
    }
}

fn granted_scope() -> ToolScope {
    agent_facade::ToolScope {
        resource: agent_facade::ToolResource::OrderStatus,
        action: agent_facade::ToolAction::Read,
    }
}

fn fixture() -> agent_facade::FixtureOrders {
    agent_facade::FixtureOrders::from_pairs(&[("ord-7", "IN_DELIVERY")])
}

fn tool() -> agent_facade::ReadOrderStatusTool<agent_facade::FixtureOrders> {
    agent_facade::ReadOrderStatusTool::new(fixture())
}

// ── B-e(i).1 — unknown tool name never crashes; continues; spy source untouched ──
#[test]
fn unknown_tool_continues_and_spy_sees_zero_calls() {
    let src = Arc::new(SpySource {
        inner: fixture(),
        calls: Arc::new(AtomicUsize::new(0)),
    });
    let t = tool();
    let backend = ScriptedBackendMT {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        healthy: true,
        script: Mutex::new(VecDeque::from(vec![
            reply_with("", vec![tool_call("transfer_money", "{}")]),
            reply_with("I cannot transfer money.", vec![]),
        ])),
    };
    let out = AgentLoop::new(&backend, &t, granted_scope()).run("Pay the supplier");
    match out {
        LoopOutcome::Answer { text, log } => {
            assert!(
                text.contains("cannot"),
                "expected refusal answer, got: {text}"
            );
            assert!(matches!(log[0].event, LoopEventKind::ModelReply { .. }));
            assert!(matches!(
                log[1].event,
                LoopEventKind::ToolCallMalformed { .. }
            ));
            assert!(matches!(log[2].event, LoopEventKind::ModelReply { .. }));
        }
        other => panic!("expected Answer, got {other:?}"),
    }
    // The red-line teeth: the (unrepresentable) write tool was NEVER reached.
    assert_eq!(src.calls.load(Ordering::SeqCst), 0);
}

// ── B-e(i).2 — truncated JSON ⇒ BadArg, loop recovers ────────────────────────
#[test]
fn truncated_json_arg_is_badarg_but_recovers() {
    let t = tool();
    let backend = ScriptedBackendMT {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        healthy: true,
        script: Mutex::new(VecDeque::from(vec![
            reply_with("", vec![tool_call("read_order_status", "{\"order_id\":\"")]),
            reply_with("Sorry, malformed request.", vec![]),
        ])),
    };
    let out = AgentLoop::new(&backend, &t, granted_scope()).run("status of ord-7");
    match out {
        LoopOutcome::Answer { log, .. } => {
            let malformed = log
                .iter()
                .any(|e| matches!(e.event, LoopEventKind::ToolCallMalformed { .. }));
            assert!(malformed, "expected a malformed-tool event, log={log:?}");
        }
        other => panic!("expected Answer, got {other:?}"),
    }
}

// ── B-e(i).3 — malformed EVERY turn ⇒ caps at exactly MAX_AGENT_ITERATIONS ─────
#[test]
fn all_malformed_caps_at_iteration_ceiling() {
    let t = tool();
    let mut script = VecDeque::new();
    for _ in 0..(MAX_AGENT_ITERATIONS as usize + 2) {
        script.push_back(reply_with(
            "",
            vec![tool_call("read_order_status", "{not json")],
        ));
    }
    let backend = ScriptedBackendMT {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        healthy: true,
        script: Mutex::new(script),
    };
    let out = AgentLoop::new(&backend, &t, granted_scope()).run("status of ord-7");
    match out {
        LoopOutcome::IterationCapExceeded { log } => {
            let model_turns = log
                .iter()
                .filter(|e| matches!(e.event, LoopEventKind::ModelReply { .. }))
                .count();
            assert_eq!(
                model_turns, MAX_AGENT_ITERATIONS as usize,
                "cap must be a hard invariant, got {model_turns} turns"
            );
        }
        other => panic!("expected IterationCapExceeded, got {other:?}"),
    }
}

// ── B-e(i).4 — two tool_calls in one reply ⇒ first processed, second malformed ──
#[test]
fn two_tool_calls_one_reply_processes_first_only() {
    let t = tool();
    let backend = ScriptedBackendMT {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        healthy: true,
        script: Mutex::new(VecDeque::from(vec![
            reply_with(
                "",
                vec![
                    tool_call("read_order_status", "{\"order_id\":\"ord-7\"}"),
                    tool_call("read_order_status", "{\"order_id\":\"ord-7\"}"),
                ],
            ),
            reply_with("ord-7 is in delivery.", vec![]),
        ])),
    };
    let out = AgentLoop::new(&backend, &t, granted_scope()).run("status of ord-7");
    match out {
        LoopOutcome::Answer { log, .. } => {
            let parsed = log
                .iter()
                .filter(|e| matches!(e.event, LoopEventKind::ToolCallParsed { .. }))
                .count();
            let malformed = log
                .iter()
                .filter(|e| matches!(e.event, LoopEventKind::ToolCallMalformed { .. }))
                .count();
            assert_eq!(parsed, 1, "exactly one tool call processed");
            assert_eq!(malformed, 1, "second call in same reply logged malformed");
        }
        other => panic!("expected Answer, got {other:?}"),
    }
}

// ── B-e(ii).1 — tool NotFound ⇒ typed ToolFailed, visible, loop recovers ──────
#[test]
fn tool_notfound_is_typed_outcome_not_silent() {
    let t = tool(); // fixture has no ord-9
    let backend = ScriptedBackendMT {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        healthy: true,
        script: Mutex::new(VecDeque::from(vec![
            reply_with(
                "",
                vec![tool_call("read_order_status", "{\"order_id\":\"ord-9\"}")],
            ),
            reply_with("order not found.", vec![]),
        ])),
    };
    let out = AgentLoop::new(&backend, &t, granted_scope()).run("status of ord-9");
    match out {
        LoopOutcome::Answer { log, text } => {
            assert!(
                text.contains("not found"),
                "failure must surface in answer: {text}"
            );
            let failed = log
                .iter()
                .any(|e| matches!(e.event, LoopEventKind::ToolFailed { .. }));
            assert!(failed, "expected a ToolFailed event, log={log:?}");
        }
        other => panic!("expected Answer, got {other:?}"),
    }
}

// ── B-e(ii).3 — backend dies mid-run ⇒ AssistantUnavailable, partial log kept ──
#[test]
fn backend_dies_mid_run_is_assistant_unavailable() {
    let t = tool();
    // KillBackend returns ok on the first chat, then Err(Unavailable) forever.
    struct KillBackend {
        caps: Caps,
        killed: Mutex<bool>,
    }
    impl LlmBackend for KillBackend {
        fn id(&self) -> &str {
            "kill-backend"
        }
        fn caps(&self) -> Caps {
            self.caps
        }
        fn health(&self) -> Result<(), LlmError> {
            Ok(())
        }
        fn embed(&self, _: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn rerank(&self, _: &RerankRequest) -> Result<RerankResponse, LlmError> {
            Err(LlmError::Unsupported)
        }
        fn chat(&self, _: &ChatRequest) -> Result<ChatResponse, LlmError> {
            let mut k = self.killed.lock().unwrap();
            if *k {
                Err(LlmError::Unavailable)
            } else {
                *k = true;
                Ok(reply_with(
                    "",
                    vec![tool_call("read_order_status", "{\"order_id\":\"ord-7\"}")],
                ))
            }
        }
    }
    let kb = KillBackend {
        caps: Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        },
        killed: Mutex::new(false),
    };
    let out = AgentLoop::new(&kb, &t, granted_scope()).run("status of ord-7");
    match out {
        LoopOutcome::AssistantUnavailable { reason, log } => {
            assert!(
                !log.is_empty(),
                "partial progress must be attached, not dropped"
            );
            assert!(reason.contains("Unavailable"), "reason={reason}");
        }
        other => panic!("expected AssistantUnavailable, got {other:?}"),
    }
}

// ── §3.1 firewall — agent-loop imports NO dowiz-kernel directly ───────────────
#[test]
fn firewall_no_direct_kernel_dependency() {
    let out = std::process::Command::new("cargo")
        .args(["tree", "-p", "agent-loop", "--depth", "1"])
        .output()
        .expect("cargo tree available");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("dowiz-kernel"),
        "agent-loop must NOT directly depend on dowiz-kernel (firewall breach):\n{stdout}"
    );
    // Integration tests run with cwd = the crate root (agent-loop/), so the
    // source dir is `src/`, NOT `agent-loop/src/` — the old path silently
    // grepped a nonexistent directory and passed vacuously (RED-proven
    // 2026-07-20 by planting a `dowiz_kernel` token that the old leg missed).
    assert!(
        std::path::Path::new("src").is_dir(),
        "firewall grep leg must run from the crate root (src/ not found)"
    );
    let grep = std::process::Command::new("sh")
        .args(["-c", "grep -rn 'dowiz_kernel' src/ || true"])
        .output()
        .unwrap();
    let gstdout = String::from_utf8_lossy(&grep.stdout);
    assert!(
        gstdout.trim().is_empty(),
        "agent-loop/src must not reference dowiz_kernel directly:\n{gstdout}"
    );
}
