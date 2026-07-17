# HARNESS — LlmBackend Port (Ollama + vLLM), Caching, Parallelism — ONE consolidated plan

> **Status: planning complete, execution-ready, no code written.** Merges what were three separate
> documents (`HARNESS-LLM-BACKEND-PLAN.md`, `DECART-llm-backend-integration.md`,
> `BLUEPRINT-LLM-BACKEND-PORT.md`) into one navigable reference, per operator directive 2026-07-16 —
> those three are deleted from disk; this file is now the single source of truth for this arc.
> Branch: `feat/harness-llm-backend` (branched from `feat/kernel-fsm-graph-analysis`). Extends
> `docs/design/sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md` §2 H3 and
> `BLUEPRINT-P15-living-organism-unbounded.md` §9 against the **actual, live-verified state of this
> host** — the single biggest change since that earlier work: **Ollama is already running.**

---

## 0. Executive summary

dowiz needs a pluggable local/managed LLM backend (M5 hub-autonomy: "a hub may use any models/API at
its discretion"). Ollama has been running on this host since 2026-07-13 with four models pulled
(`llama3.1:8b`, `qwen2.5-coder:7b`, `nomic-embed-text`, `qwen3-embedding:0.6b`) and a working
OpenAI-compatible API — so Tier-1 needs **zero install**, only wiring. This document designs one
`LlmBackend` trait with three adapters (managed-API default, Ollama Tier-1 live-now, vLLM Tier-2
GPU-gated), a two-layer response cache (exact-match, safe everywhere; semantic near-duplicate,
advisory-only, type-enforced), and a `TokenBucket`-bounded dispatcher built on the repo's own
already-vetted `ureq` HTTP-client convention (not tokio — a real correction made during this
consolidation, see §4). Two integrations are DECART-cleared (§5). The true build-dependency graph is
**one foundational step, then three genuinely parallel ones** — corrected from the original plan's
overly-conservative "strictly ordered" framing (§6).

---

## 1. Ground truth (live-verified 2026-07-16, re-checked at consolidation time)

### 1.1 No GPU on this host
`nvidia-smi` → command not found; `ls /dev/nvidia*` → no such file. 8-vCPU AMD EPYC Milan / 32GB RAM.
The vLLM Tier-2 gate (O18 GPU-unlock, per the living-interface roadmap's split) stays correctly closed
— nothing here loosens it.

### 1.2 Ollama is the local Tier-1 inference server — already running, zero install
- `systemctl is-active ollama.service` → **active** since 2026-07-13 (`/usr/local/bin/ollama serve`),
  `http://127.0.0.1:11434`, version `0.30.9`.
- `ollama list` → **four models**: `qwen2.5-coder:7b` (4.7GB, code-shaped tasks), `llama3.1:8b`
  (4.9GB, general), `nomic-embed-text:latest` (274MB, embeddings), `qwen3-embedding:0.6b` (639MB,
  embeddings).
- `ollama ps` (live) → **three models resident simultaneously** on this 32GB host at once
  (`llama3.1:8b` + both embedding models), confirming `OLLAMA_KEEP_ALIVE` residency caching is active.
- **Both API surfaces probed and confirmed:**

| Endpoint | Result |
|---|---|
| `GET /v1/models` | 200, OpenAI-compat model list |
| `POST /v1/chat/completions` | 200, full OpenAI schema incl. `usage.{prompt,completion,total}_tokens`, `system_fingerprint:"fp_ollama"` |
| `POST /v1/embeddings` | 200 (nomic) |
| `POST /api/embed` (native) | 200, different nesting: `{"embeddings":[[…]]}` vs. `/v1`'s `{"data":[{"embedding":[…]}]}` |

- **Parallelism config at defaults** — `ollama serve --help` exposes `OLLAMA_NUM_PARALLEL` (default
  1, auto-selects up to 4 by available memory), `OLLAMA_MAX_QUEUE` (default 512, then HTTP 503),
  `OLLAMA_MAX_LOADED_MODELS`, `OLLAMA_KEEP_ALIVE`. None are set in the service env — Ollama's own
  request-level parallelism is real and free; the harness must reuse it, not fight it (§4).
- **Net delta vs. the earlier design:** "install `llama-server` as a new systemd unit" is now
  obsolete — replaced by `OllamaAdapter` pointing at the already-running daemon. The GGUF `{url, sha3}`
  verify-or-deny path (F3/F27) still applies to *future* model pulls, but is not on the critical path
  to turn Tier-1 on today.

### 1.3 No Cargo workspace exists — verified, this shapes every file target below
`find /root/dowiz -maxdepth 2 -name Cargo.toml` → four **independent** crates (`kernel/`, `engine/`,
`wasm/`, `agent-governance-wasm/`), each with its own `Cargo.lock`, no root `[workspace]`. The new
`llm-adapters/` crate is therefore **another standalone crate at repo root**, path-depending on
`kernel/` directly — built with `cargo build -p llm-adapters` from its own directory, matching how
`engine/` already builds independently.

### 1.4 Existing patterns this design reuses verbatim (cited precisely)
- **Feature-gating discipline** (`kernel/Cargo.toml:1-40`): `default = ["std"]`, zero network/HTTP in
  the default build; `wasm` gates serde; `pgrust` gates `sqlx`/`tokio` — the **only** place `tokio`
  appears in this crate today, opt-in only. The new `ports::llm` module adds **zero** new deps to
  `kernel/Cargo.toml`.
- **Submodule convention**: `kernel/src/retrieval/mod.rs` / `kernel/src/isolation/mod.rs` — a `mod.rs`
  with one `pub mod x;` line + doc comment per submodule. `kernel/src/ports/mod.rs` follows this shape.
- **Content-addressing to reuse verbatim**: `kernel/src/backup.rs:29` `pub trait BlockStore { put, get,
  len, is_empty }`; `:44` `pub struct MemStore` (put is idempotent — "storing a block whose id already
  exists is a no-op"); `:24` `pub type Hash = [u8; 32]`. `kernel/src/event_log.rs:30` `pub fn
  sha3_256(input: &[u8]) -> [u8; 32]`, already public, zero-dep.
- **Structural leakage-gate pattern to extend, not rewrite**: `kernel/src/evals.rs:74` `pub struct
  MintLog` over two decorrelated FNV-1a streams, `:87` `pub fn mint(&mut self, kind: &str, payload:
  &[u8]) -> Option<u64>`. File header (`:6-7`) and `:60` state verbatim: *"the cosine-0.9 semantic gate
  is the embedding-bridge analogue (deferred §7)."* `MintLog` stays the exact-duplicate half; the new
  semantic gate is a sibling, composed alongside it (exact call site TBD at implementation time —
  flagged honestly in §6 Step 1b, not guessed).
- **HTTP-client precedent** (the load-bearing convention this whole design pivots on, §5 Decision 2):
  `tools/telemetry/rust-spool/Cargo.toml` and `tools/async-spool/Cargo.toml` both depend on
  `ureq = { version = "2", default-features = false, features = ["tls", "json"] }` under the
  **2026-07-15 operator mandate**: "rustls with ring everywhere possible... NO tokio." `llm-adapters`
  uses the identical line.
- **`BLUEPRINT-P11-compute-budget-cache.md` §4** (sovereign-roadmap arc) is the unchanged F33
  `TokenBucket` spec: capacity, refill_rate, monotonic-clock last-refill, atomic `try_acquire(n) ->
  bool`, degrade-closed, with a stated concurrent-property-test falsifier.

---

## 2. The `LlmBackend` port design

### 2.1 Kernel side — trait only, zero vendor knowledge

`kernel/src/ports/mod.rs` + `kernel/src/ports/llm.rs` (new files; `pub mod ports;` added to `lib.rs`
in its alphabetical slot between `order_machine` and `retrieval`):

```rust
pub trait LlmBackend {
    fn id(&self) -> &str;                                   // "ollama:llama3.1:8b" etc.
    fn caps(&self) -> Caps;                                  // fail-closed feature discovery
    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError>;
    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError>;
    fn rerank(&self, req: &RerankRequest) -> Result<RerankResponse, LlmError>; // Err(Unsupported) ok
    fn health(&self) -> Result<(), LlmError>;                // typed Err when absent — never a mock
}

pub struct Caps { pub chat: bool, pub embed: bool, pub rerank: bool, pub tool_calling: bool }
pub struct ChatRequest  { pub model_id: String, pub messages: Vec<Message>, pub temperature: f32,
                           pub top_p: f32, pub max_tokens: u32, pub seed: Option<u64>, pub task_class: TaskClass }
pub struct ChatResponse { pub content: String, pub usage: Usage }
pub struct Usage { pub prompt_tokens: u32, pub completion_tokens: u32, pub total_tokens: u32 }
pub struct EmbedRequest  { pub model_id: String, pub input: String }
pub struct EmbedResponse { pub embedding: Vec<f32> }
pub enum TaskClass { Code, General, Embedding }             // drives §2.2's model routing
pub enum LlmError  { Unavailable, Unsupported, BadRequest(String), Timeout }
```

No `serde` derives — plain structs. The kernel never talks HTTP or JSON; the adapter crate converts.
This is the compile firewall: `cargo tree -p dowiz-kernel` must show no HTTP client and no
`llm-adapters` after implementation (§6's done-check).

### 2.2 One transport, three adapters — with a verified Quirks split

**The load-bearing claim, checked, not assumed:** do Ollama's `/v1` and vLLM's OpenAI-compat server
share one wire shape closely enough for one transport? **Yes at the schema level, no at the details —
so the Hermes-proven `Quirks`-struct pattern this repo already cites is required.** Live probes found
concrete Ollama quirks a naive OpenAI client would trip on:

1. **`:tag` model ids** (`llama3.1:8b`) — pass through verbatim, never strip/normalize after `:`.
2. **`system_fingerprint:"fp_ollama"`** is a constant sentinel — never key caching/dedup on it.
3. **Embeddings nesting differs**: `/v1/embeddings` → `{"input":…}` → `{"data":[{"embedding":[…]}]}`;
   native `/api/embed` → `{"input":…}` → `{"embeddings":[[…]]}`. Adapter picks `/v1/embeddings` for
   OpenAI-parity, `/api/embed` as native fallback.
4. **`keep_alive`, `num_ctx`, `think`** are Ollama-only knobs surfaced through `options` — modeled
   exactly like Hermes' `custom` provider (`plugins/model-providers/custom/`) already does.
5. **Tool-calling/structured-output support differs per backend/model** — a `Caps` probe, not assumed.

Therefore: **one `OpenAiCompatTransport` + a per-adapter `Quirks` struct.**

| Adapter | base_url | Tier / gate | Quirks |
|---|---|---|---|
| `ManagedApiAdapter` | headroom proxy → OpenRouter | **Tier 0 — DEFAULT, live now** | key from EnvFile (S3); standard OpenAI envelope |
| `OllamaAdapter` | `127.0.0.1:11434` (already running) | **Tier 1 — live NOW, operator-DECART to wire (§5)** | `:tag` ids; `fp_ollama` sentinel; `/v1/embeddings` primary; `keep_alive`/`num_ctx`; model routing (below) |
| `VllmAdapter` | `127.0.0.1:8000` or Modal URL | **Tier 2 — stays O18 GPU-gated** | native OpenAI-compat; `/score`/`/rerank`; Modal H100 burst behind the same ceiling |

**Model routing inside `OllamaAdapter`** (within-adapter, distinct from Phase-5's `gov_route`
dev-tooling router, F35): `TaskClass::Code → "qwen2.5-coder:7b"`; `General → "llama3.1:8b"`;
`Embedding → "nomic-embed-text"` default, `"qwen3-embedding:0.6b"` as the higher-quality option.

**Hub choice = config, guards bind regardless**: `HubPolicy.llm_backend` / EnvFile `LLM_BACKEND=` +
`LLM_BASE_URL=` — swap is config, never a kernel recompile (M5 made real; SCOPE RULE: no dev-time gate
blocks a runtime hub switching). Backend-independent locks: F3/F27 (sha3 verify-or-deny on *future*
weight pulls), F6/E19 (`TokenBucket`/Budget, §4), M8 (local-only telemetry), M12 (red-line scopes),
E21 (honest `Err` when absent).

### 2.3 Adapter crate layout (standalone, per §1.3)

```
llm-adapters/                    (repo root, sibling to kernel/ — NOT a workspace member)
├── Cargo.toml                   dowiz-kernel = { path = "../kernel" }, ureq, serde, serde_json
├── src/
│   ├── lib.rs
│   ├── transport.rs             OpenAiCompatTransport (ureq-based HTTP + JSON envelope)
│   ├── quirks.rs                 Quirks struct (per-adapter behavior deltas)
│   ├── ollama.rs                 OllamaAdapter: impl dowiz_kernel::ports::llm::LlmBackend
│   ├── cache.rs                  CachingBackend<B, S> (§3)
│   └── dispatch.rs               Dispatcher (§4)
└── tests/
    └── ollama_roundtrip.rs
```

---

## 3. Caching design

### 3.1 What's already free (know it, don't rebuild it)
- **vLLM (Tier-2, later)**: automatic prefix caching — KV-cache blocks reused across requests sharing
  a prompt prefix, via paged non-contiguous KV blocks + iteration-level scheduling ([runpod — vLLM
  PagedAttention & continuous batching](https://www.runpod.io/articles/guides/vllm-pagedattention-continuous-batching)).
  (Naming note: "PagedAttention" as a *descriptor* retired upstream in vLLM v0.25.0; the paged-KV
  *mechanism* persists as the V1/Model-Runner-V2 engine.)
- **Ollama (Tier-1, now)**: keeps the model **resident** (`OLLAMA_KEEP_ALIVE`, observed live in `ollama
  ps`) and reuses KV context for a shared prefix within a loaded model.

Both still run a forward pass. **The harness cache below is additive** — it returns a stored answer
with **zero forward pass**, a hash + a lookup.

### 3.2 Layer A — exact-match response cache (reuses `backup.rs` content-addressing verbatim)

Key `= sha3_256(model_id ⊕ canonical_request)` where `canonical_request` is a `BTreeMap`-built (not
`HashMap` — deterministic key order) JSON of `{model_id, messages, temperature, top_p, max_tokens,
seed, tools}`. Value = the stored `ChatResponse`. **Reuses `dowiz_kernel::backup::{BlockStore,
MemStore}` and `dowiz_kernel::event_log::sha3_256` directly — no new hashing primitive, no new store
abstraction.** `MemStore::put`'s existing idempotence ("storing a block whose id already exists is a
no-op") **is** the cache-hit semantics, for free.

- **Correctness = exactness**: the key includes `model_id` and every sampling param, so a hit is
  *provably identical*. Safe for **all task classes, including gate-critical ones** — never returns a
  different answer than the model would have. (Caveat: `temperature > 0` means even identical requests
  sample differently at the model; for `temperature=0`/seed-pinned calls the cache is exact, for
  sampled calls a hit returns *one valid* prior sample — acceptable for advisory calls, must be
  disabled via a per-request `no_cache` flag for any call whose contract requires a fresh sample.)
- **Eviction**: LRU + byte ceiling + digest-invalidation (Ollama tags are mutable — a re-pulled
  `llama3.1:8b` is different weights behind the same tag; invalidate on the model's digest changing,
  read from `/v1/models`, not just on tag).

### 3.3 Layer B — near-duplicate / semantic cache (embedding-gated, advisory-only)

Embed the incoming prompt via `OllamaAdapter.embed()` (`nomic-embed-text`), search a small in-memory
index of recent cached prompt-embeddings, serve the cached response if `cosine ≥ τ` **only for task
classes that tolerate a near-answer**. Borrows the GPTCache architecture (embed → NN → threshold →
TTL-evict) — [zilliztech/GPTCache](https://github.com/zilliztech/gptcache);
[GPTCache paper, NLPOSS 2023](https://aclanthology.org/2023.nlposs-1.24.pdf). Threshold grounding:
**τ ≈ 0.8** per [arXiv 2411.05276](https://arxiv.org/pdf/2411.05276) (68.8% hit rate, >97% positive-hit
accuracy) — a defensible starting point, tune against dowiz's own harvested data, not gospel.

**The hard boundary, non-negotiable:**

> **Layer B is for advisory/exploratory tasks ONLY. NEVER consulted for any gate-critical call** —
> deterministic-oracle evals, the leakage gate itself, money/auth/RLS/migration-adjacent reasoning, any
> `deliberate()` path a deterministic gate consumes. A near-duplicate prompt is not an identical
> prompt; a stale answer to a subtly different question is exactly the proxy-over-ground-truth failure
> canon forbids.

Enforcement is a **type**, not a convention: `CachePolicy` enum on `ChatRequest` — `Exact` (default),
`SemanticOk{τ}` (Layers A then B), `NoCache`. Only call sites explicitly marked advisory may pass
`SemanticOk` — a gate-critical caller cannot accidentally opt in.

---

## 4. Parallel-processing design (⚠ corrected during consolidation — no tokio)

> The original plan sketched a `tokio`-based dispatcher (`tokio::spawn`, `tokio::sync::Semaphore`,
> `tokio::mpsc`) *before* Decision 2 (§5) picked `ureq` (synchronous, no tokio) for the adapter crate's
> HTTP client. That was a real inconsistency between two of the source documents this file merges —
> caught and fixed here, not left standing.

### 4.1 Reuse Ollama's own request-level parallelism — don't reinvent batching

vLLM's core value-add is **continuous (iteration-level) batching**: the scheduler re-evaluates its
queues after every forward pass, no head-of-line blocking, GPU saturated every step ([runpod,
ibid.](https://www.runpod.io/articles/guides/vllm-pagedattention-continuous-batching)) — a Tier-2 (GPU)
capability, stays gated. **Ollama has a real CPU analogue**: `OLLAMA_NUM_PARALLEL` lets one loaded
model serve N concurrent requests, `OLLAMA_MAX_QUEUE` (default 512) bounds the wait before HTTP 503
([glukhov.org](https://www.glukhov.org/llm-performance/ollama/how-ollama-handles-parallel-requests/)).
On this CPU-only host this is **memory-bound** (every parallel slot needs its own KV cache) — raising
`NUM_PARALLEL` past ~2-4 will thrash. **Harness policy: set `OLLAMA_NUM_PARALLEL` explicitly (start at
2) in the service EnvFile; Ollama owns intra-model batching, the harness does not re-implement it.**

### 4.2 Harness-level concurrency control — `TokenBucket`(F33)-bounded, `std::thread`-dispatched

`kernel/src/token_bucket.rs` (sibling to `spool.rs`) builds the F33 spec as-is — zero-dep,
monotonic-clock, atomic `try_acquire(n) -> bool`, degrade-closed, with the stated falsifier ("total
granted across any window never exceeds `capacity + refill_rate·elapsed`"). **Unchanged by this
correction.**

What changes: `llm-adapters/src/dispatch.rs`'s `Dispatcher` bounds concurrency with `std::sync::mpsc` +
a fixed-size worker-thread pool (`std::thread::spawn`, N workers ≤ the backend's own parallelism cap —
e.g. 2 for Ollama, matching §4.1, so the harness's own queue is where back-pressure becomes visible,
not Ollama's `MAX_QUEUE` 503). Each worker: `bucket.try_acquire(cost)` → `false` ⇒ immediate
`Err(BudgetExceeded)` (degrade-closed, never silently queued-then-downgraded); `true` ⇒ call the
(blocking) adapter, return the result over the caller's `mpsc::Sender`. Achieves the same goal (bounded
concurrency, typed refusal, visible back-pressure) with **zero tokio** in `llm-adapters`, consistent
with §5 Decision 2. (A genuinely I/O-bound future workload needing async concurrency would be a *new*
DECART decision at that time, not a default reversion to tokio.)

Every dispatched call emits an H1 harvest row (`track_record.jsonl`), pricing local calls by Ollama's
returned `usage.total_tokens` (verified §1.2) plus a CPU-time proxy — closing the EV loop so
`gov_route` can price local-vs-managed on measured data, not vibes.

---

## 5. DECART decisions (filed before any implementation, per the Integration Decart Rule)

### Decision 1 — Ollama as the Tier-1 `LlmBackend`

| Criterion | Ollama (chosen) | Fresh `llama-server` unit | Managed-API-only |
|---|---|---|---|
| Fit to sovereign core | External Go daemon, consumed only through `&dyn LlmBackend` (compile firewall) | Same external-process fit, no material difference | Best fit but forfeits M5 hub-autonomy's whole point |
| Correctness — falsifiable | Live-probed: full OpenAI schema + `usage`; `health()` fails closed when down | Same shape achievable but unbuilt, untested | N/A |
| Performance — measured | 3 models resident at once on this 32GB host, confirmed live | Unmeasured, would need building first | N/A |
| Supply-chain | Already running, MIT-licensed, zero NEW surface | New binary + unit = new surface to vet | N/A |
| Maintainability | One known model-management daemon vs. hand-rolling a unit + OpenAI shim | More owned moving parts | Simplest, but a non-choice given M5 |
| Reversibility | Exactly a port — one of three swappable `LlmBackend` impls behind config | Same if built | Already the fallback (Tier 0 default) |

**DECISION: Ollama, wired as `OllamaAdapter`.** It is already running, already holds the needed model
classes (chat/code/embeddings), already speaks the wire protocol — a parallel `llama-server` unit would
duplicate a working, vetted daemon for no measurable gain.

**Probe (honest case against):** Ollama's model store sits outside the kernel's sha3 manifest
discipline (F3/F27) — a real gap, not dismissed. `OllamaAdapter` treats the backend as
untrusted-but-available (fail-closed on absence, never silently trusted on content); F3/F27 remains a
named follow-up for *future* pulls, not solved here.

### Decision 2 — HTTP client crate: `ureq`, not `reqwest`

| Criterion | `ureq` (chosen) | `reqwest` |
|---|---|---|
| Already used in this project | **Yes, twice** — `tools/telemetry/rust-spool`, `tools/async-spool`, exact same version/feature spec, under the 2026-07-15 operator mandate ("rustls with ring everywhere possible") | Not used anywhere in this repo today |
| Fit | Synchronous, no runtime required — matches this workload (one blocking call per dispatch, concurrency bounded server-side by `OLLAMA_NUM_PARALLEL`, not client-side socket multiplexing) | Requires `tokio`; heavier graph for a use case that doesn't need async I/O |
| Correctness/security | Same rustls+ring provider this repo's own DECART worked example already chose | Also supports rustls, but pulls tokio's runtime surface in as a correctness-relevant dependency |
| Reversibility | `OpenAiCompatTransport` is the seam; swapping the client later is internal, never a kernel change | — |

**DECISION: `ureq = { version = "2", default-features = false, features = ["tls", "json"] }`** — the
exact spec already vetted twice in this repo. This *is* "harness patterns already used in the project,"
applied directly, not a fresh choice.

**Probe (honest case against):** choosing `ureq` trades away the tokio-based concurrent-dispatch design
originally sketched — §4.2's thread-pool + `mpsc` design is the accepted, consciously-chosen
alternative, not a free substitution. Tokio stays confined to the kernel's already-optional `pgrust`
feature, never pulled into `llm-adapters`.

### What does / doesn't need a DECART report, at a glance

| Item | DECART? | Why |
|---|---|---|
| Ollama as Tier-1 backend | **Yes — filed above** | New external service at a trust-relevant surface |
| HTTP client crate | **Yes — filed above** | New crate |
| A vector index for Layer-B, if it grows past linear scan | Yes, if a crate is added | Brute-force cosine over a few hundred embeddings needs no dep; an ANN library later would |
| GPTCache or any semantic-cache *library* | Flagged so nobody adds it | We borrow the *design* (free); the Python library would lose a DECART against a Rust core |
| `VllmAdapter` deployment | Yes + O18 GPU-unlock | Unchanged — design-only until the trigger fires |
| Exact-match sha3 cache (Layer A) | No | Reuses `backup.rs` — internal, no new dep |
| `TokenBucket` (F33) | No | Zero-dep spec'd internal primitive |
| `std::thread`/`mpsc` dispatch | No | Standard library |

---

## 6. Build sequence — corrected to its real wave structure

> **⚠ Correction made during this consolidation.** The original plan called steps (a)→(b)→(c)→(d)
> "strictly ordered — each unlocks the next." Re-checking the actual dependency graph: **(b), (c), and
> (d) each independently wrap or consume what (a) provides — none of them depends on each other.**
> `CachingBackend` (b) wraps any `LlmBackend`; the embeddings-gate consumer (c) only needs
> `.embed()`; the `Dispatcher` (d) only needs an adapter to dispatch to. The real structure is **one
> foundational wave, then three genuinely parallel ones** — exactly the "waves of swarms" framing this
> document is asked to account for. A future implementation session (or a swarm of three) can build
> (b), (c), (d) simultaneously once (a) merges.

### WAVE 0 — Step (a): `LlmBackend` trait + `OllamaAdapter` (foundational, must be first)

1. `kernel/src/ports/mod.rs` + `kernel/src/ports/llm.rs` (types only, §2.1) → `pub mod ports;` in
   `lib.rs`. Build: zero new entries in `kernel/Cargo.lock`.
2. `llm-adapters/Cargo.toml` + the crate skeleton (§2.3). `chat()` → `/v1/chat/completions`; `embed()`
   → `/v1/embeddings`; model routing per §2.2. **The §5 DECART report lands in the same commit.**
3. `llm-adapters/tests/ollama_roundtrip.rs` — a real (not mocked) call against the live daemon.

**Done-check:**
```
cd kernel && cargo build && cargo tree | grep -Ei "reqwest|ureq|llm-adapters"   # → empty
cd llm-adapters && cargo test ollama_chat_roundtrip                             # → passes; non-empty
                                                                                  #   content + usage.total_tokens
systemctl stop ollama && cargo test ollama_chat_roundtrip   # → typed Err, not a panic, not a mock
systemctl start ollama
```

### WAVE 1 — Steps (b), (c), (d): parallel-safe, each depends only on Wave 0

**(b) — exact-match sha3 response cache** (`llm-adapters/src/cache.rs`, §3.2)
`CachingBackend<B: LlmBackend, S: BlockStore>` wraps any backend; key via
`dowiz_kernel::event_log::sha3_256` over a `BTreeMap`-canonical request; store via
`dowiz_kernel::backup::{BlockStore, MemStore}`.
- **Done-check:** identical `temperature=0` request twice → second call is byte-identical with **zero**
  HTTP call (assert via a call-counting test double); changed sampling param or model digest → miss.

**(c) — semantic-leakage-gate embeddings consumer** (wires into `kernel/src/evals.rs`)
Wires `OllamaAdapter.embed()` (`nomic-embed-text`) into VERIFIABLE-COGNITION §3.3's cosine-0.9 gate,
currently deferred (`evals.rs:6-7,60`). `MintLog` stays the exact-duplicate half; this adds the
semantic half as a sibling. Uses `CachePolicy::Exact`/`NoCache` only — **never** Layer-B (§3.3's own
boundary: a gate can never trust a fuzzy cache hit). **One open item, honestly flagged, not guessed**:
the exact call site inside `evals.rs`'s scoring loop needs one more read of the file
(`MetamorphicGenerator`'s scoring path past `spectral_similarity`) at implementation time.
- **Done-check:** the gate rejects a planted near-duplicate item (cosine > 0.9 via local
  `/v1/embeddings`), RED→GREEN; with the daemon stopped, the gate fails CLOSED (typed `Err`, item not
  admitted), never passes silently.

**(d) — `TokenBucket`(F33)-bounded dispatch** (`kernel/src/token_bucket.rs` + `llm-adapters/src/dispatch.rs`, §4.2)
- **Done-check:** the F33 concurrent property test passes (total granted never exceeds
  `capacity + refill_rate·elapsed`); exhausting the bucket returns typed `Err(BudgetExceeded)`, never a
  silent downgrade; one `track_record.jsonl` row per dispatched call with non-null `{model, tokens,
  ms}`.

**Why Wave 0 first, Wave 1 parallel:** (a) is the one thing everything else needs and has zero
remaining install cost (Ollama is already up). Once it lands, (b)/(c)/(d) touch disjoint files
(`cache.rs` / `evals.rs` / `dispatch.rs`+`token_bucket.rs`) and read-only-consume the same `(a)` — a
textbook parallel-safe fan-out, not a forced sequence. After Wave 1, Tier-1 is genuinely live,
cached, gate-integrated, and budgeted — M5 hub-autonomy demonstrated for the first time with a real,
exercised local backend.

---

## 7. Development principles (binding on the implementation that follows this plan)

> Per operator directive 2026-07-16: this plan→DECART→blueprint→wave-corrected-consolidation process
> is itself a precedent (codified as a standing rule in `AGENTS.md`, see the "Detailed Planning
> Protocol" section added alongside this document). The principles below are what the *implementation*
> phase — whenever it starts — is bound to, so the discipline that produced this plan doesn't stop at
> the plan.

- **Spec-driven development**: every file/struct/function named in §2-§4 is the spec. Implementation
  follows it; if reality forces a deviation, the deviation is written back into this document (matching
  every "⚠ CORRECTED" marker already in this arc's history), never silently diverged from.
- **TDD (red→green)**: every "Done-check" in §6 is written and run RED (failing / not-yet-true) *before*
  the code that makes it pass. A done-check discovered to be untestable as stated is a spec bug, fixed
  in this document, not worked around in code.
- **DoD (Definition of Done)**: a step is done when its falsifiable done-check passes on a clean
  checkout, not when it "looks right." No step in §6 is markable complete without its done-check's
  actual output pasted into the commit/PR, per this session's own established evidence discipline.
- **Event-driven**: every dispatched LLM call is an event feeding the existing `event_log`/H1 harvest
  ledger (§4.2) — this backend is a *consumer and producer* of dowiz's event-sourced substrate, never a
  side-channel that bypasses it.
- **Mesh architecture (M5)**: backend choice is `HubPolicy` config, never a kernel recompile; no
  dev-time gate in this document may block a runtime hub from switching backends (SCOPE RULE); every
  new capability (embeddings, caching, dispatch) is additive to the `LlmBackend` port, never a
  hub-specific fork of it.
- **Context-awareness**: every claim in §1 was re-verified against the live host, not carried forward
  from an earlier document unchecked — the two corrections in §4 and §6 exist *because* this
  consolidation re-checked rather than copied. Future work on this arc inherits the same obligation.

---

*Supersedes and replaces (deleted 2026-07-16): `HARNESS-LLM-BACKEND-PLAN.md`,
`DECART-llm-backend-integration.md`, `BLUEPRINT-LLM-BACKEND-PORT.md`. Companion documents (unchanged,
still separate): `docs/design/sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md` §2 H3
(the original scoping), `BLUEPRINT-P15-living-organism-unbounded.md` §9 (the first version of this
port design), `BLUEPRINT-P11-compute-budget-cache.md` §4 (F33 `TokenBucket` spec, unchanged),
`kernel/src/backup.rs` / `event_log.rs` / `evals.rs` (reused primitives). Planning only — no code
written; implementation begins when the operator says so, on `feat/harness-llm-backend`.*

---

## Audit addendum (2026-07-17, appended — Phase-27 fault-isolation audit of the landed code)

Two verified defects in the implementation that has since landed on `feat/harness-llm-backend`
(full context: `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` §1.2):

1. **The Dispatcher's concurrency bound is dead code (A4, HIGH).** `llm-adapters/src/dispatch.rs`
   stores `workers` (`:60,:70`) but never reads it; `dispatch()` does `thread::spawn`
   unconditionally per call (`:89`). The module doc's claim "N workers ≤ the backend's own
   parallelism cap (e.g. 2 for Ollama)" is not enforced — `TokenBucket` bounds volume over time,
   not concurrent in-flight calls, so arbitrarily many threads can hammer the backend at once.
   Fix owned by Phase-27 Wave F1b: a counting semaphore honoring `workers`, over-cap ⇒ typed
   `Busy` refusal (degrade-closed). Done-check: 3×`workers` concurrent dispatches against a slow
   fake backend ⇒ observed in-flight never exceeds `workers`.
2. **The exact-match cache is unbounded (A3, HIGH — convergently found twice).**
   `CachingBackend`'s `Arc<Mutex<MemStore>>` (`cache.rs:36-44` → `kernel/src/backup.rs:78-86`)
   has no eviction/TTL/size cap; every distinct prompt tuple is cached for process lifetime. Fix
   is OWNED by Phase 26 (`BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md` §1.4 —
   byte-bounded LRU behind the `BlockStore` trait); recorded here so this doc's cache section is
   not read as final. Related latent hazard (A12): the shared cache lock is taken with
   `.lock().unwrap()` (`cache.rs:94,104`) — safe only while the store impl is panic-free inside
   the lock; the Phase-27 poisoning-discipline sweep covers it.
