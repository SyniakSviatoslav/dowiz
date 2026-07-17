# Research Verification — Decorrelated Re-check of RESEARCH-AND-REASONING.md (2026-07-16)

> Role: decorrelated verifier. Did NOT write the research doc; goal is to REFUTE its
> load-bearing claims against live code, not rubber-stamp them. Read-only. Branch
> `feat/spectral-energy-flow-evolution` (worktree `/root/dowiz-spectral-evolution`).
> Scope: the 5 claims the next-stage blueprints will be built on. Line numbers below are
> current live-tree file:line, re-read directly (not trusted from the doc).

---

## Check 1 — The sign-convention split (§1, most load-bearing). **CONFIRMED.**

**Claim:** `engine/src/field_frame.rs` computes `left+right+up+down − 4u` = `−(D−A)U`, while
`kernel/src/csr.rs::laplacian_spmv` (Unnormalized) computes `+(D−A)`; opposite conventions,
pinned by nothing across the seam.

**Evidence (read directly):**
- `engine/src/field_frame.rs:103` — `out[i] = left + right + up + down - 4.0 * u[i];`
  For an interior grid node degree = 4, so this is `(Σ neighbours) − 4·u_i = −(4·u_i − Σ neighbours) = −[(D−A)u]_i`.
  This is the continuum ∇² convention (negative-semidefinite; eigenvalues in `[-4,0]`, confirmed by the
  module's own docstring `field_frame.rs:22` and its sign test `laplacian_peak_negative_at_center_of_disk`
  at `:247-267`, which pins ∇²(peak) < 0).
- `kernel/src/csr.rs:316-325` (Unnormalized branch) — `acc = deg[i]*x[i]; acc -= self.val[k]*x[col]`
  = `d_i·x_i − Σ_j A_ij·x_j = +[(D−A)x]_i` (positive-semidefinite graph Laplacian). Pinned by
  `laplacian_spmv_equals_dense_laplacian_matrix` (`:820-848`) against `L = [[2,-1,-1],...]` (positive diagonal).

**Verdict:** The two primitives genuinely emit opposite-sign outputs for the same graph. There is **no
reconciling sign flip** in the call chain — the doc's reasoning is sound. Note the doc is careful and
correct that `field_frame`'s integrator is NOT itself buggy: at `field_frame.rs:155` it uses `+c²·l`
where `l` is the ∇² stencil, which equals `−c²·L_graph·U` and is physically-correct diffusion (∇² of a
peak is negative → damps the peak). The risk is strictly a **future caller crossing the kernel↔engine
seam** who assumes both `laplacian` functions share a convention — exactly as the doc states. Each side
pins its own convention; nothing pins them to agree (no cross-crate parity test exists). **No correction
needed.** The blueprint's "incidence factorization as parity-bind" (§3.1b) rests on an accurate fact.

---

## Check 2 — CLT imprisoned in one test. **CONFIRMED.**

**Claim:** `causal.rs` `empirical_converges_to_analytic_as_n_grows` derives a true asymptotic SE from
first principles and gates on it; the logic is NOT exposed as a reusable primitive anywhere.

**Evidence:**
- Test is at `kernel/src/causal.rs:2226` (`#[test]`) / `:2227` (fn) through `:2256`.
  (Minor: the doc cites `:2229-2258`; the true span is ~2226-2256 — a few lines off, not load-bearing.)
- `se_factor` derived at `:2238-2243` from the estimator's real asymptotic variance
  `Var ≈ (1/N)·Σ_z P(Z=z)²·p_yz(1−p_yz)/P(X=1,Z=z)` — **no magic constant** (the doc comment at
  `:2219-2221` states this explicitly). Gate at `:2249-2250`: `err * √N < se_factor * 6.0`. A genuine √N /
  CLT-rate envelope.
- Reusability grep (`mean_se|wilson|confidence_interval|clopper|bootstrap|se_factor|std_error`) over
  `kernel/src` + `engine/src`: `se_factor` appears **only** in `causal.rs` (4 hits, all inside this one
  test). No `wilson`/`clopper`/`confidence_interval`/`bootstrap`/`mean_se` anywhere. (The two
  `living_knowledge.rs` / `hydra.rs` hits are the `stderr` IO stream and "re-bootstraps" prose — not
  statistics.)

**Verdict:** Both halves confirmed — real SE derivation, gated; zero reusable primitive. **No correction
needed** beyond nudging the cited line span to 2226-2256.

---

## Check 3 — Self-Harness "embryo already exists". **CONFIRMED (one naming correction).**

**Claim:** trace log = `EvalRow::append_jsonl` (appender, zero readers); proposal =
`SelfAdaptator::propose_step` (never mutates state directly); acceptance = `RegressionGate` (rejects on
sustained degradation).

**Evidence:**
- **Trace log — appender with zero (Rust) readers: CONFIRMED, but the method name is wrong.** There is
  **no method named `append_jsonl`**. The real methods are `EvalRow::to_jsonl` (`evals.rs:464`) and
  `EvalRow::append_to(path)` (`evals.rs:489-498`). The doc's `EvalRow::append_jsonl` at `:487-498` points
  at the right lines but names a nonexistent method. The "zero reader" characterization is accurate at the
  Rust layer: the only consumer named is the external Node `analytics/analyze.mjs`; no in-kernel Rust code
  reads the JSONL back to cluster failures (only tests exercise `append_to`/`to_jsonl`).
  **CORRECTION for blueprint: rename `append_jsonl` → `append_to` (+ `to_jsonl` for the string form).**
- **`propose_step` never mutates external/kernel state: CONFIRMED (precise form).** `propose_step`
  (`evals.rs:707-748`) mutates only the adapter's OWN internals (`self.opt` Adam θ, `last_loss`, `steps`,
  `accepted_theta`) and returns a candidate scaler; it **never touches a `KalmanFilter`**. `apply_step`
  (`:752-755`) is the **sole** mutator of real kernel state (`kf.set_q_scaler(s)`). Rollback-to-accepted
  is real (`:741` sets `opt` back to `accepted_theta` on guard rejection). The propose/guard/apply/rollback
  separation is genuine, not just prose. (Precision note: "never mutates" is true of *kernel* state, not of
  the adapter's internal optimizer — the doc's framing and the code comment at `:651-653` both mean the
  former.)
- **`RegressionGate` rejects on sustained degradation: CONFIRMED.** `observe` (`:570-597`) pushes the
  sample through an EMA, keeps a window, and returns RED only when the monotonic-degradation streak
  `>= window - 1`. Tests pin it: flips RED on 3 consecutive rises > tol (`:887-905`), stays green when
  stable/oscillating within tol (`:907-916`), and clears on trend reversal (`:918-930`).

**Verdict:** Structurally confirmed. **One correction:** the trace-log method is `append_to` / `to_jsonl`,
not `append_jsonl`.

---

## Check 4 — Hermetic finding #8 "now has a production caller". **CONFIRMED (with a wiring caveat).**

**Claim:** `csr::laplacian_spmv` now has a production caller at `engine/src/bridge.rs:125`
(`VertexBridge::apply_field`, W20), so #8's "zero callers" half is stale.

**Evidence:**
- `engine/src/bridge.rs:121-128` — `pub fn apply_field(&mut self, x: &[f64])` calls
  `g.laplacian_spmv(x, &mut y, LaplacianKind::Normalized);` at **`:125`**. This is in the plain
  `impl VertexBridge` block — **NOT `#[cfg(test)]`, NOT feature-gated**. So `laplacian_spmv` has a genuine
  non-test library caller. The literal claim is accurate.
- **Caveat the blueprint must not overstate:** `apply_field` itself is only *reached from tests* today
  (`bridge.rs:438`, `:475`). The other `VertexBridge` construction site, `scene.rs:168 render_to_bridge`,
  does **not** call `set_field_graph`/`apply_field`. So `laplacian_spmv` is wired into a public engine API
  that no live/application loop yet drives — it is a "production (non-test) call site that is not yet
  consumed by a running path," not "wired into the live render loop." The doc's exact wording ("has a
  production caller ... 'zero callers' half is stale") is defensible; just don't upgrade it to "consumed in
  production" in the blueprint.

**Verdict:** CONFIRMED at the source level. Add the one-line caveat above so the blueprint doesn't imply a
live consumer. The "≥4 implementations / no cross parity / opposite sign" half of #8 fully stands (see
Check 1).

---

## Check 5 — `impedance.rs` zero callers + docstring rejects the circuit metaphor. **CONFIRMED.**

**Claim:** zero production callers (only `lib.rs`'s `pub mod`); docstring explicitly rejects literal
impedance/circuit matching.

**Evidence:**
- Repo-wide grep (`impedance|reflection_coefficient|FlowGate|::gate(`) over `kernel/src`, `engine/src`,
  `apps`: every hit is **inside `impedance.rs` itself** or the two `lib.rs` lines — `lib.rs:54` (doc
  comment) and `lib.rs:56` (`pub mod impedance;`). **Zero production callers.** Confirmed.
- Docstring rejection (`impedance.rs:1-13`), verbatim: line 2 "software flow is NOT a resistor"; lines 6-8
  "The plan is explicit that literal \"max-power-transfer impedance matching\" misleads here; the real
  invariant is `ρ < 1 − margin` (stable, no runaway reflection)...". The `reflection_coefficient`
  (`:19-28`, `(r*r*burst).min(0.99999)`) and two-pole `FlowGate {Admit, Backpressure}` on
  `ρ_eff < 1 − margin` (`:32-47`) match the doc's description exactly. No R_eq / series-parallel / node-loop
  composition law exists.

**Verdict:** CONFIRMED, no correction. (The doc cites the rejection at `:5-8`; the exact phrase is `:6-8` —
immaterial.)

---

## Corrections to hand to the blueprint-writing stage

1. **Check 3 (real):** `EvalRow::append_jsonl` does **not exist** — the methods are `EvalRow::append_to`
   (`evals.rs:489`) and `EvalRow::to_jsonl` (`:464`). Rename every occurrence in the research doc's §1, §2
   (row 4), and §3.3 before they propagate into a blueprint that references a nonexistent symbol.
2. **Check 4 (nuance, not a refutation):** keep the doc's "production caller at bridge.rs:125" wording, but
   the blueprint must state that `apply_field` has no non-test caller yet (only `scene.rs` constructs a
   bridge, and it doesn't call `apply_field`). It is a wired-but-unconsumed public API, not a live loop.
3. **Check 2 (cosmetic):** cited test span `:2229-2258` → actual `:2226-2256`.
4. **Check 1 & 5:** accurate as written; no change.

**Bottom line:** All 5 load-bearing claims survive re-verification. Two are fully clean (1, 5), two need
only cosmetic line-number nudges (2, 4-source-fact), and one carries a genuine symbol-name error (3:
`append_jsonl` → `append_to`) plus a wiring caveat (4). None of the corrections invalidate a STRONG
finding or a blueprint sketch; the "incidence factorization as parity-bind" fact (Check 1) — the single
load-bearing claim called out — is correct. Blueprint-writing may proceed once corrections 1-2 are applied.

*Verified live on `feat/spectral-energy-flow-evolution` as of 2026-07-16. Read-only; no code or the
original RESEARCH-AND-REASONING.md was edited.*
