# LLM-INFRA RESEARCH — WANDR · vLLM · llama.cpp (2026-07-16)

> Research artifact (no code changed). Grounds the operator ask — **"можливість нативно
> використовувати vLLM, llama.cpp за бажанням"** — in verified 2026 facts + this repo's own canon,
> and verifies (does not accept) the P05/P15 claim that llama.cpp/vLLM self-hosting must wait for
> GPU-unlock. Web facts gathered 2026-07-16 (two research lanes, sources inline); local facts
> verified against files/live host on the same date. Verified vs estimate is marked throughout.

---

## 1. WANDR — what it is, and what actually transfers to dowiz's eval layer

### 1.1 What it is (verified from repo + paper)

**WANDR** ("Wide ANd Deep Research", Perplexity AI, released 2026-07-14, Apache-2.0,
`github.com/perplexityai/wandr`) is a benchmark of **500 agentic data-collection tasks**. Each task
is a **qualification-key hierarchy** — e.g. `company(70) → appointee(1) → url(1)` — defining a
target of n×m×k **evidence records**. The headline **170,495** is the total *required* record count
across all tasks (median 245 records/task), not a gold answer set.

- **One evidence record** (the atomic unit of evaluation) is a JSONL line:
  `{"item": {…key fields…}, "url": "<source>", "excerpts": ["verbatim quote", …], "answer": {…freeform…}}`
  — cites a live page, carries verbatim excerpts that must make the claim evident *on their own*.
- **Reference-free grading**: there is **no gold answer key** ("expensive, quickly stale"). The
  verifier **re-fetches every cited URL** and judges the record against the live page.
- **The authoritative per-record scorer is an LLM judge** (pinned GPT-5.4, structured Pydantic
  `JudgmentResult`, seven booleans + 0–3 confidence; only confidence ≥ 2 counts). Verdict =
  product of `page_content_usable × answer_intent_clear × excerpts_faithful ×
  overall_valid × requirements_all_satisfied × requirements_all_supported`.
- **Soft vs hard F1** (exact semantics from `metrics.py`): precision = raw average over supplied
  child scores; recall = dedup-by-entity (take-**worst** on duplicates) → sort desc → truncate/pad
  to the required count (shortfall zero-fills). **Soft** = partial credit propagates up the
  hierarchy; **hard** = thresholded `not < 1.0` at every non-root level — one failed branch zeroes
  the whole member. Headline = unweighted mean over 500 tasks.
- **Top score 0.363 soft / 0.133 hard F1** (Perplexity Search-as-Code, GPT-5.5/high): full credit
  for roughly **one in seven** members. Claude Opus 4.8 Managed Agents 0.249/0.072; GPT-5.5
  Responses 0.121/0.035; Gemini Deep Research 0.055/0.009. Dominant failure mode across systems:
  **incomplete excerpt support** (57.5–86.6% of records), not page fetchability.
- **Harness shape**: file-output contract, not tool-loop — agent receives `instruction.md`, must
  produce the JSONL files named in `task.toml`. **The harness supplies no search tool**; each
  system brings its own web stack (`network_mode = "public"`, agent timeout 16h, verifier 8h).
  Authors' own caveats: one run/system (observational ranking), judge+refetch grading error
  reduced-not-eliminated, grader's fetch backend shared with the Perplexity solver.

### 1.2 Comparability check — is dowiz's harness the same problem? **No.**

dowiz has **two** eval layers, and neither is a wide-web-research evaluator:

| Layer | What it is | Scoring authority |
|---|---|---|
| `/root/dowiz/eval-layer/` (`eval_runs.py`, `metrics.py`, `openrouter_judge.py`) | DeepEval LLM-judge over **agent-run records** (TaskCompletion, Trajectory GEval, Quality GEval, ToolCorrectness), judge via OpenRouter (OpenAI-compatible, `OPENAI_BASE_URL` env) | LLM judge (advisory by canon) |
| `/root/dowiz/kernel/src/evals.rs` (E0–E3 DONE, 234 kernel tests green) | **Deterministic computed** metrics: `MetamorphicGenerator` (5 MR families, programmatic oracles), `MintLog` content-hash leakage gate, `ece`/`brier`/`aurc`, `EvalRow`→JSONL, `RegressionGate`, `SelfAdaptator` | Computed oracle — the gate |

Canon law (VERIFIABLE-COGNITION §3.5): **"LLM-as-judge is never the gate; deterministic oracles
are the authoritative pass/fail."** WANDR's authoritative scorer is precisely an LLM judge — so
WANDR-as-a-whole is philosophically incompatible as a *gate*, and WANDR-the-dataset measures a
different system class (production web-research stacks with their own search infra; running the
dowiz agent on it would score the managed API + search stack, not the kernel). **Do not claim
benchmark comparability with WANDR leaderboard numbers.**

### 1.3 What DOES transfer (concrete, piecewise — all Apache-2.0)

1. **The soft/hard hierarchical rollup — cleanest transfer.** `metrics.py` (~600 lines,
   self-contained, pure arithmetic over a scored key-tree) is deterministic: dedup-take-worst,
   pad-to-required-count recall, and the hard "all-or-nothing per member" threshold are exactly
   dowiz's fail-closed, anti-partial-credit ethos. Port it as a small no-dep Rust module in
   `kernel/src/evals.rs` and any hierarchical eval gains a principled score: retrieval evals over
   corpus→doc→chunk trees, the L0–L11 order-lifecycle gate (soft = diagnostic, hard = the gate),
   living-knowledge oracle rollups. This piece is **computed, not judged** — gate-eligible.
2. **The evidence-record shape** `{item, source, excerpts, answer}`. VERIFIABLE-COGNITION §2's
   groundedness metric ("claim↔source coverage = fraction of claims whose max cosine to a cited
   chunk > τ") currently has no standardized record format; WANDR's is a proven one. Swap `url`
   for a content-addressed `source_id` (sha3, per repo discipline) on the private corpus — the
   re-fetch step becomes a deterministic store lookup, *removing* WANDR's page-drift weakness.
3. **Anti-gaming mechanics**: take-worst on duplicate entities, confidence-below-2 = missing
   (hurts recall, never helps precision), shortfall zero-fill. Cheap, deterministic, adoptable.
4. **The judge triad as the advisory slot**: `excerpts_faithful` (verbatim-vs-page) ×
   `requirements_all_satisfied` (page ⊨ claim) × `requirements_all_supported` (excerpts alone ⊨
   claim) is a better-structured advisory judge than the current free-form GEval criteria in
   `eval-layer/metrics.py`. `judge_macro.md.jinja` is task-generic and can be driven through
   `openrouter_judge.py` unchanged (same OpenAI-compatible client) — **advisory only**, per §3.5.

### 1.4 What does NOT transfer

- The 500 tasks / 170k record counts themselves (web-research domain, no gold data to reuse).
- LLM-judge as authoritative scorer (violates ground-truth-over-proxy; advisory slot only).
- The agentic re-fetch stack (Perplexity-API fetch backend + browser fallback) — heavy external
  dependency; dowiz's private-corpus variant replaces it with store lookups.
- The fixed-dataset admission model — a static 500-task set is the staleness/contamination
  pattern §3.2's timestamp/content-hash minting exists to kill; WANDR's reference-free grading
  only partially mitigates it. `MintLog` metamorphic generation remains superior for kernel evals.

---

## 2. vLLM — current state (mid-2026) + integration surface

Verified (PyPI, docs.vllm.ai, GitHub releases; research lane 2026-07-16):

- **v0.25.1 (2026-07-14).** **PagedAttention was REMOVED in v0.25.0** — the V1 / "Model Runner
  V2" engine is now the only path (near-zero-overhead prefix caching, chunked prefill,
  prefill/decode scheduled independently). ⚠️ **Canon nit:** anywhere this repo describes vLLM as
  "the PagedAttention engine" is now historically-true-but-outdated; fix at next canon merge.
- **CPU mode exists and is officially supported, but second-class**: x86 AVX2+ (AVX-512
  recommended; pre-built wheels + Docker), ARM NEON/Graviton, FP32/16/BF16 + INT8/AWQ/GPTQ, manual
  `VLLM_CPU_KVCACHE_SPACE`/OMP-bind tuning. Docs call it "basic model inferencing and serving";
  every 2026 comparison found agrees **llama.cpp beats vLLM CPU-only on identical hardware**. No
  credible published CPU tok/s figures — treat vLLM-CPU as a compatibility fallback, never the
  reason to adopt vLLM.
- **Realistic minimum GPU** for 7–8B: hard floor **8 GB** (Q4/AWQ, short context, tight
  `gpu_memory_utilization`); comfortable **12–16 GB** (RTX 3060 12 GB ≈ 60–70 tok/s single-stream
  at Q8 — aggregator estimate; T4 16 GB in cloud). FP16 unquantized 8B ≈ 16 GB+ with KV cache.
- **Integration surface**: `vllm serve <model>` → OpenAI-compatible: `/v1/chat/completions`,
  `/v1/completions`, `/v1/embeddings`, audio transcribe/translate, `/v1/models`, `/health`,
  `/tokenize`, plus extensions `/score`, `/rerank`, `/classify`, `/pooling`. Tool calling +
  structured outputs. Docker `vllm/vllm-openai` (~7 GB-class image, needs `--ipc=host`). Offline
  batch: `vllm run-batch` over OpenAI Batch-format JSONL.
- **Where it fits here**: vLLM is the **GPU-throughput tier** — many concurrent requests, larger
  models, OpenAI drop-in at scale. Natural pairing with the already-canonical **Modal H100 burst**
  ($0.001097/s scale-to-zero, E22/F34): vLLM inside a scale-to-zero container behind the same
  port. Its gating on GPU-unlock (O18) **remains correct** — see §4.

## 3. llama.cpp — current state + integration surface

### 3.1 CPU-viability verdict — FRONT AND CENTER

**Verdict: CPU-only llama.cpp inference is viable TODAY, on THIS host class, for small quantized
models.** This is llama.cpp's core value proposition, and 2026 numbers confirm it:

| Hardware | Model / quant | Throughput | Source class |
|---|---|---|---|
| 2× Xeon Gold 5317 (AVX-512) | 8B Q4_K_M | **~22.4 tok/s** (tg128) | verified llama-bench |
| AWS Graviton c8g.2xlarge (8 vCPU, 16 GiB) | Llama 3 8B Q4 | **~12.4 tok/s** | verified (ClearML) |
| Ryzen 9 9950X / i9-14900K | Llama 3.1 8B Q4_K_M | ~11–12 tok/s | benchmark site |
| mid-range consumer CPUs | 3–4B Q4 (Phi-4-mini, Llama 3.2 3B) | **~9–15 tok/s** (est. 15–40 on server DDR5) | aggregator estimates |

RAM at Q4_K_M: 3–4B ≈ **2–4 GB working**; 7–8B ≈ **5–6 GB + KV**. Generation is
memory-bandwidth-bound, not core-bound.

**This host (verified live 2026-07-16): 8 vCPU AMD EPYC Milan, 32 GB RAM (29 GB available),
15 GB disk free** — comfortably runs any 1–8B Q4 model; can hold a 4B chat model AND an embedding
model resident simultaneously. Realistic expectation here: **~10–20 tok/s on 8B Q4, ~15–30 tok/s
on 3–4B Q4** — genuinely usable for single-agent workloads (classification, advisory judging,
embeddings, rerank, structured extraction), **not** for high-concurrency serving. Egress verified
live: `github.com → 200`, `huggingface.co → 200` — binary and GGUF weights are fetchable now.

### 3.2 Current state (verified from repo)

- Rolling releases (b-series, 5,000+), MIT, **single portable static binary** — zero external
  deps; Docker at `ghcr.io/ggml-org/llama.cpp` (`:server`, `:light`, `:full` + GPU variants).
  Backends: AVX/AVX2/AVX-512/AMX, NEON, CUDA/HIP/Metal/Vulkan/SYCL/WebGPU.
- Quantization 1.5–8 bit (Q4_K_M standard) + MXFP4. Speculative decoding, multimodal, GBNF
  grammar / **JSON-schema constrained output**, Jinja2 chat templates, tool calling,
  `reasoning_content` parsing, continuous batching with parallel slots.
- **llama-server endpoints** (one binary): OpenAI-compatible `/v1/chat/completions`,
  `/v1/completions`, `/v1/embeddings`, `/v1/models`, `/v1/responses`; **Anthropic-compatible
  `/v1/messages`**; native `/reranking`, `/tokenize`, `/detokenize`, `/slots`, `/health`,
  Prometheus `/metrics`; API-key auth + optional TLS.
- Small-model landscape mid-2026: **Qwen3.5 small series (0.8/2/4/9B**, 2026-03, constant-memory
  hybrid attention — CPU-KV-friendly), Gemma 3 4B, Phi-4-mini 3.8B (best sub-4B reasoning),
  SmolLM3-3B (128K ctx), Llama 3.2 3B. Canon's F35 ("hub runs tiny SmolLM on edge — possible,
  LOCK") is corroborated with headroom to spare.

### 3.3 Fit with this repo's architecture

- **S1 (zero-OCI native static binaries + systemd)**: llama-server is *exactly* that shape — one
  static binary, one systemd unit, one EnvFile. No OCI, no Python, no CUDA runtime.
- **M8 (local-only telemetry)**: `/metrics` is a local Prometheus endpoint; scrape locally only.
- **F3/F27 (sha3-verify-or-deny model ingestion)**: GGUF blobs flow through P15 §9's manifest
  `{url, sha3}` path unchanged.
- **Immediate concrete unlock**: VERIFIABLE-COGNITION §3.3's **semantic leakage gate is DEFERRED**
  solely because "rejecting an item whose embedding cosine > 0.9 requires an embedding bridge."
  `llama-server /v1/embeddings` on CPU **is that bridge** — the deferred gate becomes buildable
  with zero GPU and zero per-call API cost. Same for §7's embedding/NLI adapter seam
  (`SubprocessLivingKnowledge` / `LK_BRIDGE_CMD`) and for pointing `eval-layer/openrouter_judge.py`
  (`OPENAI_BASE_URL`) at a sovereign local judge.

---

## 4. The GPU-unlock gating verdict — P05/P15 checked, not accepted

### 4.1 What the blueprints actually say (read, verbatim anchors)

- **P05 §8** (`BLUEPRINT-P05-routing-organism-wiring.md:253-258`): "Self-hosting llama.cpp …/
  vLLM … as the *execution* tier is a separate blueprint that ships in Phase 15, **gated on
  GPU-unlock** — an external trigger (operator / network `cargo add wgpu`, ARCHITECTURE §8)."
- **P15 §9** (`BLUEPRINT-P15-living-organism-unbounded.md:311-336`): "E13's *execution* is gated
  on the external GPU-unlock trigger (O18: network `cargo add wgpu`, operator/environment)…
  Until GPU-unlock, managed-advisory remains the reality." Acceptance item 10: "execution remains
  gated on O18 / Phase 17."
- **O18's precise definition** (P17 §3): "network access that allows `cargo add wgpu` to
  succeed, **plus operator go**" — i.e. O18 is a *network+operator* trigger wearing a GPU name.

### 4.2 Verdict: **the deferral was a real mistake — a category error, definitively.**

Not "it depends." The gate conflates **three independent preconditions** and binds a
CPU-capable backend to the one it doesn't have:

| Precondition | llama.cpp CPU tier | vLLM GPU tier | wgpu video (P17) |
|---|---|---|---|
| GPU hardware | **NOT needed** (whole value prop is CPU-first) | needed (primary path) | needed |
| Network fetch (new artifact) | needed (binary + GGUF) — **satisfied: github.com/huggingface.co → 200, live 2026-07-16** | needed | needed (`cargo add wgpu`) |
| Operator go + DECART (new dep) | needed — **still open, correctly** | needed | needed |
| Host resources | **satisfied** (8 vCPU / 32 GB / 15 GB disk vs ~4–6 GB needed) | not satisfied (no GPU) | n/a |

llama.cpp involves **no cargo, no wgpu, no GPU** — every technical element of O18 is irrelevant
to it. A native, CPU-only llama.cpp backend **could be integrated now**; GPU matters only for
throughput/concurrency and larger models later. The repo's own canon already knew the ingredients
(F35 small-model-on-edge = possible; R1-B §E13 verified Hermes' `custom` provider profile makes an
OpenAI-compatible local endpoint "a config-not-code change" — re-verified this session:
`/root/hermes-agent-kernel-rewrite/plugins/model-providers/custom/__init__.py` names vLLM and
llama.cpp explicitly); the blueprints simply inherited R2:81's single "E13 ⟵ GPU-unlock" edge
without re-deriving it per-backend.

**Honest mitigations (why it happened, not why it stands):** (a) at the time W21 was recorded the
environment was treated as offline (cargo-network down), so "everything new waits for the network
unlock" was a defensible simplification — but O18 is *named and scoped* as GPU-unlock, and P05/P15
copied the mis-scoped name into law; (b) P15 §9's port design itself (Trait-as-Port, sha3
weights, TokenBucket ceiling, honest `Err` offline) is **correct and survives unchanged** — only
the activation trigger is wrong; (c) the *operator-go* half of the gate is right and stays: a new
binary + model weights is a new dependency at a trust-relevant surface → DECART report + operator
decision, per the rust-native rule. The corrected claim is **not** "self-start now"; it is "the
CPU tier's gate is operator-go + DECART only — the GPU condition must be struck."

### 4.3 What should change (concrete amendments, for the operator's canon merge)

1. **Split E13 into two tiers with separate gates:**
   - **E13-cpu** — llama.cpp CPU tier (small quantized models, single-agent throughput).
     Gate: network egress (**verified satisfied 2026-07-16**) + DECART report + operator go.
     *No GPU condition.*
   - **E13-gpu** — vLLM GPU tier + Modal H100 burst (throughput/scale, larger models).
     Gate: O18 GPU-unlock, unchanged.
2. **Amend P05 §8 + P15 §9/§10.10 + R2:81/105** accordingly: `P15 ← … [+GPU-unlock for
   E13-gpu ONLY; E13-cpu ← egress(SATISFIED) + operator-DECART]`.
3. **Amend ARCHITECTURE.md:34** "managed-advisory until GPU-unlock" → "managed-advisory default;
   E13-cpu unlockable by operator decision now; E13-gpu until GPU-unlock". Same merge: retire the
   "PagedAttention" descriptor for vLLM (removed upstream v0.25.0).
4. **Rename or annotate O18** where it gates non-GPU things: its fire-condition is
   *network+operator*; the GPU name misleads exactly the way it misled here.

---

## 5. Design sketch — pluggable LLM-backend port (M5: hub chooses, at will)

The operator's ask — *natively use vLLM/llama.cpp at will* — **is M5 verbatim** ("every hub may
use any models/API/MCP/agents at its discretion") applied to the LLM backend. So the design is not
"add llama.cpp support"; it is **make the LLM backend a hub-chosen port** with managed-API as the
safe canonical default. Per the SCOPE RULE, everything below is canonical-build recommendation —
a runtime hub may point the port anywhere it likes.

### 5.1 The load-bearing simplification

Managed APIs (OpenRouter/headroom proxy), **llama-server**, and **vLLM serve** all speak
OpenAI-compatible HTTP. So the port needs **one transport adapter with per-backend quirk
profiles**, not three transports — the exact pattern already proven in Hermes
(`plugins/model-providers/custom/`: one profile covering "Ollama, vLLM, llama.cpp, GLM/ARK" with
per-backend quirks like `think=false` / `reasoning_effort` / `num_ctx`).

### 5.2 Port (kernel side — trait only, zero vendor knowledge)

```rust
// ports/llm.rs — the PORT. Kernel/harness code sees only this. (Trait-as-Port, ARCHITECTURE §1/§3.)
pub trait LlmBackend {
    fn id(&self) -> BackendId;                    // "managed" | "llamacpp" | "vllm" | Custom(url-id)
    fn caps(&self) -> Caps;                       // chat, embed, rerank, json_schema, tools — fail-closed feature discovery
    fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LlmError>;
    fn embed(&self, req: EmbedRequest) -> Result<Vec<Embedding>, LlmError>;
    fn rerank(&self, req: RerankRequest) -> Result<Vec<Ranked>, LlmError>; // Err(Unsupported) where absent
    fn health(&self) -> Result<Health, LlmError>; // honest Err when backend absent — E21 boundary pattern
}
```

Adapters live in a **separate crate** the kernel never imports (compile-firewall, VC §7); the
kernel receives `&dyn LlmBackend` (no `Box` needed on the hot path — `&dyn` pulls no alloc).

### 5.3 Adapters

| Adapter | base_url | Tier / gate | Notes |
|---|---|---|---|
| `ManagedApiAdapter` | headroom proxy → OpenRouter | **Tier 0 — DEFAULT, live now** | key from EnvFile (S3); current reality formalized by P05 |
| `LlamaCppAdapter` | `127.0.0.1:8080` (llama-server, systemd unit) | **Tier 1 — operator-unlockable NOW** (E13-cpu gate, §4.3) | single static binary (S1-native); GGUF via F3/F27 `{url, sha3}` verify-or-deny; JSON-schema constrained output; `/v1/embeddings` + `/reranking` feed VC §3.3/§7 |
| `VllmAdapter` | `127.0.0.1:8000` or Modal URL | **Tier 2 — O18 GPU-unlock** | throughput/concurrency tier; Modal H100 burst behind the same ceiling |

All three are thin `Quirks` structs over one shared `OpenAiCompatTransport` (Hermes-proven).
A hub selects via one config field (`HubPolicy.llm_backend` / EnvFile `LLM_BACKEND=` +
`LLM_BASE_URL=`) — **swap is config, never a kernel change**. No dev-time gate may block a hub
from switching (M5/SCOPE RULE); the canonical build simply defaults to `managed`.

### 5.4 Guards that bind regardless of backend (the LOCKs)

- **F3/F27**: weights ingested only through the sha3 manifest — wrong hash refused at load.
- **F6/E19**: `TokenBucket`/`Budget` ceiling on every call — $-spend for managed, token/CPU-time
  budget for local; over-budget = typed refusal, never silent downgrade (P05 acceptance #3).
- **M8**: all inference telemetry (incl. llama-server `/metrics`) local-only.
- **M12**: red-line scopes (money/auth/secrets/migrations) denied at the capability layer — the
  backend choice never widens scope.
- **E21**: absent backend = honest `Err`, not a mock.
- **DECART**: each new binary/model = a dated DECART report before adoption (rust-native rule).

### 5.5 First consumers of Tier 1 (why unlock it — concrete value, not capability theater)

1. **VC §3.3 semantic leakage gate** — deferred *only* for lack of an embedding bridge; local
   `/v1/embeddings` un-defers it (CPU, $0/call).
2. **Sovereign advisory judge** — `openrouter_judge.py` already honors `OPENAI_BASE_URL`; point
   it at llama-server and the eval-layer's advisory judging goes local (WANDR's judge-triad
   prompts reusable here, advisory-only per VC §3.5).
3. **Hub-local classification/routing NL tasks** (P05's future hook) at zero marginal cost.
4. **M5 made real**: the first demonstration that a hub can actually choose its own model backend
   — currently a law with no exercised instance.

---

*Research artifact for the sovereign-roadmap 2026-07-16 set. Companion to BLUEPRINT-P05 §8,
BLUEPRINT-P15 §9, BLUEPRINT-P17 §3, R2 §4 O18, ARCHITECTURE.md §1 (E13/E14) + §0 M5,
VERIFIABLE-COGNITION-BLUEPRINT §2/§3.3/§3.5/§7, eval-layer/*, kernel/src/evals.rs. Amendments in
§4.3 are staged for the operator's canon merge — this document edits no canon itself.*
