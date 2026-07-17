# LOCAL AI / LOCAL AGENTS — Research + Gap-Closing Plan (2026-07-17)

> **Research/planning artifact only. No code is written or edited by this document.** Branch:
> `feat/harness-llm-backend`. This arc deepens "local AI" beyond the two planes that already
> exist: the **outbound model plane** (`LlmBackend` + `OllamaAdapter` + caching + `TokenBucket`
> dispatch — BUILT, `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md`) and the **inbound
> foreign-agent plane** (`AgentBridge` manifest admission — BUILT on the mesh branch,
> `docs/design/agentic-mesh-protocol-2026-07-17/` B1). The question this document answers:
> **what is still missing for a hub to run genuinely autonomous *local agents*** — planning/memory
> loops, tool use, routing, budgets, coordination — as distinct from "local LLM inference," which
> is solved. Built per the Detailed Planning Protocol (`AGENTS.md`): ground truth first, explicit
> dependencies, inline DECART assessment, falsifiable checks, 2-question doubt audit.

---

## §1 Ground truth (live-verified this session, 2026-07-17 — commands run, not carried forward)

### 1.1 Host and inference substrate

- 8 vCPU AMD EPYC-Milan @ 2.0GHz (4 physical cores × 2 SMT threads), 30GiB RAM, no swap,
  **no GPU** (`nvidia-smi` → command not found). Ollama **0.30.9** (`/api/version`, live).
- Models pulled (`ollama list`): `qwen2.5-coder:7b` (4.7GB), `llama3.1:8b` (4.9GB),
  `nomic-embed-text` (274MB), `qwen3-embedding:0.6b` (639MB). **No sub-7B chat model is pulled**
  — the small tier a routing cascade needs does not exist on disk yet.
- Both 7-8B models observed **simultaneously resident** (`ollama ps`: 5.1GB + 5.6GB, 100% CPU,
  context 4096 each) — two-model residency on 30GiB is confirmed fact, not estimate.

### 1.2 Measured performance (this host, this session — n small, honesty required)

| Metric | Measured | Probe |
|---|---|---|
| Decode (7B Q4, warm) | **4.8–10.5 tok/s** across 4 probes | `/api/chat` + `/api/generate` `eval_count/eval_duration` |
| Prefill | **~636 tok/s** | `prompt_eval_count/prompt_eval_duration` |
| Cold model load (disk→RAM) | ~25–31s | `load_duration` on first call after eviction |
| Warm load | ~250ms | second call, resident |
| 2 concurrent requests, default `OLLAMA_NUM_PARALLEL=1` | **serialize** — wall ≈ sum of per-request decode (60 tok in 7.1s wall; each stream reports ~10.4 tok/s eval) | two parallel `curl` |

**The binding design constraint of this entire arc**: prefill:decode ≈ **130:1**. A 500-token
"agent thought" costs 50–100s of wall clock; a 500-token *prompt* costs under a second. Every
viable local-agent design on this host is therefore **prefill-heavy, decode-light**: short,
schema-constrained outputs (tens of tokens), aggressive exact-match caching (already built),
long-context reading tolerated, long-form generation avoided. Most published agent patterns
implicitly assume 10–100× more decode throughput; §4/§5 are filtered through this constraint.

### 1.3 Tool-calling and structured-output probes (live, decisive — the load-bearing new facts)

1. `qwen2.5-coder:7b` + OpenAI-compat `/v1/chat/completions` with a `tools` array → the model
   emitted the tool call as **plain JSON text in `content`** (`finish_reason:"stop"`, no
   `tool_calls` array). Same result on native `/api/chat`. **The coder model's Ollama template
   lacks tool wiring** — a naive OpenAI-style client would silently get prose instead of a call.
2. `llama3.1:8b` + native `/api/chat` with the same `tools` → **structured `tool_calls` array**
   (`{"function":{"name":"get_weather","arguments":{"city":"Kyiv"}}}`). Tool-calling *does* work
   on this host today — it is **per-model-template**, a `Caps`-probe fact, exactly as the harness
   plan's Quirks doctrine predicted (HARNESS-LLM-BACKEND.md §2.2 item 5).
3. `format` = full JSON Schema (Ollama ≥0.5, GBNF-backed constrained sampling) → **valid,
   schema-conforming JSON** from qwen2.5-coder on the first try (`{"order_id":42,"customer":
   "Olena","total_uah":540}`). **Model-independent**: this works even for models whose template
   lacks tool support, and it guarantees *syntactic* validity by construction (grammar-masked
   sampling), not by retry.
4. **Native logprobs work on 0.30.9**: `/api/generate` with `"logprobs":true,"top_logprobs":3`
   returned per-token `logprob` + ranked `top_logprobs`. The OpenAI-compat shim **drops** them
   (upstream issue closed "not planned") — so any logprob consumer must use the **native**
   endpoint, a concrete `Quirks` split for `OllamaAdapter`. This unlocks confidence-based
   routing (§4 G3) that the harness plan did not know it had.

### 1.4 What is already BUILT (file:line, re-read this session)

- **`LlmBackend` port** — `kernel/src/ports/llm.rs`: `ChatRequest` (`:44`) carries
  `model_id/messages/temperature/top_p/max_tokens/seed/task_class/cache_policy/options` —
  **no `tools` field, no `format` field, no tool-call response variant**; `Caps.tool_calling`
  (`:22`) exists but `llm-adapters/src/ollama.rs:59` pins it `false` ("not assumed").
  `CachePolicy` type-enforced (`:79`); `Usage::cost` = total_tokens (`:99`).
- **Harness composition** — `llm-adapters/src/compose.rs` (`StackBuilder` → Ollama + cache +
  workers + budget), `dispatch.rs` (`Dispatcher`, `TrackRecord:37` with
  `{backend,model,tokens,ms,task,success,value,cost}` — the gov_route-compatible EV row),
  `telemetry.rs` (`ModelStats`/`Telemetry::from_ledger` — per-model success-rate/mean-tokens
  folds). `kernel/src/token_bucket.rs` incl. `release()` (landed `f30189262`).
- **`AgentBridge` (inbound plane)** — mesh branch (`/root/dowiz-agentic-mesh`),
  `kernel/src/ports/agent/`: signed canonical-TLV `AgentManifest` (15 fields), `Admitter` with
  pre-crypto `AdmissionLimiter`, `SandboxTier::{WasmComponent,NativeProcessRequiresKvm}` caging,
  closed `(Resource,Action)` scopes with red-line deny, `AgentTask::{InvokeTool,ReadResource,
  RenderPrompt}` (`mod.rs:75`), capability-witnessed `invoke_depth`; `agent-adapters/` crate with
  `fuel.rs` prepaid-tranche Wasmtime fuel loop (deterministic meter default; real wasmtime behind
  `wasmtime-fuel` feature) and an MCP quirks profile.
- **`deliberate()` protocol shell** — `bebop2/core/src/deliberate.rs`: lap-capped (2) author↔mirror
  dialogue with agreement gate and least-friction tiebreak, substantive critique behind a
  **pluggable `Mirror` trait**. Today its only impl is the deterministic `ParamMirror`
  (`self_mod_loop.rs`) guarding one Kalman scalar. **This trait is the single readiest seam in
  either repo where a local LLM becomes an *agent participant* rather than a text service.**
- **hermes-kernel router (3rd repo, dev-tooling)** — `/root/hermes-agent-kernel-rewrite/
  hermes-kernel/`: `routing.rs` `classify_complexity` (`:67`), `rank_models_for_bucket` (`:114`,
  harmonic-centrality-based; 5 tests); `control.rs` `ev_route_select` (`:224`), Kelly/ruin
  (`:149,:158`), `lane_size` (`:182`), `pid_parallelism` (`:191`) (13 tests); CLI ops
  `op_classify_complexity`/`op_rank_models` present in `cli/src/main.rs` (verified by a Python
  read — plain `grep` misreads that file, likely a non-UTF-8 byte; noted so nobody "disproves"
  the ops with a grep). The shipped binary `dowiz/tools/telemetry/hermes-kernel` answers
  `{"op":"gov_route",…}` live (probed → `{"route":"ESCALATE"}`). **The wiring gap stands as
  memory recorded it**: `governance.sh` calls `gov_route/gov_lane/gov_meta/gov_decide` but never
  `classify_complexity`/`rank_models`; fan-out width is a hardcoded constant. Scope honesty
  (unchanged from the 2026-07-16 audit): this router is **dev-tooling for agent sessions**, not
  a dowiz product feature — what transfers to dowiz is the *pattern* (complexity bucket →
  EV-ranked model per bucket), consumed against `llm-adapters`' own `Telemetry`, not a dependency
  on the third repo.

### 1.5 What is DESIGN-ONLY (verified unbuilt, so this doc cannot lean on it)

P15's sub-hub spawn / `HubPolicy` self-mod / per-agent capability **minting** (all hard-gated on
Phase 10's kill-switch + `HubPolicy` entity — and the kill-switch is grep-verified absent,
BLUEPRINT-P15 §1); B2 `WorkReceipt`/`Settlement` (blueprint, hard-gated on the landed P07 fix);
B3's ledger half; E13-gpu (O18-gated). The Hermetic audit's RC-2 finding (verification organs
without independent teeth; P06 key_V unbuilt) also stands — relevant because an autonomous agent
loop is *another* self-certification risk surface if its "done" claims are self-supplied.

---

## §2 State of the art 2026 — multiple local agents on modest CPU hardware

Sourced via a delegated web-research pass this session (per-claim confidence preserved: 🟢
official/primary · 🟡 secondary · 🔴 low-trust/aggregator · ⚠ inference). Items marked **[LV]**
were additionally **live-verified on this host** in §1 — those outrank the literature.

### 2.1 Serving concurrency (CPU-only)

- **Ollama** 🟢: `OLLAMA_NUM_PARALLEL` (default 1; RAM cost = parallel × context),
  `OLLAMA_MAX_LOADED_MODELS` (default **3 on CPU** — matches our observed 2–3 resident),
  `OLLAMA_KEEP_ALIVE` (5min), `OLLAMA_MAX_QUEUE` (512→503). FIFO scheduling. Ollama exposes **no**
  KV-quant or per-slot control. **[LV]** default serialization confirmed (§1.2).
- **llama.cpp server** 🟢 (the engine under Ollama; `/usr/local/lib/ollama/llama-server` is on
  disk): continuous batching on by default, `--parallel N` slots, KV-cache quantization
  (`-ctk/-ctv q8_0` ≈ −47% KV RAM near-lossless 🔴), JSON-schema→GBNF native in C++. Direct
  `llama-server` use is the escape hatch if Ollama's coarse knobs ever bind.
- **Rust-native servers**: **mistral.rs** v0.9.0 (2026-07) 🟢 is the only credible all-Rust
  OpenAI-compatible server with CPU continuous batching — unbenchmarked for sustained CPU
  multi-model load; **candle** is a library (no server/batching); `llama-cpp-rs` is raw bindings.
  None is needed while the Ollama adapter stands (see DECART, §6).
- **RAM budget** ⚠(arithmetic): two Q4_K_M 7-8B models (~9.6GB weights) + runtime + KV at 4-8K
  ctx + OS ≈ 13–17GB → **fits with headroom**; the risk lever is **context × parallel slots**
  (KV grows linearly in both), not weight size. Adding a 1.5–3B third model (~1–2GB) is free at
  this budget. Speculative decoding on CPU: reported 1.5–3× 🔴, but the draft model taxes RAM and
  the same cores — ⚠ likely not worth it here; revisit only with measurement.
- Realistic single-stream decode for 7-8B Q4 on ~8 EPYC threads: public proxy ~12.5 tok/s 🔴;
  **[LV] this host measured 4.8–10.5 tok/s** — the live number wins.

### 2.2 Local model routing / cascades

- The routing literature (RouteLLM 🟢 arXiv:2406.18665, FrugalGPT 🟢 2305.05176, AutoMix 🟢
  2310.12963, "LLM Shepherding" 🟡 2601.22132) is **cloud-cost arbitrage**; ⚠ **nothing published
  routes between two locally-hosted CPU models** — a local small→large cascade here is novel
  territory, not a recipe to copy. LLMRouter (ulab-uiuc) 🟢 is Ollama-endpoint-aware but Python
  (pattern-only for us) and unbenchmarked for local-local.
- **Escalation signals that work here**: (a) **[LV] native logprobs are available** (§1.3.4) —
  mean/min token logprob and top-margin are the cheapest confidence proxies; (b) schema-parse or
  validator failure after a `format`-constrained call (free — the call already produces it);
  (c) verbalized-confidence + tiny-k self-consistency (CISC 🟢 2502.06233: k as low as 2 — but
  ⚠ at 5–10 tok/s each extra sample is seconds, so k>2 is rarely affordable); (d) task-class
  priors from the existing `Telemetry` ledger (per-model success rates per task label — already
  harvested by `Dispatcher`).
- NVIDIA "SLMs are the future of agentic AI" 🟢 (2506.02153): sub-10B models suffice for narrow
  agentic subtasks (parsing, tool-calling, structured output); heterogeneous small-first +
  selective escalation is the recommended architecture — independently validates the
  small-first/escalate design, though no adversarial rebuttal was located (flagged, not settled).

### 2.3 Tool-use / function-calling reliability of open 7-8B models

- BFCL is at **V4** (2026-04) 🟢 but per-model V4 numbers could not be scraped this session
  (JS-rendered) — any number quoted from memory or blogs is suspect (versions are not
  comparable). Known robust findings: nested calls are the hardest category (🟢 "no model above
  10%" on nested-AST, 2024 snapshot); small models over-trigger tools and fail
  irrelevance-detection (Llama-3-8B ~20% at correctly *declining* to call 🟢); hallucinated
  parameter names, fenced-JSON-instead-of-raw, one-of-N parallel calls answered 🔴.
- **Mitigation that matters on this host**: grammar-constrained decoding guarantees **syntax,
  never semantics** — the reliable pattern is (1) closed tool set, (2) `format`-constrained
  output against the tool-call schema **[LV works]**, (3) an application-level validator
  (unknown tool name / bad arg types → typed error), (4) one retry-with-repair carrying the
  validator error, (5) escalate to the larger model on second failure. Parallel/nested tool
  calls: **do not rely on them at 7-8B** — single-call-per-step loops only.
- Rust-native constrained decoding exists if ever needed beyond Ollama's server-side `format`:
  **outlines-core** (Rust crate, JSON-schema→FSM) and **llguidance** (Rust, ~50µs/token) 🟢 —
  named as future DECART candidates, **not** adopted (§6).

### 2.4 Multi-agent orchestration patterns (patterns only — the Python frameworks are off-limits)

- **Blackboard beats supervisor** for small-model teams: 13–57% relative improvement
  (🟢 arXiv:2510.01285, Google-coauthored) — and dowiz's **event-sourced WORM log is already a
  blackboard**: agents appending typed events to a shared, replayable log is this repo's native
  idiom, not an import.
- **LangGraph's real lesson** is not the framework but the shape: an explicit, serializable
  state machine **checkpointed at every transition** — which is *precisely* `event_log`/
  `fold`/`commit_after_decide`. ⚠ The strongest SOTA orchestration pattern is one this codebase
  already practices for orders; the gap is applying it to agent steps (§4 G1).
- **Context budgeting**: ≤200-token per-agent status entries, expand-on-demand (🟡 2604.07911);
  plain observation-masking/truncation beat LLM-summarization compaction in one study 🔴 —
  good news at 5 tok/s (compression-by-LLM is unaffordable anyway). Default `num_ctx` here is
  4096 **[LV]** — raiseable via `options` (already plumbed through `ChatRequest.options`).
- **Reflection** (Reflexion 🟢): demonstrated with GPT-4-class critics; ⚠ unvalidated with a 7B
  self-critic — treat "small model critiques itself" as RC-2-shaped self-certification risk, not
  a free win. The architecture answer already exists: `deliberate()`'s Mirror is *adversarial and
  separate*, and the Hermetic P7 rule (no self-certified done) applies to agent loops verbatim.
- ReAct vs plan-then-execute at 7B: no controlled study 🔴; ⚠ architectural argument favors
  **plan-once-then-execute-narrow-steps** for small models (the planner call is one expensive
  decode; each step is cheap and schema-bound) — and it matches the decode-scarcity constraint.

---

## §3 Where "local agents" sit in dowiz's architecture — the three-plane picture

The codebase today has **two of three planes** of an agentic hub:

| Plane | Direction | Status | Anchor |
|---|---|---|---|
| **Model plane** — call a local/managed LLM | outbound | **BUILT** | `LlmBackend`/`OllamaAdapter`/cache/dispatch/telemetry |
| **Foreign-agent plane** — admit, cage, budget an *external* agent | inbound | **BUILT** (mesh branch) | `AgentBridge`/`AgentManifest`/`SandboxTier`/fuel |
| **Resident-agent plane** — a loop that *is* an agent of this hub | internal | **ABSENT** | — (this document's subject) |

Nothing in dowiz, bebop2, or hermes-kernel closes a **plan→act→observe** loop: no component
takes a goal, produces a tool call via the model plane, executes it under a capability, feeds
the observation back, checkpoints the step, and decides continue/escalate/stop. Every *organ*
exists; there is no *organism*:

- cognition — `LlmBackend` (built, but tool-blind — G2);
- hands — `AgentTask::{InvokeTool,ReadResource,RenderPrompt}` + admission/caging (built for
  *bridged* agents; reusable as the resident agent's closed tool grammar);
- memory — `retrieval/` (BM25/PPR/trigram), `spine.rs` hash-chain records,
  `living_knowledge.rs` adapter (built as primitives; no agent-facing loop — G4);
- checkpointing — `event_log`/`fold`/WORM (built; the LangGraph-shaped substrate);
- budget — `TokenBucket` + `release` + fuel tranches (flow bound built; stock bound designed — G5);
- runaway detection — `markov.rs` attractor detector (built, advisory);
- adversarial check — `deliberate()` Mirror seam (built, deterministic-only today);
- routing — hermes patterns + `Telemetry` ledger (pattern built elsewhere; unwired — G3);
- governance — red-line scopes/`RedLinePolicy` (built); **kill-switch absent** (G8).

P15 §6 designs per-agent capability *minting* — but it presupposes agents exist to mint for.
This arc is the missing antecedent: the resident agent loop is what P15's capability/minting,
depth-cap, and budget machinery will eventually *bind to*. Conversely, P15 §1's ordering law
binds this arc: **an unbounded autonomous loop may not ship before the kill-switch exists** —
so everything proposed in §5 is session-scoped/advisory until P10 lands (G8).

---

## §4 Gap register — what "genuinely autonomous local agents" still lacks

Each gap: evidence → what closes it → dependencies. Ordered by leverage.

**G1 — THE gap: no resident agent loop (plan→act→observe executor).**
Evidence: §3 sweep — grep for any ReAct/plan-execute driver across all three repos finds none;
`deliberate()` is a dialogue *protocol*, not an executor; hermes agents are Python session
tooling. Closes with: one new `AgentLoop` (adapter-crate level, sibling of `Dispatcher`) that
(a) renders a goal + closed tool list into a prefill-heavy prompt, (b) obtains a
schema-constrained step via the model plane (G2), (c) validates and executes the step as an
`AgentTask` under a scope check, (d) appends a typed step event to the log (checkpoint =
event-sourcing, §2.4), (e) loops with observation-masked context, bounded by `TokenBucket`,
step-cap, capability-witnessed depth, and the markov attractor signal, (f) exits with a typed
outcome — never a self-certified "done" (the P7/RC-2 rule: any success claim that matters gets
an independent check, e.g. the Mirror or a deterministic validator, not the loop's own word).
Dependencies: G2 (hard); G8 bounds its autonomy tier.

**G2 — the model plane is tool-blind (the first hard sub-gap).**
Evidence: `ChatRequest` (`ports/llm.rs:44`) has no `tools`/`format`/tool-call response surface;
`ollama.rs:59` pins `tool_calling:false`; yet §1.3 proves the host serves structured
`tool_calls` (llama3.1 template) and schema-constrained JSON (`format`, all models) **today**.
Closes with: additive port extension — `tools: Vec<ToolDef>`, `format: Option<JsonSchema>`
(both plain structs, no serde in kernel), a `ChatResponse` tool-call variant, `Caps.tool_calling`
probed per-model (Quirks) instead of pinned false, native-endpoint Quirk for logprob-bearing
calls (§1.3.4), and the validate→repair-once→escalate policy (§2.3). Zero new dependencies.
Cache-key note: `tools`+`format` must join the Layer-A hash (the existing `BTreeMap`-canonical
request already anticipated a `tools` member — HARNESS-LLM-BACKEND.md §3.2).

**G3 — no runtime model routing/cascade (small-first, escalate-on-doubt).**
Evidence: `TaskClass` routing is static (Code→qwen, General→llama, `ollama.rs`); hermes'
complexity/EV router is built+tested but (a) in a dev-tooling repo, (b) unwired even there
(§1.4); no ≤3B model is pulled, so there is no cheap tier to route *to*. Closes with: a
`Router` in `llm-adapters` consuming the **existing** `Telemetry`/`ModelStats` folds
(per-model×task success/latency) + the §2.2 signal set (logprobs via native endpoint;
schema-parse failure; ledger priors), pattern-borrowed from `classify_complexity`/
`rank_models_for_bucket`/`ev_route_select` (reimplemented against `TrackRecord`, ~200 lines —
not a cross-repo dependency). Precondition: pull one small model (qwen2.5:3b-instruct ~1.9GB —
an F3/F27-shaped ingestion: record `{url, sha3}` at pull; operator go per §5). Honest note:
local-local cascades are unpublished territory (§2.2) — ship behind measurement, not belief.

**G4 — no agent-facing memory loop.**
Evidence: retrieval/spine/living-knowledge exist as kernel primitives with no consumer that
reads-before-acting and writes-after-acting on behalf of an agent. Closes with: two thin calls
inside G1's loop — pre-step recall (existing `retrieval` index over prior step events + spine
records; prefill is cheap) and post-step append (typed `RecordKind::Memory` spine record).
Reflection stays adversarial (Mirror), never self-graded (§2.4). Depends on: G1. Context
budget: 4096 default ctx with ≤200-token per-item status entries (§2.4); raise `num_ctx` via
`options` only with the §2.1 KV-RAM arithmetic in hand.

**G5 — budget is flow-only.**
Evidence: `TokenBucket` (+`release`) and fuel tranches bound *rate*; nothing bounds
*outstanding commitment stock* (B3's ledger half unbuilt); P15 per-agent budget minting gated.
Interim close: per-loop hard caps (max steps, max total tokens, max wall) enforced in G1 —
degrade-closed, typed refusal. Real close: B3 ledger (already blueprinted, Wave 2 of the mesh
arc) once B2's TLV freezes. Depends on: mesh arc sequencing, not this arc.

**G6 — no in-hub multi-agent coordination.**
Evidence: nothing coordinates two concurrent resident loops. Closes with: the blackboard the
repo already owns — concurrent `AgentLoop`s append step events to the same log and read each
other's status entries (§2.4 blackboard > supervisor evidence); the `Dispatcher`'s worker pool
(sized to `OLLAMA_NUM_PARALLEL`) is the natural arbiter for model access; no new topology
machinery. Cross-hub coordination stays B2 `WorkReceipt` (designed, unbuilt). Depends on: G1;
concurrency probe P-2 (§5) before any claim about 2-agent throughput.

**G7 — resident-agent tool execution has a narrower sandbox story than bridged agents.**
Evidence: `SandboxTier`+Wasmtime fuel cage *bridged* agents; a resident loop's tools run as
host-process calls. Interim close: the resident agent's tool set is exactly the closed
`AgentTask` grammar routed through the same scope/red-line checks (`RedLinePolicy::check`,
`Scope::touches_red_line`) — no free-form shell, no open-world tool names, red-line scopes
refuse structurally. Full close: run resident tools through the same `SandboxTier` cage the
bridge uses (design already on the shelf). Depends on: G1's tool-set decision.

**G8 — the kill-switch does not exist (P10), so autonomy is capped by law, not preference.**
Evidence: BLUEPRINT-P15 §1 (grep-verified absent). Consequence, stated as a rule this arc
inherits: every loop shipped before P10 is **advisory/session-scoped** — operator-invoked,
step-capped, budget-capped, no self-modification surface, every action an auditable event.
"Unbounded" resident agents are a P15-era capability, in P15's stated order (P10 → P5 → P9
first). This is not caution theater; it is the architecture's own M11 conditional.

---

## §5 Recommended build waves (research recommendation — no code in this arc)

**Probes first (falsifiable, cheap, this week — each is a command with a pass/fail):**
- **P-1** `OLLAMA_NUM_PARALLEL=2` service-env trial (operator act — mutates a running service):
  re-run §1.2's concurrent probe; PASS = aggregate tok/s ≥ 1.5× serialized baseline at ctx 4096.
  This is the SOTA-flagged "single most decision-relevant unknown" for G6.
- **P-2** small-tier pull + bench: `ollama pull qwen2.5:3b-instruct` (+ record `{url,sha3}`),
  measure decode tok/s + `format`-constrained tool-call validity rate over a 20-case fixture vs
  the 7B pair. PASS = ≥2× decode speed at ≥80% validity — the router's economic premise.
- **P-3** tool-template check: does a newer `qwen2.5-coder` tag (or `qwen2.5:7b-instruct`) emit
  structured `tool_calls`? Decides G2's per-model `Caps` table.

**Wave A — G2 port extension (unlocked now; zero new deps; additive to `ports/llm.rs` +
`llm-adapters`).** Done-check: a RED→GREEN roundtrip test — `tools`-bearing request to
llama3.1 returns a parsed typed tool call; same against qwen2.5-coder returns typed
`Unsupported`/fallback-to-`format` (never silent prose); logprob-bearing call via native
endpoint returns non-empty logprobs; cache key changes when `tools`/`format` change.

**Wave B — G1 minimal `AgentLoop` + G4 memory hooks (after Wave A).** Plan-once-then-execute,
single tool call per step, schema-constrained, event-checkpointed, `TokenBucket`- and step-capped,
markov-watched, Mirror-checked exit. First concrete consumer (value, not theater): an
**LLM-backed `Mirror`** for `deliberate()` — the adversarial critique slot is decode-light
(short objections), leverages the existing lap-cap protocol, and cannot self-certify by
construction. Done-check: a scripted 3-step goal executes end-to-end with every step present in
the event log and replayable; budget exhaustion mid-loop yields typed refusal, not a partial
silent result; the markov detector flags a planted 2-cycle.

**Wave C — G3 router (after P-2 + Wave A).** `Router` over `Telemetry` folds + logprob/parse
signals; hermes patterns reimplemented locally; escalation ladder small→7B→managed(Tier-0).
Done-check: on a fixed 30-task fixture, router ≥ matches all-7B success at ≤60% of its decode
seconds (measured via `TrackRecord.ms`, the ledger that already exists).

Waves A/C are parallel-safe after their probes; B depends on A. G5/G6-full/G7-full ride the
mesh arc's own sequencing. Everything is bounded by G8's advisory cap until P10.

---

## §6 DECART assessment (Integration Decart Rule — inline, before any adoption)

**Headline: the core gap-closers (G1, G2, G4, G6-interim, G7-interim) require ZERO new
dependencies** — they are additive Rust over `ureq`, the existing kernel primitives, and
Ollama's already-running daemon. No DECART is owed for them (same class as "std::thread/mpsc
dispatch — No" in HARNESS-LLM-BACKEND.md §5).

Named candidates evaluated and **not** adopted now:

| Candidate | Would solve | Decision | Probe (strongest case against the decision) |
|---|---|---|---|
| **Ollama server-side `format`** (present) vs **outlines-core / llguidance** crates | constrained decoding | **Ollama `format` — adopted, zero dep, [LV] GREEN** | server-side grammars vanish if a non-grammar backend is configured; the crates become a real DECART the day a `LlmBackend` without server-side constraints ships |
| **mistral.rs** as Rust-native server | KV-quant control, logprobs-in-shim, in-process serving | **Rejected for now** — duplicates a working vetted daemon (same reasoning as HARNESS §5 Decision 1); unbenchmarked CPU multi-model | if P-1 shows Ollama's coarse knobs cost >30% aggregate throughput vs llama-server/mistral.rs tuning, re-open with a measured table |
| **hermes-kernel as a dependency** | routing | **Rejected** — pattern-borrow (~200 lines vs cross-repo coupling of dev-tooling into product) | if the reimplementation drifts from the 18-test-covered original, a parity fixture (P2-style) is owed |
| **qwen2.5:3b-instruct weights** | small tier for G3 | **Adopt via P-2, operator go** — not a crate; F3/F27 `{url,sha3}` ingestion discipline applies (record hash at pull; verify-or-deny path remains the P15 §5 design) | 3B tool-validity may fall under 80% (P-2's falsifier) — then the small tier is embedding/classification-only |

**Banned-reason check**: no candidate above is decided by "industry standard/battle-tested";
each carries a falsifiable trigger to re-open.

---

## §7 The 2-question doubt audit (mandatory, `AGENTS.md`)

**Q1 — least confident (7 items, not rounded down):**
1. **Performance n is small**: decode 4.8–10.5 tok/s spans 2× across 4 probes (load state,
   prompt shape, co-resident model all uncontrolled). Treat as a range; P-1/P-2 fix the rigor.
2. **`OLLAMA_NUM_PARALLEL≥2` behavior is unmeasured here** — §1.2 only proves default
   serialization. G6's viability rests on P-1, which needs an operator service-env change.
3. **BFCL v4 per-model scores were not obtained** (JS-rendered) — every §2.3 reliability number
   is pre-V4 or secondary; our own 20-case fixture (P-2/P-3) outranks all of them for decisions.
4. **qwen2.5-coder tool-template failure was not chased to root** — a newer tag may fix it
   (P-3); the conclusion "per-model Caps probe required" holds either way.
5. **hermes claims were spot-verified, not exhaustively re-read**, and `grep` demonstrably
   misreads `cli/src/main.rs` (Python arbiter used) — any future audit of that file must not
   trust grep-negatives.
6. **SOTA citations carry the subagent's epistemics** (🟢/🟡/🔴 markers preserved); arXiv IDs
   were not independently re-fetched. Live probes were run precisely where a claim was
   load-bearing (logprobs, `format`, tool_calls, serialization).
7. **Layer-B semantic cache interaction with agent steps is unexamined** — agent-loop calls are
   likely `Exact`/`NoCache` territory (steps are state-dependent), but §4 G1 does not yet state
   the rule; implementation must, or a near-duplicate step observation could be served stale
   (the exact proxy-over-ground-truth failure §3.3 of the harness plan forbids).

**Q2 — the biggest thing being missed:** the honest blind spot is **economic, not
architectural**. At 5–10 tok/s decode, most of the "agentic" design space the literature
assumes is simply unaffordable on this host — a 10-step ReAct loop with 300-token thoughts is
~10 minutes of wall clock. The gap register above is real, but the *decisive* lever for "local
agents that actually run" may be the unglamorous P-2 (a 3B tier at 2-4× decode speed with
schema-constrained short outputs) plus caching — capability follows economics here, and a
resident-agent plane built before the economics are measured would be capability theater. A
secondary blind spot, named rather than hidden: dowiz ships **no production agent feature** —
this arc is hub-organism/harness work (M5/M11 lineage), and per the standing scope-honesty rule
it must not be sold as a product roadmap item; whether resident agents ever face customers is
an operator direction call, not an architecture inevitability.

---

*Written 2026-07-17 on `feat/harness-llm-backend`. Live probes this session: `lscpu`/`free`,
`ollama list`/`ps`, `/api/version`, `/v1/chat/completions`+tools (qwen2.5-coder),
`/api/chat`+tools (qwen2.5-coder, llama3.1), `/api/chat`+`format` JSON-schema,
`/api/generate`+`logprobs`, 2-concurrent serialization probe, `gov_kern` gov_route probe,
hermes `cli/src/main.rs` Python read. Code read: `kernel/src/ports/llm.rs`,
`llm-adapters/src/{ollama,compose,dispatch,cache,telemetry}.rs`, `kernel/src/token_bucket.rs`
(commit `f30189262`), mesh-branch `kernel/src/ports/agent/*` + `agent-adapters/src/fuel.rs`,
`bebop2/core/src/{deliberate,self_mod_loop}.rs`, `hermes-kernel/kernel/src/{routing,control}.rs`,
`tools/telemetry/governance.sh`. Companion docs: HARNESS-LLM-BACKEND.md (built plane),
AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md (inbound plane), BLUEPRINT-P15 (the eventual governance
binding), HERMETIC-ARCHITECTURE-PRINCIPLES.md (RC-2/P7 constraints on self-certifying loops).
No code written or edited.*
