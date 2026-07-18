# BLUEPRINT P21 — Local LLM on this host: native `/models`, the Hermes routing lane, resource wiring (P25/P26), and a real-time bench+eval harness (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9 — every point addressed).
> Promotes **P21 "Local AI / Local Agents"** (master roadmap §8.1 row 21, which today points only at
> `LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md`) to a full standalone blueprint at the same depth
> as `BLUEPRINT-P40/P41`. It EXTENDS that research doc (all of its §1–§7 findings inherited and
> re-verified where load-bearing) — it does not contradict or re-derive it. Siblings consumed, not
> rebuilt: **P40** (`kernel/src/agent/loop.rs` AgentLoop — the consumer of whatever backend this
> phase lists), **P41** (`AiMode`/`BackendConfig` — the mode substrate this phase plugs into),
> **P25** (`BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md` — admission classes),
> **P26** (`BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md` — `MemoryBudget`),
> **P45** (`BLUEPRINT-P45-ops-security-monitoring.md` §4b — the ONLY alerting mechanism used here).
>
> **Operator asks answered (verbatim intent):** (1) "план для підключення і використання
> локального llm на цьому ж сервері через hermes — добавити у список /models — native, враховуючи
> усі плани для чанкування та розподілення на ядрах, керування пам'яттю" — §3.1–§3.6. (2) "усі
> бенчмарки, evals, etc для перевірки та харнесу у реальному часі" — §3.7–§3.9. (3) model options
> "ollama, mistral, mixture of agents" + "Mixture-of-Experts/Mixtral — потрібно це, якщо є змога
> використовувати повністю локально" — §3.4, honest verdicts from live-measured numbers.
>
> **PART 2 appended same day (§11):** the operator's Tiered-Intelligence/SMMA proposal,
> reconciled and live-verified. NOTE: Part 1's disk-crisis numbers (90% full) are SUPERSEDED —
> disk was recovered to 65%/26GB free later this session (§11.6 lists exactly which verdicts
> that moves; TRIGGER-MIXTRAL is not one of them).

---

## 0. Ground truth — every cite re-verified live THIS pass (2026-07-18), standard §2 item 1

Working tree `/root/dowiz` (branch `main`, `f9b2eb9bb`) + `/root/hermes-agent-kernel-rewrite/`.

| Claim | Fresh evidence (this pass) | Status |
|---|---|---|
| Host: 8 vCPU = 4 physical cores × 2 SMT, AMD EPYC-Milan @ 2.0GHz, AVX2 (no AVX-512), **30Gi RAM (27Gi available, 0B swap)**, no GPU | `nproc`=8, `lscpu`, `free -h` run this pass; no-GPU per research doc §1.1 (`nvidia-smi` absent) — unchanged host | verified — same box as P25/P26's arithmetic |
| **Disk is the NEW binding constraint**: `/` 75G at **90% (7.6G free)**; `/mnt/volume-fsn1-1` 49G at 84% (7.8G free) | `df -h` this pass | verified — no prior P21-adjacent doc did disk math; §3.4/§3.5 do |
| Ollama **0.30.9 active** (`systemctl` → active), loopback `127.0.0.1:11434`; `GET /api/tags` live-probed → 4 models with `{size, quantization_level, context_length, capabilities}` per model | `ollama list` + `curl /api/tags` this pass | verified — `/api/tags` IS the native model-list mechanism (§3.2) |
| Models pulled: `qwen2.5-coder:7b` (4.7GB Q4_K_M, ctx 32768, caps completion+**tools**+insert per `/api/tags`), `llama3.1:8b` (4.9GB Q4_K_M, ctx 131072, caps completion+tools), `nomic-embed-text` (274MB F16), `qwen3-embedding:0.6b` (639MB Q8_0) | `/api/tags` JSON this pass | verified — note `/api/tags` now REPORTS `capabilities` incl. `tools`; reconcile with the per-template probe findings (research §1.3) at execution: the tags field is the daemon's claim, the live probe is ground truth |
| Measured perf (this host): decode **4.8–10.5 tok/s** (7-8B Q4, warm), prefill **~636 tok/s** (≈130:1), cold load 25–31s, warm 250ms, `OLLAMA_NUM_PARALLEL=1` serializes | research doc §1.2 (live-probed 2026-07-17; same daemon, same models — inherited one day old, not re-run) | inherited-fresh — re-run in T1's baseline seeding |
| `LlmBackend` port: trait at `kernel/src/ports/llm.rs:339-354` (`id/caps/chat/embed/rerank/health`) — **no `list_models` anywhere** (`grep list_models` → only a doc-comment false hit at `:360,365`); `AiMode`/`BackendConfig::from_env` landed `:164-299` (P41 substrate) | file read this pass (544 lines) | verified — §3.2's gap is real |
| `OllamaAdapter` (`llm-adapters/src/ollama.rs`, 85 lines): `route_model` Code→qwen2.5-coder / General→llama3.1 / Embedding→nomic (`:36-44`); `caps()` pins `tool_calling:false` (`:53-60`) | file read this pass | verified — G2 (tool-blind port) still open, owned by the research doc's Wave A, not this blueprint |
| Transport health = `GET {base}/v1/models` (`llm-adapters/src/transport.rs:110-112`) | grep this pass | verified — liveness probe exists; §3.9 reuses it |
| Dispatcher A4 defect still live: `workers` stored (`dispatch.rs:60,70`) never enforced (spawn-per-call `:85-87`) | grep this pass | verified — owned by Phase-27 F1b (HARNESS audit addendum), cited not re-fixed here |
| `kernel/src/agent/loop.rs` (651 lines): `LoopOutcome` `:71`, `run(&self, reasoner, user_request)` `:151` — P40's consumer surface | grep this pass | verified |
| **`kernel/src/memory_budget.rs` does NOT exist** — P26 §3.1's `MemoryBudget {try_reserve/release/reserved}` is a proposal; P26 §3.5 already names "(3) any future resident-agent plane (P21) sizing its own working set" as a caller | `ls` this pass + P26 read | verified — §3.5 consumes the design as specified, builds nothing parallel |
| **`kernel/src/admission.rs` does NOT exist** — P25 §3.6's admission fn is a proposal; P25 §3.5 defines the **L-class (local-inference)** admission class: daemon governor = `OLLAMA_NUM_PARALLEL` (auto ≤4) + `OLLAMA_MAX_QUEUE` (512→503); each in-flight local inference **counts against the C budget** (4 strict-core slots, `taskset -c 0,2,4,6`); class attaches to the work unit | `ls` this pass + P25 §3.5 read | verified — the "third class" question is ALREADY answered by P25; §3.5 consumes it verbatim |
| Hermes repo identity: `/root/hermes-agent-kernel-rewrite/` = NousResearch **hermes-agent** (Python multi-platform agent harness: TUI/gateway/Telegram/providers) + `hermes-kernel/` Rust crates (`kernel/src/routing.rs` 264 lines — HK-05 `classify_complexity`:67 + `rank_models_for_bucket`:114; `control.rs` 629 lines — `ev_route_select` etc.; `cli/src/main.rs` JSON-stdin bridge) | README + file reads this pass | verified — §3.1's ruling rests on this |
| **Hermes HK-05 wiring is NOW LIVE (memory-stale correction):** `agent/turn_context.py:187-191` calls `agent/model_routing.py::reorder_fallback_chain(agent, user_message)` **once per turn**, order-only, fail-open — the 2026-07-16/17 memory claim "gap = governance.sh doesn't call it yet" is stale for the hermes-agent side | file reads this pass | verified — fresh fact |
| Residual gaps that DO still stand: dowiz's `tools/telemetry/governance.sh` calls only `gov_route/gov_lane/gov_meta/gov_decide` (`:50,:74,:353`) — never `classify_complexity`/`rank_models`; and Hermes' `smart_model_routing.enabled` config key is **write-only** (set `False` by blank-slate setup `hermes_cli/setup.py:3071`, asserted in its test, consumed by NO runtime code — `reorder_fallback_chain` runs unconditionally) | greps this pass | verified — two honest open items, named in §3.3 |
| Hermes model list mechanism: `hermes_cli/models.py` `model_ids()`:1424 + `list_available_providers()`:1653; provider `"custom"` = any OpenAI-compat endpoint, **aliases `"ollama"/"vllm"/"llamacpp"` map to `"custom"`** (`models.py:1273`, `cli-config.yaml.example:36-37`); `model_aliases:` maps short alias → `(model, provider, base_url)` and is checked **BEFORE** the models.dev catalog (`cli-config.yaml.example:1237-1253`) | file reads this pass | verified — §3.3's exact insertion point |
| Ollama library sizes (live `curl ollama.com/library/*` this pass): `mistral:7b` **4.4GB** · `mistral-nemo` (12B) **7.1GB** · `mistral-small` (24B) **13–14GB** · `mixtral:8x7b` **26GB** / `8x22b` **80GB** · `qwen2.5:3b` **1.9GB** | live fetch this pass | verified — §3.4's model math uses these, not assumptions |
| MoA papers exist as cited: arXiv **2406.04692** "Mixture-of-Agents Enhances Large Language Model Capabilities" + arXiv **2502.00674** "Rethinking Mixture-of-Agents: Is Mixing Different Large Language Models Beneficial?" | both `<title>`s fetched live this pass | verified — §3.4's MoA verdict grounded on real papers |
| Bench/alert substrate: `kernel/benches/{criterion.rs, baseline.json, bench_track.py, BENCH_HISTORY.md}`; `llm-adapters/benches/{criterion.rs, baseline.json, BENCH_HISTORY.md}`; `tools/telemetry/logs/bench.jsonl` + `lib.sh` `log_event`; P45 §4b.3 nightly median-of-3 tracker, `BENCH_CONFIRM_RUNS=2` consecutive breaches → S1 Telegram, baseline-refresh ledger discipline | `ls`/greps this pass + P45 §4b.3 read | verified — §3.7/§3.9 extend THIS, fork nothing |
| Quality-eval statistics primitive: `kernel/src/stats.rs::wilson_interval` (`:100`, E2 spectral arc, landed) | grep this pass | verified — §3.8 reuses it |
| `spikes/living-knowledge` eval-memory.mjs is dead (confirmed earlier this session); the surviving pattern is the fixed-oracle discipline now native in `kernel/src/retrieval/{fixtures,recall}.rs` (12-query oracle) — a RETRIEVAL-recall eval, a different thing from LLM-response-quality eval | session finding + file list this pass | verified — §3.8 keeps the two distinct |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P21 owns vs what it must NOT do

**P21 owns (build items §3):**

| Item | Content |
|---|---|
| M-a | **Native `/models` on the port**: `list_models()` on `LlmBackend` (default `Err(Unsupported)`, additive), `OllamaAdapter` impl via native `GET /api/tags` — enumerates what is ACTUALLY runnable on this machine right now (id, size, quantization, ctx, capability claims) |
| M-b | **Hermes `/model` native entries**: `model_aliases` rows (`native`, `native-general`, `native-embed`) pointing at the loopback Ollama through Hermes' existing `custom` provider — the local LLM appears in Hermes' `/model` list and tab-completion, and its per-turn outcomes feed HK-05's per-bucket ranking (already wired) |
| M-c | **The Hermes-role ruling** (§3.1): Hermes = agent-harness + model-ROUTING/dispatch layer, never a model server — this determines the whole shape and is settled before anything else |
| M-d | **Resource wiring as a CONSUMER**: L-class admission per P25 §3.5, `MemoryBudget` reservations per P26 §3.1/§3.5, `OLLAMA_NUM_PARALLEL=2` EnvFile per HARNESS §4.1, plus the NEW disk-budget constraint (§3.5) |
| M-e | **Model-selection ruling** (§3.4): Ollama runtime confirmed; Mistral family sized against live numbers; Mixtral-MoE honest verdict + named upgrade trigger; Mixture-of-Agents deferred with named triggers |
| M-f | **Chunking (чанкування) resolution** (§3.6): which of the three plausible meanings are real here, each connected to its existing owner |
| M-g | **Real-time bench + eval harness** (§3.7–§3.9): `llm-bench` runner + `EVAL-20` fixture + P45 §4b.3 scope-list extension (three new tracked ids + one monitoring-table row) |

**P21 explicitly does NOT do (anti-scope, each review-rejectable):**

1. **NOT redesigning P25/P26 resource machinery.** The L-class admission rules and the
   `MemoryBudget` API are cited verbatim and consumed; a second admission predicate or a second
   byte-budget primitive is a fork and fails review. Where P26 is unbuilt, this blueprint states
   the reservation CONTRACT and waits — it does not land a parallel stopgap.
2. **NOT proposing a model that doesn't fit the live-measured specs.** Every size claim in §3.4
   carries the live `ollama.com` number and the live `df`/`free` number it is checked against.
   Mixtral's verdict (§3.4.3) is the worked example: preference never overrides measurement.
3. **NOT building a new alerting/monitoring mechanism.** P45 §4b.3 owns benchmark-regression
   alerting (nightly median-of-3, 2-consecutive-breach S1, Telegram tunnel, baseline-refresh
   ledger discipline); §3.9 EXTENDS its tracked-id list and its §4 monitoring table by rows.
   A second cron/checker/alert channel is the failure mode this anti-scope exists to block.
4. **NOT conflating retrieval-chunking with LLM-serving chunking.** `kernel/src/retrieval/`
   (BM25/PPR/diffusion over records) is an existing, distinct subsystem; §3.6 states the
   boundary explicitly — retrieval SELECTS chunks, the LLM lane consumes them as prefill.
5. **NOT touching P40's loop or P41's mode machinery.** The loop stays backend-blind
   (P41 §3.4's "parity by blindness"); `AiMode`/`BackendConfig` gain zero variants here;
   `list_models` is an additive port method with a fail-closed default, nothing more.
6. **NOT streaming.** Response streaming-in-chunks is P42's lane per P41 anti-scope 5 —
   flagged in §3.6, not designed.
7. **NOT making hermes-kernel a dowiz dependency.** The research doc's DECART already rejected
   it ("pattern-borrow, ~200 lines, not cross-repo coupling of dev-tooling into product") —
   that ruling stands; §3.1's two-lane split is how both sides get served without coupling.
8. **NOT shipping an autonomous resident loop.** G8's rule is inherited: everything here is
   operator-invoked/session-scoped/advisory until P10's kill-switch lands. This blueprint's
   deliverables are a listing surface, config rows, resource contracts, and a measurement
   harness — none of them acts on its own.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/ports/llm.rs — ADDITIVE extension (M-a). Plain structs, zero
// serde/HTTP in the kernel (compile firewall unchanged, llm.rs:3-7). ──────────

/// One locally-runnable model, as reported by the backend's own registry
/// (Ollama: native `GET /api/tags`, live-probed §0). Field semantics follow
/// that wire shape; the adapter converts, the kernel never parses JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// Backend-native id, `:tag` preserved verbatim (Quirk 1): "qwen2.5-coder:7b".
    pub id: String,
    /// On-disk size in bytes (drives the §3.5 MemoryBudget/disk arithmetic).
    pub size_bytes: u64,
    /// e.g. "Q4_K_M". Empty when the backend doesn't report one.
    pub quantization: String,
    /// Max context length the backend reports for this model.
    pub context_length: u32,
    /// Backend's CLAIMED capabilities ("completion","tools","embedding",…).
    /// A claim, not ground truth — Caps probing (research §1.3) outranks it.
    pub capability_claims: Vec<String>,
}

pub trait LlmBackend {
    // …existing six methods unchanged…
    /// Enumerate models actually present/runnable on this backend RIGHT NOW.
    /// Default fail-closed: a backend that cannot enumerate reports
    /// `Err(Unsupported)` — never an assumed/hardcoded list.
    fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Err(LlmError::Unsupported)
    }
}

// ── llm-adapters/src/bin/llm-bench.rs — NEW small binary (M-g, §3.7).
// Zero new deps (ureq already in-crate). Emits one JSON line per metric to
// stdout; the nightly wrapper appends to tools/telemetry/logs/bench.jsonl via
// the existing lib.sh log_event. ──────────────────────────────────────────────

/// One bench observation. Serialized by the ADAPTER-side binary (serde already
/// in llm-adapters), never by the kernel.
pub struct LlmBenchRow {
    pub bench_id: String,     // one of the BENCH_* ids below
    pub model: String,
    pub value: f64,
    pub unit: &'static str,   // "tok_s" | "ms" | "rate"
    pub n: u32,               // probes per sample (median taken, P45 §4b.3 shape)
}

// Tracked ids (P45 §4b.3 scope-list extension — these three rows join the
// existing bench.jsonl id namespace; thresholds live beside P45's consts):
pub const BENCH_LLM_DECODE: &str = "llm.decode_tok_s";      // per-model
pub const BENCH_LLM_PREFILL: &str = "llm.prefill_tok_s";    // per-model
pub const BENCH_LLM_TTFT1: &str = "llm.ttft1_ms";           // max_tokens=1 wall time
pub const BENCH_LLM_EVAL_PASS: &str = "llm.eval_pass_rate"; // per task-class, Wilson-LB

// Baselines to seed from THIS host's measured floor (research §1.2, re-run at T1):
pub const LLM_DECODE_FLOOR_TOK_S: f64 = 4.8;   // measured worst of 4 probes
pub const LLM_PREFILL_FLOOR_TOK_S: f64 = 500.0; // measured ~636, floor with margin
pub const EVAL_PASS_WILSON_LB_MIN: f64 = 0.80;  // P-2's own validity bar (research §5)

// ── llm-adapters/tests/eval_fixture.rs + fixtures/eval20.json (M-f, §3.8) ────
/// One deterministic eval case. Assertions are SHAPE checks (schema-parse +
/// validator), never an LLM judge (RC-2/P7: no self-certification).
pub struct EvalCase {
    pub id: String,
    pub task_class: TaskClass,          // existing enum, ports/llm.rs:29
    pub prompt: String,
    pub json_schema: String,            // Ollama `format` payload (GBNF-backed, [LV])
    pub expect: EvalExpect,             // required fields / enum values / ranges
}
pub enum EvalOutcome { Pass, FailParse, FailValidator(String), Unavailable }
```

```yaml
# ── Hermes side (M-b): cli-config.yaml `model_aliases` rows — the operator's
# "/models — native" made literal. Inserted per cli-config.yaml.example:1237-1253
# (aliases resolve BEFORE the models.dev catalog; provider "custom" = any
# OpenAI-compat endpoint, models.py:1273 maps bare "ollama" to it). No API key:
# loopback Ollama is unauthenticated; base_url pins loopback (P41's invariant
# honored on the Hermes side by construction — a non-loopback "native" alias is
# a review-rejectable config diff).
model_aliases:
  native:            # code-shaped default — the strongest local tool-former on disk
    model: "qwen2.5-coder:7b"
    provider: custom
    base_url: "http://127.0.0.1:11434/v1"
  native-general:    # tool_calls-capable template (research §1.3.2, live-verified)
    model: "llama3.1:8b"
    provider: custom
    base_url: "http://127.0.0.1:11434/v1"
```

**Rejected alternatives (DECART-style, one line each):** a NEW first-class Hermes provider
`"native"` — rejected: `custom` + aliases is the repo-sanctioned shape for local servers
(`cli-config.yaml.example:36-37` names Ollama explicitly; a provider fork duplicates catalog
machinery for zero behavior). `list_models` via OpenAI-compat `GET /v1/models` — rejected as
primary: live probe shows the native `/api/tags` carries `size/quantization/context_length/
capabilities` which `/v1/models` lacks, and §3.5's budget arithmetic needs `size_bytes`;
`/v1/models` stays the HEALTH probe (transport.rs:110-112, unchanged). A separate model-registry
file — rejected: the daemon's own registry is the single source of truth; a second list drifts.

---

## 3. Build items — spec → RED test → adversarial case (items 3, 5)

### 3.1 M-c FIRST — the Hermes-role ruling (this determines the whole shape)

**Verified verdict: Hermes is a ROUTING/DISPATCH layer over provider-served models — it is not,
and cannot be, a model server.** Evidence (§0): its providers are all OpenAI-compat HTTP
endpoints (OpenRouter/Portal/Azure/`custom`); local serving is explicitly delegated to external
daemons (Ollama/vLLM/llama.cpp/LM Studio) reached through `custom`; the Rust `hermes-kernel`
contributes pure DECISION functions (complexity classification, per-bucket EV ranking, Kelly/
ruin, PID parallelism) exposed over a JSON-stdin CLI. Nothing in either half loads weights.

**Therefore "local LLM via Hermes" decomposes into exactly two lanes, and only two:**

| Lane | What | Serving | Routing | Status |
|---|---|---|---|---|
| **Lane 1 — dev-harness (this machine's agent sessions)** | Hermes-agent sessions gain the local model as a first-class `/model` candidate | Ollama daemon (running) | HK-05 `reorder_fallback_chain`, **already wired per-turn** (`turn_context.py:187-191`) — outcomes accumulate per complexity bucket via `model_routing_history.py`, so Hermes LEARNS when `native` suffices vs when to fall back to a stronger remote model | §3.3 lands the aliases; routing needs zero new code |
| **Lane 2 — dowiz product (the kernel's own AI plane)** | P41 `AiMode::LocalOffline` → `OllamaAdapter` → P40 `AgentLoop` | Same Ollama daemon | The G3 router: reimplemented over `llm-adapters`' own `Telemetry`/`TrackRecord` folds, pattern-borrowed from HK-05 — **never a dependency on the hermes repo** (research §6 DECART, upheld) | port already shipped; router is research-doc Wave C, unchanged |

**What this ruling forbids:** routing dowiz product traffic THROUGH the Hermes Python process
(a Python bridge in the product's AI path violates the execution-substrate rule, STANDARD §1),
and treating Hermes as a serving runtime (it has no such capability). What it enables: the same
physical daemon serves both lanes; the L-class admission (§3.5) governs their combined load
because the class attaches to the work unit (P25 §3.5 rule 3).

**Two honest open items (fresh, §0):** (a) `smart_model_routing.enabled` is write-only config —
routing runs unconditionally; if the operator wants the flag to actually gate, that is a
one-line Hermes-side patch, recorded here as an OPEN decision, not silently made. (b) dowiz's
`governance.sh` still calls only `gov_route`-family ops — wiring `classify_complexity`/
`rank_models` into it remains the HK-05/HK-09 arc's own open item, cited not absorbed.

### 3.2 M-a — `list_models()` on the port: the native `/models` list (dowiz side)

**Spec:** §2's `ModelInfo` + default-`Err(Unsupported)` trait method (additive — existing
implementors, including P41's test `DeadBackend`, compile unchanged and stay fail-closed).
`OllamaAdapter::list_models` calls native `GET /api/tags` (live-probed shape §0), maps each
entry `{name→id, size→size_bytes, details.quantization_level→quantization,
details.context_length→context_length, capabilities→capability_claims}`. `:tag` ids verbatim
(Quirk 1). Consumers, in order of real need: the §3.5 budget arithmetic (`size_bytes`), the G3
router's candidate list (Wave C, research doc), the §3.7 bench runner (bench per listed model),
and P48's owner-hub settings surface (cross-reference only — no UI here, same contract as P41
§3.6 item 3: P48 reads this method through the facade, one writer one reader).

**RED→GREEN:** `llm-adapters/tests/list_models.rs::{live_tags_roundtrip}` — against the live
daemon: ≥4 models, every row has non-empty id + `size_bytes > 0`; the two known chat models
present by exact id. RED first (method absent → compile fail; then stub `Unsupported` → test
fail). **Adversarial:** (a) `systemctl stop ollama` → `Err(Unavailable)` typed, never a cached
or fabricated list (fail-closed listing — an empty-but-Ok result when the daemon is down is the
lying failure mode this test forbids); (b) a scripted malformed-JSON double → `BadRequest`,
never a panic; (c) kernel firewall unchanged: `cd kernel && cargo tree | grep -Ei
"ureq|serde_json"` → empty (P41 C-a's gate re-run — `ModelInfo` is plain data).

### 3.3 M-b — the Hermes `/model` native entries (Lane 1)

**Spec:** the §2 YAML lands in this machine's live Hermes config (`~/.hermes/`-resolved
`cli-config.yaml` — file location verified at execution, not assumed). Acceptance is
behavioral: `hermes model` / TUI `/model` tab-completion lists `native` and `native-general`;
selecting `native` and sending one prompt yields a completion whose provider resolves to
`custom @ http://127.0.0.1:11434/v1` (Hermes' own session log names the endpoint). Because
`reorder_fallback_chain` is already live per-turn, no routing code is added: outcomes for the
`native` aliases accrue per complexity bucket exactly like any other model, and HK-05's
per-bucket ranking (RED-proven in `routing.rs:217-235` — per-bucket, not global-average)
decides when the local model leads.

**Adversarial:** stop the daemon, select `native`, send a prompt — the turn must fall back
through Hermes' existing availability machinery (error taxonomy/cooldown, untouched — HK-05's
own boundary doc `model_routing.py:134-144`), never hang. **Anti-scope check:** zero edits to
`turn_context.py`/`model_routing.py`/`models.py` — this item is config-only by design; needing
a code edit means the design is wrong, stop and re-derive.

### 3.4 M-e — model selection: Ollama · Mistral · Mixtral-MoE · Mixture-of-Agents (honest verdicts)

#### 3.4.1 Ollama — ADOPTED (unchanged; it is the serving layer, not a model)

The runtime question was settled by HARNESS §5 Decision 1 (DECART table: running, MIT, probed,
reversible-as-a-port) and nothing in this pass disturbs it. Re-confirmed today: daemon active,
both API surfaces answering. Ollama remains the ONE local serving layer both lanes point at.

#### 3.4.2 Mistral (dense family) — NAMED CANDIDATES, sized against live numbers

| Model | Live size (ollama.com, this pass) | Fits 7.6G free disk? | Fits RAM budget? | Verdict |
|---|---|---|---|---|
| `mistral:7b` | 4.4GB | **barely** (leaves ~3G) | yes (same class as the resident pair) | CANDIDATE — adopt only via a P-3-shaped probe: does its Ollama template emit structured `tool_calls`, and does it beat `llama3.1:8b` on the EVAL-20 fixture? A third 7B that loses the probe is 4.4GB of disk this box cannot spare |
| `mistral-nemo` (12B) | 7.1GB | **NO** (7.6G free — no safety margin) | yes (~8GB resident) | BLOCKED BY DISK — named trigger: ≥12GB free on the model store's filesystem |
| `mistral-small` (24B) | 13–14GB | NO | fits RAM arithmetically but decode ≈ ⅓ of the 7B's 4.8–10.5 tok/s → ~1.6–3.5 tok/s, below usable even for batch | REJECTED for this host |

**The immediate, previously-unstated constraint is DISK, not RAM** (§0: `/` at 90%). Before ANY
new pull: either free space or move `OLLAMA_MODELS` — and the volume (7.8G free) is no escape.
Every pull follows F3/F27 ingestion (`{url, sha3}` recorded — HARNESS §5 Decision 1's named gap).

#### 3.4.3 Mixtral (Mixture-of-EXPERTS — a single sparse model) — operator-preferred; honest verdict: DOES NOT FIT THIS BOX

The operator's requirement, taken seriously: Mixtral IS wanted **if** runnable 100% locally,
zero cloud. The live numbers, checked in that order:

- **Sparse ≠ small-resident.** Mixtral 8x7B activates ~13B params/token (2 of 8 experts), but
  ALL expert weights must be resident — routing picks different experts per token. Q4_K_M =
  **26GB** (live, this pass); 8x22B = 80GB (out of the question).
- **Disk:** 26GB pull vs **7.6GB free** on `/` (7.8GB on the volume). **It cannot even be
  pulled onto this machine's current disks.** Gap: ~18GB.
- **RAM:** 26GB weights + KV + daemon overhead vs **30GB total** — technically loadable only by
  evicting both resident models AND zeroing P25 §3.4's `MEM_AGENT_BUDGET = 16GB` (D_max
  collapses toward 0), with 0B swap meaning the failure mode is the OOM-killer, not paging.
  Degrade-closed forbids exactly this shape.
- **CPU-only speed** (no GPU, confirmed): ~13B active on 4 physical cores that do 4.8–10.5
  tok/s at 7-8B, with a 26GB working set defeating cache locality → realistically ~2–4 tok/s ⚠
  (estimate, flagged) — marginal even for offline/batch.

**VERDICT: on THIS host, Mixtral fails on disk (26 needed / 7.6 free) and on the RAM *budget*
(26 weights / 30 total with 16 already committed to agent dispatch), independent of preference.
The requirement is not silently downgraded — it converts to a NAMED UPGRADE TRIGGER:**

> **TRIGGER-MIXTRAL:** when the deployment host offers **≥ 64GB RAM** (26GB weights + KV +
> P25's 16GB agent budget + OS, with margin) **and ≥ 30GB free model-store disk**, Mixtral
> 8x7B Q4 becomes the preferred local model per the operator's standing preference, entering
> through the SAME `OllamaAdapter`/`list_models`/EVAL-20 path with zero code change — that
> portability is what this blueprint buys. Until then the Wave-0 default stays the resident
> pair (`qwen2.5-coder:7b` + `llama3.1:8b`), with `qwen2.5:3b` (1.9GB, fits disk) as the
> small routing tier (research P-2) and `mistral:7b` as the probe-gated dense candidate.

#### 3.4.4 Mixture-of-AGENTS (MoA — an orchestration technique, NOT Mixtral) — DEFERRED, named triggers

MoA (arXiv 2406.04692, live-verified title §0) is layered multi-model orchestration: N proposer
models answer, an aggregator synthesizes — a per-response cost of **(N+1)× decode**. Distinct
from MoE in every way that matters here: MoE is one model with sparse weights; MoA is many full
model calls. Two findings decide it:

1. **Economics (measured, this host):** at 4.8–10.5 tok/s decode, a 3-proposer + aggregator MoA
   pass on ~300-token outputs is ~2–4 MINUTES of wall clock per response — research §7 Q2's
   "capability follows economics" verdict applies verbatim. Interactive MoA is unaffordable.
2. **The literature already undercut the mixing premise:** arXiv 2502.00674 (live-verified)
   shows "Self-MoA" — aggregating repeated samples of the SINGLE best model — often beats
   mixing different models. On a host with one strong local pair, the case for mixing is
   thinner still.

**Where it WOULD belong — stated, because the fit is real:** Hermes' routing layer (Lane 1) is
architecturally the right home — it already holds the multi-model candidate list and per-bucket
track records, and HK-05's ranking would pick proposers. And dowiz already owns the honest
2-model core of the idea: `deliberate()`'s adversarial Mirror (research §1.4) — proposer +
independent critic, decode-light by design. **DEFERRED with triggers:** (a) the small tier
lands (P-2: `qwen2.5:3b` at ≥2× decode) making proposers cheap enough for OFFLINE/batch quality
passes (e.g. eval-case triage — never the EVAL-20 judge itself, which stays deterministic);
(b) O18 GPU unlock changes the decode economics wholesale. Adopting MoA today would be
capability theater; this paragraph is the citation to reach for when the triggers fire.

### 3.5 M-d — resource wiring: consume P25 + P26, add the disk budget (nothing redesigned)

**Admission class — settled by citation.** Local-LLM inference is **L-class** (P25 §3.5,
verbatim rules): (1) intra-daemon concurrency is delegated to `OLLAMA_NUM_PARALLEL` — set
**explicitly to 2** in the service EnvFile (HARNESS §4.1's standing policy; P-1's probe is the
pass/fail: aggregate tok/s ≥ 1.5× serialized baseline, else revert to 1); (2) each in-flight
local inference **counts against the C budget** (4 strict-core slots) — an agent lane waiting
on `native` is NOT a D-class lane no matter how I/O-shaped its own process looks; (3) the class
attaches to the work unit, so mixed waves admit per-lane. NOT C-class (we don't `taskset` the
daemon — it is a system service the admission function accounts for; P25 §2.3's `nice`/
`cpu.weight` remains P25's own complementary knob), NOT D-class (its wait IS this host's CPU),
and NOT a new fourth class — the third class already exists and this phase is its first
fully-specified consumer.

**Memory — the `MemoryBudget` contract (P26 §3.1, unbuilt; P21 is its named caller #3 per P26
§3.5).** When `memory_budget.rs` lands, the LLM plane's reservations are:

| Reservation | Bytes (this host, observed not assumed) | Reserve/release point |
|---|---|---|
| Model residency, per loaded model | OBSERVED resident, not disk size: `qwen2.5-coder:7b` ≈ 5.1GB, `llama3.1:8b` ≈ 5.6GB (`ollama ps`, research §1.1) — `list_models().size_bytes` seeds the estimate, `ollama ps` truths it | reserve at load-admission (before a cold call that would page a model in), release on keep-alive eviction |
| KV cache | `f(num_ctx × num_parallel)` — the lever P25 §2.1 names as THE risk (linear in both); at defaults (ctx 4096 × 2) small; any `options.num_ctx` raise must re-reserve | reserve at request admission when `num_ctx` exceeds default |
| Harness overhead (bench/eval runners) | ~0 (they use resident models; no separate reservation) | — |

Budget arithmetic cross-checked against P25 §3.4 (consistent, not parallel): 30GB total −
~10.7GB two-model residency − ~1–2GB KV/daemon − ~4GB OS/product ≈ **16GB `MEM_AGENT_BUDGET`**
— exactly the number P25 already committed. This blueprint adds no second arithmetic; it shows
the same one from the LLM side. Until `MemoryBudget` lands, the interim guard is what already
exists: `OLLAMA_MAX_LOADED_MODELS` (CPU default 3) + keep-alive eviction — named as interim,
not silently treated as sufficient.

**Disk — the new budget (this pass's finding).** Model store on `/`: 7.6GB free ⇒ pull budget
≤ ~4.5GB with safety margin. Standing rule: `list_models()`-reported total + candidate-pull
size must stay under (free − 3GB floor); a pull that would breach is refused BEFORE `ollama
pull` runs. This is a one-line check in the pull runbook (T6), not new machinery.

### 3.6 M-f — chunking ("чанкування") — three meanings, each resolved to its owner

The operator's phrase "усі плани для чанкування та розподілення на ядрах" — the
core-distribution half is P25 (§3.5 above). "Чанкування" has three plausible readings for LLM
serving; the two real ones are addressed concretely, the third deferred by citation:

1. **Context/prompt chunking (REAL, the primary meaning here).** This host's binding ratio is
   prefill:decode ≈ 130:1 (measured §0) — reading is cheap, writing is expensive. The design
   therefore: **retrieval selects, prefill consumes.** Long material is chunked and ranked by
   the EXISTING retrieval subsystem (`kernel/src/retrieval/` — BM25/PPR/diffusion; a distinct
   subsystem, anti-scope 4), and the winning chunks are assembled into ONE prefill-heavy,
   schema-constrained prompt (research §2.4's ≤200-token status-entry discipline). What is
   REJECTED with the same measurement: LLM-summarization compaction of long contexts —
   compression-by-LLM is decode-bound and therefore unaffordable at 4.8–10.5 tok/s (research
   §2.4's finding that plain truncation/observation-masking beat it is convenient, not just
   consoling). `num_ctx` raises ride `ChatRequest.options` (plumbed, §0) and pay §3.5's KV
   reservation — chunk-window sizing and memory budgeting are the SAME decision seen twice.
2. **Weight chunking / lazy loading (REAL, already owned — cite, build nothing).** llama.cpp
   under Ollama mmaps GGUF weights: pages fault in on first touch (the measured 25–31s cold
   load IS this page-in; 250ms warm proves residency). No dowiz-side weight-chunking design
   exists or is needed; the knobs are Ollama's (`keep_alive`, `OLLAMA_MAX_LOADED_MODELS`),
   already surfaced through `options`.
3. **Response streaming in chunks (DEFERRED by citation).** P41 anti-scope 5 assigns streaming
   to P42's lane; nothing here front-runs it.

### 3.7 M-g(a) — `llm-bench`: latency/throughput measured from the daemon's own counters

**Instrument-fit ruling (honest, before code):** criterion is the WRONG instrument for model
calls — it is built for µs–ms pure functions (its statistical model wants thousands of
iterations; a 10-second generation makes that absurd), and both criterion benches
(`kernel/benches`, `llm-adapters/benches`) stay exactly as they are for their hot paths.
`llm-bench` is instead a small adapter-side binary (§2) whose measurements come from **Ollama's
own native metrics** — `prompt_eval_count/prompt_eval_duration` (prefill tok/s),
`eval_count/eval_duration` (decode tok/s), `load_duration` (cold/warm load) — the fields the
research doc's probes already used, which are the daemon's ground truth rather than a
client-side stopwatch. TTFT proxy: wall time of a `max_tokens=1` call (`llm.ttft1_ms`) —
honest name for what it is, since streaming (real TTFT) is P42's.

**Protocol (P45 §4b.3's shape, reused not re-derived):** per listed chat model (from
`list_models()` — the bench sweeps what the box actually has), n=3 probes per metric, fixed
prompt set (a 60-token and a 600-token prompt — the 130:1 ratio makes both regimes worth
tracking), **median** recorded, one `LlmBenchRow` JSON line each → appended to
`tools/telemetry/logs/bench.jsonl` via the existing `log_event` (lib.sh:186-187 mechanism, per
P45). Baselines seeded from a fresh T1 run and pinned beside the existing baseline files with
the SAME baseline-refresh ledger discipline (P45 §4b.3 anti-gaming: refresh = explicit ledger
row).

**RED→GREEN:** run with daemon stopped → every row `Unavailable`, exit non-zero, zero rows
appended (a bench that fabricates rows when its subject is down is the cry-wolf failure —
P45's falsifier discipline). GREEN: live run appends ≥ 3 metrics × ≥ 2 models with plausible
values (decode within [1, 100] tok/s sanity bounds). **Adversarial:** plant
`OLLAMA_NUM_PARALLEL` mis-set to 8 on a scratch config and confirm the decode median visibly
drops (the bench must be able to SEE a resource-tuning regression — that is its entire job).

### 3.8 M-g(b) — `EVAL-20`: deterministic quality evals (a different thing from retrieval recall)

**Anti-conflation, stated once:** `kernel/src/retrieval/`'s 12-query oracle measures RETRIEVAL
recall; the dead `spikes/living-knowledge` eval-memory.mjs is its ancestor, and its surviving
pattern is exactly the **fixed-oracle + deterministic-assertion discipline** — which this item
reuses for a DIFFERENT target: LLM-response quality. The two suites never share fixtures or ids.

**Spec:** `llm-adapters/tests/fixtures/eval20.json` — the research doc's P-2 "20-case fixture"
promoted to a committed artifact. Per case (§2 `EvalCase`): a task-class-tagged prompt + a JSON
schema (served via Ollama `format`, GBNF-backed — live-verified GREEN, research §1.3.3) + shape
expectations (required fields, enum membership, numeric ranges). Grading is **parse +
validator only** — `EvalOutcome::{Pass, FailParse, FailValidator}` — NEVER an LLM judge (RC-2/
Hermetic P7: a model grading itself is self-certification; the deterministic validator is the
independent authority). Aggregate = pass-rate per (model × task-class) with the **Wilson 95%
lower bound** (`kernel/src/stats.rs::wilson_interval:100` — landed, reused) — at n=20 a raw
rate lies; the lower bound doesn't. Threshold: `EVAL_PASS_WILSON_LB_MIN = 0.80` (the P-2 bar).

**RED→GREEN:** commit the fixture with ONE case whose schema the current model provably fails
(a deliberately-breaking case, standard item 5) → suite reports 19/20-shaped output with the
failing id named; fix/replace the case only via a ledger-row'd decision. **Adversarial:**
(a) daemon down → `Unavailable` outcome per case, run marked VOID, never counted as 0% (a
false-RED page) nor 100% (a lie); (b) the same fixture run twice at `temperature=0` +
`seed` must agree case-for-case (determinism check — flaky cases are spec bugs, fixed in the
fixture, per HARNESS §7's TDD rule).

### 3.9 M-g(c) — the real-time lane: THREE rows added to P45's machinery, zero new mechanisms

P45 §4b.3 already runs the nightly on-box cron → `bench.jsonl` → `ops-alert bench-drift`
median-vs-baseline check → **2-consecutive-breach S1 Telegram** → recovery S2 → weekly digest
with Δ-vs-baseline-date (the boiling-frog counter). This blueprint's ENTIRE alerting design is
a scope-list extension of that:

| New tracked id | Baseline (seeded T1) | Breach rule (P45 consts, reused) |
|---|---|---|
| `llm.decode_tok_s.<model>` | measured median (floor 4.8) | median **below** baseline × (1 − PCT/100), 2 consecutive nights → S1 (direction inverted vs time-benches; the checker already compares medians — the invert is a flag, not a fork) |
| `llm.ttft1_ms.<model>` | measured median | standard above-baseline breach → S1 |
| `llm.eval_pass_rate.<class>` | Wilson-LB from T1 run | LB < 0.80 two consecutive nights → S1 |

Plus ONE row in P45 §4's full-layer monitoring table (`:487-491` shape): **AI-plane liveness**
— the existing `health()` probe (`GET /v1/models`, transport.rs:110-112) folded into the
nightly run; daemon down at bench time → the VOID/Unavailable path (§3.7/§3.8) plus an S1
naming `ollama.service`, via the same tunnel. **Nothing else.** No new cron, no new checker
binary beyond the `llm-bench`/eval runners that PRODUCE rows, no second alert channel — the
P45 anti-scope (this blueprint's anti-scope 3) is load-bearing.

The regression this design would have caught, worked P45-style: a silent `OLLAMA_NUM_PARALLEL`
or `num_ctx` change (or a model re-pull behind the same `:tag` — mutable tags, HARNESS §3.2's
digest-invalidation warning) shifts decode median or eval pass-rate → 2 nights → S1 with the
commit range. Today such a change would be discovered only anecdotally.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

- **Silent egress:** unchanged and inherited — P41's `NonLoopbackLocal` typed refusal +
  one-constructor grep guard govern Lane 2; Lane 1's aliases hard-pin a loopback `base_url`
  in reviewed config (a non-loopback "native" alias is a reviewable diff, and Hermes-side
  provenance never enters the dowiz product path per §3.1's ruling). Nothing in this phase
  constructs a backend.
- **Resource exhaustion (this phase's own hazard):** unreachable-by-arithmetic rather than
  policed — the daemon's governor bounds in-daemon concurrency (NUM_PARALLEL=2, MAX_QUEUE 503
  = a typed refusal, not a stall); L-class counts inference against the 4 C-slots so admission
  cannot over-dispatch CPU; `MemoryBudget.try_reserve` (when landed) makes an over-RAM model
  load a refused state; the §3.5 disk rule makes an over-disk pull a refused act. The 0B-swap
  OOM cliff is why §3.4.3's verdict is structural, not preferential.
- **Self-certification (RC-2/P7):** the eval harness's grader is a deterministic validator —
  the model cannot pass its own eval by persuasion; `capability_claims` are explicitly labeled
  claims, outranked by probes; bench numbers come from daemon counters, not the harness's
  self-report.

### 4.2 Schemas & scaling axes (item 8)

`ModelInfo` scales by models-on-disk (single digits here; the disk budget caps it long before
any data-shape concern — stated axis: a hub with 100+ models would want pagination on
`/api/tags`, which is the day `list_models` gains a filter arg, not before). `eval20.json`
scales by cases: at ~50+ cases the Wilson bound tightens enough to raise
`EVAL_PASS_WILSON_LB_MIN` — a ledger decision, not a silent edit. `bench.jsonl` rows: +~12
lines/night (3 ids × 2 models × 2 prompts) — noise for the existing JSONL infra.

### 4.3 Isolation / bulkhead (11), mesh (12), rollback (13), living memory (15), tensor/eqc (16)

- **Bulkhead:** the daemon IS the bulkhead — an external process whose death yields typed
  `Unavailable` everywhere (port, bench, eval, Hermes fallback chain) and whose queue overflows
  as HTTP 503, never back-pressure into the kernel. P41 C-e's in-process proof (orders flow
  with the assistant dead) covers this phase's failure mode by construction.
- **Mesh:** everything here is **node-local** — model lists, aliases, benches, budgets. No
  protocol message carries a model list; a hub's models are its operator's business (M5).
  Cross-hub model discovery is explicitly NOT designed (it would be a capability-advertisement
  surface with Sybil questions — B-arc territory, flagged only).
- **Rollback vocabulary (precise):** Self-Termination leg only — over-budget loads/pulls/
  dispatches are refused states; config rollback = alias/EnvFile revert, stateless. No
  Self-Healing claim (an OOM-killed daemon is restarted by systemd, which is systemd's claim,
  not this design's); no Snapshot-Re-entry claim (nothing here has epochs).
- **Living memory (15):** the real temporal pattern is the track ledgers — Hermes'
  `model_routing_history` and dowiz's `TrackRecord`/`Telemetry` folds accumulate per-bucket
  outcome history that the routers consume; both exist, both cited; no new store.
- **Tensor/spectral/eqc (16):** honest N/A for serving; the one genuine reuse is HK-05's
  harmonic-centrality ranking (already built on the shared `centrality` primitive) — consumed
  via Lane 1, pattern-borrowed in Lane 2's Wave-C router.

### 4.4 Linux-discipline verdicts (item 9)

`list_models` = **ALREADY-EQUIVALENT** (fail-closed capability discovery extended by one
method; same shape as `caps()`). L-class/MemoryBudget consumption = **REINFORCES** (P25/P26's
own discipline, consumed at their stated seams). The bench-from-daemon-counters choice =
**REINFORCES** (ground-truth-over-proxy applied to measurement). The disk-budget rule =
**EXTENDS** (no prior budget covered the model store; justified by a live 90%-full reading —
the existing pattern was shown insufficient by measurement, item 19 satisfied). MoA/Mixtral
adoption = **DOES-NOT-TRANSFER today** (both fail this host's measured constraints; named
triggers recorded).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (before) | GREEN (after) | Named permanent check (item 17) |
|---|---|---|---|
| M-a list | compile-fail → `Unsupported` stub fails live test | `live_tags_roundtrip` green (≥4 models, exact ids); daemon-stopped → typed `Unavailable`; kernel `cargo tree` firewall unchanged | `llm-adapters/tests/list_models.rs::{live_tags_roundtrip, down_is_unavailable, malformed_is_badrequest}` |
| M-b Hermes | `/model` list lacks `native` | `native`+`native-general` listed + one live completion resolving to the loopback endpoint; zero Hermes code edits (`git -C hermes… diff --stat` → config only) | config diff + a session-log acceptance note (Hermes side has no dowiz CI; recorded in the ledger row) |
| M-d resources | — (consumption contract) | EnvFile sets `OLLAMA_NUM_PARALLEL=2` with P-1's probe output attached (≥1.5× aggregate or reverted); disk-budget rule present in the pull runbook; `MemoryBudget` rows in §3.5's table cross-referenced from P26 execution when it lands | P-1 probe output in ledger; runbook check T6 |
| M-e models | — (ruling) | §3.4 verdict table + TRIGGER-MIXTRAL recorded; any new pull carries `{url, sha3}` + probe result | ledger rows per pull |
| M-f eval | fixture's planted breaking case fails | 20-case run green minus declared-open cases; VOID-on-down proven; seed-determinism proven | `llm-adapters/tests/eval_fixture.rs::{eval20_run, down_is_void, seeded_runs_agree}` |
| M-g bench | daemon-stopped run exits non-zero, zero rows | live run appends rows; parallel-mis-set adversarial visibly moves the median | `llm-adapters/src/bin/llm-bench.rs` + its `tests/bench_rows.rs` |
| M-g alerts | planted regression (sleep-shim on the bench path / broken eval case) does NOT alert before wiring | same plant fires exactly one S1 after 2 nightly runs; 30-synthetic-nights noise replay fires zero (P45's cry-wolf falsifier, re-run over the new ids) | P45 §4b.3 tracker config + `ops-alert bench-drift` id list |

Ledger obligations (`docs/regressions/REGRESSION-LEDGER.md`): one row per new tracked id's
baseline seeding; one row for the disk-budget rule; one row recording TRIGGER-MIXTRAL and the
MoA deferral (so neither is re-litigated from scratch). All land with red→green proof before
P21 is called done.

---

## 6. Benchmark plan (item 10) — budgets first, measurements recorded, no estimates as facts

1. **Decode/prefill/TTFT1/load** — §3.7's protocol; budgets = the measured floors
   (`LLM_DECODE_FLOOR_TOK_S = 4.8`, prefill floor 500, cold load ≤ 35s, warm ≤ 500ms), all
   re-seeded from T1's fresh run on the current daemon, recorded in `BENCH_HISTORY.md`
   convention with the P45 refresh discipline.
2. **`list_models` overhead** — one loopback GET + parse; budget ≤ 50ms. Measured once in the
   live test, recorded; not nightly-tracked (it is not a hot path — honest scoping, P-A §6
   precedent).
3. **Eval fixture wall-time** — 20 cases × short schema-constrained outputs ≈ 20 × (prompt
   prefill ~1s + ≤50 decode tokens ~5–10s) → budget ≤ 5 min nightly; if measurement exceeds
   it, shrink outputs, never the case count (decode discipline, §3.6.1).
4. **Telemetry** — every dispatched call already emits a `TrackRecord` row; per-model×class
   stats fold via `telemetry.rs` — the router's food and the digest's per-model table come
   from the same rows. Zero new channels.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md` (the P21 research substrate this blueprint
promotes — §1.2 measurements, §1.3 probes, G1–G8 register, P-1/P-2/P-3 probes, Wave A/B/C, §6
DECART incl. the hermes-dependency rejection) · `HARNESS-LLM-BACKEND.md` (§1.2 daemon facts,
§2.2 Quirks, §4.1 NUM_PARALLEL policy, §5 DECARTs, audit addendum A3/A4) ·
`BLUEPRINT-P40-agent-loop-tool-wiring.md` + `kernel/src/agent/loop.rs` (consumer) ·
`BLUEPRINT-P41-three-mode-ai-operation.md` (AiMode substrate; C-g BYO; anti-scope 5 streaming
→ P42; loopback invariant) · `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md`
§3.5 (L-class — consumed verbatim) · `BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md`
§3.1/§3.5 (MemoryBudget; P21 as named caller) ·
`BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` (P24 gauge surface the
admission function reads) · `BLUEPRINT-P45-ops-security-monitoring.md` §4b.3/§4 (the alerting
mechanism — extended, never forked) · `BLUEPRINT-P-F-local-ai-mesh.md` (Layer-F rollup this
phase sits under) · `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.1 row 21 (the
index entry this deepens; pointer added there this pass) · arXiv 2406.04692 + 2502.00674
(MoA; titles live-verified). Memory files: `hk05-hk09-routing-status-2026-07-16` (**corrected
here**: turn-context wiring is now live; governance.sh gap and the write-only flag stand) ·
`model-routing-policy-2026-07-03` (the routing pattern's ancestry) · `verified-by-math-2026-07-07`
· `ground-truth-over-proxy-2026-07-07` (daemon-counter benches) ·
`performance-priority-over-minimal-change-2026-07-17` (why the harness is real machinery, not a
checkbox) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style). Supersedes: nothing —
additive over the research doc and the §8.1 index row.

---

## 8. Hermetic principles honored (item 20)

- **P2 CORRESPONDENCE:** one daemon, one listing authority (`/api/tags`), one alerting
  mechanism (P45), one admission model (P25) — every concept in this phase maps to exactly one
  existing primitive; the two lanes share the serving layer rather than duplicating it.
- **P6 CAUSE-AND-EFFECT:** every adoption/rejection in §3.4 is a function of a measured number
  (disk GB, RAM GB, tok/s) with the number cited; changing the verdict requires changing the
  measurement (TRIGGER-MIXTRAL is that statement as a trigger).
- **P7 GENDER (no self-certification):** benches read the daemon's counters, evals are graded
  by a deterministic validator, capability claims are subordinated to probes — in every pair,
  the certifier is external to the certified.

(Other principles not load-bearing here; not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points)

| §2 item | Where |
|---|---|
| 1 ground truth | §0 (incl. 3 fresh corrections: HK-05 wiring live, write-only flag, disk at 90%) |
| 2 DoD | §5 |
| 3 spec/TDD/event-driven | §2 types-first; §3 RED-first per item; bench/eval rows are events into the existing jsonl/ledger streams |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.2 (down/malformed), §3.3 (daemon-down fallback), §3.7 (parallel mis-set; fabrication refusal), §3.8 (planted breaking case; VOID-on-down), §5 M-g (noise replay) |
| 6 hazard math | §4.1 |
| 7 links | §7 |
| 8 scaling axes | §4.2 |
| 9 Linux verdicts | §4.4 |
| 10 benchmarks+telemetry | §3.7/§6 (measured floors, not estimates) |
| 11 bulkhead | §4.3 |
| 12 mesh | §4.3 (node-local, cross-hub discovery explicitly not designed) |
| 13 rollback vocabulary | §4.3 (Self-Termination only) |
| 14 error-propagation gates | §5's named tests; P45 breach rules; kernel firewall re-run |
| 15 living memory | §4.3 (track ledgers as the real temporal store) |
| 16 tensor/eqc | §4.3 (honest N/A + the one real reuse, HK-05 centrality) |
| 17 regression ledger | §5 ledger obligations |
| 18 agent instructions | §10 |
| 19 reuse-first | §2 rejected-alternatives; §3.5/§3.9 consume-don't-fork; §4.4 EXTENDS justified by measurement |
| 20 Hermetic | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repos: `/root/dowiz` (product) + `/root/hermes-agent-kernel-rewrite` (dev-harness; config-only
here). T1 first (it seeds every baseline); T2–T5 are collision-free after it.

1. **T1 — baseline seeding (first, everything depends on it).** Re-run the research §1.2
   probes fresh: per chat model, 3× decode/prefill via native `/api/chat` metrics, cold+warm
   `load_duration`, `ttft1` (max_tokens=1 wall). Record medians in
   `llm-adapters/benches/BENCH_HISTORY.md` + seed the three `llm.*` baselines. Acceptance:
   numbers within sanity bounds recorded with date+daemon version.
2. **T2 — M-a `list_models`.** Add §2's `ModelInfo` + default trait method to
   `kernel/src/ports/llm.rs` (additive; kernel stays HTTP-free); implement in
   `llm-adapters/src/ollama.rs` via native `/api/tags` (transport gains one native-GET helper
   beside the existing `/v1` calls — Quirks doc note). Write the 3 named tests RED-first
   (§5 row M-a). Acceptance: `cd llm-adapters && cargo test list_models` green;
   `cd kernel && cargo build` with zero new deps in `Cargo.lock`.
3. **T3 — M-b Hermes aliases.** Locate the live Hermes config (`hermes` CLI's own config
   path); add §2's `model_aliases` rows EXACTLY (loopback pinned). Verify `/model` listing +
   one live completion through `native`. Acceptance: session-log line naming
   `custom @ 127.0.0.1:11434`; `git diff` in the hermes repo shows config-only (if the config
   lives outside the repo, record the file path + contents in the ledger row instead).
4. **T4 — M-g bench+eval.** Create `llm-adapters/src/bin/llm-bench.rs` (§3.7 protocol) +
   `tests/fixtures/eval20.json` + `tests/eval_fixture.rs` (§3.8, incl. the planted breaking
   case). RED-first per §5. Acceptance: both suites' RED and GREEN outputs pasted in the
   landing commit; daemon-stopped runs behave per spec (VOID/non-zero, zero rows).
5. **T5 — M-g alert wiring.** Add the three tracked ids + the AI-liveness row to P45 §4b.3's
   tracker config and §4 table (P45's files, scope-list edits only — if the edit wants a new
   mechanism, stop: anti-scope 3). Run the planted-regression and 30-night-noise falsifiers
   over the new ids. Acceptance: exactly one S1 on the plant, zero on noise.
6. **T6 — M-d resource acts (operator-gated where marked).** (a) EnvFile
   `OLLAMA_NUM_PARALLEL=2` + run P-1's probe — **operator act** (mutates a running service);
   revert if <1.5×. (b) Add the disk-budget check to the model-pull runbook. (c) If pulling
   `qwen2.5:3b` (P-2) or probing `mistral:7b` (§3.4.2): check §3.5's disk rule FIRST, record
   `{url, sha3}` at pull, run EVAL-20 against it before any routing change. Acceptance:
   probe outputs + ledger rows present.
7. **T7 — close-out.** `cd kernel && cargo test --lib`; `cd llm-adapters && cargo test`; the
   kernel firewall grep (P41 C-a) unchanged; §5 rows all green or declared-open; ledger rows
   (incl. TRIGGER-MIXTRAL + MoA deferral) present. The Hermes-side acceptance (T3) has no CI —
   its evidence lives in the ledger row, stated plainly, never claimed as CI-covered.

---

# PART 2 — Tiered-Intelligence reconciliation (operator proposal, same day, 2026-07-18)

## 11. The operator's SMMA/tiered proposal, verified and reconciled against Part 1

> **Provenance.** The operator delivered a real engineering analysis (not prose): (a) Mixtral
> 8x7B is infeasible for 12 parallel swarm workers on ~32GB/8 cores by RAM arithmetic — an
> independent second angle on Part 1 §3.4.3's disk+budget rejection; (b) a three-tier
> "Tiered Intelligence" architecture (Tier 0 small always-resident router model · Tier 1
> lightweight-client swarm workers against ONE central inference server ("SMMA") · Tier 2
> heavier expert model for deep reasoning); (c) a model table (Gemma-2-9B-IT / Mistral-7B-v0.3 /
> Phi-3-medium-14B) with size and CPU tok/s estimates; (d) a priority-queue + prefill-batching
> serving design, with llama.cpp HTTP-server mode named as a candidate over Ollama; (e) a
> closing question: *"чи вистачить тобі однієї Mistral-7B для виконання всіх ролей, чи тобі
> критично потрібно розділяти моделі за спеціалізаціями?"* Every load-bearing claim below was
> re-verified live this pass; operator numbers are labeled as estimates wherever not
> independently corroborated. This part EXTENDS Part 1 — where a Part 1 number is superseded
> (disk state), §11.6 says so explicitly; nothing else in Part 1 moves.

### 11.0 Fresh ground truth (this pass, 2026-07-18 — supplements §0)

| Claim | Fresh evidence (this pass) | Status |
|---|---|---|
| **Disk is NO LONGER the binding constraint**: `/` now 65% used, **26GB free** (was 90% / 7.6GB in §0) | `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` §1 — 19 git-ignored `target/` dirs cleared this session, `df` verified 90%→65% | verified — §11.6 lists which Part 1 verdicts this moves (and which it does NOT) |
| RAM: operator says "32GB"; live is **30Gi total / 27Gi available, 0B swap** (§0) | `free -h` (§0, unchanged host) | verified — the ~2GB delta changes no verdict; all math below uses 30GB |
| Ollama `mistral` / `mistral:7b` latest tag **IS Mistral-7B-Instruct v0.3** ("A new version of Mistral 7B that supports function calling"), 4.4GB default quant | `ollama.com/library/mistral` fetched this pass | verified — the operator's "Mistral-7B-v0.3" and Part 1's `mistral:7b` are the SAME model; the operator's 6–8GB row is its Q8_0 quant, Part 1's 4.4GB is the default ~Q4 tag |
| Ollama request queue is **FIFO-only** (`OLLAMA_MAX_QUEUE` 512 → 503); **no request priority**; **no Unix-socket listener** (feature request open since 2023, unimplemented) | web search this pass: Ollama concurrency docs coverage + `ollama/ollama#739` (open) | verified — the operator's priority queue does NOT exist daemon-side; §11.3 puts it client-side |
| **Head-of-line-blocking report exists**: `ollama/ollama#14578` (2026-03, v0.17.5, open, maintainer-unconfirmed) — a ~300ms request to one loaded model waited ~50s behind a long generation on ANOTHER loaded model | issue fetched this pass | verified-as-reported — unconfirmed and on an older daemon than our 0.30.9; converted to a falsifiable probe (`llm.hol_block_ms`, §11.5), not assumed true |
| `llama-server` (llama.cpp): `--parallel/-np` slots + `--cont-batching` (default ON) + `--cache-prompt`/`--cache-reuse` prefix reuse + `-ctk/-ctv` KV-cache quantization + **Unix socket** (`--host *.sock`); **no request-priority mechanism either** | `tools/server/README.md` fetched this pass | verified — the ONE feature the switch was proposed for (priority) exists in NEITHER server |
| Live sizes for the operator's table: `gemma2:9b` default 5.4GB, `9b-instruct-q6_K` **7.6GB**, `q8_0` **9.8GB**; `phi3:14b` (q4) **7.9GB**; `phi3:mini`/3.8b **2.2GB** | `ollama.com/library/{gemma2,phi3}/tags` fetched this pass | verified — operator's Gemma 8–10GB ✓ accurate; operator's Phi-3-medium "10–12GB" overstates (live q4 = 7.9GB) |
| **Real swarm workload mix** (the evidence the closing question is answered from): `git log` 2026-07-17→18 on `main` shows ~15 kernel Rust feat/fix commits (CORDIC Q30, Kalman SoA AVX2, eigensolver, money-law overflow fixes marked `[RED-LINE money]`, HybridSigner, P40 AgentLoop, P42 MCP port), ~12 `docs(roadmap)` full-blueprint commits, ~14 `integrate(wave)` merge lanes, ~3 CI/grep-gate edits, ~2 audit/archaeology passes — **zero simple-classification payloads**; all authored by remote frontier-model lanes | `git log --since=2026-07-17` this pass | verified — §11.4.3's answer is grounded on THIS distribution, not a hypothetical one |
| P25's DECART **categorically rejected LLM-in-the-loop admission** ("ask an agent 'should I dispatch?' — ✗ violates LOCAL-DECISION rule outright") | P25 DECART table, re-read this pass | verified — load-bearing for §11.1 |

### 11.1 Tier 0 reconciled: the router is DETERMINISTIC CODE that already exists — the "small fast model" is a small WORKER, not the router

The operator's Tier 0 asks for "a small, always-resident, very fast model whose job is
understanding where to route a request." Verified against what exists: **both lanes already have
a router, and neither is (or should be) an LLM.**

- **Lane 1 (Hermes):** HK-05 `classify_complexity` (`routing.rs:67`) + `rank_models_for_bucket`
  (`:114`) — pure Rust, µs-scale, **already wired live once per turn** (`turn_context.py:187-191`,
  §0). It is "always-resident and very fast" by construction, because it is a function, not a
  model: zero RAM held, zero decode latency, deterministic, per-bucket outcome-learned.
- **Lane 2 (dowiz product):** the G3 router (research Wave C) — the same pattern over
  `TrackRecord`/`Telemetry` folds, equally deterministic.
- **Why a router MODEL is rejected, not just deferred:** (a) P25's DECART already rejected
  LLM-in-the-loop dispatch decisions categorically (§11.0 last row) — a routing decision is an
  admission-adjacent decision and inherits that ruling; (b) arithmetic: even the smallest
  candidate (Qwen2-1.5B, ~0.9GB) at this host's decode rates spends ~1–3 s per routing decision
  and holds RAM permanently, replacing a µs-scale pure function — a strict regression on every
  axis it was proposed to improve; (c) it would be a second competing router next to a live one
  (Part 1 anti-scope: resolve the overlap, never duplicate).

**What survives of Tier 0 — the small-worker tier (real, already named):** the legitimate need
under the operator's Tier 0 is *cheap classification/extraction sub-tasks* (semantic triage,
tagging, schema-fill) that heuristics can't do. Those are WORK, not routing: the deterministic
router dispatches them TO a small model like any other task. That tier already exists in Part 1
as the P-2 small tier (`qwen2.5:3b`, 1.9GB, live-priced §0). The operator's named candidates
slot into the SAME tier as probe alternates: `phi3:mini` (2.2GB live, but 4k context in its
4k variant — tight for the prefill-heavy §3.6.1 design) and Qwen2-1.5B (~0.9GB, two
generations behind the qwen2.5/qwen3 series). **Ruling: one EVAL-20 + decode probe picks at
most ONE small-tier model; `qwen2.5:3b` remains the front-runner; no model ever sits IN the
routing path.** Concretely, the architecture is:

```
            requests (both lanes)
                    │
        ┌───────────▼───────────┐
        │ DETERMINISTIC ROUTER   │  Tier 0 = code, not a model
        │ Lane1: HK-05 (live)    │  µs-scale, always-resident
        │ Lane2: G3 (Wave C)     │  learns from TrackRecord/history
        └───┬───────┬───────┬───┘
   simple   │       │       │  beyond-local
   bounded  │       │       └────────────► Tier 2 = REMOTE frontier lane
            ▼       ▼                      (Hermes fallback chain — exists)
      small tier   resident pair           §11.4.3: NOT a third local model
      qwen2.5:3b   qwen2.5-coder:7b
      (P-2 probe)  + llama3.1:8b        ── ALL local calls → ONE Ollama daemon
                   (mistral:7b v0.3        127.0.0.1:11434 (SMMA, §11.3)
                    probe-gated challenger)
```

### 11.2 Ollama vs llama.cpp-direct — resolved by live feature check, not assumption

The switch was proposed for three capabilities. Checked against both servers' actual 2026
surfaces (§11.0):

| Wanted capability | raw `llama-server` | Ollama 0.30.x | Verdict |
|---|---|---|---|
| **Priority queueing** | NO — slots + FIFO task queue, no per-request priority | NO — FIFO, `OLLAMA_MAX_QUEUE` → 503 | **Neither has it.** The switch buys nothing for its headline justification; priority must live client-side either way (§11.3) — and the client-side machinery already exists (P25) |
| **Unix-socket transport** | YES (`--host *.sock`) | NO (issue #739, open since 2023) | Real differentiator, worthless here: loopback-TCP overhead is µs–ms against 1–10 s generations — never load-bearing at this host's latency scale |
| **Explicit prompt-batching / cache control** | `--cont-batching` (default ON), `--cache-prompt`, `--cache-reuse N`, `-ctk/-ctv` KV quant | continuous batching + per-slot prompt caching inherited (Ollama's runner is llama.cpp-based); KV-cache quantization via daemon-global `OLLAMA_KV_CACHE_TYPE` + `OLLAMA_FLASH_ATTENTION` ⚠ (env-var surface from prior knowledge — verify at execution) | llama.cpp exposes FINER control; Ollama exposes ENOUGH for this host: at `NUM_PARALLEL=2` there is nothing for `--cache-reuse` tuning to win that the P-1 probe wouldn't show |

**VERDICT: stay on Ollama.** HARNESS §5 Decision 1's DECART (running, probed, reversible-as-a-
port) stands; the operator's proposed reason to leave (priority) exists in neither server, and
what llama.cpp-direct genuinely adds (Unix socket, per-run cache flags) is not load-bearing at
this host's measured latency scale. Switching would also discard the landed `OllamaAdapter`,
the native `/api/tags` listing (M-a), and model lifecycle management, for zero measured gain.
**Named reopening trigger — TRIGGER-LLAMACPP-DIRECT:** if the `llm.hol_block_ms` probe (§11.5)
reproduces #14578-shaped cross-model head-of-line blocking on OUR daemon (0.30.9) AND the
shallow-queue discipline of §11.3 cannot contain it, the first escape is **a second Ollama
instance on a second loopback port** (config-only, the workaround named in #14578 itself —
small-tier daemon + pair daemon); `llama-server`-direct is the escape after that, entering
through the same OpenAI-compat transport (adapter change ≈ base-URL + listing path).

### 11.3 The SMMA queue — P25's admission function IS the priority queue (one additive field, no fork)

Three of the operator's four serving claims are confirmed-and-already-true; the fourth is a
genuine small extension:

1. **"ONE centralized inference server, workers as lightweight clients" — already the
   architecture, now stated as an invariant.** Every local consumer (the `OllamaAdapter`, the
   Hermes `native` aliases, `llm-bench`, EVAL-20) is an HTTP client of the single daemon at
   `127.0.0.1:11434`; no worker process holds weights (§3.1 "the same physical daemon serves
   both lanes"). The operator's feared shape — 12 workers × one model copy each — is doubly
   unreachable: the daemon is the only weight-holder, and P25 §3.5 rule 2 (each in-flight local
   inference counts against the 4-slot C budget) means an admission-honest system never has 12
   local inferences in flight — at `OLLAMA_NUM_PARALLEL=2`, at most 2 decode while others wait.
   (Today's real 11-wave swarm never touched this path at all: its lanes were remote-model
   D-class clients, §11.0.)
2. **"KV-cache sized for one large shared context (12–16GB), not per-stream" — right spirit,
   wrong number for this box.** Keeping slot-count low so KV is shared-not-multiplied is
   exactly what `NUM_PARALLEL=2` already does. But a standing 12–16GB KV pool would consume
   P25 §3.4's committed `MEM_AGENT_BUDGET = 16GB` entirely — the §3.5 reservation table already
   prices KV honestly (`f(num_ctx × num_parallel)`, ~1–2GB incl. daemon overhead at defaults,
   re-reserved through `MemoryBudget` when a request raises `num_ctx`). Adopted in spirit,
   corrected in arithmetic; no number in P25/P26 moves.
3. **"Batching where queued requests share a prefill pass" — already daemon-owned.** Continuous
   batching is ON in the llama.cpp runner under Ollama; client-side prefill-merging machinery
   would duplicate it blind. The P-1 probe (aggregate ≥1.5× serialized at NUM_PARALLEL=2, §3.5)
   is the falsifier that says whether daemon batching pays on this host; build nothing until it
   has run.
4. **Priority — the real gap, closed as a P25 EXTENSION.** P25 §3.6's verdict is
   `Admit | Defer { retry_after_ms }` — admission-or-not, with no ORDER among deferred units;
   and the daemon's own queue is FIFO with no priority (§11.0). So the operator's priority
   queue lands as two rules in the EXISTING admission design, not a new queue anywhere:
   - **Additive type:** the L-class work unit gains a tag —
     `pub enum LlmPriority { Interactive, Harness, Batch }` — and deferred L-class units
     re-admit in `(priority, then age)` order instead of pure backoff-timer order. One field +
     one sort at the §3.6 re-admission point; `admission.rs` is still unbuilt (§0), so this
     lands as a spec amendment to P25 §3.6 now and one table-driven unit test when it lands.
     Until then the lead applies it manually, exactly like P25 §3.6's "callers" clause.
   - **Shallow-queue discipline (invariant):** at most `OLLAMA_NUM_PARALLEL` L-class requests
     are ever in flight to the daemon; NOTHING deliberately waits in the daemon's FIFO
     (`OLLAMA_MAX_QUEUE` remains an overflow backstop that 503s, never a scheduler). All
     ordering authority stays client-side where priority exists. This is also the structural
     mitigation for #14578-shaped blocking: a queue the design keeps empty cannot
     head-of-line-block, and the `llm.hol_block_ms` probe verifies the residual (in-decode)
     blocking is bounded.

   Is this "P25's I/O-bound-dispatch lane reused"? No — and P25 already answered why: L-class
   is its own third class (§3.5, "a C-class storm wearing a D-class label"). Local inference
   needed no fourth mechanism either: the distinct queue discipline the operator sensed is
   real, and it is exactly one enum + one invariant on the class that already exists.

### 11.4 Model verdict — the operator's table reconciled, and the closing question answered

#### 11.4.1 Version alignment + the table, corrected against live numbers

**`mistral:7b` ≡ Mistral-7B-Instruct-v0.3 — CONFIRMED** (§11.0: the library page's own "v0.3 /
latest / supports function calling"). Part 1's probe-gated candidate and the operator's table
row are the same model at different quants: 4.4GB default (~Q4) vs the operator's 6–8GB Q8_0.
On a memory-bandwidth-bound CPU host, Q8_0 nearly doubles the bytes read per token for a
marginal quality delta at 7B — the default ~Q4 tag stays the probe target.

Decode corroboration method (falsifiable, replaces trusting anyone's tok/s): CPU decode is
memory-bandwidth-bound, so `tok/s ≈ effective_bandwidth / weight_bytes`. Part 1's MEASURED
4.8–10.5 tok/s at 4.4–4.9GB implies this host sustains ~23–49 GB/s effective. Scaling:

| Model (live size, §11.0) | Operator's claim | Bandwidth-scaled prediction (this host) | Verdict |
|---|---|---|---|
| Mistral-7B-v0.3 ~Q4, 4.4GB | "6–9 tok/s, fastest, best tool-calling" | **measured class: 4.8–10.5** (Part 1 §0) | operator's rate ✓ consistent with measurement; "fastest" ✓ (smallest weights); tool-calling = the P-3 probe's question, not assumed |
| Gemma-2-9B-IT Q6_K 7.6GB / Q8_0 9.8GB | "~8–10GB ✓, 4–6 tok/s, best reasoning" | ~3.0–6.4 (Q6_K) / ~2.3–5.0 (Q8_0) ⚠ estimate | size ✓ accurate; rate plausible-to-slightly-optimistic; "best reasoning" is a claim EVAL-20 must earn, and §11.4.2 shows the RAM cost is the real problem |
| Phi-3-medium-14B q4, live **7.9GB** | "~10–12GB, 2–4 tok/s, 32–64k ctx" | ~2.9–6.2 ⚠ estimate | size row CORRECTED (live 7.9GB, not 10–12); rate plausible; its 128k-ctx tags exist but long-context prefill at 636 tok/s ≈ 3.4 min for 128k — context headroom this host can't actually afford to fill |

All ⚠ rows are estimates until a T1-style probe on this host; none is copied into any verdict
as fact — verdicts below rest only on sizes (live) and the measured 7-8B floor.

#### 11.4.2 The tier verdicts on THIS host (disk unblocked, RAM now the binding constraint)

- **Mixtral 8x7B — REJECTION STANDS, now doubly grounded.** Part 1 §3.4.3 rejected on disk
  (26GB pull vs 7.6GB free) + RAM budget. The disk leg has moved (26GB free) but STILL fails
  the §3.5 pull rule (free − 3GB floor = 23GB < 26GB). The operator's RAM math is the second,
  independent leg and it is decisive regardless of disk: ~26GB weights + per-stream KV on a
  30GB host leaves ~4–6GB for OS + embeddings + everything; on a generic 32GB box WITH swap
  that collapses to swap-thrash (operator's "1 token per 10–20s", breaking agent timeout
  chains); on THIS box with **0B swap** the failure mode is harsher — the OOM-killer, i.e.
  degrade-open. Same verdict from two directions; **TRIGGER-MIXTRAL (§3.4.3) unchanged**
  (≥64GB RAM ∧ ≥30GB free model-store disk).
- **Local Tier 2 (Gemma-2-9B-IT Q6_K / Phi-3-medium-14B) — NOT as a third resident model.**
  Resident math: 10.7GB (pair) + 7.6–7.9GB (tier-2) + ~1–2GB KV/daemon + ~4GB OS ≈ 24–25GB →
  `MEM_AGENT_BUDGET` collapses from the committed 16GB to ~5–7GB — it would break P25 §3.4's
  arithmetic to serve a tier whose decode (~2.3–6.4 tok/s ⚠) is SLOWER than the tier-1 pair it
  escalates from. As a **swap-in batch model** (evict one 7-8B, ~40–60s load ⚠ scaled from the
  measured 25–31s at 5GB) it is now disk-affordable (7.6GB ≤ 23GB pull budget) and allowed as
  a PROBE-GATED offline/batch candidate — legitimate only if a real workload materializes that
  the pair fails on EVAL-20-extended AND that tolerates minutes-scale turnaround.
  **TRIGGER-LOCAL-TIER2:** host RAM ≥ 48GB (third-resident becomes budget-honest) OR that
  probe-documented workload exists. Until then, **Tier 2 IS the remote frontier lane** that
  Hermes' fallback chain already provides — see the workload evidence next.
- **`mistral-nemo` (12B, 7.1GB):** Part 1's named trigger ("≥12GB free") **has now fired** via
  the disk cleanup. It joins `mistral:7b` as a probe-gated candidate under the same EVAL-20
  gate — noting its ~8GB residency prices it as a pair-REPLACEMENT (vs llama3.1:8b), never an
  addition, by the same §3.4 arithmetic.

#### 11.4.3 The closing answer: one Mistral-7B for all roles, or specialization?

**Answer: NO — one Mistral-7B does not cover all roles; but the split that earns its
complexity is NOT three local model-tiers. It is: deterministic router (code) + TWO resident
local dense models + tiny probe-gated small tier + REMOTE escalation.** Grounded in the real
workload distribution (§11.0), not a hypothetical one:

1. **The routing role needs NO model at all.** The largest "role" the operator's Tier 0
   assigns — knowing where to route — is already done in microseconds by HK-05/G3 code
   (§11.1). Giving it to any LLM, including Mistral-7B, is a regression.
2. **The dominant observed workloads exceed EVERY local candidate.** Today's real swarm output
   was kernel Rust with `[RED-LINE money]` invariants, crypto signers, AVX2 numerics, and
   ~12 full 20-point blueprints — work done by remote frontier lanes. No 7B, and no local 9-14B
   "expert tier", credibly holds these roles; pretending otherwise is capability theater
   (Part 1 §3.4.4's own standard). **The real Tier 2 already exists and is remote.**
3. **Within the roles local models CAN hold** — bounded tool-calling loops, structured
   extraction (schema-`format` output), EVAL-20/bench, ≤200-token status entries, draft
   passes — the measured task split is code-shaped vs general/tool-shaped, and the resident
   pair (`qwen2.5-coder:7b` + `llama3.1:8b`) IS that specialization, already on disk, already
   priced (10.7GB residency inside the §3.5 arithmetic). One Mistral-7B replacing BOTH would
   save ~5GB RAM at the cost of merging two measurably distinct capability profiles into one
   unprobed one — the wrong trade while the budget holds.
4. **Where Mistral-7B-v0.3 actually enters:** as Part 1 §3.4.2 already ruled — a probe-gated
   CHALLENGER: if it beats `llama3.1:8b` on EVAL-20 + the tool-calls probe, it takes the
   general/tools slot (pull now disk-unblocked, §11.6). If the operator ever forces a
   ONE-model constraint (e.g. a smaller future host), Mistral-7B-v0.3 is the right single
   pick — smallest weights, fastest measured class, native function calling — and that
   contingency is hereby recorded so it needn't be re-derived.

So: **specialization is critical, and it is already 80% built** — the missing 20% is the P-2
small-tier probe and the mistral:7b challenger probe, both EVAL-20-gated, both now unblocked.

### 11.5 Harness metrics for the tiered shape — named ids into P45 §4b.3 (extends §3.9's three rows; same mechanism, zero new)

| New tracked id | What it measures | Source | Baseline (seeded T1′) | Breach rule (P45 consts) |
|---|---|---|---|---|
| `llm.queue_wait_ms.<prio>` | admission→daemon-dispatch wait per `LlmPriority` class (median + p95) | dispatch log rows at the §11.3 admission point | T1′ run | p95 above baseline×(1+PCT), 2 consecutive nights → S1; `Interactive` additionally hard-capped: p95 > 5000ms is an immediate S1 (an interactive lane silently degrading to batch is the tiering failing) |
| `llm.queue_depth` | deferred L-class units at sample time | same log, sampled by the nightly runner | T1′ (expected ~0–2) | median > `NUM_PARALLEL × 2` two nights → S1 (standing backlog = admission mis-tuned or daemon degraded) |
| `llm.hol_block_ms` | head-of-line probe: wall time of a short (`max_tokens=8`) request to model A fired mid-generation of a long request on model B, minus its solo baseline | new `llm-bench` concurrent probe (the #14578 falsifier on OUR daemon) | T1′ | above baseline×(1+PCT) 2 nights → S1; first-ever run is the accept/reject evidence for TRIGGER-LLAMACPP-DIRECT's precondition |
| `llm.tier2_escalation_rate` | share of local-first turns that escalated to the remote lane | Lane 1: Hermes fallback-chain outcomes (`model_routing_history`); Lane 2: `TrackRecord` folds — both existing stores, §4.3 | T1′ | drift BOTH directions: above baseline band 2 nights → S1 (local tier failing its roles); ~0 for 7 days → S2 (escalation path possibly broken/fail-open — cry-wolf-checked like every P45 rule) |
| `llm.route_efficacy` | 1 − (fallback-after-route rate): fraction of routed turns whose FIRST-ranked model completed without falling back | same fallback-chain outcome rows | T1′ | below baseline×(1−PCT) 2 nights → S1 |

**Honesty note on "tier-0 routing accuracy":** true routing ACCURACY is unmeasurable without
ground-truth labels for "which model should have gotten this turn." `llm.route_efficacy` is the
honest outcome-grounded proxy (the router's pick sufficed / didn't) and is named as a proxy —
same discipline as `llm.ttft1_ms` standing in for TTFT (§3.7). Per-tier LATENCY needs no new
id: `llm.decode_tok_s.<model>` / `llm.ttft1_ms.<model>` (§3.9) are already per-model, and the
tiers are model-sets. All five rows ride the existing nightly runner → `bench.jsonl` →
`ops-alert bench-drift` → 2-breach S1 pipeline; the plant + 30-night-noise falsifiers (§5 M-g)
re-run over the expanded id list. No new cron, checker, or channel — anti-scope 3 still
load-bearing.

### 11.6 Amendments to Part 1 (explicit supersessions — nothing else moves) + DoD/task additions

| Part 1 claim | Status after this pass |
|---|---|
| §0: "`/` 75G at 90% (7.6G free)"; §3.4.2/§3.4.3/§3.5 disk math built on it | **SUPERSEDED**: 65% used, 26GB free (`BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md`). §3.5's disk RULE (pull ≤ free − 3GB floor) is unchanged and now yields a 23GB pull budget |
| §3.4.2 `mistral:7b` "barely fits" / disk-gated | pull now unblocked; the EVAL-20/tool-probe gate (the real gate) unchanged |
| §3.4.2 `mistral-nemo` "BLOCKED BY DISK — trigger ≥12GB free" | **trigger FIRED** — promoted to probe-gated pair-replacement candidate (§11.4.2) |
| §3.4.3 TRIGGER-MIXTRAL | **STANDS** — disk leg still fails the pull rule (26 > 23) and the RAM leg (≥64GB) is decisive; operator's swap/OOM math added as the second independent ground |
| §3.9 tracked-id list (3 rows) | extended to 8 rows by §11.5 (same mechanism) |

**DoD additions (extend §5):**

| Item | RED | GREEN | Named permanent check |
|---|---|---|---|
| M-h `LlmPriority` + ordered re-admission (P25 §3.6 amendment) | — (spec now; code lands with `admission.rs`) | table-driven test: deferred units re-admit in (priority, age) order | future `kernel/src/admission.rs` tests; until then the §11.3 invariant is applied manually and named in wave plans |
| M-i HoL probe | probe absent — #14578-shaped blocking would be invisible | `llm-bench` concurrent probe emits `llm.hol_block_ms`; daemon-stopped run stays VOID per §3.7 | `llm-adapters/src/bin/llm-bench.rs` (probe added) |
| M-j metric wiring | new ids absent from tracker | 5 ids tracked; plant fires exactly one S1, 30-night noise fires zero | P45 §4b.3 tracker config id list |

**Worker-task additions (extend §10; T8+ depends on T1/T4/T5):**

8. **T8 — probes now unblocked by disk.** (a) `ollama pull mistral:7b` (4.4GB ≤ 23GB budget;
   `{url, sha3}` ledger row per F3/F27) → run EVAL-20 + the tool-calls probe vs `llama3.1:8b`;
   swap the general/tools slot only on a win, ledger-rowed. (b) Same for `qwen2.5:3b` (P-2
   small tier; operator alternates `phi3:mini`/Qwen2-1.5B enter only if it fails). (c) Do NOT
   pull Gemma-2-9B/`mistral-nemo`/Phi-3-14B without the §11.4.2 trigger conditions in hand.
9. **T9 — `llm-bench` HoL probe (M-i).** Implement §11.5's concurrent probe; record the
   first-run verdict in the ledger as the TRIGGER-LLAMACPP-DIRECT precondition evidence.
10. **T10 — metric + amendment wiring (M-h, M-j).** Add the 5 ids to P45 §4b.3's tracker +
    re-run its falsifiers; append the `LlmPriority` amendment to P25 §3.6 (spec edit, cited
    back here); ledger rows: mistral-nemo trigger-fired, TRIGGER-LLAMACPP-DIRECT,
    TRIGGER-LOCAL-TIER2, and the one-model contingency ruling (§11.4.3 item 4).

**Compliance note (STANDARD §2, Part 2 scope):** ground truth §11.0 (item 1); DoD §11.6
(item 2); adversarial cases — HoL probe, cry-wolf-checked escalation-rate, hard-capped
interactive wait (item 5); hazard math — OOM-vs-swap distinction, shallow-queue invariant,
budget-collapse arithmetic (item 6); links — P25 §3.5/§3.6 + DECART, P26 §3.1/§3.4,
P45 §4b.3, `DISK-OPS-CLEANUP` §1, `ollama/ollama#739`/`#14578`, llama.cpp
`tools/server/README.md`, `ollama.com/library/{mistral,gemma2,phi3}` (item 7); reuse-first —
every §11 verdict consumes an existing mechanism or extends one by a field/row, and every
operator estimate is labeled ⚠ unless independently corroborated (item 19).
