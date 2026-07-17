# BATCH 1 — Kernel-Level Tensor / Memory Primitives: Research + Audit Findings (2026-07-17)

> **Research + audit artifact, NOT a blueprint.** Produced under `AGENTS.md` §"Detailed Planning
> Protocol" (ground-truth-first) and §"Global doctrine — Anu (logic) & Ananke (organization)": every
> load-bearing claim carries a live `file:line` cite, a live command result, or a primary/blueprint
> citation; no mythological or ecological metaphor framing (MEMORY standing directive 2026-07-17).
> A later Fable synthesis pass turns these verdicts into an executable blueprint — this file does not
> pre-empt that, it feeds it.
>
> **Epistemics tags** (mirroring `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md:7-9`):
> **[MEASURED]** = probe/live-read run this session on this host; **[GROUNDED]** = external source or
> in-repo `file:line` fact, cited; **[SPECULATIVE]** = brainstorm/analytic, not decided, honestly
> assessed.
>
> **Rejection rule for THIS arc (operator override, source-prompt lines 22-23):** complexity, rewrite
> size, and "over-engineering" are **explicitly NOT valid rejection grounds**. A concept is rejected
> ONLY on correctness / physics / determinism-contract / absent-hardware grounds. Where a concept is
> genuine but has no consumer at today's data scale, the verdict is **DEFER-WITH-FALSIFIABLE-TRIGGER**
> (a measurable condition that flips it to ADOPT), never "too complex."
>
> **Cluster scope:** the smallest kernel abstractions — sparse tensor graphs, Z-order/Morton,
> branchless programming, SoA/AoS, cache-line alignment, software prefetch, HugePages/THP, arena
> allocators, tiling, the token-stream "mipmap" analogy — plus the Part-B signal-processing/
> tensor-geometry addendum (Laplace/Z-transform properties, dimensionality-reduction table,
> Nyquist-Shannon).

---

## 0. Executive summary

The single most important finding: **the dowiz kernel already embodies, as shipped and tested code,
the *data-oriented* half of the dialogue's memory-wall program** — contiguous single-buffer storage
(`mat.rs`, `csr.rs`), genuine SoA-across-batch SIMD with sentinel padding (`simd.rs`), FMA-fused
inner kernels with runtime feature detection (`householder.rs`), allocation-free hot loops
(`field_frame.rs`, `csr::laplacian_spmv`), content-addressed caching (`spectral_cache.rs`,
`cache.rs`), and a **trait-as-port deferral template for host-specific locality machinery**
(`core_pinning.rs`) that is the exact shape every deferred item below should copy. What is genuinely
**absent** — verified by a zero-match grep across `kernel/src` + `engine/src` — is the *OS/hardware*
half: no `mmap`, no `madvise`/`MADV_HUGEPAGE`, no `#[repr(align(…))]`, no `_mm_prefetch`, no Morton/
Z-order, no arena. [MEASURED, §1.6]

Therefore most dialogue concepts are **EXTEND-EXISTING or ALREADY-EQUIVALENT**, a few are genuine
tools with no consumer at today's `n≈10²–10³` scale (**DEFER-WITH-FALSIFIABLE-TRIGGER**), and only the
continuous-time transforms are **REJECT** (domain-mismatch: a discrete deterministic kernel has no
continuous s-plane implementation surface). The one **arena** concept the dialogue emphasizes is
already fully designed in `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` §3.3 (Phase 28)
— the dialogue **strengthens** it by supplying the HugePage-backing and Morton-tiling extensions that
blueprint does not have; it does not conflict with it. [GROUNDED, §1.5]

### Verdict table (detail in §2/§3)

| # | Concept | Verdict | Lands at / exists at |
|---|---|---|---|
| A1 | Sparse tensor graph — COO-ingest→CSR-compute, canonical ordering + hashing | **EXTEND-EXISTING** | `csr.rs:79-115`, `spectral_cache.rs:98`, 3-way = Phase-28 rung 2 |
| A2 | Z-order / Morton indexing | **DEFER-WITH-TRIGGER** | no consumer; trigger = blocked traversal over >L2 grid |
| A3 | Branchless (cmov / masking / sentinel padding) | **ALREADY-EQUIVALENT (padding) + EXTEND (CSR loops)** | `simd.rs:79-90,118-124`; extend to `csr.rs:175-183` |
| A4 | Structure-of-Arrays vs Array-of-Structures | **ALREADY-EQUIVALENT (simd) + REAL FINDING (zerocopy mislabel)** | `simd.rs:44-149`; bug-adjacent `engine/src/zerocopy.rs:22` |
| A5 | Cache-line alignment `#[repr(align(64))]` | **DEFER-WITH-TRIGGER** | zero today; trigger = multi-thread shared write OR measured aligned-load win |
| A6 | Software prefetch (`_mm_prefetch`) | **DEFER-WITH-TRIGGER** | SpMV gather `csr.rs:180-183`; trigger = nnz working set > L2 |
| A7 | HugePages / THP (`madvise MADV_HUGEPAGE`) | **DEFER-WITH-TRIGGER (via port template)** | copy `core_pinning.rs:41-64`; trigger = persistent arena > 2 MB |
| A8 | Arena allocator | **ADOPT (build Phase-28 arena.rs) + EXTEND (HugePage hook)** | `BLUEPRINT-CACHE-…-ARENA §3.3` (designed, not built) |
| A9 | Tiling (2D/3D block layout to page size) | **DEFER-WITH-TRIGGER** | naive GEMM `mat.rs:93-112`; trigger = dense matmul n≥128 on a hot path |
| A10 | Token-stream "mipmap" / hierarchical compression | **EXTEND-EXISTING** | spectral low-rank ladder (Phase-28 rung 1) + `chunker.rs` dedup |
| B11 | Laplace transform of Dirac delta `L{δ}=1` | **REJECT (domain-mismatch)** | continuous s-plane; no discrete surface |
| B12 | Dim-reduction table (PCA / t-SNE / UMAP / Isomap / LLE) | **SPLIT: PCA ADOPT-as-existing; Isomap DEFER; t-SNE/UMAP/LLE-iter REJECT (determinism)** | PCA = `spectral::topk_symmetric` (eigenvector blueprint) |
| B13 | Z-transform integration property `Σx[k]↔X(z)·z/(z−1)` | **EXTEND-EXISTING (analytic lens)** | drift boundary `spectral.rs:319`; accumulators `budget.rs:110` |
| B14 | Nyquist–Shannon sampling `fs≥2B` | **EXTEND-EXISTING (real bound to add)** | 50 Hz clock `lib.rs:219`, `field_frame.rs:51` |
| B15 | Laplace integration property `∫f↔F(s)/s` | **REJECT (domain-mismatch), redirect to B13** | discrete counterpart is B13 |
| B16 | Kuen surface (const. negative curvature) | **REJECT-as-decorative** | no hyperbolic-geometry consumer |

---

## 1. Ground truth (live-read this session)

### 1.1 Contiguous single-buffer storage is already the binding convention

- `mat.rs` — `Mat { nrows, ncols, data: Vec<f64> }`, row-major, element `(i,j)` at `data[i*ncols+j]`
  (`mat.rs:16-21,56-64`). The module doc states the intent verbatim: "For the data-oriented + SIMD
  invariant we want a single contiguous `Vec<f64>` laid out row-major so a matmul walks linear memory
  and auto-vectorizes… This module is the ONE backing store and the ONE matmul implementation"
  (`mat.rs:4-8`). [GROUNDED]
- `csr.rs` — `Csr { row_ptr, col_idx, val }`, "one contiguous `val` + parallel `col_idx`… the CSR
  analogue of `mat.rs`'s one-contiguous-buffer invariant" (`csr.rs:26-28,45-54`). "Single contiguous
  backing store — cache-friendly, SIMD-ready" (`csr.rs:42-44`). [GROUNDED]
- `householder.rs` — `Matrix32x32 { data: [f64; 1024] }`, stack-resident, "gives the compiler a fixed
  stride so LLVM auto-vectorizes and the data sits cache-contiguous" (`householder.rs:77-83`).
  [GROUNDED]

**Consequence:** the dialogue's "use contiguous arrays, not pointer-chasing" is not a proposal here —
it is the established invariant, with `Vec<Vec<f64>>` retained only as a boundary-compat shim
(`mat.rs:66-87`, `spectral.rs:32-39`).

### 1.2 CSR canonical ordering + content-addressed hashing already exist

- Canonical ordering: `Csr::from_edges` sorts `col_idx` ascending within each row and **merges
  duplicate `(src,dst)` pairs by summing weights** (`csr.rs:92-103`) — deterministic, idempotent,
  HashMap-free ("no iteration-order hazard", `csr.rs:36-37,80`). [GROUNDED]
- Content-addressed hashing over a canonical byte layout: `matrix_content_address` = FNV-1a over
  row-index-framed `f64::to_bits` (`spectral_cache.rs:98-112`); and the LLM cache key is sha3-256 over
  the BTreeMap-canonical request (`llm-adapters/src/cache.rs:57-81`, per Phase-28 §1.1). [GROUNDED]
- The dialogue's "COO for construction, CSR for compute" split is **already the shape**: `from_edges`
  ingests `(src,dst,w)` tuples (a COO triplet stream) and emits CSR; there is no separate COO struct
  because the edge-tuple `&[(usize,usize,f64)]` *is* the COO ingest contract, shared with
  `incidence.rs` (`incidence.rs:20-21,49`). [GROUNDED]

### 1.3 SIMD / branchless / FMA are already live, bit-identity-tested

- `simd.rs` — genuine **SoA-across-batch**: "vectorise ACROSS the batch of independent rows, never
  WITHIN a single row's reduction" (`simd.rs:4-9`); AVX2 `_mm256_*` lane, runtime `is_x86_feature_
  detected!("avx2")` with scalar fallback (`simd.rs:57-180`); **sentinel padding** (`-inf` for the max
  pass `simd.rs:79-90`, `0.0` for the sum pass `simd.rs:118-124`) so inactive lanes cannot contaminate
  a reduction; bit-identical to scalar, asserted `assert_eq!` on f64 bits (`simd.rs:218-232`).
  [GROUNDED]
- `householder.rs` — FMA-fused dot via `_mm256_fmadd_pd`, runtime `is_x86_feature_detected!("fma")`,
  scalar fallback, `unsafe` confined and documented (`householder.rs:29-75`). [GROUNDED]
- `unsafe` exists in exactly four files: `messenger.rs`, `householder.rs`, `simd.rs`,
  `engine/src/zerocopy.rs` [MEASURED, grep §1.6] — confirming Phase-28 §1.4's "confined unsafe
  precedent."

### 1.4 Allocation-free hot loops are already the pattern

- `field_frame::step` — pre-allocated `lap_scratch` + `next_scratch` in `new` (`field_frame.rs:
  125-146`), three-buffer rotation via `std::mem::swap` (no alloc, no drop) (`field_frame.rs:180-185`),
  bit-identical falsifier + 1000-step endurance test (`field_frame.rs:375-409`). [GROUNDED]
- `csr::laplacian_spmv` — "No heap is touched in the per-edge accumulation loop (only an O(n) degree
  scratch is allocated once, before the loop)" (`csr.rs:304-359`). [GROUNDED]
- `csr::spmv` — caller-owned `out`, overwritten in place, fixed summation order (`csr.rs:166-185`).
  [GROUNDED]
- `incidence.rs` (in the `/root/dowiz-spectral-evolution` worktree, commit `6bd181a02` — **NOT on the
  current `feat/harness-llm-backend` branch** [MEASURED, §1.6]) — fused grad→weight→div in one pass
  over edges, "no intermediate alloc" (`incidence.rs:107-117`); the canonical `+(D−A)` reference all
  other Laplacians parity-check against.
- **Counter-example (the real allocation churn):** `spectral::matmul` converts `Vec<Vec<f64>>⇄Mat`
  twice per call (`spectral.rs:35-39`); `charpoly` calls it `n−1` times ⇒ ≈`n²+O(n)` transient
  allocations per call for `n>32` (`spectral.rs:113-137`; Phase-28 §1.3 puts n=64 at ≈4.3k allocs).
  This is the arena's primary in-kernel consumer. [GROUNDED]

### 1.5 The deferral template already exists: `core_pinning.rs`

`core_pinning.rs` ships **only a trait-as-port seam** (`CorePinning`) with a zero-cost
`NoOpCorePinning` default, and an inline DECART: "on a single-socket host there is no locality to
exploit, so a pinner would be a guaranteed no-win… When a multi-socket / NUMA host appears, swap in a
real impl behind this same trait — the call sites do not change" (`core_pinning.rs:8-14,54-58`).
[GROUNDED] **This is the exact shape items A5/A6/A7/A9 below should take** — a named port, a NoOp
default, a documented falsifiable trigger — so "the birds can fly later" without a rewrite. The same
Trait-as-Port pattern is used for compute offload in `budget.rs:65-77` (`JobPort` / `OfflineJobPort`,
fail-closed).

### 1.6 What is genuinely absent (zero-match grep, this host)

```
grep -rn --include=*.rs -e repr(align -e UnsafeCell -e Bump -e prefetch -e Morton \
     -e madvise -e mmap kernel/src engine/src   →   NO MATCHES  [MEASURED]
grep -rn --include=*.rs "#[repr" kernel/src engine/src            →   NO MATCHES  [MEASURED]
find . -name incidence.rs (this branch)                           →   absent; only in
                                                    /root/dowiz-spectral-evolution  [MEASURED]
ls kernel/benches/                → criterion.rs, BENCH_HISTORY.md, baseline.json,
                                    bench_track.py; criterion="0.5" (Cargo.toml:66)  [MEASURED]
```

So: **no OS/hardware memory primitive exists yet**, and the criterion bench harness DOES exist — every
"measurable" verdict below is executable on the present harness without new infrastructure.

### 1.7 Relationship to the two already-landed blueprints (do not duplicate)

- `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` (Phase 28) already ADOPTS and designs:
  the **CacheGraph** (co-access CSR + existing PPR), the **hybrid tensor-decomposition ladder**
  (eigen → RESCAL → CMTF → SQ/PQ), and the **BumpArena** (`kernel/src/arena.rs`, ~80 LOC, `UnsafeCell`
  + `Cell<usize>` offset + `T: Copy` bound, degrade-closed, `_in` variants) — operator overrode three
  P26 REJECTs to get there. None of it is built yet (grep §1.6). [GROUNDED]
- `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` re-homes the low-rank solver to
  **`spectral::topk_symmetric(&Csr,k,iters)`** (sparse tier) + **`householder::eigh_contig`** (dense
  n≤32, DSTEQR-shape, reflector+Givens accumulation), one public eigen surface, `lowrank.rs`
  superseded. Decided; this batch **extends**, does not re-litigate. [GROUNDED]

My cluster's job is to determine whether the dialogue's sparse-tensor/Z-order/HugePages/branchless
ideas **strengthen, extend, or conflict** with that. Finding: they **strengthen** (A8 arena gains a
HugePage hook, A2/A9 give the arena a tiling story) and **extend** (A1/A10/B12-PCA feed the tensor
ladder); **none conflict**. The only overlap to avoid duplicating is the arena itself (A8 = "build
Phase-28's arena," not "design a new one").

---

## 2. Part A concept verdicts

### A1 — Sparse tensor graph: COO-ingest→CSR-compute, canonical ordering + hashing → **EXTEND-EXISTING**

**What exists:** CSR with canonical per-row ordering + duplicate-merge-by-sum (`csr.rs:92-103`),
content-address hashing (`spectral_cache.rs:98`), COO-triplet ingest via `from_edges` (§1.2).
[GROUNDED]

**What the dialogue adds that is genuine:** the **3-way** sparse tensor `𝓧 ∈ ℝ^(n×n×m)`
(entity × entity × relation) is a real generalization CSR does not natively hold — and it is
**already the object Phase-28 rung 2 designs** (RESCAL `X_k ≈ A·R_k·Aᵀ`, `BLUEPRINT-CACHE-…-ARENA
§3.2`, fed by type-tagged edge slices `§3.1.3`). The mesh reuse: a Bebop2 "DecisionUnit" state is a
relational object (agent × agent × interaction-type) with the identical shape — it should be stored as
type-tagged CSR slices and content-addressed by `matrix_content_address` exactly as `DecompCache`
does, so identical mesh states dedup for free. [SPECULATIVE — mesh mapping; GROUNDED — the primitive]

**Verdict:** EXTEND-EXISTING. Build nothing new for the 2-way case (it is `Csr`); the 3-way slice
tensor lands as Phase-28 rung 2, keyed by the existing content-address. **No COO struct is warranted**
— the edge-tuple contract is the COO layer. Not rejected (correct + has a consumer).

### A2 — Z-order / Morton indexing → **DEFER-WITH-FALSIFIABLE-TRIGGER**

**Physics:** Morton (bit-interleaved) ordering improves cache locality for 2D/3D **blocked** traversal
by keeping spatially-near cells memory-near. [GROUNDED, standard]

**Ground truth:** the only 2D structure is the field grid, traversed **row-major** in a 5-point
stencil (`field_frame.rs:106-116`, `i = r*w + c`). A row-major stencil already reads contiguous rows;
Morton pays only when (a) the grid exceeds cache so row-major suffers capacity misses **and** (b) a
cache-blocked/tiled traversal is introduced (A9). Neither exists. [MEASURED]

**Verdict:** DEFER. **Trigger (measurable):** field grid working set `w*h*4 bytes > L2` (≈ >256×256 at
f32) AND a blocked stencil is adopted — then a criterion A/B of row-major vs Morton on
`laplacian_into` decides. Not a correctness issue; not rejected on complexity — deferred on absent
consumer with a numeric trigger.

### A3 — Branchless programming (cmov / bitwise masking / sentinel padding) → **ALREADY-EQUIVALENT (padding) + EXTEND-EXISTING (CSR loops)**

**Already present:** sentinel padding is live and proven in `simd.rs` — `-inf` and `0.0` sentinels so
inactive SIMD lanes cannot contaminate a reduction (`simd.rs:79-90,118-124`), with a bit-identity test
(`simd.rs:218-232`). That is precisely the dialogue's "sentinel padding" technique. [GROUNDED]

**Genuine extension:** the CSR hot loops carry data-dependent branches — `spmv`'s `if xi == 0.0
{ continue }` (`csr.rs:175`) and `matmul_contig`'s `if aik == 0.0 { continue }` (`mat.rs:102`). These
are *sparsity skips*, usually well-predicted, but a branchless masked-accumulate variant is a genuine,
measurable option **subject to one hard constraint:** the determinism contract requires bit-identical
f64 output and fixed summation order (`csr.rs:31-34,226-227`). A branchless rewrite must preserve exact
op order — `simd.rs` proves this is achievable and testable. [GROUNDED]

**Verdict:** ALREADY-EQUIVALENT for padding; EXTEND-EXISTING for explicit branchless-masking of the
CSR/GEMM skips, gated by a byte-identical falsifier (same rule `simd.rs` already meets). Not rejected.

### A4 — Structure-of-Arrays vs Array-of-Structures → **ALREADY-EQUIVALENT (simd) + REAL FINDING (zerocopy mislabel)**

**SoA done right:** `simd.rs` is genuine SoA-across-batch (§1.3). [GROUNDED]

**Real finding (bug-adjacent, surface for the synthesis pass):** `engine/src/zerocopy.rs:22` documents
the particle buffer as *"SoA-record layout: `[x, y, vx, vy, life]` contiguous per particle (stride
5)."* A layout that stores each particle's fields **interleaved** at `stride 5` (`zerocopy.rs:22-23,
43,63`) is **Array-of-Structures (AoS), not SoA** — SoA would store all `x`s contiguous, then all
`y`s. The label is inverted. [MEASURED] This is currently harmless (the GPU vertex-upload path may
legitimately want interleaved attributes), but it is a correctness-of-documentation defect and it
matters the moment a SIMD particle integrator is built — the `simd.rs` doc already names exactly that
future consumer: *"The N-courier Kalman SoA consumer… is a TODO — the `f64x4` lane primitive here is
exactly the substrate it needs"* (`simd.rs:20-24`). A true-SoA transpose is required there.

**Verdict:** ALREADY-EQUIVALENT for the batch-SIMD lane; **fix the `zerocopy.rs` comment** (AoS, not
SoA) and, when the SIMD Kalman integrator lands, store its per-courier state true-SoA. Genuine reuse,
plus one honest doc fix. Not rejected.

### A5 — Cache-line alignment `#[repr(align(64))]` → **DEFER-WITH-FALSIFIABLE-TRIGGER**

**Physics:** `repr(align(64))` earns its keep in two situations — (a) preventing **false sharing**
when two threads write adjacent fields in one cache line, and (b) enabling aligned SIMD loads
(`_mm256_load_pd`) instead of unaligned. [GROUNDED]

**Ground truth:** (a) the hot passes are single-threaded per invocation — PPR (`csr.rs:228-264`),
spectral, `field_frame::step` all run on one thread — so there is **no false-sharing surface** today.
(b) the SIMD kernels deliberately use **unaligned** loads (`_mm256_loadu_pd`, `simd.rs:80,88,108`;
`householder.rs:37,43`); on Haswell-and-later x86 an unaligned load that does not cross a cache line is
as fast as an aligned one, so aligning buys ≈0 measured. [MEASURED — loadu confirmed; GROUNDED — the
perf equivalence]

**Verdict:** DEFER. **Trigger:** either a genuinely multi-threaded structure with adjacent per-thread
writes appears (then 64-byte padding removes false sharing — a real win), OR a criterion A/B on this
host shows an aligned-load delta on a hot kernel. Rejected on neither complexity nor correctness —
deferred on "no measured win + no false-sharing surface today."

### A6 — Software prefetch (`_mm_prefetch`) → **DEFER-WITH-FALSIFIABLE-TRIGGER**

**Genuine target:** `csr::spmv` is the textbook prefetch candidate — the indirect gather
`out[j] += xi*val[k]` with `j = col_idx[k]` (`csr.rs:180-183`) has irregular access into `out`; the
classic optimization prefetches `out[col_idx[k+D]]` ahead. [GROUNDED]

**Ground truth:** at Phase-28's stated graph regime (`n≈10²–10³`, `nnz≈8k`; `BLUEPRINT-CACHE-…-ARENA
§3.3`) the entire graph (≈`nnz*16` bytes ≈128 KB) fits in L2, so a prefetch hint buys ≈0 and can
regress (wrong distance evicts live lines). [GROUNDED]

**Verdict:** DEFER. **Trigger:** `nnz*16 bytes > L2` on the target host **and** a criterion profile
shows `spmv` memory-bound. Composes with A8 (arena-contiguous buffers make prefetch effective). Not
rejected — deferred on measured working-set scale.

### A7 — HugePages / THP (`madvise MADV_HUGEPAGE`) → **DEFER-WITH-FALSIFIABLE-TRIGGER (via the port template)**

**Physics:** transparent huge pages (2 MB) cut TLB misses for large contiguous regions; below ~2 MB of
hot footprint there is no TLB pressure to relieve. [GROUNDED]

**Ground truth:** no `mmap`/`madvise` anywhere (§1.6); all allocation is `Vec` (glibc). The natural
future home is the A8 arena's backing `Vec<u8>` region — but that region is sized by measured
`high_water` (Phase-28 §3.3), expected **µs-scale ⇒ well under 2 MB**, so THP buys nothing yet.
[GROUNDED]

**Verdict:** DEFER, and **specify the shape now so it is a seam, not a rewrite:** a `HugePageHint`
Trait-as-Port with a `NoOp` default, mirroring `core_pinning.rs:41-64` line-for-line (single-region
today ⇒ no TLB pressure; swap in a `madvise` impl behind the same trait when a large region appears).
**Trigger:** a persistent tensor-arena region > 2 MB (the RESCAL/CMTF factor matrices at `n≥10⁵`,
Phase-28 rung 4) — measured by `high_water()`. Rejected on neither complexity nor absent hardware
(Linux `madvise` is present) — deferred on measured footprint, with the port named so the bird can fly.

### A8 — Arena allocator → **ADOPT (build Phase-28's `arena.rs`) + EXTEND (HugePage hook)**

**Already designed, not built:** `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md §3.3`
fully specifies `kernel/src/arena.rs` `BumpArena` — `UnsafeCell<Vec<u8>>` region + `Cell<usize>` bump
offset + `high_water` telemetry, `alloc_slice<T: Copy + Default>` (the `Copy` bound eliminates the
bumpalo no-Drop hazard **at compile time**), O(1) `reset(&mut self)` (borrow-checker proves no live
loans), degrade-closed to heap on exhaustion, `_in` variants (`from_edges_in`, `row_normalize_in`,
`personalized_pagerank_in`, a charpoly scratch path). Consumer sites and the criterion A/B done-check
are already named. Operator overrode the P26 REJECT to get here. [GROUNDED]

**What the dialogue strengthens:** it supplies the two things that blueprint's arena does *not* have —
the **HugePage backing hook** (A7) and the **tiling/Morton story** (A2/A9) that give the arena a
spatial-memory identity ("tile = HugePage-backed arena region"). These are additive extensions, not
conflicts. [GROUNDED]

**Verdict:** ADOPT = build Phase-28's `arena.rs` as specified; do **not** design a second arena. Extend
it with the deferred `HugePageHint` seam (A7). This is the **smallest, highest-leverage** unit in the
cluster: it is the substrate the CSR rebuild, the `spectral::matmul` churn (§1.4), and PPR all sit on.
Not rejected — it is the operator-directed centerpiece.

### A9 — Tiling (2D/3D block layout matched to page size) → **DEFER-WITH-FALSIFIABLE-TRIGGER**

**Physics:** cache-blocked (tiled) GEMM/stencil is the standard win once a matrix exceeds cache — it
raises arithmetic intensity per loaded line. [GROUNDED]

**Ground truth:** `matmul_contig` is a naive `ijk` triple loop (`mat.rs:93-112`). But the matrices it
multiplies are `n≤32` (the Householder fast path, whole matrix = 8 KB, L1-resident) — tiling buys ≈0
there — or the Faddeev `n>32` fallback, which the eigenvector blueprint already rules "the wrong
algorithm and the wrong matrix representation for the cache-graph regime" (`BLUEPRINT-EIGENVECTOR-
REFACTOR §3.4`), i.e. it will not be on a hot path. Tiling pays for dense GEMM at `n≳128`. [GROUNDED]

**Verdict:** DEFER. **Trigger:** a dense matmul at `n≥128` appears on a hot path (none exists). Couples
to A2 (Morton = the tile-ordering) and A7 (tile = HugePage region). Not rejected — deferred on absent
large-dense-matmul consumer.

### A10 — Token-stream "mipmap" / hierarchical compression → **EXTEND-EXISTING (genuine, not decorative)**

**The honest mapping:** a literal image mipmap (box-filter downsample) is *not* what a token/state
stream needs — but the dialogue's *intent* (multi-resolution hierarchical compression of a stream)
maps onto two real primitives. (1) **Spectral coarsening**: truncated eigendecomposition
`W ≈ U_k Λ_k U_kᵀ` (Phase-28 rung 1 = `spectral::topk_symmetric`) IS a coarse "mip level" — the
top-k eigenmodes are the low-frequency (coarse) content of the relation graph, and the Laplacian
gives a principled graph-coarsening pyramid. (2) **Content-defined hierarchical dedup**: `chunker.rs`
(Buzhash rolling CDC, content-address = `event_log::sha3_256`, `chunker.rs:1-20`) already compresses a
byte stream hierarchically so "a one-byte change re-hashes only the local block." [GROUNDED]

**Verdict:** EXTEND-EXISTING. The "mipmap" is the spectral low-rank ladder (already being built) plus,
if a true pyramid is wanted, a Laplacian graph-coarsening operator on top of `topk_symmetric`. Genuine
and load-bearing for state-compression. **Honest boundary:** reject the *literal* pixel-mipmap framing
(box-filter downsampling of tokens is meaningless); adopt the spectral-coarsening reading. Not
rejected — redirected to the correct math.

---

## 3. Part B — signal-processing / tensor-geometry addendum

The operator's test for Part B: **genuine mathematical reuse vs decorative name-dropping.** Applied
strictly.

### B11 — Laplace transform of Dirac delta `L{δ(t)} = 1` → **REJECT (domain-mismatch)**

**Correctness ground for rejection (not complexity):** the kernel is discrete-time and deterministic —
fixed-K iteration, integer/f64 with fixed summation order, no continuous evolution
(`csr.rs:7-24,226-227`). There is **no continuous s-plane** to transform into, so `L{δ}=1` has no
implementation surface. [GROUNDED]

**The one true thing it encodes, already used implicitly:** "an impulse has a flat, all-frequency
spectrum" is the justification for an **impulse (one-hot) seed** — which PPR already uses
(`csr.rs:604-621`, one-hot seed on a path node) and which the field integrator already uses as a
spatial delta source (`field_frame.rs:279`, isolated spike). [GROUNDED] So the *idea* is present; the
*transform* is not adoptable. **Verdict:** REJECT the primitive on domain-mismatch; note the
impulse-response framing is already live.

### B12 — Dimensionality reduction (PCA / t-SNE / UMAP / Isomap / LLE) → **SPLIT VERDICT**

The comparative table maps cleanly onto the determinism contract, which is the deciding axis:

- **PCA (`Z = XV`, `Σv = λv`) → ADOPT-as-existing.** PCA of a symmetric matrix **is** truncated
  eigendecomposition, which **is** `spectral::topk_symmetric` (the eigenvector blueprint's decided
  primitive) — deterministic, fixed-order, composes with CSR/PPR. It is literally Phase-28 rung 1.
  [GROUNDED] Nothing new to build; it is the tensor ladder's first rung.
- **Isomap (`Z = MDS(D_G)`) → DEFER-WITH-TRIGGER.** Deterministic in principle (classical MDS =
  eigendecomposition of a double-centered geodesic-distance Gram matrix, servable by the same
  `topk_symmetric`), **but** needs a k-NN graph + all-pairs geodesic distances (`O(n³)`) that the repo
  does not build. Trigger = a manifold-embedding consumer + a k-NN builder exist. Not rejected —
  deferred on absent infrastructure. [GROUNDED]
- **t-SNE (`min KL(P‖Q)`), UMAP (`min CE`), LLE-iterative → REJECT on the determinism contract.**
  These are stochastic-init / SGD optimizers; t-SNE and UMAP use random initialization and
  perplexity/negative-sampling that are **not bit-reproducible**, directly violating the kernel's
  "no `thread_rng`, fixed summation order, identical input ⇒ identical bytes" contract
  (`csr.rs:36-37,226-227`; determinism requirement echoed in the source dialogue itself). [GROUNDED]
  This is a **correctness/mechanism rejection** (the allowed kind), not a complexity one. (LLE's
  closed-form weight solve is deterministic but needs the same k-NN infra as Isomap ⇒ same DEFER if
  ever wanted.)

**Verdict:** PCA is already the plan; Isomap deferred; the stochastic embedders are rejected on
determinism. The operator's "genuine reuse, not name-dropping" test is *passed* by PCA and *failed* by
t-SNE/UMAP — and the failure is principled, not aesthetic.

### B13 — Z-transform integration property `Σx[k] ↔ X(z)·z/(z−1)` → **EXTEND-EXISTING (analytic lens)**

**Genuine content:** the running-sum ↔ pole-at-`z=1` identity is the discrete integrator, and it
correctly *names an existing structural boundary*: a pole at `z=1` is marginal stability, which is
exactly `DriftClass::Resonant` (`ρ ≈ 1`, `spectral.rs:315-352`) — the kernel already classifies the
`z=1` boundary, it just does not describe it in z-domain terms. Pure accumulators already in the tree:
`ComputeBudget::debit` (`budget.rs:110`), PPR's geometric-series accumulation (`csr.rs:248-254`).
[GROUNDED]

**Verdict:** EXTEND-EXISTING as an analytic lens — it explains and can *document* the drift-class
boundary and the accumulator semantics; it adds no new primitive. Genuine but modest (it names what
exists). Not rejected.

### B14 — Nyquist–Shannon sampling theorem `fs ≥ 2B` → **EXTEND-EXISTING (a real bound to add)**

**Genuine, correctness-relevant:** the kernel has a fixed sampling clock — `DT_STABLE = 0.02 s = 50 Hz`,
"one governed field/animation clock" (`lib.rs:219`, `field_frame.rs:40-53`, pinned by
`field_default_dt_matches_kernel_dt_stable`). Nyquist says this integrator can faithfully represent
source/motion content only up to **25 Hz**; content above that **aliases**. `field_frame`'s stability
doc currently states only the **CFL** bound on `dt` (`field_frame.rs:19-27,59-72`) — it does **not**
state the dual **Nyquist bound on the source `S`**. That is a genuine, missing correctness statement:
a source oscillating faster than 25 Hz will alias into spurious low-frequency field content, invisibly.
[GROUNDED] Also directly relevant to the mesh's epoch/HLC clock: gossip sampling faster than
half-the-epoch-rate aliases state.

**Verdict:** EXTEND-EXISTING — add the Nyquist source-bandwidth bound alongside the CFL bound in
`field_frame`, and carry the same reasoning to the mesh epoch-clock design. Small, genuine,
correctness-relevant. Not rejected.

### B15 — Laplace integration property `∫₀ᵗf ↔ F(s)/s` → **REJECT (domain-mismatch), redirect to B13**

Same continuous-vs-discrete domain-mismatch as B11: no continuous s-plane surface. Its **discrete
counterpart is B13** (`X(z)·z/(z−1)`), where the genuine reuse lives. [GROUNDED] **Verdict:** REJECT
the continuous form on mechanism; the discrete integrator (B13) is the adoptable version.

### B16 — Kuen surface (constant negative Gaussian curvature) → **REJECT-as-decorative**

No hyperbolic-geometry consumer exists; the field is a flat 2D grid (`field_frame.rs`) and the graphs
are combinatorial, not embedded in a curved manifold. A Kuen/pseudosphere embedding would matter only
if hyperbolic embedding of the relation graph were ever built (no trigger, no consumer). [GROUNDED]
**Verdict:** REJECT-as-decorative (no consumer, and no falsifiable trigger names one). Recorded so the
synthesis pass does not spend on it.

---

## 4. The one real code finding to carry forward

`engine/src/zerocopy.rs:22` labels an **interleaved stride-5 (AoS)** particle layout as **"SoA-record
layout."** [MEASURED] The label is inverted. Impact today: none (GPU upload tolerates interleaved
attributes). Impact when the `simd.rs` Kalman SoA consumer (`simd.rs:20-24`) is built: a true-SoA
transpose is required, and the misleading comment will send the implementer wrong. Cheapest fix = one
comment correction now; correct fix at build time = store per-courier filter state true-SoA. This is
the only genuine defect (as opposed to absence) the audit surfaced.

---

## 5. Prioritized build order (smallest abstraction first)

Per the operator's "від малого до великого, найменші абстракції на рівні ядра є ключовими та першими у
пріоритеті." Each item names its consumer and its falsifiable done-check; the Fable synthesis pass
turns this into the blueprint. Ordered by abstraction size, not by dependency alone.

1. **`kernel/src/arena.rs` `BumpArena` (A8).** Smallest substrate; everything else sits on it. Build
   exactly as `BLUEPRINT-CACHE-…-ARENA §3.3` specifies (do not redesign). Done-check: criterion A/B
   `graph_rebuild_rank` heap-vs-arena in `BENCH_HISTORY.md` + counting-allocator assertion (≤8 heap
   allocs on the arena path) + byte-identical PPR output arena-vs-heap + Miri-clean. **Extend** with a
   named `HugePageHint` NoOp port (A7) so the seam exists.
2. **`spectral::topk_symmetric` + `householder::eigh_contig` (A8-adjacent, B12-PCA).** The eigenvector
   solver = the real dimensionality-reduction primitive (PCA rung 1). Build per `BLUEPRINT-EIGENVECTOR-
   REFACTOR §5.1`, born arena-aware. Done-check: that blueprint's §5.4 KAT (`A·v=λ·v`), orthonormality,
   sparse-vs-dense parity, byte-determinism, reconstruction-error monotonicity.
3. **Branchless/sentinel hardening of `csr::spmv` + `mat::matmul_contig` skips (A3).** Extend the
   proven `simd.rs` sentinel/mask pattern to the two CSR/GEMM sparsity branches. Done-check:
   byte-identical output vs the branchy path (the `simd.rs` bit-identity rule) + a criterion delta.
4. **3-way relation-slice tensor + content-address dedup (A1).** Type-tagged CSR slices keyed by
   `matrix_content_address`; the substrate for Phase-28 rung 2 (RESCAL) and the mesh DecisionUnit
   state object. Done-check: identical slice-sets produce identical content-address (a `DecompCache`-
   style no-thrash falsifier) + RESCAL fixtures compile (Phase-28 W6).
5. **Nyquist source-bandwidth bound + z-domain drift doc (B14, B13).** Add the `fs≥2B` bound alongside
   the CFL bound in `field_frame` (and carry to the mesh epoch clock); document the `z=1`-pole ↔
   `DriftClass::Resonant` correspondence. Done-check: a test asserting an above-Nyquist source is
   flagged/aliased as documented; drift-class doc references the z-pole.
6. **Deferred trait-as-port seams, each with a NoOp default + measurable trigger, mirroring
   `core_pinning.rs` (A7 HugePage, A6 prefetch, A5 align, A2/A9 Morton/tiling).** Named-but-not-built
   so the later flight needs no rewrite. Done-check per seam: the NoOp compiles, the trigger is a
   numeric condition recorded in the seam's doc comment (not prose), and a `#[ignore]` failing-by-
   design test names the activation condition (Phase-28 W6 style).

**Rejections recorded (correctness/physics grounds only):** B11/B15 (continuous-transform
domain-mismatch), t-SNE/UMAP/LLE-iterative (determinism-contract violation), B16 Kuen surface (no
consumer, no trigger), literal pixel-mipmap (meaningless on a token stream — redirected to spectral
coarsening). None rejected on complexity, per the operator override.

---

## 6. Anu / Ananke check

**Anu (derivable, not asserted).** Every "already exists" verdict is a live `file:line` read (§1), not
a memory claim; the absence verdicts rest on a zero-match grep on this host (§1.6), not on assumption;
the determinism-based rejections (B12 stochastic embedders) derive from the kernel's own stated
contract (`csr.rs:36-37`), not from taste; the domain-mismatch rejections (B11/B15) derive from the
discrete/continuous distinction, checkable against the code's fixed-K deterministic evolution. The one
place the dialogue's framing is *corrected* rather than *complied with* — the "SoA-record" label that
is actually AoS (§4) — is a direct live-read of `zerocopy.rs:22`. Weakest links, named: the mesh
mappings (A1 DecisionUnit, B14 epoch clock) are [SPECULATIVE] analogies, flagged as such, not asserted
as facts.

**Ananke (structural, not hoped).** The plan's good outcome is made structural, not left to a future
reader: every DEFER carries a **numeric trigger** (a byte size, an `n` threshold, an L2-crossing) so
"when does this flip to ADOPT" is a measurement, not a judgment call; every deferred item is specified
as a **Trait-as-Port with a NoOp default** copying the already-shipped `core_pinning.rs` template
(§1.5), so "the bird can fly later" is a compile-time seam, not a rewrite promise; the arena and
solver done-checks are **byte-identical / Miri / criterion** falsifiers on the **existing** bench
harness (§1.6), not "looks faster." What is NOT yet structural, named honestly: the `zerocopy.rs`
mislabel is fixed only by a comment today — the type system does not enforce SoA-vs-AoS — so the
enforcement is a grep-able convention until the SIMD Kalman consumer forces a true-SoA type (§4).

---

## Appendix — provenance

Live-read this session (current branch `feat/harness-llm-backend`): `kernel/src/{mat,csr,spectral,
householder,simd,spectral_cache,budget,core_pinning}.rs`, `engine/src/{field_frame,zerocopy}.rs`,
`kernel/benches/` listing, and the zero-match memory-primitive grep. Read cross-branch:
`/root/dowiz-spectral-evolution/kernel/src/incidence.rs` (commit `6bd181a02`, **not on this branch**).
Blueprints read in full: `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md`,
`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`. Source dialogue: `00-SOURCE-PROMPT.md`,
`01-RAW-DIALOGUE-PART-A.md` (this directory). Discipline: `AGENTS.md` §§"Integration Decart Rule",
"Detailed Planning Protocol", "Global doctrine — Anu & Ananke". Epistemics-tag pattern:
`BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md:7-9`.
