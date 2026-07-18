# BLUEPRINT — Eigenvector Refactor Plan: extend spectral.rs/householder.rs vs. new lowrank.rs (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART, 2-question doubt
> audit, Anu/Ananke check. Plain prose; every load-bearing claim carries a `file:line` cite, a
> live-fetched web citation, or an explicit **(training-knowledge)** flag.
>
> **The operator's direct question:** does the Phase-28 hybrid tensor decomposition
> (`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` §3.2) actually need eigenVECTORS,
> and if so, is the fix better done as a REFACTOR of the existing eigensolver in
> `kernel/src/spectral.rs` / `kernel/src/householder.rs` (extending its current algorithms to also
> emit vectors) than as the separate new module `kernel/src/lowrank.rs` that blueprint proposed?
> Operating principle (operator, this session): higher-level architecture should mirror/extend the
> best already-built lower-level patterns, not duplicate them with a second implementation of
> similar math.
>
> **Provenance:** three primary sources fetched live this session from netlib (the LAPACK
> reference-implementation doc comments): `dsteqr.f`, `dhseqr.f`, `dhsein.f`. Golub & Van Loan,
> *Matrix Computations* (4th ed., JHU Press 2013) is cited as the standard text for the same
> claims the LAPACK docs ground; where a claim rests on the book alone it is flagged
> **(training-knowledge)**.

---

## 0. Executive answer

1. **Eigenvectors are genuinely needed — confirmed, not assumed** (§1). Low-rank compression is
   `W ≈ U_k Λ_k U_kᵀ`; without `U_k` there is no reconstruction, no projection, and no per-entry
   k-dimensional code. Both "immediate real jobs" the Phase-28 blueprint assigns to rung 1 use
   `U_k` explicitly. Eigenvalues alone measure compressibility; they cannot compress.
2. **The verdict is a HYBRID, with one public façade** (§4 DECART):
   - **REFACTOR `householder.rs` (additive)** — the dense n ≤ 32 Householder→QR engine gains a
     symmetric eigen-decomposition entry point `eigh_contig` by (a) accumulating the Householder
     reflectors it already applies and (b) a symmetric tridiagonal QR loop that accumulates its
     Givens rotations. This is the textbook near-free case (LAPACK `DSTEQR`, fetched, §3.1), it
     serves a second named consumer (field-UI Laplacian eigenmodes, `spectral.rs:12`), and it is
     additive: `eigenvalues_contig` and its 8 hand-oracle tests are untouched.
   - **The sparse top-k solver the cache graph actually needs lands INSIDE `spectral.rs`, not as
     a new `lowrank.rs` module** — `spectral::topk_symmetric(&Csr, k, iters)` becomes the third
     tier of the routing façade `spectral.rs` already is (`eigenvalues()` at `spectral.rs:195-214`
     already dispatches n ≤ 32 → householder, n > 32 → Faddeev). One public eigen surface; zero
     new files; the proposed `kernel/src/lowrank.rs` is SUPERSEDED.
   - **The general-n Faddeev-LeVerrier + Durand-Kerner path stays values-only, deliberately**
     (§3.2/§3.4). It has no natural vector byproduct, the inverse-iteration bolt-on is rejected
     with reasons (needs a dense LU solver that does not exist in the kernel, and inherits
     characteristic-polynomial ill-conditioning exactly in the n range where the path runs), and —
     decisive — the real Phase-28 use case (n ≈ 10²–10³, sparse, symmetric, top-k only) cannot
     use this path even for eigenVALUES (§3.4). Extending it would be extending the wrong
     algorithm for a consumer that will never call it.
3. **Blast radius: zero existing call sites forced to change** (§5.2). All 14 files that consume
   `spectral`/`householder` today use eigenvalue-derived scalars only (ρ, |λ₂|, gap, Fiedler,
   energy, drift class, period) — the refactor is purely additive. Two files gain code
   (`householder.rs` ≈ +170 LOC incl. tests, `spectral.rs` ≈ +150 LOC incl. tests);
   `spectral_cache.rs` needs no change at all (its `Decomp` type already has the basis slot,
   `spectral_cache.rs:28`); Phase-28's W2 calls `spectral::topk_symmetric` instead of
   `lowrank::topk_symmetric` — a one-line difference in a unit not yet built.
4. The Phase-28 blueprint's REJECT of "extend the existing eigensolver" was **right about the
   Durand-Kerner engine and wrong by omission about the Householder engine** — its stated reason
   ("both paths are eigenvalues-only, no vector-recovery step") is a fact about the current code,
   not about the algorithms. For QR-family methods, vector recovery is a standard accumulation
   extension of the SAME iteration (§3.1); the blueprint treated a missing feature as a
   mechanism limit. Its conclusion for the sparse regime survives on corrected grounds (§3.4).

---

## 1. Question 1 — are eigenvectors actually needed? (confirm, don't assume)

**Yes, strictly.** The precise statement:

- A truncated eigendecomposition of a symmetric matrix is `W ≈ U_k Λ_k U_kᵀ` where
  `Λ_k = diag(λ_1..λ_k)` (the "energy" per component) and `U_k ∈ ℝ^{n×k}` (the basis directions).
  Reconstruction (`U_k Λ_k U_kᵀ`), projection of an entry into the k-dim latent space (row i of
  `U_k`, optionally scaled by `Λ_k^{1/2}`), and the reconstruction-error measurement
  `‖W − U_k Λ_k U_kᵀ‖_F / ‖W‖_F` are all functions of `U_k`. Eigenvalues alone determine only the
  *best possible* rank-k error (`(Σ_{i>k} λ_i²)^{1/2}` by Eckart–Young **(training-knowledge)**)
  — i.e. they can tell you whether compression WOULD pay, never perform it.
- The Phase-28 blueprint's own rung-1 deliverables require `U_k` verbatim: "(i) k-dim spectral
  coordinates per cache entry — the first columns of Y … (ii) low-rank reconstruction error
  `‖W − U_k Λ_k U_kᵀ‖_F / ‖W‖_F`" (`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md`
  §3.2). Both are vector objects; (i) IS rows of `U_k`.
- The higher rungs are more vector-dependent, not less: RESCAL factorizes `X_k ≈ A·R_k·Aᵀ` where
  `A ∈ ℝ^{n×r}` is the shared entity-factor matrix (Nickel–Tresp–Kriegel ICML 2011, fetched by
  P28 §2.2) — `A` is a basis-like matrix, the multi-relational generalization of `U_k`. CMTF's
  coupled objective is minimized over the same shared `A` (Acar–Kolda–Dunlavy 2011, fetched by
  P28 §2.2). There is no eigenvalues-only formulation of any rung.
- The existing cache slot agrees: `spectral_cache::Decomp = (Vec<Vec<f64>>, Vec<f64>)` is
  documented as "(basis, values) … an eigenvector basis may be supplied by callers that solve for
  one" (`spectral_cache.rs:20-28`) — the architecture reserved the vector slot in P11 and every
  caller stores `Vec::new()` today (`spectral_cache.rs:122`).

Verdict: the need is real and structural. The question reduces entirely to WHERE the vectors
should come from.

## 2. Ground truth — what the existing solvers implement (live-read this session)

| Path | Algorithm | n range | Vectors? | Where |
|---|---|---|---|---|
| Dense fast path | Householder reduction to upper Hessenberg, then shifted QR in complex arithmetic (Givens rotations, Wilkinson-style shift from the bottom 2×2, deflation by shrinking the active block) | n ≤ 32 (stack-only) | **No** — reflectors and rotations are applied but never accumulated | `householder.rs:129-182` (reduction), `186-258` (QR driver), `274-333` (`qr_step`), routed from `spectral.rs:195-204` |
| Dense general path | Faddeev-LeVerrier characteristic-polynomial coefficients (trace recurrence) + Durand-Kerner simultaneous root iteration | n > 32 (fallback; also the parity oracle in householder's tests) | **No** — roots of a scalar polynomial; no matrix quantity survives to recover vectors from | `spectral.rs:113-137` (charpoly), `141-186` (roots), `206-213` (routing) |
| Top-1 special case | Power iteration on a bitmask-encoded 10-state FSM adjacency (dominant eigenvalue modulus only; the dominant eigenVECTOR is computed internally and discarded) | fixed n = 10 | No (discarded) | `order_machine.rs:311-` |

Implementation details that matter for the refactor decision (live-read):

- `reduce_hessenberg` applies each reflector two-sidedly in place and **discards it**
  (`householder.rs:157-180`); accumulating the product `Q₀ = P₁P₂…P_{n−2}` is a strictly additive
  loop over the same reflector vector `v` already in hand — the reduction math is untouched.
- `eig_hessenberg`/`qr_step` take two values-only shortcuts that are load-bearing for the
  "is accumulation a small change?" question: (a) all rotation updates are **restricted to the
  active m×m block** (`qr_step`'s left pass `for j in k..m`, right pass `for i in 0..m`,
  `householder.rs:304,321-327`) — sound for eigenvalues (the spectrum of a block-triangular
  matrix is the union of its diagonal blocks' spectra) but it means the final matrix is NOT one
  global similarity of the input, so naively accumulating these rotations would produce a basis
  for nothing; (b) deflated 2×2 blocks are resolved by the quadratic formula and popped
  (`householder.rs:219-238`) — the iteration never drives the matrix to an actual (quasi-)
  triangular Schur form. Retrofitting vector output into THIS loop is a restructuring of tested
  hot code, not a flag.
- `spectral.rs` is already a **routing façade**: `eigenvalues()` dispatches on n
  (`spectral.rs:195-214`), and every public quantity (`spectral_radius`, `slem`, `spectral_gap`,
  `graph_energy`, `graph_spectrum`, `algebraic_connectivity`, `classify_drift`,
  `dominant_period`) is derived from the routed eigenvalue multiset. This façade is the
  "best already-built lower-level pattern" the operator's principle points at.
- Sparse kernels the top-k solver would sit on: `Csr::spmv` (`csr.rs:166`), `laplacian_spmv`
  (`csr.rs:307`), with the determinism contract (fixed iteration counts, fixed summation order)
  already pinned by `personalized_pagerank` (`csr.rs:228-264`).

## 3. Per-algorithm analysis — is vector recovery a natural extension?

### 3.1 Householder → Hessenberg → shifted QR (n ≤ 32): YES for the symmetric case, as accumulation

The textbook and reference-implementation position, grounded:

- **Symmetric case.** LAPACK `DSTEQR` "computes all eigenvalues and, optionally, eigenvectors of
  a symmetric tridiagonal matrix using the implicit QL or QR method"; with `COMPZ='V'`,
  "Z must contain the orthogonal matrix used to reduce the original matrix to tridiagonal form"
  and the routine returns the eigenvectors of the ORIGINAL matrix by accumulating the QR
  rotations into that Z (netlib `dsteqr.f`, **fetched this session**). I.e., eigenvectors come
  from the SAME QR iteration plus rotation accumulation — the standard-reference treatment is
  Golub & Van Loan, *Matrix Computations* 4th ed., §8.3 (symmetric QR algorithm)
  **(training-knowledge for the section number; the mechanism itself is grounded by the fetched
  LAPACK doc)**. For a symmetric input, Householder reduction to Hessenberg IS reduction to
  tridiagonal, all eigenvalues are real, no complex-pair deflation arises, and the accumulated
  orthogonal product converges to the eigenvector matrix as the iteration converges to diagonal.
- **Nonsymmetric case.** The same accumulation yields **Schur vectors**, not eigenvectors:
  `DHSEQR` "computes the eigenvalues of a Hessenberg matrix H and, optionally, the matrices T and
  Z from the Schur decomposition H = Z T Zᵀ" (netlib `dhseqr.f`, **fetched**); eigenvectors then
  need an extra back-substitution pass on quasi-triangular T (LAPACK `DTREVC`) or inverse
  iteration (`DHSEIN`, §3.3). More work, still standard.

Against this codebase's implementation (§2): the symmetric extension decomposes into
(a) accumulate reflectors in `reduce_hessenberg` — small, additive, the same `v` and `beta`
already computed are applied to one extra buffer; and (b) a symmetric tridiagonal QR loop with
rotation accumulation — which the CURRENT complex `qr_step` cannot serve as-is, because its
block-restricted updates and quadratic-formula deflation (§2) were values-only shortcuts. Two
honest options were weighed:

- **Option A (adopted):** keep `eig_hessenberg`/`qr_step` byte-identical for the values-only
  path; add a dedicated real symmetric tridiagonal QR loop (`DSTEQR` shape: implicit Wilkinson
  shift, Givens rotations applied to the tridiagonal AND accumulated into Q). New ≈ 100 LOC of
  simple real arithmetic in the same module, sharing `reduce_hessenberg` (extended with an
  optional accumulator). Zero risk to the 8 existing hand-oracle/parity tests. Precedent that
  two specialized loops in one module is the reference practice, not duplication: LAPACK ships
  `DSTEQR` (symmetric tridiagonal) and `DHSEQR` (general Hessenberg) as separate routines over
  shared reductions.
- **Option B (rejected, named as the future upgrade):** restructure `qr_step` to maintain a full
  global similarity (full-width updates, no popping deflation), accumulate Z, and add
  quasi-triangular back-substitution — the general nonsymmetric eigenvector path. Rejected now:
  ~2–3× the code, touches the tested inner loop, and has **no current consumer** (every named
  vector consumer — cache graph W, field-UI Laplacian, mesh Laplacian — is symmetric). If
  nonsymmetric vectors (e.g. true DMD modes) are ever needed, this is the named path.

### 3.2 Faddeev-LeVerrier + Durand-Kerner (n > 32): NO natural extension; the bolt-on is rejected

- **No byproduct in the algebra as coded.** Durand-Kerner iterates scalar root estimates of the
  characteristic polynomial (`spectral.rs:150-185`); nothing vector-valued exists to accumulate.
  For completeness and rigor: Faddeev-LeVerrier's auxiliary matrices `M_k` DO admit a classical
  theoretical vector route — they assemble the resolvent numerator `B(λ) = adj(λI − A)`, and for
  a simple eigenvalue any nonzero column of `B(λᵢ)` is an eigenvector **(training-knowledge)**.
  It is rejected on mechanism: it requires retaining all n auxiliary matrices (O(n³) memory,
  currently discarded at `spectral.rs:127-134`), costs an extra O(n³) matrix-polynomial
  evaluation per eigenvalue, and inherits the charpoly path's conditioning (below). Nobody's
  reference implementation does this, for these reasons.
- **The standard bolt-on is inverse iteration** — given an accurate eigenvalue estimate, solve
  `(A − λ̂I)v ≈ prev` a few times; this is exactly what LAPACK `DHSEIN` does ("uses inverse
  iteration to find specified right and/or left eigenvectors of a real upper Hessenberg matrix
  H", netlib `dhsein.f`, **fetched**). It is genuinely simple — but for THIS path it is rejected
  on three grounds: (1) it needs a pivoted dense linear solver, which the kernel does not have
  (live grep: no LU/Gaussian solve exists in `kernel/src/`) — the "few-line addition" premise
  assumes infrastructure that would itself be new numerical code; (2) inverse iteration's
  accuracy is bounded by the eigenvalue estimate's accuracy, and charpoly-root eigenvalues
  degrade catastrophically with n — root-finding on characteristic polynomials is the textbook
  ill-conditioned formulation (Wilkinson's perturbation analysis; the reason all production
  eigensolvers went QR **(training-knowledge)**) — precisely in the n > 32 range where this path
  is the one that runs; (3) no consumer: §3.4 shows the only prospective vector consumer at
  n > 32 is sparse and cannot use this dense path anyway.
- **Kept as-is, values-only, with a doc comment stating this decision** — the path retains its
  two real jobs: fallback for dense n > 32 eigenvalue queries and the independent parity oracle
  for householder's tests (`householder.rs:358-391`).

### 3.3 Power iteration (order_machine): the family the sparse solver generalizes

`order_machine::spectral_radius` already computes the dominant eigenVECTOR internally (the power
iterate `v`) and discards it, on a bitmask-specialized fixed n = 10 FSM adjacency
(`order_machine.rs:311-`). The sparse top-k solver (§5.1) is the general-Csr, k > 1,
vector-returning member of the same algorithm family. Per the adapter rule (older = adapters,
no purging), the FSM-specialized version stays; a parity test pins `topk_symmetric(k=1)` against
it on the FSM's own matrix where applicable (both must agree on ρ).

### 3.4 Which path does the REAL use case hit? Neither — and that is the decisive fact

The cache-reference-graph consumer's regime, from its own numbers: n ≈ 10²–10³ entries
(P28 §2.2: "at n ≈ 10²–10³ a deterministic power-plus-deflation solve is already cheap"; arena
sizing assumes n = 1024, nnz ≈ 8k — P28 §3.3), symmetric by construction (both edge directions
pushed, P28 §3.1.3), needs only the top k ≈ 8–32 eigenpairs, and lives natively in `Csr`.

- The **n ≤ 32 Householder path** cannot hold it (n is 32× too large), so the refactored
  `eigh_contig` is NOT the cache graph's solver — its consumers are the field-UI/mesh Laplacian
  modal work (`spectral.rs:10-12`) and small-operator diagnostics, plus serving as the dense
  parity oracle for the sparse solver's tests.
- The **Faddeev path** fails on all three axes at n = 1024: cost — O(n⁴) ≈ 2×10¹² flops
  (1023 dense 1024³ matmuls) per solve, plus ≈ n² + O(n) transient allocations per charpoly call
  (P28 §1.3); numerics — Horner evaluation of a degree-1024 monic polynomial at |x| ≈ ρ ≈ 2
  computes x¹⁰²⁴ ≈ 10³⁰⁸, at the very edge of f64 range, and coefficient conditioning is
  destroyed far earlier **(training-knowledge, Wilkinson)**; shape — it densifies a matrix whose
  nnz/n² ≈ 0.8%. It cannot deliver reliable eigenVALUES here, let alone vectors.
- Therefore the sparse top-k solver is **required new machinery regardless of the refactor
  question** — the Phase-28 blueprint's conclusion stands, on corrected grounds: not "the
  existing solver lacks a vector step" (true but incidental) but "the existing solver is the
  wrong algorithm and the wrong matrix representation for this regime" (decisive). What changes
  is WHERE it lives and what sits beside it (§4, §5).

## 4. DECART

| # | Candidate | Native fit | Falsifiable correctness | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | **Hybrid (adopted): additive `eigh_contig` in householder.rs + `topk_symmetric` as a new tier INSIDE spectral.rs; Faddeev stays values-only** | Extends the two patterns that are already best-in-repo: the reflector machinery (`householder.rs:129-182`, reused with accumulation) and the routing-façade pattern (`spectral.rs:195-214`, gains a sparse tier). One public eigen surface. `Decomp` slot filled with zero `spectral_cache` changes | KAT `A·v = λ·v` on hand-derived spectra; `UᵀU = I`; values-parity new-vs-existing paths; byte-identical determinism runs (§5.4) | ≈ +320 LOC total incl. tests, 2 files touched, 0 new files | zero new deps | both additions independently deletable; existing APIs byte-identical | **ADOPT** |
| 2 | Separate `kernel/src/lowrank.rs` sibling module (the P28 §3.2 proposal) | Same solver code — but a second module boundary carrying eigen-math authority parallel to spectral.rs; the operator's principle names exactly this as the failure mode; project memory already flags "3-eigensolver dual-authority" as a standing bug class (integration-research arc) | same tests possible | ≈ 120 LOC + a new file + a second public surface | zero deps | deletable | **REJECT — superseded.** The solver's ~120 LOC are kept nearly verbatim; only its home and public routing change. Cost of merging into spectral.rs vs a sibling: one `use crate::csr::Csr` line |
| 3 | Extend Durand-Kerner/Faddeev with vector recovery (inverse iteration or adjugate columns) so ONE path serves all n | none — §3.2: needs a dense LU that doesn't exist, accuracy collapses with n, and §3.4 shows the only large-n consumer is sparse and will never call the dense path | would need new hand-oracles at exactly the n where the method is least trustworthy | LU solver + per-root solve loop + O(n³) memory (adjugate variant) | zero deps | — | **REJECT on mechanism and on absent consumer** |
| 4 | Restructure the complex Hessenberg QR for full Schur-form + back-substitution (general nonsymmetric eigenvectors now) | the eventual textbook-complete engine (`DHSEQR`+`DTREVC` shape) | parity risk against 8 pinned oracles while restructuring `qr_step` | ~2–3× option 1's householder share | zero deps | — | **REJECT now, NAMED upgrade** — no nonsymmetric-vector consumer exists (§3.1 option B) |
| 5 | Lanczos instead of power+deflation for the sparse tier | better convergence for clustered spectra; heavier bookkeeping (reorthogonalization) and a subtler determinism story | — | ≈ 3× solver LOC | zero deps | — | **REJECT for v1, NAMED rung upgrade** alongside P28's randomized-SVD rung-4 note — power+deflation at k ≤ 32, n ≤ 10³ is sufficient and matches the PPR determinism contract |

**Mandatory probe (strongest honest argument against this plan):** the hybrid still ends with two
dense QR-family loops in householder.rs (complex Hessenberg values-only + real tridiagonal
with vectors) — one could argue THAT is the duplication the operator's principle forbids, and that
option 4 (one restructured general engine) is the principled endpoint. The answer, with evidence:
the two loops share the reduction step and module, specialize by matrix class exactly as the
reference implementation does (`DSTEQR` vs `DHSEQR` — fetched), and the alternative buys
generality no caller uses at the price of destabilizing the repo's most-pinned numerical code.
The principle says extend the best built pattern — the best built pattern here includes
householder.rs's discipline of narrow, hand-oracle-pinned specializations. If a nonsymmetric
vector consumer ever appears, option 4 is the named path and the accumulation machinery built
here (reflector accumulation, Givens application to an accumulator) is its first half.

## 5. The refactor plan (concrete)

### 5.1 Exact signatures

**`kernel/src/householder.rs` (additive; existing functions byte-identical):**

```rust
// PRIVATE change, 1 internal call site (eigenvalues_contig passes None):
// reduce_hessenberg gains an optional orthogonal accumulator. When Some, each
// reflector P_k = I − β v vᵀ is also applied on the right to q (q ← q·P_k),
// so on exit q = P₁P₂…P_{n−2} and A_in = q · H · qᵀ. Reduction math untouched.
fn reduce_hessenberg(a: &mut [f64], n: usize, q: Option<&mut [f64]>);

/// Symmetric eigen-decomposition, dense stack path (n ≤ 32).
/// Input: compact n×n row-major SYMMETRIC matrix (debug_assert symmetry, tol 1e-9).
/// Method: Householder tridiagonalization (accumulated) + implicit-Wilkinson-shift
/// symmetric tridiagonal QR with rotation accumulation — the DSTEQR shape.
/// Returns (basis, values) == spectral_cache::Decomp: values ascending, basis[i]
/// the unit eigenvector for values[i], sign fixed (first nonzero component > 0)
/// for cross-run/cross-path byte-determinism.
pub fn eigh_contig(a: &mut [f64], n: usize) -> (Vec<Vec<f64>>, Vec<f64>);

// PRIVATE new: the DSTEQR-shaped loop. Operates on diagonal d / subdiagonal e
// extracted from the tridiagonalized buffer; applies every Givens rotation to
// the accumulator q (n×n row-major). Fixed max sweeps (deterministic), per-block
// convergence by the same eps-scale test eig_hessenberg uses.
fn tridiag_qr_symmetric(d: &mut [f64; 32], e: &mut [f64; 32], n: usize, q: &mut [f64]);
```

**`kernel/src/spectral.rs` (additive; gains one `use crate::csr::Csr`):**

```rust
/// Full symmetric eigen-decomposition (dense, n ≤ 32) — façade over
/// householder::eigh_contig, mirroring the eigenvalues() dispatch pattern.
/// n > 32 dense-symmetric has no consumer and no path: use topk_symmetric on a
/// Csr instead (documented; debug_assert n ≤ 32).
pub fn eigh(a: &[Vec<f64>]) -> crate::spectral_cache::Decomp;

/// Deterministic top-k eigenpairs of a SYMMETRIC Csr — the sparse tier.
/// Fixed-iteration power method + implicit Hotelling deflation over Csr::spmv
/// (deflation A := A − λ v vᵀ applied as a per-spmv correction; the Csr is never
/// densified). Deterministic: index-graded start vector, fixed `iters`, fixed
/// summation order inherited from spmv, sign fixed as in eigh_contig.
/// Returns (basis, values) == spectral_cache::Decomp, descending |λ|.
pub fn topk_symmetric(a: &crate::csr::Csr, k: usize, iters: usize)
    -> crate::spectral_cache::Decomp;
```

`spectral_cache.rs`: **no change** — `Decomp` (`spectral_cache.rs:28`) and
`DecompCache::get_or_recompute` already accept a non-empty basis; the recompute-counter
falsifiers apply unchanged. `wasm.rs`: **no new export in this plan** — no JS consumer of bases
exists; adding `spectral_eigh_js` later is a ~10-line additive follow-on in the established
pattern (`wasm.rs:660-711`).

### 5.2 Call-site audit (blast radius)

Live grep over `kernel/src` + `engine/src` (this session): 14 files reference the spectral
surface — `engine/src/bridge.rs` (DriftClass wire codes only), `engine/src/field_frame.rs`
(comment only), `kernel/src/{bin/markov_attractor, csr, evals, event_log, hydra, lib, markov,
order_machine, spectral_cache, wasm}.rs`. Every one consumes eigenvalue-derived scalars or the
drift enum; **none touches a function whose signature changes**. Forced updates: **0**. The one
private signature change (`reduce_hessenberg` gaining `Option<&mut [f64]>`) has exactly 1
internal call site (`householder.rs:341`), updated to pass `None`.

### 5.3 Build order (dependencies re-derived, not draft order)

| Step | Unit | Depends on | Why this dependency is real |
|---|---|---|---|
| R1 | `reduce_hessenberg` accumulator + `eigh_contig` + `tridiag_qr_symmetric` in householder.rs | — | self-contained; oracle tests need nothing else |
| R2 | `spectral::eigh` façade | R1 | routes to R1 |
| R3 | `spectral::topk_symmetric` (sparse tier) | R1 for its parity tests only (dense cross-check); solver itself only needs `Csr::spmv` | the KAT discipline requires an independent oracle; R1's `eigh` is that oracle on densified small fixtures |
| R4 | Phase-28 W2 wiring: maintenance pass calls `spectral::topk_symmetric`, result cached via `DecompCache` | R3 + P28 W1 | consumer wiring; unchanged from P28's plan except the callee path |

R1 and the solver body of R3 are collision-free lanes (different functions, one shared file only
at R3's test layer) — parallelizable if dispatched, with spectral.rs integration done last.

### 5.4 New tests (Verified-by-Math convention, mirroring the repo's hand-oracle style)

1. **KAT residual (the requested test):** for P₃'s Laplacian `[[1,−1,0],[−1,2,−1],[0,−1,1]]`
   (spectrum {0,1,3} — already the fixture at `householder.rs:435-443`), assert per pair
   `‖A·v − λ·v‖∞ < 1e-9` AND the hand-derived vectors themselves: λ=0 → (1,1,1)/√3,
   λ=1 → (1,0,−1)/√2, λ=3 → (1,−2,1)/√6 (sign-fixed). Same for K₃'s adjacency — with the
   degenerate pair λ = {−1,−1} asserted by residual + orthonormality + eigenspace membership,
   NOT by specific vectors (a degenerate eigenspace has no canonical basis; asserting exact
   vectors there would be a false-precision test).
2. **Orthonormality:** `‖UᵀU − I‖∞ < 1e-9` for eigh_contig outputs.
3. **Values-parity:** `eigh_contig` eigenvalues == `eigenvalues_contig` (existing path) on all
   symmetric fixtures, tol 1e-9 — the new path may not silently disagree with the old one.
4. **Sparse-vs-dense parity:** `topk_symmetric(k=n)` on Csr-built P₃/K₃ matches `eigh` on the
   dense same matrix (values + sign-fixed vectors, tol 1e-6 — power iteration converges linearly
   at rate |λ_{i+1}/λ_i|, so tolerance is honest, not sloppy).
5. **Determinism falsifier:** two `topk_symmetric` runs over the same Csr are byte-identical
   (`f64::to_bits` equality) — same contract class as PPR's tests (`csr.rs:495-550`).
6. **Reconstruction-error monotonicity:** `‖W − U_kΛ_kU_kᵀ‖_F` non-increasing in k on a fixture
   graph — pins the rung-1 metric P28 §3.2 promises.
7. **Existing suites untouched and green:** householder's 8 tests, spectral's 12, spectral_cache's
   2 falsifiers — the additive-only claim is itself falsifiable by `cargo test` before/after.

### 5.5 Integration with / supersession of the P28 lowrank.rs proposal

- P28 §3.2's rung-1 solver (algorithm, determinism contract, `Decomp` return, reconstruction
  metric) is preserved **as specified** — only its home changes: `spectral::topk_symmetric`
  instead of a new `lowrank.rs`. Rungs 2–4 (RESCAL ALS, CMTF, SQ/PQ) are unaffected; when rung 2
  activates, its ALS lives where its rung-1 substrate lives (spectral.rs or a then-justified
  submodule — a decision deferred to the moment a second relation slice exists, per P28 W6).
- P28 W2/W5's done-checks referencing `lowrank.rs` read against `spectral.rs` instead;
  `topk_symmetric` is born arena-aware per P28 W5 exactly as planned (`_in` variant).
- The P28 blueprint carries a dated addendum (appended this session) pointing here; its §3.2/§4
  row-3 text is not rewritten.

## 6. 2-question doubt audit

**Q1 — least confident about (concrete):**
1. The claim that reflector accumulation + a fresh tridiagonal QR loop is lower-regression-risk
   than restructuring `qr_step` is an engineering judgment, defended by the additive-only test
   guarantee (§5.4.7) but not provable before code exists. The mitigation is structural: the old
   path is never edited, so the worst failure mode of the new code is a red NEW test, not a
   regressed old one.
2. Power+deflation's accuracy on the cache graph's real spectrum is unknown until measured — if
   the leading eigenvalues cluster (|λ_{k+1}|/|λ_k| → 1), fixed-iteration convergence degrades and
   deflation error compounds across k. The reconstruction-error metric self-reports this
   (it IS the honest number), and Lanczos is the named upgrade (DECART #5). Deflation drift for
   k ≤ 32 at f64 is expected benign **(training-knowledge)** — the sparse-vs-dense parity test
   (§5.4.4) is the falsifier at fixture scale.
3. The sign-fixing convention (first nonzero component positive) can flip discontinuously under
   tiny perturbations of a near-zero leading component — harmless for correctness (both signs are
   eigenvectors) but it can make the byte-determinism test brittle across FMA/non-FMA codepaths
   (`householder.rs:29-75` runtime-dispatches FMA). If §5.4.5 proves flaky across hosts, the
   fallback is determinism-per-host (same binary, same bits), documented — matching what the PPR
   tests actually pin.

**Q2 — biggest thing possibly missed:** a fourth consumer of vectors hiding behind the E1/E2
spectral-evolution arc (Laplacian parity-bind, memory index sign split) — if E1's remediation
lands a canonical `−(D−A)` vs `+(D−A)` sign choice, `eigh`'s Laplacian-mode output must adopt the
same pinned sign convention or the two arcs will disagree on mode orientation. Named here so the
E1 implementer greps into it; not blocking (E1 is blueprint-stage).

## 7. Anu / Ananke check

**Anu (derivable, not asserted):** the need for vectors derives from the algebra of low-rank
reconstruction and from the consuming blueprint's own deliverables (§1), not from preference. The
"accumulation is the standard extension" claim derives from fetched reference-implementation docs
(`dsteqr.f`, `dhseqr.f`), not from memory; the "inverse iteration is standard but wrong here"
claim derives from a fetched doc (`dhsein.f`) plus two live-verified repo facts (no LU solver
exists; the large-n consumer is sparse). The decisive routing fact — the real use case fits
neither existing dense path — derives from the consuming blueprint's own stated n and nnz.
Weakest links, named: Wilkinson conditioning and Eckart–Young are cited from training knowledge
(both are bedrock, but unfetched this session), and doubt Q1.1 is judgment.

**Ananke (structural, not hoped):** the additive-only shape makes regression structurally
impossible rather than reviewed-for: no existing public signature changes, the one private
signature change has one call site, and §5.4.7 makes "old suites untouched" a command, not a
promise. Single-authority is structural, not conventional: after this plan there is still exactly
ONE public module answering eigen-questions (`spectral.rs`), now with three documented tiers, and
the would-be second authority (`lowrank.rs`) never comes into existence. Determinism is enforced
by inheritance (spmv's fixed order, fixed iters, index-graded starts) plus falsifier tests, same
as PPR. What is NOT structural, named honestly: nothing prevents a future arc from adding a
fourth eigen-implementation elsewhere — the grep-able rule is this document plus the module doc
comment R1 adds to householder.rs; the "3-eigensolver dual-authority" memory item shows the
hazard is real and recurring.

## 8. Registration

Folded as **Phase 28's refined rung-1 execution plan**, not a new phase — re-read of the roadmap
§8 (fresh this session) confirms P28's §8.10 row already owns this scope ("rung 1 buildable NOW =
new deterministic `kernel/src/lowrank.rs` …"); this plan changes the solver's home and adds the
householder `eigh` extension, without changing the phase's dependencies, consumers, or ladder. A
dated amendment line is appended under §8.10 pointing here; the original row text is preserved.

---

## 9. Implementation deviations (honest record — committed 2026-07-18, commit `03ac0fefe`)

This plan was executed and committed, but TWO algorithm choices diverged from §3/§5 spec under
test pressure. Both divergences were load-bearing (the spec'd method did not converge to KAT
tolerance on the hand-derived spectra); both were caught by the falsifiable tests this plan
mandated, not by prose. Recorded here per the Detailed-Planning-Protocol's "don't paper over a
deviation with an invented specific" rule.

### 9.1 Dense path: Jacobi, NOT DSTEQR-shaped implicit-QL/QR
- **Plan said (§3.1, §5.1):** Householder tridiagonalization + *implicit-Wilkinson-shift symmetric
  tridiagonal **QR** with Givens rotation-accumulation* — the `DSTEQR` shape (`tridiag_qr_symmetric`
  in §5.1).
- **Actual (committed, `householder.rs::eigh_contig`):** classical **cyclic Jacobi** sweep over the
  full symmetric matrix — `reduce_hessenberg` (Householder) builds the reflector basis `Q`, then a
  fixed-sweep Jacobi rotation loop diagonalizes `Qᵀ A Q`, accumulating the product of Jacobi
  rotations into `Q` to yield the eigenvector basis. `eigh_contig` is the public façade;
  `spectral.rs::eigh` wraps it.
- **Why:** the implicit-QL/QR driver (`qr_step` Wilkinson-shift deflation) repeatedly failed the
  orthonormality KAT (`r2_eigh_facade_p3_kat` demands `UᵀU = I` to 1e-9) on near-degenerate spectra
  and produced wrong eigenvalues on the `k`-sized P3 hand-derived matrix. Jacobi, though O(n³) per
  sweep, is unconditionally stable for n ≤ 32 dense-symmetric (our only dense consumer) and hit
  1e-9 orthonormality + correct λ on every oracle. The plan's own §8 Ananke argument ("regression
  structurally impossible") still holds: `eigenvalues_contig` and its 8 oracle tests are untouched;
  only the *new* `eigh_contig` internals differ. Determinism contract unchanged (fixed max sweeps,
  fixed rotation order, sign fixed by first-nonzero > 0).
- **Cost:** Jacobi is the slower of the two for large n, but n ≤ 32 → negligible; the plan's own
  n ≤ 32 stack-only bound makes this a non-issue. No consumer needs the tridiagonal intermediate.
- **Upgrade trigger (per `innovate:` convention):** if a dense n ≫ 32 symmetric consumer appears,
  swap `eigh_contig`'s inner loop for the spec'd implicit-QL with properly-accumulated Givens and
  re-pin `r2_eigh_facade_p3_kat`. Until then Jacobi stays.

### 9.2 Sparse path: LCG-seeded start, NOT index-graded
- **Plan said (§5.1 `topk_symmetric`, §3 determinism contract):** *index-graded start vector*
  (deterministic start derived from row indices), matching the PPR/spmv determinism contract.
- **Actual (committed, `spectral.rs::topk_symmetric`):** start vector is seeded by a **fixed-seed
  LCG** (`x₀[i] = lcg(seed, i)`), not from index-graded spmv state. Power+Hotelling-deflation
  (A := A − λ v vᵀ per spmv) is otherwise exactly as spec'd: fixed `iters`, fixed summation order
  inherited from `Csr::spmv`, sign fixed as in `eigh_contig`, descending |λ|.
- **Why:** an index-graded start produced eigenvalue-order instability across runs on the oracle
  (the `r3_topk_symmetric_*` tests asserted a *deterministic* λ ordering; index-graded starts
  occasionally flipped the k-th vs (k+1)-th pair on clustered spectra). A fixed-seed LCG start is
  byte-identical across runs/hosts for the same binary and removed the ordering nondeterminism. The
  determinism *guarantee* the plan required (cross-run byte-determinism) is still met — just via a
  seeded PRNG rather than index arithmetic.
- **Cost:** none for correctness; the determinism story is now "determinism-per-host + per-fixed-
  seed" (same as the PPR/matmul fallback the plan itself permitted at §7), not "index-graded closed
  form." Documented in `topk_symmetric`'s doc comment.
- **Upgrade trigger:** if a consumer requires the start vector to be a *function of the matrix
  structure* (e.g. warm-start from a prior decomposition), reintroduce index-graded starts and
  re-pin the ordering KAT.

### 9.3 Closure evidence (falsifiable, run this session)
- `cargo test --lib` (kernel) → **561 passed** (+11 vs pre-R1-R3 baseline 550).
- Key KATs GREEN: `r1_*` (Householder + eigh_contig), `r2_eigh_facade_p3_kat` (UᵀU=I to 1e-9,
  correct λ on P3 hand-derived matrix), `r3_topk_symmetric_*` (deterministic top-k, descending |λ|,
  byte-stable across repeated runs).
- No existing public signature changed; `eigenvalues_contig` + 8 oracle tests untouched (§8 Ananke
  claim verified, not assumed).
- Remaining honest gap: §10-style "3-eigensolver dual-authority" grep rule + module doc-comment
  guard added in `householder.rs` (R1) — the `lowrank.rs` second-authority was never created, so
  single eigen-surface (`spectral.rs`) holds.
