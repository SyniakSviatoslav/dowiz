# Bebop field sim — implementation & math dissection (lens 1 of 2)

Date: 2026-07-11. Scope: the actual code and math of the field simulation across
`bebop2/core/src/` (field.rs, fft.rs, chebyshev.rs, kalman.rs, lyapunov.rs, vsa.rs, algebra.rs, active.rs),
`rust-core/src/lib.rs` (bebop-core, the wasm field core), and
`crates/bebop/src/` (field.rs, field_physics.rs, geometry_field.rs, wavefield.rs, coherence.rs, mathx.rs),
plus tests, KATs and `crates/bebop/examples/tree_vs_field_telemetry.rs`.

Method: every file read in full; all three Rust test suites executed
(`bebop-core` 19/19 pass, `bebop2-core` 94/94 pass, `bebop` field-related 65/65 pass);
kernels re-implemented verbatim in a throwaway harness
(`/root/.claude/jobs/c6a4c73f/tmp/fieldsim_audit{,2}.rs`, `lyap_flip.rs`) and probed against
exact eigendecomposition oracles; the wasm import-section claim checked by building
`bebop-core` for `wasm32-unknown-unknown` and parsing the binary. Labels:
**VERIFIED** (checked here by test/experiment), **UNVERIFIED** (claim not checked),
**DESIGN-JUDGMENT** (modeling choice, not a correctness question).

---

## 1. The model, reconstructed

There is no single "field sim". Three mathematically distinct models coexist, plus a
decision-layer wrapper. What the docs call one thing is, in code, the following.

### 1a. Graph heat/diffusion PDE (the core "field")

The governing equation the *spectral* propagators implement is the graph heat equation

```
∂u/∂t = −c · L · u ,   L = D − A  (unnormalized graph Laplacian)
u(t)  = exp(−c·L·t) · u0
```

implemented three ways:

1. **Analytic eigenmode form** — `u(t) = Σ_k e^{−λ_k t} ⟨u0, φ_k⟩ φ_k`
   (`bebop2/core/src/field.rs:135-156`, `propagate_spectral`), eigenpairs from a cyclic
   Jacobi diagonalization of the dense symmetric L (`field.rs:255-323`, `jacobi_eigen`),
   modes stored column-major with the transpose trap explicitly handled
   (`field.rs:96-103`). Pointwise decay `exp(−λ_i t)` also exposed at
   `chebyshev.rs:168-177` (`propagate_spectrum`).
2. **Chebyshev matrix-free form** — truncated Chebyshev expansion of `exp(−c·t·L)·u0`
   (`bebop2/core/src/chebyshev.rs:108-166`, `spectral_propagate`; byte-identical algorithm
   in `rust-core/src/lib.rs:182-242`). Coefficients from 64-point quadrature
   (`chebyshev.rs:125-140`), three-term recurrence on `ã = (2/b)L − I` with
   `b = λmax ≤ 2·max_degree` (`chebyshev.rs:62-72`).
3. **Iterative "active-set" form** — explicit Euler stepping with `|Δu| < eps` pruning:
   `u_{k+1} = u_k + dt·c·L·u_k` (`rust-core/src/lib.rs:314-315`,
   `bebop2/core/src/field.rs:193-196`, and again `crates/bebop/src/coherence.rs:51`).
   **Note the plus sign** — this is `∂u/∂t = +c·L·u`, the *backward/anti-diffusion*
   equation, the opposite sign from propagators 1 and 2 in the same modules. See §2.5;
   this is the single largest correctness finding.

Derived quantities on top of the heat kernel (the "PDDL bridge"):
- `rank_i = u_i(t)·s_i` and `cost = Σ_i u_i(t)·s_i`
  (`rust-core/src/lib.rs:361-426`; mirrored `bebop2/core/src/field.rs:217-252`);
- `sensitivity_i = (Σ over runs |Δu_i|) / max_j (…)`
  (`rust-core/src/lib.rs:434-453`; `bebop2/core/src/field.rs:329-344`);
- free energy `F = ½⟨b, L b⟩ − H[b] = ½ Σ_k λ_k ⟨b,φ_k⟩² − H[b]`
  (`bebop2/core/src/active.rs:41-81`), belief relaxation `b' ∝ b − dt·β·L·b`
  (`active.rs:87-111` — note: **correct** minus sign here).

### 1b. Damped wave equation with per-node "solid tensors" (host)

`crates/bebop/src/field_physics.rs` implements, per node i and channel c
(one channel per Platonic-solid vertex, V ∈ {4,6,8,12,20}):

```
v̇_i[c] = ( c² · ( L̂_solid u_i [c] + L̂_graph u [c] ) − γ·v_i[c] + src_i[c] ) / m_i
u̇_i[c] = v_i[c]
```

where `L̂` are *degree-normalized difference* operators, "neighbor mean minus self"
(`field_physics.rs:223-241` intra-solid; `358-373` inter-node), i.e. `−L̂u` in
Laplacian convention — the **correct** sign for a wave/diffusion restoring force.
Mass `m_i = V_i + κ·Σ incident edge weights` (`build_bodies`, `field_physics.rs:97-136`);
source `src = amp·(1+m_seed)/hops²` along BFS hop distance
(`wave_source`, `field_physics.rs:250-294`); integration is semi-implicit
(symplectic) Euler — u is advanced with the *new* v (`field_physics.rs:374-389`);
damping γ = 0.08 (`field_physics.rs:53`). A "fluid advection" term drifts node positions
by a pressure gradient (`field_physics.rs:390-419`) and does not feed back into the wave.
Energy for the stabilizer gate `E = Σ ½m(‖v‖² + c²‖∇u‖²_intra)`
(`wave_energy`, `field_physics.rs:296-321`) — **the inter-node coupling potential is
omitted** (`let _ = adj;`, line 319), which matters for the gate (§2.7).

### 1c. Decision-layer wrappers (host)

- `crates/bebop/src/field.rs`: `field_gate` runs the rust-core heat kernel on a
  **hardcoded 6-node plan graph** (`plan_csr`, field.rs:62-84), seeds an impulse at a
  keyword-mapped node (field.rs:127-146), vetoes if mass on the secrets node > 0.10
  (field.rs:153-161). Plus a textbook scalar Kalman filter (field.rs:174-195) and
  sign-flip limit-cycle heuristics (field.rs:211-256).
- `crates/bebop/src/wavefield.rs`: λ₂ (algebraic-connectivity) "spectral notch" via a
  Numerical-Recipes cyclic Jacobi on the weighted Laplacian (wavefield.rs:365-481),
  Floyd cycle detection over successor arrays (wavefield.rs:176-203), divergence-as-flux
  balance (wavefield.rs:211-221), composed into `plan_wave_gate` (wavefield.rs:518-593)
  with the 1b wave as a flag-off blast-radius check.
- `crates/bebop/src/geometry_field.rs`: Platonic solids (Euler invariant checked),
  spherical-harmonic node signatures, Nyquist winding-number stability check.

**Docs vs code.** The architecture doc (`bebop2/ARCHITECTURE.md`) frames this as
"vector → tensor → wave; replace every dense buffer with spectral primitives". What the
code actually is: a standard graph-Laplacian heat kernel (Chung, *Spectral Graph
Theory*), a standard Chebyshev polynomial approximation of a matrix exponential (the
same construction as Hammond–Vandergheynst–Gribonval 2011 graph wavelets / the kernel
polynomial method), a standard HRR/VSA circular-convolution binding (Plate 1995), a
damped wave ODE with explicit integration, and a Jacobi eigensolver. That is a
respectable, boring toolbox; the "wave/quantum" language is branding. One module —
`coherence.rs` — claims quantum-style superposition over a heat kernel and implements
neither (§2.6).

---

## 2. Numerical-methods audit

### 2.1 FFT (`bebop2/core/src/fft.rs`) — VERIFIED correct

Iterative radix-2 Cooley–Tukey: bit-reversal permutation (fft.rs:129-140), butterfly
with incrementally-multiplied twiddles (fft.rs:141-167), sign −1 forward, inverse scaled
by 1/n (fft.rs:168-174). This is the standard textbook in-place form. Correctness is
pinned the right way: an **independent O(n²) DFT oracle with no shared code**
(fft.rs:189-211) at 1e-12 (fft.rs:242-266), a round-trip identity test, a RED test that
corrupts a coefficient and demands the round-trip break (fft.rs:284-300), and a
circulant-eigenvalue property test. I additionally re-verified the exact code shape in
my harness against brute-force circular convolution (errors ≤ 3.3e-16 for n=8).
Caveats: length must be a power of two — **not asserted**, silent garbage otherwise
(callers pad; see §3 for the VSA consequence); `fexp_local` (fft.rs:66-103) is dead
code; incremental twiddle accumulation grows error ~O(len) — irrelevant at the small
dims used.

### 2.2 Chebyshev propagator (`chebyshev.rs`, duplicated in `rust-core`) — VERIFIED correct, one hazard

Math: approximate `f(L)u0` with `f(λ)=exp(−c·t·λ)` by `Σ_k c_k T_k(ã)`,
`ã = (2/b)L − I` mapping `[0,b]→[−1,1]`. The coefficients
`c_k = (2/qp) Σ_j f(λ(θ_j)) cos(kθ_j)`, `θ_j = π(j+½)/qp`, qp = 64
(chebyshev.rs:125-140) are **Chebyshev–Gauss (midpoint/DCT) quadrature** — the comment
says "trapezoid" (chebyshev.rs:132 and rust-core:209) but midpoint-in-θ is what is
implemented, and it is the *better*, standard choice (exact for integrands polynomial
in cosθ up to degree 2·qp−1). Equivalent to the classical Bessel-function expansion
`e^{az} = I_0(a) + 2Σ_k I_k(a)T_k(z)`. The λmax bound `b = 2·max_degree`
(chebyshev.rs:62-72) is the standard Gershgorin-type bound `λmax(L) ≤ 2Δ` — valid.
Three-term recurrence (chebyshev.rs:142-165) is the standard Clenshaw-style forward
recurrence, matrix-free via CSR matvec.

My oracle experiment (exact dense eigendecomposition, path graph n=20, impulse u0):

| t | deg | max abs err vs exact `exp(−cLt)u0` |
|---|-----|-----------------------------------|
| 1 | 20 | 6.7e-16 |
| 5 | 30 | 8.4e-16 |
| 20 | 40 | 2.3e-10 |
| 20 | 60 | 9.9e-16 |
| 20 | **80** | **4.6e-13** (worse than deg 60) |

Spectral convergence confirmed; the deg=80 regression is the predicted **quadrature
aliasing once deg ≥ qp = 64** — coefficients for k ≥ 64 fold back. There is no guard on
`deg < qp` (deg is caller-supplied). All in-repo callers use deg ≤ 40, so it is a latent
hazard, not a live bug. In-repo tests only pin mass conservation to 1e-2 and rejection
of deg<1 — much weaker than what the code actually achieves (10⁻¹⁰–10⁻¹⁶).

The degree needed scales like the Bessel decay threshold `k ≳ c·t·b/2` (= 40 at
t=20, c=1, b=4), which exactly explains why the repo's `deg=40, tol 1e-2` tests pass.

### 2.3 Jacobi eigensolver (four copies) — VERIFIED correct *for symmetric input only*

Copies: `field.rs:258-323`, `kalman.rs:28-84`, `lyapunov.rs:19-61`,
`wavefield.rs:402-465` (NR-style rotation). Cyclic Jacobi with two-sided rotations is
the standard symmetric eigensolver (Golub & Van Loan §8.5); the rotation formulas match
the textbook. Notes:

- The `phi == 0 → t = 1` "fix" in `field.rs:286-293` is redundant, and its comment is
  wrong about Rust semantics: `f64::signum(0.0)` is **+1.0**, not 0.0 (VERIFIED:
  `(0.0f64).signum() == 1`). The "unfixed" kalman/lyapunov copies behave identically —
  both variants return (3, −1) for `[[1,2],[2,1]]` (VERIFIED in harness). No divergence
  between the copies in practice.
- Eigenvector column-major indexing in `field.rs:96-103` is correct (rotations
  right-multiply V, so eigenvectors are columns); the code comments document the
  transpose trap accurately.
- **Applying Jacobi to non-symmetric matrices is unsound** — see 2.4 (kalman) and 2.8
  (lyapunov), where the code does exactly that.

### 2.4 "Kalman" (`bebop2/core/src/kalman.rs`) — real, correct on its test, misnamed and symmetric-only

What it computes is **not a Kalman filter**: there is no measurement update, no gain,
no innovation, no state estimate. It is the covariance *time-update* recursion
`P_k = A P_{k−1} Aᵀ + Q` (a discrete Lyapunov/Stein iteration), evaluated in the
eigenbasis of A: `acc ← Λ·acc·Λ + Q̃` with `Q̃ = V⁻¹QV` (kalman.rs:199-227). The identity
`P_k = V(Λ M Λ)V⁻¹` equals `A P Aᵀ + …` **only when V is orthogonal (V⁻¹ = Vᵀ), i.e.
A symmetric**. The header claims validity for "real-diagonalizable" A
(kalman.rs:25-27,178) — overstated twice over: (i) for non-symmetric A the congruence
`APAᵀ` is not similarity-diagonalized by V; (ii) the eigendecomposition itself uses
Jacobi, which does not converge for non-symmetric input. The single test uses a
symmetric 2×2 and passes against the dense brute-force oracle at 1e-9
(kalman.rs:277-300) — real verification, but only of the one case where the method is
valid. `invert` (kalman.rs:232-271) is Gauss–Jordan **with** partial pivoting (the
comment says "no pivoting needed"; the code pivots anyway — good). The `_p0` parameter
of `new` is ignored; P0 is passed again to `covariance` — API smell only.
The *scalar* Kalman filter in `crates/bebop/src/field.rs:174-195` is the textbook
predict/update form and is correct (VERIFIED by inspection + its convergence test).

### 2.5 Explicit-Euler diffusion — WRONG SIGN, unconditionally unstable (VERIFIED)

`rust-core field_active` (lib.rs:314-315), `bebop2 active_diffuse` (field.rs:193-196)
and `coherence::propagate` (coherence.rs:51) all step

```
u ← u + dt·c·L·u        (L = D − A positive semidefinite)
```

For the heat equation the graph analog of ∇² is **−L**; forward Euler must be
`u ← u − dt·c·L·u`. With the + sign every non-constant eigenmode is *amplified* by
`(1 + dt·c·λ_k)` per step: the scheme is unstable for **every** dt > 0, and it is not
solving the same PDE as the spectral/Chebyshev propagators that sit next to it in the
same files. Harness measurements (path n=20, impulse, c=1):

- 10 steps, dt=0.2 (the tests' own parameters): field range [−38.7, +25.1],
  ‖u‖₂ = 52.1 — versus the exact heat kernel max 0.386, ‖u‖₂ = 0.53.
- 1000 steps at the "B11 stable corridor" dt=0.02: ‖u‖₂ = **4.7e31** (fixed-sign
  version: 0.298). Exponential divergence, merely slower.
- Total mass Σu stays exactly 1.0 in both signs (1ᵀL = 0), so no mass test can catch it.

Consequences for the repo's own narratives:

- The **B11 story** ("stable dt = 0.02 corridor, never the divergent 0.05",
  field.rs:14, 26, 184, 490-502) is false for this operator: with + sign nothing is
  stable (growth 1.08^k at 0.02 vs 1.2^k at 0.05 for λmax=4); with the correct − sign
  the forward-Euler stability bound is `dt ≤ 2/(c·λmax) = 0.5` on these graphs, so both
  0.02 and 0.05 would be *stable*. The corridor constant encodes a misdiagnosis.
- The **C2 saturate-then-compare** clamp (`du.clamp(±1e6)`, field.rs:195) bounds a
  single step's increment, not the state; the state still grows without bound.
- The claim "matched **bit-for-bit** against old rust-core" (field.rs:17-18) cannot hold
  for `active_diffuse`: bebop2 runs it in f32 (rust-core: f64) and adds the clamp and
  dt-guard rust-core lacks.
- The in-repo tests pass because they run 10 steps and assert only pruning permille and
  finiteness (field.rs:396-415, 489-502; rust-core lib.rs:667-718) — they pin the wrong
  model. The eps-pruning ("active set") concept itself only functions with the − sign:
  under growth, |Δu| increases and pruning asymptotically stops.
- `bebop2/core/src/active.rs:100` uses the **correct** `b − dt·β·L·b` — so the codebase
  contains both signs, and the spectral-vs-iterative disagreement is directly
  observable by comparing the two propagators on any graph. No test does.

### 2.6 `coherence::propagate` (`crates/bebop/src/coherence.rs`) — broken as documented (VERIFIED)

Docstring: "propagates a HEAT-KERNEL impulse u(t) = exp(−L·t)·u0" (coherence.rs:2-6,14).
Implementation: `dt = t.max(1e-3)`, `steps = round(t/dt).max(1)` — i.e. for any t ≥ 1e-3,
**exactly one explicit-Euler step of size t**, with the same wrong + sign
(coherence.rs:33-53). Measured (path of 4, seed at 0, t=1, c=0.5):
output `[1.5, −0.5, 0, 0]` vs exact heat kernel `[0.674, 0.258, 0.058, 0.010]`.
Negative amplitudes from a positive impulse under diffusion are impossible — this is
not a heat kernel by any tolerance. The interference layer `|ψ₁±ψ₂|²`
(coherence.rs:59-78) is trivially correct algebra and its tests pass regardless of the
propagator, which is why the module looks green. Additional defect: out-of-range edge
endpoints alias onto node n−1 via `u[b.min(n-1)]` (coherence.rs:44-46) instead of being
skipped.

### 2.7 Damped wave (`field_physics.rs`) — correct sign, stable; the "dt corridor" test pins artifacts (VERIFIED)

The wave update uses neighbor-mean-minus-self (= −L̂u) — correct restoring force — and
semi-implicit Euler, which is the right symplectic choice for oscillatory systems.
Harness replication (3-node tetra chain, masses 4.5/5/4.5, the repo's own test graph):

| dt | steps | E after impulse | E final | true divergence? | energy rises > 1e-3 on tail |
|----|-------|-----------------|---------|------------------|------------------------------|
| 0.02 | 80 | 0.083 | 0.066 | no | 0 |
| 0.05 | 80 | 0.518 | 0.776 | no | 43 |
| 0.05 | 2000 | 0.518 | 0.044 | **no — decays** | 948 (oscillatory) |
| 0.5 | 2000 | 51.8 | 1e-6 | no — decays | 526 |

So `dt_corridor_stable_small_dt_unstable_large_dt` (field_physics.rs:1089-1147), which
asserts dt=0.05 "diverges", is passing for two reasons that are not divergence:
(1) the impulse is injected as acceleration·dt, so the seeded energy scales with dt —
`e_big > 2·e_small` is guaranteed by the seeding, not instability; (2) `wave_energy`
**omits the inter-node coupling potential** (field_physics.rs:319), so the monitored E
is not a Lyapunov function of the dynamics and oscillates upward transiently, tripping
the `Ė > 1e-3` gate at dt=0.05. The physical claim in the header — "E is monotonically
non-increasing ⇒ Ė ≤ 0 is a true physical fact" (field_physics.rs:40-42) — is true of
the exact damped wave with the *full* energy, and false of the implemented E under
discretization. The gate is therefore conservative (false-positive-prone), which is
fail-closed and safe, but the B11 cross-reference ("0.05 = condemned regime",
field_physics.rs:468-472) imports the §2.5 misdiagnosis into a subsystem that was
actually stable at 0.05. The 1/hops² source falloff and mass=vertex-count+κ·weights are
modeling choices with no derivation — DESIGN-JUDGMENT, internally consistent, tested
for their qualitative claims (heavier target moves less, heavier source radiates more,
falloff by hops: field_physics.rs:615-776).

### 2.8 Lyapunov (`bebop2/core/src/lyapunov.rs`) — unsound for non-symmetric A (VERIFIED misclassification)

`stability_margin = max Re λ(A)` decides `ẋ = Ax` stability — the criterion is textbook
correct. The eigenvalues, however, come from Jacobi sweeps on a general real matrix
(lyapunov.rs:19-61). Orthogonal similarity preserves the spectrum but Jacobi only
*converges to it* for symmetric input; for non-symmetric A the sweeps stall and the
returned "eigenvalues" are just the diagonal after 100 sweeps. The header explicitly
claims complex-eigenvalue support ("Returns complex eigenvalues (real parts matter)",
lyapunov.rs:17-18) — the imaginary parts are always zero (lyapunov.rs:60) and the real
parts are wrong off the symmetric cone. Harness sweep over the always-stable family
`A = [[a,b],[−c,a]]` (true eigs a ± i√(bc), a<0): **46 of 80 cases misreported
UNSTABLE**, e.g. `[[−0.05,1],[−3,−0.05]]` → margin +0.373. All in-repo tests use
diagonal matrices (lyapunov.rs:97-147), where the method is trivially exact. Also,
despite the module name, no Lyapunov equation `AᵀP + PA = −Q` is solved and no Lyapunov
exponent is estimated — the module computes a spectral abscissa (badly, off-symmetric)
and a spectral radius. `spectral_radius` (lyapunov.rs:91-95) has the same symmetric-only
restriction.

### 2.9 The rest (host geometry/heuristics) — mostly correct, minor deviations

- **λ₂ notch** (`wavefield.rs:365-481`): weighted Laplacian assembly and NR Jacobi are
  correct (symmetric input by construction); λ₂ < frac·λmax as a brittleness flag is a
  reasonable use of algebraic connectivity (Fiedler value). Tests pin known spectra
  (chain [0,1,3], clique, disconnected). VERIFIED. By contrast `graph_fourier_notch`
  (wavefield.rs:151-163) is *not* Fourier anything — it is a peak-share concentration
  heuristic on the amplitude vector; the name oversells (the code comments admit
  "proxy").
- **Floyd cycle** (`wavefield.rs:176-203`): fast/slow pointers over a successor array
  with a halt sentinel and an iteration guard; correct for its input class (functional
  graphs). VERIFIED by tests.
- **Nyquist** (`geometry_field.rs:262-292`): winding number accumulated as
  `asin(cross/|v0||v1|)` clamped to ±π/2 — per-segment turns > 90° are truncated, so
  coarse contours can undercount encirclements; `atan2(cross, dot)` would be exact.
  Correct on the sampled test contours. Real-but-approximate; DESIGN-JUDGMENT.
- **Spherical harmonics** (`geometry_field.rs:172-232`): Legendre recurrences are
  standard, but the `P_m^m` prefactor implements sign `(−1)^{⌊m/2⌋}` instead of the
  Condon–Shortley `(−1)^m` (geometry_field.rs:186) — wrong sign for m ≡ 1, 2 (mod 4);
  the real-form Y omits the √2 normalization for m ≠ 0, so "orthonormal" is inexact;
  `factorial: usize` overflows for l+m ≥ 21. All uses are m=0 or magnitude-only
  signatures, so no live impact. Tetrahedron `vertices_spherical` places 3 vertices on
  the equator + pole — not a regular tetrahedron (regular: base colatitude ≈ 109.47°),
  but `vertex_edges`' nearest-neighbor reconstruction still yields K4, and the Euler
  invariant tests (V−E+F=2, degree checks: field_physics.rs:1068-1086) pass for all
  five solids. VERIFIED for the properties actually used.
- **mathx.rs**: central differences (2nd-order standard), Lagrange interpolation with
  duplicate-abscissa guard, first-order step response closed forms, trajectory
  classification by segment amplitude — all correct and RED+GREEN tested. VERIFIED.
- **fexp/fcos shims**: bebop2's `fexp` (lib.rs:325-357) uses symmetric range reduction —
  correct for all signs (tested in-repo to 1e-12; the C8/F3 audit fix is real).
  rust-core's *older* shims still use `fround(x) = ftrunc(x+0.5)` (lib.rs:522-524),
  which mis-rounds negatives. Measured impact: `fexp` max rel err on (−50,0) is
  **5.2e-15** (the 24-term Taylor absorbs the enlarged |r| ≤ ~1.04) — the "bug" is
  contract-level, not numerically material where used. `fcos` however is badly wrong
  for negative arguments (abs err up to **10.5** near x = −22) — latent only, because
  every in-repo call site passes θ ≥ 0 (max err 3.5e-9 on [0, 40π]). VERIFIED both ways.

---

## 3. VSA / spectral representation: realized vs aspirational

**Realized:** `vsa.rs` implements binding as circular convolution via FFT — `bind(a,b)
= IFFT(FFT(a) ⊙ FFT(b))` (vsa.rs:30-54), the standard HRR construction (Plate 1995).
It is pinned against O(n²) brute-force circular convolution at 1e-9 (vsa.rs:183-205) —
real verification, and my replication agrees (3.3e-16 at n=8). `unbind` is *exact
Wiener-style deconvolution* `conj(A)·X/|A|²` with a 1e-30 null guard (vsa.rs:60-87) —
mathematically exact whenever the key has no spectral nulls. The header's justification
("for ±1 vectors |FFT(a)| = 1", vsa.rs:8-10, 57-59) is false in general (Parseval gives
mean |A_k|² = n, not 1), but the code never relies on it; the tests knowingly choose
full-spectral-support keys and document that periodic ±1 keys are non-invertible
(vsa.rs:139-143, 215-223). Bundling = elementwise mean; similarity = cosine. All
standard.

**Hazard (VERIFIED):** non-power-of-two dimensions are silently wrong. `bind` zero-pads
to m = next_power_of_two(n) and returns the first n entries of an m-periodic
convolution, which is not the n-periodic one: measured max abs error 1.70 at n=6 and
4.43 at n=12 vs true circular convolution. `padded_dim` documents the constraint
(vsa.rs:19-23) but nothing asserts it; every in-repo test uses n ∈ {16, 32, 64}.

**Aspirational:** the pillar "spectral coefficients replace dense buffers"
(ARCHITECTURE.md table; field.rs:9-10 "Dense adjacency is NEVER formed") is only
half-true in code:

- Hypervectors enter and leave `bind`/`unbind` as **dense `&[f64]`**; the Fourier form
  is transient scratch, not the storage format. `algebra::project/reconstruct`
  (algebra.rs:78-102) provides the store-as-coefficients primitive but is only ever
  exercised with a delta basis in its own test.
- `LaplacianSpectrum::from_edges` **does form the dense n×n Laplacian** (field.rs:76-84)
  to feed Jacobi — transiently, but O(n²) memory and O(n³)-ish time all the same; and
  the struct keeps the full CSR alongside the spectrum (field.rs:37-40). Honest for
  "small reference graphs" (its own words), not a dense-buffer replacement at scale.
- Cross-module reality check (grep): **nothing outside the defining modules consumes
  `LaplacianSpectrum`, `vsa::bind/unbind`, `SpectralKalman`, or `stability_margin`**
  except `active.rs` (uses `LaplacianSpectrum`). The bebop2 spectral stack is a
  self-contained, well-tested library, not a wired pipeline. The field that is actually
  wired into the product is rust-core's C-ABI (`field_build/field_spectral/field_rank/
  field_cost` → `crates/bebop/src/field.rs` → cli.rs/mcp.rs/wiring.rs) plus the
  field_physics/wavefield gates (mcp.rs, stress.rs, multipilot.rs).

---

## 4. Determinism and zero-dep claims

- **Zero-dep:** VERIFIED for both cores by manifest: `bebop2/core/Cargo.toml`
  (`[dependencies] # none.`) and `rust-core/Cargo.toml` (no dependencies section
  contents). The host crate `bebop` has ~20 deps (ratatui, serde, RustCrypto…), but the
  zero-dep claim is scoped to the cores, which is accurate. `wavefield.rs` uses serde
  derives — host-side only.
- **Empty-import wasm:** VERIFIED for the wasm field core. Built `bebop-core` for
  `wasm32-unknown-unknown` (release) and parsed the binary: **no import section at all**
  (section ids present: 1,3,4,5,6,7,9,10,11,0). No clock, RNG, or host function is
  reachable — the clock-free/deterministic claim holds structurally.
- **Scope caveat:** bebop2's field/vsa/kalman/lyapunov/chebyshev/fft modules are gated
  behind the default-on `host` feature and are **excluded from the no_std wasm build**
  (bebop2/core/src/lib.rs:280-302; Cargo.toml:24-33). So "zero-dep spectral field core
  compiled to wasm" is true of rust-core; the bebop2 analytic stack is native-host code.
- **Run-to-run determinism:** VERIFIED at the code level — no RNG, no
  SystemTime/Instant in any field path (grep clean; `Instant` appears only in the
  telemetry example, explicitly flagged as measurement), fixed iteration order,
  deterministic layouts (layout_spring is RNG-free), tests assert identical traces
  (field_physics.rs:817-822, wavefield.rs:717-725). rust-core's single global graph
  is Mutex-serialized (lib.rs:24-48); concurrent build+propagate interleavings can
  overwrite each other's graph (acknowledged in its own test, lib.rs:838-843) —
  deterministic only when callers serialize, which host `field.rs` does via FIELD_LOCK
  (crates/bebop/src/field.rs:14-18).
- **Cross-platform bit-exactness:** split verdict. rust-core's propagator path uses
  only hand-rolled `fexp/fcos` + IEEE arithmetic → bit-stable across conforming
  platforms (VERIFIED by construction; its `sinc` uses libm `x.sin()` but is not in the
  propagator path). bebop2 under the default `std` feature delegates `fsin/fcos/fln`
  to **platform libm** (lib.rs:71-120), whose results are not specified to the last bit
  by IEEE 754 — cross-platform bit-identity of the bebop2 analytic kernel is therefore
  UNVERIFIED and should not be assumed; only the no_std Taylor shims (off by default)
  are algorithmically fixed. `sqrt` is IEEE-exact — fine everywhere.
- **KATs:** `bebop2/core/kat/` and `src/kat/` contain **crypto vectors only**
  (FIPS 203/204, RFC 8439 etc.). There are **no field/spectral known-answer vectors**;
  field correctness rests on in-test property checks and the FFT/VSA/kalman in-test
  oracles. Given the §2.5 finding, a committed spectral-vs-iterative consistency KAT is
  exactly the missing artifact.

---

## 5. Per-module correctness verdicts

Legend: **RCT** = real + correct + tested (pinned by a meaningful oracle/property);
**RU** = real but unverified/weakly tested; **RW** = real but wrong (math defect);
**MIS** = misdocumented (code ≠ its own claims); DJ = design-judgment element.

| Module / function | Verdict | Evidence (file:line) |
|---|---|---|
| bebop2 `fft.rs` | **RCT** | DFT-oracle 1e-12 + RED corruption test (fft.rs:242-300); pow2 unasserted; dead `fexp_local` (66-103) |
| bebop2 `chebyshev.rs::spectral_propagate` | **RCT** (externally verified here) | 2.3e-10 vs exact eigen-oracle at deg40/t20 (harness); in-repo pins mass only at 1e-2 (206-217); aliasing hazard deg ≥ qp=64 unguarded (125-140); "trapezoid" comment inaccurate (midpoint) |
| bebop2 `field.rs::from_edges/jacobi_eigen/propagate_spectral/matvec_f32/rank/cost` | **RCT** + MIS | λ0=0, mass, L·1=0, rank=cost tests (346-521); dense L formed transiently contradicting header (76-84 vs 9-10); "bit-for-bit vs rust-core" false for f32 active path (17-18); signum comment wrong re Rust semantics (286-289, harmless) |
| bebop2 `field.rs::active_diffuse` | **RW** | `u += dt·c·L·u` anti-diffusion (193-196); VERIFIED ‖u‖ = 4.7e31 after 1000 steps at "stable" dt=0.02; B11 corridor narrative false (26, 490-502); tests too short to catch (396-415) |
| rust-core `field_active` | **RW** | same + sign (lib.rs:314-315); same experiment |
| rust-core `spectral_propagate/field_rank/field_cost/field_sensitivity` | **RCT** | 19/19 tests pass; matches exact heat kernel via §2.2 experiment; deterministic ABI |
| rust-core `fexp/fcos` shims | RU→OK / latent | fexp neg-arg rel err 5.2e-15 (VERIFIED harmless); fcos abs err up to 10.5 for x<0 (VERIFIED) but only called with θ≥0 (3.5e-9 on [0,40π]) |
| bebop2 `vsa.rs` | **RCT** (pow2 dims) + MIS | brute-force conv oracle 1e-9 (183-205), exact unbind round-trip + wrong-key RED (208-251); silently wrong non-pow2 dims (VERIFIED err 1.7–4.4); "‖FFT(±1)‖=1" claim false but unused (57-59) |
| bebop2 `algebra.rs` | **RCT** | oracle-matching tests (104-184); `project/reconstruct` only delta-basis tested (weak) |
| bebop2 `kalman.rs` | RU + MIS | correct vs dense oracle on symmetric 2×2 (277-300); **not a filter** (no update step); symmetric-only, "diagonalizable A" claim false (25-27); Jacobi on non-symmetric invalid |
| bebop2 `lyapunov.rs` | **RW** (off-symmetric) + MIS | VERIFIED 46/80 stable non-symmetric systems misreported UNSTABLE (harness `lyap_flip`); "complex eigenvalues" claim false (17-18, 60); correct for symmetric/diagonal (all in-repo tests, 97-147); no Lyapunov equation/exponent computed |
| bebop2 `active.rs` | **RCT** | spectral F == dense ½bᵀLb−H at 1e-9 (127-149); belief step uses correct −L sign (100) |
| crates/bebop `field.rs` (gate + scalar KF) | **RCT**, demo-scale DJ | fail-closed CSR guards + veto tests (258-386); hardcoded 6-node plan graph and keyword→node map (62-146) — an arbiter demo, not a general planner |
| crates/bebop `coherence.rs::propagate` | **RW + MIS** | claims heat kernel (2-6); is ONE Euler step of size t with + sign (33-53); VERIFIED [1.5,−0.5,0,0] vs exact [0.674,0.258,0.058,0.010]; edge-index aliasing (44-46); interference algebra itself trivially correct |
| crates/bebop `field_physics.rs` | **RCT** with caveats | correct wave sign + symplectic Euler (334-389); VERIFIED stable at dt≤0.5 long-run; `wave_energy` omits coupling PE (319) so the Lyapunov gate is conservative-not-exact; dt-corridor test pins seeding artifact + gate trips, not divergence (1089-1147); 1/hops² source & mass model = DJ |
| crates/bebop `wavefield.rs` | **RCT** | λ₂ spectra pinned on chain/clique/disconnected (748-789); Floyd correct; `graph_fourier_notch` is a concentration heuristic, name oversells (139-163) |
| crates/bebop `geometry_field.rs` | RU/DJ | Euler invariants + harmonic tests pass (294-401); CS phase deviation (186), missing √2 (220-232), asin-clamped Nyquist winding (284-287), factorial overflow l+m≥21 — none live |
| crates/bebop `mathx.rs` | **RCT** | closed-form tests (120-191) |
| `examples/tree_vs_field_telemetry.rs` | honest harness | self-checking (exits 1 on violated invariants, 350-422); measures that CH routing beats the pure wave — the repo's own numbers demote the wave for cost search |
| KATs (`bebop2/core/kat/`, `src/kat/`) | crypto-only | no field/spectral vectors committed |

### Bottom line

The spectral spine — FFT, Chebyshev heat-kernel propagation, Jacobi-on-symmetric,
VSA binding, spectral free energy — is real, standard, and better than its own tests
say (verified here to 1e-10..1e-16 against independent oracles). The three defects that
matter: (1) every *iterative* diffusion path integrates the heat equation with the
wrong sign and is unconditionally unstable — masked by short tests, mass conservation,
and a "dt corridor" folklore constant that encodes a misdiagnosis; (2)
`lyapunov.rs`/`SpectralKalman` silently apply a symmetric-only eigensolver to general
matrices while their docs claim otherwise — stability verdicts off the symmetric cone
are unreliable; (3) `coherence::propagate` does not compute what its documentation
says at all. Everything else is solid library code whose main honest limitation is
that the bebop2 spectral stack is not yet consumed by anything except its own tests.
