# BLUEPRINT P40 — Agent executor → product wiring (2026-07-19)

> **Standalone wiring blueprint** (`agent-loop` + `tools/native-spa-server`, dowiz-side only — does NOT touch bebop).
> This blueprint FILLS the gap flagged by the Wave-1 recon: `agent-loop/src/lib.rs::AgentLoop::run`
> (bounded, fail-closed executor) has **zero product callers** — only `agent-loop/src/main.rs` (a
> smoke binary). The operator's live directive: *red lines are autopilot only via exact blueprints*;
> no `BLUEPRINT-P40` exists on disk, so this document is authored FIRST and the implementation is
> gated on operator confirmation. It is the spec; Wave-3 executes it in an isolated worktree.
>
> **One sentence:** wire the verified `AgentLoop` (kernel `agent/loop.rs` via the `agent-facade`
> port) into the one native runtime that can own a session — `native-spa-server`'s `/api/agent`
> route — behind the SAME `verify_chain` capability-cert middleware the order API already uses,
> with the P54 money-law firewall + P42 tool boundary enforced by grep gates, and bounded
> termination guaranteed by `MAX_AGENT_ITERATIONS` / `TokenBucket`.

---

## 0. Ground truth — every cite re-verified live this pass (2026-07-19)

Working tree `/root/dowiz`, branch `main` (`7ec18dc3c`), read from live files this pass.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| `AgentLoop` executor landed, bounded, fail-closed; `MAX_AGENT_ITERATIONS: u8 = 8` | `kernel/src/agent/loop.rs:40` | verified |
| `AgentLoop::run(&self, reasoner: &dyn AgentReasoner, user_request: &str) -> LoopOutcome` | `kernel/src/agent/loop.rs` (`run` in `agent-loop/src/lib.rs:89`) | verified |
| Reasoning seam abstract: `pub trait AgentReasoner { fn next(&self, ctx: &ReasonerContext) -> AgentStep; }` | `kernel/src/agent/loop.rs:112` | verified |
| `LoopOutcome::{Answer, AssistantUnavailable, IterationCapExceeded}` — every path typed | `kernel/src/agent/loop.rs:70-86` | verified |
| Tool boundary closed-enum: `ToolResource{OrderStatus}`, `ToolAction{Read}`, `GrantSet.covers()` first | `kernel/src/ports/tool.rs:26,34,67` | verified |
| `McpPort<R>::call_tool` rejects unknown/scope-less BEFORE body | `kernel/src/ports/mcp.rs:144,198,221` | verified |
| Money law integer, fail-closed: `apply_tax` / `decide` / `fold_transitions` | `kernel/src/money.rs:270`, `kernel/src/decision/mod.rs:257`, `kernel/src/order_machine.rs:156` | verified |
| `agent-loop` crate depends ONLY `agent-facade` + `llm-adapters`; **firewall test greps Cargo.toml for `dowiz-kernel`** | `agent-loop/Cargo.toml:10-16` + `agent-loop/tests/firewall.rs` | verified |
| `llm-adapters` provides `OllamaAdapter: LlmBackend` (and `FakeBackend` for tests) | `llm-adapters/src/ollama.rs:20,84` + `llm-adapters/src/dispatch.rs:248` | verified |
| `native-spa-server` already deps `dowiz-kernel` (json-api), has `api::build_api_router(ApiState)` + cap middleware reusing `verify_chain` | `tools/native-spa-server/Cargo.toml:38` + `src/api.rs:34,110` | verified |
| `native-spa-server` is the "zero-OCI" static+order server; `api` module is a THIN SHELL (grep gate `r11_thin_shell_grep_gate`) | `tools/native-spa-server/src/api.rs:9-16` | verified |
| `native-spa-server` currently has **no** agent route; only `ROUTE_ORDER_*` + `/healthz` | `tools/native-spa-server/src/api.rs:49-52` | verified |
| `LlmBackend` trait is the agent's only model port; `OllamaAdapter` is the product backend | `kernel/src/ports/llm.rs` + `llm-adapters/src/ollama.rs` | verified |

**Ground truth is non-discussible; everything below builds on this table only.**

---

## 1. Scope — what this blueprint owns vs must NOT do

**P40 owns:**

| Item | Content |
|---|---|
| W-a | **The exact product caller:** a new `POST /api/agent` route in `native-spa-server/src/api.rs` that constructs an `AgentLoop` over an `LlmBackend` and drives one bounded turn per request. |
| W-b | **Cap-gate reuse:** the route is layered under the EXISTING `verify_chain` capability-cert middleware (same `SignedFrame`/`AnchorRoster`/`RevocationSet` the order API uses) — no second auth path. |
| W-c | **Bounded termination guarantee:** `AgentLoop::run` already bounds to `MAX_AGENT_ITERATIONS=8` and `TokenBucket`; the route returns the `LoopOutcome` as JSON (Answer text / AssistantUnavailable / IterationCapExceeded) — never hangs. |
| W-d | **Two firewall grep gates** (committed in-repo): (1) P41 — `agent-loop` must NOT gain a `dowiz-kernel` dependency (its `tests/firewall.rs` already enforces this; keep it); (2) P54 — the agent lane must contain NO `apply_tax` / `money::` / `::decide` / `fold_transitions` symbol (the money law is kernel-only) — add a grep gate on `tools/native-spa-server` + `agent-loop`. |
| W-e | **One red→green test:** `tools/native-spa-server/tests/agent_route.rs` proving the route boots, runs one `AgentLoop` turn (over a scripted/fake `LlmBackend` or the real `OllamaAdapter` if a daemon is present), returns a typed `LoopOutcome`, enforces the cap middleware (unsigned frame → 401), and that the money-law grep gate holds. |

**P40 explicitly does NOT do (anti-scope):**

1. **NOT a new auth policy / money movement.** It reuses the existing `verify_chain` cap middleware and the existing kernel money law by *firewall* (the agent literally cannot name money symbols). No new capability schema, no new settlement path.
2. **NOT touching `kernel/src/agent/loop.rs`** — the executor is correct and verified; this blueprint only *calls* it. A change to the loop is a mis-designed wire.
3. **NOT adding a `dowiz-kernel` dependency to `agent-loop`** — the P41 compile firewall stays; `agent-loop` talks to the model via `LlmBackend` (`llm-adapters`), never the kernel directly.
4. **NOT a resident/autonomous loop.** Every agent turn is operator-requested (one HTTP POST = one bounded turn). The feedback output is advisory; it never gates the deterministic core.
5. **NOT an LLM judge / self-certification.** The route returns the loop's typed outcome; grading (if any) is a separate harness (P54).

---

## 2. DECART — the new-dependency decision (Integration Decart Rule applies)

Adding an agent endpoint to `native-spa-server` is a **new integration** (new dep on `agent-loop` + `llm-adapters`, which pulls an HTTP client into the previously zero-OCI binary). The rule mandates an honest comparison before adoption.

| Candidate | Bare-metal fit | Falsifiable correctness | Perf | Supply-chain/license | Maintainability | Reversibility | Verdict |
|---|---|---|---|---|---|---|---|
| **A. `native-spa-server` depends on `agent-loop`+`llm-adapters` directly** (in-process agent) | Tightest: one binary serves static + order + agent. But `llm-adapters` pulls `reqwest`/HTTP into the zero-OCI artifact — breaks the DK-04 "static-only, no runtime deps" promise. | Reuses verified `AgentLoop`; cap-gate reuse is clean. | One process; no IPC latency. | Adds a heavy HTTP client + TLS dep to the SPA binary → supply-chain surface grows. | Simplest deploy. | Hard to back out (binary already shipped with it). | **Rejected as default** |
| **B. `native-spa-server` proxies `/api/agent` to a separate `agent-loop` service** (keep SPA server zero-OCI; agent runs in its own crate/process) | Preserves DK-04 zero-OCI invariant; agent is a sibling service. | Same `AgentLoop`; proxy is a thin forward. | One extra hop (localhost). | SPA binary stays lean; agent deps isolated to the agent process. | Two deployments to manage. | Trivially reversible (drop the proxy route). | **RECOMMENDED** |
| **C. agent-loop as a wasm component invoked in-process** | Over-engineering for a server endpoint. | Unknown porting cost. | n/a | New wasm runtime dep. | High. | Hard. | Rejected |

**DECISION:** **Candidate B (proxy to a sibling `agent-loop` service)** — OPERATOR-CONFIRMED (2026-07-19). Preserves the DK-04 zero-OCI property, isolates the heavy `llm-adapters` dependency to the agent process, and is trivially reversible (drop the proxy route). Choice A (in-process dep) was explicitly rejected by the operator after a brief A→B correction; the grep gates (W-d) are identical either way, but B keeps the shipped SPA binary lean. This blueprint specifies B.

**Mandatory probe (strongest argument against B):** a proxy adds one localhost hop and a second deployable unit. Answer: the hop is localhost (<1 ms), and the operator's own "deliberately last / zero deployability" audit finding (ARCHITECT-whole-system-hostile) favours *more* cleanly-separated services over a monolith that can't ship. B is the smaller risk.

---

## 3. Predefined types & constants (named BEFORE implementation)

```rust
// ── tools/native-spa-server/src/api.rs (ADD) ──
pub const ROUTE_AGENT: &str = "/api/agent";

/// Request body for one agent turn. The agent has NO money tool, so `request`
/// must never carry an order total / payment instruction the kernel would act on.
#[derive(Deserialize)]
pub struct AgentRequest { pub prompt: String }

/// Response: the typed LoopOutcome, serialized so the caller sees the bound.
#[derive(Serialize)]
pub struct AgentResponse {
    pub outcome: String,        // "answer" | "unavailable" | "cap_exceeded"
    pub text: Option<String>,   // present on Answer
    pub log: Vec<String>,       // the LoopLogEntry sequence (event-driven, debug-friendly)
}
```

---

## 4. Build items — spec → RED test → code (items 3, 5)

### 4.1 W-a — the `/api/agent` route (product caller)

- **Spec:** add `POST /api/agent` to `api::build_api_router`, gated by the existing cap middleware (`verify_chain`). Handler:
  1. Authenticate the `x-dowiz-cap` frame (reuse `verify_chain` + `ApiState` roster/revocation).
  2. Construct `AgentLoop::new(backend, tool, granted)` where `backend: &dyn LlmBackend` is the product `OllamaAdapter` (or a configured backend), `tool: &dyn ToolPort` is a grant-filtered `McpPort`, `granted: ToolScope` is read from the verified cap.
  3. `let outcome = agent.run(&prompt);` — ONE bounded turn.
  4. Serialize `LoopOutcome` to `AgentResponse`; return 200.
- **RED `red_agent_route_absent`:** today `tools/native-spa-server` has no `/api/agent` handler → `POST /api/agent` 404s. RED now; GREEN once the route returns a typed `AgentResponse`.
- **Adversarial `red_agent_unauthenticated_rejected`:** a `POST /api/agent` with NO/forged `x-dowiz-cap` frame → 401 (the reused `verify_chain` middleware, never a bypass).

### 4.2 W-d — the two firewall grep gates

- **Spec:** (1) keep `agent-loop/tests/firewall.rs` (`dowiz-kernel` must be absent from `agent-loop/Cargo.toml`); (2) add `tools/native-spa-server/tests/money_law_firewall_grep.rs` asserting the agent lane (`agent-loop/src` + the new `/api/agent` route body) contains **no** `apply_tax` / `money::` / `::decide` / `fold_transitions` symbol — the money law is kernel-only by construction.
- **RED `red_money_law_firewall_present`:** the grep gate does not exist → a future PR could sneak `use kernel::money::apply_tax` into the agent lane. RED now; GREEN once the gate is a committed `cargo test`.
- **Adversarial `red_money_tool_absent`:** assert `ToolResource` in the agent's granted catalog has NO `PriceQuote`/money variant (P54 prong 1) — proven by the closed-enum `ToolResource{OrderStatus}` only.

### 4.3 W-e — the red→green integration test

- **Spec:** `tools/native-spa-server/tests/agent_route.rs`: boot the router with a scripted/fake `LlmBackend` (reuse `llm-adapters`' `FakeBackend` if exported, else a tiny scripted `LlmBackend`), POST a valid cap-framed `/api/agent` request, assert 200 + `AgentResponse.outcome == "answer"` + bounded (no hang, test times out otherwise). Also assert unsigned → 401, and that the money-law grep gate holds.
- **RED `red_agent_turn_bounded`:** before wiring, the route 404s and no `AgentLoop` runs; after, one POST drives exactly one bounded turn and returns within the test's wall-time budget (proves `MAX_AGENT_ITERATIONS` + `TokenBucket` bound it).
- **Adversarial `red_agent_no_money_reach`:** a prompt that *demands* the agent compute a total → the response text may comply in prose but the grep gate + closed tool enum prove no decision path consumed it (P54 prong 1/2).

---

## 5. DoD — falsifiable, RED→GREEN, machine-checkable

| # | Done when… | Falsifier (check) |
|---|---|---|
| D1 | `POST /api/agent` exists, returns a typed `LoopOutcome` | `red_agent_route_absent` |
| D2 | unsigned/forged cap → 401 (reused `verify_chain`) | `red_agent_unauthenticated_rejected` |
| D3 | one bounded turn per request, no hang | `red_agent_turn_bounded` |
| D4 | `agent-loop` still has NO `dowiz-kernel` dep | `agent-loop/tests/firewall.rs` |
| D5 | agent lane has NO money symbol (kernel owns money) | `red_money_law_firewall_present` + `red_money_tool_absent` |
| D6 | DECART decision (B proxy, or A if operator overrides) recorded + reversible | the §2 table in this file; the proxy route is removable without touching kernel |

---

## 6. Benchmarks + telemetry (standard item 10)

- The agent turn already emits `dowiz_agent_*` metric IDs into `track_record.jsonl` (per `agent-loop/src/main.rs` harvest pattern) — this blueprint reuses that; no new metric system.
- Required measured number: the route's per-request latency = one `AgentLoop::run` wall time (already bounded by `MAX_AGENT_ITERATIONS × step_cost`); recorded in the test, not asserted.

---

## 7. Cross-cutting obligations (standard items 6, 11, 13, 20)

- **Hazard-safety (item 6):** the only new hazard is an unbounded agent turn — made unrepresentable by `MAX_AGENT_ITERATIONS=8` (const, reviewed) + `TokenBucket` degrade-close, both already in the executor. The route adds no loop.
- **Isolation / bulkhead (item 11):** under DECART-B, the agent runs in a sibling process; `native-spa-server` is a thin proxy, so an agent panic cannot take down order serving. Under DECART-A (if chosen), the `TokenBucket` + iteration cap still contain a runaway turn.
- **Rollback/self-heal as math (item 13):** removing the `/api/agent` route (DECART-B) is a one-route deletion; the kernel money law and executor are untouched. Self-termination = the iteration cap.
- **Hermetic principles (item 20):** Cause & Effect — a request causes exactly one bounded turn; Correspondence — `AgentResponse.outcome` corresponds exactly to `LoopOutcome`.

---

## 8. For the worker (exact acceptance path, after operator confirmation)

1. DECART choice: **B (confirmed by operator)**. Stand up `agent-loop` as a runnable sibling service (its existing `src/main.rs` already runs one turn over `OllamaAdapter` — extend to listen on a localhost socket, or have `native-spa-server` forward). Do NOT add `agent-loop`/`llm-adapters` to `native-spa-server/Cargo.toml` (that is rejected choice A).
2. Add `ROUTE_AGENT` + handler in `tools/native-spa-server/src/api.rs`, gated by existing `verify_chain` middleware.
3. Add `tools/native-spa-server/tests/agent_route.rs` (RED first: 404 / no turn; GREEN: 200 + bounded + 401 on unsigned).
4. Add `tools/native-spa-server/tests/money_law_firewall_grep.rs` (D5).
5. Keep `agent-loop/tests/firewall.rs` green (D4).
6. Run `cd tools/native-spa-server && cargo test` (both the route test AND the grep gate) — evidence required.
7. Do NOT push. Leave the worktree for lead review + commit.
