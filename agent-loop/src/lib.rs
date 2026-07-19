//! agent-loop — P40's bounded agent executor.
//!
//! `AgentLoop::run` drives at most `MAX_AGENT_ITERATIONS` plan→act→observe turns
//! over an [`LlmBackend`] behind [`agent_facade`]. It imports ONLY `agent-facade` —
//! `dowiz-kernel` is reachable only at depth 2, only through the facade (see the
//! committed firewall test at the bottom of this file). The model can influence the
//! loop ONLY through `ChatResponse.content` and `ChatResponse.tool_calls`; those bytes
//! map onto exactly three continuations (return as Answer / UnknownTool / hand to the
//! one `ToolPort`). There is no fourth path and no retry construct (§4.1/§4.3).

use agent_facade::{
    CachePolicy, ChatRequest, ChatResponse, LlmBackend, Message, TaskClass, ToolCallReq, ToolError,
    ToolInvocation, ToolPort, ToolScope, Usage,
};
use std::collections::BTreeMap;

/// P40 DECART-B: the sibling HTTP service that `native-spa-server` proxies to.
pub mod service;

/// Hard iteration ceiling. 4 = one plan turn + one tool turn + one recovery
/// turn + one answer turn. Raising it is a reviewed const change, not a knob.
pub const MAX_AGENT_ITERATIONS: u8 = 4;

/// Per-tool-invocation wall-clock ceiling. The P40 fixture source is instant; a real
/// I/O-backed source enforces this via a watchdog thread + `recv_timeout` (std-only,
/// matching the dispatcher's `std::thread` discipline). Kept here as the named constant.
pub const TOOL_TIMEOUT_MS: u64 = 5_000;

/// One loop event — the log is a `Vec` of these; tests assert on the SEQUENCE
/// (standard item 3: event-driven, matches the kernel's own decide/fold shape).
#[derive(Debug, Clone)]
pub enum LoopEventKind {
    /// Model returned a final/partial reply.
    ModelReply { content: String, total_tokens: u32 },
    /// A tool call was parsed from the model's reply (name + verbatim raw arg).
    ToolCallParsed { tool_name: String, raw_arg: String },
    /// A malformed tool call — logged as an observation, NOT a crash.
    ToolCallMalformed { raw: String, reason: String },
    /// A tool produced output (the observation fed back to the model).
    ToolResult { tool_name: String, output: String },
    /// A tool failed (typed `ToolError` rendered) — visible in the log, never swallowed.
    ToolFailed { tool_name: String, error: String },
}

/// One loop event — the log is a `Vec` of these; tests assert on the SEQUENCE.
#[derive(Debug, Clone)]
pub struct LoopLogEntry {
    pub iteration: u8,
    pub event: LoopEventKind,
}

/// Terminal outcome of one loop run. EVERY path lands here — no panic, no unbounded path.
#[derive(Debug)]
pub enum LoopOutcome {
    /// The model produced a final answer (with the full event log attached).
    Answer {
        text: String,
        log: Vec<LoopLogEntry>,
    },
    /// Backend absent/refusing before or during the run. P41's degradation contract
    /// consumes exactly this variant.
    AssistantUnavailable {
        reason: String,
        log: Vec<LoopLogEntry>,
    },
    /// caps().tool_calling == false after the live probe — fail-closed refusal to
    /// start a tool run (the loop does NOT degrade to tool-less chat silently).
    ToolCallingUnsupported { backend_id: String },
    /// The iteration ceiling fired. The log shows why (e.g. repeated malformed calls).
    /// Never a silent truncation — the caller sees the cap by type.
    IterationCapExceeded { log: Vec<LoopLogEntry> },
}

/// The executor. Constructed with the backend it chats through, the single tool it may
/// call, and the scope it is granted (checked fail-closed by the tool impl).
pub struct AgentLoop<'a> {
    backend: &'a dyn LlmBackend,
    tool: &'a dyn ToolPort,
    granted: ToolScope,
}

impl<'a> AgentLoop<'a> {
    pub fn new(backend: &'a dyn LlmBackend, tool: &'a dyn ToolPort, granted: ToolScope) -> Self {
        AgentLoop {
            backend,
            tool,
            granted,
        }
    }

    /// Run one user-initiated request to completion and stop (no self-scheduling).
    pub fn run(&self, user_request: &str) -> LoopOutcome {
        let mut log: Vec<LoopLogEntry> = Vec::new();

        // 0. Preflight — fail-closed on backend absence.
        if let Err(e) = self.backend.health() {
            return LoopOutcome::AssistantUnavailable {
                reason: format!("health: {:?}", e),
                log,
            };
        }
        // 0b. Preflight — fail-closed on no tool-calling capability.
        let caps = self.backend.caps();
        if !caps.tool_calling {
            return LoopOutcome::ToolCallingUnsupported {
                backend_id: backend_id_of(self.backend),
            };
        }

        let tool_spec = self.tool.spec().clone();
        // system message = the tool contract (verbatim from ToolSpec).
        let system_msg = Message {
            role: "system".to_string(),
            content: format!(
                "You are a delivery-ops assistant. You may call ONE tool: {}({}) — {}. \
                 Respond with a tool call when you need it, otherwise answer directly.",
                tool_spec.name, tool_spec.arg_name, tool_spec.description
            ),
        };

        // Observation buffer: model replies accumulate messages we feed back.
        let mut messages: Vec<Message> = vec![
            system_msg,
            Message {
                role: "user".to_string(),
                content: user_request.to_string(),
            },
        ];

        for iteration in 1..=MAX_AGENT_ITERATIONS {
            // Build the request carrying the tool declaration.
            let req = ChatRequest {
                model_id: String::new(), // adapter routes by task_class
                messages: messages.clone(),
                temperature: 0.0,
                top_p: 1.0,
                max_tokens: 1024,
                seed: None,
                task_class: TaskClass::General,
                cache_policy: CachePolicy::NoCache, // live proof must not hit cache
                options: BTreeMap::new(),
                tools: vec![agent_facade::ToolDecl {
                    name: tool_spec.name.to_string(),
                    description: tool_spec.description.to_string(),
                    arg_name: tool_spec.arg_name.to_string(),
                }],
            };

            let resp: ChatResponse = match self.backend.chat(&req) {
                Ok(r) => r,
                Err(e) => {
                    return LoopOutcome::AssistantUnavailable {
                        reason: format!("chat iter {}: {:?}", iteration, e),
                        log,
                    };
                }
            };
            let usage: Usage = resp.usage;
            log.push(LoopLogEntry {
                iteration,
                event: LoopEventKind::ModelReply {
                    content: resp.content.clone(),
                    total_tokens: usage.total_tokens,
                },
            });

            // No tool calls ⇒ final answer.
            if resp.tool_calls.is_empty() {
                return LoopOutcome::Answer {
                    text: resp.content.clone(),
                    log,
                };
            }

            // One tool, one call per turn. Anything further is logged malformed.
            let call: &ToolCallReq = &resp.tool_calls[0];
            if resp.tool_calls.len() > 1 {
                log.push(LoopLogEntry {
                    iteration,
                    event: LoopEventKind::ToolCallMalformed {
                        raw: format!("{} calls in one reply", resp.tool_calls.len()),
                        reason: "multiple calls".to_string(),
                    },
                });
            }

            // Unknown tool name ⇒ malformed observation, continue.
            if call.name != tool_spec.name {
                let reason = format!("unknown tool '{}'", call.name);
                log.push(LoopLogEntry {
                    iteration,
                    event: LoopEventKind::ToolCallMalformed {
                        raw: call.arguments_json.clone(),
                        reason: reason.clone(),
                    },
                });
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: resp.content.clone(),
                });
                messages.push(Message {
                    role: "tool".to_string(),
                    content: format!("error: {}", reason),
                });
                continue;
            }

            // Known tool: log parsed, invoke, fold outcome back as observation.
            log.push(LoopLogEntry {
                iteration,
                event: LoopEventKind::ToolCallParsed {
                    tool_name: call.name.clone(),
                    raw_arg: call.arguments_json.clone(),
                },
            });
            let inv = ToolInvocation {
                tool_name: call.name.clone(),
                raw_arg: call.arguments_json.clone(),
            };
            match self.tool.invoke(self.granted, &inv) {
                Ok(out) => {
                    log.push(LoopLogEntry {
                        iteration,
                        event: LoopEventKind::ToolResult {
                            tool_name: call.name.clone(),
                            output: out.content.clone(),
                        },
                    });
                    messages.push(Message {
                        role: "tool".to_string(),
                        content: out.content.clone(),
                    });
                }
                Err(e) => {
                    // §3.5.2: a known-tool with a malformed ARG is a malformed
                    // CALL (observation, loop continues), NOT a hard tool failure.
                    // Other tool errors (NotFound/Unavailable/Timeout/ScopeDenied)
                    // are typed ToolFailed outcomes.
                    let (event, obs) = match e {
                        ToolError::BadArg(_) => (
                            LoopEventKind::ToolCallMalformed {
                                raw: call.arguments_json.clone(),
                                reason: format!("{:?}", e),
                            },
                            format!("error: malformed argument: {:?}", e),
                        ),
                        other => (
                            LoopEventKind::ToolFailed {
                                tool_name: call.name.clone(),
                                error: format!("{:?}", other),
                            },
                            format!("error: {}", format!("{:?}", other)),
                        ),
                    };
                    log.push(LoopLogEntry { iteration, event });
                    messages.push(Message {
                        role: "tool".to_string(),
                        content: obs,
                    });
                }
            }
            // Loop continues: the model gets the observation and either answers
            // (next iteration, empty tool_calls → Answer) or malforms again.
        }

        LoopOutcome::IterationCapExceeded { log }
    }
}

/// Best-effort backend id label for the `ToolCallingUnsupported` variant.
fn backend_id_of(_b: &dyn LlmBackend) -> String {
    "llm-backend".to_string()
}
