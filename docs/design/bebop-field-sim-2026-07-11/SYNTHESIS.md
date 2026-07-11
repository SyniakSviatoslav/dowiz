# bebop Field-Sim — Dissection Synthesis — 2026-07-11

> Consolidation of two independent read-only lenses (both repos left as found):
> `01-implementation-math.md` (numerical-methods audit, ran the code against eigen/DFT oracles) and
> `02-purpose-usage-claims.md` (call-path tracing, benchmark-honesty audit, theory-docs critical read).
> Operator ask: "досліди та розбери field sim у bebop." Verdict labels mirror the project's own
> audits: VERIFIED (checked against an oracle / traced in code), CLAIMED-UNVERIFIED, CONTRADICTED.

---

## 1. One-paragraph verdict

The bebop "field sim" is **three different models wearing one name**, and the honest split is sharp:
the **static heat kernel `exp(−cLt)·u₀`** (spectral + Chebyshev) and the **FFT** and **VSA
bind/unbind** are genuinely correct and oracle-pinned; but **every *iterative* diffusion path carries
a sign bug** that makes it unconditionally unstable, and the green test suite **masks** it — a
textbook Verified-by-Math violation (tests that pass while the math is wrong). Meanwhile the heavy
"physics" is **almost entirely orphaned** — the only live field path is a 5-keyword string match on a
hardcoded 6-node graph (a lookup table with a physics costume), and the actual dispatch/routing is
classical k-d+BFS+A*/CH with **zero callers in any binary**. The benchmarks that claim the field wins
are **not honest** (circular recall metric, Rust-vs-JS speedups sold as method wins, a k-d strawman).
The theory docs split cleanly into grounded-and-self-critical vs numerology. **Verdict: park the
field-sim for delivery.** Salvage the three correct primitives as libraries; if it is ever revived,
fix the sign bug first. And one application is categorically blocked by a standing red line.

## 2. What is CORRECT and tested (VERIFIED against oracles)

| Piece | Status | Evidence |
|---|---|---|
| Static graph heat kernel `exp(−cLt)u₀` (spectral + Chebyshev) | correct to **1e-10 … 1e-16** vs an eigen-oracle | lens 1 §numerics |
| Damped wave ODE with per-solid tensors | correct sign, symplectic Euler, stable | lens 1 |
| FFT (radix-2) | DFT-pinned 1e-9…1e-12 | lens 1 |
| VSA bind/unbind (circular convolution) | brute-force-convolution-pinned — **but only for power-of-2 dims** | lens 1 |
| `field_gate` fail-closed contract | genuinely good engineering (the veto refuses on ambiguity) | lens 2 |
| Zero-dep + empty-import wasm | VERIFIED (built + parsed: no import section) | lens 1 |

## 3. What is BROKEN (VERIFIED bugs the green suite hides)

1. **The diffusion sign bug (the big one).** Every iterative path — `rust-core field_active`, bebop2
   `active_diffuse`, `coherence::propagate` — steps `u += dt·c·L·u`. That is **anti-diffusion (wrong
   sign)** → unconditionally unstable; ‖u‖ → **4.7e31** at the "stable" dt=0.02 over 1000 steps. The
   B11 "dt corridor" safety test **encodes the misdiagnosis** (with the correct sign, dt≤0.5 is
   stable). Masked by 10-step tests + exact mass conservation. **This is a real, landable fix.**
2. **`coherence::propagate` mislabels its math** — claims a heat kernel, actually does ONE Euler step
   of size t (verified output `[1.5,−0.5,0,0]` vs exact `[0.674,0.258,…]`).
3. **`lyapunov.rs` / `SpectralKalman`** apply symmetric-only Jacobi to general matrices while the docs
   claim complex/diagonalizable support → **46 of 80 stable non-symmetric systems misreported
   UNSTABLE**; the "kalman" is a covariance time-update, not a filter.
4. **VSA silently wrong for non-power-of-2 dims** (error 1.7–4.4, verified).
5. **All suites pass (19/19, 94/94, 65/65)** — but the active-diffuse / corridor / coherence tests
   **pin the wrong model**. Green-while-wrong is exactly the false-positive-metric class the repo's
   own VbM rule forbids.

## 4. Is it USED? (VERIFIED by call-path tracing — mostly no)

- **Only live field path:** CLI/MCP `field_gate` = a 5-keyword string match onto a hardcoded 6-node
  graph; heat-kernel outputs are constants → the "physics veto" is **extensionally a lookup table**.
- **Orphaned:** `field_physics` (gravity/springs/Lyapunov), `geometry_field` (Platonic solids /
  spherical harmonics), `plan_wave_gate` — callers are the benchmark example + internal cross-calls
  only.
- **The real dispatch/routing** (`matcher` + `hybrid_route`) is classical k-d+BFS+A*/CH, explicitly
  "NO PDE solver," with **zero callers in any binary**; the matcher never even selects among couriers.

## 5. Benchmark honesty + theory (VERIFIED / itemized)

- **Benchmarks over-claim.** The Rust example is honestly anti-field (exits 1 unless CH beats the
  wave). The markdown reports sell it via: a **circular recall metric** (the field's own output as
  ground truth), **Rust-vs-JS speedups sold as method wins**, a **k-d strawman** (BFS is the fair
  baseline), **synthetic data**, and now-broken reproduce paths (the TS stack was archived).
- **Theory splits cleanly.** Grounded + self-critical: `tensor-field-theory`, `cycle-consistency`,
  the `math-physics` audit (which itself labels Emden/redshift/vorticity/Noether "**POETRY**"), and
  the bebop2 ARCHITECTURE numerics doctrine (sound, with rhetorical inflation). Numerology: the
  "∇·F/∇×F law of reasoning" and the Platonic-solid node tensors.

## 6. Red-line block (standing operator ruling 2026-07-11)

`reputation.rs` designs **courier trust feeding the cost surface** — that is **courier scoring**,
which the operator ruled out as a hard red line (see living memory
`local-first-and-no-courier-scoring-2026-07-11.md`). Any field-sim application that ranks couriers is
**blocked outright**, independent of whether the math works.

## 7. Recommendation

1. **Park the field-sim for the delivery hub.** By bebop's *own* measurements the dispatch/routing
   incumbents win — `attemptHonestDispatch`, ORS, and Hungarian-matching if optimal assignment is ever
   needed. The PDE machinery buys the delivery product nothing today.
2. **Keep three primitives as honest libraries:** the static heat kernel, FFT, and VSA bind/unbind
   (power-of-2), all oracle-pinned. Fence off the buggy iterative/lyapunov/kalman paths or fix them.
3. **If revived, the sign bug is the first fix** and the corridor test must be re-pinned to the
   correct model (its current green is protecting the bug).
4. **The one non-delivery candidate that survives scrutiny:** heat-kernel "regression radius" ranking
   over the **dowiz codebase graph** (which files a change can affect) — with a falsifiable test:
   recall@20 vs a transitive-closure + git co-change baseline on real PR history, RED case defined. If
   it loses that test, this synthesis is its tombstone. (Note: this is a dev-tooling use, unrelated to
   couriers — no red-line exposure.)

*Two independent lenses, both read-only; code executed against eigen/DFT/convolution oracles in a
scratch dir. The only repo files created are the two lens reports + this synthesis.*
