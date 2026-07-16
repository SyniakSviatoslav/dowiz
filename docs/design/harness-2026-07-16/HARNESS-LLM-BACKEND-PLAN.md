# HARNESS — `LlmBackend` PORT, CACHING & PARALLELISM PLAN (2026-07-16)

> **Design plan, not a fix. No code written or edited here** — the operator starts implementation on a
> new branch after this lands. Finalizes the `LlmBackend` port design from
> [`HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md`](../sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md) §2 H3,
> [`LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md`](../sovereign-roadmap-2026-07-16/LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md) §5, and
> [`BLUEPRINT-P15-living-organism-unbounded.md`](../sovereign-roadmap-2026-07-16/BLUEPRINT-P15-living-organism-unbounded.md) §9,
> against the **actual, re-verified live host state** of 2026-07-16. The single largest change since that
> work: **Ollama is already running as a systemd service with four models pulled** — the earlier
> "install llama-server as a new systemd unit" plan is now obsolete. Every local fact below was probed
> live on this host today; external facts carry inline citations.

---

## 1. Current-state evidence (re-verified live, 2026-07-16)

**No GPU. Confirmed.** `nvidia-smi` → `command not found`; `ls /dev/nvidia*` → *No such file or directory*.
This host is 8-vCPU AMD EPYC Milan / 32 GB RAM, GPU-less. The vLLM Tier-2 gate (O18 GPU-unlock) is
therefore still correctly closed — nothing in this plan loosens it.

**Ollama is already the local Tier-1 inference server — zero install remaining.** Verified:

- `systemctl is-active ollama.service` → **active**, running since **2026-07-13** (`/usr/local/bin/ollama
  serve`, `User=ollama`), reachable at `http://127.0.0.1:11434`. Version (`/api/version`) → **0.30.9**.
- `ollama list` → four models: **`qwen2.5-coder:7b`** (4.7 GB, code-shaped tasks), **`llama3.1:8b`**
  (4.9 GB, general), **`nomic-embed-text:latest`** (274 MB, embeddings), **`qwen3-embedding:0.6b`**
  (639 MB, embeddings).
- `ollama ps` (live) → **three models resident simultaneously** on 100 % CPU (`llama3.1:8b` 5.6 GB,
  `qwen3-embedding:0.6b` 2.4 GB, `nomic-embed-text` 376 MB), each with `UNTIL … from now` — confirming
  the keep-alive residency cache is active (default `OLLAMA_KEEP_ALIVE=5m`). Proof the 32 GB host holds a
  chat model **and** both embedding models resident at once, exactly as LLM-INFRA §3.1 predicted.

**Ollama speaks BOTH API surfaces — probed, not assumed:**

| Endpoint | Probe result |
|---|---|
| `GET /v1/models` (OpenAI-compat) | `{"object":"list","data":[{"id":"qwen2.5-coder:7b",…}]}` — 200 |
| `POST /v1/chat/completions` | `{"id":"chatcmpl-109","object":"chat.completion",…,"usage":{"prompt_tokens":12,"completion_tokens":3,"total_tokens":15}}` — **full OpenAI schema incl. `usage` token accounting**, `system_fingerprint:"fp_ollama"` |
| `POST /v1/embeddings` (OpenAI-compat) | `{"object":"list","data":[{"object":"embedding","embedding":[…]}]}` — 200 (nomic) |
| `POST /api/embed` (native) | `{"model":"qwen3-embedding:0.6b","embeddings":[[…]]}` — 200 |

So the OpenAI-compat `/v1/*` surface (chat + embeddings + models) is live **and** returns per-call token
counts — which the H1 harvest ledger (`track_record.jsonl`) needs to price local calls. The embeddings
bridge that BLUEPRINT-P15 §9.6 and VERIFIABLE-COGNITION §3.3 named as *deferred* is **available right now,
at $0/call**, against either embedding model.

**Parallelism config is at defaults (unset env) — knobs confirmed present.** `ollama serve --help` exposes
`OLLAMA_NUM_PARALLEL` (max parallel requests **per model**), `OLLAMA_MAX_QUEUE` (queued-request cap before
HTTP 503), `OLLAMA_MAX_LOADED_MODELS`, `OLLAMA_KEEP_ALIVE`, `OLLAMA_CONTEXT_LENGTH`. The service env
(`/proc/<pid>/environ`) sets none of these, so Ollama's auto-defaults are in effect: `OLLAMA_NUM_PARALLEL`
defaults to **1, auto-selecting 1 or 4 by available memory**; `OLLAMA_MAX_QUEUE` defaults to **512**
([Ollama FAQ](https://docs.ollama.com/faq); [glukhov.org — how Ollama handles parallel requests](https://www.glukhov.org/llm-performance/ollama/how-ollama-handles-parallel-requests/)).
This matters for §4: Ollama's own request-level parallelism is real and free, but on this CPU-only box it
is memory-bound and currently un-tuned — the harness must reuse it, not fight it.

**Net delta vs the prior design:** `LlamaCppAdapter` + "install llama-server systemd unit" → **replaced by
`OllamaAdapter` pointing at the already-running `127.0.0.1:11434`.** Nothing to install, nothing to
sha3-fetch for the first slice (Ollama already manages the four pulled blobs). The GGUF `{url, sha3}`
verify-or-deny path (F3/F27) still applies to *future* model pulls but is not on the critical path for
turning Tier-1 on today.

---

## 2. The finalized `LlmBackend` adapter design

### 2.1 Port (kernel side — trait only, zero vendor knowledge)

Unchanged in shape from LLM-INFRA §5.2 / P15 §9.1 — the trait was already correct; only the adapter set
changes. `ports/llm.rs` defines `trait LlmBackend { id, caps, chat, embed, rerank, health }`; the kernel
sees only `&dyn LlmBackend`. Adapters live in a **separate crate the kernel never imports** (compile
firewall, VERIFIABLE-COGNITION §7) — `cargo tree -p <kernel-crate>` must show no HTTP client. `caps()` is
fail-closed feature discovery; `rerank` returns `Err(Unsupported)` where a backend lacks it; `health()`
returns a typed `Err` when the backend is absent (E21 — never a mock, never a silent tier fallback).

### 2.2 One transport, three adapters — with a **verified** quirks split

**The load-bearing claim checked:** do Ollama's `/v1` and vLLM's OpenAI-compat server speak the same wire
shape closely enough to share one `OpenAiCompatTransport`? **Yes at the schema level, no at the details —
so the Hermes-proven `Quirks`-struct pattern this repo already cites is required, not optional.** Live
probes surfaced concrete Ollama quirks that a naive OpenAI client would trip on:

1. **Model-name format.** Ollama model ids carry a `:tag` (`llama3.1:8b`, `qwen2.5-coder:7b`,
   `qwen3-embedding:0.6b`) — not the bare `gpt-4o` shape. The adapter must pass the tagged id through
   verbatim; a transport that strips or normalizes after `:` breaks Ollama.
2. **`system_fingerprint:"fp_ollama"`** is a constant sentinel, not a real fingerprint — the transport
   must not key caching or dedup on it (it would collapse all Ollama responses; §3 keys on request, not
   response fingerprint).
3. **Embeddings input shape.** OpenAI-compat `/v1/embeddings` takes `{"input": …}` and returns
   `{"data":[{"embedding":[…]}]}`; the native `/api/embed` takes `{"input": …}` and returns
   `{"embeddings":[[…]]}` (note the nesting difference). The adapter picks `/v1/embeddings` for
   OpenAI-parity and treats `/api/embed` as the native fallback quirk.
4. **`keep_alive`, `num_ctx`, `think`** are Ollama/native-only knobs (surfaced through `options`), absent
   from OpenAI's schema — exactly the `think=false` / `num_ctx` quirks Hermes' `custom` provider already
   models (`plugins/model-providers/custom/`).
5. **Function/tool-calling and structured-output support differ per backend and per model** — a `Caps`
   probe, not an assumption.

Therefore: **one `OpenAiCompatTransport` (HTTP + OpenAI JSON envelope) + a per-adapter `Quirks` struct**
(base_url, model-id policy, embeddings-endpoint choice, extra-options passthrough, tool-calling flag). This
is precisely the pattern R1-B verified in Hermes and that P15 §9.1 already prescribes — the probes
*confirm* it rather than change it.

| Adapter | base_url | Tier / gate | Quirks (verified) |
|---|---|---|---|
| **`ManagedApiAdapter`** | headroom proxy → OpenRouter | **Tier 0 — DEFAULT, live now** | key from EnvFile (S3); the current reality formalized by P05; standard OpenAI envelope |
| **`OllamaAdapter`** | `127.0.0.1:11434` (already running) | **Tier 1 — live NOW, operator-DECART to wire in** | `:tag` model ids; `fp_ollama` sentinel; `/v1/embeddings` primary, `/api/embed` fallback; `keep_alive`/`num_ctx` options; model routing — `qwen2.5-coder:7b` for code-shaped tasks, `llama3.1:8b` general, `nomic-embed-text`/`qwen3-embedding:0.6b` for the embeddings capability |
| **`VllmAdapter`** | `127.0.0.1:8000` or Modal URL | **Tier 2 — stays O18 GPU-gated** | native OpenAI-compat; `/score`/`/rerank` extensions; Modal H100 burst behind the same ceiling (E22/F34) |

**Model selection inside `OllamaAdapter`** is a small task→model policy, config-driven per hub: a
`ChatRequest` tagged `code` routes to `qwen2.5-coder:7b`; `general` to `llama3.1:8b`; an `EmbedRequest`
routes to `nomic-embed-text` (768-dim, fast, 274 MB) by default with `qwen3-embedding:0.6b` as the
higher-quality option. This is *within-adapter* routing and must not be confused with the Phase-5
`gov_route` dev-tooling router (F35 distinction, P11 §4) — the adapter picks *which local model*, the hub
policy picks *which backend*.

### 2.3 Hub choice = config, and the guards that bind regardless (unchanged, still correct)

A hub selects the backend via `HubPolicy.llm_backend` / EnvFile `LLM_BACKEND=` + `LLM_BASE_URL=` — **swap
is config, never a kernel recompile** (M5 made real; no dev-time gate may block a runtime hub from
switching, SCOPE RULE). Backend-independent LOCKs bind for every tier: **F3/F27** sha3 verify-or-deny on
any *future* weight pull; **F6/E19** `TokenBucket`/`Budget` ceiling with typed refusal (§4); **M8**
local-only telemetry (Ollama's logs/metrics never leave the host); **M12** red-line scopes unchanged;
**E21** honest `Err` when the backend is down.

---

## 3. The caching design

### 3.1 What is already cached for free (know it, don't rebuild it)

- **vLLM (Tier-2, later):** the V1 engine does **automatic prefix caching** — KV-cache blocks are reused
  across requests that share a prompt prefix, via paged non-contiguous KV blocks and iteration-level
  scheduling ([runpod — vLLM PagedAttention & continuous batching](https://www.runpod.io/articles/guides/vllm-pagedattention-continuous-batching)).
  (Naming note carried from LLM-INFRA §2: the "PagedAttention" *descriptor* was retired upstream in vLLM
  v0.25.0; the paged-KV *mechanism* persists as the V1 / Model Runner V2 engine.) This is intra-server KV
  reuse — it still runs a forward pass.
- **Ollama (Tier-1, now):** keeps the model **resident** between calls (`OLLAMA_KEEP_ALIVE`, observed live
  in `ollama ps`) so there is no reload cost, and reuses KV context for a shared prefix within a loaded
  model. Also still a forward pass.

**Both are additive to, not a substitute for, a harness response cache:** the harness cache below returns a
stored answer with **zero forward pass at all** — a hash + a lookup. The two layers do not overlap.

### 3.2 Layer A — exact-match response cache (reuse dowiz's content-addressing verbatim)

**Design:** key `= sha3_256(model_id ⊕ normalized_prompt ⊕ canonical_params)`; value = the stored
`ChatResponse`. A cache hit costs **one hash + one lookup, zero model call.** This reuses — verbatim — the
content-addressing pattern the kernel already ships in **`kernel/src/backup.rs`**: the `BlockStore` trait
whose `put` is *idempotent* ("storing a block whose id already exists is a no-op — that IS the dedup"),
keyed by the chunker's `sha3_256` id via `crate::event_log::sha3_256`. The response cache is a
`BlockStore`-shaped map from a `sha3_256` request-key to a serialized response; `MemStore` is the
single-node in-memory implementation already present. **No new hashing primitive, no new store abstraction
— the exact `Hash = sha3_256` content-address the backup organ uses.**

- **Normalization** (so trivially-equal prompts collide correctly): canonical JSON of `{model_id,
  messages, temperature, top_p, max_tokens, seed, tools}` with sorted keys; whitespace-insensitive only if
  the task class allows (default: byte-exact — determinism first).
- **Correctness = exactness.** Because the key includes `model_id` and every sampling param, a hit is a
  *provably identical* request. This layer is therefore **safe for all task classes, including
  gate-critical ones** — it never returns a different answer than the model would have. (Caveat:
  temperature > 0 makes even identical requests non-deterministic at the model; for `temperature=0` /
  `seed`-pinned calls the cache is exact, for sampled calls it returns *one valid* prior sample — which is
  acceptable for advisory calls and must be **disabled for any call whose contract requires a fresh
  sample**, a per-request `no_cache` flag.)
- **Eviction:** LRU with a byte ceiling, plus TTL on entries whose `model_id` tag can move (Ollama tags
  are mutable — a re-pulled `llama3.1:8b` is a different model behind the same tag; invalidate on the
  model's `created`/digest changing, read from `/v1/models`).
- **Invalidation:** keyed on `model_id` **digest**, not just tag, closes the "same tag, new weights" hole.

### 3.3 Layer B — near-duplicate / semantic cache (embedding-gated, advisory-only)

**Design:** embed the incoming prompt via `OllamaAdapter.embed()` (`nomic-embed-text`), search a small
in-memory / content-addressed index of recent cached prompt-embeddings, and serve the cached response if
`cosine ≥ τ` **only for task classes that tolerate a near-answer.** This borrows the GPTCache architecture
— embed → vector nearest-neighbor → similarity-evaluator → threshold → serve-or-miss, with similarity-based
and TTL eviction ([zilliztech/GPTCache](https://github.com/zilliztech/gptcache);
[GPTCache paper, NLPOSS 2023](https://aclanthology.org/2023.nlposs-1.24.pdf)). Threshold grounding: the
GPT-Semantic-Cache study found **τ ≈ 0.8** the optimal trade-off (hit rate up to 68.8 %, positive-hit
accuracy > 97 %) ([arXiv 2411.05276](https://arxiv.org/pdf/2411.05276)) — a defensible starting τ, to be
tuned against dowiz's own harvested data, not adopted as gospel.

**The hard boundary — stated explicitly and non-negotiably:**

> **Layer B (semantic cache) is for advisory / exploratory tasks ONLY. It is NEVER consulted for any
> gate-critical call** — anything the project's VERIFIED-BY-MATH discipline requires exact determinism
> from: deterministic-oracle evals, the leakage gate itself, money/auth/RLS/migration-adjacent
> reasoning, any `deliberate()` path whose output a deterministic gate consumes. A near-duplicate prompt
> is **not** an identical prompt; returning a stale answer for a subtly different question is precisely
> the proxy-over-ground-truth failure canon forbids.

Enforcement shape: a `CachePolicy` enum on `ChatRequest` — `Exact` (Layer A only, default),
`SemanticOk{τ}` (Layers A then B), `NoCache`. Gate-critical call sites pass `Exact` or `NoCache`; only
call sites explicitly marked advisory may pass `SemanticOk`. The boundary is a **type**, not a convention,
so a gate-critical caller cannot accidentally opt into fuzzy hits.

---

## 4. The parallel-processing design

### 4.1 Reuse Ollama's own request-level parallelism (don't reinvent it)

vLLM's core value-add is **continuous (iteration-level) batching**: the scheduler re-evaluates its
waiting/running/swapped queues after *every* forward pass, evicting finished sequences and slotting waiting
ones into freed KV blocks — no head-of-line blocking, GPUs saturated every step
([runpod, ibid.](https://www.runpod.io/articles/guides/vllm-pagedattention-continuous-batching)). That is a
Tier-2 (GPU) capability and stays gated. **Ollama has a real, already-available analogue on CPU:**
`OLLAMA_NUM_PARALLEL` lets one loaded model serve N concurrent requests (each consuming a KV slot), with
`OLLAMA_MAX_QUEUE` (default 512) bounding the wait queue before HTTP 503
([glukhov.org, ibid.](https://www.glukhov.org/llm-performance/ollama/how-ollama-handles-parallel-requests/)).
On this CPU-only host this is **memory-bound** — every parallel slot needs its own KV cache, and CPU
inference is bandwidth-bound, so raising `NUM_PARALLEL` past ~2–4 will thrash. **Harness policy: set
`OLLAMA_NUM_PARALLEL` explicitly (start at 2) in the service EnvFile and let Ollama own intra-model
batching; the harness does not re-implement batching.**

### 4.2 Harness-level concurrency control — `TokenBucket`-bounded `tokio` dispatch

> **⚠ CORRECTED 2026-07-16.** This subsection's `tokio`-based framing predates
> `DECART-llm-backend-integration.md`'s Decision 2, which chose `ureq` (synchronous, no tokio) for
> `llm-adapters`'s HTTP client — matching the exact dependency already used twice elsewhere in this
> repo (`tools/telemetry/rust-spool`, `tools/async-spool`) under the 2026-07-15 operator mandate. The
> `TokenBucket`(F33) primitive below is unchanged; the dispatch *mechanism* around it is not tokio —
> see `BLUEPRINT-LLM-BACKEND-PORT.md` §4 for the corrected thread-pool + `std::sync::mpsc` design.

**Honest correction to the prior design:** P15 §9.4 says "reusing … the existing `transport_policy.rs`
`TokenBucket`." **There is no such file and no existing Rust `TokenBucket`** — a repo-wide
`grep` for `TokenBucket`/`token_bucket` over `*.rs` returns nothing. What exists is the **F33
specification** in [`BLUEPRINT-P11-compute-budget-cache.md`](../sovereign-roadmap-2026-07-16/BLUEPRINT-P11-compute-budget-cache.md)
§4: a zero-dep kernel primitive — `capacity`, `refill_rate` (tokens/sec), a **monotonic-clock** last-refill
timestamp (never wall-clock, so an NTP jump can't bypass the throttle), atomic token count, `try_acquire(n)
-> bool` that refills lazily and grants iff `tokens ≥ n` — with a stated concurrent property-test falsifier
("total granted across any window never exceeds `capacity + refill_rate·elapsed`"). P11 §4 explicitly
records "no Rust `TokenBucket` (F33; the old TS limiter was atticked)." So the design here is: **build the
F33 `TokenBucket` per its P11 §4 spec** (it is small and already fully specified) and use it as the shared
mechanism — *not* import a file that doesn't exist. This correction should also be folded into the P15 §9.4
line at the next canon merge (it currently over-claims an existing file).

**Dispatch pattern:**

- One `TokenBucket` **per hub / per agent** meters both **spend** (managed tier: $ / tokens) and
  **concurrency** (local tier: in-flight-request tokens). 1 token = 1 unit of the metered resource (P11 §4).
- Async dispatch = `tokio` tasks, each acquiring from the bucket before issuing a backend call:
  `bucket.try_acquire(cost)? ⇒ spawn ⇒ on completion, telemetry`. Over-budget = **typed refusal**
  (`Err(BudgetExceeded)` / F6/E19), **never a silent downgrade** — degrade-closed (P11 §4's load-bearing
  word). This binds identically for managed and local tiers.
- A `tokio::sync::Semaphore` (std-adjacent, already in the async runtime — not a new dep) bounds *in-flight
  concurrency* to a per-backend ceiling. **Per-model request queues** exist when the harness's desired
  concurrency exceeds Ollama's `OLLAMA_NUM_PARALLEL` cap: the harness holds the overflow in a bounded
  channel (`tokio::mpsc`) rather than letting Ollama's `MAX_QUEUE` 503, so back-pressure is visible and
  typed at the harness boundary, not a raw HTTP error.
- Telemetry: every dispatched call emits a row for the H1 harvest ledger (`track_record.jsonl`), pricing
  local calls by the `usage.total_tokens` Ollama already returns (verified §1) plus a CPU-time proxy —
  closing the EV loop so `gov_route` prices local-vs-managed on measured data (P15 §9.7).

---

## 5. DECART-flagged items (anything not already a reused dowiz pattern)

Per [`integration-decart-rule.md`](../operating-model/integration-decart-rule.md) and AGENTS.md §"Integration
Decart Rule": a **new dependency / external service / backend / transport, or replacing one with another**,
needs a dated DECART comparison report (the 7-criterion table + mandatory probe) *before* adoption. What
qualifies here, and what does **not**:

| Item | DECART needed? | Why |
|---|---|---|
| **Wiring Ollama in as `OllamaAdapter` (Tier-1 backend)** | **YES** — a new external inference service at a trust-relevant surface | Even though Ollama is already running, *the kernel adopting it as a backend* is a new integration. Report should compare Ollama-serve vs a fresh llama-server unit vs managed-only, on the 7 criteria; the probe: "the strongest case against is that Ollama adds a Go daemon + its own model store outside the sha3 manifest — answer honestly." |
| **An HTTP client crate for `OpenAiCompatTransport`** (e.g. `reqwest`/`ureq`/`hyper`) | **YES** — new crate | The adapter crate needs one HTTP client; pick it by DECART (Rust-native fit, supply-chain, reversibility). It lives in the adapter crate only (compile firewall), never the kernel. |
| **A vector index for Layer-B semantic cache** (if it grows past a linear scan) | **YES if a crate is added** | A brute-force cosine over a few hundred recent embeddings needs **no dependency** (a `Vec<f32>` dot product — no DECART). Adopting an ANN library (hnsw/faiss-rs) later is a new dep ⇒ DECART. |
| **GPTCache / any semantic-cache library** | **YES — but we are NOT adopting the library** | We borrow GPTCache's *design* (embed→NN→threshold→TTL), which is free. Pulling `gptcache` (Python) as a dependency would need a DECART and would almost certainly lose (Python dep at a Rust core). Flagged so nobody `pip install`s it. |
| **`VllmAdapter` deployment** | **YES + O18 GPU-unlock** | Unchanged: Tier-2 stays gated on real GPU-unlock *and* a DECART. Design-only here. |
| Exact-match sha3 cache (Layer A) | **No** | Reuses `backup.rs` `BlockStore`/`sha3_256` — internal, no new dep. |
| `TokenBucket` (F33) | **No** | A zero-dep kernel primitive already specified in P11 §4; building a spec'd internal primitive is not an "integration." |
| `tokio` concurrency / `Semaphore` / `mpsc` | **No** | Already the kernel's async runtime; no new dep. |

---

## 6. First-slice build sequence (ordered, small, falsifiable)

The operator starts real implementation on a new branch after this lands. Each step is sized for **one
focused session** and ends with a **falsifiable done-check (a real command/test, not vibes)**. Steps are
strictly ordered — each unlocks the next.

### Step (a) — `LlmBackend` trait + `OllamaAdapter` (smallest real thing; zero new deps beyond one HTTP client)

Define `ports/llm.rs` (`trait LlmBackend { id, caps, chat, embed, rerank, health }`) in the kernel; build
the `OllamaAdapter` + `OpenAiCompatTransport` + `Quirks` in a **separate adapter crate**. `chat()` →
`/v1/chat/completions`; `embed()` → `/v1/embeddings`; model routing per §2.2. **DECART report for the Ollama
integration + the HTTP-client crate lands in the same commit** (§5).

- **Done-check:** `cargo test -p <adapter-crate> ollama_chat_roundtrip` passes against the live daemon —
  a `chat()` to `llama3.1:8b` returns non-empty content and a populated `usage.total_tokens`; **AND**
  `cargo tree -p <kernel-crate>` shows **no HTTP client and no adapter crate** (compile firewall intact);
  **AND** with the daemon stopped, `health()` returns a typed `Err`, not a panic and not a mock.

### Step (b) — exact-match sha3 response cache (Layer A; reuses `backup.rs` content-addressing)

Wrap `OllamaAdapter.chat()` in a `CachingBackend` whose key is `sha3_256(model_id ⊕ canonical_request)`
over a `BlockStore`-shaped `MemStore` (§3.2). `CachePolicy::Exact` default; LRU + digest-invalidation.

- **Done-check:** a test issues the same `temperature=0` request twice; the second returns byte-identical
  content with **zero HTTP call** (assert via a call-counter or a mocked transport hit-count of 1);
  **AND** changing any sampling param or the `model_id` digest produces a miss (new call). Idempotence:
  the store's `put` for an already-present key is a no-op (`stored_bytes` unchanged) — the `backup.rs`
  dedup invariant, re-asserted.

### Step (c) — semantic-leakage-gate embeddings consumer (first REAL value; closes a deferred BLUEPRINT-P15 gap)

Wire `OllamaAdapter.embed()` (`nomic-embed-text`) into VERIFIABLE-COGNITION §3.3's cosine-0.9 semantic
leakage gate — the gate `kernel/src/evals.rs` explicitly marks *deferred pending an embedding bridge*
(lines 6–7, 60: "the cosine-0.9 semantic gate requires the embedding bridge and is deferred"). The
`MintLog` structural (content-hash) gate already exists; this adds the semantic half. Embeddings feed the
**gate**, so this consumer is `CachePolicy::Exact`/`NoCache` — **never** Layer-B semantic cache (§3.3
boundary).

- **Done-check:** the gate **rejects a planted near-duplicate** eval item (cosine > 0.9 to an existing
  item via local `/v1/embeddings`) — RED→GREEN; **AND** with the Ollama daemon stopped the gate **fails
  CLOSED** (typed `Err`, item not admitted), never passes silently. This is P15 §9 acceptance-item 5,
  now buildable at $0/call.

### Step (d) — `TokenBucket`(F33)-bounded concurrent dispatch (spend + concurrency ceiling)

Build the F33 `TokenBucket` per P11 §4 (monotonic clock, atomic `try_acquire`), and a `tokio`-based
`Dispatcher` that acquires before every backend call, bounds in-flight concurrency with a `Semaphore`, and
holds overflow in a bounded `mpsc` queue (§4.2). Emit an H1 harvest row per call using Ollama's returned
`usage`.

- **Done-check:** the **F33 concurrent property test** (P11 §4 falsifier) passes — under many concurrent
  `try_acquire` callers, total granted over any window never exceeds `capacity + refill_rate·elapsed`;
  **AND** a test that exhausts the bucket makes the next dispatch return a **typed `Err(BudgetExceeded)`**
  (degrade-closed), never a silent downgrade to the managed tier; **AND** one harvested
  `track_record.jsonl` row exists per dispatched call with a non-null `{model, tokens, ms}`.

**Why this order:** (a) unlocks everything and has zero remaining install cost (Ollama is up); (b) is a
tiny content-addressing reuse that makes every later call cheaper and is safe for all task classes; (c) is
the first consumer that turns capability into *value* — it closes a gap the blueprint itself named as
deferred, and it is gate-critical so it forces the §3.3 cache boundary to be real from day one; (d) adds
the spend/concurrency safety rail and closes the EV measurement loop back to H1. After this slice, Tier-1
is genuinely live, budgeted, cached, and feeding the routing ledger — M5 hub-autonomy demonstrated for the
first time with a real, exercised local backend.

---

*Companion to: HARNESS-IMPROVEMENT-SYNTHESIS-PLAN §2 H3, LLM-INFRA-RESEARCH §2–§5, BLUEPRINT-P15 §9,
BLUEPRINT-P11 §4 (F33 `TokenBucket` spec — the real source, corrects the P15 §9.4 "existing
transport_policy.rs" over-claim), VERIFIABLE-COGNITION-BLUEPRINT §3.3/§3.5/§7, `kernel/src/backup.rs`
(`BlockStore`/`sha3_256` content-addressing), `kernel/src/evals.rs` (`MintLog`; deferred cosine gate),
`eval-layer/openrouter_judge.py:41` (`OPENAI_BASE_URL`), `docs/operating-model/integration-decart-rule.md`.
External facts cited inline: Ollama FAQ/glukhov (parallelism defaults), runpod (vLLM continuous batching /
V1 engine), zilliztech/GPTCache + arXiv 2411.05276 (semantic-cache design & τ≈0.8). This document edits no
code and no canon; §5 DECART items and the P15 §9.4 correction are staged for the operator's next merge.*
