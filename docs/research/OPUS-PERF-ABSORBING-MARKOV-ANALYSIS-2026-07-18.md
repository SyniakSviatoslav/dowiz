# OPUS Perf Analysis — `kernel/src/absorbing.rs` (absorbing-Markov fundamental matrix)

**Date:** 2026-07-18
**Author:** Opus research pass (sibling to the `ppr.rs` pass)
**File under study:** `/root/dowiz/kernel/src/absorbing.rs`
**Verdict (one line):** **NON-ISSUE.** `fundamental_matrix` has **zero production/WASM/FFI callers**, runs only on the **fixed 5×5 order-lifecycle FSM** (`n` is bounded, not data-scaled), and is already anchored by a bench. **No rewrite warranted.** Two doc nits + a growth-characterization bench are the entire actionable surface.

---

## STEP 1 — GROUND TRUTH (the load-bearing question)

### 1.1 Every call site, repo-wide

`fundamental_matrix`, `expected_steps`, and `absorption_probs` are called from **exactly two places**, both non-product:

| Caller | Location | What it is |
|---|---|---|
| Own unit tests | `kernel/src/absorbing.rs:90, 124, 137, 167` | 4 tests (2 green fixtures, 1 property, 1 red refuse) |
| Criterion bench | `kernel/benches/criterion.rs:215-216` | `absorbing/fundamental_matrix_16` |

**There are no other callers anywhere in the repo.** Verified by:
- `grep -rn "fundamental_matrix\|absorption_probs\|expected_steps\|absorbing::"` over all `*.rs/*.ts/*.tsx/*.js` → only the file itself + `criterion.rs`.
- No `wasm-bindgen`/`extern`/FFI wrapper mentions these fns (the only `wasm` mention is a doc comment in `mat.rs:8`).
- `mod absorbing` is declared `pub` at `kernel/src/lib.rs:18` (so a consumer *could* exist), but none does.
- `git log -- kernel/src/absorbing.rs` shows only two commits: `18b46ac0d` (the original "R3 funnel closed forms" feature) and `5ca497e1c` (the matmul→`Mat` consolidation refactor). It is a **reverse-engineering / derivation artifact**, never wired into a decision path.

> The bench's own doc comment (`criterion.rs:202-203`) claims the fundamental matrix is *"used by agentic decision gating."* **This is aspirational, not true** — grep proves no such call site exists. That is a documentation nit to correct (see §4.2), not evidence of a hot path.

### 1.2 What `Q` actually is — is `n` ever data-scaled?

**`n` is bounded at 5. It never grows with order/data volume.** The doc comment (`absorbing.rs:3-14`) fixes the transient set as the order-lifecycle FSM:

```
T = {Pending, Confirmed, Preparing, Ready, InDelivery}   → |T| = 5   (absorbing.rs:4)
longest lifecycle path = 4 edges ⇒ Q⁵ = 0                            (absorbing.rs:10)
```

Every real `Q` is the **5×5** transient sub-block of this one FSM (the test fixture `lifecycle_qr()` at `absorbing.rs:99-118` is the canonical instance). The chain is **per-*state-machine*, not per-*order***: processing a million orders does not enlarge `Q` — each order is one *walk* through the same fixed 5-node graph. There is **no aggregate / mesh-wide / per-order matrix** anywhere that feeds a growing `Q` into this function. The only `n≠5` instance in the whole repo is the synthetic `n=16` in the bench, which exists purely to anchor a baseline.

**Consequence:** at the real `n=5`, `n³ = 125` and the finite Neumann series terminates on `Q⁵ = 0` after ≤5 matmuls of a 5×5 — sub-microsecond, dominated by allocation, not arithmetic. The O(n³)/O(n⁴) "scaling risk" is a **benchmarking gap, not a real scaling risk.**

### 1.3 The kernel matmul — `matmul_contig` (`mat.rs:132-151`)

- **Implementation:** naive `i-k-j` triple loop, **O(m·k·n)**, over a **single contiguous row-major `Vec<f64>`** (`Mat`, `mat.rs:12-21`). No BLAS, no cache-blocking, no explicit SIMD.
- **Optimizations present:** (a) the `i-k-j` loop order is the **cache-friendly** one — the inner `j` loop streams `b`'s row and `c`'s row over contiguous memory, which the compiler can auto-vectorize (`mat.rs:144-147`); (b) an `aik == 0.0` short-circuit skips zero source elements (`mat.rs:141-143`), preserving sparse-ish speedups; (c) an arena-backed twin `matmul_contig_in` (`mat.rs:161-185`) keeps the scratch buffer off the heap.
- **Fit for small `n`:** **good.** At `n=5` the entire matrix (25 f64 = 200 B) is L1-resident; a BLAS call's dispatch overhead would *lose* to this inlined loop. Cache-friendliness matters more than asymptotics here, and the code already has it. The one avoidable cost is the `Vec<Vec<f64>>` ⇄ `Mat` conversion on **every** `matmul` call (`absorbing.rs:27-35`) — an allocation per iteration — but at `n=5` this is still sub-µs and not worth churning.

### 1.4 Actual complexity of `fundamental_matrix` (a minor correction to the doc)

The loop (`absorbing.rs:52-62`) runs **up to `n` matmuls**, each **O(n³)**. So the true worst case is **O(n⁴)**, not the O(n³) the bench comment (`criterion.rs:202`) and mental model suggest. Precisely:

- **Nilpotent DAG of longest-path length `L`:** terminates at `Q^{L+1}=0` after `L+1` matmuls ⇒ **O(L·n³)**. Real FSM: `L=4, n=5` ⇒ ~5 matmuls of 5³ ⇒ ~625 core FLOPs, then early-exit (`absorbing.rs:54-55`).
- **Non-nilpotent (cyclic) `Q`:** never early-exits ⇒ full `n` matmuls ⇒ **O(n⁴)**, then returns `None` (`absorbing.rs:63`).

### 1.5 Existing coverage / historical numbers

- **Bench exists:** `absorbing/fundamental_matrix_16` (`criterion.rs:205-218`).
- **Baseline:** `26655 ns` ≈ **26.7 µs** (`kernel/benches/baseline.json:2`).
- **History:** `26655 → 26727 ns (+0.3%)` (`kernel/benches/BENCH_HISTORY.md:309`) — flat/stable.
- **Regression ledger:** `docs/regressions/REGRESSION-LEDGER.md` has **no** absorbing-chain entry (the only "Markov" hit is the unrelated loop-detector, ledger row 18).
- **Bench realism caveat:** the bench's `Q` uses modular wraparound — `q[i][(i+1)%n]` and `q[i][(i+3)%n]` (`criterion.rs:210-213`) — which makes the graph **cyclic**, so the bench actually measures the **pessimal `None`-returning O(n⁴) path** (all 16 iterations), *not* the DAG early-exit that real usage takes. It over-states cost and never exercises the real code path. 26.7 µs is the *worst* case at n=16, and even that is negligible.

---

## STEP 2 — ALGORITHMIC RESEARCH (invested lightly, since Step 1 found no large-`n` use case)

The math is standard and worth recording so the "if it ever grows" question is answered with citations rather than re-derived later.

### 2.1 The closed form is textbook-correct

For `P = [[Q,R],[0,I]]`, the fundamental matrix is `N = (I−Q)⁻¹ = I + Q + Q² + …` (Neumann series), `t = N·1` = expected steps to absorption, `B = N·R` = absorption probabilities. This is exactly what `absorbing.rs` implements, and it matches Grinstead & Snell (LibreTexts 11.2) and Wikipedia's absorbing-chain page:
- <https://stats.libretexts.org/Bookshelves/Probability_Theory/Introductory_Probability_(Grinstead_and_Snell)/11%3A_Markov_Chains/11.02%3A_Absorbing_Markov_Chains>
- <https://en.wikipedia.org/wiki/Absorbing_Markov_chain>
- <https://lips.cs.princeton.edu/the-fundamental-matrix-of-a-finite-markov-chain/>

### 2.2 The DAG/nilpotent guarantee IS the key to a better-than-Neumann algorithm

The file's own nilpotency guarantee (transient subgraph is a DAG) is precisely the hypothesis under which the current O(n⁴) series is **the worst choice**, and a linear/quadratic algorithm exists:

1. **You rarely need the full `N`.** The product artifacts are `t = N·1` and `B = N·R`, i.e. solve **`(I−Q)X = [1 | R]`**. Forming `N` explicitly (the Neumann series) is wasted work.

2. **Topological order ⇒ triangular solve.** Order the transient states topologically (a DAG always admits this, in **O(V+E)** — CMU 15-451 §DAGs, TU-Delft 3.6). Then `Q` is strictly triangular and `(I−Q)` is **unit-triangular**, so each right-hand side is solved by **forward/back-substitution in O(n²) dense** (O(nnz) sparse) — the standard triangular-solve cost (Algowiki; Cornell CS4220 ch.6). For `t` alone that's a single O(n²) solve; for `B` it is `m` solves. This replaces **O(n⁴)** with **O((1+m)·n²)**.
   - <https://www.cs.cmu.edu/~yangp/15-451/lecture3.pdf>
   - <https://algowiki-project.org/en/Forward_substitution>
   - <https://www.cs.cornell.edu/courses/cs4220/2014sp/CVLBook/chap6.pdf>

3. **Linear-time optimum for `t` (backward induction / DP over the DAG).** `t[i] = 1 + Σ_j Q[i][j]·t[j]`, evaluated in **reverse topological order**, is a one-pass DP over the DAG: **O(V + E) = O(n + nnz)**. This is the asymptotic floor and needs no matrix at all. (Same DAG-DP framing as CMU 15-451 §3; absorption/hitting-time recurrences with boundary conditions per Grinstead & Snell.)

4. **Even the full `N`, if genuinely needed:** forward-substitution against the `n` identity columns is **O(n³) dense / O(n·nnz) sparse** — still strictly better than the O(n⁴) Neumann loop.

### 2.3 Iterative solvers — the *hypothetical* large-and-cyclic tier only

Iterative Krylov solvers become relevant **only if `Q` were both large *and* cyclic** — a case `absorbing.rs` **explicitly refuses** (`absorbing.rs:13-14, 63`, returns `None`). For completeness:
- `(I−Q)` for a substochastic `Q` (ρ(Q)<1) is a nonsingular **M-matrix**, generally **nonsymmetric** ⇒ **Conjugate Gradient does not apply** (CG needs SPD). Use **GMRES** (Arnoldi, restart for memory) or **BiCGStab** (short-recurrence, lower memory) for large sparse nonsymmetric systems.
  - <https://www.dmsa.unipd.it/~berga/Teaching/Phd/gmres_slides.pdf>
  - <https://maths-people.anu.edu.au/brent/pd/rpb206.pdf>
  - <https://arxiv.org/pdf/1607.00351> (preconditioned Krylov comparison, large nonsymmetric)
- **But for a DAG (the only case this code accepts), iterative solvers are the wrong tool** — the direct triangular solve of §2.2 is *exact* and cheaper. So even the growth tier resolves to "topological direct solve," not GMRES, as long as the refuse-on-cycle guard holds.

---

## STEP 3 — CALIBRATED RECOMMENDATION (three-tier, honest)

| Tier | Trigger | Action |
|---|---|---|
| **① Small/bounded (THIS CASE)** | `n` fixed at 5, zero production callers | **Bench it, document the bound, no rewrite.** ✅ |
| ② Genuinely sparse/growing DAG | a real caller feeds `n ≫ 5` DAG `Q` | Swap Neumann series → **topological triangular solve** (§2.2): O(n⁴)→O((1+m)·n²); DP for `t` alone → O(n+nnz). Determinism-preserving (exact, no tolerance). |
| ③ Large *and* cyclic | would require lifting the refuse-on-cycle guard | **Operator sign-off.** Redesign to preconditioned GMRES/BiCGStab (§2.3) — but this contradicts the current DAG invariant and the FSM golden-signature; do not do it silently. |

**We are unambiguously in Tier ①.** Manufacturing a rewrite here would be over-engineering against a 5×5 matrix nobody calls. The correct outputs are minimal:

### Actionable items (all optional, none urgent)

1. **Keep the code as-is.** It is mathematically exact, correctly guarded (refuses non-DAG `Q`), and already benched. This is the right lazy-senior default.
2. **Doc nit A (`absorbing.rs` header / `criterion.rs:202`):** the worst-case cost is **O(n⁴)** (up to `n` matmuls × O(n³) each), not O(n³). One-line correction.
3. **Doc nit B (`criterion.rs:202-203`):** delete/soften *"used by agentic decision gating"* — no caller exists; call it "unused closed-form derivation; benched to anchor the kernel matmul path."
4. **Bench nit (optional):** the current `n=16` bench is cyclic ⇒ measures the pessimal `None` path, not the real DAG early-exit. Add a **DAG** instance so both paths are characterized (below).

---

## STEP 4 — CONCRETE BENCH PROPOSAL

**Goal:** (a) prove the real path is negligible; (b) characterize the actual growth curve of the O(n⁴) worst case even though real usage never reaches it. Add to `kernel/benches/criterion.rs`, register in `criterion_group!`, and add baselines to `baseline.json`.

```rust
/// Realistic path: the actual 5-state order-lifecycle FSM (DAG ⇒ early-exit on Q⁵=0).
/// This is the ONLY shape real usage ever produces; expect sub-µs.
fn bench_absorbing_lifecycle_5(c: &mut Criterion) {
    let t3 = 1.0 / 3.0;
    let q = vec![
        vec![0.0, t3, 0.0, 0.0, 0.0],   // Pending → Confirmed
        vec![0.0, 0.0, 0.5, 0.0, 0.5],  // Confirmed → Preparing, InDelivery
        vec![0.0, 0.0, 0.0, 1.0, 0.0],  // Preparing → Ready
        vec![0.0, 0.0, 0.0, 0.0, 0.5],  // Ready → InDelivery
        vec![0.0, 0.0, 0.0, 0.0, 0.0],  // InDelivery → (absorb)
    ];
    c.bench_function("absorbing/fundamental_matrix_lifecycle_5", |b| {
        b.iter(|| black_box(absorbing::fundamental_matrix(black_box(&q))))
    });
}

/// Stress sweep: pure upper-triangular DAG chain (0→1→…→n-1). Nilpotent ⇒ exercises the
/// REAL early-exit path at growing n, isolating the O(depth·n³) curve real usage would follow
/// if n ever grew while staying a DAG. n far beyond any real FSM — characterization only.
fn bench_absorbing_dag_sweep(c: &mut Criterion) {
    let mut g = c.benchmark_group("absorbing/dag_chain");
    for &n in &[8usize, 16, 32, 64, 128, 256] {
        let mut q = vec![vec![0.0f64; n]; n];
        for i in 0..n - 1 { q[i][i + 1] = 1.0; }        // strict upper-triangular DAG
        g.bench_with_input(BenchmarkId::from_parameter(n), &q, |b, q| {
            b.iter(|| black_box(absorbing::fundamental_matrix(black_box(q))))
        });
    }
    g.finish();
}
```

Keep the existing `fundamental_matrix_16` (relabel it `_cyclic_16` — it measures the pessimal `None`/O(n⁴) path). **Criterion of success (grounded in Step 1):**

- **Real path:** `fundamental_matrix_lifecycle_5` should land in the **hundreds of ns** (allocation-bound, not arithmetic). This is the number that proves "non-issue."
- **Growth characterization:** the `dag_chain` sweep should show a **super-cubic** slope (≈ n⁴ for the strict chain, whose longest path = n−1 ⇒ no early exit). If the slope is ever *worse* than ~n⁴, or a real caller ever appears feeding n≳32, **re-open Tier ②** and implement the topological triangular solve (§2.2). Until then, the bench is a tripwire, not a mandate.

**Bottom line:** the "O(n³) blind spot" is a benchmarking gap on dead-simple, uncalled code operating on a fixed 5×5 matrix — not a scaling risk. Correct two doc lines, add the two benches above as a tripwire, and move on. No rewrite.
