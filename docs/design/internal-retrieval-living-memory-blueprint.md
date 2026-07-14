# Internal Retrieval, Living Memory & Spectral Search — Blueprint

> Status: **v2 (2026-07-14)**. Grounded in a five-lane deep research pass (search/compression
> SOTA · spectral-wave-search viability · tensors-vs-vectors · codebase inventory ·
> living-memory→pgrust+TTL) plus a live read of both repos. **v2 (§11)** incorporates the
> operator's engineering critique — CSR / cache-locality / deterministic fixed-point push — and
> the second-order field operator (velocity/momentum, eigenvectors, divergence, oscillators),
> and promotes **quantization + PQ + CoD + CDC** from deferred to active. §11 refines §1a's
> push recommendation and §3's L3 layer; read it as the load-bearing implementation spec. Every claim tagged **PROVEN**
> (peer-reviewed / measured this session) · **RESEARCHED** (sourced) · **SPECULATIVE**
> (design hypothesis, flagged). Companion: `math-first-architecture-blueprint.md` (this is
> its retrieval/memory substrate — serves master-roadmap P8 ops + P9 growth), and reuses the
> §1.5.5 **spectral-waves** invariant.

## 0. Thesis (one sentence)

**There are two complementary retrieval regimes — EXACT ("where is X": grep / inverted index
/ B-tree, deterministic, worst-case-bounded, unbeatable for keys) and RELATEDNESS ("what's
related to X": diffusion / spectral ranking on a graph, no total order) — and the job is to
add the relatedness layer *next to* exact search, never instead of it; on top sits a
compression layer and a living-memory-as-pgrust local database whose TTL is *decay + tier
demotion, never deletion*, with one heat-kernel diffusion operator hypothesised (to
prototype and measure) across recall, cache-prefetch, and code-relatedness.**

## 1. The two honest verdicts (what this research killed — the most valuable output)

### 1a. "Replace binary search with wave graph search" is a **category error** (as literally stated)

Binary search / B-tree / hash answer *"where is the exact key X"* in O(log n)/O(1) over a
**total order**. Spectral / diffusion / wave methods answer a *different* question —
*"what is related to X, and how strongly"* on a graph with **no total order**. A diffusion
score is a **ranking**, not an index into an ordered key domain. You cannot do `WHERE id=42`
with PageRank. **They are complementary primitives, not substitutes.** (PROVEN — the phrase
only becomes legitimate when it smuggles in a *different* job: "replace linear scan / keyword
matching with relatedness ranking," which is real.)

- **Where diffusion/spectral genuinely wins** (RESEARCHED, production-proven): associative
  recall on a note graph; ranking-by-relatedness & prefetch (Twitter WTF, Pinterest Pixie =
  random-walk-with-restart at 3B nodes / >80% of engagement); code-module detection (Fiedler
  λ₂ / spectral clustering); diffusion-based cache prefetch.
- **Where exact structures are unbeatable and must stay**: exact key lookup, ordered range
  scans (`ORDER BY`, pagination), point queries, exact keyword (inverted index). Money / auth
  / RLS / FSM-signature paths are the *wrong* place for a probabilistic ranking layer. 🔴
- **The cheap entry point** (PROVEN, critical): diffusion ranking does **NOT** need an
  eigendecomposition. Personalized-PageRank via the local **push algorithm** (Andersen-Chung-
  Lang, FOCS 2006) runs in `O(1/(αε))` — *independent of graph size*; heat-kernel via a
  truncated Taylor solve (Kloster-Gleich, KDD 2014). Full O(n³) spectra are infeasible past
  ~10⁴–10⁵ nodes (a 10⁶ dense eigensolve took ~1h on an 82,944-node supercomputer). **Start
  with local-push, never a precomputed spectrum.**
- The diffusion family is one thing (Chung 2007, PNAS): PageRank/RWR = *geometric* decay
  `(I−(1−α)W)⁻¹`; heat-kernel = *Poisson* decay `e^{−tL}`. Same `Σ_k c_k Wᵏ f`, different filter.

### 1b. "Replace vectors completely with tensors" is **overreach** (partly a definitional error)

A vector *is* a rank-1 tensor, so "replace vectors with tensors" is incoherent as stated.
(PROVEN.) The grounded position:

- **Vectors stay the default** retrieval unit — O(n) not O(nᵈ), cache-friendly, what SIMD &
  ANN indexes (HNSW) are built for, and what every embedding already is.
- **VSA / hyperdimensional computing is the pragmatic "tensor-like" win** — bind (circular
  convolution `FFT(a)⊙FFT(b)`) / bundle / permute encode role-filler records, sequences, and
  graphs into **fixed-width** hypervectors at O(n log n), no rank blow-up. **Already built,
  tested (exact round-trip, RED-guarded), and live** in `bebop2/core/src/vsa.rs` (and in the
  courier-matching prod path via `rust-core`). This is the right structure-encoding layer.
- **Tensor-Train decomposition** earns its keep only for **compressing large dense discrete-
  key tables** (measured 100–2000× on embedding tables) — which **do not exist** in this
  corpus (150 sparse relational notes). Deferred until such a table exists.
- **Tucker/RESCAL** multi-relational factorization = optional *offline analytics*, never the
  live path. This aligns exactly with bebop2's own directive 1: *"replace every dense tensor
  with its spectrum, never form the dense operator"* (`bebop2/core/src/lib.rs:7`). Memory =
  **spectral coefficients / VSA hypervectors, not dense tensors.**

## 2. Ground truth — what exists today (codebase inventory)

- **A proven retrieval engine already exists but is stranded**: `spikes/living-knowledge/`
  (recall@5 = 1.000 on a 29-query oracle; semantic⊕BM25⊕title fusion) lives only on
  `origin/feat/sovereign-core-phase-zero`, **unmerged**, and its own `ingest.mjs` was
  *designed* to ingest living-memory but never wired. **Biggest "already built, just merge +
  wire it" win.** (Today it's a linear scan over 77 files — small-corpus fusion, not an index.)
- **The living-memory store has no algorithm running over it**: `MEMORY.md` index + **174
  flat `.md` files** + `MEMORY-ATTIC.md`, cross-linked by **462 `[[wikilinks]]`** (a real
  graph, unextracted). Recall today = manual LLM navigation.
- **pgrust is a *plan*, not code** — only `deploy/pgrust.{env,toml,service}`; it's an upstream
  binary (malisper/pgrust, disk-compatible with PG18.3, ~67% compat, extensions incomplete →
  **pgvector blocked**). `kernel/src/event_log.rs` already wires it via an `EventStore` trait
  (in-mem `MemEventStore` until pgrust lands) — the house style exists. → spectral/vector work
  is an **app-layer operator over pgrust-stored data**, not a fork of pgrust internals.
- **Spectral organs are retrieval-ready**: `bebop2/core/src/field.rs` heat-kernel propagator
  `u(t)=Σe^{−λt}⟨u0,φ⟩φ`; `wavefield.rs` graph-Fourier over *typed* nodes (memory/file/entity);
  `kernel/src/markov.rs:123-142` damped PageRank power-iteration; `kernel/src/spectral.rs`
  general eigensolver (Fiedler λ₂). All graph-agnostic — re-pointing them at a content graph
  is a parameter change, not new math.
- **Compression primitives exist, partly stubbed**: `tools/vsa/` codec (34.3% reduction, JS,
  unmerged branch); `renormalizer.rs` claim-preserving gate (RED+GREEN proven) whose length
  oracle `compression_length_bits` is a **naive byte-count proxy** flagged for a real
  compressor swap; `entropy_ledger.rs`.

## 3. The architecture — a layered retrieval stack

Four layers, each answering a different question. Adopt cheapest-proven first; measure each
against an honest baseline before adding the next.

| Layer | Question | Technique (lane A/B) | Determinism | Reuse |
|---|---|---|---|---|
| **L0 · Exact byte/regex** | "find literal / pattern" | **trigram inverted index** + `regex-automata`/`memchr`/`aho-corasick` verify | exact | new (~400–900 LOC) |
| **L1 · Ranked full-text** | "best keyword matches" | hand-rolled **positional inverted index + BM25** (not tantivy — 34 deps, no WASM) | exact | new (~450–900 LOC) |
| **L2 · Semantic** | "similar meaning" | **flat SIMD → HNSW**; one embedding per note (rank-1) | flat = exact; HNSW = avg-case | `instant-distance` pattern |
| **L3 · Relatedness / diffusion** | "what's related, even indirectly" | **personalized-PageRank / heat-kernel** over the wikilink & import graph, via **local-push** (no eigendecomp) | deterministic (integer/fixed iterations) | reuse `markov.rs` / `spectral.rs` |

- **L0 is the "improve grepping" ask.** Trigram is the proven design (Google Code Search,
  zoekt): index ≈20% of corpus, narrows candidates ~100×, then the real regex verifies (zero
  false positives reach the caller). Keep postings **literal-keyed** (no Bloom filter) to stay
  bitwise-deterministic.
- **L3 is the "spectral wave search."** It is a **complement**, seeded at a query node, ranked
  by diffusion amplitude — the honest framing from §1a. Start with local-push PPR over the
  442-edge wikilink graph; measure vs the L1/L0 baseline before investing in spectra.

## 4. Compression

- **Note storage → `zstd` with a trained dictionary** (lane A: the one justified C-dep). The
  `sqlite-zstd` analog (many small similar rows) measured 72–91% reduction — markdown notes
  are a textbook match. Train on the corpus (>100 samples).
- **Swap the renormalizer's length oracle**: `compression_length_bits` byte-count → real
  `zstd(x).len()` — a self-declared, RED+GREEN-guarded, low-risk upgrade.
- **Event-log / sorted sequences → delta-of-delta + varint** (hand-roll ~30–50 LOC;
  Gorilla-style, ~96% of regular timestamps → 1 bit).
- **VSA zero-token match** (`tools/vsa`/`bebop2 vsa.rs`) for lesson/loop/memory similarity
  without embeddings.
- **MDL** stays an **ADR principle, not a pipeline stage** (lane A negative result: no
  production system implements literal MDL; measure actual compressed bytes instead).

## 5. Living memory as a pgrust local database

The living-memory pattern *is already* a database (index = covering view, notes = rows, attic
= soft-delete, wikilinks = a graph). Formalize it (lane E):

```sql
CREATE TABLE memory_notes (
  id      BYTEA PRIMARY KEY,            -- sha3_256(concept‖payload), event_log.rs style
  slug    TEXT UNIQUE NOT NULL,         -- = current *.md filename
  concept TEXT, payload TEXT NOT NULL, topic TEXT, entities TEXT[] DEFAULT '{}',
  layer   SMALLINT DEFAULT 1,           -- Working/Short/Long
  salience DOUBLE PRECISION DEFAULT 0,
  tier    SMALLINT DEFAULT 0,           -- 0 Hot / 1 Warm / 2 Cold / 3 Attic
  embedding BYTEA,                      -- app-level float4[] (pgvector blocked on pgrust)
  decay_tau DOUBLE PRECISION DEFAULT 7.0,  -- PER-ROW (HLR-style), not one global τ
  created_at TIMESTAMPTZ DEFAULT now(), last_reinforced TIMESTAMPTZ DEFAULT now()
);                                       -- NO DELETE anywhere in the write API, by policy
CREATE TABLE memory_links (src_id BYTEA, dst_id BYTEA, weight DOUBLE PRECISION DEFAULT 1.0,
  PRIMARY KEY (src_id, dst_id));         -- the 462 wikilinks, extracted
CREATE INDEX memory_notes_hot ON memory_notes (salience DESC) WHERE tier < 3;
```

**The single most important decision: there is no `attic` *table*.** Attic = `tier=3` on the
same row + a partial index; eviction is a metadata `UPDATE`, never a `DELETE`. Postgres/pgrust
MVCC gives "soft-update, reclaim physical space later via VACUUM" for free — no LSM tombstones,
no CRDT-merge machinery (dowiz already rejected CRDT-merge for anything money-adjacent; memory
is single-writer). **Never-delete is enforced by the schema, not by convention.**

## 6. Memory TTL — decay & tiering, never deletion

The reconciliation in one sentence (lane E): **every eviction algorithm computes a *rank*;
the never-delete invariant constrains the *action*, not the ranking math.**

- **TTL is redefined**: `TTL_expired ⟺ demote one tier`, never `⟺ delete`. Hot→Warm→Cold→Attic
  by salience threshold; a row at `tier=attic` still exists, full payload intact, restorable
  by `reinforce()`.
- **Decay**: keep bebop's exponential `s·e^{−Δt/τ}` but upgrade the **global τ → per-row τ**
  (Half-Life-Regression, Duolingo ACL 2016 — features: reinforcement count, wikilink in-degree,
  arc-liveness). Power-law fits human forgetting better than exponential (Wixted) — a later
  option. Compute **salience lazily at read time** from `(salience, last_reinforced, τ)` (Redis
  lazy-expiry) — no global sweep.
- **Real eviction is allowed only in a read cache in front of pgrust** (W-TinyLFU / ARC —
  SOTA, self-tuning) — it's a cache, dropping cold entries costs nothing; the store never drops.
- **Two-speed clock**: fast automatic decay/demotion; **slow, operator-gated compaction** of
  long-dwelling attic rows (payload → hash + summary, never drop the row/hash) — mirrors the
  existing "operator-approved floor cut" and pre-empts Automerge's tombstone-bloat failure mode.
- This is exactly `MEMORY.md`'s standing rule **"RANK-only never CULL"**, formalized into a
  state machine with a physical schema. 🔴 reversibility invariant preserved.

## 7. The unifying operator — field-sim across layers (the spectral/tensor connection)

The gem (lane E, PROVEN identity): bebop's decay `s·e^{−Δt/τ}` **is the heat kernel `e^{−tL}`
in the degenerate 1×1 (isolated-node, no-edges) case.** Put the edges back — diffuse salience
across `memory_links` (`L=D−A`) — and a reinforced note **warms its wikilinked neighbours**
before decaying (citing an old note back into use should warm what it cites). And **recall =
personalized/heat-kernel PageRank seeded at the query node = exactly `markov.rs:123-142`**
power-iteration-with-teleport, re-pointed at the wikilink adjacency.

**The hypothesis (SPECULATIVE — prototype and measure, do not assume):** one diffusion
operator `Σ_k g(λ_k) φ_k φ_kᵀ · u₀`, parameterized by `(seed, graph, decay filter g)`, serves
three layers:
- **Memory recall** — seed = query/context, graph = wikilinks → ranked associative recall.
- **Cache prefetch** — seed = recently-accessed, graph = access/dependency → prefetch candidates.
- **Code relatedness** — seed = edited file, graph = import/call → ranked related files.

All three are the same math family, all computable via local-push *without* eigendecomposition.
No published system ships exactly this one-operator-across-three-layers design → it is a design
hypothesis to build small and measure, not a proven pattern. **Where it does NOT apply**: any
exact-match / total-order / money / RLS path.

**Tensors, concretely**: vectors as the default; **VSA** (`vsa.rs`) for compositional memory
keys (`bind(role, filler)` → `bundle` facts → `unbind` to query, all via the existing FFT);
one embedding vector per note (rank-1) as the "tensor"; TT-decomposition deferred until a large
dense table exists.

## 8. Roadmap (M0…M8 — forward-only, each gated by a benchmark vs an honest baseline)

- **M0 — Merge + wire the living-knowledge engine** to the 174-file living-memory corpus
  (repoint from 77 files; its ingest was designed for exactly this). *Biggest existing win, no
  new math.* Re-prove recall@5 on an expanded oracle.
- **M1 — L0 exact search**: trigram inverted index + `regex-automata` verify. Deterministic,
  incremental. Benchmark vs `ripgrep` on the repo (candidate-reduction × latency).
- **M2 — L1 ranked full-text**: positional inverted index + BM25 (hand-rolled, zero-dep). Bench
  vs the M0 fusion engine.
- **M3 — L3 diffusion recall**: extract the wikilink graph → `memory_links`; personalized-
  PageRank via `markov.rs` **local-push** seeded at the query. **Measure vs the M1/M2 baseline
  before any spectral precompute.** The "spectral wave search," honestly scoped.
- **M4 — Living-memory → pgrust**: the §5 schema + §6 tier-TTL (never delete) + per-row τ; keep
  the `EventStore`-trait / in-mem-until-pgrust house style.
- **M5 — Compression**: `zstd`-dictionary note storage; swap the renormalizer length oracle to
  real zstd; delta/varint for the event log.
- **M6 — L2 semantic**: one embedding per note; flat SIMD first, HNSW (`instant-distance`
  pattern) only when flat-scan latency exceeds budget.
- **M7 — Unifying field-sim operator** (SPECULATIVE): prototype the one diffusion operator
  across recall + cache-prefetch + code-relatedness; measure each against its own baseline
  (grep / co-access heuristic). Kill or keep on measured evidence.
- **M8 — VSA composite memory keys** (`bind`/`bundle`/`unbind`); Tensor-Train **deferred**
  (revisit only if a large dense embedding table materializes).

## 9. Guardrails (carried from the standing rules)

1. **Cheapest-proven-first**: local-push before any spectrum; flat SIMD before HNSW; trigram
   before FM-index; `Vec<u32>`+delta before Roaring. Defer every big-data technique (FM-index,
   IVF-PQ, ScaNN, DiskANN, succinct, learned-index-as-primary) until a *measured* bottleneck.
2. **Measure vs an honest baseline before adding spectral/semantic** — grep/inverted index for
   exact, a co-access heuristic for prefetch. (Verified-by-Math / ground-truth-over-proxy.)
3. **Relatedness/ranking never touches exact/total-order/money/auth/RLS/FSM paths.** 🔴
4. **Never-delete**: the store demotes tiers; only a front read-cache may evict.
5. **Determinism**: literal-keyed postings (no Bloom), integer/fixed-iteration diffusion, PGM
   (not RMI) if any learned index, everything testable against a naive oracle.
6. **pgrust stays upstream** — build the app-layer operator over it, don't fork its internals.

## 10. Reconciliation with the existing roadmaps

This is the **retrieval/memory substrate**: it serves the master 10-phase roadmap's **P8**
(ops/single-pane — pgrust local DB) and **P9** (self-development/growth — the living memory is
the growth substrate), and it *is* an application of the math-first blueprint's **§1.5.5
spectral-waves invariant** (the same `field.rs`/`markov.rs`/`spectral.rs` organs, re-pointed at
a content graph) and its **S6 equation-IR / memory** phase. It never front-runs the master
INVARIANT — *build DOWN from the first real order.*

## 11. v2 engineering refinements — physical engine, second-order operator, active quantization

Incorporates the operator's critique (CSR / cache-locality / deterministic push) and design
questions (eigenvectors, divergence, oscillators, velocity terms; quantization + PQ + CoD +
CDC now). The critique is correct and sharpens the plan.

### 11.1 How the graph is stored: CSR (decided)

**Yes — Compressed Sparse Row.** `Vec<Vec<T>>` adjacency + `HashMap` edges are pointer-chasing =
near-guaranteed cache misses; a 90 ns math kernel would stall on RAM. CSR lays every vertex's
neighbours contiguously (`row_ptr[]`, `col_idx[]`, `val[]`) → a sweep reads memory sequentially
→ hardware prefetch → high hit rate. This **is** the DOD invariant (§1.5.3) applied to the graph
layer — the same "flatten `Vec<Vec>` → contiguous" that kills the kernel's matmul heap-churn.
**Data layout beats formula micro-optimization** — agreed. Scale check: the memory-note CSR
(~150 nodes, ~462 edges) is a few KB → **fits L1**; the code import graph (thousands of nodes) →
tens–hundreds of KB → fits L2. So **at our scale CSR alone gives ~90%+ hits with no tiling**;
cache-tiling / memory-blocking is the >L3 (~10⁶-edge) path — real, but not yet. The sweep
(`Âπ` / `c²LU`) is a sparse mat-vec = the SIMD target (FMA over contiguous neighbour runs); the
kernel just feeds vertices into the SIMD engine (CPU-pipeline design, yes).

### 11.2 Determinism vs local-push — the critique is right; the fix (refines §1a)

ACL local-push is **order-dependent** (the ε-truncated residual depends on push order), spikes on
low-conductance bottlenecks, and cache-misses — so a naïve async push makes the kernel
non-deterministic. The converged PPR π is unique; its ε-approximation is not. Resolution, ranked:
1. **Deterministic synchronous fixed-point (Jacobi power-iteration)** — `π_{k+1}=α·e_seed +
   (1−α)·π_kÂ`, fixed K, fixed summation order. Each iteration is a pure function of the previous
   vector (one CSR mat-vec) → **order-independent, bitwise-reproducible on any hardware**. Already
   what `kernel/src/markov.rs:123-142` does. O(K·nnz), cheap at our scale (CSR in cache). **The
   deterministic default** — this *replaces* v1's "start with local-push."
2. **Deterministic-ordered local-push** — ACL locality (touch only active nodes) with a *fixed*
   worklist order (residual-priority + node-id tie-break), fixed ε → reproducible. The
   large-graph (>10⁵-node) optimization, when a full sweep is too costly and the query is local.
3. **Async / relaxed-consistency push** — fastest, **non-deterministic** → advisory layer ONLY,
   never on a gated/money/RLS path. 🔴
Float note: ranking is advisory → f64 + fixed-K + fixed-order is reproducible; if a ranking ever
*gates* a decision, move it to integer fixed-point (math-first Pillar B). Never gate on an async push.

### 11.3 Why not just diffusion — eigenvectors, divergence, oscillators, velocity terms

v1's "diffusion, first-order" is the *overdamped special case* of a richer operator; use the full
machinery for the jobs where it pays:

- **Second-order field operator (velocity/momentum).** Heat diffusion `∂u/∂t=−Lu` has no
  momentum. The general form is the field-UI engine's own **`M Ü + Γ U̇ + c²L U = S`** (FE-08,
  ζ=1 critically-damped), read as *salience-as-a-field*: `S`=reinforcement (source at the touched
  note), `Γ U̇`=decay (damping), `c²L U`=link-diffusion (warm wikilinked neighbours, §7), and
  **`U̇`=momentum** — a note gaining citations has salience-*velocity*, so it ranks higher and
  decays slower than its instantaneous score. This ONE operator subsumes diffusion-recall (its
  overdamped limit) + momentum ranking + decay + link-warming, and it is *the same operator the
  UI runs* (§1.5.5 made literal). **ζ=1** = monotone, no overshoot, deterministic at fixed
  timestep → the gated/default form; **underdamped** = exploratory recall / resonance.
- **Eigenvectors** — warranted, but **top-k iterative (Lanczos/power), never full dense O(n³)**.
  On a static, many-query graph, precompute bottom-k Laplacian eigenvectors once and amortize:
  Fiedler λ₂ → **module/community detection** (refactor boundaries); Laplacian eigenmaps → a
  **coordinate embedding** of the notes. `spectral.rs` already computes these. "Avoid
  eigendecomposition" only ever meant *avoid full dense per-query*, not *avoid the spectrum*.
- **Divergence (∇·).** The Laplacian is discrete `div·grad`; a node's divergence = net relevance
  out/inflow = a **hub-authority / source-sink / anomaly** signal (`wavefield::graph_spectral_notch`,
  `mathx::divergence_2d`) — a derived signal next to the diffusion score.
- **Oscillators / interference.** `coherence.rs |ψ₁±ψ₂|²` = **multi-query fusion** — constructive
  (union of seeds) vs destructive (contrast, "related to A but not B"); resonance = a subgraph's
  frequency signature. The *wave* uses the first-order heat kernel can't express.

Net: retrieval/memory dynamics unify under the **same critically-damped field operator as the
UI**, with diffusion as its overdamped limit — the strongest form of the §7 "one operator across
layers" hypothesis, now second-order. Determinism kept by ζ=1 + fixed timestep (FE-08's monotone
integrator).

### 11.4 Quantization + PQ — active now, and deterministic

Placement: **L2 embedding storage** (never L0/L1/money paths).
- **Scalar (int8) by default now** — 4×, ~free recall, SIMD int8; deterministic with **fixed
  ranges** (per-dim min/max from a committed snapshot, not float-order-dependent). ~10 LOC, zero-dep.
- **Product Quantization (PQ) as the scale path** — 64× (128-d → 8 B via 8×256 codebooks). 🔴
  **Determinism requirement**: the codebook is k-means (init/order-dependent) → train
  **deterministically** (fixed seed, fixed iteration count, deterministic k-means++ init, fixed
  reduction order), then **freeze + commit it** (a "codebook manifest", like the Manifesto of
  Constants). Query-time PQ (LUT lookup) is then fully deterministic. **Never retrain a live
  codebook.** OPQ (learned rotation first) same discipline.
- **Binary quantization** (16–32×) only with a float/SQ8 **rescore** stage; model-dependent.
- Standard pipeline: quantized shortlist → exact rerank against full/SQ8 vectors — keeps recall
  at small footprint, deterministic.

### 11.5 CoD + CDC — both, in the compression layer

- **Chain-of-Density (CoD)** — the **compaction summariser** (§6's slow tier-3→archive step:
  `payload → dense summary + hash`, never delete). CoD iteratively packs salient entities into a
  fixed-length summary. **Edge/offline LLM tool** (non-AI-core preserved); its output is admitted
  ONLY through the **`renormalizer.rs` claim-set gate** (reject if any claim dropped/hallucinated)
  — generation at the edge, deterministic acceptance.
- **Content-Defined Chunking (CDC — FastCDC / Rabin)** — **dedup**: split notes at content-defined
  boundaries so near-duplicate blocks (shared boilerplate across arc files) share storage; pairs
  with MinHash/LSH (§4). Rabin fingerprinting is **deterministic**. Complements zstd-dictionary
  (CDC dedups *across* notes; zstd compresses *within*).

### 11.6 Roadmap deltas (v2)

- **M3** implementation spec = CSR + deterministic synchronous power-iteration (markov.rs), not
  local-push; and it carries the **second-order `MÜ+ΓU̇+c²LU=S` operator** (ζ=1) as the general
  form, diffusion as its overdamped limit.
- **M5** gains **CoD** (compaction summariser, renormalizer-gated) + **CDC** (dedup).
- **M6** gains **SQ int8 now** + **PQ (deterministic frozen codebook)** as the scale path.
- **M7** (unifying operator) is explicitly the second-order field operator, ζ=1 gated / underdamped
  exploratory — measured against the overdamped-diffusion baseline.

---

### Key sources (consolidated)
Andersen-Chung-Lang local-push (FOCS 2006) · Kloster-Gleich heat-kernel (KDD 2014) · Chung heat
kernel as PageRank (PNAS 2007) · Haveliwala topic-sensitive PageRank · Malkov-Yashunin HNSW
(arXiv:1603.09320) · Russ Cox trigram index / zoekt design · BurntSushi regex-automata /
aho-corasick / memchr · Robertson BM25 / BM25F · Oseledets Tensor-Train (SIAM 2011) · TT-Rec
(arXiv:2101.11714) · Kleyko et al. VSA survey (ACM CSUR) · Settles-Meeder Half-Life Regression
(ACL 2016) · Wixted power-law of forgetting · Megiddo-Modha ARC / Einziger W-TinyLFU · Ferragina
PGM-index (VLDB 2020) · Leis ART (ICDE 2013) · sqlite-zstd dictionary case study · malisper/pgrust · Adams et al.
Chain-of-Density summarization (2023) · Xia et al. FastCDC content-defined chunking (USENIX ATC
2016) · Jégou et al. Product Quantization (TPAMI 2011) / OPQ · CSR + GraphBLAS sparse mat-vec ·
field-UI operator `MÜ+ΓU̇+c²LU=S` (FE-08, engine/src/motion.rs, ζ=1 critical damping).
Codebase: `spikes/living-knowledge/`, `bebop2/core/src/{vsa,fft,field}.rs`,
`crates/bebop/src/{wavefield,coherence,memory,renormalizer}.rs`,
`kernel/src/{spectral,markov,event_log}.rs`, `MEMORY.md` + 174 topic files, `deploy/pgrust.*`.
