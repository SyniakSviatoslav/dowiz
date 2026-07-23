//! agent/loop.rs — the bounded, fail-closed AgentLoop executor (WAVE P40).
//!
//! # What this is
//! A single-owner, offline-testable executor that drives a pluggable reasoning seam
//! through a bounded plan→act→observe cycle. The "model" is the [`AgentReasoner`]
//! trait: the kernel defines ONLY the abstract contract (a `next` step), so the loop
//! can be exercised deterministically with a scripted fake and wired to a real
//! `LlmBackend` adapter by a downstream crate — exactly the KernelFacade discipline
//! `ports/llm.rs` / `ports/tool.rs` establish (abstract contract in-kernel, concrete
//! impl downstream).
//!
//! # Wiring (reuse, not re-invention)
//! - Tool DISCOVERY + ACTIVATION: [`crate::ports::tool::SkillRegistry`] (`resolve`
//!   by name, `cards` for the surface catalog). No second registry is defined here.
//! - Tool EXECUTION + CAPABILITY GATE: [`crate::ports::mcp::McpPort`]. Every tool
//!   call is `mcp.call_tool(&McpToolCall{..})`. The `McpPort` is the one door: it
//!   rejects unknown tools and refuses scope-less calls BEFORE the `ToolPort` body
//!   runs. The loop therefore cannot invoke a tool outside the verified grant.
//! - BUDGET: [`crate::token_bucket::TokenBucket`] (the kernel's degrade-closed
//!   budget primitive, reused verbatim — no second budget mechanism).
//!
//! # Fail-closed branches (see `mod.rs` for the full contract)
//! 1. No tool runs without a verified capability → `McpServeError::ScopeDenied`.
//! 2. Unknown tool rejected → `McpServeError::UnknownTool`.
//! 3. Budget exhaustion terminates the loop → `LoopOutcome::AssistantUnavailable`.
//!
//! There is no panic path and no unbounded path in `run`.

use crate::ports::mcp::{
    GrantSet, McpPort, McpServeError, McpToolCall, McpToolListEntry, McpToolResult,
};
use crate::ports::tool::{SkillRegistry, Surface};
use crate::token_bucket::TokenBucket;

/// Hard iteration ceiling. The loop executes at most this many model-driven steps;
/// raising it is a reviewed const change, not a knob. Bounds worst-case wall time to
/// `MAX_AGENT_ITERATIONS × (step_cost)` — no supervisor needed (Self-Termination leg).
pub const MAX_AGENT_ITERATIONS: u8 = 8;

/// One loop event — the log is a `Vec` of these; tests assert on the SEQUENCE
/// (event-driven, matching the kernel's own decide/fold shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopLogEntry {
    /// The 1-based iteration this event belongs to.
    pub iteration: u8,
    /// What happened at this step.
    pub event: LoopEventKind,
}

/// The kind of event recorded for one loop step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopEventKind {
    /// The reasoner produced a reply/decision step.
    ModelReply { content: String, total_tokens: u32 },
    /// The reasoner asked to call a tool; the raw invocation is recorded verbatim.
    ToolCallParsed { tool_name: String, raw_arg: String },
    /// A tool call that could not be dispatched — logged as an observation, not a crash.
    /// `raw` carries the offending name/arg; `reason` is the rendered failure.
    ToolCallMalformed { raw: String, reason: String },
    /// A tool ran and returned output.
    ToolResult { tool_name: String, output: String },
    /// A tool was attempted but failed (rendered `ToolError` / scope refusal).
    ToolFailed { tool_name: String, error: String },
}

/// Terminal outcome of one loop run. EVERY path lands here — no panic, no unbounded
/// continuation.
#[derive(Debug)]
pub enum LoopOutcome {
    /// The reasoner produced a final answer (with the full event log attached).
    Answer {
        text: String,
        log: Vec<LoopLogEntry>,
    },
    /// The reasoner/backend is unavailable, or the budget was exhausted mid-run
    /// (partial progress is attached, never presented as an answer).
    AssistantUnavailable {
        reason: String,
        log: Vec<LoopLogEntry>,
    },
    /// The iteration ceiling fired. The log shows why (e.g. repeated tool calls).
    /// Never a silent truncation — the caller sees the cap by type.
    IterationCapExceeded { log: Vec<LoopLogEntry> },
}

/// Context handed to the [`AgentReasoner`] on each `next` call.
#[derive(Debug, Clone, Copy)]
pub struct ReasonerContext<'a> {
    /// The original user request.
    pub user_request: &'a str,
    /// The tools the capability grant actually exposes (discovery tier). Under an
    /// empty grant this is empty — the reasoner cannot even SEE tools it may not call.
    pub available_tools: &'a [McpToolListEntry],
    /// The event log so far (the model's running observation buffer).
    pub history: &'a [LoopLogEntry],
}

/// One decision from the reasoning seam. The kernel defines only this abstract step;
/// a real adapter implements `next` over an `LlmBackend`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStep {
    /// Call a tool (name + raw argument payload, verbatim — the port impl parses it).
    CallTool { name: String, raw_arg: String },
    /// Emit a final answer and stop the loop.
    Answer { text: String },
    /// Fan-out: dispatch N parallel sub-tasks, each with a name and argument.
    /// The swarm coordinator claims these from the spool and runs them concurrently.
    FanOut { tasks: Vec<SubTask> },
    /// Merge: combine results from previously fan-out tasks into a single answer.
    Merge { task_ids: Vec<usize>, strategy: MergeStrategy },
}

/// A sub-task dispatched by FanOut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubTask {
    /// Unique task id (assigned by the coordinator).
    pub id: usize,
    /// Tool or skill name to invoke.
    pub name: String,
    /// Raw argument payload.
    pub raw_arg: String,
}

/// How Merge combines sub-task results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Concatenate all results in order.
    Concat,
    /// Pick the result with the highest confidence score.
    BestFirst,
    /// Vote: majority-wins on a categorical answer.
    MajorityVote,
}

/// The reasoning seam — the "model". Plug a scripted fake in tests; a downstream
/// crate plugs an `LlmBackend`-backed reasoner in production. Object-safe.
pub trait AgentReasoner {
    /// Given the conversation-so-far, decide the next step.
    fn next(&self, ctx: &ReasonerContext) -> AgentStep;
}

/// The bounded, fail-closed agent loop. Constructed once (consuming the tool
/// registry into a capability-scoped [`McpPort`]) and `run` to completion.
pub struct AgentLoop<R: SkillRegistry> {
    /// The capability-scoped tool boundary. Owns the registry; every tool call is
    /// routed through it — the loop never touches a `ToolPort` directly.
    mcp: McpPort<R>,
    /// The degrade-closed budget. Debits one unit per iteration; exhaustion stops
    /// the loop (fail-closed branch 3).
    budget: TokenBucket,
}

impl<R: SkillRegistry> AgentLoop<R> {
    /// Build the loop from a tool registry, an already-verified capability
    /// [`GrantSet`], the product surface, and a [`TokenBucket`] budget.
    ///
    /// The `grant` is the authority: every tool call is gated against it by the
    /// `McpPort`. A downstream crate derives the grant from `verify_chain` (the
    /// proven hybrid-signed admission path); the loop only ever consumes it.
    pub fn new(registry: R, grant: GrantSet, surface: Surface, budget: TokenBucket) -> Self {
        AgentLoop {
            mcp: McpPort::new(registry, grant, surface),
            budget,
        }
    }

    /// The discovery-tier catalog this loop is authorized to use (never empty-only-in-
    /// name — under an empty grant this returns `[]`, so callers/agents see nothing).
    pub fn available_tools(&self) -> Vec<McpToolListEntry> {
        self.mcp.list_tools()
    }

    /// Run one bounded plan→act→observe cycle. Terminates in at most
    /// [`MAX_AGENT_ITERATIONS`] model steps, or earlier on a final answer or budget
    /// exhaustion. Every branch lands in a [`LoopOutcome`] (no panic, no hang).
    pub fn run(&self, reasoner: &dyn AgentReasoner, user_request: &str) -> LoopOutcome {
        // Discovery tier under the grant. The reasoner only ever sees granted tools.
        let available = self.mcp.list_tools();
        let mut log: Vec<LoopLogEntry> = Vec::new();
        let mut ctx = ReasonerContext {
            user_request,
            available_tools: &available,
            history: &log,
        };

        for iteration in 1..=MAX_AGENT_ITERATIONS {
            // FAIL-CLOSED 3: budget exhaustion terminates the loop. Debit one unit;
            // on refusal, return the partial log (never present it as an answer).
            if !self.budget.try_acquire(1.0) {
                return LoopOutcome::AssistantUnavailable {
                    reason: "budget exhausted".to_string(),
                    log,
                };
            }

            // ITEM 59 — agent-turn timing closure (gap G12). Each loop iteration IS one
            // agent turn; wrap it in the kernel's existing FDR span machinery so the
            // turn's wall-clock duration (Δwall) is recorded as a `SpanClose` FDR record.
            // This is the only timed path in the kernel; the executor previously bypassed
            // it. The span is inert unless a span observer is installed, and takes NO
            // clock on `wasm32` (where `Instant::now` would panic) — pure P3 telemetry,
            // never a loop-control input (MANIFESTO C2: no clock in the decision path).
            let _turn_span = crate::fdr::info_span!("agent_turn").entered();

            let step = reasoner.next(&ctx);
            match step {
                AgentStep::Answer { text } => {
                    log.push(LoopLogEntry {
                        iteration,
                        event: LoopEventKind::ModelReply {
                            content: text.clone(),
                            total_tokens: 0,
                        },
                    });
                    return LoopOutcome::Answer { text, log };
                }
                AgentStep::CallTool { name, raw_arg } => {
                    log.push(LoopLogEntry {
                        iteration,
                        event: LoopEventKind::ToolCallParsed {
                            tool_name: name.clone(),
                            raw_arg: raw_arg.clone(),
                        },
                    });

                    // Route through the capability-scoped MCP boundary (ports::mcp).
                    // FAIL-CLOSED 1 (no tool runs without a verified capability):
                    //   `McpPort::call_tool` returns ScopeDenied BEFORE invoking.
                    // FAIL-CLOSED 2 (unknown tool rejected): returns UnknownTool.
                    let req = McpToolCall {
                        name: name.clone(),
                        raw_arg: raw_arg.clone(),
                    };
                    match self.mcp.call_tool(&req) {
                        Ok(McpToolResult::Ok { content }) => {
                            log.push(LoopLogEntry {
                                iteration,
                                event: LoopEventKind::ToolResult {
                                    tool_name: name.clone(),
                                    output: content,
                                },
                            });
                        }
                        Ok(McpToolResult::ToolError { message }) => {
                            log.push(LoopLogEntry {
                                iteration,
                                event: LoopEventKind::ToolFailed {
                                    tool_name: name.clone(),
                                    error: message,
                                },
                            });
                        }
                        Err(McpServeError::UnknownTool { tool }) => {
                            // A name not in the registry: logged as a malformed call,
                            // never a crash, never a default tool.
                            log.push(LoopLogEntry {
                                iteration,
                                event: LoopEventKind::ToolCallMalformed {
                                    raw: name.clone(),
                                    reason: format!("unknown tool: {tool}"),
                                },
                            });
                        }
                        Err(McpServeError::ScopeDenied { tool }) => {
                            // FAIL-CLOSED 1 evidence: the gate refused BEFORE the tool
                            // body ran. Rendered as an observation, loop continues.
                            log.push(LoopLogEntry {
                                iteration,
                                event: LoopEventKind::ToolFailed {
                                    tool_name: tool,
                                    error: "scope denied: no verified capability".to_string(),
                                },
                            });
                        }
                        Err(McpServeError::Tool(message)) => {
                            // A tool-level failure surfaced as a typed observation
                            // (MCP spec: tool errors ride results, never crash).
                            log.push(LoopLogEntry {
                                iteration,
                                event: LoopEventKind::ToolFailed {
                                    tool_name: name.clone(),
                                    error: message,
                                },
                            });
                        }
                        Err(McpServeError::Unauthorized) => {
                            // Only reachable via `from_verified_capability`; defensive
                            // here (the loop builds its McpPort via `new`).
                            return LoopOutcome::AssistantUnavailable {
                                reason: "capability unauthorized".to_string(),
                                log,
                            };
                        }
                    }
                }
                AgentStep::FanOut { tasks } => {
                    // Fan-out: log each sub-task as a parsed call. The actual dispatch
                    // is the swarm coordinator's responsibility — the loop only records
                    // the intent. Each sub-task becomes a ToolCallParsed observation.
                    for task in &tasks {
                        log.push(LoopLogEntry {
                            iteration,
                            event: LoopEventKind::ToolCallParsed {
                                tool_name: task.name.clone(),
                                raw_arg: format!("[fanout:{}] {}", task.id, task.raw_arg),
                            },
                        });
                    }
                }
                AgentStep::Merge { task_ids, strategy } => {
                    // Merge: log the merge intent. The actual result combination
                    // is the coordinator's responsibility — the loop records the signal.
                    log.push(LoopLogEntry {
                        iteration,
                        event: LoopEventKind::ToolCallParsed {
                            tool_name: "merge".to_string(),
                            raw_arg: format!(
                                "ids={:?} strategy={:?}",
                                task_ids, strategy
                            ),
                        },
                    });
                }
            }

            // Re-bind the context for the next iteration (immutable borrow of `log`
            // ended with the `next` call above).
            ctx = ReasonerContext {
                user_request,
                available_tools: &available,
                history: &log,
            };
        }

        LoopOutcome::IterationCapExceeded { log }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::tool::{
        StaticSkillRegistry, ToolAction, ToolError, ToolInvocation, ToolOutput, ToolPort,
        ToolResource, ToolScope, ToolSpec,
    };
    use std::cell::{Cell, RefCell};

    // A spy tool: records invocation count so we can prove the fail-closed gate runs
    // the tool body ZERO times on an unauthorized / unknown call. Uses a shared
    // `Rc<Cell>`-style counter via `RefCell` so the test can read it after `run`.
    struct SpyTool {
        spec: ToolSpec,
        invocations: std::rc::Rc<Cell<u32>>,
        // Optional: return an error instead of success (for the typed-error path).
        fail: RefCell<Option<ToolError>>,
    }
    impl ToolPort for SpyTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }
        fn invoke(
            &self,
            _granted: ToolScope,
            _inv: &ToolInvocation,
        ) -> Result<ToolOutput, ToolError> {
            self.invocations.set(self.invocations.get() + 1);
            if let Some(e) = self.fail.borrow_mut().take() {
                return Err(e);
            }
            Ok(ToolOutput {
                content: "IN_DELIVERY".to_string(),
            })
        }
    }

    fn order_status_scope() -> ToolScope {
        ToolScope {
            resource: ToolResource::OrderStatus,
            action: ToolAction::Read,
        }
    }

    fn build_registry(invocations: std::rc::Rc<Cell<u32>>) -> StaticSkillRegistry {
        let spec = ToolSpec {
            name: "read_order_status",
            description: "Read the status of an order.",
            arg_name: "order_id",
            scope: order_status_scope(),
        };
        let tool = SpyTool {
            spec,
            invocations,
            fail: RefCell::new(None),
        };
        let card = crate::ports::tool::SkillCard {
            name: "read_order_status",
            description: "Read the status of an order.",
            surface: Surface::Owner,
            scope: order_status_scope(),
        };
        StaticSkillRegistry::new(vec![(card, Box::new(tool))])
    }

    fn covering_grant() -> GrantSet {
        GrantSet::new(vec![order_status_scope()])
    }

    // A scripted reasoner: replays a fixed list of steps, then answers "done".
    struct Scripted {
        steps: Vec<AgentStep>,
        idx: Cell<usize>,
    }
    impl AgentReasoner for Scripted {
        fn next(&self, _ctx: &ReasonerContext) -> AgentStep {
            let i = self.idx.get();
            self.idx.set(i + 1);
            self.steps.get(i).cloned().unwrap_or(AgentStep::Answer {
                text: "done".to_string(),
            })
        }
    }

    // A reasoner that ALWAYS calls the (authorized) tool — used to drive the loop to
    // its iteration cap / budget drain (no final answer is ever produced).
    struct AlwaysTool;
    impl AgentReasoner for AlwaysTool {
        fn next(&self, _ctx: &ReasonerContext) -> AgentStep {
            AgentStep::CallTool {
                name: "read_order_status".to_string(),
                raw_arg: "ord-7".to_string(),
            }
        }
    }

    fn scripted(steps: Vec<AgentStep>) -> Scripted {
        Scripted {
            steps,
            idx: Cell::new(0),
        }
    }

    #[test]
    fn happy_path_tool_result_then_answer() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(16.0, 1.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        let reasoner = scripted(vec![AgentStep::CallTool {
            name: "read_order_status".to_string(),
            raw_arg: "ord-7".to_string(),
        }]);
        let outcome = loop_.run(&reasoner, "status of ord-7?");

        match outcome {
            LoopOutcome::Answer { text, log } => {
                assert_eq!(text, "done");
                // Tool was invoked exactly once.
                assert_eq!(inv.get(), 1);
                assert!(log
                    .iter()
                    .any(|e| matches!(e.event, LoopEventKind::ToolResult { .. })));
            }
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    // FAIL-CLOSED 2: an unknown tool name is rejected; the real tool body runs ZERO
    // times (the spy proves it).
    #[test]
    fn unknown_tool_is_rejected_and_runs_nothing() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(16.0, 1.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        let reasoner = scripted(vec![
            AgentStep::CallTool {
                name: "transfer_money".to_string(),
                raw_arg: "{}".to_string(),
            },
            AgentStep::Answer {
                text: "cannot do that".to_string(),
            },
        ]);
        let outcome = loop_.run(&reasoner, "send money");

        match outcome {
            LoopOutcome::Answer { text, log } => {
                assert_eq!(text, "cannot do that");
                // The REAL tool was never invoked.
                assert_eq!(inv.get(), 0, "unknown tool must never reach the tool body");
                assert!(log
                    .iter()
                    .any(|e| matches!(e.event, LoopEventKind::ToolCallMalformed { .. })));
            }
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    // FAIL-CLOSED 1: under an EMPTY grant (no verified capability), a tool call is
    // refused with ScopeDenied BEFORE the tool body runs — spy count stays 0.
    #[test]
    fn no_tool_runs_without_verified_capability() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(16.0, 1.0);
        // Empty grant: authorizes nothing.
        let loop_ = AgentLoop::new(reg, GrantSet::empty(), Surface::Owner, budget);

        // Discovery leaks nothing under an empty grant.
        assert!(loop_.available_tools().is_empty());

        let reasoner = scripted(vec![
            AgentStep::CallTool {
                name: "read_order_status".to_string(),
                raw_arg: "ord-7".to_string(),
            },
            AgentStep::Answer {
                text: "no capability".to_string(),
            },
        ]);
        let outcome = loop_.run(&reasoner, "status of ord-7?");

        match outcome {
            LoopOutcome::Answer { text, log } => {
                assert_eq!(text, "no capability");
                assert_eq!(
                    inv.get(),
                    0,
                    "tool must NOT run without a verified capability"
                );
                assert!(log
                    .iter()
                    .any(|e| matches!(e.event, LoopEventKind::ToolFailed { .. })));
            }
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    // FAIL-CLOSED 3: a drained budget terminates the loop immediately as
    // AssistantUnavailable, with an empty log (no step ran).
    // ITEM 59 — agent-turn timing closure (gap G12): the loop MUST emit a per-turn
    // `agent_turn` `SpanClose` FDR record (the kernel's only timed path). This is the
    // falsifiable RED→GREEN proof: install a scoped span observer, run the loop, and
    // assert the observer received a span-close named exactly `agent_turn` — one per
    // turn driven. Without the instrument this assertion fails (RED); with it, GREEN.
    #[test]
    fn agent_turn_emits_timing_span_close() {
        use crate::fdr::SpanObserver;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;

        struct TurnObs {
            span: &'static str,
            hits: Arc<AtomicU64>,
        }
        impl SpanObserver for TurnObs {
            fn on_span_close(&self, name: &'static str, _dur_us: u64) {
                if name == self.span {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        let hits = Arc::new(AtomicU64::new(0));
        let _guard = crate::fdr::set_scoped_observer(Arc::new(TurnObs {
            span: "agent_turn",
            hits: hits.clone(),
        }));

        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(16.0, 1.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        // Two turns: one tool call, then a final answer.
        let reasoner = scripted(vec![AgentStep::CallTool {
            name: "read_order_status".to_string(),
            raw_arg: "ord-7".to_string(),
        }]);
        let _outcome = loop_.run(&reasoner, "status of ord-7?");

        // The loop drove exactly two turns (the tool call, then the default final
        // answer), so exactly two `agent_turn` span-closes must have been recorded.
        assert_eq!(
            hits.load(Ordering::Relaxed),
            2,
            "agent loop must emit exactly one `agent_turn` span close per turn driven"
        );
    }

    #[test]
    fn budget_exhaustion_terminates_loop() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        // Capacity 0 → first debit fails.
        let budget = TokenBucket::new(0.0, 0.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        let reasoner = scripted(vec![AgentStep::CallTool {
            name: "read_order_status".to_string(),
            raw_arg: "ord-7".to_string(),
        }]);
        let outcome = loop_.run(&reasoner, "status of ord-7?");

        match outcome {
            LoopOutcome::AssistantUnavailable { reason, log } => {
                assert_eq!(reason, "budget exhausted");
                assert!(log.is_empty(), "no step may run once the budget is dry");
                assert_eq!(inv.get(), 0, "tool must not run under a dry budget");
            }
            other => panic!("expected AssistantUnavailable, got {other:?}"),
        }
    }

    // A mid-run budget drain attaches the partial log rather than presenting an answer.
    #[test]
    fn budget_drain_midrun_attaches_partial_log() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        // Capacity 2 → exactly 2 iterations succeed, 3rd is refused.
        let budget = TokenBucket::new(2.0, 0.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        // Reasoner always asks for the (authorized) tool.
        let reasoner = AlwaysTool;
        let outcome = loop_.run(&reasoner, "status");

        match outcome {
            LoopOutcome::AssistantUnavailable { reason, log } => {
                assert_eq!(reason, "budget exhausted");
                // Two tool calls succeeded before the drain.
                assert_eq!(inv.get(), 2);
                assert_eq!(log.len(), 4, "2 parsed + 2 result events");
            }
            other => panic!("expected AssistantUnavailable, got {other:?}"),
        }
    }

    // The iteration ceiling is a hard invariant: a reasoner that never answers stops
    // exactly at MAX_AGENT_ITERATIONS (never a silent truncation, never unbounded).
    #[test]
    fn iteration_cap_is_hard_invariant() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(1024.0, 1.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);

        // Reasoner always calls the authorized tool → loop must cap, not run forever.
        let reasoner = AlwaysTool;
        let outcome = loop_.run(&reasoner, "status");

        match outcome {
            LoopOutcome::IterationCapExceeded { log } => {
                let parsed: usize = log
                    .iter()
                    .filter(|e| matches!(e.event, LoopEventKind::ToolCallParsed { .. }))
                    .count();
                assert_eq!(parsed as u8, MAX_AGENT_ITERATIONS);
                assert_eq!(inv.get(), MAX_AGENT_ITERATIONS as u32);
            }
            other => panic!("expected IterationCapExceeded, got {other:?}"),
        }
    }

    // A typed tool error is a visible observation, never swallowed.
    #[test]
    fn tool_error_is_visible_observation() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let _reg = build_registry(inv.clone());
        // Make the spy fail with NotFound on its first (only) call.
        // Reach into the registry is not possible; instead craft a failing tool
        // directly via a second registry builder path.
        let spec = ToolSpec {
            name: "read_order_status",
            description: "Read the status of an order.",
            arg_name: "order_id",
            scope: order_status_scope(),
        };
        struct FailingTool {
            spec: ToolSpec,
        }
        impl ToolPort for FailingTool {
            fn spec(&self) -> &ToolSpec {
                &self.spec
            }
            fn invoke(
                &self,
                _g: ToolScope,
                _inv: &ToolInvocation,
            ) -> Result<ToolOutput, ToolError> {
                Err(ToolError::NotFound("ord-9".to_string()))
            }
        }
        let card = crate::ports::tool::SkillCard {
            name: "read_order_status",
            description: "Read the status of an order.",
            surface: Surface::Owner,
            scope: order_status_scope(),
        };
        let failing_reg = StaticSkillRegistry::new(vec![(card, Box::new(FailingTool { spec }))]);
        let budget = TokenBucket::new(16.0, 1.0);
        let loop_ = AgentLoop::new(failing_reg, covering_grant(), Surface::Owner, budget);

        let reasoner = scripted(vec![
            AgentStep::CallTool {
                name: "read_order_status".to_string(),
                raw_arg: "ord-9".to_string(),
            },
            AgentStep::Answer {
                text: "order not found".to_string(),
            },
        ]);
        let outcome = loop_.run(&reasoner, "status of ord-9?");

        match outcome {
            LoopOutcome::Answer { text, log } => {
                assert_eq!(text, "order not found");
                assert!(log
                    .iter()
                    .any(|e| matches!(e.event, LoopEventKind::ToolFailed { .. })));
                // The failing tool WAS invoked (the error is from inside the gate).
                let _ = inv;
            }
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    // A tool-less answer works even under an empty grant (the loop does not forbid
    // answering — it only refuses TOOL calls without a verified capability).
    #[test]
    fn answer_without_tools_under_empty_grant() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(16.0, 1.0);
        let loop_ = AgentLoop::new(reg, GrantSet::empty(), Surface::Owner, budget);

        let reasoner = scripted(vec![AgentStep::Answer {
            text: "hello".to_string(),
        }]);
        let outcome = loop_.run(&reasoner, "hi");
        match outcome {
            LoopOutcome::Answer { text, .. } => assert_eq!(text, "hello"),
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    // The bounded cycle is finite by construction: even an always-tool reasoner with an
    // infinite budget terminates at the cap (no supervisor, no hang).
    #[test]
    fn always_tool_reasoner_terminates() {
        let inv = std::rc::Rc::new(Cell::new(0u32));
        let reg = build_registry(inv.clone());
        let budget = TokenBucket::new(f64::INFINITY, 0.0);
        let loop_ = AgentLoop::new(reg, covering_grant(), Surface::Owner, budget);
        let reasoner = AlwaysTool;
        let start = std::time::Instant::now();
        let outcome = loop_.run(&reasoner, "x");
        assert!(
            start.elapsed().as_secs() < 5,
            "loop must terminate promptly"
        );
        assert!(matches!(outcome, LoopOutcome::IterationCapExceeded { .. }));
    }
}
