# BLUEPRINT — LlmBackend Port: exact repo layout, signatures, migration (Steps a–d)

> Execution-ready detail for `HARNESS-LLM-BACKEND-PLAN.md`'s 4-step build sequence, grounded in the
> **actual, directly-verified structure of this repo** — not the abstract "a separate crate" framing
> the plan used. Everything below was checked against the live tree on `feat/harness-llm-backend`
> (branched from `feat/kernel-fsm-graph-analysis`) on 2026-07-16. Planning/blueprint only — no code
> written here, per the operator's explicit "just plan, no implementation yet" instruction.
>
> **⚠ Corrects `HARNESS-LLM-BACKEND-PLAN.md` §4.2**, which still describes `tokio`-based dispatch
> (`tokio::spawn`, `tokio::sync::Semaphore`, `tokio::mpsc`). That framing predates
> `DECART-llm-backend-integration.md`'s Decision 2 (`ureq`, synchronous, no tokio in the adapter
> crate) and is now stale — §4 below is the corrected design. This is exactly the kind of
> plan-vs-DECART drift this document exists to close.

---

## 1. Ground truth: exact repo layout (verified, not assumed)

- **No Cargo workspace exists.** `find /root/dowiz -maxdepth 2 -name Cargo.toml` → four independent
  crates: `kernel/Cargo.toml`, `engine/Cargo.toml`, `wasm/Cargo.toml`, `agent-governance-wasm/Cargo.toml`
  — each with its **own** `Cargo.lock`, no root `[workspace]`. A new `llm-adapters` crate is therefore
  **another standalone crate at repo root**, not a workspace member — it gets its own `Cargo.toml` +
  `Cargo.lock`, path-depends on `kernel/` directly (`dowiz-kernel = { path = "../kernel" }`), and is
  built with `cargo build -p llm-adapters` run *from its own directory* (matching how `engine/` is
  already built standalone per this session's earlier `BLUEPRINT-P02` finding).
- **`kernel/Cargo.toml`'s feature discipline** (verified, `kernel/Cargo.toml:1-40`): `default = ["std"]`,
  zero network/HTTP deps in the default build; `wasm` feature gates `serde`/`serde_json`/`serde_yaml`;
  `pgrust` feature gates `sqlx`/`tokio` — the **only** place `tokio` appears in this crate today, and
  only behind an opt-in feature. **The `ports::llm` trait module must add ZERO new dependencies to
  `kernel/Cargo.toml`** — no serde, no tokio, no HTTP crate — or it breaks this discipline the same way
  `pgrust` was carefully kept optional.
- **Existing submodule-directory convention to mirror**: `kernel/src/retrieval/mod.rs` — a `mod.rs`
  with one `pub mod x;` line + a doc comment per submodule, `#[cfg(feature = "wasm")]` gating where a
  submodule needs serde. `kernel/src/isolation/mod.rs` is the same pattern. **`kernel/src/ports/mod.rs`
  follows this exact shape.**
- **Existing content-addressing to reuse verbatim**: `kernel/src/backup.rs` — `pub trait BlockStore { put, get, len, is_empty }`
  (line 29), `pub struct MemStore` (line 44, `HashMap<Hash, Vec<u8>>`, `put` idempotent — "storing a
  block whose id already exists is a no-op"), `pub type Hash = [u8; 32]` (line 24). `kernel/src/event_log.rs:30`
  — `pub fn sha3_256(input: &[u8]) -> [u8; 32]`, already public, already zero-dep. **Step (b)'s cache
  reuses these two items directly — no new hashing, no new store trait.**
- **Existing structural-leakage-gate pattern to extend**: `kernel/src/evals.rs` — `pub struct MintLog`
  (line 74, `HashSet<(u64,u64)>` over two decorrelated FNV-1a streams) with
  `pub fn mint(&mut self, kind: &str, payload: &[u8]) -> Option<u64>` (line 87, returns `None` on an
  exact-duplicate). The file's own header comment (lines 6-7) and the mint-log section comment (line 60)
  state verbatim: *"the cosine-0.9 semantic gate is the embedding-bridge analogue (deferred §7)."*
  **Step (c) adds a sibling gate, not a rewrite of `MintLog`** — `MintLog` stays the exact-duplicate
  half; the new type is the semantic half, composed alongside it.
- **Existing HTTP-client precedent** (this is what Decision 2 in the DECART report reuses):
  `tools/telemetry/rust-spool/Cargo.toml` and `tools/async-spool/Cargo.toml` both depend on
  `ureq = { version = "2", default-features = false, features = ["tls", "json"] }` under the
  **2026-07-15 operator mandate** ("rustls with ring everywhere possible... NO tokio"). `llm-adapters`
  uses the **identical** dependency line — copy it, don't reinvent it.
- **`BLUEPRINT-P11-compute-budget-cache.md` §4** (sovereign-roadmap arc, already written) is the F33
  `TokenBucket` spec Step (d) builds from — capacity, refill_rate, monotonic-clock last-refill, atomic
  `try_acquire(n) -> bool`, degrade-closed. **That spec is unchanged by this document** — Step (d) below
  only corrects the *dispatch* mechanism around it (ureq/threads, not tokio).

---

## 2. Step (a) — `LlmBackend` trait + `OllamaAdapter`

### 2.1 Kernel side — `kernel/src/ports/mod.rs` + `kernel/src/ports/llm.rs` (new files)

```
kernel/src/ports/
├── mod.rs          # pub mod llm;  (mirrors retrieval/mod.rs's one-line-per-submodule shape)
└── llm.rs          # the trait + request/response types — ZERO new Cargo.toml deps
```

`kernel/src/lib.rs` gains one line among the existing `pub mod` list (alphabetical slot, between
`order_machine` and `retrieval` per the current ordering at `lib.rs:108-111`): `pub mod ports;`

`ports/llm.rs` signature sketch (types only, no logic — the kernel never talks HTTP):

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

No `serde` derives here — plain structs. The adapter crate owns JSON (de)serialization and converts
to/from these types manually. This IS the compile firewall: `cargo tree -p dowiz-kernel` must show no
HTTP client and no `llm-adapters` after this step, verified by the done-check below.

### 2.2 Adapter side — new standalone crate `llm-adapters/` (repo root, sibling to `kernel/`)

```
llm-adapters/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── transport.rs     # OpenAiCompatTransport (ureq-based HTTP + JSON envelope)
│   ├── quirks.rs         # Quirks struct (per-adapter behavior deltas)
│   └── ollama.rs         # OllamaAdapter: impl dowiz_kernel::ports::llm::LlmBackend
└── tests/
    └── ollama_roundtrip.rs
```

`Cargo.toml` (exact, matching the verified `ureq` line):

```toml
[package]
name = "llm-adapters"
version = "0.1.0"
edition = "2021"
description = "LlmBackend adapters (Ollama Tier-1, vLLM Tier-2, ManagedApi Tier-0). Kernel never imports this crate — compile firewall."

[dependencies]
dowiz-kernel = { path = "../kernel" }
ureq = { version = "2", default-features = false, features = ["tls", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`quirks.rs`:

```rust
pub struct Quirks {
    pub base_url: String,
    pub model_id_passthrough: bool,      // Ollama: true (":tag" kept verbatim, never stripped)
    pub embeddings_path: &'static str,   // "/v1/embeddings" (OpenAI-parity) or "/api/embed" (native)
    pub extra_options: serde_json::Value, // keep_alive, num_ctx, think=false — Ollama/native-only knobs
}
```

`ollama.rs` — `OllamaAdapter { transport: OpenAiCompatTransport, quirks: Quirks }`, `chat()` POSTs
`/v1/chat/completions` (verified live 2026-07-16: returns full OpenAI schema incl. `usage`,
`system_fingerprint:"fp_ollama"` — the transport must NOT key anything on that sentinel, per
`HARNESS-LLM-BACKEND-PLAN.md` §2.2 item 2); `embed()` POSTs `quirks.embeddings_path`; model routing
per `TaskClass`: `Code → "qwen2.5-coder:7b"`, `General → "llama3.1:8b"`, `Embedding → "nomic-embed-text"`
(all four names verified present via `ollama list` on this host, 2026-07-16).

### 2.3 Migration steps, in order

1. `kernel/src/ports/mod.rs` + `ports/llm.rs` (types only) → add `pub mod ports;` to `lib.rs`.
2. `cargo build -p dowiz-kernel` (or `cd kernel && cargo build`) — must succeed with **zero** new
   entries in `kernel/Cargo.lock` (types-only module, no deps added).
3. `llm-adapters/Cargo.toml` + the four `src/` files above.
4. `cargo build` inside `llm-adapters/` — pulls `ureq`/`serde`/`serde_json` **only into this crate's own
   lock file**, never `kernel/Cargo.lock`.
5. `llm-adapters/tests/ollama_roundtrip.rs` — a real (not mocked) call against `127.0.0.1:11434`.

### 2.4 Falsifiable done-check (unchanged from the plan, now with exact commands)

```
cd kernel && cargo build && cargo tree | grep -Ei "reqwest|ureq|llm-adapters"   # → empty
cd llm-adapters && cargo test ollama_chat_roundtrip                             # → passes, non-empty
                                                                                  #   content + populated usage.total_tokens
# then stop the daemon and re-run: systemctl stop ollama && cargo test ollama_chat_roundtrip
# → health() / chat() return Err(LlmError::Unavailable), not a panic, not a mock response
systemctl start ollama   # restore
```

---

## 3. Step (b) — exact-match sha3 response cache (Layer A)

**New file:** `llm-adapters/src/cache.rs` — `CachingBackend<S: dowiz_kernel::backup::BlockStore>`
wraps any `&dyn LlmBackend` (composition, not inheritance — `CachingBackend` itself implements
`LlmBackend` so it's a drop-in wrapper at any call site).

```rust
pub struct CachingBackend<B: LlmBackend, S: BlockStore> { inner: B, store: S, policy: CachePolicy }
pub enum CachePolicy { Exact, SemanticOk { tau: f32 }, NoCache }  // per-request, on ChatRequest
```

Key derivation reuses `dowiz_kernel::event_log::sha3_256` **directly** (already `pub`, zero new hashing
primitive) over a canonical-JSON encoding of `{model_id, messages, temperature, top_p, max_tokens, seed}`
with sorted keys (serde_json's `Map` is already insertion/sorted-key stable when built from a `BTreeMap`
— use `BTreeMap<String, Value>` for the canonical form, not `HashMap`, so key order is deterministic).
Cache **value** storage reuses `dowiz_kernel::backup::{BlockStore, MemStore}` verbatim — `MemStore::put`
is already idempotent (`backup.rs:62-70`, "storing a block whose id already exists is a no-op"), which
**is** the cache-hit semantics for free.

**Falsifiable done-check:** two identical `temperature=0` requests through `CachingBackend` — assert the
*inner* adapter's transport is hit exactly once (wrap `OllamaAdapter` in a call-counting test double for
this one test only); a third request with a changed `max_tokens` is a fresh miss.

---

## 4. Step (d), corrected — `TokenBucket`(F33)-bounded dispatch, WITHOUT tokio

> This section supersedes `HARNESS-LLM-BACKEND-PLAN.md` §4.2 for the dispatch mechanism. The
> `TokenBucket` primitive itself (capacity/refill_rate/monotonic-clock/atomic `try_acquire`) is
> **unchanged** — it's `BLUEPRINT-P11-compute-budget-cache.md` §4's spec, build it as-is in the kernel
> (zero-dep, `kernel/src/token_bucket.rs`, sibling to `spool.rs`). What changes is everything *around*
> it, because `llm-adapters` uses synchronous `ureq`, not `tokio`.

**New file:** `llm-adapters/src/dispatch.rs` — `Dispatcher` bounds concurrency with
`std::sync::mpsc` + a fixed-size worker-thread pool (`std::thread::spawn`, N workers = the per-backend
concurrency ceiling, e.g. `2` for Ollama matching its own `OLLAMA_NUM_PARALLEL` default per
`HARNESS-LLM-BACKEND-PLAN.md` §4.1 — the harness pool size should **not exceed** the backend's own
parallelism cap, or requests queue at Ollama's `OLLAMA_MAX_QUEUE` instead of the harness's own visible
queue). Each worker: `bucket.try_acquire(cost)` → on `false`, reply `Err(BudgetExceeded)` immediately
(degrade-closed, never queued-then-silently-downgraded) → on `true`, call the (blocking) adapter, send
the result back over a `mpsc::Sender` the caller holds. This achieves the plan's stated goal (bounded
concurrency, typed refusal, back-pressure visible at the harness boundary, not a raw Ollama 503) with
**zero tokio dependency** in `llm-adapters`, consistent with the DECART decision. If a future workload
genuinely needs async I/O concurrency (none does yet — this workload is compute/model-bound on Ollama's
side, not socket-bound), that is a *new* DECART decision at that time, not a default reversion to tokio.

**Falsifiable done-check:** the F33 concurrent property test (`BLUEPRINT-P11` §4's own falsifier) passes
unchanged; a test that exhausts the bucket via the thread-pool dispatcher gets `Err(BudgetExceeded)` on
the next `dispatch()` call, not a hang and not a silent fallback to `ManagedApiAdapter`.

---

## 5. Step (c) note — grounded, one open item flagged honestly

Step (c) (wiring `OllamaAdapter.embed()` into `evals.rs`'s deferred semantic gate) is grounded above
(§1's `MintLog`/`evals.rs` citations, re-verified live at `evals.rs:60,74,87`). The **one thing this
document does NOT yet pin down**, honestly: the exact call site inside `evals.rs`'s scoring path where
a semantic-gate check should be invoked alongside `MintLog::mint()` — that requires reading the rest of
`evals.rs` (the `MetamorphicGenerator` scoring loop, past the `spectral_similarity` MR shown at line 147
onward) at implementation time, not guessed here. Flagged rather than fabricated.

---

*Companion to: `HARNESS-LLM-BACKEND-PLAN.md` (the plan this executes), `DECART-llm-backend-integration.md`
(the two decisions this blueprint implements — Ollama Tier-1, `ureq` HTTP client), `BLUEPRINT-P11-compute-budget-cache.md`
§4 (F33 `TokenBucket`, unchanged), `kernel/src/backup.rs` / `event_log.rs` / `evals.rs` (reused primitives,
all citations re-verified live 2026-07-16). Planning only — no code written; implementation begins when
the operator says so, on `feat/harness-llm-backend`.*
