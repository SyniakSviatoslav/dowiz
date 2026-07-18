# BLUEPRINT P40 — AgentLoop executor + tool-calling capability wiring (2026-07-18)

> **STATUS CORRECTION (2026-07-18, later same day — consistency pass):** no longer purely
> planned. The wave swarm landed a real `AgentLoop` executor: `kernel/src/agent/loop.rs`
> (651 lines) + `kernel/src/agent/mod.rs`, fail-closed, commits `626236886`/`e25e9fed8`;
> `kernel/src/ports/tool.rs` (ToolPort) landed with the P42 wave (`575a75a20`). P40 is
> **PARTIAL**. The swarm may have implemented details differently than designed below —
> reconciling design-vs-implementation is deliberately NOT done in this note.

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 below — every point
> addressed, none skipped). This blueprint deepens the roadmap-index DoD for **P40** in
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.4 (lines 989-1000) to
> the standard's depth — it extends that DoD, it does not contradict it. Structure/depth template:
> `BLUEPRINT-P-A-kernel-primitives.md` (same directory). The backend layer this loop sits on is
> **shipped and not re-designed here**: `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md`
> is the port/adapter/firewall convention source, reused verbatim (its §2.2 Quirks item 5 already
> anticipates tool-calling as a `Caps` probe; its §3.2 already anticipates a `tools` field in the
> cache key — this blueprint builds exactly those two anticipated seams plus the loop that
> document explicitly does not contain).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree `/root/dowiz`, branch `main` (`f9b2eb9bb`), 2026-07-18. Every row below was read
from the live file this session, not inherited.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| `LlmBackend` trait: `id`/`caps`/`chat`/`embed`/`rerank`/`health`, sync `&self`, typed `Result` | `kernel/src/ports/llm.rs:154-169` | verified |
| `Caps { chat, embed, rerank, tool_calling }`, doc: "fail-closed feature discovery" | `kernel/src/ports/llm.rs:17-23` | verified |
| `Caps.tool_calling` HARD-PINNED `false` for Ollama ("Rerank/tool-calling not assumed") | `llm-adapters/src/ollama.rs:53-61` (the pin is `:59`) | verified — the exact gap DoD-2 opens |
| `LlmError { Unavailable, Unsupported, BadRequest(String), Timeout }` | `kernel/src/ports/llm.rs:140-150` | verified |
| `ChatRequest` has NO `tools` field; `ChatResponse` has NO `tool_calls` field | `kernel/src/ports/llm.rs:44-59` / `:105-109` | verified — the wire seam §3.3 adds |
| Exact-match cache key = BTreeMap-canonical over the CURRENT field set; `grep -n tools llm-adapters/src/cache.rs` → **0 hits** | `llm-adapters/src/cache.rs:55-60` (`cache_key`) | verified — **live drift vs HARNESS doc §3.2**, which specs `tools` in the key. Real cache-poisoning hazard if §3.3 lands without §3.3's key extension |
| `AgentLoop` / any plan→act→observe executor: **0 grep hits** repo-wide (`grep -rn "AgentLoop\|agent_loop" --include="*.rs" .` → empty) | — | verified this pass — the gap this blueprint fills |
| `Harness<S>` composition surface: `chat` (via `Dispatcher`, returns `DispatchError`), `embed`/`rerank`/`health`/`caps` direct | `llm-adapters/src/compose.rs:39-70` | verified |
| `StackBuilder` defaults: Ollama local, `workers: 2`, `capacity: 64`, `refill_rate: 8.0`, cache on | `llm-adapters/src/compose.rs:75-95` | verified |
| `DispatchError { BudgetExceeded, Backend(LlmError) }` + `TrackRecord` harvest row | `llm-adapters/src/dispatch.rs:23-29`, `:37-45` | verified |
| Model routing: `TaskClass::General → "llama3.1:8b"` | `llm-adapters/src/ollama.rs:36-45` | verified |
| `AgentBridge` = a DIFFERENT thing (PROTOCOL's mesh foreign-agent admission/caging, B1) | `kernel/src/ports/agent/mod.rs:1-24` (port), `agent-adapters/src/lib.rs:1-25` (MCP-server bridge for FOREIGN agents) | verified — zero code links it to `LlmBackend`; §4.4 makes non-conflation a hard constraint |
| KernelFacade pattern (the compilation-firewall precedent §3.1 mirrors): "Exposes **exactly two** public methods" | `/root/bebop-repo/bebop2/proto-cap/src/facade.rs:123` (`pub fn submit_intent`) | verified — **DRIFTED** from the master roadmap's cite `facade.rs:64` (§10.3 item 5); pattern confirmed present, line moved |
| Offline-proof anchor: full order→delivery with zero peers | `/root/bebop-repo/bebop2/delivery-domain/src/intake.rs:408` (`ac6_solo_island_full_flow_no_peers`) | verified |
| `OrderStatus` enum, 12 variants incl. `Refunding`/`CompensatedRefund`; canonical string vocabulary via `from_str` | `kernel/src/order_machine.rs:8-25`, `:29-45` | verified — the one tool's output domain |
| CI: unconditional offline kernel+engine `cargo test` job exists; **no `cargo tree` firewall job exists anywhere in `ci.yml`** | `.github/workflows/ci.yml:107-120`; `grep -n "cargo tree" .github/workflows/*.yml` → 0 hits | verified — P41 owns adding it; P40's firewall check lands as a committed red-proof + test first |
| Ollama daemon: v0.30.9, `http://127.0.0.1:11434`, `llama3.1:8b` resident | `HARNESS-LLM-BACKEND.md` §1.2 (live-probed there) | inherited cite, host facts — re-probe in T1 before relying on it |
| `Quirks::managed_api(api_key)` exists (bearer-auth managed profile) | `llm-adapters/src/quirks.rs:69-70` | verified — P41's connected half; cited here only for the shared-transport argument |
| Self-mod effector — a DIFFERENT kind of agent, out of scope | `/root/bebop-repo/bebop2/core/src/self_mod.rs` + `self_mod_loop.rs` (both exist on disk) | verified present; see §1 anti-scope note |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P40 owns vs what it must NOT touch

**P40 owns (build items §3):**

| Item | Content |
|---|---|
| B-a | `ToolPort` trait in the kernel-ports layer + the `agent-facade`/`agent-loop` crate pair behind a KernelFacade-style compilation firewall, with a committed red-proof |
| B-b | `Caps.tool_calling` un-pinned for Ollama via a live per-model probe (`/api/show` capabilities), fail-closed on any probe failure |
| B-c | `LlmBackend` wire extension: `ChatRequest.tools` + `ChatResponse.tool_calls` (plain structs, extend-don't-rewrite) **and** the mandatory cache-key extension the HARNESS doc §3.2 already specs |
| B-d | The `AgentLoop` executor (bounded plan→act→observe) + exactly ONE tool (`read_order_status`) proven end-to-end against local Ollama and one test order |
| B-e | Adversarial closure: malformed tool-call, tool error/timeout, iteration-cap, budget-exhaustion — each a typed outcome, never a crash or silent retry |

**P40 explicitly does NOT own (anti-scope, each a review-rejectable smell):**

1. **NOT a multi-tool framework.** No tool registry, no dynamic tool discovery, no second tool.
   Exactly one tool ships: read-order-status-by-id. A PR adding a second tool to P40 is out of
   scope by definition (P42 standardizes the pattern after it is proven on one tool).
2. **NOT write/mutating tools — structurally.** `ToolAction` (§2) has exactly one variant,
   `Read`. A write tool is not forbidden by review; it is **unrepresentable** in the type.
3. **NOT money/auth/RLS/migration tools, ever** — red-line class (memory
   `test-integrity-rules-2026-06-27`, `never-bypass-human-gates-2026-06-29`). No future phase
   inherits a hook for them from P40; there is no extension point that reaches those surfaces.
4. **NOT touching `AgentBridge`/`agent-adapters`.** `kernel/src/ports/agent/{admission,cap,manifest,scope}.rs`
   and `agent-adapters/src/{cache,dispatch,mcp,fuel,manifest,quirks,transport}.rs` are PROTOCOL's
   mesh foreign-agent admission/caging surface (B1, agentic-mesh arc). P40's agent is the LOCAL
   delivery-ops assistant over `LlmBackend`. Zero code links the two today and that separation is
   intentional (§10.5.4 naming discipline). §4.4 states the check that keeps it true.
5. **NOT a redesign of `LlmBackend`.** It is shipped; §3.3 extends it with two optional fields
   whose defaults keep every existing call site compiling and behaving identically.
6. **NOT streaming, NOT autonomy.** The loop executes one user-initiated request to completion
   and stops. It does not schedule itself, poll, or subscribe to anything.
7. **Out of scope, flagged once for awareness — self-mod effector (bebop-repo):**
   `bebop2/core/src/self_mod.rs` + `self_mod_loop.rs` are a code-self-modification actuator, not
   a delivery-ops assistant; dormant (called only from own unit tests); its header's
   "operator-authorized" claim is self-asserted, not independently corroborated. P40 builds
   nothing related to it and shares no code with it.

**Dependency posture (from §10.5.4, restated precisely):** DELIVERY's P37 (HTTP order surface,
being blueprinted in parallel) is P40's **soft** dependency — it supplies the *real* tool target
later. P40's DoD is satisfiable **now** from a stub/local target, because a minimal read-only
tool loop must function on a solo offline node (offline-first hard requirement, §10.3 item 2;
anchor: `ac6_solo_island_full_flow_no_peers`). The P37 seam is a named trait (§2
`OrderStatusSource`), not a blocked task. P40 blocks P41 (mode parity needs a loop to be parity
OF) and P42 (MCP re-exposes the tool port).

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

Everything new, declared up front. No magic numbers, no stringly-typed slots.

```rust
// ── kernel/src/ports/tool.rs — NEW module (B-a). Registered in ports/mod.rs beside
// `llm` and `agent` with a one-line doc, matching the existing convention
// (kernel/src/ports/mod.rs:1-9). ZERO network/HTTP/JSON/serde — mirrors llm.rs:1-7's
// compile-firewall header verbatim in spirit. ─────────────────────────────────────

/// Closed resource enum. A tool target not listed here is UNREPRESENTABLE.
/// P40 ships exactly one variant. P42 may extend; money/auth/RLS/migration
/// resources are never added (red-line — see §4.1's reachability argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolResource { OrderStatus }

/// Closed action enum. `Read` is the ONLY variant in P40 — a mutating tool
/// invocation is not policy-forbidden, it is type-unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAction { Read }

/// The capability scope a tool invocation executes under. Granted by the
/// composition layer (agent-facade), checked fail-closed by the port impl:
/// the granted scope must cover the tool's declared scope or the invocation
/// is refused BEFORE the tool body runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolScope { pub resource: ToolResource, pub action: ToolAction }

/// Static declaration of one tool — what the model is told, verbatim.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,         // "read_order_status"
    pub description: &'static str,  // natural-language contract handed to the model
    pub arg_name: &'static str,     // "order_id"
    pub scope: ToolScope,           // declared requirement, checked against the grant
}

/// One parsed tool invocation (the model's ask, post-parse, pre-execution).
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub tool_name: String,
    /// The raw arguments payload from the model, verbatim (JSON text on the
    /// OpenAI-compat wire). The PORT IMPL parses it; the loop never does.
    pub raw_arg: String,
}

/// Tool output — plain text handed back to the model as the observation.
#[derive(Debug, Clone)]
pub struct ToolOutput { pub content: String }

/// Typed tool failure. Every variant is a loop OBSERVATION or OUTCOME —
/// never a panic, never a silent retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    UnknownTool(String),        // model asked for a tool that doesn't exist
    BadArg(String),             // arguments unparseable / missing order_id
    ScopeDenied,                // granted scope does not cover the tool's declared scope
    NotFound(String),           // order id valid in form, absent in the source
    Unavailable,                // the tool's backing source is down (P37-later / stub-never)
    Timeout,                    // tool execution exceeded TOOL_TIMEOUT_MS
}

/// The tool port. Implemented in agent-facade; consumed ONLY by agent-loop
/// as `&dyn ToolPort` (§3.1 firewall).
pub trait ToolPort {
    fn spec(&self) -> &ToolSpec;
    fn invoke(&self, granted: ToolScope, inv: &ToolInvocation) -> Result<ToolOutput, ToolError>;
}

// ── kernel/src/ports/llm.rs — B-c extension (extend, don't rewrite) ─────────────
/// A tool the backend may call — plain struct; the adapter serializes it into the
/// OpenAI `tools` array. Mirrors ToolSpec's fields as owned Strings (the kernel
/// port stays 'static-free at the wire boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDecl { pub name: String, pub description: String, pub arg_name: String }

/// A tool call the model returned — parsed by the adapter from `message.tool_calls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallReq { pub name: String, pub arguments_json: String }

// ChatRequest gains:  pub tools: Vec<ToolDecl>,        // Default: Vec::new() — existing
// ChatResponse gains: pub tool_calls: Vec<ToolCallReq>,// call sites compile unchanged

// ── agent-loop/src/lib.rs — NEW crate (B-d). Depends on agent-facade ONLY. ──────
/// Hard iteration ceiling. 4 = one plan turn + one tool turn + one recovery
/// turn + one answer turn. Raising it is a reviewed const change, not a knob.
pub const MAX_AGENT_ITERATIONS: u8 = 4;

/// Per-tool-invocation wall-clock ceiling (enforced by the port impl for I/O-backed
/// sources; the P40 stub is instant). See §4.3 for the total-wall-time bound.
pub const TOOL_TIMEOUT_MS: u64 = 5_000;

/// One loop event — the log is a Vec of these; tests assert on the SEQUENCE
/// (standard item 3: event-driven, matches the kernel's own decide/fold shape).
#[derive(Debug, Clone)]
pub struct LoopLogEntry { pub iteration: u8, pub event: LoopEventKind }

#[derive(Debug, Clone)]
pub enum LoopEventKind {
    ModelReply { content: String, total_tokens: u32 },
    ToolCallParsed { tool_name: String, raw_arg: String },
    ToolCallMalformed { raw: String, reason: String },   // observation, not a crash
    ToolResult { tool_name: String, output: String },
    ToolFailed { tool_name: String, error: String },     // rendered ToolError
}

/// Terminal outcome of one loop run. EVERY path lands here — there is no panic
/// path and no unbounded path (§4.3's finite-wall-time argument).
#[derive(Debug)]
pub enum LoopOutcome {
    /// The model produced a final answer (with the full event log attached).
    Answer { text: String, log: Vec<LoopLogEntry> },
    /// Backend absent/refusing before or during the run (LlmError::Unavailable
    /// / Timeout / DispatchError::BudgetExceeded surfaced). P41's degradation
    /// contract consumes exactly this variant.
    AssistantUnavailable { reason: String, log: Vec<LoopLogEntry> },
    /// caps().tool_calling == false after the live probe — fail-closed refusal
    /// to start a tool run (the loop does NOT degrade to tool-less chat silently).
    ToolCallingUnsupported { backend_id: String },
    /// The iteration ceiling fired. The log shows why (e.g. repeated malformed
    /// calls). Never a silent truncation — the caller sees the cap by type.
    IterationCapExceeded { log: Vec<LoopLogEntry> },
}

pub struct AgentLoop<'a> {
    backend: &'a dyn agent_facade::LlmBackend,   // re-export — see §3.1
    tool: &'a dyn agent_facade::ToolPort,
    granted: agent_facade::ToolScope,
}
// impl: pub fn run(&self, user_request: &str) -> LoopOutcome;

// ── agent-facade/src/lib.rs — NEW crate (B-a). The ONLY crate in the agent lane
// that imports dowiz-kernel. Re-exports the two port surfaces and NOTHING else
// of the kernel (no domain, no money, no order_machine in its public API), and
// owns the one concrete tool: ──────────────────────────────────────────────────
pub use dowiz_kernel::ports::llm::*;   // the whole llm port surface, verbatim
pub use dowiz_kernel::ports::tool::*;  // the whole tool port surface, verbatim

/// The P37 seam (soft dependency, named not built): where order status comes from.
pub trait OrderStatusSource {
    /// Returns the canonical oracle string form ("PENDING" … "COMPENSATED_REFUND",
    /// the order_machine.rs:29-45 vocabulary) or a typed error.
    fn status_of(&self, order_id: &str) -> Result<String, ToolError>;
}

/// Stub source for P40's DoD: a fixed map, solo-offline by construction.
pub struct FixtureOrders(/* BTreeMap<String, OrderStatus> */);

/// LATER (P37 landed, separate PR): HttpOrderStatusSource { base_url } — same
/// trait, inherits P37's capability-cert auth (§4.1 residual-risk note). NOT P40.

/// The one tool. Wraps any OrderStatusSource; enforces scope + arg parsing.
pub struct ReadOrderStatusTool<S: OrderStatusSource> { /* source: S */ }
// impl ToolPort for ReadOrderStatusTool<S>
```

**Rejected alternative (DECART-style, one line each):** defining `ToolPort` inside `agent-loop`
itself (no kernel involvement at all) — rejected because §10.3 item 5 and §10.5.4 DoD-1 place
the trait in the kernel-ports layer where `llm.rs`/`agent/` already live, keeping ONE ports
convention (standard item 19); a loop-private trait would be a second, unshared tool vocabulary
that P42's MCP layer would then have to re-derive. Defining the tool impl in `llm-adapters` —
rejected: that crate is the LLM wire adapter; mixing tool execution into it couples two
independently-testable surfaces and breaks the one-crate-one-adapter-family convention
(`agent-adapters`' own header states the sibling discipline, `agent-adapters/src/lib.rs:3-8`).

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 B-a — `ToolPort` + the compilation firewall (KernelFacade pattern, mirrored explicitly)

**The pattern being mirrored, cited:** PROTOCOL's `KernelFacade`
(`/root/bebop-repo/bebop2/proto-cap/src/facade.rs:123 submit_intent` — drifted from the
roadmap's `:64` cite, §0) is "the anti-corruption layer … exposes exactly two public methods";
consumers reach the kernel only through it, and the lack of direct kernel imports is proven by
`cargo tree` + a committed red-proof (§10.3 item 5: "Three instances, one pattern: PROTOCOL's
KernelFacade …, AGENT's ToolPort (P40 DoD-1), and the MCP layer (P42 DoD-3)"). P40 is the
second instance.

**The honest tension, resolved (not papered over):** §10.5.4 DoD-1 requires BOTH "the trait
lives in the kernel-ports layer" AND "the loop crate does not import `dowiz-kernel` directly."
In Rust, naming a kernel-defined trait puts the kernel in your transitive graph — the two
clauses are compatible only with a facade crate between them, which is exactly the KernelFacade
move. Resolution: `agent-facade` imports `dowiz-kernel` and re-exports ONLY
`ports::llm` + `ports::tool` (§2); `agent-loop` imports `agent-facade` and nothing else
kernel-shaped. `dowiz-kernel` appears in `agent-loop`'s graph **only at depth 2, only via the
facade** — the audited chokepoint. Two structural checks, both committed:

```
cargo tree -p agent-loop --depth 1 | grep dowiz-kernel     # → empty (no DIRECT dep)
grep -rn "dowiz_kernel" agent-loop/src/                    # → empty (no direct path refs)
```

**Why the firewall is load-bearing, not ceremonial (hazard preview, full argument §4.1):**
`agent-facade`'s public API contains no kernel mutation symbol — `agent-loop` cannot *name*
`decide`, `fold`, `apply_tax`, or any store. The model's worst output can only select among
`ToolPort` invocations. That is a namespace-reachability guarantee, checkable by the grep above,
not a promise.

RED→GREEN: (i) the two checks above are added to `agent-loop`'s test suite as a
`#[test]`-wrapped `std::process::Command` check (same self-auditing shape as the kernel's own
firewall done-check, `llm.rs:5-7`); the RED commit demonstrates it firing by temporarily adding
`dowiz-kernel = { path = "../kernel" }` to `agent-loop/Cargo.toml` (output pasted into the
commit message — the committed red-proof §10.3 item 5 demands), then removing it. (ii) kernel
side unchanged: `cargo tree -p dowiz-kernel` still shows no HTTP client and no adapter crates
(the existing WAVE-0 done-check, re-run not re-invented).

**Adversarial:** a test that constructs `ReadOrderStatusTool` with a granted scope of a
mismatched resource — P40 has one resource variant, so this test is written against the
`ScopeDenied` arm using a deliberately-wrong granted/declared pairing constructed in-test
(e.g. granted scope built for a hypothetical second tool via a test-only enum extension is NOT
possible — closed enum — so the test asserts the *cover check runs before the body* by counting
source invocations on a spy `OrderStatusSource`: a `ScopeDenied` path performs ZERO source
calls). Fail-closed proven by observation order, not by trust.

### 3.2 B-b — `Caps.tool_calling` un-pinned via live per-model probe, fail-closed

**Today:** `llm-adapters/src/ollama.rs:59` hard-pins `tool_calling: false` with the comment
"Rerank/tool-calling not assumed" — correct fail-closed behavior for a capability nobody had
probed. **The fix is a probe, not a flag flip** (HARNESS doc §2.2 item 5: "Tool-calling/
structured-output support differs per backend/model — a `Caps` probe, not assumed").

Mechanics:

- `OpenAiCompatTransport` gains `show_capabilities(&self, model_id: &str) ->
  Result<Vec<String>, LlmError>` hitting Ollama's native `GET /api/show` (POST body
  `{"model": id}`) and reading the response's `capabilities` array (present on this host's
  v0.30.9; contains `"tools"` for tool-capable models). Any HTTP error, missing field, or parse
  failure → `Err` → **`false`** (fail-closed — a probe failure is indistinguishable from "no").
- `OllamaAdapter` memoizes probe results per model id
  (`std::sync::Mutex<BTreeMap<String, bool>>` — same std-only discipline as the crate's ureq/no-
  tokio mandate, HARNESS §5 Decision 2). `caps()` reports `tool_calling` for the model
  `route_model` would pick for `TaskClass::General` (`llama3.1:8b`, `ollama.rs:42`) — the model
  the loop actually uses. Per-request model overrides re-probe through the same memo.
- The probe runs lazily on first `caps()` call, never at construction (constructing an adapter
  while the daemon is down must stay cheap and non-failing — `health()` is the liveness check,
  unchanged).

RED→GREEN: RED = a test asserting `adapter.caps().tool_calling == true` against the live daemon
fails today (pin). GREEN after the probe lands. Both directions pinned:

```rust
// llm-adapters/tests/ollama_roundtrip.rs additions (live-daemon convention of that file):
#[test] fn tool_calling_probed_true_for_llama31() { /* live /api/show, expect true */ }
#[test] fn tool_calling_probed_false_for_embed_model() {
    // nomic-embed-text has no "tools" capability → false, NOT an error
}
```

**Adversarial (fail-closed proven, not assumed):** with the daemon stopped
(`systemctl stop ollama`, the done-check choreography HARNESS §6 WAVE-0 already uses),
`caps().tool_calling` returns `false` — not a panic, not a stale `true` from a previous memo of
a different model, not an `Err` escaping through `caps()`'s infallible signature. Plus a
malformed-response probe: a test transport double returning `capabilities: "yes"` (string, not
array) must yield `false`.

### 3.3 B-c — `LlmBackend` wire extension + the cache-key extension (found live drift, closed here)

`ChatRequest` gains `tools: Vec<ToolDecl>`; `ChatResponse` gains `tool_calls: Vec<ToolCallReq>`
(§2 types; `Default` for `ChatRequest` extends with `tools: Vec::new()`, `llm.rs:61-75`, so
every existing call site compiles and behaves identically — extend-don't-rewrite honored
structurally). The adapter (`transport.rs`) serializes `tools` into the OpenAI `tools` array
(`[{"type":"function","function":{"name":…,"description":…,"parameters":{…one string
property…}}}]`) and parses `message.tool_calls[].function.{name,arguments}` into
`ToolCallReq { name, arguments_json }`. All JSON stays in the adapter; the kernel structs carry
plain strings (firewall unchanged).

**The mandatory companion (this pass's found drift, §0):** the live cache key
(`llm-adapters/src/cache.rs:55-60`) canonicalizes the CURRENT field set and has **no `tools`
entry**, while HARNESS §3.2 specs the key as `{model_id, messages, temperature, top_p,
max_tokens, seed, tools}`. Landing the `tools` field without extending `cache_key` is a
**cache-poisoning hazard**: a tool-bearing request and its tool-less twin would collide, and a
cached tool-less answer would be served to a tool run (or vice versa). The key extension ships
in the SAME commit as the field — never separately.

RED→GREEN (event-counter form, same falsifier discipline as the cache's existing done-check):

```rust
#[test]
fn tools_field_partitions_the_cache() {
    // identical request twice, once with tools=[] once with tools=[read_order_status]
    // against a call-counting backend double:
    // RED today (field doesn't exist / would collide) — GREEN = 2 backend calls, 2 entries.
}
```

**Adversarial:** a request with `tools` non-empty against a backend whose probe said
`tool_calling == false` → the loop preflight refuses (`LoopOutcome::ToolCallingUnsupported`,
§3.4) — the request is never sent. This is asserted with a spy transport: zero HTTP calls on
the refusal path.

### 3.4 B-d — the `AgentLoop` executor + the ONE tool, end-to-end

The loop, exactly (plan→act→observe, bounded — no framework):

```
run(user_request):
  0. preflight: backend.health()?            → Err ⇒ AssistantUnavailable
                backend.caps().tool_calling  → false ⇒ ToolCallingUnsupported
  1. messages = [system(tool contract from ToolSpec), user(user_request)]
  2. for iteration in 1..=MAX_AGENT_ITERATIONS:
       resp = backend.chat(req with tools=[tool.spec() as ToolDecl])
              — via the Harness/Dispatcher path (budget + harvest apply)
              → Err ⇒ AssistantUnavailable (typed, log attached)
       log ModelReply
       if resp.tool_calls is empty: return Answer { resp.content, log }
       for the FIRST tool_call only (one tool, one call per turn — anything
       further in the same reply is logged ToolCallMalformed "multiple calls"):
         parse → ToolInvocation; unknown name / bad JSON ⇒ log ToolCallMalformed,
           append an observation message stating the error, continue      (§3.5)
         tool.invoke(granted, inv)
           Ok(out)  ⇒ log ToolResult, append observation(out.content), continue
           Err(e)   ⇒ log ToolFailed, append observation(rendered e), continue (§3.6)
  3. return IterationCapExceeded { log }
```

There is **no retry construct in the loop**: repetition exists only as model-driven turns, and
those are capped. "Silent retry-forever" is not prevented by policy — it has no representation
(§4.1).

**The one tool:** `ReadOrderStatusTool<FixtureOrders>` (§2). `invoke` = scope-cover check →
parse `raw_arg` as JSON `{"order_id": "..."}` (serde_json in agent-facade — an adapter-layer
crate; the kernel stays serde-free) → `source.status_of(id)` → `ToolOutput { content:
"order <id> status: IN_DELIVERY" }` using the oracle string vocabulary
(`order_machine.rs:29-45`) verbatim, so the model's answer text can only contain canonical
status words.

**End-to-end DoD test** (`agent-loop/tests/e2e_read_order_status.rs`, live-daemon convention of
`ollama_roundtrip.rs`):

```rust
#[test]
fn agent_reads_order_status_end_to_end() {
    // FixtureOrders: {"ord-7" → InDelivery}. Request: "What is the status of order ord-7?"
    // Assert (strict, on the EVENT SEQUENCE — standard item 3):
    //   log contains ToolCallParsed{tool_name:"read_order_status", raw_arg ~ ord-7}
    //   followed by ToolResult{..} — the agent DID something, provably.
    // Assert (outcome): LoopOutcome::Answer whose text contains "IN_DELIVERY".
    // Model: the General route (llama3.1:8b), temperature 0, CachePolicy::NoCache
    // (a live proof must not be served from cache — falsifiability over speed).
}
```

RED first: committed against a stub `AgentLoop::run` returning
`IterationCapExceeded { log: vec![] }` — RED on the sequence assertion; GREEN with the real
loop. This is §10.5.4 DoD-3's falsifiable "the agent can DO something" gate, verbatim,
deepened with the event-sequence assertion.

### 3.5 B-e(i) — adversarial: malformed tool-call never crashes the loop

The model is an adversary here (hallucinating names, emitting broken JSON, calling twice).
Each case is a test with a scripted fake backend (implements `LlmBackend`, returns canned
`tool_calls`) — no live daemon needed, fully deterministic:

1. `tool_calls: [{name:"transfer_money", arguments:"{}"}]` → log `ToolCallMalformed`
   (UnknownTool rendered), observation fed back, loop CONTINUES; scripted turn 2 answers
   normally → outcome `Answer`. Assert the exact event sequence
   `[ModelReply, ToolCallMalformed, ModelReply]` and — the red-line teeth — assert the spy
   `OrderStatusSource` saw **zero** invocations.
2. `arguments: "{"order_id":"` (truncated JSON) → `ToolCallMalformed(BadArg)`, same
   continue-then-recover shape.
3. The model malforms EVERY turn → loop terminates at exactly `MAX_AGENT_ITERATIONS` with
   `IterationCapExceeded`; assert `log.len()` shows exactly 4 model turns — the cap is a hard
   invariant boundary, not a soft counter (§4.3 Self-Termination).
4. Two tool_calls in one reply → first processed, second logged malformed ("multiple calls") —
   the one-tool discipline is enforced per-turn, not just per-phase.

### 3.6 B-e(ii) — adversarial: tool error/timeout is a typed outcome, never silent

1. `FixtureOrders` without "ord-9", model asks for ord-9 → `ToolFailed(NotFound)` observation →
   model (scripted) answers "order not found" → `Answer`. The failure is VISIBLE in the log and
   in the answer — never swallowed.
2. A `SlowSource` test double sleeping past `TOOL_TIMEOUT_MS` → `ToolError::Timeout` → logged,
   observed, loop continues or caps. (The stub path enforces the timeout in
   `ReadOrderStatusTool::invoke` via a watchdog thread + `mpsc::recv_timeout` — std-only,
   matching the dispatcher's own std::thread discipline, `dispatch.rs:1-11`.)
3. Budget exhaustion mid-loop: `TokenBucket` drained (capacity 1, cost > 1) →
   `DispatchError::BudgetExceeded` → `AssistantUnavailable { reason: "budget", log }` — the
   loop inherits the harness's degrade-closed refusal (`dispatch.rs:23-29`) instead of
   inventing a second budget mechanism (standard item 19).
4. Backend dies BETWEEN iterations (fake backend scripted: turn 1 ok, turn 2
   `Err(Unavailable)`) → `AssistantUnavailable` with the partial log attached — partial
   progress is never presented as an answer.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6) — what can a compromised/hallucinating model actually reach?

Reachability is argued from type structure, not from a promise:

- **The model's entire influence channel** is the byte content of `ChatResponse.content` and
  `ChatResponse.tool_calls`. The loop maps those bytes onto exactly three continuations:
  (a) return them as `Answer` text, (b) look up `tool_calls[0].name` — any string other than
  `"read_order_status"` lands in `UnknownTool`, (c) pass `arguments_json` to
  `ReadOrderStatusTool::invoke`. There is no fourth continuation in the code.
- **Write actions are unrepresentable:** `ToolAction` has one variant, `Read`
  (§2). A mutating invocation is not caught by review or by a validator — it cannot be
  constructed. Extending the enum is a kernel-ports diff, reviewable at the type level
  (standard item 14's compile-time gate: the closed enum IS the smart index).
- **Kernel mutation surface is out of namespace:** `agent-loop` can name only what
  `agent-facade` re-exports — the two port surfaces. `decide`/`fold`/`apply_tax`/stores are
  not importable without a Cargo.toml + facade diff, both firewall-tested (§3.1's two checks).
  Money/auth/RLS/migrations are therefore unreachable through ANY model output: the worst a
  fully-adversarial model can do is (i) read the status of order ids present in the configured
  `OrderStatusSource` and (ii) write misleading TEXT into `Answer` — and the answer is
  advisory by the §10.3 invariant (no decision path consumes it).
- **Residual risk, named honestly:** when the source becomes P37's HTTP surface, "order ids
  present in the source" widens to whatever P37 serves. Obligation transferred, in writing:
  `HttpOrderStatusSource` must call P37 through its capability-cert auth (§10.3 item 3), never
  an unauthenticated bypass — that integration lands with P37, is out of P40's scope, and this
  sentence is its tracking anchor. Second residual: prompt-injection via order-status STRINGS
  is nil here because the fixture returns closed-vocabulary status words only
  (`order_machine.rs:29-45`); the P37 integration must preserve the closed-vocabulary output
  contract (status words, not free text) for the same reason.

### 4.2 Schemas & scaling axes (item 8)

`LoopLogEntry` vectors scale by iteration count — hard-capped at `MAX_AGENT_ITERATIONS` (4), so
a log is ≤ ~a dozen entries, ≤ a few KB: no eviction story needed, axis named. `ToolDecl`
serialization scales by tool count — exactly 1 in P40; the OpenAI `tools` array shape is the
break point to re-examine when P42 grows the catalog (its blueprint's job, cross-referenced not
solved). The probe memo (`BTreeMap<String, bool>`) scales by distinct model ids — single digits
on this host (`ollama list`: 4 models), unbounded growth impossible without unbounded distinct
model-id strings, which the config surface doesn't produce. Cache-key growth (§3.3): the
canonical JSON gains one `tools` array — key size scales with tool-decl text, bounded by the
1-tool catalog.

### 4.3 Isolation / bulkhead (item 11), mesh (item 12), rollback (item 13), living memory (item 15)

- **Isolation:** tool-call failure cannot propagate to order/money state for two independent
  reasons, either of which suffices: (i) the tool is read-only by type (§4.1), (ii) the order
  flow never calls the loop — the no-AI invariant (§10.3 item 1) means the dependency arrow
  points one way only. The loop additionally sits behind the existing Dispatcher bulkhead
  (TokenBucket budget → typed `BudgetExceeded`, `dispatch.rs:23-29`) so a runaway
  conversational burst degrades the ASSISTANT, never the node. Known upstream defect inherited,
  not hidden: the Dispatcher's `workers` bound is dead code (A4, HARNESS doc audit addendum) —
  its fix is owned by Phase-27 Wave F1b; P40 does not paper over it and does not depend on it
  (the iteration cap bounds the loop's own concurrency at 1 in-flight call).
- **Mesh (item 12):** P40 is **node-local. Not mesh-gossiped, no transport dependency, no
  payload budget.** The loop, the tool, and the log live and die inside one node's process.
  Foreign-agent mesh exposure is `AgentBridge`'s lane (§4.4) and stays there.
- **Rollback / self-healing vocabulary (item 13, used precisely):** P40 claims only the
  **Self-Termination / unrepresentable-state leg**: the iteration cap and the typed outcomes
  make "stuck loop" a state with no representation — worst-case wall time is the closed-form
  bound `MAX_AGENT_ITERATIONS × (transport_deadline + TOOL_TIMEOUT_MS)` (both deadlines exist:
  `LlmError::Timeout` is transport-level, `llm.rs:148-149`; `TOOL_TIMEOUT_MS` §2), a finite
  number, not a supervisor's judgment call. No Self-Healing claim (no redundancy math here), no
  Snapshot-Re-entry claim (the loop is stateless between runs — recovery = run again). Every
  build item is mechanically reversible: delete the two crates + the `ports/tool.rs` module +
  revert the two `llm.rs` field additions; kernel tests are unaffected by construction
  (P41 DoD-1 proves it continuously).
- **Living memory (item 15):** the loop's event log is ephemeral per-run; the durable record is
  the existing H1 harvest row per dispatched call (`TrackRecord`, `dispatch.rs:37-45`) — P40
  adds no second telemetry channel (standard item 19). If loop logs ever need durable recall,
  that is a `living_knowledge` consumer decision
  (`internal-retrieval-living-memory-arc-2026-07-14`), explicitly deferred with that trigger.

### 4.4 Non-conflation with `AgentBridge` (hard constraint, checkable)

Two "agents," one repo (§10.5.4 naming discipline, restated as a check): the LOCAL delivery-ops
assistant (this blueprint: `ports/llm.rs` + `ports/tool.rs` + `llm-adapters` + `agent-facade` +
`agent-loop`) vs the mesh foreign-agent admission/caging seam (`ports/agent/{admission,cap,
manifest,scope}.rs` + `agent-adapters/*` — PROTOCOL's B1). P40 makes **zero edits** under
`kernel/src/ports/agent/` and `agent-adapters/` — enforced by review + the P40 close-out check
`git diff --stat <base>..HEAD -- kernel/src/ports/agent agent-adapters` → empty. The naming
guard: the new crate is `agent-loop`, the new port module is `ports/tool.rs` (NOT
`ports/agent2.rs`, NOT anything that would suggest the admission seam). `agent-facade` must not
re-export `ports::agent::*` — that re-export appearing is the conflation smell, grep-checkable:
`grep -rn "ports::agent" agent-facade/src agent-loop/src` → empty, added to the §3.1 firewall
test.

### 4.5 Linux-discipline verdict framework (item 9)

Applying `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s categories, not
re-deriving them: the facade chokepoint is **ALREADY-EQUIVALENT** ("one gate, one place" — the
third instance of a proven repo pattern, §10.3 item 5); typed outcomes for every failure path
**REINFORCES** the repo's no-silent-failure discipline (every port is fallible — Phase-27's
rule); the per-model capability probe **EXTENDS** the existing fail-closed `Caps` discipline
from static pin to live discovery, with the fail-closed default preserved (probe failure ⇒
`false` — never a fail-open capability).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Extends §10.5.4's four P40 DoD lines with real test names; none is a prose checkbox. P40 is
DONE iff every row is demonstrably true.

| Item | RED (fails before) | GREEN (passes after) | Named test / check (permanent, item 17) |
|---|---|---|---|
| B-a firewall | `firewall_no_direct_kernel_dep` fails when `dowiz-kernel` is added to `agent-loop/Cargo.toml` (red-proof output committed) | `cargo tree -p agent-loop --depth 1` clean; grep checks clean | `agent-loop/tests/firewall.rs::firewall_no_direct_kernel_dep` + `::no_agentbridge_reexport` (§4.4) |
| B-b probe | `tool_calling_probed_true_for_llama31` RED vs the `:59` pin | live probe true for llama3.1:8b, false for nomic-embed-text, false with daemon stopped | `llm-adapters/tests/ollama_roundtrip.rs::tool_calling_probed_*` (3 tests) |
| B-c wire+key | `tools_field_partitions_the_cache` uncompilable/RED (no field) | field + key extension in one commit; partition test green; all existing llm-adapters tests untouched-green | `llm-adapters/src/cache.rs` tests mod::`tools_field_partitions_the_cache` |
| B-d e2e | `agent_reads_order_status_end_to_end` RED vs stub loop | live Ollama run: event sequence `ToolCallParsed→ToolResult` + `Answer` containing `IN_DELIVERY` | `agent-loop/tests/e2e_read_order_status.rs::agent_reads_order_status_end_to_end` |
| B-e(i) malformed | sequence tests RED vs stub | 4 scripted-adversary tests green; spy source sees 0 calls on UnknownTool | `agent-loop/tests/adversarial.rs::{malformed_unknown_tool_recovers, truncated_json_recovers, all_malformed_hits_cap, multiple_calls_first_only}` |
| B-e(ii) typed errors | error-path tests RED vs stub | NotFound/Timeout/Budget/mid-run-death each a typed outcome with log | `agent-loop/tests/adversarial.rs::{not_found_is_visible, slow_tool_times_out, budget_exhaustion_unavailable, backend_dies_midrun}` |
| bounded loop | `all_malformed_hits_cap` asserts exactly `MAX_AGENT_ITERATIONS` model turns | same | same test (doubles as the §10.5.4 DoD-4 bound proof) |

Ledger obligation: one row in `docs/regressions/REGRESSION-LEDGER.md` for the cache-partition
hazard (§3.3 — "tools-not-in-cache-key poisoning; guardrail: `cargo-test`
`tools_field_partitions_the_cache`"), red→green proof per the ledger's standing ratchet rule
(`REGRESSION-LEDGER.md:9-16`). The firewall check becomes a CI-gate row when P41 DoD-1's CI job
lands (P41 owns the CI wiring; P40 ships the tests it will run — dependency named, not
duplicated).

---

## 6. Benchmark plan (item 10) — existing harnesses only

`llm-adapters` already has a criterion harness (`llm-adapters/benches/criterion.rs` +
`baseline.json` + `BENCH_HISTORY.md` — verified on disk this pass). P40 adds **two benches and
zero infrastructure**:

1. `agent_loop/loop_overhead_scripted` — full `run()` against a zero-latency scripted backend
   + instant stub tool: measures everything EXCEPT model decode and tool I/O (parse, log,
   message assembly, scope check). **Budget: ≤ 1 ms per run** — the loop must be invisible next
   to decode (measured ground truth from the latency blueprint: p50 4.9 s per managed call;
   local CPU decode 4.8–10.5 tok/s — `BLUEPRINT-LATENCY-ELIMINATION-…-2026-07-17.md`). A loop
   overhead 3 orders of magnitude under the floor of one model call is the falsifiable target.
2. `agent_loop/probe_cost_api_show` — one live `/api/show` probe round-trip, measured; and the
   memoized path (**budget: memo hit ≤ 1 µs** — a Mutex+BTreeMap lookup). Probe overhead is
   paid once per model per process; the bench documents the once-cost so nobody "optimizes" it
   into a startup stall later without a number.

Numbers go into `llm-adapters/benches/BENCH_HISTORY.md` (RED-commit baseline seeding, same
`bench_track` convention as the kernel benches). End-to-end latency (model-dominated) is
tracked by the existing H1 harvest rows (`TrackRecord.ms`), not a bench — real traffic beats a
synthetic number for the decode-dominated segment (telemetry hook per item 10's second half,
already built: `llm-adapters/src/telemetry.rs`).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.3 items 1/2/5 + §10.5.4 P40 (the
index-level DoD this deepens) · `HARNESS-LLM-BACKEND.md` (backend layer: §2.2 Quirks item 5 =
the probe mandate, §3.2 = the cache-key mandate, §5 Decision 2 = ureq/no-tokio discipline, §6 =
done-check choreography, audit addendum = A4 workers defect inherited-not-hidden) ·
`BLUEPRINT-P-A-kernel-primitives.md` (structure template + the honest-overlap-note convention) ·
`BLUEPRINT-P41-three-mode-ai-operation.md` (sibling, consumes `LoopOutcome::AssistantUnavailable`
and the parity surface) · `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md`
(measured latency ground truth for §6 budgets) · `docs/regressions/REGRESSION-LEDGER.md` (§5
row) · KernelFacade: `/root/bebop-repo/bebop2/proto-cap/src/facade.rs:123` (the mirrored
pattern) · `ac6_solo_island_full_flow_no_peers`
(`/root/bebop-repo/bebop2/delivery-domain/src/intake.rs:408`, offline-proof spirit anchor).
Memory files: `harness-llm-backend-and-hermetic-remediation-2026-07-17` (substrate arc status) ·
`never-bypass-human-gates-2026-06-29` + `test-integrity-rules-2026-06-27` (red-line classes
§1.3) · `verified-by-math-2026-07-07` (§4.1's reachability-not-promise stance) ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4.3 deferred trigger) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline applied). Supersedes:
nothing — additive over §10.5.4's index entry.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): ONE tool vocabulary (`ports/tool.rs`)
  shared by loop, facade, and (later) P42's MCP exterior — no loop-private second tool grammar
  (§2's rejected alternative). ONE budget mechanism (the existing TokenBucket/Dispatcher),
  never a second loop-side budget (§3.6 case 3).
- **P6 CAUSE-AND-EFFECT** (determinism as law): every adversarial test runs on scripted fake
  backends — fully deterministic event sequences, asserted exactly (§3.5/§3.6); the one live
  test (§3.4) pins temperature 0 + `NoCache` so its falsifiability is not laundered through a
  cache hit.
- **P7 GENDER** (paired creation, no self-certification): the loop never certifies its own
  safety — the firewall is proven by an external `cargo tree`/grep process check, the tool's
  read-only-ness by the closed enum the KERNEL owns (a different crate than the loop), and the
  e2e claim by a live model run, not a mock of one.

(Other principles are not load-bearing for these items and are not claimed decoratively, per
the Anu/Ananke discipline.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites incl. 2 found drifts: cache-key `tools` gap, facade line move) |
| 2 DoD | §5 |
| 3 spec/event-driven TDD | §2 (spec first), §3 per-item RED tests, event-SEQUENCE assertions §3.4/§3.5 |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.5/§3.6 (8 named adversary tests), §3.1/§3.2 fail-closed probes |
| 6 hazard-safety as math | §4.1 (reachability from closed enums + namespace, not promises) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 |
| 9 Linux discipline | §4.5 |
| 10 benchmarks+telemetry | §6 (2 benches, budgets, H1 harvest as the telemetry hook) |
| 11 isolation/bulkhead | §4.3 (read-only type + one-way dependency arrow + Dispatcher bulkhead) |
| 12 mesh awareness | §4.3 (node-local, explicitly not gossiped) |
| 13 rollback/self-heal vocabulary | §4.3 (Self-Termination leg only, closed-form wall-time bound) |
| 14 error-propagation gates | §4.1 (closed enum = compile-time gate), §5 (named tests per path) |
| 15 living memory | §4.3 (H1 harvest reuse; durable-log trigger named + deferred) |
| 16 tensor/spectral + eqc reuse | N/A-honest: no closed-form math in this phase; no decorative claim made |
| 17 regression ledger | §5 (cache-partition row named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §2 (rejected alternatives), §3.6 (budget reuse), §6 (no new harness) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Execute in this order. Every task names its files, acceptance command, and gate. Repo:
`/root/dowiz`. Prereq for T4/T5-live: `systemctl is-active ollama` → active (daemon on
`127.0.0.1:11434`).

1. **T1 (B-c kernel half).** In `kernel/src/ports/llm.rs`: add `ToolDecl` + `ToolCallReq` (§2
   verbatim), add `tools: Vec<ToolDecl>` to `ChatRequest` (+ `Default`), `tool_calls:
   Vec<ToolCallReq>` to `ChatResponse`. NO serde, NO new deps (`kernel/Cargo.toml` untouched —
   check `git diff kernel/Cargo.toml` → empty). Acceptance: `cd kernel && cargo test --lib`
   green (existing suite compiles unchanged — the Default extension guarantees it).
2. **T2 (B-a kernel half).** Create `kernel/src/ports/tool.rs` with §2's tool types + trait
   verbatim; add `pub mod tool;` with a one-line doc to `kernel/src/ports/mod.rs` (convention:
   `ports/mod.rs:1-9`). Copy the compile-firewall header comment style from `llm.rs:1-7`.
   Acceptance: `cd kernel && cargo test --lib` green; `cargo tree` (in kernel/) still shows no
   HTTP/adapters.
3. **T3 (B-c adapter half — ONE commit, both halves).** In `llm-adapters/src/transport.rs`:
   serialize `req.tools` into the OpenAI `tools` array; parse `message.tool_calls` into
   `ToolCallReq`. In `llm-adapters/src/cache.rs::cache_key` (`:55-60`): add `tools` to the
   canonical BTreeMap. Write `tools_field_partitions_the_cache` (§3.3) RED-first against a
   call-counting double (the crate already has that double pattern — see the cache's existing
   done-check test). Acceptance: `cd llm-adapters && cargo test` green incl. the new test;
   grep confirms `tools` now appears in `cache_key`. Add the REGRESSION-LEDGER row (§5).
4. **T4 (B-b).** In `llm-adapters/src/transport.rs`: add `show_capabilities(model_id)` (native
   `POST /api/show`, parse `capabilities: Vec<String>`, any failure ⇒ `Err`). In `ollama.rs`:
   replace the `:59` pin with the memoized probe (§3.2 mechanics — `Mutex<BTreeMap<String,
   bool>>`, General-route model, fail-closed). Add the 3 probe tests to
   `tests/ollama_roundtrip.rs` (§3.2/§5 names) + the malformed-capabilities double test.
   Acceptance: `cd llm-adapters && cargo test` green with daemon up; then
   `systemctl stop ollama && cargo test tool_calling_probed` → the fail-closed test green,
   `systemctl start ollama`.
5. **T5 (B-a crates).** Create `agent-facade/` (repo root, standalone crate per the no-workspace
   convention, HARNESS §1.3): `Cargo.toml` deps `dowiz-kernel = { path = "../kernel" }`,
   `serde_json` (arg parsing only); `src/lib.rs` = the two re-exports + `OrderStatusSource` +
   `FixtureOrders` + `ReadOrderStatusTool` (§2). Create `agent-loop/` (repo root): `Cargo.toml`
   deps `agent-facade = { path = "../agent-facade" }` ONLY (plus dev-deps); `src/lib.rs` = §2's
   loop types + `AgentLoop::run` per §3.4's pseudocode. Write `agent-loop/tests/firewall.rs`
   (§3.1's two checks + §4.4's no-`ports::agent`-reexport grep) — produce the RED-proof by
   temporarily adding `dowiz-kernel` to `agent-loop/Cargo.toml`, paste the failing output into
   the commit message, remove it. Acceptance: `cd agent-loop && cargo test firewall` green.
6. **T6 (B-e — before the live test; scripted, deterministic).** Write
   `agent-loop/tests/adversarial.rs`: the 8 named tests (§3.5 items 1-4, §3.6 items 1-4)
   against scripted `LlmBackend` fakes + spy/slow `OrderStatusSource` doubles. RED first
   against the stub loop, GREEN with the real one. Acceptance: `cd agent-loop && cargo test`
   fully green, deterministic (run twice, identical).
7. **T7 (B-d live e2e).** Write `agent-loop/tests/e2e_read_order_status.rs` (§3.4 verbatim:
   fixture ord-7 → InDelivery, temp 0, `NoCache`, event-sequence + `IN_DELIVERY` assertions).
   Wire through `agent-facade` composition (backend = the existing `Harness`/`StackBuilder`
   stack from `llm-adapters/src/compose.rs` — reuse, don't rebuild the stack). Acceptance:
   test green against the live daemon; then `systemctl stop ollama` → the same test path
   yields `AssistantUnavailable` (run the §3.6-style preflight assertion), restart daemon.
8. **T8 (benches).** Add the two §6 benches to `llm-adapters/benches/criterion.rs` (or an
   `agent-loop/benches/` sibling if the crate boundary demands — keep ONE history file
   convention). Record numbers + budgets pass/fail in `BENCH_HISTORY.md`. Acceptance: loop
   overhead ≤ 1 ms, memo hit ≤ 1 µs, numbers committed.
9. **T9 (close-out).** Run: `cd kernel && cargo test --lib`, `cd engine && cargo test`,
   `cd llm-adapters && cargo test`, `cd agent-loop && cargo test`, plus
   `git diff --stat <base>..HEAD -- kernel/src/ports/agent agent-adapters` → **empty** (§4.4).
   Verify every §5 DoD row. Do not mark P40 done if any adversarial test was weakened,
   `#[ignore]`d, or tolerance-inflated (ledger ratchet rule, `REGRESSION-LEDGER.md:9-16`).
   Hand the baton to P41: its DoD-2 parity test reuses T7's test with the backend swapped by
   config only — zero loop-code diff is P41's assertion, so do not "helpfully" parameterize
   the loop source for it.
