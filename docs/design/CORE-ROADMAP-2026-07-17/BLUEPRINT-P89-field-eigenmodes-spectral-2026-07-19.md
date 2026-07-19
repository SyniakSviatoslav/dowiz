# BLUEPRINT P89 — Field eigenmodes via kernel `spectral.rs` (the falsifiable bet) (2026-07-19)

> **Standalone DESIGN blueprint (dowiz `kernel/` spectral + `engine/` field-render, CPU-only).** One
> coherent, independently-buildable unit against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B3 and
> `OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md` — **both of which recommended DCT/FFT, NOT
> `spectral.rs`.** The operator OVERRULED that recommendation; this unit builds the operator's way and
> names the metric that settles it. Divergence recorded in
> `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §2 row 3 (operator item D) and §4.5. Format
> precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/dowiz` at
> HEAD, read live this pass.
>
> **One sentence:** realize the field engine's modal needs (FE-10 impulse response, G5 near-settle
> truncation) by calling the EXISTING kernel `spectral.rs` (`topk_symmetric` over the graph Laplacian)
> rather than a new FFT/DCT spectral-ocean — a **falsifiable bet** whose success is decided by three
> reconciliation tests plus a three-path head-to-head benchmark, with the numbers, not this document,
> making the call.

---

## THE BET (stated up front — this is a bet, not a foregone conclusion)

The operator's framing is binding and quoted: *"if metrics show I'm wrong I'll admit it."* This
blueprint therefore states the bet, the exact metric, the threshold, and **what happens if it fails**
— not an assumption of success.

**The claim being bet:** for the two designed homes — **FE-10 sparse-impulse response** and **G5
near-settle advance**, where the mode count `r ≤ ~16` — a numerical eigensolve of the field domain's
Laplacian via kernel `spectral.rs` meets the frame budget and the E1 energy tolerance, and is
therefore the right modal engine, **displacing the DCT/FFT the research recommended.**

**Two named ways the bet can WIN:**
1. Correctness (§4): the sign/domain reconciliation T1–T3 go green — the graph Laplacian's
   eigenvectors ARE the field's, with negated eigenvalues.
2. Cost (§6): at `r ≤ 16` the `spectral.rs` path meets the P81 frame budget for FE-10/G5 **at real
   grid scale** and within E1 tolerance.

**Three named ways the bet can LOSE — each recorded up front, each triggering the operator's exit:**
1. **The wrong-end-of-the-spectrum obstacle (the load-bearing risk, §5).** At real grid scale the
   field is `n = W·H ≫ 32` cells, so `spectral.rs` routes through the SPARSE tier `topk_symmetric`,
   which returns the **LARGEST**-|λ| modes — the **fastest-decaying** field modes — while FE-10/G5 need
   the **SMALLEST**-λ (slowest) modes. The kernel has **no shift-invert solver**
   (`spectral_laplacian.rs:70-74`, verbatim). The bet REQUIRES a shifted-operator wrapper (§5); if that
   wrapper's power iteration cannot converge cheaply (the Laplacian's smallest modes cluster near λ=0,
   the worst case for power iteration), the precompute blows the budget and **the bet loses on cost
   even if T1–T3 pass.**
2. **Full-field evolution (all `n` modes):** DCT/FFT wins by construction — O(n log n) vs O(r·n) with
   r→n. This loss is **expected and recorded here up front**; if a future need is full-spectrum, B wins
   and the operator's exit applies for that home.
3. **T1–T3 fail numerically** (the reconciliation does not hold to tolerance): the whole approach is
   wrong for this domain and DCT stands. (Least likely — the math in §4 is sound — but it is a real RED
   gate, run first.)

**The exit is honest and pre-committed:** the DoD (§8) is *"T1–T3 green AND the three-path bench table
filled with measured numbers AND a written verdict paragraph citing those numbers — in either
direction."* A losing bench is a valid, complete deliverable that records the loss. **P89 is not done
by shipping a modal engine; it is done by producing the number that decides whether to.**

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim read from source **this pass** (`/root/dowiz`, HEAD).

### 0.1 The two Laplacians and their sign split (the reconciliation's subject)

- **Field stencil = `−(D−A)`** (negative-definite): `field_frame.rs::laplacian` computes
  `left+right+up+down − 4·u` (`field_frame.rs:151`), documented as the physics Laplacian `∇² = −(D−A)`
  (`field_frame.rs:119-128`, TORVALDS-21).
- **Kernel graph Laplacian = `+(D−A)`** (PSD, `L·1 = 0`): `Csr::laplacian_spmv`
  (`kernel/src/csr.rs:552-600`), `Incidence::laplacian` (`kernel/src/incidence.rs:107`),
  `spectral::laplacian` (`kernel/src/spectral.rs:620`).
- **The split is already pinned by a live test:** `field_energy.rs:201-248`
  (`sign_pin_field_frame_stencil_is_negative_incidence`) proves `stencil == −incidence.laplacian` on
  interior nodes and that the naive `+` form is FALSE there. So `λ_field = −λ_graph` with **identical
  eigenvectors** is not a hope — half of it is already a green test. P89's T1–T2 extend it to the
  eigenpairs.

### 0.2 The sparse eigensolver P89 must use — and its documented smallest-λ gap (THE obstacle)

`spectral::eigh` (`kernel/src/spectral.rs:251-261`) is dense and **capped at `n ≤ 32`**
(`:253` debug_assert). `laplacian_eigenmodes` (`kernel/src/spectral_laplacian.rs:83-101`) dispatches:
`n ≤ 32` → dense `eigh` (ascending, smallest modes — but this is the "field-UI regime" only for a
**tiny** grid); **`n > 32` → `topk_symmetric`, which returns the DOMINANT (largest-|λ|) modes**
(`spectral_laplacian.rs:96-99`). The module doc states the gap verbatim (`:70-74`):

> "at `n > 32` the result is therefore the LARGEST-magnitude modes, NOT the smallest — a shift-invert
> solver (absent in the kernel) would be required for the true smallest eigenvalues at scale."

`topk_symmetric` itself (`spectral.rs:269-402`): fixed-iteration power method + Hotelling deflation
over `Csr::spmv`, **deterministic** (fixed-seed LCG start `:290`, fixed `iters`, fixed summation order,
sign-fixed `:380-392`), returns pairs descending `|λ|` (`:396-401`). **This determinism discipline is a
real point in the bet's favor** (a hand-rolled FFT would have to re-earn it), but it solves for the
WRONG end of the spectrum for FE-10/G5. §5 is the whole obstacle.

### 0.3 A real field grid is `n ≫ 32` — so P89 lives in the sparse tier, not the dense one

`FieldFrame` grids in the tests run `24²`, `40×28`, and the settle test uses `24²`
(`field_frame.rs:382-425`); the P38 particle/field budget assumes far larger (`128²…512²`,
P38 §4.2 / P87 §7). `n = W·H` = 576 at 24², 16 384 at 128². **All ≫ 32.** Therefore the FE-10/G5 modal
solve is firmly in the `topk_symmetric` sparse tier — the dense `eigh` smallest-mode path is
unreachable at real scale. This is why §5's obstacle is load-bearing and not a corner case.

### 0.4 The analytic DCT modes the reconciliation tests against (path B's basis)

For a rectangular `N×M` Neumann-grid 5-point Laplacian, the eigenvectors are the separable cosines
`φ_{p,q}(x,y) = cos(πp(x+½)/N)·cos(πq(y+½)/M)` (the DCT-II basis) with graph-Laplacian eigenvalues
`λ_{p,q} = 2(2 − cos(πp/N) − cos(πq/M))`, `p∈0..N-1`, `q∈0..M-1`. This is the closed form T1/T2 assert
`spectral.rs`'s numerical modes against, and it is exactly path B's "free" analytic basis.

### 0.5 The consumer seam is CPU-precompute-then-consume — P89 is NOT GPU-gated

The FE-07 bridge contract is "flat f64 array, zero eigen-math in the engine" (P38 §4.6,
`bridge.rs:662` spectral output consumed in-process). P79-B6 provides the **contiguous eigenvector
flatten** (a `k·n` f64 buffer). P89 precomputes on the CPU once per grid/domain and the engine consumes
the flat buffer — the DyRT (decompose-once, reconstruct-per-frame) pattern `OPUS-SPECTRAL-EVERYWHERE`
§2a confirmed. **No GPU eigensolve → P89 is NOT gated on P38 §4.2** (unlike P86/P87). It is pure CPU,
buildable now given P79 + P81.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P89 uses it — and what it does NOT take |
|---|---|---|
| **kernel `spectral.rs` `topk_symmetric`** (`spectral.rs:269-402`) | deterministic sparse top-k eigenpairs of a symmetric `Csr` via deflated power iteration | **The whole engine of the bet** (path A). **NOT taken raw:** it returns largest-|λ|; §5 wraps it in a shift to reach the smallest modes FE-10/G5 need. |
| **Analytic DCT/FFT modes** (`OPUS-PHYSICS-WAVE` §B3, the research's pick) | for a rectangular Neumann grid the eigenmodes ARE the DCT-II basis, computable in O(n log n) with no eigensolve | **NOT taken as the engine** (operator overrule). **Taken as the ORACLE** — T1/T2 assert `spectral.rs` against the analytic DCT (§4); path B in the bench (§7). The disagreement is settled by the number, not by fiat. |
| **Shift / spectral-transform for smallest eigenvalues** (`(σI − L)`; standard in ARPACK-style solvers) | the largest eigenvalues of `σI − L` are the smallest of `L`, same eigenvectors | **Adopt as a thin `spmv` wrapper** (§5) so the existing power method reaches the slow modes — the minimal addition that makes the bet buildable at scale. **NOT taken:** a full shift-invert (needs a linear solve; the kernel has none) — the additive shift is the cheap half. |
| **DyRT decompose-once / reconstruct-per-frame** (`OPUS-SPECTRAL-EVERYWHERE` §2a) | precompute the modal basis once per domain; per frame is a cheap O(r·n) reconstruction | **Adopt** — CPU precompute, engine consumes the flat buffer (§0.5). |
| **P79-B6 contiguous evec flatten** | a `k·n` f64 buffer feeding FE-07's "flat array, zero engine eigen-math" contract | **Adopt as the handoff format** (§3). |
| **Phase-28 single-eigen-surface ruling** (`spectral.rs` is THE eigen surface) | one authoritative spectral module, no second one | **Honored** — a DCT module would create a second spectral authority; this is a structural argument FOR the operator's path (§6). |

---

## 2. Scope — what P89 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P89 OWNS

1. **The sign/domain reconciliation** T1–T3 (§4): eigenvector identity, eigenvalue map, evolution
   equivalence — the RED→GREEN gate run before anything else.
2. **The smallest-eigenvalue shift wrapper** over `topk_symmetric` (§5): the additive `(σI − L)`
   `spmv` shim that lets the existing power method reach the slow modes FE-10/G5 need.
3. **The two modal homes:** FE-10 sparse-impulse response and G5 near-settle advance, each a modal
   reconstruction consuming the P79-B6 flat buffer.
4. **The three-path head-to-head benchmark** (§7) — the verdict mechanism; no other document may
   pre-empt the modal-vs-DCT call.
5. **The written verdict paragraph** citing the measured numbers, in either direction (§8 DoD).

### 2.2 P89 does NOT own (anti-scope)

- **A new FFT/DCT module.** The bet is precisely to NOT build one; the analytic DCT appears only as the
  T1/T2 oracle and the path-B bench baseline (a few lines of closed form, not a module).
- **A GPU eigensolve.** P89 is CPU-only (§0.5); it does not touch P38 §4.2.
- **A full shift-invert solver.** §5 adds only the additive shift `spmv` wrapper; a linear-solve-based
  shift-invert is explicitly out of scope (flagged as a fallback in §5 if the additive shift's
  convergence fails).
- **`money.rs` / the determinism oracle.** Money is `i64` exact, spectral/float is walled from it
  (`SYNTHESIS §5`, `OPUS-SPECTRAL-EVERYWHERE` §3 "money = absolute wall"). P89 is presentation-side
  field math only.
- **The engine doing eigen-math.** FE-07 forbids it; the engine consumes the flat buffer, the kernel
  owns every eigen-operation (§0.5).

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs:** `spectral::topk_symmetric` + `spectral_laplacian::laplacian_eigenmodes`
(`kernel/src/spectral.rs`, `spectral_laplacian.rs`); `Csr`/`Csr::spmv`/`laplacian_spmv`
(`kernel/src/csr.rs`); the sign-pin (`engine/src/field_energy.rs:201-248`); the field stencil +
`compose()` oracle (`engine/src/field_frame.rs`); the E1 energy tolerance `TOL_E`
(`field_energy.rs:165`). **P79-B6** (contiguous evec flatten — the handoff buffer). **P81** (engine
bench harness — the §7 substrate). **P75** (bench-id/baseline schema the §7 benches register into).
**Consumers:** FE-10 (impulse response), G5 (settle-region truncation) in the engine render path.

### 2.4 Honest reconciliation with the research (standard §2 item 6)

The research (`OPUS-PHYSICS-WAVE` §B3, `OPUS-SPECTRAL-EVERYWHERE`) recommended DCT/FFT and is
**not wrong on the rectangular-grid case**: for a perfect rectangular Neumann grid the modes ARE the
DCT basis, free and O(n log n). P89 does not claim otherwise — it bets that (a) the *domain-general*
case (masked/SDF-carved/widget-coupled fields) makes DCT simply *wrong* (§6 steelman), and (b) at
`r ≤ 16` the numerical path is cheap enough for the two designed homes to not need DCT's asymptotic
edge. Both are empirical; §7 measures them. Where the research is right (full-field evolution), §7
records the expected DCT win rather than hiding it.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

```rust
// kernel/src/spectral.rs  (EXTEND — the shift wrapper lives beside topk_symmetric, single surface)

/// Additive spectral shift for reaching the SMALLEST eigenvalues of a PSD Laplacian via the existing
/// largest-|λ| power method: the dominant eigenpairs of (σI − L) are the smallest of L, same vectors.
/// σ MUST be ≥ λ_max(L) so (σI − L) is PSD and its dominant end is L's small end (§5).
/// For a 2-D 5-point graph Laplacian λ_max ≤ 8 (degree bound); SHIFT_SIGMA_GRID2D pins a safe σ.
pub const SHIFT_SIGMA_GRID2D: f64 = 8.0;

/// Top-r SMALLEST eigenpairs of a symmetric PSD `Csr` L, via a shifted power method over
/// `spmv_shifted(x) = σ·x − L·x`. Deterministic (inherits topk_symmetric's fixed seed/iters/order).
/// Returns (basis, values) where `values` are the RECOVERED L-eigenvalues λ = σ − μ (μ = shift
/// eigenvalue), ascending. This is the wrapper the FE-10/G5 homes actually call.
pub fn topk_smallest_symmetric(
    l: &crate::csr::Csr,   // the graph Laplacian +(D−A) as a Csr
    r: usize,
    sigma: f64,            // ≥ λ_max(L)
    iters: usize,
) -> crate::spectral_cache::Decomp { /* §5 */ }
```

```rust
// engine/src/field_modal.rs  (NEW; CPU-side modal reconstruction — consumes the kernel flat buffer,
//                             does ZERO eigen-math itself, per FE-07)

/// A precomputed modal basis for one field domain: r eigenvectors (P79-B6 contiguous k·n flatten) +
/// their FIELD eigenvalues λ_field = −λ_graph (sign flip applied ONCE here, §0.1). Precomputed once
/// per grid/domain on the CPU; consumed per frame. NO eigen-math in the engine — this is data.
#[derive(Debug, Clone)]
pub struct FieldModalBasis {
    pub r: usize,
    pub n: usize,               // == w*h
    pub evecs: Vec<f64>,        // r*n contiguous (P79-B6 layout); evecs[k*n + i]
    pub lambda_field: Vec<f64>, // r entries; λ_field[k] = −λ_graph[k]  (negated, §0.1)
}

/// The mode count for the two designed homes. The bet is scoped to r ≤ this; beyond it DCT wins (§7).
pub const MODAL_R_MAX: usize = 16;
```

---

## 4. The reconciliation — T1–T3 (RED→GREEN before anything else) (standard §2 items 2, 3, 5)

The reports' objection is real (graph `+(D−A)` vs field `−(D−A)`), but reconcilable **in principle**:
a regular Neumann grid IS a lattice graph, so the two operators share **identical eigenvectors** with
**negated eigenvalues**. Whether that holds **numerically** is T1–T2; whether it drives the field
correctly is T3. These run FIRST — if they fail, the approach is wrong and the bench is moot.

### 4.1 T1 — eigenvector identity (numerical)

- **Spec.** Build the `N×M` grid's graph Laplacian as a `Csr` (via `lattice_csr`,
  `field_energy.rs:35-53` pattern). Run the smallest-mode solve (§5) for the first `r` modes. Assert
  `|⟨φ_k^spectral, φ_k^DCT⟩| ≥ 1 − 1e-6` against the analytic separable-cosine modes (§0.4), handling
  **degenerate eigenvalue subspaces by subspace angle**, not vector-by-vector match (a repeated λ has
  a rotation-free basis only up to the subspace).
- **RED `red_spectral_modes_match_dct_basis`:** RED if any non-degenerate mode's overlap < 1−1e-6, or
  if a degenerate subspace's principal angle exceeds tolerance. GREEN confirms `spectral.rs` recovers
  the DCT eigenvectors numerically.
- **Adversarial `red_degenerate_subspace_handled`:** a square grid (`N==M`) has degenerate modes
  (`λ_{p,q} = λ_{q,p}`); assert the test uses subspace angle there and does NOT demand an exact vector
  match (which would spuriously fail on a valid basis rotation).

### 4.2 T2 — eigenvalue map (analytic)

- **Spec.** Assert the recovered graph eigenvalues `λ_k^graph` match `2(2−cos(πp/N)−cos(πq/M))` (§0.4)
  within tolerance, and that the field-side advance uses `λ_field = −λ_graph` with the damped-oscillator
  closed form (the `FieldEquilibrium` scheme, `field_frame.rs:23-30`).
- **RED `red_eigenvalues_match_analytic`:** RED if any `λ_k` deviates beyond tolerance. GREEN confirms
  the eigenvalue map (and, with T1, that `spectral.rs` solved the right operator).
- **RED `red_field_eigenvalue_is_negated_graph`:** assert `λ_field[k] == −λ_graph[k]` exactly in
  `FieldModalBasis` construction (the sign flip applied once, §0.1) — pins the one place the sign lives.

### 4.3 T3 — evolution equivalence (the physics)

- **Spec.** An `r`-mode modal advance of a smooth initial field (project onto the r modes, advance each
  by its `λ_field` damped-oscillator closed form, reconstruct) matches the stencil `step()` evolution
  within the E1 energy-gate tolerance (`field_energy.rs`, `lyapunov_nonincreasing`) over `M` steps.
- **RED `red_modal_advance_matches_stencil`:** RED if the modal reconstruction diverges from `step()`
  beyond the E1 tolerance over M steps for a smooth (low-mode) initial field. GREEN confirms the modal
  engine reproduces the authoritative integrator for the regime it targets.
- **Adversarial `red_modal_advance_fails_high_frequency`:** a field with energy in modes `> r` must
  show the modal advance *diverging* from `step()` (the truncation error is real) — proving the r-mode
  claim is scoped to smooth/near-settle fields, not universal (non-vacuousness: the truncation is a
  genuine approximation with a known failure regime).

---

## 5. The smallest-eigenvalue obstacle — the load-bearing engineering risk (standard §2 item 6)

**This is where the bet is most likely to be lost, and it is stated plainly.** At real grid scale
(`n ≫ 32`, §0.3) `spectral.rs` uses `topk_symmetric`, which returns the **largest**-|λ| modes
(§0.2). FE-10/G5 need the **smallest** (slowest-decaying) modes. The raw call returns the wrong end.

**The fix (in scope):** a thin shifted-`spmv` wrapper, `topk_smallest_symmetric` (§3). Apply the
existing power method to `(σI − L)` with `σ = SHIFT_SIGMA_GRID2D ≥ λ_max(L)`; its dominant eigenpairs
are L's smallest, same eigenvectors, and `λ_graph = σ − μ` recovers them. This is `spmv_shifted(x) =
σ·x − L.laplacian_spmv(x)` — a diagonal + scalar correction over the existing `Csr::spmv`, ~10 lines,
no new solver, staying on the single eigen surface (Phase-28).

**Why the fix may not be enough (the honest cost risk):** power iteration converges at rate
`|μ_2/μ_1|`. For `(σI − L)`, the dominant μ correspond to L's eigenvalues **nearest 0** — and a
Laplacian's small eigenvalues are **clustered near 0** (the spectral gap `λ_2` is small; for a large
grid `λ_2 = O(1/n)`). So `|μ_2/μ_1| → 1` and the power method converges **slowly** exactly where
FE-10/G5 need it. The precompute cost `O(iters · nnz · r)` may need large `iters` to separate clustered
slow modes — potentially blowing the "once per grid/domain" precompute budget. **This is the concrete
mechanism by which the bet loses on cost even with T1–T3 green.** §7's `bench_precompute_spectral`
measures it directly.

**Named fallbacks if the additive shift's convergence fails (NOT built in P89, recorded for the exit):**
(a) a true shift-invert solver (needs a linear solve the kernel lacks — a separate build unit);
(b) accept DCT for the rectangular case and reserve `spectral.rs` only for masked/irregular domains
(§6) where DCT is simply wrong — a *scoped* win rather than a full displacement. The bench decides
which, if any, applies.

---

## 6. The honest steelman of the operator's bet (undersold by the research — recorded because true)

The DCT argument holds ONLY for a **perfect rectangular Neumann grid**. Three points genuinely favor
`spectral.rs`, and the research understated them:

1. **Domain generality.** The moment the field domain is masked or shaped — SDF-carved regions
   (`engine/src/sdf.rs`), widget-graph-coupled fields, obstacles — all plausible in this UI, the
   analytic DCT basis is simply **wrong**, and a numerical eigensolve of the actual domain's Laplacian
   is the **only exact modal method**. `spectral.rs` is the domain-general path; DCT is not.
2. **Single eigen surface (Phase-28).** `spectral.rs` is ruled THE eigen surface; a DCT module would
   create a **second spectral authority** — a structural cost the research did not price.
3. **Determinism already earned.** `topk_symmetric`'s fixed-seed / fixed-iteration / fixed-order / sign
   -fixed discipline (§0.2) is byte-reproducible across runs and paths; a hand-rolled FFT would have to
   re-earn all of it (and FFT libraries are a per-platform-libm hazard the repo avoids,
   `rust-native-bare-metal-decision-2026-07-14`).

**These do not decide the bet — the number does (§7).** They are recorded so the verdict weighs the
real tradeoff, not a strawman: even if DCT wins on the rectangular-grid cost bench, point 1 may make
`spectral.rs` the required engine for the masked-domain homes, a *scoped* win the verdict must state.

---

## 7. The three-path head-to-head bench — the verdict mechanism (standard §2 item 10)

Benched in P81's harness, registered into P75's bench-id schema. **No other document may pre-empt this
table** (Ledger §4 item 14, single-owner contract).

| Path | Precompute | Per-frame (r modes, n cells) | Domain generality |
|---|---|---|---|
| **A. `spectral.rs` `topk_smallest_symmetric`** | `O(iters·nnz·r)` once per grid/domain (§5 — the cost risk) | `O(r·n)` reconstruction + `O(r)` advance | **Any domain** (masked/irregular incl.) |
| **B. DCT/FFT (research's pick)** | none (analytic basis) | `O(n log n)` full-spectrum advance | Rectangular Neumann grids ONLY |
| **C. Stencil `step()` (baseline/oracle)** | none | `O(n)` per step × steps | Any domain; the authority |

**Benches (each a P75-registered id):**
- `bench_precompute_spectral` — path A precompute vs `iters`, per grid {64², 128², 256²} (the §5 risk,
  measured directly — does the shifted power method converge for the clustered slow modes within budget?).
- `bench_perframe_modal_A` / `bench_perframe_dct_B` / `bench_perframe_stencil_C` — per-frame cost at
  `r ∈ {4, 8, 16}` (A/B) vs the full stencil (C), per grid.
- `bench_modal_accuracy` — reconstruction error of A and B vs the C oracle within E1 tolerance.

**Named falsifiable outcome (pre-committed):**
- **FE-10 (r ≤ 16 sparse impulse) + G5 (r ≤ 16 near-settle):** if path A's precompute clears the
  once-per-domain budget AND per-frame meets the P81 frame budget AND accuracy is within E1 tolerance
  → **the bet is CONFIRMED for those homes.**
- **Full-field evolution (r → n):** B wins by construction (`O(n log n)` vs `O(r·n)`) — **expected loss,
  recorded up front**; if a home needs it, the operator's exit applies.
- **If `bench_precompute_spectral` shows non-convergence/budget-blow (§5):** the bet LOSES on cost even
  with T1–T3 green → the verdict records it and either the §5 fallback (b) scoped-to-masked-domains
  applies or DCT stands. **The bench numbers, not this document, make the call.**

---

## 8. DoD — falsifiable, the verdict IS the deliverable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the sign/domain reconciliation holds numerically | `red_spectral_modes_match_dct_basis`, `red_degenerate_subspace_handled` (T1) |
| D2 | the eigenvalue map + field-sign negation are pinned | `red_eigenvalues_match_analytic`, `red_field_eigenvalue_is_negated_graph` (T2) |
| D3 | an r-mode modal advance matches the stencil oracle within E1 tolerance (and fails, as it should, above r) | `red_modal_advance_matches_stencil`, `red_modal_advance_fails_high_frequency` (T3) |
| D4 | the smallest-mode shift wrapper returns L's smallest eigenpairs deterministically | `red_shifted_solve_returns_smallest`, `red_shifted_solve_is_deterministic` (§5) |
| **D-VERDICT** | **the three-path bench table is filled with MEASURED numbers AND a written verdict paragraph cites them (either direction)** | §7 bench output present + a verdict paragraph in this file's addendum / REGRESSION-LEDGER; **a losing verdict is a valid, complete deliverable** |
| D-NOREG | the field oracle stays bit-identical; the engine does zero eigen-math (FE-07) | `compose_returns_deterministic_frame` green; a grep/call-graph assert that `field_modal.rs` calls no `spectral::*` eigen fn (kernel owns it) |

**The DoD is deliberately NOT "a modal engine ships."** Per the bet's framing, done = *the number that
decides* exists, plus the reconciliation that makes the number meaningful. If the number says DCT,
that is a completed P89, not a failed one.

---

## 9. Cross-cutting obligations (standard §2 items 8, 9, 11–16, 20)

- **Schemas & scaling axis (item 8):** axis = **(grid cells n, mode count r)**. The `FieldModalBasis`
  is `r·n` f64 (P79-B6 layout); it scales linearly in both. Break point: the §5 precompute cost, which
  is superlinear in the effective `iters` needed as the slow-mode cluster tightens with n — the named
  risk, measured by `bench_precompute_spectral`. Beyond r ≈ 16 the O(r·n) reconstruction loses to
  DCT's O(n log n) (the §7 crossover).
- **Isolation / bulkhead (item 11):** the modal path is a **presentation-side consumer** of kernel
  spectral output — a modal bug corrupts a *displayed* frame, never the authoritative `step()` state
  (the C oracle always exists as fallback). The engine holds no eigen-math (FE-07), so a spectral
  regression cannot reach engine state logic.
- **Mesh awareness (item 12):** **N/A** — CPU-local precompute + local render; the flat buffer is
  in-process (`bridge.rs:662`), zero transport payload.
- **Rollback / self-healing as math (item 13):** **Snapshot-re-entry** = the modal basis is
  recomputable from the domain Laplacian at any time (decompose-once is regenerative). **Self-Termination**
  = T3's truncation-error bound is a hard invariant (above r, the modal path is *known* wrong and the
  system uses C). **Self-healing (error-correcting): NOT claimed** — modal truncation is an
  approximation, not redundancy.
- **Error-propagation / smart index (item 14):** the bug classes (wrong-sign eigenvalue; wrong end of
  the spectrum; engine doing eigen-math) become **test-time** failures — `red_field_eigenvalue_is_negated_graph`,
  `red_shifted_solve_returns_smallest`, and the FE-07 call-graph assert (D-NOREG).
- **Living-memory awareness (item 15):** the modal basis is a per-domain **precompute cached** artifact
  (decompose-once, reconstruct-many) — a temporal access pattern (cold precompute, hot reconstruction),
  reusing the `spectral_cache` discipline the kernel already has.
- **Tensor/spectral (item 16):** this unit IS the spectral item — it REUSES `spectral.rs` (Phase-28
  single surface) rather than adding a DCT module, exactly per the standard's "reuse the spectral
  tensor-graph machinery" directive. The sign reconciliation reuses the already-tested incidence
  operator (`field_energy.rs:201-248`).
- **Linux discipline (item 9):** **REUSE-not-reinvent** (`spectral.rs` over a new FFT module);
  **EXTENDS** — the shifted-`spmv` smallest-mode wrapper is a new capability on the existing surface;
  **ALREADY-EQUIVALENT** on determinism (inherits `topk_symmetric`'s fixed-seed/order discipline);
  **GAP honestly named** — no shift-invert solver exists (§5), the additive shift is the cheap partial.
- **Hermetic principles (item 20):** **Correspondence** — "as the graph, so the field": the field's
  modes ARE the graph's, negated; the reconciliation is the principle made a test. **Polarity** — the
  `+(D−A)`/`−(D−A)` sign split is a polarity reconciled by one negation, not two authorities.

---

## 10. Standard-compliance map (all 20 points — standard §2)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth `file:line` | §0 (both Laplacians, the sparse-tier obstacle, grid scale, consumer seam) |
| 2 | Falsifiable DoD | §8 (D-VERDICT = the number, either direction) |
| 3 | Spec→test→code, event-modeled | §4 (T1–T3 spec→RED→code; modal advance as a per-step event) |
| 4 | Predefined types & constants | §3 (`topk_smallest_symmetric`, `FieldModalBasis`, `MODAL_R_MAX`) |
| 5 | Adversarial/breaking tests | §4 (`red_modal_advance_fails_high_frequency`, `red_degenerate_subspace_handled`) |
| 6 | Hazard-safety from math | §5 (the convergence risk), §9 (sign/eigen-math test gates) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §9 ((n, r) axis; precompute break point) |
| 9 | Linux engineering discipline | §9 (REUSE/EXTENDS/ALREADY-EQUIVALENT/GAP) |
| 10 | Benchmarks + telemetry | §7 (the three-path verdict bench) |
| 11 | Isolation / bulkhead | §9 (presentation-side consumer; C oracle fallback) |
| 12 | Mesh awareness | §9 (N/A, CPU-local) |
| 13 | Rollback/self-heal as math | §9 (snapshot-re-entry recompute; truncation self-termination) |
| 14 | Error-propagation / smart index | §9 (sign/spectrum/FE-07 test gates) |
| 15 | Living-memory awareness | §9 (decompose-once precompute cache) |
| 16 | Tensor/spectral | §9 (reuses `spectral.rs`, the whole point) |
| 17 | Regression tracking | §8 D-VERDICT (bench + verdict in REGRESSION-LEDGER), D-NOREG |
| 18 | Clear worker instructions | §12 |
| 19 | Reuse-first, upgrade-if-needed | §1 (`topk_symmetric` reused; shift wrapper is the minimal extension), §2.2 |
| 20 | Hermetic principles | §9 (Correspondence, Polarity) |

---

## 11. Rollout sequencing (per the master ledger)

Master Ledger §3 wave-3 ("the falsifiable bets") + §4 item 14. **P89 is CPU-only and NOT gated on
P38 §4.2** (§0.5) — it parallelizes fully with P88's policy writing.

1. **P79-B6** (contiguous evec flatten) lands first — the handoff buffer format.
2. **P75** (bench-id/baseline schema) + **P81** (engine bench harness) provide the §7 substrate.
3. **P89** runs T1–T3 (correctness) FIRST, then §5 shift wrapper, then the §7 three-path bench,
   then writes the verdict. Delivers the §2-row-3 modal-vs-DCT decision data.
4. No wave-5 / GPU dependency — P89 completes on the CPU.

---

## 12. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §2 row 3 (the operator overrule + falsifiable
  exit), §4.5 (this unit's sketch, T1–T3 + the three-path table).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P89 row), §3 wave-3, §4 item 14 (bench = verdict, single owner).
- `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B3 (DCT recommendation,
  overruled), `OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md` §2a/§3 (DyRT + money-wall; recovered —
  restore per Ledger §0).
- `BLUEPRINT-P38-webgpu-render-engine.md` §4.6 (FE-07 flat-array / zero-engine-eigen-math contract).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Memory: `sovereign-architecture-19-phase-roadmap-2026-07-17.md`,
  Phase-28 single-eigen-surface ruling (MEMORY: eigenvector refactor `03ac0fefe`),
  `rust-native-bare-metal-decision-2026-07-14.md` (no per-platform-libm FFT).

**Existing code this blueprint edits/extends (exact targets, dowiz):**
- **EXTEND** `kernel/src/spectral.rs` — add `topk_smallest_symmetric` + `SHIFT_SIGMA_GRID2D` beside
  `topk_symmetric` (single eigen surface; the shifted-`spmv` wrapper only).
- **NEW** `engine/src/field_modal.rs` — `FieldModalBasis` (consumes the P79-B6 flat buffer), the r-mode
  reconstruction + damped-oscillator advance; **zero eigen-math** (calls no `spectral::*` eigen fn).
- **NEW test module** (kernel-side or `engine/src/field_energy.rs`-adjacent) — T1–T3 + the analytic DCT
  oracle (§0.4) as a few closed-form lines, NOT a module.
- **REUSE unchanged** `kernel/src/csr.rs` (`spmv`/`laplacian_spmv`), `kernel/src/spectral_laplacian.rs`,
  `engine/src/field_frame.rs` (`step`/`compose` oracle), `field_energy.rs:201-248` (the sign-pin).
- **DO NOT** build a DCT/FFT module (it is the T1/T2 oracle + path-B bench only); **DO NOT** touch
  `money.rs`/the determinism oracle; **DO NOT** put eigen-math in the engine (FE-07).

**For the worker with zero session context — exact acceptance path:**
1. Run **T1–T3 FIRST** (the reconciliation). If they fail numerically, the approach is wrong for this
   domain — record it and STOP (DCT stands); do not build the modal homes.
2. Add the **§5 shift wrapper** `topk_smallest_symmetric`; prove it returns L's smallest eigenpairs
   deterministically (D4). This is the make-or-break for scale — the raw `topk_symmetric` returns the
   WRONG end of the spectrum (§0.2/§5).
3. Build `FieldModalBasis` + the r-mode reconstruction consuming the P79-B6 flat buffer (engine does
   zero eigen-math).
4. Run the **§7 three-path bench** in P81's harness, registered into P75's schema. **Fill the table
   with measured numbers.**
5. **Write the verdict paragraph** citing the numbers — confirm the bet for FE-10/G5 (r ≤ 16) if A
   meets budget + E1 tolerance, or record the loss (precompute non-convergence §5, or DCT winning) and
   the applicable §5 fallback. File it in `docs/regressions/REGRESSION-LEDGER.md`.
6. Anti-scope: never build a standing DCT module; never move eigen-math into the engine; never touch
   money/oracle. The deliverable is **the number and the verdict**, not necessarily a shipped modal engine.

---

## 13. Open operator-decision points

| # | Decision | Blocks / affects | Default if unruled |
|---|---|---|---|
| OD-P89-1 | If `bench_precompute_spectral` (§5) shows the additive shift can't converge within budget: build a true shift-invert solver (new unit), or accept the §5(b) masked-domains-only scoped win | P89 verdict scope | §5(b) scoped win recorded; no new solver built speculatively |
| OD-P89-2 | If the §7 verdict says DCT wins the rectangular case but `spectral.rs` is required for masked domains (§6): ship BOTH (DCT fast-path + spectral general-path) or spectral-only | future modal engine | Verdict records both costs; no engine shipped in P89 (DoD = the number) |
| OD-P89-3 | `MODAL_R_MAX` (default 16) — the mode budget scoping the bet; higher r shifts the §7 crossover toward DCT | §7 crossover | 16 (the FE-10/G5 regime) |

---

*Cross-references: `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (§2 row 3/§4.5) ·
`MASTER-STATUS-LEDGER-2026-07-19.md` (P89 row, wave-3, §4 item 14) ·
`BLUEPRINT-P38-webgpu-render-engine.md` (§4.6 FE-07) · `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 ·
source: `kernel/src/spectral.rs` (`topk_symmetric` :269-402, `eigh` :251-261),
`kernel/src/spectral_laplacian.rs` (:70-101, the smallest-λ gap), `kernel/src/csr.rs`,
`engine/src/field_frame.rs` (:119-128 sign, :255-262 oracle), `engine/src/field_energy.rs` (:201-248
sign-pin) · memory: Phase-28 single-eigen-surface ruling, `rust-native-bare-metal-decision-2026-07-14.md`,
`OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md`.*
