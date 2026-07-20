# BLUEPRINT — Local Model Wiring, Testing, Training, and Usage

**Status: BLUEPRINT / PLAN — no code written, nothing built.**
**Date:** 2026-07-20
**Source of facts:** the completed Opus research pass on local-model wiring/testing/training (this session), verified against the live tree at `/root/dowiz`.
**Settled context this blueprint does not reopen:** the concurrency-architecture ruling from this session — the agent/LLM lane is synchronous, "async only where it brings value," LLM calls capped at ~2-way parallelism by the backend itself, no async runtime anywhere in this lane.

## 0. Scope honesty

This is the smallest-scope blueprint of the four in this batch, and it should be read that way. Most of it is (a) closing one already-identified wiring gap between two pieces of code that both exist and are both tested, and (b) one focused new test suite. There is no large build here. Where the research surfaced bigger topics (fine-tuning, routing cascades, supply-chain transparency), this blueprint's job is to rule them out or defer them explicitly, not to design them. Padding this into a major workstream would be dishonest; the honest framing is: **one wiring fix, one test suite, one config/documentation pass, one optional integrity tool.**

## 1. Architecture confirmation — what is locked in and must NOT change

The research's central structural finding: **"local model wiring" in dowiz is an HTTP-client-to-a-local-service problem, not an in-process ML-runtime problem.** This blueprint locks that in as the one architectural decision it settles rather than reopens.

Concretely, the following stays exactly as it is:

1. **`llm-adapters/` remains a synchronous HTTP client to a local inference server (Ollama or equivalent), never an in-process inference runtime.** Its external dependencies stay at `ureq` (no default features, TLS+json, no tokio) plus `serde`/`serde_json`. No candle, no llama.cpp bindings, no ONNX runtime, no GPU crate — none of that enters this crate or the kernel. This holds the default kernel build pure-`std`/serde-free (DECART discipline), keeps wasm32 viable, and means every future backend is a wire-format question, not a dependency question.
2. **The dispatch model stays as built.** `dispatch.rs` + `compose.rs`: `StackBuilder::default()` with `workers: 2` (mirroring Ollama's own `OLLAMA_NUM_PARALLEL`), the std-only non-blocking counting semaphore (`WorkerSlots`, degrade-closed `Busy` on overflow), and the `TokenBucket` volume budget (capacity 64, refill 8/s). The research confirmed the ~2-way cap is real and backend-imposed; async would buy nothing here. Settled this session; not relitigated.
3. **One fixed backend per deployment.** The `LlmBackend` port (`kernel/src/ports/llm.rs`: `id, caps, chat, embed, rerank, health`) stays single-backend; the only multiplicity is within a backend via `TaskClass` routing among its models. Multi-backend selection is routing-cascade territory and is deferred (§7).
4. **Agent-logic test infrastructure stays untouched.** `agent-loop`'s deterministic `ScriptedBackendMT` (canned `ChatResponse`s from a `VecDeque`) + `FixtureOrders`, with the one live-Ollama end-to-end test `#[ignore]`d. The research confirmed this separation is already correct: loop logic is tested deterministically; model output quality is a different problem (§3).
5. **New backends are Quirks presets, not new crates.** `VllmAdapter`/`ManagedApiAdapter` do not exist as types today; what exists is `Quirks::vllm()` / `Quirks::managed_api(api_key)` — wire-delta presets consumed by the one `OpenAiCompatTransport`. That is the intended extension mechanism and it stays the only one.

Known limitation carried forward unchanged, deliberately: `rerank` still returns `Err(Unsupported)` on Ollama. Fixing rerank belongs to the retrieval arc, not this blueprint; it is recorded here so nobody mistakes silence for ignorance.

## 2. The real wiring gap — `AiMode` becomes the actual composition switch

### 2.1 The gap, stated precisely

`kernel/src/ports/llm.rs` defines `AiMode { Off, LocalOffline, Connected }` and `BackendConfig::from_env` — a fail-closed three-mode selector (P41 C-b per its own doc comments). It is thoroughly unit-tested and its contract is strict:

- **Default `Off`** — unset `DOWIZ_AI_MODE` means no backend is constructed, no data egresses.
- **`LocalOffline`** — pins loopback (default base `http://127.0.0.1:11434`); a non-loopback base URL is a typed `ConfigError::NonLoopbackLocal` refusal, never a warning.
- **`Connected`** — requires both `DOWIZ_LLM_BASE_URL` and a readable key **file** at `DOWIZ_LLM_API_KEY_FILE` (never the key in env); partial config is a typed error (`MissingBaseUrl`/`MissingApiKey`), never a fallback to local or a default remote.
- `from_env` is the single non-test construction site for `Connected` — silent mode-escalation is unrepresentable by construction.
- `from_env_get(read)` is a dependency-injected resolver core, so composition tests need no global env mutation.

**But a repo-wide grep finds no consumer outside the port itself.** `llm-adapters/src/compose.rs` line 128 hardcodes `OllamaAdapter::new(&self.base)` and never consults `AiMode`. The port's own comment says the canonical home for this type is `llm-adapters` ("landed here first per the P41 wave; `llm-adapters` reuses it by import") — the reuse never happened. The operator's three-mode directive exists as a type but is not the switch. This is the concrete gap; closing it is Phase 1.

### 2.2 The wiring design

Add one constructor to `compose.rs`:

```
StackBuilder::from_config(cfg: &BackendConfig) -> Composed
```

where `Composed` is a two-variant result:

- **`Composed::Disabled`** — returned for `AiMode::Off`. No adapter, no transport, no thread pool is constructed. This matches the port's own documented semantics ("the agent surface is absent; no backend is constructed"). Callers treat `Disabled` as **feature absence, not failure**: every LLM-backed feature degrades closed — the agent surface simply does not exist in this process — rather than surfacing errors at call time. There is nothing to call. This is degrade-closed in the same sense as the existing `WorkerSlots` `Busy` behavior: a bounded, typed, non-erroring refusal.
- **`Composed::Ready(stack)`** — for the two live modes:
  - `LocalOffline` → `OllamaAdapter::new(&cfg.base_url)` — the existing constructor, now fed the loopback-verified base URL instead of a builder-supplied string. The loopback pin is enforced upstream by `BackendConfig`; composition never re-validates and never widens.
  - `Connected` → the OpenAI-compatible path: `OpenAiCompatTransport` + `Quirks::managed_api(api_key)` with `cfg.base_url` and the key loaded from file by `from_env`. Note this **necessarily creates the thin managed-backend constructor that does not exist today** — that is not scope creep; `Connected` mode is uncomposable without it, and the Quirks preset already exists, so this is a constructor, not a backend build (see open decision §9.2).

`BackendConfig` resolution errors (`UnknownMode`, `NonLoopbackLocal`, `MissingBaseUrl`, `MissingApiKey`) surface at composition time as typed errors before any adapter exists — misconfiguration is loud and early; only the deliberate `Off` state is silent-by-design.

The existing `StackBuilder` fluent API remains for tests and embedders that want explicit control; `from_config` becomes the path production composition uses. No new crate, no new dependency, no new env var — the env contract already exists and is already tested.

### 2.3 Companion fix: configurable `TaskClass` → model map

`ollama.rs::route_model` hardcodes `Code→qwen2.5-coder:7b`, `General→llama3.1:8b`, `Embedding→nomic-embed-text`, with a non-empty per-request `model_id` passed through verbatim, and `qwen3-embedding:0.6b` reachable only by caller override. The fix is minimal, matching the operator's "configurable" framing without building a model registry:

- Add `StackBuilder::model_for(TaskClass, name)` overrides (three optional strings), defaulting to the current hardcoded map — behavior is bit-identical when unconfigured.
- Optionally read `DOWIZ_MODEL_CODE` / `DOWIZ_MODEL_GENERAL` / `DOWIZ_MODEL_EMBEDDING` in `from_config`, same fail-closed spirit: unset means current defaults, set means verbatim pass-through to Ollama (which will error honestly on an absent model — no name validation layer invented here).
- The live `tool_calling` capability probe (`POST /api/show`, memoized, fail-closed) already handles arbitrary model names correctly; nothing changes there.

Explicitly **not** built: a model-registry system, model-file discovery, automatic pulling, capability-based model matching. The research did not ask for any of it and the deployment reality (four models physically resident, one operator, one box) does not justify it.

## 3. Testing — two distinct problems, one of which is already solved

### 3.1 Agent-logic testing: already correct, do not touch

`ScriptedBackendMT` + `FixtureOrders` deterministically cover what the loop does with a response: tool-call parsing, degrade paths, iteration caps. This is the right tool for that problem and it needs no changes. The only additions are the Phase 1 composition tests (§8), which are logic tests, not model tests.

### 3.2 Model-output-quality testing: the real hole, and the minimal fill

The research independently confirmed there is **no LLM-quality benchmark suite anywhere in kernel/tools/agent-*/llm-adapters** — nothing measures whether the actual pinned models, on this actual box, select the right tools for dowiz's actual tool surface. That is worth stating without softening: model-output quality is currently untested, entirely.

The fill is a **Rust-native golden tool-calling suite**, adopting BFCL's philosophy (score the structured call, not the prose — BFCL V4, July 2025, is the standard here) but scoped strictly to dowiz's own tool surface, not a general benchmark import:

- **Location:** `llm-adapters/tests/golden_toolcalls.rs`, every test `#[ignore]`d, run explicitly via `cargo test --ignored` — the exact placement pattern the existing live-Ollama test already established. Off the hot path, off default CI, zero new dependencies (`serde_json` is already in the crate for fixtures).
- **Fixture format:** a checked-in JSON file of cases: `{ prompt, tools_offered, expect }` where `expect` is one of:
  - `tool(name, required_args)` — the model must select exactly this tool and its arguments must contain these keys (AST-style structural check; argument *values* checked only where deterministic, e.g. an order ID quoted in the prompt);
  - `no_tool` — for deliberately ambiguous/underspecified prompts, the model must **not** silently pick a tool. Negative cases are first-class; a suite that only contains happy paths cannot detect the most dangerous failure mode (confident wrong tool selection).
- **Scoring:** per-case pass/fail on tool selection + argument structure. **Never on response text.** Temperature 0 where the backend honors it.
- **Regression tracking (technique (c) from the research):** each run appends one line — model IDs, per-case results, pass count — to a local scorecard `.jsonl`, so a model-version bump or prompt edit that silently degrades tool selection is visible as a diff. Per the repo's existing bench policy, host-noisy LLM probes are **pass/fail probes, not baseline-gated CI** — this suite is an operator-run/schedulable probe, advisory by design, and stays out of the deterministic gate set.
- **No LLM-as-judge anywhere in this suite.** The research is unambiguous that judges are unreliable in isolation as of 2025-2026 (pervasive verbosity/position/self-enhancement/authority biases; RAND found no uniformly reliable judge model, with frontier models exceeding 50% error on advanced bias tests). Tool selection is mechanically checkable; use the mechanical check.

### 3.3 The fate of `eval-layer/` — recommendation: retire after Phase 2 lands

Current facts, stated plainly: `eval-layer/` (`eval_runs.py`, `metrics.py`, `openrouter_judge.py`) is a standalone Python DeepEval prototype scoring `ToolCorrectness`/`TaskCompletion`/two `GEval` rubrics via an external OpenRouter judge (`gpt-4o`). Nothing in the main tree produces the `runs.json` it consumes; its output is gitignored; `--dry-run` writes mock scores; no CI references it. It is stranded, advisory, and externally dependent.

**Recommendation: retire (delete) it once the Phase 2 golden suite is green**, for three reasons:

1. Its one mechanically-sound function (`ToolCorrectness`) is exactly what the Rust golden suite replaces, natively, with no external judge and no Python environment.
2. Its remaining functions are LLM-judge rubrics — the technique the research flags as unreliable in isolation. An external-API judge also sits awkwardly against local-first and minimal-dependency defaults.
3. The strict-discipline standing rule says confirmed-dead legacy code gets deleted, not kept as ambiance — and "not CI-wired, no producer of its input, gitignored output" is as confirmed-dead as it gets.

One thing survives the retirement, as recorded design knowledge rather than code: `eval-layer`'s choice of an **external, different-model judge** (rather than self-grading) is exactly what the 2025-2026 judge-bias literature validates. If dowiz ever needs subjective-quality scoring (it does not today), the pattern is: a different model than the one under test, temperature 0, reference-anchored rubric — and it is rebuilt then, against a real `runs.json` producer, not preserved now as bit-rotting Python. Final call is the operator's (§9.1).

## 4. Model and quantization guidance — configuration, not infrastructure

Grounded in what is physically on the box today (`qwen2.5-coder:7b` 4.7GB, `llama3.1:8b` 4.9GB, `nomic-embed-text` 274MB/768-dim, `qwen3-embedding:0.6b` 639MB — all four route targets resident) and the research's quantization findings (GGUF standard; Q4_K_M the efficiency default at ~1-3% quality loss vs FP16 for 7B; min VRAM ≈ file size + ~1-1.5GB; **Q5_K_M flagged as the practical minimum for code/agent tasks specifically**):

- **General and Embedding task classes: keep the current Q4_K_M-class models.** They are adequate per the research and comfortably resident.
- **Code task class: adopt Q5_K_M as the recommended quantization.** dowiz's router already splits `Code` from `General`, so the research's code/agent guidance maps one-to-one onto an existing seam: pull a Q5_K_M variant of the code model and point the (newly configurable, §2.3) `Code` entry at it. This is an `ollama pull` plus one config value — **no new infrastructure, no code beyond Phase 3's config surface.**
- **Embedding default (`nomic-embed-text` vs the resident, research-noted-better `qwen3-embedding:0.6b`):** flipping the default is one config value after §2.3 — but note that embeddings from different models are not mutually comparable, so any persisted vectors/indexes built under the old default must be rebuilt on switch. Operator call at Phase 3 time; the mechanism costs nothing either way.
- Document the VRAM rule of thumb (Q4_K_M file size + ~1-1.5GB; 8B@Q4_K_M ≈ 6-7GB; CPU-only viable but slow) alongside the config so future model swaps are sized honestly.

## 5. Fine-tuning and training — ruled out now, with the trigger named

**Ruling: no fine-tuning or training work is pursued now.** The research's own honest assessment: dowiz's actual LLM use cases — tool-calling, simple chat, classification over its own order/intent vocabulary — are tasks a good off-the-shelf small model plus good prompting plus tight tool schemas already handle. Training here is premature optimization, and the 2025 consensus path (LoRA/QLoRA — 4-bit base + low-rank adapters, ~90% GPU-memory reduction, near-full-finetune quality) does not change that calculus; it only makes training *cheaper if ever justified*, not *justified*.

- **The one concrete future trigger:** a narrow, dowiz-specific **intent/order classifier** whose accuracy demonstrably plateaus below need under prompting.
- **Mandatory first step before any training investment, even then:** try a distilled small model + few-shot prompting first, measured against a golden suite of the §3.2 kind extended to classification cases. Only a measured, falsifiable failure of that cheaper path opens the training question.
- If that day comes, training is offline/batch tooling (Python or a separate `tools/` binary) with zero contact with the request path — the settled sync/async calculus is untouched by construction.

## 6. Weight integrity — a scoped reuse, not a supply-chain system

The 2025 state of the art (Sigstore model-transparency + the OpenSSF Model Signing spec, June 2025) reduces to: hash every artifact — weights, config, tokenizer — into a manifest; verify against the manifest. The research's key observation is that **dowiz already implements the underlying primitive**: `kernel/src/backup.rs` + `chunker.rs` is a real content-addressed blob store (Buzhash CDC chunking, sha3_256-keyed dedup, and `FileBlockStore::get_owned` re-hashing on-disk bytes against the filename on every read — fail-closed integrity). And `llm-adapters` is already coupled to it: its own `cache.rs` uses this exact `BlockStore` for LLM response caching.

**Scope: "verify what you have locally." Nothing more.**

- A small `tools/` binary (or `llm-adapters` helper) that (a) records a manifest — model name, weight-file paths, sha3_256 content addresses via the existing chunker — when a model is installed, and (b) on demand re-hashes and diffs, reporting any drift fail-closed. Structurally this is the OMS manifest pattern with the signing/transparency layers removed.
- **Explicitly not built:** Sigstore/OIDC keyless signing, transparency logs, any publication or attestation machinery. For a local single-hub deployment pulling weights onto its own disk, that is disproportionate — the threat this addresses is local corruption/tampering of resident weights, not registry-level supply-chain provenance. (One dormant note, not a proposal: if manifests ever need signing — e.g. weights distributed hub-to-hub over the mesh — the kernel's existing ML-DSA-65 is the obvious signer. That trigger, not this blueprint, would scope it.)

This is Phase 4 and it is optional/deferred (§9.3).

## 7. Model routing and cascading — deliberately not designed here

Cascade routing is mature externally (RouteLLM, ICLR 2025: ~85% cost savings at 95% of top-model quality, escalating only ~14% of queries), and dowiz already holds the local substrate a cost-aware router would extend: the within-adapter `TaskClass` router plus the `gov_route` EV-pricing telemetry loop (`track_record.jsonl`). But a prior research arc — **HK-05/HK-09 real-time model routing, in the separate `hermes-agent-kernel-rewrite` repo** — already owns this problem (compute ~95% built and tested there; the gap is governance wiring, and it is dev-tooling, not product). Re-deriving routing design in this blueprint would duplicate that arc.

**Ruling: routing/cascading is out of scope here, by deliberate non-duplication.** One-line pointer for the future: if cross-model cascading ever becomes a near-term dowiz product need, a dedicated synthesis should reconcile the HK-05/HK-09 work with dowiz's `TaskClass` + `gov_route` substrate — that synthesis, not this one.

## 8. Phased build order — RED→GREEN acceptance per phase

Every phase lands with a test that provably failed before the change and passes after. No phase claims done without its GREEN.

### Phase 1 — `AiMode` becomes the real composition switch (`compose.rs`)

*The core of this blueprint. Everything else is smaller.*

- **RED:** a new `compose.rs` test calling `StackBuilder::from_config(&BackendConfig { mode: Off, .. })` and asserting `Composed::Disabled` — fails today because `from_config`/`Composed` do not exist and composition unconditionally constructs `OllamaAdapter::new(&self.base)` (line 128).
- **GREEN acceptance (all falsifiable, using the existing `from_env_get` injection so no global env mutation):**
  1. `Off` (and unset `DOWIZ_AI_MODE`) composes to `Disabled`: no adapter constructed, and a consumer holding `Disabled` exposes LLM-backed features as absent — degraded closed, not erroring at call time.
  2. `LocalOffline` with a non-loopback `DOWIZ_LLM_BASE_URL` never reaches composition: `from_env_get` returns `ConfigError::NonLoopbackLocal` and a test asserts no adapter type is ever constructed on that path.
  3. `LocalOffline` with default/loopback base composes to `Ready` wrapping the Ollama constructor with the config's base URL (assert the stack's `id`/base, not live traffic).
  4. `Connected` with complete config composes to `Ready` on the `OpenAiCompatTransport` + `Quirks::managed_api` path; partial config (`MissingBaseUrl`/`MissingApiKey`) is a typed composition-time refusal.
- **Includes:** the thin managed-API constructor entailed by mode 3 (§2.2). Excludes any live remote test — wire-shape assertions only.

### Phase 2 — Rust-native golden tool-calling suite

- **RED (two-sided, proving the suite can fail):**
  1. Before the suite exists: no test anywhere exercises live-model tool selection (the confirmed hole).
  2. Self-falsification case: with a deliberately-wrong `expect` (or scrambled fixture), the suite must report failure against the live backend — a suite that cannot go red is not evidence.
- **GREEN acceptance (run via `cargo test --ignored` against live Ollama):**
  1. Every known-good fixture prompt yields selection of the expected tool with the required argument keys.
  2. Every known-ambiguous fixture prompt yields `no_tool` — the model does not silently pick one.
  3. A run appends one scorecard line (model IDs + per-case results) to the local `.jsonl`; two consecutive runs produce a diffable record.
- **Placement discipline:** `#[ignore]`d, non-CI, advisory probe — mirroring the existing live-test pattern and the repo's rule that host-noisy LLM probes are never baseline-gated.

### Phase 3 — model/quantization configuration pass

- **RED:** a test asserting `route_model` honors a builder/env override for `TaskClass::Code` — fails today because the map is hardcoded.
- **GREEN acceptance:**
  1. Unconfigured behavior is bit-identical to today's defaults (regression test on all three classes).
  2. Overrides via `model_for`/env vars pass through verbatim.
  3. Documentation lands: Q5_K_M recommendation for `Code`, VRAM rule of thumb, embedding-default caveat (rebuild persisted vectors on switch).
  4. Operationally (not a code gate): the Q5_K_M code model pulled and the `Code` entry pointed at it, verified by one Phase 2 suite run under the new model — which is precisely the regression-tracking scorecard earning its keep.
- This phase is deliberately mostly configuration + documentation, and is stated as such.

### Phase 4 — weight-integrity verification (optional / deferred, pending §9.3)

- **RED:** manifest-record then flip one byte in a recorded weight file (in a temp copy) — verification must report the mismatch; a verifier that stays green under a flipped byte fails acceptance by definition.
- **GREEN acceptance:**
  1. Record: manifest of sha3_256 content addresses for a model's files via the existing `chunker.rs` path.
  2. Verify-clean: untouched files pass.
  3. Verify-tampered: the flipped-byte copy is reported, fail-closed, naming the file.
- No new hashing code — reuse the primitive `cache.rs` already depends on.

## 9. Open decisions for the operator

1. **`eval-layer/` fate — retire / keep-advisory / absorb.** Recommendation in §3.3: **retire after Phase 2 is green**, preserving only the validated external-different-model-judge pattern as recorded knowledge. Keeping it costs a bit-rotting Python harness with an external OpenRouter dependency and no input producer; absorbing it means porting judge-rubric scoring the research itself flags as unreliable in isolation. Awaiting ruling.
2. **vLLM / managed-API backend: now or when needed.** Explicitly cheap either way — `Quirks::vllm()` and `Quirks::managed_api(api_key)` already exist, so a backend is a preset + thin constructor, never a new dependency. Note Phase 1's `Connected` mode already entails the thin managed-API constructor; the question is only whether to build/test a vLLM lane beyond that now. Recommendation: defer until an actual vLLM deployment exists to test against; the cost of deferring is near zero by construction.
3. **Phase 4 weight integrity: build now or defer.** Recommendation: **defer** — it is real and cheap (primitive exists, `llm-adapters` already couples to it) but addresses no active threat while Ollama is the sole weight manager on a single box. Same category as this session's other "nice but not core" flags (e.g. sonification in the intent-interface blueprint), with one difference worth recording: a named concrete trigger — the first time model weights are distributed between nodes/hubs rather than pulled per-box — at which point Phase 4 stops being speculative and becomes required.

## 10. Non-goals (explicit)

- No in-process inference runtime, ever, in this lane (§1 — the locked decision).
- No async runtime, no change to the 2-worker dispatch (settled this session).
- No model-registry/discovery/auto-pull system (§2.3).
- No LLM-as-judge in any deterministic gate (§3.2).
- No fine-tuning/training work now (§5).
- No routing/cascade design here (§7 — owned by the HK-05/HK-09 arc).
- No Sigstore/OIDC/transparency-log adoption (§6 — manifest verification only, if Phase 4 is approved).
- No change to `rerank`'s `Err(Unsupported)` status (owned by the retrieval arc).

---

*End of blueprint. Everything above is design; nothing is built. Phase 1 is the load-bearing item; Phases 2-3 are small and independent once Phase 1's config surface exists; Phase 4 awaits the §9.3 ruling.*
