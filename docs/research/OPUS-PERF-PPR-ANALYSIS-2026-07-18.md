# PPR performance analysis — `kernel/src/retrieval/ppr.rs`

**Date:** 2026-07-18 · **Model:** Opus 4.8 · **Scope:** performance + determinism study of the
Personalized-PageRank power-iteration engine, calibrated to *actual* usage before any algorithm work.

**Verdict up front:** No code change to `ppr.rs` is warranted today. The one real caller runs at
`n=20, k=20` over a **frozen** fixture, a criterion bench for this path **already exists and is
gated**, and the kernel **already owns** a deterministic *sparse* PPR (`csr.rs`) for the scaled case.
The only genuine gap is that the O(n²) growth curve is invisible at the current bench size and no
revisit threshold is written down. Recommendation: add a growth-visibility bench + a named threshold.
Everything below is the evidence for that calibration.

---

## 1. Ground truth — what actually calls this code

### 1.1 The engine
`ppr.rs` stores a **dense** `Vec<Vec<f64>>` transition matrix and iterates a fixed `k` times
(`ppr.rs:20-23`, `ppr.rs:42-67`). The determinism contract is explicit and load-bearing
(`ppr.rs:10-16`): fixed iteration count `K` (no epsilon / early-out), fixed summation order
**i outer, j inner** (`ppr.rs:49-56`), mirroring `markov.rs` so the same f64 ops land in the same
sequence; the per-step `÷ sum` normalize is dropped because the `(1−α)` diffusion + `α` restart
conserves mass exactly (`ppr.rs:14-16`, proven live by the mass test `ppr.rs:94-101` and
`diffusion.rs:245-251`).

The mirror is real. `markov.rs:161-177` is:
```rust
for i in 0..n {
    let pii = pi[i];
    if pii == 0.0 { continue; }
    for j in 0..n { nxt[j] += pii * ((1.0 - d) * a[i][j] + d / n as f64); }
}
```
`ppr.rs:49-56` is the same loop minus the `+ d/n` uniform-teleport term and minus the per-step
`/sum` (`markov.rs:172-175`). Same skip-zero-row guard, same per-term association
`pii * ((1-α) * W[i][j])`. The claim in the doc comment is accurate, not aspirational.

### 1.2 Every real call site (grep, whole tree)
`grep -rn "Ppr::new|\.rank(" kernel/ engine/` → the **only** non-test production caller is
`diffusion.rs`:
- `diffusion.rs:126-128` `wiki_ppr()` = `Ppr::new(wiki_row_stochastic())`
- `diffusion.rs:133-139` `related(seed)` → `ppr.rank(seed, ALPHA, K)`

Everything else that grep returns is either `bm25.rs` / `recall.rs` (a *different* `.rank()` on
the BM25 index — unrelated symbol) or the `ppr.rs` / `diffusion.rs` test modules.

### 1.3 What `n` and `k` actually are
Both are **compile-time constants over a frozen fixture** (`diffusion.rs:21-31`):
- `N = 20` — a hard-coded 20-node / 41-edge wikilink graph (`WIKI_EDGES`, `diffusion.rs:38-100`).
- `K = 20`, `ALPHA = 0.15`.

`wiki_row_stochastic()` builds `W` from the `WIKI_EDGES` **const** (`diffusion.rs:109-123`). There is
**no** code path that grows this graph from a live/growing corpus — the "wikilink graph" is a
static test fixture whose nodes are named after the L0 corpus for vocabulary sharing
(`diffusion.rs:10-13`). So in production today: `n = 20`, `k = 20`, ~`20·20·20 = 8000` FMA/call.
This is not a hot path by size; it is a *correctness* fixture for the L3 relatedness layer.

**Conclusion (Step 1):** usage is **small and bounded**, not growing. `n` is a literal `20`.

### 1.4 The bench blind spot is already closed
The operator's premise ("ppr/absorbing are the UNCOVERED ones") is **stale**. Both benches now exist:
- `bench_ppr` → `ppr/rank_32x32_k20`, `n=32, k=20, α=0.85` (`criterion.rs:186-200`)
- `bench_absorbing` → `absorbing/fundamental_matrix_16` (`criterion.rs:205-218`)

Both are registered in `criterion_group!` (`criterion.rs:250-251`) **and** present in the gated
`baseline.json` (keys `ppr/rank_32x32_k20`, `absorbing/fundamental_matrix_16`). The gate is live:
`bench_track.py` forwards `native-trackers bench kernel --threshold 10` exit codes
(`bench_track.py:15-17,86-98`), and a real measured run is recorded —
`ppr/rank_32x32_k20: 8042.50ns -> 7860.00ns (-2.3%)` (`BENCH_HISTORY.md:317`). So the ppr path is
already measured (~8µs at n=32) and already regression-gated at 10%.

Note: the existing bench uses `α=0.85` while the real caller uses `α=0.15` (`diffusion.rs:28`).
`α` changes only the *values*, never the operation count, so timing is unaffected — but the bench
would read more honestly at the production `α=0.15`.

### 1.5 The kernel already owns a deterministic SPARSE PPR (`csr.rs`)
This is the pivotal finding. `ppr.rs`'s dense `Vec<Vec<f64>>` is an **outlier**: a sibling module
already implements the *same* computation sparsely, with the *same* determinism contract.

`csr.rs` (`kernel/src/csr.rs:1-37` docs) is "deterministic CSR + synchronous Jacobi PPR",
explicitly binding (retrieval-blueprint v2): **synchronous fixed-point power iteration, FIXED `K`,
FIXED summation order → bit-reproducible on any hardware**. Same `π_{k+1} = α·e + (1−α)·(π_k·Â)`
recurrence (`csr.rs:313-366`). It stores one contiguous `val: Vec<f64>` + `col_idx` + `row_ptr`
(`csr.rs:26-29`), and its `spmv` (`csr.rs:268-287`) is the LEFT product `out_j = Σ_i x_i·Â[i][j]`
with the **same** i-ascending, skip-`x_i==0` accumulation as `ppr.rs`/`markov.rs`
(`csr.rs:275-286`). It is used at real scale — `n=1024` in `bench_graph_rebuild_rank`
(`criterion.rs:151-181`, `graph_rebuild_rank/heap` ≈ 109µs, `/arena` ≈ 87µs).

So the sparse, scalable, deterministic PPR the operator would reach for **already exists**. `ppr.rs`
is a second, dense implementation kept for the specific 20-node diffusion fixture. Zero new
dependencies are required to go sparse — the infrastructure is `csr.rs`, already in-tree and tested.

**Caveat (§3.2):** `csr.rs`'s PPR is *not byte-identical* to `ppr.rs`'s — same family, different
rounding. Details below.

---

## 2. Algorithmic research (state of the art, and whether it keeps bit-exactness)

The deciding axis for *this* file is not raw speed — it is whether a technique preserves the stated
**bit-exact, order-fixed reproducibility**. Ranked against that constraint:

| Technique | Cost profile | Bit-exact & deterministic? |
|---|---|---|
| **Dense power iteration** (today) | `O(k·n²)` | ✅ exact, order-fixed |
| **Sparse CSR power iteration** (skip-zero spmv) | `O(k·nnz)` | ✅ exact **iff** same per-term expr + i-ascending order + skip only exact-zero (see §3.1) |
| **Forward / local push** (Andersen–Chung–Lang, "PageRank-Nibble") | sub-linear, `O(1/(ε·α))` work | ❌ **approximate** (error param ε) **and** push-order-dependent |
| **Monte-Carlo PPR** (random walks) | `O(walks)` | ❌ approximate + RNG-seeded |
| **FAST-PPR / BidirectionalPPR** (Lofgren et al.) | ~70× faster than MC, per-target | ❌ approximate (rel-error only for `π > δ`), hybrid MC |

- **Forward Push (Andersen–Chung–Lang, FOCS 2006).** Repeatedly pushes residual mass from vertices
  above a threshold; an error parameter `ε` controls approximation quality
  ([localpartition.pdf](https://fanchung.ucsd.edu/wp/localpartition.pdf)). It is sub-linear and
  excellent for *local* clustering, but it is (a) an **approximation** and (b) **order-dependent** —
  different push orders yield different valid approximate vectors. This is exactly why `csr.rs:21-24`
  declares async/relaxed local-push **explicitly out of scope**: "it is order-dependent and therefore
  non-deterministic. We DO NOT implement it." The codebase has *already ruled on this*.
- **FAST-PPR / BidirectionalPPR (Lofgren, Banerjee, Goel, Seshadhri, KDD 2014; thesis 2015).**
  Combines ReversePush over a frontier set with Monte-Carlo walks; ~70× faster than prior MC, but
  guarantees only *small relative error when `π_s(t) > δ`*
  ([arXiv:1507.05999](https://arxiv.org/pdf/1507.05999),
  [FAST-PPR talk](https://cs.stanford.edu/people/plofgren/Fast-PPR_KDD_Talk.pdf),
  [thesis](https://cs.stanford.edu/people/plofgren/bidirectional_ppr_thesis.pdf)). Approximate + RNG.
- **Monte-Carlo PPR.** Estimates π by sampling `α`-decayed walks; inherently randomized, non-exact.

**Every technique faster than sparse power-iteration trades exact reproducibility for approximation.**
Given `ppr.rs`'s hard determinism constraint, tiers (c)-class redesigns are off the table unless the
operator relaxes that constraint — which the codebase has already declined to do once (`csr.rs:21-24`).

### The crux: is dense→sparse a *free* (bit-exact) win?
**Yes, precisely — under two conditions that are easy to meet.** The argument has two independent
floating-point facts:

1. **Skipping the addition of an exact `0.0` never changes an f64 sum.** In IEEE-754 round-to-nearest,
   `x + (+0.0) == x` for every finite `x`; the sole special case is `(-0.0) + (+0.0) = +0.0`, which
   does not change the *numeric value*. In `ppr.rs` every accumulator starts at `+0.0` and only ever
   adds non-negative finite terms `pii·((1−α)·W[i][j]) ≥ 0`, so `-0.0`/`NaN`/`±Inf` never arise. A
   zero entry contributes `pii·((1−α)·0.0) = +0.0`, whose addition is a bitwise identity. Therefore a
   sparse loop that iterates only the stored nonzeros of each row, in the **same i-ascending order**,
   produces a **byte-identical** result to the dense loop. Each output `out[j]` accumulates one
   contribution per source row `i`, in ascending `i` — within-row column order is irrelevant because
   distinct `j` land in distinct buckets. This matches `csr.rs`'s own `spmv` contract
   (`csr.rs:262-266`). *This is the genuine free win.*

2. **But you may NOT reuse `csr::personalized_pagerank` verbatim and keep the bytes.** IEEE-754 is
   **not distributive**: `Σ_i pii·((1−α)·W[i][j])` (ppr.rs, `(1−α)` folded *into* each term) and
   `(1−α)·Σ_i (pii·W[i][j])` (csr.rs, `(1−α)` applied *after* the sum, `csr.rs:352-354`) round
   differently — rounding happens per operation, so distributing a constant multiply over a sum is
   not exact ([IEEE 754, Wikipedia](https://en.wikipedia.org/wiki/IEEE_754);
   [floating-point precision pitfalls](https://aiwikiproject.com/articles/floating-point-arithmetic)).
   On top of that, `csr.rs` applies a **final normalize divide** (`csr.rs:358-364`) — the exact divide
   `ppr.rs` deliberately removed (`ppr.rs:14-16`) — and normalizes the seed (`csr.rs:332-344`).

   So `csr::personalized_pagerank` is **still deterministic run-to-run** (it is engineered for that,
   `csr.rs:328-329`) but yields **different exact values** than `ppr.rs`. A bit-exact sparse `ppr.rs`
   would need a *new* small sparse loop that preserves the exact per-term expression
   `pii * ((1.0 - alpha) * val)` and drops the final normalize — reusing `Csr`'s **storage**, not its
   **`personalized_pagerank` method**.

---

## 3. Calibrated recommendation

### (a) PRIMARY — `n` is small/bounded today → no `ppr.rs` change; make growth visible + name the threshold
`n=20, k=20` over a frozen fixture (§1.3); the bench already exists and is gated (§1.4). There is
nothing to optimize and no evidence of scale. The only real deficiency is that at `n=32` the **O(n²)
growth curve is invisible** — a future change that quietly makes the graph dynamic would not trip a
single-point bench. Fix that cheaply:

1. **Add a growth-visibility bench** (below) at `n ∈ {32, 128, 256}` so the quadratic is on the record
   and any migration to a growing graph shows up as a step-change, not a silent creep.
2. **Write down the revisit threshold** (there is none today). Proposed, grounded in §1.3–§1.5:
   > Revisit `ppr.rs` representation when the diffusion graph exceeds **~256 nodes** *or* the
   > `ppr/rank_*` bench exceeds **~50µs/call**, whichever first. Below that, dense `O(k·n²)` at
   > `k=20` is sub-100µs and not worth the churn. At/above it, migrate to the CSR path (tier b).

### (b) SAFE determinism-preserving optimization — only once (a)'s threshold is crossed
The matrix *is* genuinely sparse (out-degree 1-4 per node; `nnz ≈ 41` vs `n² = 400` — ~90% zeros),
so the win is real when `n` grows. Two determinism-preserving routes, no new dependency:
- **b1 (byte-identical):** give `ppr.rs` a sparse backend that reuses `Csr` **storage** but keeps the
  exact per-term expression and no final normalize — bit-identical to today's output (§2 fact 1),
  `O(k·nnz)` instead of `O(k·n²)`.
- **b2 (reuse `csr::personalized_pagerank`, simpler):** migrate `diffusion.rs` to build a `Csr` and
  call the existing `personalized_pagerank`. This is *still deterministic run-to-run* but changes the
  exact values (§2 fact 2). Crucially, **the diffusion tests assert properties, not a golden vector** —
  mass≈1 (`diffusion.rs:245-251`), hop-ordering + exact-zero on unreachable nodes
  (`diffusion.rs:257-299`), personalization (`diffusion.rs:317-326`), run-to-run identity
  (`diffusion.rs:190-203`). All of these survive the csr semantics (one-hot seed normalizes to
  itself; unreachable nodes stay exact `0.0`; ordering is monotone). So b2 would pass the existing
  suite. **Flag:** b2 is nonetheless a behavior change (different bytes than `markov.rs`-mirrored
  `ppr.rs`), so it deserves an explicit note even though no test pins the old bytes.

### (c) Algorithmic redesign (Forward Push / MC / FAST-PPR) — REJECT for this codebase
Every faster-than-sparse method is approximate and/or order-dependent (§2). The codebase already
rejected async local-push as non-deterministic and out-of-scope (`csr.rs:21-24`). Adopting any of
these means **trading exact determinism for approximation** — an **operator decision**, not an
engineering default, and one that contradicts a standing binding decision. Do not pursue without an
explicit reversal of the determinism requirement.

---

## 4. Concrete criterion bench proposal (ship this regardless of any code change)

Grounds: real caller is `n=20, k=20, α=0.15` (`diffusion.rs:22,28,31`); existing single-point bench is
`n=32` (`criterion.rs:186-200`). This **replaces** the lone `bench_ppr` with a growth sweep so the
O(n²) curve is visible and the migration threshold (§3a) is measurable. Same deterministic ring+skip
graph shape already used, `α` corrected to the production `0.15`.

```rust
/// Blind-spot coverage: personalized PageRank (retrieval M3, diffusion L3) is
/// O(k·n^2) in the DENSE transition matrix. The only real caller runs n=20/k=20
/// over a frozen fixture (diffusion.rs), but the dense cost is quadratic — so we
/// sweep n to keep the growth curve visible and give §3a's migration threshold a
/// measurable tripwire. α = 0.15 matches the production caller (diffusion.rs:28);
/// α affects values, not op-count, so it is timing-neutral but reads honestly.
fn bench_ppr(c: &mut Criterion) {
    for &n in &[32usize, 128, 256] {
        // Deterministic row-stochastic transition matrix (ring + skip edges).
        let mut w = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            w[i][(i + 1) % n] = 0.5;
            w[i][(i + 7) % n] = 0.5;
        }
        let ppr = Ppr::new(w);
        c.bench_function(&format!("ppr/rank_{n}x{n}_k20"), |b| {
            b.iter(|| black_box(ppr.rank(0, 0.15, 20)))
        });
    }
}
```

- **Realistic `n`:** `32` (keeps the current gated baseline key `ppr/rank_32x32_k20` unbroken),
  plus `128` and `256` to expose the quadratic and bracket the §3a threshold. All comfortably above the
  real `n=20` and far below anything the fixture reaches today.
- **`k = 20`** — the production constant `diffusion.rs:31`.
- **Baseline follow-up:** add `ppr/rank_128x128_k20` and `ppr/rank_256x256_k20` to `baseline.json`
  after the first measured run (as `absorbing/…` and the existing ppr key already are), so all three
  are gated at the 10% threshold via `bench_track.py`. Expected shape from the existing `n=32 ≈ 8µs`
  anchor: `n=128` ≈ 16× ≈ 125µs, `n=256` ≈ 64× ≈ 500µs — i.e. `n=256` already sits near the 50µs…
  threshold band, which is exactly the signal §3a wants surfaced.

**Net:** no product-code change is justified now; the deliverable is a 3-point growth bench + a written
revisit threshold, turning an invisible quadratic into a gated tripwire while the determinism contract
stays untouched.

---

## Sources
- Andersen, Chung, Lang — *Local Graph Partitioning using PageRank Vectors* (FOCS 2006):
  <https://fanchung.ucsd.edu/wp/localpartition.pdf>
- Lofgren, Banerjee, Goel, Seshadhri — *FAST-PPR* (KDD 2014) talk:
  <https://cs.stanford.edu/people/plofgren/Fast-PPR_KDD_Talk.pdf>;
  *Bidirectional PPR* (arXiv:1507.05999): <https://arxiv.org/pdf/1507.05999>;
  thesis: <https://cs.stanford.edu/people/plofgren/bidirectional_ppr_thesis.pdf>
- IEEE 754 (non-associativity / non-distributivity of FP): <https://en.wikipedia.org/wiki/IEEE_754>;
  floating-point precision pitfalls: <https://aiwikiproject.com/articles/floating-point-arithmetic>
- In-tree ground truth: `kernel/src/retrieval/ppr.rs`, `kernel/src/retrieval/diffusion.rs`,
  `kernel/src/markov.rs:161-177`, `kernel/src/csr.rs:1-37,262-366`,
  `kernel/benches/criterion.rs:151-218,250-251`, `kernel/benches/baseline.json`,
  `kernel/benches/bench_track.py:15-98`, `kernel/benches/BENCH_HISTORY.md:317`.
