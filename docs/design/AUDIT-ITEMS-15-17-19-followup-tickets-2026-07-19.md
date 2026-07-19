# AUDIT — Items 15 / 17 / 19 follow-up tickets + Item 16 resolution note (2026-07-19)

> Tier-0 audit cluster from `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A. Planning
> source: `BLUEPRINT-ITEMS-15-16-17-19-eigen-spectrum-engine-audit-2026-07-19.md`. Every
> file:line below was re-verified against the live worktree this session (branch
> `exec/space-grade-tier0-2026-07-19`, after commit `e125f0c97`; source-identical to `main` for
> the audited files except the item-16 collapse). Items 15/17/19 are **read-only audits → tickets
> filed** (no code built — each opens NEW scope). Item 16 was a confirmed P2 → **refactor landed**,
> summarized in §0; it is NOT a ticket.

---

## 0. Item 16 — RESOLVED (refactor landed, not a ticket)

The blueprint pre-classified item 16 as the cluster's one P2-class finding and expected a defect
doc. Independent re-verification confirmed the defect, and the roadmap's item-16 rule ("Read-only
**unless a P2 defect forces collapse**") plus its stated proof condition ("a refactor lands making
the functionals post-processing of `GraphSpectrum`, with all existing numeric tests green") both
license the collapse. It landed rather than being deferred.

**Confirmed defect (pre-refactor):** `graph_spectrum()` (`kernel/src/spectral.rs`) computed the
adjacency spectrum three times — once directly, once inside its own `classify_drift(adj)` call —
while its doc-comment claimed a "single eigenvalue pass". `graph_energy_report()` made four
independent passes (`classify_drift` + `graph_energy` + `spectral_radius` +
`algebraic_connectivity`) while claiming "single pass". Both doc-claims were false as written.

**Fix shape chosen — option (b), internal, zero public-signature change** (justified by the caller
grep: the seven standalone functionals have many independent external callers —
`wasm.rs:731/735/739/743/798-803`, `evals.rs:193-194/1245-1249`, `hydra.rs:60/221/308`,
`markov.rs:211/494`, `csr.rs:310`, `event_log.rs:434`, `order_machine.rs:396`,
`spectral_cache.rs:268` — each wanting ONE scalar; routing them through `graph_spectrum` would make
every standalone call strictly MORE expensive, i.e. a regression, not a collapse). Instead:
- extracted `drift_guards_ok` + `drift_band`; `classify_drift` now delegates to them;
- added `classify_drift_with_rho(a, rho)` — identical fail-closed guards, takes an already-computed
  ρ instead of recomputing `eigenvalues(a)`;
- `graph_spectrum` derives drift from its own ρ → **exactly two** eigenvalue passes (adjacency +
  Laplacian, two distinct operators);
- `graph_energy_report` reads every field from one `graph_spectrum` profile (4 → 2 passes).

**Proof:** a test-only thread-local `EIGEN_CALLS` counter (compiled out in release) asserts
`graph_spectrum` and `graph_energy_report` each enter `eigenvalues()` exactly twice — this proves
"one computation, many functionals", not mere value agreement. A value-consistency test pins the
profile fields to the standalone functionals. Kernel suite **902 passed / 0 failed / 3 ignored**
(was 899; +3 new). Committed `e125f0c97`, pushed to `exec/space-grade-tier0-2026-07-19`.

---

## Ticket I15-T1 — vector-scope cross-solver parity test for the sparse/dense eigen surface

**Class:** parity-test gap (test scope), P3. **Not a defect in the code under test.**

**Verified state (single-surface claim HOLDS):**
- `kernel/src/spectral.rs:225` `eigenvalues` routes n ≤ 32 into
  `householder::eigenvalues_contig` and n > 32 into the Faddeev-LeVerrier + Durand-Kerner fallback
  — one dispatch, one backend family. `eigh` (`:251`) is a pure façade over `householder::eigh_contig`.
  `lowrank.rs` is **absent** (`ls` fails; zero grep hits in `kernel/src/`). Commit `03ac0fefe`'s
  single-eigen-surface claim is confirmed true.

**The gap (verified):** the cross-solver parity test `r3_topk_symmetric_parity_p3`
(`spectral.rs:1216`) binds **eigenvalues only** — it compares magnitude sets (`:1237-1238`) and
validates only the *dominant* eigenvector by residual `A v ≈ λ v` (`:1241-1253`). The dense `eigh`
eigenvector basis is explicitly discarded: **`let _ = dvecs;`** at `spectral.rs:1254`. So the
sparse `topk_symmetric` basis is never compared against the dense `eigh` basis across the full
spectrum. `r3_topk_symmetric_determinism` (`:1257`) pins bitwise self-determinism, not cross-solver
vector parity. This is bebop finding #25's shape ("parity harness binds eigenvalues only; vectors
validated by residual, no second solver").

**Ask:** add a cross-solver **vector-scope** parity test: `topk_symmetric` basis vs `eigh` basis on
a shared symmetric fixture, comparing eigenVECTORS (not just values/dominant-residual), with an
explicit convention for the three things that legitimately differ between solvers and must be
normalized before comparison:
1. **sign** — both already sign-fix "first nonzero component > 0" (`spectral.rs:386-398` topk,
   `eigh_contig` per its doc); assert the fixed convention holds identically.
2. **ordering** — sparse returns descending |λ|, dense ascending; align by value before pairing.
3. **degenerate subspaces** — for repeated eigenvalues (e.g. K₃'s {2,−1,−1}) individual
   eigenvectors are basis-dependent; compare the **projector** `Σ vᵢvᵢᵀ` onto each eigenspace (a
   convention-free invariant), not raw vectors.

**Forcing reason / why not now:** NEW scope (the audit was read-only; a degenerate-subspace-correct
vector-parity test is real design work, not a one-liner). **DoD:** a green test in
`spectral.rs::tests` that fails if the two solvers' eigenSPACES diverge on a fixture with both
simple and repeated eigenvalues. **Cross-ref:** does not overlap item 18 (Laplacian *values* pin);
this is a sparse↔dense *vector* pin.

---

## Ticket I17-T1 — pin the engine L-operator at the kernel↔engine boundary (RC-4, last open mirror)

**Class:** unpinned mirror at the kernel↔engine seam (RC-4), P2-parity. **Structural, not a
value bug today.**

**RC-4 context (traced):** RC-4 = "Unpinned mirrors at the kernel↔engine seam", root cause 4 of the
2026-07-16 hermetic audit (`docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md:238`),
closed for two of its three engine-relevant mirrors by the H2 sweep (`4dec04218`). The synthesis
(§13, line 237) names the engine triple for this table as **DriftClass / dt / L-operator**.

**Verified state of the three mirrors:**
1. **`DriftClass`** — CLOSED. Re-declared `engine/src/bridge.rs:683` (`drift_from_code` `:706`) but
   PINNED: kernel authority `DriftClass::wire_code()` (`kernel/src/spectral.rs`), engine round-trip
   test `drift_wire_contract_matches_kernel` (`bridge.rs`, exhaustive over all three variants).
2. **`dt`** — CLOSED. `engine/src/field_frame.rs:62` binds `dt: dowiz_kernel::DT_STABLE as f64`
   directly; `engine/src/loop_.rs:19` `DT_STABLE: f32 = 0.02` is a hand mirror carrying the pin
   test `dt_stable_matches_kernel_contract` (`loop_.rs`), matching the kernel's
   `dt_stable_is_authoritative`.
3. **L-operator** — **OPEN.** `engine/src/field_frame.rs:10-40` implements a damped-wave
   semi-implicit scheme with an **engine-side 5-point Neumann Laplacian** (left+right+up+down−4u,
   zero-flux edges; discrete eigenvalues λ∈[−8,0]). This is engine-side COMPUTATION unpinned to the
   kernel's own Laplacian surface (`kernel/src/csr.rs:552 laplacian_spmv`, plus
   `spectral_laplacian.rs` / `field_eigenmodes.rs`, which the engine already consumes elsewhere via
   `engine/src/lib.rs:18 field_modal`).

**Ask:** a parity-pin test at the engine boundary asserting the engine's 5-point Neumann Laplacian
stencil agrees with the kernel's Laplacian operator on a shared small-grid fixture (a
matvec/`spmv` comparison, bit-exact or within a stated tolerance). Classification for the item-17
thick/thin table: **computation-that-lives-in-engine, forcing reason = the 2-D grid stencil with
Neumann zero-flux edges is a render-plane operator with no current kernel consumer of that exact
boundary condition; pin rather than move.**

**Cross-reference (do NOT duplicate):** roadmap **item 18** already landed a Laplacian parity pin
**within the kernel** (`kernel/src/spectral.rs` dense `laplacian()` ↔ `csr.rs:552 laplacian_spmv`,
commit `21efa70e8` on this branch). This ticket is a **different** pin — the engine's own grid
Laplacian against the kernel, at the crate boundary — not covered by item 18's intra-kernel pin.
The item-17 table row 3 should cross-reference item 18, not restate it.

**Forcing reason / why not now:** NEW scope + crate-boundary test wiring (engine test depends on
`dowiz-kernel`; the grid-vs-CSR fixture equivalence needs a stated boundary-condition mapping).
**DoD:** a green engine-side test that fails if the engine stencil and the kernel Laplacian diverge.

---

## Ticket I19-T1 — parity-pin `ppr.rs` inner loop against `markov.rs` (comment-bound mirror, RC-4 intra-kernel)

**Class:** comment-bound, unpinned mirror of a proven inner loop (RC-4 shape, intra-kernel). **Not a
collapse — see forcing reason.** P3-parity.

**Verified state (independence-by-design HOLDS; the mirror is the finding):**
- `kernel/src/retrieval/ppr.rs` (152 lines) and `retrieval/diffusion.rs` (327 lines) contain
  **zero** `use crate::spectral` / `GraphSpectrum` / `eigh` / `topk` references (grep confirmed).
  `ppr.rs:6-7` documents "**No eigendecomposition** — pure power iteration". This is item 16's
  question ("is retrieval the second GraphSpectrum consumer?") answered **NO**, correctly — the
  forcing reason (PPR is an O(K·n²) `(I−αW)⁻¹`-style iteration; fixed-K determinism is load-bearing)
  stands. §15(d)'s hoped-for "second spectral consumer" does not exist.
- **The mirror finding:** `ppr.rs:3-5` states it "Reuses the EXACT deterministic accumulation order
  of `kernel/src/markov.rs`'s damped-PageRank kernel … we **mirror** its proven bitwise-reproducible
  left-product." `diffusion.rs:9` and `retrieval/mod.rs:10` repeat the mirror claim; `mod.rs:14`
  states the red-line "`markov.rs` is NEVER modified". The mirrored loop is `markov.rs:162-170`
  (`nxt[j] += pii * ((1.0 - d) * a[i][j] + d/n)`, i-outer / j-inner). `ppr.rs:48` carries the
  same "SAME accumulation order as markov.rs: i outer, j inner" comment.
- **No test enforces the mirror.** Grep for `markov` across `retrieval/` returns only doc-comments;
  grep for `ppr`/`Ppr` in `markov.rs` returns nothing. The bitwise-parity claim is comment-bound.

**Ask:** a parity-pin test (RC-4 / H2-row format) asserting `ppr.rs`'s diffusion inner product is
bit-exact against `markov.rs`'s damped step on a shared fixture — i.e. one power-iteration step of
each on the same W, seed, and α, `assert_eq!` on the raw `f64` accumulator (not approx). This makes
the "we mirror markov's proven left-product" comment executable.

**Forcing reason — parity-pin, NOT collapse (decided, not defaulted):** `retrieval/mod.rs:14`
records a red-line that `markov.rs` is never modified, and the two consumers legitimately diverge
downstream — `ppr.rs:14-16` drops markov's per-step `÷ sum` normalization because the personalized
restart conserves mass (Σ = (1−α)·1 + α·1 = 1), keeping the result bit-exact WITHOUT the divide.
Collapsing PPR onto a shared `markov` helper would either force markov's normalization back in
(wrong for PPR) or fork markov (violates the red-line). The correct output is a **pin that the
shared inner-loop accumulation stays identical**, letting each keep its own restart/normalization
tail. **DoD:** a green test (in `retrieval/tests.rs` or `markov.rs::tests`) that fails if either
inner loop's summation order or arithmetic drifts.

---

## Handoff summary

| Item | Verdict | Artifact | State |
|---|---|---|---|
| 15 | single eigen-surface HOLDS; parity is values + dominant-residual only (`spectral.rs:1254` `let _ = dvecs;`) | **Ticket I15-T1** (vector-scope cross-solver parity, degenerate-subspace-correct) | filed, not built (new scope) |
| 16 | P2 CONFIRMED — 3×/4× recompute + 2 false "single pass" doc-claims | **Refactor landed** (`e125f0c97`), §0 above | DONE, pushed |
| 17 | RC-4 triple: DriftClass + dt CLOSED; L-operator OPEN (`field_frame.rs:10-40` engine-side Laplacian, unpinned) | **Ticket I17-T1** (engine-boundary Laplacian pin; cross-refs item 18's intra-kernel pin) | filed, not built (new scope) |
| 19 | independent-by-design (no GraphSpectrum consumer); comment-bound `ppr.rs`↔`markov.rs` mirror, no test pin | **Ticket I19-T1** (parity-pin, NOT collapse — red-line forbids touching markov) | filed, not built (new scope) |
