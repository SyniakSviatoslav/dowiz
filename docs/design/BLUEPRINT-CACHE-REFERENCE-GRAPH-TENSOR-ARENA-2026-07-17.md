# BLUEPRINT — Phase 28: Cache Reference Graph + Hybrid Tensor Decomposition + Bump Arena (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART, 2-question doubt
> audit, Anu/Ananke check. Plain prose, no metaphor; every load-bearing statement carries a
> `file:line` cite, a live-command ground, a web citation, or is tagged **(proposal)** /
> **(training-knowledge)**.
>
> **OVERRIDE NOTICE (operator directive, 2026-07-17):** this blueprint overrides three verdicts of
> `BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md` (P26) by explicit operator direction
> ("my vision, no objections"): P26 §0.3/§2.2/§4-row-2 (arena REJECT), §0.7/§2.5/§4-row-6
> (graph-scored cache REJECT), §0.8/§2.6/§4-table-row "Tucker/CP/TT" (tensor REJECT-as-premature).
> The P26 document carries a dated addendum pointing here; its evidence is not erased — where its
> findings remain true as *evidence* (e.g., no production lineage for centrality-scored eviction),
> they are restated here and answered with falsifiers rather than silently dropped. The task here
> is to design these three forward, concretely, not to re-litigate whether they are warranted.
>
> **Operator sharpening (received mid-task, binding):**
> 1. The tensor-decomposition target is NOT a plain embedding matrix. It is a HYBRID multi-way
>    object: the same relations that form the cache reference graph's edges (semantic similarity,
>    co-access, derivation) PLUS embeddings where available — modes (cache-entry × cache-entry ×
>    relation-type), coupled with an (entry × feature) side matrix mixing embedding components and
>    graph-derived features. Task 2 is bound to task 1's graph, not a parallel concern.
> 2. The arena's nanosecond-level saving is wanted on its own terms. "Small relative to network
>    latency in the same request path" is NOT an acceptable dismissal — that comparison was the
>    overridden verdict. The arena is specified as a complete primitive with its own honest
>    benchmark-able claim.
>
> **Provenance:** web sources below marked *fetched* were retrieved live this session (arXiv
> abstracts, two full PDFs read directly: RESCAL ICML 2011 pp.1–2 and Kolda–Bader SIAM Rev. 2009
> pp.455–456, plus GitHub/docs.rs pages). Items P26 fetched are cited as *(fetched, P26 §N)*.
> One item resisted full-text fetch and is flagged where used (Jégou PQ — located via the Faiss
> wiki's direct HAL link, *fetched* at the wiki level).

---

## 0. Executive answer

1. **Cache reference graph (task 1) — the centerpiece, ADOPT (build).** Node = a cache entry's
   existing sha3-256 request key (`llm-adapters/src/cache.rs:57-81`) mapped to a dense index by
   insertion order (insertion order IS the chronology axis). Edges = **co-access** (primary, v1:
   two entries touched within one sliding access window; O(window) counter work at access time,
   aggregation free via `Csr::from_edges`'s duplicate-merge-by-summing, `kernel/src/csr.rs:93-103`),
   **derivation** (v1.1: cached response used as context for a later request, recorded when the
   harness threads provenance), **semantic** (deferred to Layer B's embeddings, unchanged trigger).
   Storage = `Csr` verbatim; query = the existing deterministic `personalized_pagerank`
   (`csr.rs:228-264`) seeded from the recently-accessed set; output = a retention/prefetch ranking
   consumed by P26's `BoundedStore` eviction (PPR-primary, LRU tie-break and fallback). This is
   the living-memory blueprint's own second named layer — "Cache prefetch — seed =
   recently-accessed, graph = access/dependency" (`internal-retrieval-living-memory-blueprint.md`
   §7) — instantiated for cache retention, not a fourth competing design. Chronology (when cached)
   = node order + timestamps; topology (what relates to what) = the relation edges; recall (what
   matters now) = PPR from recent seeds. HippoRAG (NeurIPS 2024, *fetched*) is the production-
   adjacent precedent that PPR over an association graph is a workable memory-retrieval mechanism.
2. **Hybrid tensor decomposition (task 2) — ADOPT a concrete ladder, first rung buildable now.**
   The object is a third-order tensor 𝓧 ∈ ℝ^(n×n×m): modes 1–2 = cache entries (shared), mode 3 =
   relation type — exactly RESCAL's entity×entity×relation shape, factorized X_k ≈ A·R_k·Aᵀ with
   one shared entity factor A (Nickel–Tresp–Kriegel, ICML 2011, *fetched PDF*). Embeddings, when
   Layer B builds them, enter as a coupled side matrix Y ∈ ℝ^(n×d) sharing the entry mode —
   coupled matrix-tensor factorization (Acar–Kolda–Dunlavy 2011, *fetched*), NOT a separate
   embeddings-only compression path. Rung 1 (today, m=1, no embeddings): the tensor degenerates to
   the symmetric co-access matrix, and its truncated eigendecomposition W ≈ U_k Λ_k U_kᵀ IS the
   decomposition (Tucker is "a higher-order form of principal component analysis" — Kolda–Bader,
   SIAM Rev. 51(3), 2009, *fetched PDF*). **DECART verdict on the spectral.rs question, honest:**
   `spectral.rs` computes eigenVALUES only (`spectral.rs:195-214`; the Householder fast path too,
   `householder.rs:338`) — a compression primitive needs eigenVECTORS, so "extend the existing
   eigensolver" is NOT the minimal path through the Durand-Kerner engine. The minimal path is a
   new ~120-LOC sibling `kernel/src/lowrank.rs`: deterministic fixed-K power iteration + Hotelling
   deflation for top-k symmetric eigenpairs over the EXISTING `Csr::spmv` — reusing the sparse
   kernel verbatim and landing the basis in `spectral_cache.rs`'s `Decomp` type, whose basis slot
   already exists and is always empty today (`spectral_cache.rs:28,122`). Vector-storage
   compression (int8 SQ, then PQ) is complementary, not competing, and keeps P26's fetched numbers.
3. **Bump arena (task 3) — ADOPT (build) at the graph/spectral rebuild site.** The real
   local-compute hot loop is the one task 1 creates plus two that already exist: (i) full CSR
   rebuild per maintenance pass (`Csr::from_edges` + `row_normalize` ≈ 2n+7 heap allocations per
   rebuild — the same full-rebuild-per-invocation pattern RCI's `rci derive` uses,
   `realtime-change-intelligence-2026-07-17/resolution.md` M5: "one honest full CSR rebuild per
   invocation, O(E log E) ≈ 5–10 ms"); (ii) the dense-spectral `matmul` wrapper that converts
   `Vec<Vec>` ⇄ `Mat` on every call inside charpoly's n−1 matmuls (`spectral.rs:35-39,113-137`) —
   ≈ n² + O(n) allocations per `charpoly` call for n > 32; (iii) per-iteration `nxt` allocation in
   the dense `Ppr::rank` (`retrieval/ppr.rs:47`). Design: `kernel/src/arena.rs` — a hand-rolled,
   zero-dep `BumpArena` (`Vec<u8>` region, `Cell<usize>` offset, `alloc_slice<T: Copy>`,
   O(1) `reset(&mut self)`), degrade-closed on exhaustion (falls back to plain heap `Vec`, never
   grows, never panics). The claim is stated on its own terms: ~2,055 malloc/free pairs per
   n=1024 rebuild become 3 pointer bumps + one reset; at a measured 20–100 ns per glibc
   malloc/free pair this is an expected ~40–200 µs per rebuild invocation, to be confirmed or
   refuted by a criterion A/B in the existing bench harness (`kernel/benches/criterion.rs`) —
   no comparison against network latency appears anywhere in the verdict.

---

## 1. Ground truth (live-verified this session)

### 1.1 The cache being graphed

- `CachingBackend<B, S>` wraps any backend with an `Arc<Mutex<S: BlockStore>>` store
  (`llm-adapters/src/cache.rs:31-34`). Key = sha3-256 of the BTreeMap-canonical request
  (`cache.rs:57-81`) — an opaque 32-byte content id, the natural node identity. Hit path
  `cache.rs:93-99`; put path `cache.rs:100-105`. `CachePolicy::NoCache` bypasses (`cache.rs:58`).
- The store trait is `BlockStore` (`kernel/src/backup.rs:39-57`): `put` (idempotent dedup),
  `get`, `get_owned`, `len`. `MemStore` (`backup.rs:60-93`) is the default;
  `stored_bytes()` (`backup.rs:71-76`) exists. P26 M2 plans `BoundedStore` (byte-budgeted LRU)
  behind this same trait — the eviction seam task 1 plugs into.
- The `Dispatcher` (`llm-adapters/src/dispatch.rs:57-127`) carries **no session/lane id today** —
  co-access windows must therefore be time-based in v1 (a per-dispatch tag is a small additive
  field if lane-scoped windows are ever wanted; noted, not required).
- `llm-adapters` already depends on `dowiz_kernel` (`cache.rs:13-18` imports), so the graph
  observer can live beside the cache and use `kernel::csr` directly.

### 1.2 The graph machinery being reused (not reinvented)

- `Csr` (`kernel/src/csr.rs:39-54`): one contiguous `val`/`col_idx` + `row_ptr` — the repo's
  binding sparse format. `from_edges` (`csr.rs:79-115`): per-row bucket sort, **duplicate (src,dst)
  pairs merged by SUMMING weights** (`csr.rs:93-103`) — which means a co-access edge buffer can
  append raw `(i, j, 1.0)` observations and the builder does the count aggregation with zero new
  code. `row_normalize` (`csr.rs:125-152`) with deterministic dangling-node self-loops.
- `personalized_pagerank` (`csr.rs:228-264`): synchronous Jacobi, fixed K, fixed summation order,
  bit-reproducible (tests `csr.rs:495-550`). `recall_at_k`/`precision_at_k` scorers
  (`csr.rs:387-427`). `laplacian_spmv` (`csr.rs:307-359`) for the spectral rung.
- The living-memory blueprint (`docs/design/internal-retrieval-living-memory-blueprint.md` §7)
  already names three layers of ONE diffusion operator: memory recall, **"Cache prefetch — seed =
  recently-accessed, graph = access/dependency → prefetch candidates"**, code relatedness. Its §11
  v2 refinements fix CSR storage and synchronous-Jacobi determinism as binding. Task 1 is that
  second layer, scoped to retention/eviction as well as prefetch — the SAME unification
  (chronology + topology + recall), not a new pattern.
- The dense duplicate (`retrieval/ppr.rs:18-22`, `Vec<Vec<f64>>`) is P26 M4's consolidation
  target; nothing here depends on it — task 1 goes straight to `Csr`.

### 1.3 The spectral machinery — what exists and what is genuinely missing

- `spectral.rs` computes **eigenvalues only**: Faddeev-LeVerrier charpoly (`spectral.rs:113-137`)
  → Durand-Kerner roots (`spectral.rs:141-186`); n ≤ 32 routes to the stack-only Householder
  engine, also values-only (`spectral.rs:195-214`, `householder.rs:81-115,338`). **No function in
  the module returns an eigenvector.** This is the load-bearing fact for task 2: truncated
  eigendecomposition-as-compression needs the vectors.
- `spectral_cache.rs` already defines `pub type Decomp = (Vec<Vec<f64>>, Vec<f64>)` — "(basis,
  values)… an eigenvector basis may be supplied by callers that solve for one"
  (`spectral_cache.rs:20-28`); every current caller stores an empty basis
  (`spectral_cache.rs:122`). The cache slot for a truncated basis exists; only the solver is
  missing.
- Allocation shape of the dense path: `matmul` wrapper does `Mat::from_vecvec` twice +
  `matmul_contig` result + `into_vecvec` per call (`spectral.rs:35-39`, `mat.rs:93`); `charpoly`
  calls it n−1 times ⇒ ≈ n²+O(n) transient allocations per call for n > 32 (for n=64: ≈4.3k
  allocations, ≈2.2 MB churn). This is ground truth for the arena site, §1.4.

### 1.4 Existing conventions the three designs must match

- Zero external deps in kernel; hand-rolled primitives (BM25, trigram, token_bucket, householder).
- `unsafe` precedent exists and is confined: `householder.rs`, `simd.rs`, `messenger.rs`
  (live grep). An arena with one documented unsafe block is in-convention.
- Bench harness exists: `kernel/benches/criterion.rs` + `BENCH_HISTORY.md` (criterion 0.5,
  `kernel/Cargo.toml:66-69`). Arena and lowrank claims are measurable without new infrastructure.
- Determinism contract: fixed iteration counts, fixed summation order, no HashMap iteration on
  ranked paths (`csr.rs:36-37`); advisory-vs-gated split via `CachePolicy` (`cache.rs:6-7`).
- The full-rebuild-per-invocation precedent: RCI resolution M5 — `rci derive` "does one honest
  full CSR rebuild per invocation, O(E log E) ≈ 5–10 ms"
  (`docs/design/realtime-change-intelligence-2026-07-17/resolution.md:243`). Task 1's maintenance
  pass adopts the same shape; task 3's arena serves both.

---

## 2. Research (web, fetched this session)

### 2.1 PPR-scored memory/retrieval — the precedent that did not exist in cache-eviction literature

P26 §2.5's finding stands as evidence: production cache-eviction lineage is recency/frequency
(ARC, W-TinyLFU), not graph centrality. What HAS production-adjacent standing is PPR over an
association graph as a *memory retrieval* mechanism: **HippoRAG** (Gutiérrez, Shu, Gu, Yasunaga,
Su — "HippoRAG: Neurobiologically Inspired Long-Term Memory for Large Language Models," NeurIPS
2024, arXiv:2405.14831, *fetched*) "synergistically orchestrates LLMs, knowledge graphs, and the
Personalized PageRank algorithm," reporting up to 20% improvement on multi-hop QA with single-step
retrieval 10–30× cheaper than iterative retrieval. `csr.rs` already names its scorers
"HippoRAG-style groundedness" (`csr.rs:362-369`). The honest framing for task 1: PPR-ranked
**relevance** over cache entries is precedented (HippoRAG-class); PPR-ranked **eviction** is the
operator's deliberate experiment, so the design carries a hit-rate falsifier against an LRU
baseline (§3.1.6) rather than a borrowed credential.

Semantic caching for LLM responses is likewise precedented: **GPTCache** (zilliztech, *fetched*)
converts queries to embeddings, searches a vector store, and evaluates similarity before serving —
with plain LRU/FIFO/LFU/RR eviction underneath. That is Layer B's mechanism (edge type (a)) and
confirmation that nobody ships graph-scored eviction — which is exactly why the falsifier stays.

### 2.2 Multi-relational tensor factorization — the hybrid-matrix construction, grounded

- **RESCAL** (Nickel, Tresp, Kriegel — "A Three-Way Model for Collective Learning on
  Multi-Relational Data," ICML 2011, *fetched PDF, pp.1–2 read directly*): models dyadic
  multi-relational data as a three-way tensor 𝓧 of shape n×n×m — "two modes are identically
  formed by the concatenated entities of the domain and the third mode holds the relations" —
  factorized per relation slice as **X_k ≈ A·R_k·Aᵀ**, A ∈ ℝ^(n×r) a latent-component
  representation shared across ALL relations, R_k ∈ ℝ^(r×r) asymmetric per-relation interaction.
  This is byte-for-byte the (cache-entry × cache-entry × relation-type) object the operator
  directed: entities = cache entries, relations = {co-access, derivation, semantic}.
- **CP for knowledge-base completion** (Lacroix, Usunier, Obozinski — "Canonical Tensor
  Decomposition for Knowledge Base Completion," ICML 2018, arXiv:1806.07297, *fetched*): KB
  completion framed as 3rd-order binary tensor completion; with a tensor nuclear p-norm
  regularizer, plain CP is state-of-the-art. Relevant if the graph ever needs link *prediction*
  (prefetch = predicting the next co-access edge) rather than just compression.
- **Coupled matrix-tensor factorization** (Acar, Kolda, Dunlavy — "All-at-once Optimization for
  Coupled Matrix and Tensor Factorizations," 2011, arXiv:1105.3422, *fetched*): joint
  factorization of a tensor and matrices sharing a mode ("fitting outer-product models to
  higher-order tensors and matrices in a coupled manner"). This is the named mechanism for the
  operator's second sharpening: the (entry × entry × relation) tensor and an (entry × feature)
  side matrix — embedding dims plus graph-derived features — share the entry mode and are
  factorized through ONE shared factor A. Embeddings become one more coupled feature block, not a
  separate compression subject.
- **Tucker/CP foundations** (Kolda, Bader — "Tensor Decompositions and Applications," SIAM Review
  51(3):455–500, 2009, DOI 10.1137/07070111X, *fetched PDF, pp.455–456 read directly*): "CP
  decomposes a tensor as a sum of rank-one tensors, and the Tucker decomposition is a higher-order
  form of principal component analysis." The two-way degenerate case of everything above is
  truncated PCA/SVD — which for a symmetric matrix is the truncated eigendecomposition. That is
  the bridge from "tensor decomposition" to what dowiz already has half of.
- **Randomized truncated SVD** (Halko, Martinsson, Tropp — "Finding structure with randomness,"
  SIAM Review 53(2):217–288, 2011, arXiv:0909.4061, *fetched*): random sampling identifies a
  subspace capturing "most of the action of a matrix," then deterministic post-processing.
  Named and REJECTED for this codebase's ranked paths: the random test matrix conflicts with the
  fixed-summation-order bitwise-determinism contract unless carefully seeded, and at n ≈ 10²–10³
  a deterministic power-plus-deflation solve is already cheap. Kept in the ladder as the named
  upgrade if n ever reaches 10⁵+.
- **Vector-storage quantization** (complementary track, numbers already fetched by P26): int8
  scalar quantization = 4× at ~99.3% retrieval retention *(fetched, P26 §2.3 — HF embedding-
  quantization benchmark)*; product quantization compresses e.g. 64 f32 → 8 bytes = 32×, with
  approximate distances (Faiss wiki "Lower memory footprint," *fetched*, which links the source
  paper directly: Jégou, Douze, Schmid — "Product Quantization for Nearest Neighbor Search," IEEE
  TPAMI 2011, located via the wiki's HAL link; full text not re-fetched this session). Factorization
  compresses relational structure into r-dim codes; quantization compresses stored vectors; they
  compose (quantize the factor matrix A itself at scale).

### 2.3 Region/bump allocation — the primitive, on its own terms

**bumpalo** (docs.rs, *fetched*): allocation is a pointer bump ("a quick check that we have
enough capacity left… then update the pointer by the object's size"); mass deallocation is
"extremely fast" (reset the pointer); allocated objects' `Drop` implementations are NOT invoked;
`Bump` is `!Sync`. These are the three design facts the hand-rolled arena must inherit or
neutralize: (1) bump-pointer allocation, (2) O(1) reset, (3) the no-Drop hazard — neutralized at
compile time here by a `T: Copy` bound, which is strictly stronger than bumpalo's runtime
convention (a `Copy` type cannot implement `Drop`; rustc enforces it). The `!Sync` constraint is
kept deliberately: one arena per rebuild pass, no sharing. Region allocation's lineage
(Tofte–Talpin 1997) was already flagged **(training-knowledge)** by P26 §2.2 and nothing here
depends on its details. No external crate is adopted — the repo's zero-dep convention and the
~80-LOC size of the needed subset both point at the hand-roll, and unlike P26's context, there is
now a named local-compute consumer (§3.3).

---

## 3. Design

### 3.1 Task 1 — `CacheGraph`: the cache reference graph (proposal; the centerpiece)

**Where it lives:** `llm-adapters/src/cache_graph.rs` (beside `cache.rs`; llm-adapters already
imports `dowiz_kernel`, §1.1). Graph math stays in `kernel::csr` — zero new graph code.

**3.1.1 Node.** A cache entry's content id — the sha3-256 request key `CachingBackend` already
computes (`cache.rs:57-81`) — interned to a dense `usize` by an append-only table:

```rust
pub struct CacheGraph {
    ids: Vec<Hash>,                 // node index -> content id; INSERTION ORDER = chronology
    index: BTreeMap<Hash, usize>,   // content id -> node index (BTreeMap: no iteration-order hazard class at all)
    meta: Vec<NodeMeta>,            // created_at_ns, last_access_ns, access_count, bytes
    recent: VecDeque<(usize, u64)>, // sliding co-access window (node, monotonic ns), cap N_RECENT
    edges: Vec<(usize, usize, f64)>,// append-only observation buffer; Csr::from_edges aggregates
}
```

Chronology is structural, not stored logic: node index order IS creation order, and `NodeMeta`
carries the timestamps. Nothing is ever deleted from `ids`/`index` (RANK-only-never-CULL applied
to the graph: an *evicted* entry's node persists as history/topology; only the `BoundedStore`
payload goes — re-inserting the same content id reuses its node and its accumulated edges).

**3.1.2 Edges (topology) — the three relation types, with the cost argument.**

| Type | When recorded | Cost at record time | Weight | Status |
|---|---|---|---|---|
| (b) co-access | On every cache access (hit or put): for each other node in the sliding window (`recent`, cap N_RECENT=64, span W=60 s — policy-as-data), append `(i, j, 1.0)` and `(j, i, 1.0)` to `edges` | O(window) pushes of 24-byte tuples; no I/O, no hashing beyond the existing key, no embedding | co-access count (aggregation FREE via `from_edges` duplicate-summing, `csr.rs:93-103`) | **v1 — primary** |
| (c) derivation | At cache-write time when the request carries provenance (`ChatRequest` gains `derived_from: Vec<Hash>`, optional, default empty — additive, no caller breaks) | O(parents) lookups + pushes | fixed 2.0 per parent link, both directions (one derivation outweighs one co-access; policy-as-data) | **v1.1 — lands when the harness threads provenance** |
| (a) semantic | At cache-write time once Layer B exists: brute-force cosine vs the in-memory embedding index, add edges for cosine ≥ τ (top-k=4) | O(n·d) per write at Layer B's designed "few hundred" scale — Layer B's own DECART already accepted this cost for its lookup path | cosine similarity (∈ [τ, 1]) | **deferred to Layer B's build — the trigger is Layer B existing, not a size** |

The bottleneck answer the operator asked for, concretely: the ONLY work on the request path is
(b)'s window scan — ≤ 64 tuple pushes behind the observer's own mutex (not the store mutex), a
few hundred ns. Everything expensive (CSR build, normalization, PPR) happens in the **maintenance
pass** (§3.1.4), off the request path, full-rebuild-per-invocation like `rci derive` (§1.4).
The observation buffer is bounded: at E_MAX (256k tuples ≈ 6 MB) the oldest observations are
dropped after a rebuild snapshots them into a retained base graph — degrade-closed, no unbounded
growth (that would repeat P26 §1.4's leak shape).

**3.1.3 Relation combination.** v1 maintains ONE collapsed weighted graph
`W = Σ_r β_r · A_r` (β: co-access 1.0, derivation 2.0, semantic as-cosine — policy-as-data).
This is the mode-3 contraction of the (entry × entry × relation) tensor; the UNCOLLAPSED slices
are retained in the observation buffer's type tags so task 2's rung 2 can factorize them
per-relation (RESCAL) without re-collection. Undirected treatment throughout (both directions
pushed, per `csr.rs:72-74`'s documented convention).

**3.1.4 Query (recall).** The maintenance pass (triggered by the eviction path needing scores, or
a fixed cadence — consumer's choice):

1. `let g = Csr::from_edges(n, &edges)` — aggregation happens here.
2. `let a = g.row_normalize()` — dangling entries get the deterministic self-loop (`csr.rs:135-139`),
   so never-co-accessed nodes are well-defined: they hold only their teleport mass ⇒ rank lowest ⇒
   evict first, which is the correct semantics for an entry nothing relates to.
3. Seed `e` = uniform over the nodes currently in `recent` (deterministic; recency-decayed
   weighting is a named v2 knob, not v1).
4. `let pi = a.personalized_pagerank(&e, 0.15, 20)` — fixed α, fixed K, bit-reproducible.

`pi[i]` = "how much this entry matters to what the system is touching right now, given everything
it has ever related to." That is chronology + topology + recall in one number — the living-memory
§7 pattern, scoped to the cache.

**3.1.5 Consumption.**
- **Eviction/retention (primary):** P26-M2's `BoundedStore` gains a pluggable scorer: on
  over-budget, evict ascending by `(pi[i], last_access_ns)` — PPR-primary, LRU tie-break. If no
  fresh `pi` is available (graph too young, maintenance not yet run), pure LRU — the graph is
  advisory over an idempotent upstream (`cache.rs:9-11`), so a stale or absent score can cost a
  re-fetch, never correctness. This satisfies the same advisory-only asymmetry `CachePolicy`
  already encodes for Layer B.
- **Prefetch (secondary, later):** top-ranked non-resident nodes are prefetch candidates — the
  living-memory §7 use, listed as a consumer, not built in v1.
- **Task 2 input (structural):** the graph IS mode 1–2 of the tensor; `pi`, degree, and recency
  are rows of the coupled feature matrix Y (§3.2).

**3.1.6 The falsifier (honesty within the override).** P26 §2.5's evidence (no production lineage
for centrality-scored eviction) is answered, not ignored: W3 (§7) replays a recorded access log
against PPR-scored eviction vs plain LRU at equal byte budgets and reports both hit rates. If
PPR-scored ≥ LRU, it stays the default scorer. If it loses, eviction reverts to LRU **and the
graph remains** — its other two consumers (prefetch candidates, task-2 tensor input) and its
observability value (which entries relate to which) do not depend on winning the eviction
benchmark. Building the graph is the operator's direction; which policy reads it is decided by
the measurement.

### 3.2 Task 2 — hybrid tensor decomposition, planned forward (proposal)

**The object (per operator sharpening #1).** Not (entry × embedding-dim). The full object is:

- **𝓧 ∈ ℝ^(n×n×m)** — mode 1 and 2 = cache entries (shared identity), mode 3 = relation type
  {co-access, derivation, semantic}: the RESCAL shape exactly (§2.2), fed directly by task 1's
  type-tagged observation buffer (§3.1.3).
- **Y ∈ ℝ^(n×d)** — coupled side matrix over the SAME entry mode; columns = embedding components
  (when Layer B exists) ∥ graph-derived features (PPR score, degree, log access_count, recency,
  log bytes). Coupled through the shared factor per CMTF (§2.2): minimize
  `Σ_k ‖X_k − A R_k Aᵀ‖² + λ‖Y − A Vᵀ‖²` over A ∈ ℝ^(n×r), R_k, V. The output A is the
  compressed unified representation — r numbers per cache entry encoding relational position AND
  feature profile in one code.

**The ladder — each rung concrete, first rung buildable now (not deferred behind a trigger):**

| Rung | Condition | Object | Method | Code |
|---|---|---|---|---|
| 1 | **NOW** (m=1 effective, no embeddings) | symmetric co-access W | truncated eigendecomposition W ≈ U_k Λ_k U_kᵀ — the 2-way case of Tucker/PCA (Kolda–Bader, §2.2) | **new `kernel/src/lowrank.rs`** (below); basis cached in `spectral_cache::Decomp`'s existing empty slot |
| 2 | ≥2 real relation slices (derivation lands) | 𝓧 n×n×m | RESCAL ALS: X_k ≈ A R_k Aᵀ, shared A (Nickel et al., §2.2) | `lowrank.rs` extension; ALS = repeated least-squares over `Csr` products — fixtures written at rung 1 time so activation is a solver drop-in, not a redesign |
| 3 | Layer B embeddings exist | 𝓧 + Y coupled | CMTF (Acar et al., §2.2) — one more coupled block in the same ALS loop | same module; Y's embedding block enters through the identical shared-A normal equations |
| 4 | n ≥ 10⁵ or d·n ≥ 100 MB | storage of Y / A themselves | int8 SQ first (4×, ~99.3%, P26 §2.3), PQ after (32×, Faiss/Jégou, §2.2); randomized SVD (Halko) replaces power-deflation if n makes fixed-K iteration slow | P26 row 5's trigger, unchanged — quantization composes with rungs 1–3 |

**`kernel/src/lowrank.rs` (rung 1, the concrete unit):** deterministic top-k symmetric eigenpairs
by fixed-K power iteration + Hotelling deflation, over the existing allocation-free
`Csr::spmv`/`laplacian_spmv` kernels:

```rust
/// Top-k eigenpairs of a SYMMETRIC Csr, deterministic: fixed iteration count,
/// fixed summation order (inherited from spmv), deflation A := A − λ v vᵀ applied
/// implicitly (v stored, correction applied per spmv — the Csr is never densified).
pub fn topk_symmetric(a: &Csr, k: usize, iters: usize) -> (Vec<Vec<f64>>, Vec<f64>) // (basis, values) == spectral_cache::Decomp
```

- Determinism: fixed `iters`, deterministic start vector (index-graded, not random), fixed
  summation order ⇒ same contract as `personalized_pagerank`. No Durand-Kerner involvement.
- Return type IS `spectral_cache::Decomp` — the basis slot that has been empty since P11
  (`spectral_cache.rs:28,122`) gets its first real occupant; `DecompCache`'s recompute-counter
  falsifiers apply unchanged.
- Immediate real jobs (why rung 1 is not speculative shelf-ware): (i) k-dim spectral coordinates
  per cache entry — the first columns of Y, and a 2-D/3-D projection for observability; (ii)
  low-rank reconstruction error `‖W − U_k Λ_k U_kᵀ‖_F / ‖W‖_F` as a measured, falsifiable
  statement of how compressible the relation structure actually is — the number that decides
  whether rungs 2–3 will ever pay.

**DECART verdict (the direct question asked):** extending `spectral.rs`'s existing eigensolver to
serve as the compression primitive is **REJECTED on mechanism** — both its paths produce
eigenvalues only (§1.3) and the Durand-Kerner engine has no vector-recovery step; retrofitting one
(inverse iteration per root against a dense matrix) would be more code and O(n³) dense work versus
~120 LOC of sparse power-deflation reusing `spmv`. The **ADOPTED** minimal-new-code path is
`lowrank.rs` as a sibling: same module family, same determinism contract, same cache type,
zero deps — the honest sense in which "the existing machinery generalizes" is that `Csr::spmv` is
the inner loop of the new solver and `Decomp` is its output type, not that the existing root
finder can be bent into producing bases. A dedicated quantization primitive is NOT simpler for
the operator-directed object: quantization compresses stored vectors but cannot produce the
shared-entity factor A that unifies relations with features — it remains the complementary rung-4
track exactly where P26 left it.

### 3.3 Task 3 — `kernel/src/arena.rs`: the bump arena (proposal)

**Site selection (survey, honest):**

| Candidate site | Allocation churn found | Verdict |
|---|---|---|
| LLM dispatch I/O path | tens of allocations per multi-second call (P26 §1.3) | not the site (unchanged finding — but no longer used to reject the primitive) |
| P25 admission predicate | pure µs-scale predicate over procfs; module not yet built; no allocation loop in its design | not the site |
| P24 ring drainer | SPSC ring is preallocated by design (P24 blueprint); `bounded_drainer.rs:70` runs caller closures | not the site; closures may USE the arena |
| **Graph/spectral rebuild-and-rank pass** (task 1 §3.1.4 + `rci derive` + dense spectral) | `from_edges`: n bucket Vecs + n merge Vecs + 3 output Vecs ≈ **2n+7 allocations/rebuild** (n=1024 ⇒ ≈2,055); `row_normalize`: +3; `personalized_pagerank`: +4 (`csr.rs:79-115,125-152,228-247`); dense `charpoly` n>32: **≈ n²+O(n) allocations/call** via the `matmul` wrapper (`spectral.rs:35-39,125-135`; n=64 ⇒ ≈4.3k allocations, ≈2.2 MB churn); dense `Ppr::rank`: K `nxt` Vecs/call (`retrieval/ppr.rs:47`) | **CHOSEN** — repeated build-use-discard of same-shaped buffers, freed together at pass end: the textbook phase/region shape, now with a real recurring local-compute consumer |

**The primitive** — hand-rolled, zero-dep, ~80 LOC + tests, matching the repo's convention
(`token_bucket.rs`/`householder.rs` class):

```rust
/// Vec<u8>-backed bump region. Fixed capacity (policy-as-data), pointer-bump
/// alloc, O(1) reset. !Sync by construction (Cell) — one arena per pass/thread.
pub struct BumpArena {
    buf: std::cell::UnsafeCell<Vec<u8>>, // region; capacity fixed at construction, NEVER grows
    offset: std::cell::Cell<usize>,      // bump pointer
    high_water: std::cell::Cell<usize>,  // max offset ever reached (honest sizing telemetry)
}
impl BumpArena {
    pub fn with_capacity(bytes: usize) -> Self;
    /// Bump-allocate a zero-initialized slice. `T: Copy` ⇒ no Drop obligations,
    /// enforced by rustc (a Copy type cannot implement Drop) — the bumpalo
    /// no-Drop hazard is eliminated at COMPILE time, not by convention.
    /// Alignment: offset rounded up to align_of::<T>(). Returns None when the
    /// region is exhausted — DEGRADE-CLOSED: the caller falls back to a plain
    /// heap Vec (the `_in` wrappers do this internally); never grows, never panics.
    pub fn alloc_slice<T: Copy + Default>(&self, len: usize) -> Option<&mut [T]>;
    /// O(1): offset := 0. Takes &mut self, so the borrow checker PROVES no
    /// loans from alloc_slice are still alive — soundness by signature.
    pub fn reset(&mut self);
    pub fn high_water(&self) -> usize;
}
```

One `unsafe` block (pointer cast + write inside `alloc_slice`), documented with its two
invariants (alignment satisfied by construction; region exclusively owned via `UnsafeCell` with
lifetimes tied to `&self`, uniqueness of each slice guaranteed by the monotone offset) — the same
confinement discipline as `householder.rs`/`simd.rs` (§1.4).

**Integration — additive `_in` variants, existing signatures untouched:**
`Csr::from_edges_in(n, edges, &arena)`, `row_normalize_in`, `personalized_pagerank_in`, a
`matmul_contig_in` used by a charpoly scratch path, and `topk_symmetric` (§3.2) born arena-aware.
Each `_in` falls back to the heap on `None`. Task 1's maintenance pass owns one arena sized by
measured `high_water` + slack, `reset()` between passes.

**The claim, on its own terms (operator sharpening #2 — no relative-noise framing):**

> Per maintenance-pass invocation at n=1024, nnz≈8k: ≈2,062 malloc/free pairs (build + normalize
> + PPR) become **3–8 bump advances + one O(1) reset**. At a locally measured glibc malloc/free
> fast-path cost of 20–100 ns per pair *(order-of-magnitude prior; the benchmark, not this
> sentence, is the authority)*, expected saving ≈ **40–200 µs per invocation**, plus the
> unmeasured-but-real locality effect of same-pass buffers being contiguous ("things allocated
> close in time get allocated close in memory" is the property mimalloc's sharding chases —
> *(fetched, P26 §2.1)* — a bump region gets it by construction). For dense `charpoly` n=64:
> ≈4.3k pairs → arena, expected ≈ 90–430 µs per call. These numbers are the benchmark's
> null hypothesis, stated to be confirmed or refuted — not padding.

Done-check (§7 W5): criterion A/B on the existing harness — `graph_rebuild_rank/heap` vs
`graph_rebuild_rank/arena` — plus an allocation-count assertion via a counting test allocator
(≤ 8 heap allocations on the arena path vs ≈2k baseline), plus the determinism falsifier: PPR
output byte-identical arena vs heap (the arena moves where f64s live, never the operation order;
if this test ever fails the arena variant is abandoned for that call site — determinism outranks
the µs, same rule as P26 M4).

---

## 4. DECART

| # | Candidate | Native fit | Falsifiable correctness | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | **CacheGraph: co-access-primary reference graph, CSR + existing PPR** vs invent a new graph store vs no graph | `Csr`/PPR verbatim (`csr.rs:79-264`); duplicate-merge does the aggregation; observer is ~150 LOC beside `cache.rs` | window-scan cost measured (< 2 µs/access); PPR determinism inherited from existing tests; W3 hit-rate A/B vs LRU is the policy falsifier | ~150 LOC observer + ~60 LOC scorer seam; bounded buffers throughout | zero new deps | graph is advisory; scorer seam reverts to LRU by config; module deletable | **ADOPT.** New graph store REJECTED (csr.rs exists and is the operator-named pattern); no-graph REJECTED (it is the overridden verdict) |
| 2 | Co-access (b) as primary edge vs semantic-first (a) vs derivation-first (c) | (b) needs zero new infrastructure — the access stream exists today | (b) recordable + benchable immediately | (a) blocked on Layer B (no embedding matrix exists — grep-verified by P26 §1.7, unchanged); (c) blocked on provenance threading | — | β weights are policy-as-data | **(b) v1, (c) v1.1, (a) at Layer B's build** — order forced by which signals physically exist |
| 3 | **lowrank.rs top-k power+deflation over Csr::spmv** vs extend Durand-Kerner to vectors vs randomized SVD vs dense Jacobi eigensolver | reuses `spmv` inner loop + fills `Decomp`'s existing empty basis slot (`spectral_cache.rs:28,122`) | hand-oracle tests (known spectra: path/triangle/K₃ already used in csr.rs tests); reconstruction-error metric is self-reporting | ~120 LOC | zero deps | delete module; Decomp basis returns to empty | **ADOPT.** Durand-Kerner extension REJECTED (values-only engine, no vector path, dense O(n³)+ retrofit — §1.3); randomized SVD REJECTED-for-now (determinism contract; named rung-4 upgrade); dense Jacobi REJECTED (densifies the sparse graph) |
| 4 | **Tensor ladder: eigen (now) → RESCAL (m≥2) → CMTF (embeddings) → SQ/PQ (scale)** vs build full RESCAL/CMTF solver now vs P26's flat reject | ladder rung 1 is real code with a real job now; rungs 2–3 are shaped by data that task 1 already collects (type-tagged edges) | rung 1's reconstruction error quantifies compressibility BEFORE rungs 2–3 spend anything; each rung has fixtures written one rung early | rung 1 ≈ 120 LOC; rungs 2–3 ≈ ALS loops over the same kernels | zero deps | each rung independently abandonable | **ADOPT ladder.** Full solver now REJECTED (m=1 today — a RESCAL over one slice IS the eigendecomposition; building the general solver before a second slice exists produces untestable code); flat reject overridden per operator direction |
| 5 | **BumpArena hand-roll at the graph/spectral rebuild site** vs bumpalo dep vs Vec-reuse-only vs no arena | zero-dep hand-roll, unsafe confined per house precedent (§1.4); T: Copy kills the Drop hazard at compile time | criterion A/B + allocation-count assertion + byte-identical-output falsifier (§3.3) | ~80 LOC + `_in` variants | zero (bumpalo REJECTED: external dep for an ~80-LOC subset) | `_in` variants additive; heap fallback is the existing path; module deletable | **ADOPT.** Vec-reuse-only REJECTED as the whole answer (it cannot serve `from_edges`' n transient bucket Vecs or charpoly's conversion churn with one O(1) release); no-arena is the overridden verdict |
| 6 | Arena scope: rebuild/spectral pass only vs also the dispatch path | dispatch path allocates ~tens of objects per multi-second call (P26 §1.3, still true as *fact*) | — | wiring cost with no recurring loop to serve | — | — | **Rebuild/spectral pass only.** The dispatch path is excluded because it has no recurring same-shaped-buffer loop for a region to serve — a mechanism-fit reason, not a relative-latency reason |

**Mandatory probe (strongest honest argument against the plan):** the three designs could become
machinery that exists because machinery was ordered — the graph's eviction win is unproven
(§2.1), rung 1's compressibility number could come back "not compressible" (spectral mass spread
flat), and the arena's µs live inside passes that run on maintenance cadence, not per-request.
The answer is that each unit carries its own measured exit: W3's hit-rate A/B decides the
eviction policy (graph keeps its other consumers either way), rung 1's reconstruction error is
precisely the number that gates rungs 2–3 spend, and W5's criterion delta + allocation-count
assertion make the arena's value a printed number rather than a belief. Every unit is
independently deletable (DECART reversibility column) — the plan's worst honest outcome is three
small, tested, zero-dep primitives whose measurements argue against their own expansion, recorded
in BENCH_HISTORY.md.

## 5. 2-question doubt audit

**Q1 — least confident about (concrete):**
1. The co-access window parameters (N_RECENT=64, W=60 s, E_MAX=256k) are judgment values, not
   derived — all three are policy-as-data and the maintenance pass logs window-saturation stats
   so they can be tuned from evidence, but v1 ships them as guesses.
2. PPR-seeded-from-recents can starve a *periodically* valuable entry (accessed every N hours,
   never inside the window with anything else): it holds only teleport mass and ranks near
   isolated nodes. The LRU tie-break does not fully save it (its last_access is old by
   construction at eviction time). If W3's replay shows this class exists in real logs, the fix
   is a small frequency prior added to the seed (access_count-weighted) — named now so it is a
   knob, not a redesign.
3. `alloc_slice` returning `&mut [T]` from `&self` is the bumpalo pattern but the soundness
   argument (monotone offset ⇒ disjoint slices; `reset(&mut self)` ⇒ no live loans) must survive
   Miri, not just this paragraph — W5's done-check runs the arena tests under Miri.
4. Rung-2 RESCAL's ALS over an n×n×m tensor with the kernel's determinism contract (fixed
   iteration counts, fixed order) has no published deterministic-ALS precedent I could verify
   this session — the fixtures-one-rung-early rule exists precisely to surface this before
   activation **(training-knowledge boundary, flagged)**.
5. The 20–100 ns malloc/free prior is an order-of-magnitude estimate; if the real fast path on
   this host is faster, the arena's µs shrink accordingly. The claim's authority is explicitly
   the benchmark (§3.3); the risk is only to the *size* of the printed number, not to the
   primitive's correctness.

**Q2 — biggest thing possibly missed:** the graph observer records accesses for a cache that P26
M2 has not yet bounded — if this phase lands before M2, PPR scores exist with no eviction seam to
consume them. Sequencing answer: W1–W2 are independently useful (observability + rung-1
coordinates), and W4 (scorer seam) explicitly depends on P26 M2 in §7. Also possibly missed: a
second `CachingBackend` instance (the Dispatcher's shared handle, `cache.rs:26-29`) means the
observer must sit inside the shared `Arc`, not per-clone — the design places it beside the shared
store for exactly this reason, but the build must test the two-handle case (W1 done-check).

## 6. Anu / Ananke check

**Anu (derivable, not asserted):** node identity derives from a key the cache already computes;
edge aggregation derives from a documented existing behavior (`from_edges` duplicate-summing);
the query is an existing tested primitive, unmodified. The tensor object's shape derives from the
operator's directive grounded in a directly-read primary source (RESCAL's n×n×m with shared A),
and the rung-1-equals-eigendecomposition identity derives from Kolda–Bader's stated relationship
between Tucker and PCA in the two-way case. The eigenvector gap in spectral.rs is a live-read
fact (§1.3), which is what forces `lowrank.rs` to be new code rather than an extension — the one
place the operator's "extend the existing eigensolver" framing is answered with evidence rather
than compliance. The arena's claim chain is: counted allocation sites (live-read code) × measured
cost prior (flagged as prior) = expected saving, with the benchmark named as the authority.
Weakest links, named: the deterministic-ALS question (Q1.4) and the malloc-cost prior (Q1.5).

**Ananke (structural, not hoped):** the graph cannot corrupt correctness by construction — it
feeds an advisory scorer over an idempotent upstream, and the fallback on any absence/staleness
is the existing LRU path; `CachePolicy` already fences gate-critical callers from advisory
machinery. Boundedness is structural: `recent` is a capped VecDeque, `edges` has E_MAX with
snapshot-then-drop, the arena's region NEVER grows and its exhaustion path is the plain heap.
Determinism is enforced by inheritance (fixed-K PPR, fixed-order spmv, index-graded start
vectors) and by falsifier tests (byte-identical arena-vs-heap output; `DecompCache`'s recompute
counters), not by review vigilance. The `T: Copy` bound and `reset(&mut self)` signature make the
arena's two hazard classes (Drop leaks, use-after-reset) compile-time impossible rather than
convention. What is NOT structural yet, named: nothing prevents a future caller from consuming
`pi` on a gate-critical path — the grep-able convention ("CacheGraph output is advisory") plus
W4's review is the cheap version until a type-level fence earns its cost.

## 7. Build plan — falsifiable done-checks

| # | Unit | Depends on | Falsifiable done-check |
|---|---|---|---|
| W1 | `llm-adapters/src/cache_graph.rs` — `CacheGraph` observer (nodes, co-access window, edge buffer) wired into `CachingBackend` | — | Unit tests: interning is stable across re-insertion; window emits both edge directions; buffer honors E_MAX (snapshot-then-drop); two-handle (Dispatcher + Harness clone) test shows ONE shared graph; measured observer overhead < 2 µs/access (criterion) |
| W2 | Maintenance pass: `from_edges` → `row_normalize` → PPR → score vector; plus `kernel/src/lowrank.rs` `topk_symmetric` (rung 1) filling `spectral_cache::Decomp` | W1 | PPR scores byte-identical across two passes over identical buffers; `topk_symmetric` matches hand-derived spectra (triangle/path/K₃ oracles per csr.rs test convention); reconstruction-error metric printed for the fixture graph; `DecompCache` recompute falsifiers stay green with a non-empty basis |
| W3 | Eviction A/B: replay a recorded access log, PPR-scored vs plain LRU at equal byte budgets | W2, P26 M2 (`BoundedStore`) | Both hit rates recorded in BENCH_HISTORY.md; policy default set by the winner; the losing policy remains selectable (config), the graph remains regardless (§3.1.6) |
| W4 | Scorer seam in `BoundedStore` (`(pi, last_access)` ascending eviction; LRU fallback on absent scores) + `derived_from` provenance field (additive) on `ChatRequest` | W2, P26 M2 | Eviction test: with fresh scores, lowest-PPR evicts first; with no scores, behavior is bit-identical to plain LRU (regression test against M2's suite); `derived_from` default-empty breaks zero existing callers (compile + test suite green) |
| W5 | `kernel/src/arena.rs` `BumpArena` + `_in` variants (`from_edges_in`, `row_normalize_in`, `personalized_pagerank_in`, charpoly scratch path) | — (parallel lane) | Criterion A/B `graph_rebuild_rank` heap-vs-arena recorded in BENCH_HISTORY.md with the §3.3 claim confirmed or refuted; counting-allocator test: ≤ 8 heap allocations on the arena path (baseline ≈ 2k at n=1024); byte-identical PPR output arena-vs-heap; arena tests pass under Miri; `high_water()` reported |
| W6 | Rung-2/3 activation kit: RESCAL/CMTF fixtures + data-shape contracts (type-tagged slices from W1's buffer; Y-matrix layout) — code-ready design, solver lands when a second relation slice or embeddings exist | W2 | Fixtures compile and encode the m≥2 shapes; a failing-by-design `#[ignore]` test names the activation condition, so the path's existence is checked by CI, not by memory |

W1 ⊥ W5 (startable now, parallel); W2 after W1; W3/W4 gated on P26 M2; W6 after W2. All units
C-class at verification (cargo test/bench), D-class during authoring (P25 §6 classification).

---

## Appendix — phase-table registration

Registered in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8 as **Phase 28** (28
confirmed free at registration time: §8 ends at §8.9/P27; grep for `P28|Phase 28` returned
nothing — re-read fresh this session). Depends on: **26** (W3/W4 consume its M2 `BoundedStore`
seam; overrides three of its verdicts by operator direction — its addendum points here), soft on
**Layer B** (`HARNESS-LLM-BACKEND.md` §3.3 — edge type (a) and tensor rung 3 activate when it
builds), soft on **P24** (bench/telemetry surfaces). Off-critical-path lane, same class as
P5/P8/P11/P12/P24–P27.

---

## Addendum (2026-07-17, same day, later session) — rung-1 solver re-homed; `lowrank.rs` superseded

> Applies to §0.2, §1.3, §3.2 (including the "DECART verdict" paragraph), §4 row 3, and W2/W5.
> The original text above is preserved unedited per append-don't-rewrite; where it conflicts with
> this addendum, the addendum governs.

The operator asked the direct follow-up this blueprint's §3.2 DECART invited: is the right fix a
REFACTOR of the existing eigensolver rather than the parallel sibling module proposed here? The
answer, worked in full in
[`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`](BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md):

1. **The eigenvector need is confirmed** (that document §1) — this blueprint's premise stands.
2. **This blueprint's REJECT rationale was right about Durand-Kerner, wrong by omission about the
   Householder path.** "Both paths are eigenvalues-only … no vector-recovery step" (§1.3, §3.2) is
   a fact about the current code, not a mechanism limit: for QR-family solvers, eigenvector
   recovery IS a standard accumulation extension of the same iteration (LAPACK `DSTEQR`
   `COMPZ='V'`, fetched there). The rejection of the DENSE paths for the cache-graph consumer
   survives on corrected, stronger grounds: at this graph's own regime (n ≈ 10²–10³, sparse,
   symmetric, top-k) the n ≤ 32 path can't hold the matrix and the Faddeev path can't deliver
   reliable eigenvalues at all (O(n⁴) cost + charpoly conditioning + densification) — see that
   document §3.4.
3. **Supersession:** `kernel/src/lowrank.rs` is NOT built. The rung-1 solver — algorithm,
   determinism contract, `Decomp` return type, reconstruction-error metric, arena-awareness, all
   exactly as specified in §3.2 — lands as **`spectral::topk_symmetric(&Csr, k, iters)`**, a third
   tier inside the existing routing façade (`spectral.rs:195-214`'s dispatch pattern), so the repo
   keeps ONE public eigen surface. Additionally `householder.rs` gains an additive symmetric
   dense decomposition `eigh_contig` (reflector + Givens accumulation, DSTEQR shape, n ≤ 32) —
   serving the field-UI/mesh Laplacian-mode consumers and acting as the sparse solver's dense
   parity oracle. Zero existing call sites change; existing test suites untouched.
4. **Read-through for W2/W5:** every reference to `lowrank.rs`/`lowrank::topk_symmetric` in §3.2,
   §4 row 3, W2, and W5 reads as `spectral.rs`/`spectral::topk_symmetric`. W2 additionally
   inherits that document's §5.4 test plan (KAT `A·v = λ·v`, orthonormality, sparse-vs-dense
   parity, byte-determinism, reconstruction-error monotonicity). Rungs 2–4 of the ladder are
   unaffected.

---

## 8. BumpArena swarm-dispatch readiness — added 2026-07-18

> Re-verified live this pass (2026-07-18), not inherited from the 2026-07-17 text above. This
> section does not rewrite §3.3/§4 row 5/§7 W5 — it packages them into dispatch-ready form and
> records what has drifted since authoring.

### 8.1 Role & responsibility

`BumpArena` is a zero-dependency, hand-rolled bump/region allocator — a single `Vec<u8>` backing
region with a monotonically-advancing `Cell<usize>` offset — that exists to remove the per-rebuild
malloc/free churn at the graph/spectral rebuild hot path (§3.3's chosen site, re-verified live
below). It does O(1) allocation by pointer-bump and O(1) bulk deallocation by resetting the offset
to zero, is restricted to `T: Copy` so the bumpalo-class no-Drop hazard is a compile error rather
than a runtime convention, and degrades closed on exhaustion (falls back to a plain heap `Vec`,
never grows, never panics) — it does not become a new failure mode, only a fast path that can
silently stop being fast. It plugs into the maintenance-pass call sites task 1/2 of this blueprint
create or already touch, verified live this pass:
- `kernel/src/csr.rs:79-115` `Csr::from_edges` — n-row bucket sort + duplicate-merge-by-summing at
  `csr.rs:93-103` (confirmed live, byte-identical to the doc's own description).
- `kernel/src/csr.rs:125-152` `Csr::row_normalize` — deterministic dangling-node self-loop
  (confirmed live, exact line match to §1.4/§3.3's citation).
- `kernel/src/csr.rs:280-316` `Csr::personalized_pagerank` — fixed-K synchronous Jacobi (the
  behavior this blueprint's §1.2/§3.3 cites as `csr.rs:228-264` has **drifted ~52 lines**: live
  grep places the function at 280-316, not 228-264, because `to_adjacency` and `energy`
  (`csr.rs:239-264` in the live file) were inserted between `row_normalize` and
  `personalized_pagerank` sometime after this blueprint was authored. The allocation shape and
  determinism contract are unchanged — only the citation is stale. Fix the line numbers in §1.2/§3.3
  the next time this file is touched for substance, not as part of this append-only pass.)
- `kernel/src/spectral.rs:35-39` the `matmul` wrapper (`Mat::from_vecvec` ×2 + `matmul_contig` +
  `into_vecvec` per call) and `kernel/src/spectral.rs:113-137` `charpoly` (Faddeev-LeVerrier, n−1
  calls into `matmul`) — both confirmed live, exact line match, ≈n²+O(n) transient allocations per
  `charpoly` call for n > 32 unchanged.
- `kernel/src/retrieval/ppr.rs:47` the per-iteration `let mut nxt = vec![0.0f64; self.n];` inside
  `Ppr::rank`'s k-loop — confirmed live, exact line match.

### 8.2 Definition of DONE — falsifiable, numbered

1. `kernel/src/arena.rs` exists and implements the exact API §3.3 already specifies — every
   signature, verbatim:
   - `pub struct BumpArena { buf: std::cell::UnsafeCell<Vec<u8>>, offset: std::cell::Cell<usize>, high_water: std::cell::Cell<usize> }`
   - `pub fn with_capacity(bytes: usize) -> Self`
   - `pub fn alloc_slice<T: Copy + Default>(&self, len: usize) -> Option<&mut [T]>` — zero-initialized,
     alignment rounded to `align_of::<T>()`, `None` on exhaustion (never grows, never panics).
   - `pub fn reset(&mut self)` — O(1), `&mut self` so the borrow checker proves no live loans.
   - `pub fn high_water(&self) -> usize`
   - Exactly one documented `unsafe` block inside `alloc_slice` (pointer cast + write), matching
     the confinement discipline already in-convention at `householder.rs`/`simd.rs`/`messenger.rs`.
2. It is **wired into the real call sites**, not standalone: additive `_in` variants exist and are
   exercised by the maintenance pass —
   `Csr::from_edges_in(n, edges, &arena)`, `Csr::row_normalize_in`, `Csr::personalized_pagerank_in`,
   a `matmul_contig`-based charpoly scratch path taking an arena, and the eigenvector rung-1 solver
   (per the addendum above: `spectral::topk_symmetric`, NOT `lowrank.rs`) born arena-aware. Every
   `_in` variant falls back to the pre-existing heap path on `None` — existing non-`_in` signatures
   are untouched (additive only, per §3.3's "Integration" note).
3. The named falsifiers from §3.3/§7 W5 exist and pass:
   - Criterion A/B benchmark group `graph_rebuild_rank` with `heap` and `arena` sub-benchmarks
     (i.e. `graph_rebuild_rank/heap` vs `graph_rebuild_rank/arena`) in the existing
     `kernel/benches/criterion.rs` harness (confirmed live: this file and `criterion = "0.5"`
     dev-dependency already exist, `kernel/Cargo.toml:83,88` — no new bench infra needed), with the
     result recorded as a new row in the existing `kernel/benches/BENCH_HISTORY.md` (confirmed live,
     already present).
   - A counting-allocator test asserting ≤ 8 heap allocations on the arena path for the n=1024
     maintenance-pass rebuild, against the ≈2,055-allocation heap baseline this blueprint computed
     (§0.3/§3.3) — the baseline number itself must be re-measured, not assumed, since it is
     currently a hand count over the code shown in §8.1, not an instrumented run.
   - A determinism falsifier: PPR output byte-identical arena-vs-heap on the same fixture graph. Per
     §3.3's own rule, if this ever fails the arena variant is abandoned for that call site —
     determinism outranks the microseconds.
   - The arena's own unit tests (bump/reset/exhaustion/alignment) pass under Miri (§5 Q1.3's
     explicit ask — the soundness argument for `&mut [T]` from `&self` "must survive Miri, not just
     [the] paragraph").
   - `high_water()` is exercised and its reported value used to size the arena's real capacity
     constant (§3.3's "sized by measured `high_water` + slack").

### 8.3 Definition of NOT-done / explicit anti-scope

1. **`arena.rs` existing and compiling but not wired into the CSR-rebuild/dense-charpoly call
   sites is NOT done.** An allocator module with passing unit tests and zero real callers proves
   nothing about the ≈2k-malloc-pair claim this blueprint exists to make — see DoD item 2. This is
   the single most common false-completion trap for allocator/arena work: a clean, well-tested
   module sitting unused reads as "done" in a diff but changes nothing at the hot path it was built
   for.
2. **Swapping in `bumpalo` (or any general-purpose arena crate) instead of the zero-dep hand-rolled
   version is scope regression, not progress**, and directly contradicts this blueprint's own
   DECART verdict: §4 row 5 explicitly evaluated "BumpArena hand-roll ... vs bumpalo dep" and
   rejected the dependency — "zero (bumpalo REJECTED: external dep for an ~80-LOC subset)" — because
   the repo's zero-external-dep-in-kernel convention (§1.4) and the small size of the needed subset
   both favor the hand-roll. Reaching for `bumpalo` under time pressure produces a worse outcome
   than this blueprint already rejected once.
3. **Building `kernel/src/lowrank.rs` is NOT part of this item's done-criteria.** It is a separate
   P28 sub-item (rung 1 of the tensor ladder, §3.2) that this same file's own later addendum
   (2026-07-17, "rung-1 solver re-homed") explicitly supersedes: `lowrank.rs` is **not built**; the
   rung-1 solver lands as `spectral::topk_symmetric` instead. Live-verified this pass (2026-07-18):
   neither `kernel/src/lowrank.rs`, nor `spectral::topk_symmetric`, nor `householder::eigh_contig`
   exist yet (`find`/`grep -rn` over `kernel/src/` all empty). `MASTER-ROADMAP-SOVEREIGN-
   ARCHITECTURE-2026-07-16.md` §8.12 (P30 row) states this in one line: "P28 — co-owned substrate:
   P30 W2 builds P28's `arena.rs` and rung-1 solver per the eigenvector-refactor plan (**no second
   arena, no lowrank.rs**)." Do not conflate the two sub-items' DoD, and do not build `lowrank.rs`
   under any name — it was designed and then explicitly retired within this same document.
4. **Skipping the Miri run or the byte-identical determinism falsifier because the criterion
   benchmark alone looks good is NOT done.** §3.3/§7 W5 require BOTH a speed proof (criterion A/B +
   allocation-count assertion) AND a correctness proof (Miri clean + byte-identical output) before
   this item counts as closed — a fast arena that silently reorders or corrupts f64 accumulation is
   a worse outcome than no arena, per the doc's own "determinism outranks the µs" rule (§3.3, §6
   Ananke).
5. **Treating the ≈2,055-allocations/≈40–200 µs claim as already-proven is NOT done.** §3.3 is
   explicit that this is "the benchmark's null hypothesis... not padding" and the malloc/free 20–100
   ns figure is an "order-of-magnitude prior" (§5 Q1.5) — the criterion A/B result is the actual
   authority, and a result outside that range does not itself invalidate the arena (only the
   expected-saving sentence).
6. **Widening scope to the LLM-dispatch I/O path is explicitly out of scope** (§4 row 6: "Rebuild/
   spectral pass only... the dispatch path is excluded because it has no recurring same-shaped-
   buffer loop for a region to serve").

### 8.4 Context & docs

- [BLUEPRINT-P-A-kernel-primitives.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md)
  §1 ("P-A cites-and-sequences but does not restate... the BumpArena (W2-L1, fully designed in
  `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` §3.3)... Their designs are decided;
  re-deriving them here would violate the standard's own reuse rule") and §4.4 ("Non-contradiction
  with the eigenvector plan" — the hard sequencing constraint: A4's `eig_hessenberg` dedup must land
  before W2-L2 `eigh_contig` starts; `topk_symmetric`/`eigh_contig` signatures are untouched by P-A).
- `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.12 (P30 row) — co-ownership: "P30 W2
  builds P28's `arena.rs` and rung-1 solver per the eigenvector-refactor plan (no second arena, no
  lowrank.rs)."
- [CORE-ROADMAP-INDEX.md](CORE-ROADMAP-INDEX.md) Layer A row (P04 · P11 · P28 · eqc-rs wiring ·
  this file) and Layer B row (P28 snapshot seam, P30 W1-L2/L11) — the two altitude rows this item
  is indexed under.
- [BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md](BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md)
  §5 — the sibling design for the rung-1 solver this arena item must stay arena-aware for, without
  building it itself.
