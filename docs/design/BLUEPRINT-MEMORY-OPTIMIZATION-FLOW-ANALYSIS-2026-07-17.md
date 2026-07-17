# BLUEPRINT — Phase 26: Memory Optimization & Flow Analysis — Raising the D_max Ceiling (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART, 2-question doubt
> audit, Anu/Ananke check. Style contract: plain prose, no metaphor; every load-bearing statement
> carries a `file:line` cite, a live-command ground, a web citation, or is tagged **(proposal)** /
> **(training-knowledge)**.
>
> **Operator ask (2026-07-17):** Phase 25 found that I/O-bound agent-dispatch concurrency is gated
> by memory, not CPU (`D_max = ⌊MEM_AGENT_BUDGET / MEM_PER_AGENT⌋`, default-capped at 16, while
> the wait/compute arithmetic alone would allow 400+). Research how to make memory usage itself
> maximally fast and optimized so the achievable ceiling rises: reduce `mem_per_agent`, improve
> throughput/locality codebase-wide, think in flow terms (bottlenecks, leaks, what can be quantized
> or cached), and evaluate honestly whether the existing spectral/graph machinery and tensor
> techniques have genuine reuse potential for memory management.
>
> **Provenance note:** this session's WebSearch budget was already exhausted (200/200) before this
> task began — the identical condition Phase 25 recorded. All web sources below were fetched live
> via direct URL this session (marked *fetched*); canonical literature that resisted direct fetch
> (USENIX and HAL returned 403) is cited from training knowledge and flagged
> **(training-knowledge)** per the style contract. No design-load-bearing *number* rests on an
> unfetched source: the quantization numbers, allocator history, and glibc arena defaults were all
> fetched; the flagged items (Tofte–Talpin, Jégou PQ, TT-Rec) support reject/defer verdicts, where
> being wrong about the citation detail cannot flip the decision.

---

## 0. Executive answer (the rest of the document is the derivation)

1. **The dominant `mem_per_agent` term is not in this repo's Rust code — say so first.** A
   D-class dispatched lane is a CLI agent process (Node-based harness + context); its RSS
   (estimated 0.5–1 GB, P25 §3.4, still unmeasured — P25 W1) dwarfs anything the kernel or
   llm-adapters allocate. No Rust allocator choice, arena, or quantization in this repo shrinks
   that term. What this repo CAN do: (a) make the memory budget **measured and enforced** instead
   of assumed — which is what actually lets D_max rise from the conservative 16 to a defensible
   24+; (b) guarantee its own native surfaces add **bounded** memory on top (today two of them are
   unbounded, §1.4–§1.5); (c) stop paying for memory it never uses (one dense-matrix duplicate of
   an existing sparse structure, §1.6).
2. **Allocator: KEEP the system allocator (glibc malloc).** No `#[global_allocator]` exists
   anywhere in the repo today (live grep, §1.1) and none should be added now. Rust itself removed
   jemalloc-as-default in 1.32 for exactly this workload-honesty reason (*fetched*, §2.1). The
   kernel's hot paths are allocation-free by existing contract (P24 §3.3); the processes are
   short-lived CLIs or drainer threads. One zero-code mitigation is named if measurement ever
   shows arena bloat: `MALLOC_ARENA_MAX` (§2.1). One measured trigger is named for revisiting
   mimalloc (§4 row 1).
3. **Arena/bump allocation: REJECT — both the crate and the hand-roll.** The per-dispatch working
   set is textbook phase-oriented region allocation in shape, but the flow arithmetic kills it:
   a dispatch performs a handful of allocations then waits tens of seconds on the network. Saving
   nanoseconds per allocation against a 10⁷× larger wait is not a lever (§2.2). The stdlib-native
   arena — a reused `Vec` + `clear()` retaining capacity — already exists as the pattern for any
   future hot loop that profiling actually flags.
4. **`MemoryBudget` primitive: ADOPT (build).** `kernel/src/memory_budget.rs` as `TokenBucket`'s
   sibling — reserve/release bytes, no time-refill (bytes do not refill with time; it is a
   counting semaphore over bytes, not a rate limiter — the one semantic difference from
   `token_bucket.rs`, §3.1). It is the mechanism that turns P25's static `D_max = 16` into
   `try_reserve(mem_per_agent)` per admission with a measured, live number (§3.5).
5. **Cache bounding + eviction: ADOPT plain byte-budgeted LRU; DEFER W-TinyLFU with a named
   trigger; REJECT ARC.** The exact-match LLM cache is **unbounded today** (`MemStore` HashMap,
   no eviction path at all — §1.4). At its actual scale (hundreds-to-thousands of entries, one
   process) the published hit-rate advantage of W-TinyLFU over LRU (*fetched*, §2.4) is real but
   immaterial; the miss cost is one LLM call on a truly-new prompt. Bound it first; sophisticate
   eviction only at the named size trigger (§4 row 4).
6. **Embedding quantization: DEFER-WITH-TRIGGER, quantified.** Layer B (semantic cache) is
   designed but unbuilt; at its designed scale (a few hundred 768-dim f32 vectors ≈ 1–2 MB) int8
   quantization saves under 2 MB against a 16 GB budget. The fetched numbers (int8 = 4× smaller
   at ~99% retrieval retention; binary = 32× at ~96% with rescoring, §2.3) go into the trigger:
   adopt int8 scalar quantization when the Layer-B index would exceed ~100 MB. The money.rs
   parallel is noted where it belongs (§2.3): money is integer-exact by red line
   (`money.rs:1-10`); embeddings are advisory-only by the existing `CachePolicy` type — the same
   asymmetry that makes quantization acceptable on one and forbidden on the other.
7. **Graph-based cache scoring (PPR/centrality over an access graph): HONEST REJECT.** It is a
   research-prototype technique with no production adoption against ARC/W-TinyLFU-class policies
   (§2.5), and this codebase's cache entries have no reference graph between them — building one
   to justify existing machinery would be forcing the fit the task brief warned against. The
   **genuine** reuse of the existing machinery is the *sparsity*, not the *spectrum*:
   `retrieval/ppr.rs` stores its transition matrix dense (`Vec<Vec<f64>>`, `ppr.rs:18-22`) while
   `csr.rs` already implements the same deterministic PPR sparse — consolidating removes a real
   O(n²) memory duplicate using machinery that already exists (§3.4).
8. **Tensor decomposition (Tucker/CP/TT): REJECT as premature, plainly.** dowiz stores no
   embedding matrix today (grep-verified, §1.7). The technique targets billion-entry embedding
   tables (§2.6). The one "tensor" technique that genuinely applies here is the sparse format —
   and it is already shipped (`csr.rs:39-54`, the one-contiguous-buffer invariant). "Tensoring"
   beyond that is, for this codebase at this scale, a buzzword.

---

## 1. Ground truth (live-verified this session)

### 1.1 Allocator configuration — none exists; the default is glibc malloc

```
grep -rn "global_allocator" /root/dowiz --include="*.rs" --include="*.toml"  → no matches
```

No crate in the repo overrides the allocator (checked every `Cargo.toml`: kernel, engine,
llm-adapters, wasm, agent-governance-wasm, tools/*). Rust's `System` allocator "is based on
`malloc` on Unix platforms" and is what binaries use absent an override (*fetched*:
[std::alloc::System](https://doc.rust-lang.org/std/alloc/struct.System.html)). So the live
baseline for every native process in this repo is **glibc malloc** — this is the grounded starting
point, not an assumption. `kernel/Cargo.toml:86-88` sets `opt-level = 3, lto = true` for release;
no allocator-adjacent settings.

### 1.2 The memory budget this feeds (from P25, consumed not re-derived)

P25 §3.4: 30 GB total, 27 GB available (live `free -g`); `MEM_AGENT_BUDGET = 16 GB` after Ollama
residency (~6 GB, three models resident per `HARNESS-LLM-BACKEND.md` §1.2) + C-class/page-cache
headroom; `MEM_PER_AGENT ≈ 0.5–1 GB` **(P25's own flag: estimate, unmeasured — its W1)**;
`D_max = 16` default, "raiseable to 24+ once MEM_PER_AGENT is measured and memory PSI stays clean
at 16." The measurement surface is P24 §3.4: `/proc/self/status` VmRSS + PSI memory `some avg10` +
`total` — already committed, this blueprint adds no new gauge mechanism.

### 1.3 Where LLM-path memory actually flows (llm-adapters, read this session)

- `CachingBackend` (`llm-adapters/src/cache.rs:31-34`): wraps any backend with an
  `Arc<Mutex<S: BlockStore>>` store; key = sha3-256 of the canonical request
  (`cache.rs:57-81`); value = encoded `ChatResponse` bytes.
- `Dispatcher` (`llm-adapters/src/dispatch.rs:75-89`): fresh `std::thread` per job, bounded by
  `TokenBucket::try_acquire`; each thread's stack is virtual until touched — at ≤16 in-flight
  jobs this is noise, noted and left alone.
- Clone/allocation density in the dispatch path is low and network-dominated: `cache_key` builds
  a `BTreeMap` + JSON per call (`cache.rs:61-80`) — a few µs against a multi-second LLM
  round-trip. Measured clone counts (live grep this session): top of the whole repo is
  `kernel/src/causal.rs` at 14 `.clone()` per 2330 lines; `llm-adapters` files hold 0–4 each.
  **Finding: there is no clone-hotspot campaign to run.** The proxy the task suggested was
  checked and came back clean.

### 1.4 Leak-shaped finding #1 — the exact-match cache is unbounded

`MemStore` is a bare `HashMap<Hash, Vec<u8>>` (`kernel/src/backup.rs:60-93`). `put` is idempotent
(dedup) but **nothing ever evicts**; `CachingBackend` inserts every distinct response forever
(`cache.rs:100-105`). At today's usage this is MBs, not GBs — but it is the exact shape DHAT's
manual names: "blocks allocated by the point just accumulate, and are freed only at the end of
the run" (*fetched*: [Valgrind DHAT manual](https://valgrind.org/docs/manual/dh-manual.html)).
A process that runs 16+ concurrent lanes for hours multiplies the accumulation. `MemStore` even
already ships the measurement hook — `stored_bytes()` (`backup.rs:71-76`) — currently used by
nothing on the cache path.

### 1.5 Leak-shaped finding #2 — `FileBlockStore` eagerly mirrors the whole disk store into RAM

`FileBlockStore::open` walks `blocks/` and reads **every block's bytes** into an in-memory
`HashMap` cache (`backup.rs:106-131` doc + `load()` at `backup.rs:135-150`), so the process RSS of
anything that opens a backup store equals the full store size, forever growing with the store.
The comment is explicit that this exists to satisfy the trait's borrowed-slice `get` contract; the
trait *also* already has `get_owned` (`backup.rs:46-50`), which re-reads and re-hashes from disk.
The fix is structural and small (§3.3): keep an **index** (ids only), not a **mirror** (bytes).

### 1.6 Duplicate-representation finding — dense PPR beside sparse PPR

`kernel/src/retrieval/ppr.rs:18-22`: "Row-stochastic transition matrix W (n·n), stored as dense
`Vec<Vec<f64>>`" — O(n²)·8 B plus per-row `Vec` headers (the exact `Vec<Vec>` pointer-chasing
`csr.rs:28-29` names as the anti-pattern its one-buffer layout exists to avoid).
`kernel/src/csr.rs:1-54`: the same deterministic fixed-K, fixed-order PPR over CSR storage
(O(nnz)). Two authorities for one computation, one of them paying O(n²) memory for a sparse
graph. (This is the retrieval-side cousin of the "3-eigensolver dual-authority" issue already in
standing memory.) Bitwise-parity note for the consolidation: both iterate source-node outer /
target inner, and skipping structural zeros is exact under f64 (adding `+0.0` to a non-negative
accumulator is an identity; all masses and weights here are ≥ 0 products of ≥ 0 inputs) — so
CSR-backed evaluation can reproduce the dense result bit-for-bit; the parity test in §7 M4 is the
falsifier, not this argument.

### 1.7 Embedding-storage reality — Layer B is designed, not built; no embedding matrix exists

`HARNESS-LLM-BACKEND.md` §3.3: Layer B = embed prompt via `nomic-embed-text`, search "a small
in-memory index of recent cached prompt-embeddings," cosine ≥ τ, **advisory-only** — and its own
DECART already ruled "brute-force cosine over a few hundred embeddings needs no dep." Grep over
`kernel/src/retrieval/` finds BM25/trigram/PPR structures and no stored `Vec<f32>` embedding
matrix. `EmbedResponse { embedding: Vec<f32> }` (`ports/llm.rs`) is the only embedding value type.
Sizes for the quantization math: nomic-embed-text = 768 dims → 3,072 B/vector f32;
qwen3-embedding:0.6b = 1,024 dims → 4,096 B. "A few hundred" vectors ≈ 1–2 MB.

### 1.8 Existing primitives this design extends (not reinvents)

- `TokenBucket` (`kernel/src/token_bucket.rs:26-79`): `Mutex<Inner>`, monotonic-clock refill,
  degrade-closed `try_acquire` — the structural template for `MemoryBudget` (§3.1).
- P24 gauge surface (P24 §3.4): VmRSS + PSI memory — the measurement source for
  `mem_per_agent` (§3.5). This blueprint adds **no third monitoring mechanism** (P25's own rule,
  inherited).
- P25 admission predicate (`admit_dispatch()`, P25 §3.4) — the consumer of §3.5's output.

---

## 2. Research — which techniques are real levers here, and which are not

### 2.1 Allocator choice: glibc malloc vs jemalloc vs mimalloc

- **Rust's own history is the decisive precedent.** Rust 1.32 (2019) removed jemalloc as the
  default: jemalloc "usually" won but not consistently, added ~300 KB to every binary, and had
  accumulated platform/maintenance issues; the system allocator became the default, with opt-in
  via `#[global_allocator]` for programs that measure a win (*fetched*:
  [Rust 1.32 announcement](https://blog.rust-lang.org/2019/01/17/Rust-1.32.0/)). The burden of
  proof sits on *adopting* an allocator, not on keeping the default — which matches this repo's
  zero-dep convention exactly.
- **mimalloc's published numbers** (*fetched*:
  [microsoft/mimalloc README](https://github.com/microsoft/mimalloc)): marginally faster than
  tcmalloc/jemalloc on `cfrac` (many small single-threaded allocations); "quite a bit faster" on
  `larsonN` (cross-thread alloc/free); large margins on `xmalloc-testN` (asymmetric
  alloc-thread/free-thread). Its free-list sharding claim — "things allocated close in time get
  allocated close in memory" — is the locality property relevant to a request-scoped workload.
  Caveats are the vendor's own: "interpret these results with care since some benchmarks test
  synthetic or uncommon situations," and they do not measure long-running worst-case latency.
  **These are the right benchmarks to rerun locally if a trigger fires, not a reason to adopt
  today** — none of this repo's processes has an allocation-rate profile within orders of
  magnitude of `larson`-class churn (§1.3), and the kernel hot paths are allocation-free by
  contract (P24 §3.3: "the hot path never touches a file, a lock, or an allocator").
- **The one glibc-specific hazard worth naming for the concurrent future:** glibc malloc creates
  per-thread arenas to cut lock contention; the arena hard limit defaults to a multiple of core
  count (`M_ARENA_TEST` default 8 on 64-bit before the limit is computed), and "the more arenas
  you have, the lower the per-thread contention, but the higher the memory usage" (*fetched*:
  [mallopt(3)](https://man7.org/linux/man-pages/man3/mallopt.3.html)). A future long-lived,
  many-threaded native process (GapWire drainer + dispatcher pool) could see RSS inflated by
  unused arena slack. The mitigation is **zero code**: `MALLOC_ARENA_MAX=2` in that process's
  environment — an operational knob, applied only if P24's VmRSS gauge actually shows the bloat.

### 2.2 Region/arena allocation — right shape, wrong magnitude here

- The pattern is real and well-established: region-based memory management allocates
  request-scoped objects into a region freed in one operation (Tofte & Talpin, "Region-Based
  Memory Management," Information and Computation 1997 **(training-knowledge — HAL/USENIX fetches
  403'd this session; the verdict below does not depend on the citation's details)**). Rust's
  `bumpalo` implements it: allocation is a pointer bump; "mass deallocation [is] _extremely_
  fast, but allocated objects' Drop implementations are not invoked," and `Bump` is `!Sync`
  (*fetched*: [bumpalo docs](https://docs.rs/bumpalo/latest/bumpalo/)).
- **Why it is rejected here anyway:** an agent-dispatch turn allocates a request string, a
  response buffer, and some JSON scaffolding — tens of allocations — then blocks for tens of
  seconds on the network (P25 §2.1's W/C ≥ 50 arithmetic). Arena allocation converts ~µs of
  malloc work into ~ns; the flow spends 10⁷× longer waiting. There is no latency, fragmentation,
  or throughput problem for it to solve at this allocation rate. Adopting `bumpalo` would add a
  dependency plus a no-Drop / `!Sync` hazard class for zero measurable win; hand-rolling an arena
  would be hand-rolling the same zero. This is NOT the class of appropriate hand-rolling the
  repo's crypto/math primitives represent — those exist because the alternative was a dependency
  with a real job to do. The honest stdlib answer already in the codebase's idiom: **reuse a
  `Vec` scratch buffer and `clear()` it** — capacity is retained, allocation happens once, and
  it is exactly what a hot loop should do *if profiling ever flags one* (DHAT is the named tool,
  §2.7).

### 2.3 Embedding quantization — real numbers, applied to the real (tiny) scale

- Fetched benchmark (*fetched*:
  [Hugging Face, "Binary and Scalar Embedding Quantization"](https://huggingface.co/blog/embedding-quantization),
  measured on mxbai-embed-large-v1, MTEB Retrieval, NDCG@10): **int8 scalar quantization = 4×
  smaller, ~99.3% performance retention** (52.79 vs 54.39 float32); **binary = 32× smaller,
  ~92.5% without rescoring, ~96% with rescoring**; int8 with a rescore multiplier of 4–5 reaches
  ~99%. Speedups: up to ~24.8× (binary), ~3.7× (int8) on retrieval ops. Flag: model-specific
  (mxbai-large, 1024-dim); nomic/qwen3 numbers would need local re-measurement, but the *order*
  (int8 ≈ free accuracy-wise, binary needs rescoring) is consistent across the benchmark's 15
  datasets.
- Product quantization (Jégou, Douze, Schmid, "Product Quantization for Nearest Neighbor
  Search," IEEE TPAMI 2011 **(training-knowledge, fetch 403'd)**) — subspace decomposition +
  per-subspace codebooks; the right tool at million-to-billion vector ANN scale, with codebook
  training machinery that is pure overhead at thousands of vectors.
- **Applied to §1.7's reality:** Layer B at designed scale is 1–2 MB of f32. int8 would save
  ~1.5 MB against a 16 GB budget — a 0.01% effect. The correct decision is a **trigger, not an
  adoption**: quantize int8 when the Layer-B index would exceed ~100 MB (≈25–35k vectors at
  1024-dim), i.e. when it stops being "a small in-memory index" per its own design doc. PQ/binary
  reserved for a scale dowiz does not have and may never have. — And the precision-red-line
  parallel, stated once: `money.rs:1` ("RED LINE: zero float arithmetic on monetary values") is
  the codebase's proof it already treats precision-vs-representation as a *per-domain* decision;
  Layer B is structurally advisory-only (`CachePolicy` prevents gate-critical callers from
  touching it, `cache.rs:6-7`), which is precisely why lossy quantization is admissible there
  and nowhere gate-critical.

### 2.4 Cache eviction policy — the literature, then the honest scale check

- **ARC** (Megiddo & Modha, FAST '03 **(training-knowledge for the paper text — USENIX fetch
  403'd; mechanism and adoption *fetched* via
  [Wikipedia: Adaptive replacement cache](https://en.wikipedia.org/wiki/Adaptive_replacement_cache))**:
  self-tuning recency (T1) + frequency (T2) lists with ghost lists (B1/B2); outperforms LRU;
  deployed in IBM DS-series controllers, ZFS/OpenZFS, VMware vSAN. Also: IBM patented it (2006);
  PostgreSQL adopted it in 8.0.0 and "quickly replaced it … citing concerns over an IBM patent."
- **W-TinyLFU** (Einziger, Friedman, Manes, "TinyLFU: A Highly Efficient Cache Admission
  Policy," *fetched*: [arXiv:1512.00727](https://arxiv.org/abs/1512.00727)): the key idea is
  **admission** filtering (compare the incoming item's approximate frequency, from a compact
  Bloom-filter-family sketch, against the eviction candidate's), not smarter eviction; claims
  "equal or better hit-ratios than other state of the art replacement policies … on all traces."
  The Caffeine library's production evaluation across Wikipedia/database/search/OLTP/loop traces
  concludes "Window TinyLfu provides a near optimal hit rate and is competitive with ARC and
  LIRS" (*fetched*:
  [Caffeine Efficiency wiki](https://github.com/ben-manes/caffeine/wiki/Efficiency)).
- **The honest scale check:** those results are measured on storage/CDN/database traces with
  10⁵–10⁷ objects under hard capacity pressure. This codebase's LLM response cache holds
  10²–10³ entries in one process; today it holds *everything* (no eviction at all, §1.4), so its
  current hit-rate is by definition optimal and the only problem is boundedness. After bounding,
  the policy delta between LRU and W-TinyLFU at 10³ entries with a strongly recurrence-skewed key
  stream (repeated near-identical dispatch prompts hash to *identical* exact keys — the recurring
  mass is concentrated, not long-tailed) is noise, and a miss on a genuinely-new prompt was never
  cacheable anyway. **Verdict: plain LRU with byte accounting now; W-TinyLFU's count-min sketch
  is an honest hand-rollable upgrade (it is a small, well-specified primitive — the same class as
  the repo's hand-rolled BM25/trigram) IF the trigger in §4 row 4 fires. ARC rejected**: its
  adaptivity solves scan-vs-loop workload shifts this cache does not have, and the patent history
  is a named external hazard with zero offsetting benefit here.

### 2.5 Graph-based cache scoring — research status, honestly

The question posed: is "PageRank/centrality over a reference/access graph decides what stays
cached" a credentialed production technique? **Finding: no.** The production lineage of cache
policy is recency/frequency structures — ARC in ZFS/IBM/VMware (*fetched*, §2.4), W-TinyLFU in
Caffeine (and via it a large fraction of the JVM ecosystem) (*fetched*, §2.4) — none score
entries by graph centrality. Graph-scored caching appears in information-centric-networking and
CDN placement research prototypes **(training-knowledge — flagged; no fetchable survey this
session)**, where a real object-reference graph exists (web pages linking pages, named-data
hierarchies) and cache placement is a network-wide optimization. Neither precondition holds here:
dowiz's cache entries are independent LLM responses with **no reference edges between them**
(`cache.rs` keys are opaque sha3 hashes of full requests), and the "cluster" is one process on
one host. Constructing an artificial access-adjacency graph so that `csr.rs`+PPR could score it
would add O(nnz) state and O(K·nnz) scoring work per maintenance pass to approximate what an LRU
list's order already encodes in O(1) per touch. **REJECT — this is the over-engineering branch
the task brief predicted, and the machinery-already-exists argument does not survive the
workload check.** The genuine reuse of the graph machinery is §3.4 (CSR replacing a dense matrix
inside the retrieval layer itself — same technique, real target).

### 2.6 Tensor techniques — one real (already shipped), the rest premature

- **Sparse formats (COO/CSR/CSC):** genuinely applicable and **already in production in this
  codebase** — `csr.rs:39-54` stores only nonzeros in one contiguous buffer; this *is* the
  memory-compression technique the "tensoring" question asks about, applied to the 2-D case.
  The extension with a real target is §1.6/§3.4 (the dense PPR duplicate). No other
  multi-dimensional sparse structure exists in the repo to convert (checked: `mat.rs` is dense
  by design for the eigensolver; spectral fields are dense small-n).
- **Low-rank decomposition (Tucker/CP/tensor-train):** the production success stories compress
  *huge learned embedding tables* — e.g. TT-Rec compresses deep-learning recommendation-model
  embedding tables via tensor-train factorization **(training-knowledge, flagged — MLSys 2021;
  cited only to locate the technique's home turf)**. Preconditions: an embedding matrix with
  10⁷–10⁹ entries and tolerance for approximate reconstruction. dowiz has **no stored embedding
  matrix at all** (§1.7). **REJECT as premature**; if a large embedding store ever exists, scalar
  quantization (§2.3) precedes decomposition on both simplicity and benchmark evidence.

### 2.7 Finding real hotspots instead of guessing — the profiling discipline

- **DHAT** (*fetched*: [Valgrind DHAT manual](https://valgrind.org/docs/manual/dh-manual.html))
  measures per-allocation-site block counts, bytes, **lifetimes**, and access counts — exactly
  the instrument that separates "short-lived allocations worth an arena" from "accumulating
  blocks that are a leak," and it has a copy-profiling mode for finding excessive `memcpy`. This
  is the tool the arena/clone questions should be answered with *before* any allocator work, run
  against `cargo bench`-driven kernel workloads and an `lm` CLI query pass.
- The measured clone-density pass this session (§1.3) is the cheap proxy version and it came
  back clean; process-level truth stays with P24's VmRSS/PSI gauges (no new mechanism).
- `mmap` for large read-only data: the only large read-only native files are P24's RRD history
  tiers (fixed 4.5 MB, pwrite-addressed — P24 §3.5) and `FileBlockStore` blocks (fixed by §3.3
  without mmap). **No mmap adoption needed**; named so its absence is a decision.

---

## 3. Design — the minimal native plan

### 3.1 `kernel/src/memory_budget.rs` — `MemoryBudget`, TokenBucket's sibling (proposal)

Same structural pattern as `token_bucket.rs:26-79` (plain `std`, `Mutex<Inner>`, degrade-closed),
different semantics — reserve/release, **no time-based refill**:

```rust
pub struct MemoryBudget {
    capacity_bytes: u64,          // e.g. MEM_AGENT_BUDGET = 16 GiB (policy-as-data)
    inner: Mutex<Inner>,          // Inner { reserved_bytes: u64 }
}
impl MemoryBudget {
    pub fn try_reserve(&self, bytes: u64) -> bool;   // grant iff reserved + bytes <= capacity
    pub fn release(&self, bytes: u64);               // saturating decrement (never underflows)
    pub fn reserved(&self) -> u64;                   // telemetry/tests
}
```

- **Verified-by-math property (the falsifier):** at every point in any interleaving,
  `reserved_bytes ≤ capacity_bytes`, and Σ(successful reserves) − Σ(releases) = `reserved()`.
  Degrade-closed: an over-budget `try_reserve` returns `false`, never a partial grant — the
  caller defers (P25's backoff-with-jitter, §3.6, unchanged).
- **Deliberately NOT a `TokenBucket` parameterization:** a refill rate on bytes would assert that
  memory returns with time, which is false — memory returns when a holder releases it. Folding
  both into one generic would corrupt the simpler invariant each one proves. Two small structs,
  one pattern, two truths.
- **Callers:** (1) the P25 admission function — see §3.5; (2) the bounded cache store — §3.2;
  (3) any future resident-agent plane (P21) sizing its own working set.

### 3.2 Byte-bounded cache store with LRU eviction (proposal)

A `BoundedStore` implementing the existing `BlockStore` trait (`backup.rs:39-57` — unchanged), so
`CachingBackend` needs **zero changes** (it is generic over `S: BlockStore`, `cache.rs:31`):

- Wraps `MemStore` + an LRU order (a `VecDeque<Hash>` or intrusive list; hand-rolled, `std` only)
  + `budget_bytes` (its own cap or a shared `MemoryBudget` handle).
- `put`: insert, then evict LRU entries while `stored_bytes() > budget_bytes`
  (`MemStore::stored_bytes`, `backup.rs:71-76`, finally earns its keep). `get`: refresh recency.
- Policy: plain LRU per §2.4's verdict. The eviction upgrade path (count-min-sketch admission,
  W-TinyLFU-style) is a named deferred item behind the §4 row 4 trigger — the trait boundary
  means it would be a drop-in store swap, not a cache rewrite.
- Semantic note: evicting a cache entry is always safe here — the cache is a cost optimization
  over an idempotent upstream call (`cache.rs:9-11`); correctness never depends on a hit.

### 3.3 `FileBlockStore`: index, don't mirror (proposal)

Replace the eager whole-store byte cache (`backup.rs:106-150`) with an id **index**
(`HashSet<Hash>` populated from filenames only — no `fs::read` of block bytes at `open`):

- `len`/`is_empty`/dedup-check run off the index; `get_owned` already re-reads + re-hashes from
  disk (`backup.rs:46-50` default + the store's own integrity-checking override) — unchanged.
- The borrowed-slice `get(&self) -> Option<&[u8]>` cannot be served without a resident copy; the
  honest contract change: `FileBlockStore::get` returns `None` (disk-backed callers must use
  `get_owned` — which is what restore paths should be doing anyway), documented at the trait.
  If any live caller depends on borrowed `get` from a disk store, the fallback design is a
  small bounded LRU of recently-got blocks (reusing §3.2's structure) instead of the unbounded
  mirror — either way the RSS ≈ store-size behavior ends.
- Effect: opening a backup store stops costing its full size in RSS (§1.5) — the largest single
  bounded-vs-unbounded fix in the repo's native surface.

### 3.4 Consolidate dense PPR onto CSR (proposal — the genuine spectral-machinery reuse)

`retrieval/ppr.rs` keeps its public API (`Ppr::new(w: Vec<Vec<f64>>)`, `rank(seed, alpha, k)`) but
internally converts W to `Csr` once at construction (`Csr::from_edges`, `csr.rs:79-115`) and
delegates the iteration to the CSR left-product (`csr.rs` `spmv`, whose orientation is exactly the
PPR step — `csr.rs:31-34`). Memory: O(n²) → O(nnz) for the retained matrix. At today's corpus
(hundreds of nodes) this is single-digit MB — stated honestly as a *small* win whose real value is
removing a dual authority (§1.6) using machinery that already exists, i.e. the legitimate version
of the reuse the operator asked about. Determinism is the gate: M4's done-check (§7) requires
bitwise-identical output on the existing fixtures (the §1.6 argument says it will hold; the test
decides). If bitwise parity fails on any fixture, the consolidation is **abandoned, not fudged** —
determinism outranks the MBs (`ppr.rs:10-16` is a determinism proof, not a suggestion).

### 3.5 How this updates P25's D_max — measurement in, static default out

The formula is unchanged (`D_max = min(⌊MEM_AGENT_BUDGET / MEM_PER_AGENT⌋, API_CONCURRENCY,
WORKFLOW_CAP × N_WORKFLOWS)`, P25 §3.4). What changes is how its memory term is evaluated:

1. **Measure:** P25 W1 (VmRSS of ≥3 live lanes) seeds `MEM_PER_AGENT`; thereafter the
   orchestrator maintains an EWMA of completed-lane peak VmRSS from P24's gauge surface
   (**no new mechanism** — the gauges exist; this is one subtraction and one multiply per lane
   exit).
2. **Enforce:** admission calls `budget.try_reserve(mem_per_agent_ewma)` per D-class lane and
   `release` on lane exit. The static `inflight_agents < D_MAX (= 16)` check becomes the
   *secondary* bound; the byte reservation is primary. PSI-memory stays in the predicate
   unchanged as the ground-truth backstop (reservation math can drift; reclaim pressure cannot).
3. **Consequence:** if measured `MEM_PER_AGENT` ≈ 0.5 GB, the same 16 GiB budget admits ~32 lanes
   — P25's "raiseable to 24+" claim gets its mechanism, with PSI + hysteresis (P25 §3.6)
   guarding the ramp. If measurement says ~1 GB, D_max stays ~16 and the honest answer to "more
   concurrency" is a bigger host, not a cleverer allocator — this blueprint makes that answer
   *provable* either way.

A short append-only note pointing here is added to the P25 blueprint (done with this blueprint's
registration; P25's own numbers are not rewritten — its §3.4 already flags the estimate and names
W1).

### 3.6 Explicitly not adopted (each a decision, not an omission)

| Technique | Verdict | One-line reason (derived above) |
|---|---|---|
| jemalloc/mimalloc `#[global_allocator]` | Not now; trigger in §4 row 1 | No allocation-rate problem exists to solve (§1.3, §2.1); Rust 1.32 precedent |
| `bumpalo` / hand-rolled arena | REJECT | ns saved vs 10-second network waits (§2.2); `Vec`+`clear()` is the native fallback |
| W-TinyLFU / ARC now | DEFER / REJECT | 10³-entry single-process cache; LRU+bytes suffices (§2.4); ARC patent history |
| PPR/centrality cache scoring | REJECT | No production lineage; no reference graph exists between entries (§2.5) |
| Tucker/CP/TT decomposition | REJECT (premature) | No embedding matrix exists to compress (§1.7, §2.6) |
| int8/binary embedding quantization now | DEFER with trigger | Layer B unbuilt; 1–2 MB at designed scale (§2.3) |
| mmap adoption | Not needed | Only large read-only files are already fixed-size/pwrite (§2.7) |
| io_uring, NUMA work | Out of scope | P25 §2.4–2.5 already ruled; nothing memory-side changes that |

---

## 4. DECART — every concrete choice

| # | Candidate | Native fit | Falsifiable correctness | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | **Allocator: keep system (glibc) vs adopt mimalloc/jemalloc** | keep = zero code (live grep: no override exists, §1.1) | keep is trivially correct; adoption would need the mimalloc bench suite rerun on THIS workload | adoption: new dep + a hazard class (allocator bugs are process-global) | mimalloc/jemalloc = external crates (crates.io egress constraints recorded in P15 §9) | adoption reversible (delete 3 lines) | **KEEP system allocator.** Trigger to revisit, falsifiable: a DHAT/heaptrack profile of a long-lived native process showing >5% cycles in malloc/free OR VmRSS > 1.3× live-bytes (fragmentation/arena bloat) — then bench mimalloc first (its concurrent-workload numbers, §2.1), `MALLOC_ARENA_MAX=2` as the zero-code intermediate |
| 2 | **Arena (bumpalo / hand-rolled) vs `Vec` scratch reuse** | `Vec` reuse = stdlib, already idiomatic here | arena win must show in a profile; none exists (§1.3) | bumpalo: dep + `!Sync` + no-Drop hazards (*fetched*, §2.2) | bumpalo external | — | **REJECT arena, both forms; `Vec`+`clear()` is the named pattern** for any future profiled hot loop |
| 3 | **`MemoryBudget` as new sibling struct vs parameterizing `TokenBucket` vs no primitive** | sibling = pure std, mirrors proven `token_bucket.rs` shape | invariant is a table-driven unit test + a concurrent stress test (§3.1) | ~100 LOC | zero deps | trivial (delete module) | **ADOPT (build) as sibling.** Parameterizing TokenBucket REJECTED: time-refill semantics are false for bytes (§3.1); no-primitive REJECTED: leaves D_max static and caches unbounded |
| 4 | **Cache bound: LRU+bytes vs W-TinyLFU vs ARC vs stay unbounded** | LRU+bytes = hand-rolled std over existing `BlockStore` trait | eviction keeps `stored_bytes ≤ budget` (unit test); hit-rate regression measurable via existing call-counting test double (`cache.rs:182-220`) | ~150 LOC | zero deps | store swap behind trait | **ADOPT LRU+bytes.** W-TinyLFU DEFER — trigger: cache >10⁴ entries AND measured hit-rate < 0.9× unbounded baseline; then hand-roll count-min sketch (repo-consistent). ARC REJECT (patent history + unneeded adaptivity, §2.4). Unbounded REJECT (it is the leak shape, §1.4) |
| 5 | **Layer-B quantization: int8 now vs trigger vs never** | int8 SQ = trivial native code when needed (scale+clamp) | retention re-measurable locally on the 12-query oracle when adopted | negligible | zero deps | lossless copy kept until proven | **DEFER with named trigger** (index >100 MB); numbers already fetched (4×/~99%, §2.3) so the trigger decision is pre-armed. Binary/PQ reserved beyond that; "never" rejected because the trigger is plausible if living-memory embeddings ever materialize |
| 6 | **Graph/PPR cache scoring** | machinery exists (csr.rs) but inputs don't (no entry graph) | would need an invented graph — unfalsifiable benefit | O(nnz) state + O(K·nnz) per pass vs O(1) LRU | zero | — | **REJECT** (§2.5). The reuse-shaped item that survives scrutiny is #7 |
| 7 | **ppr.rs dense → CSR delegation** | csr.rs already implements the same fixed-order PPR (§1.6) | bitwise parity on existing fixtures — hard falsifier; abandon on any mismatch | small refactor | zero | revert = keep dense | **ADOPT (cheap)** — real O(n²)→O(nnz), removes a dual authority; honestly small today and labeled so |
| 8 | **FileBlockStore index-not-mirror** | std-only change inside one struct | RSS after `open` on a 100 MB store < 10 MB (measured via VmRSS); round-trip restore tests stay green | moderate (contract note on `get`) | zero | revert = restore mirror | **ADOPT** — the largest single unbounded-RSS fix in the native surface (§1.5) |

**Mandatory probe (strongest honest argument against the whole plan):** every ADOPT here bounds
or measures memory; none *reduces* the dominant `mem_per_agent` term (the agent process itself,
§0.1) — so the operator could object that the plan doesn't do the thing asked. The answer is
§3.5's consequence line: the ask was "raise the achievable concurrency ceiling"; the ceiling is
`⌊budget/per-agent⌋`, and at current knowledge *both numbers are estimates*. Measurement +
enforcement is the only honest way that ceiling rises (16 → ~32 if the 0.5 GB estimate holds), and
bounded native surfaces are what keep it risen under load. A speculative allocator/quantization
campaign would have produced impressive-sounding diffs and zero D_max movement.

## 5. 2-question doubt audit

**Q1 — least confident about (concrete):**
1. The claim that skipping structural zeros preserves bitwise f64 parity (§1.6) covers +0.0
   accumulation but assumes no negative weights ever enter W; `Csr::from_edges` doesn't forbid
   negative edge weights. M4's parity test is the decider, and the abandon-on-mismatch rule is
   stated — but the *argument* is one signed-zero edge case away from wrong, so it is flagged.
2. `FileBlockStore::get → None` (§3.3) is a behavioral contract change; a caller relying on
   borrowed `get` against a disk store would silently start missing. Mitigation named (bounded
   LRU fallback), but the caller audit happens at build time, not in this document.
3. The 100 MB Layer-B quantization trigger and the >10⁴-entry eviction-upgrade trigger are
   judgment thresholds, not derived ones — both err toward simplicity, both are policy-as-data
   revisable.
4. Whether the EWMA of per-lane peak VmRSS is the right `MEM_PER_AGENT` estimator (vs p95) is
   unresolved; a heavy-tailed lane distribution would under-reserve. PSI backstop bounds the harm
   (P25's own argument), but the estimator choice deserves one sentence of measurement before
   W-M5 hard-codes it.
5. glibc arena-bloat (§2.1) is cited from the manual but not observed on this host — the
   `MALLOC_ARENA_MAX` mitigation is pre-armed, deliberately not pre-applied.

**Q2 — biggest thing possibly missed:** the session-level token/spend budget may dominate long
before 16→32 lanes stress 16 GiB — P25's own Q2 said the same and it stands unchanged here. If
spend is the true frontier, this blueprint's value narrows to bounded native surfaces + honest
measurement (still worth having), and the D_max ceiling raise becomes latent capacity. Also
possibly missed: the Node-side agent process might expose its own memory knobs
(`--max-old-space-size` class) that shrink the dominant term directly — out of this repo's Rust
scope, flagged for the operator rather than silently dropped.

## 6. Anu / Ananke check

**Anu (derivable, not asserted):** the keep-the-system-allocator verdict derives from a live grep
(no override exists), Rust's own fetched 1.32 rationale, and the measured absence of an
allocation-rate problem — not from allocator folklore. The eviction verdict derives from the
fetched TinyLFU/Caffeine claims *plus* the scale check that discounts them here — the same source
that would justify W-TinyLFU at 10⁷ entries justifies LRU at 10³. The quantization numbers are
fetched, model-flagged, and applied to a byte count taken from the repo's own design doc. The two
rejects (graph scoring, tensor decomposition) each name the missing precondition (no entry graph;
no embedding matrix) — falsifiable by pointing at the structure if it ever exists. Weakest links,
named: three training-knowledge citations (Tofte–Talpin, Jégou, TT-Rec), all confined to
reject/defer verdicts where citation error cannot flip the outcome; and the §1.6 parity argument,
which M4's test replaces with evidence.

**Ananke (structural, not hoped):** boundedness is enforced by types and tests, not vigilance —
`BoundedStore` cannot exceed its byte budget without failing its own invariant test;
`MemoryBudget::try_reserve` is degrade-closed by construction like the `TokenBucket` it mirrors;
the `BlockStore` trait boundary makes every store decision reversible as a swap. The D_max raise
cannot outrun safety structurally: reservation is advisory-fast but PSI-memory remains in the
admission predicate (P25 §3.4) as the kernel-ground-truth backstop, so a wrong EWMA degrades to
today's behavior, never past it. What is NOT structural yet, named: nothing prevents a future
caller from instantiating a raw unbounded `MemStore` for a new cache — a grep-able convention
("caches wrap `BoundedStore`") until an eventual lint/test enforces it (W-M2 adds that grep to
the done-check as the cheap version).

## 7. Build plan — falsifiable done-checks

| # | Unit | Depends on | Falsifiable done-check |
|---|---|---|---|
| M1 | `kernel/src/memory_budget.rs` (`MemoryBudget`) + tests | — | `cargo test -p dowiz-kernel memory_budget::` green: invariant test (never over-capacity across a 16-thread reserve/release stress), saturating release, degrade-closed refusal; zero new deps (`grep` no new `Cargo.toml` entries) |
| M2 | `BoundedStore` (byte-budgeted LRU `BlockStore`) + wire as `CachingBackend` default store | M1 (shares budget handle, optional) | Eviction test: puts beyond budget evict LRU-first and `stored_bytes() ≤ budget` at every step; `cache.rs` call-counting double still proves exact-hit-zero-HTTP; repo grep: no *new* bare `MemStore::new()` on a cache path |
| M3 | `FileBlockStore` index-not-mirror | — | RSS falsifier: open a generated ~100 MB store, `VmRSS` delta < 10 MB (was ≈ store size); all existing backup round-trip tests green; caller audit for borrowed-`get` recorded in the commit message |
| M4 | `retrieval/ppr.rs` delegates to `csr.rs` | — | Bitwise parity: existing ppr fixtures produce identical `Vec<f64>` (== on bits) dense vs CSR; on ANY mismatch the unit is abandoned and this table updated — determinism outranks memory |
| M5 | `mem_per_agent` measurement + reservation wiring into admission | M1, P25 W1/W3, P24 W1b | Admission fixture test: with `capacity=2 GiB`, EWMA=0.6 GiB, the 4th lane is refused (`try_reserve` false) and re-admitted after a `release`; P25 blueprint carries the append-only note pointing here (done at registration) |
| M6 | DHAT/heaptrack baseline pass over `lm` + kernel benches, archived | — | A recorded profile artifact exists under the telemetry logs dir; the §4 row-1 allocator trigger becomes evaluable against it (the check is the artifact's existence + a one-paragraph reading) |

M1 ⊥ M3 ⊥ M4 ⊥ M6 (mutually independent, startable now); M2 after M1; M5 is the integration step
and the only one gated on sibling phases. Wave classification per P25 §6: all units are C-class at
verification time (cargo test/bench), D-class during authoring.

---

## Appendix — phase-table registration

Registered in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8 as **Phase 26** (26
confirmed free at registration time: §8 ends at §8.7/P25; grep for `P26|Phase 26` returned
nothing — re-read fresh this session, not assumed). Depends on: **25** (consumes its D_max
formula and admission predicate as the integration point; M1–M4, M6 have no hard dependency and
are startable immediately) and **24** (soft — VmRSS/PSI gauges; pre-P24 the same
`/proc/self/status` + `/proc/pressure/memory` files are read directly). Off-critical-path lane,
same class as P5/P8/P11/P12/P24/P25.

---

## Addendum (2026-07-17, same day, later session) — three verdicts OVERRIDDEN by operator direction

Explicit operator direction ("my vision, no objections") overrides three of this document's
verdicts. They are now planned forward in
[`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md`](BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md)
(registered as Phase 28):

1. **§0.7 / §2.5 / §4 row 6 — graph-scored cache (REJECT) → OVERRIDDEN.** A cache reference
   graph is built (co-access primary, derivation next, semantic at Layer B), as `Csr` + the
   existing deterministic PPR, following the living-memory blueprint's own "cache prefetch"
   layer. The evidence here (no production lineage for centrality-scored *eviction*) is retained
   in the new doc as a falsifier: PPR-scored eviction must beat plain LRU in a replay A/B or the
   scorer reverts while the graph stays for its other consumers.
2. **§0.8 / §2.6 / §3.6 tensor row — Tucker/CP/TT (REJECT premature) → OVERRIDDEN.** Planned as
   a hybrid multi-way object per operator sharpening: (cache-entry × cache-entry × relation-type)
   tensor (RESCAL shape) coupled with an (entry × feature) matrix mixing embeddings and
   graph-derived features (CMTF). Rung 1 (truncated symmetric eigendecomposition via a new
   deterministic `lowrank.rs` over `Csr::spmv`) is buildable now; this doc's finding that
   `spectral.rs` has no eigenvector path is confirmed there (`spectral.rs:195-214`) and shapes
   the design.
3. **§0.3 / §2.2 / §4 row 2 — arena/bump allocation (REJECT) → OVERRIDDEN.** A hand-rolled
   zero-dep `BumpArena` is specified at the graph/spectral rebuild-and-rank site (CSR full
   rebuild ≈ 2n+7 allocations/pass; dense charpoly ≈ n²+O(n) allocations/call), with its own
   benchmark-able claim stated on its own terms — the "ns vs 10-second network waits" comparison
   in §2.2 is explicitly NOT reused as a dismissal, per operator direction.

This document's remaining verdicts (M1–M6, allocator-keep, LRU-bound, quantization triggers)
stand unchanged; P28's W3/W4 depend on M2's `BoundedStore`.
