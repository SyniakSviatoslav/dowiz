# Equations Library — 2026-07-19

> Two columns, one file: every equation named in the operator's pasted prompt
> (`OPERATOR-PROMPT-VERBATIM-2026-07-19.md`, same directory), and every equation already
> implemented in this codebase, with `file:line` citations verified live against the working
> tree (not the stale Repowise index). Built for the research/synthesis/blueprint passes that
> follow — the point of this file is to make "is this already implemented?" a lookup, not a
> re-derivation, per the standing rule to check what already works in-repo before building new.
>
> Scope note: this file catalogues: it does not judge fit or propose wiring. That's
> `SYNTHESIS-*` / `BLUEPRINT-*` work downstream (tasks #5–7).

---

## §1. Gradient / potential-field family (prompt §1, §C1, §C2)

| Equation | Prompt form | Existing kernel equivalent |
|---|---|---|
| Gradient of a scalar field | `∇φ = ∂φ/∂x·î + ∂φ/∂y·ĵ + ∂φ/∂z·k̂` | Not implemented as a literal `∇` operator; the kernel's analogous construct is the **graph gradient** `B` in `kernel/src/incidence.rs:73` — `(Bx)_e = x_head − x_tail` (discrete edge-gradient on a graph, not a continuous field) |
| Force = −∇(potential) | `F = −∇U`, `g = −∇Φ`, `E = −∇V`, `q = −k∇T` | The canonical reference Laplacian `L = BᵀWB = div(W·grad(x))` at `kernel/src/incidence.rs:102` **is** this pattern discretized on a graph: `laplacian()` = divergence of a weighted gradient. `field_frame` computes `∇²U = −(D−A)U` (`incidence.rs:9`) — same minus-sign convention as `F=−∇U`. |
| Poisson / Laplace / Helmholtz (BVP, prompt §C2) | `−∇²u=f`, `∇²u=0`, `∇²u+k²u=0` | `Csr::laplacian_spmv(x, out, kind)` with `LaplacianKind::{Unnormalized,Normalized,RandomWalk}` (`kernel/src/csr.rs:307-359`) is the discrete `L·x` operator these PDEs are built from once discretized on a graph/mesh. |
| Navier–Stokes (prompt §C1) | `ρ(∂v⃗/∂t+(v⃗·∇)v⃗)=ρg⃗−∇p+μ∇²v⃗`, `∇·v⃗=0` | **Not implemented.** No fluid-sim surface in this kernel. Flagged as prompt-only content — see synthesis pass for a fit/no-fit call, not assumed relevant. |

## §2. Spectral / eigenvector family (prompt asks explicitly: "scalar & thermodynamics equations logic stored in the eigenvectors")

| Equation | Prompt form | Existing kernel equivalent |
|---|---|---|
| Graph Laplacian | `L = D − A` | `kernel/src/spectral.rs:642` — `Graph Laplacian L = D − A of an (assumed symmetric) adjacency matrix`. Two independently-implemented sign conventions confirmed live and reconciled: `field_frame` computes `−(D−A)U`, `csr`/`spectral` compute `+(D−A)U` (`incidence.rs:9`, proved-not-a-bug by the E1 Laplacian-parity blueprint, `kernel/src/incidence.rs` module docstring lines 2–17). |
| Oriented incidence factorization | `L = BᵀWB` | `kernel/src/incidence.rs:17,32,102` — a genuinely different shape from `D−A` (one row per **edge**, not per node); orientation-invariant. |
| Eigendecomposition | (implicit — "stored in the eigenvectors") | `kernel/src/householder.rs` — Householder→Hessenberg→QR, O(n³), `householder::eigh_contig` (Jacobi, n≤32). `kernel/src/spectral.rs:249` — `spectral::eigh` façade returns `(basis, values) == Decomp`, `UᵀU=I` to 1e-9 KAT (Phase 28 rung-1, commit `03ac0fefe`). `kernel/src/spectral.rs:685` — `topk_symmetric` sparse top-k eigenpairs (LCG-seeded). |
| Spectral gap | `γ = 1 − |λ₂|` | `kernel/src/spectral.rs:21,585` — governs BOTH stability (`|λ|`-vs-1) and iteration count; for a row-stochastic matrix (`λ₁=1`) this is the mixing-time proxy. |
| Spectral radius (Perron–Frobenius) | `ρ(A) = max|λ|` | `kernel/src/spectral.rs:558,747` — `ρ(A) — spectral radius = largest eigenvalue modulus`. Used as the drift-classification gate (`event_log.rs:347-375`: **rejects an `Unstable` (ρ>1) mutation pre-persist**). |
| Graph energy | `E = Σ|λᵢ|` (McClelland bound `E ≤ √(2mn)`, Koolen–Moulton `E ≤ n(1+√n)/2`) | `kernel/src/spectral.rs:592-600,746` — implemented; a prior wrong bound comment (`2(n−1) ≤ E ≤ 2n√(n−1)`) was found-and-fixed live (empty graph `E=0` breaks the old lower bound). |
| Characteristic polynomial / dominant period | (n/a in prompt) | `kernel/src/spectral.rs:145` `charpoly_in() -> Option<Vec<f64>>`; `spectral.rs:769` `dominant_period() -> Option<f64>`. |
| Personalized PageRank (spectral diffusion, not in prompt but the kernel's actual "eigenvector-adjacent" recall primitive) | `π = α·e + (1−α)·A·π` (Jacobi) | `kernel/src/csr.rs:330-360` — fixed-K, fixed-summation-order, bit-reproducible. |

**Rust `Option<T>` pattern actually used across this math surface** (the prompt's second explicit ask):
`Option<Vec<f64>>` / `Option<f64>` is the kernel's convention for **"this decomposition/derivation may not converge or may be structurally undefined for this input"** — not a generic nullability hack:
- `spectral::charpoly_in(...) -> Option<Vec<f64>>` (`spectral.rs:145`) — `None` when the arena can't serve the transient matrices.
- `spectral::dominant_period(...) -> Option<f64>` (`spectral.rs:769`) — `None` when no dominant period exists (e.g. no complex eigenvalue pair).
- `Csr::personalized_pagerank_in(...) -> Option<Vec<f64>>` (`csr.rs:467`) — `None` on arena exhaustion, not on numerical failure (PPR itself always converges for `alpha<1`).
- `geo::CourierSpeedEma::observed(&self) -> Option<(f64,u32)>` (`geo.rs:261-263`) — **CORRECTED 2026-07-19 (SYNTHESIS §1; Research A §2.4): the prior claim here was wrong.** There is no `LiveSpeed` type, and `observed()` returns `Some((self.v_hat, self.pings))` **unconditionally** — a cold smoother returns `Some((0.0, 0))`, never `None`. It is a *saturating sentinel* (the exact opposite of "undefined-until-observed") and belongs with the Wilson `n=0` case below, not here. The `Option<(f64,u32)>` the prior pass actually saw is a different construct — the function *parameter* `live_speed_mps: Option<(f64,u32)>` of `eta_seconds_adaptive` at `geo.rs:213`, not the return of `observed()`. Drop the "thermodynamic state undefined until observed" analogy: no code in `geo.rs` supports it.
- Wilson score interval deliberately does **NOT** return `Option` at `n=0` — it returns the maximally-uncertain `(0.0,1.0)` instead (`stats.rs:99`) — a documented, explicit design choice to prefer a saturating sentinel over `None` when the caller cannot branch cheaply. This is the one place the codebase explicitly rejected `Option<T>` for this class of "possibly undefined" derivation, and is worth citing verbatim in the synthesis pass as the codebase's own stated tradeoff.

## §3. Thermodynamics / statistical-mechanics-adjacent (prompt §1.4, §4.1, entropy in §D5)

| Equation | Prompt form | Existing kernel equivalent |
|---|---|---|
| Fourier's law (heat conduction) | `q⃗ = −k∇T` | Same minus-gradient shape as `L=BᵀWB`/`field_frame` above — no literal thermal surface, but the operator identity is the one `incidence.rs` module docstring explicitly claims to unify ("`∇²U=−(D−A)U`... the graph Laplacian... is implemented ≥3 times"). |
| Shannon entropy | `H(S) = −Σpᵢlog₂(pᵢ)` | **Already implemented, independently, twice:** (1) `kernel/src/markov.rs:179-189` — entropy rate of the tool-outcome Markov chain: `row_h -= p*p.log2()` summed and weighted by stationary `pi[i]`, i.e. `H = Σᵢ πᵢ·(−Σⱼ p_ij log₂ p_ij)`. (2) conceptually mirrored by `kernel/src/spectral.rs` graph energy as an alternate "disorder" measure. **This is the single strongest existing cross-link in the whole prompt** — the reference doc's Entropy formula (§D5) is not a new idea to import, it is byte-for-byte the formula already live in `markov.rs`, just applied to tool-call transitions instead of a generic distribution. |
| Foster-Lyapunov drift / progress potential | (not named in prompt; the general "thermodynamic potential decreasing" idea the prompt's `F=−∇U` section gestures at) | `kernel/src/markov.rs:29` `potential(s)` (Foster-Lyapunov, `run_ok`→1.0 escape state); `kernel/src/noether.rs:82` `lyapunov_nonincreasing()` — one-sided Lyapunov check, `|proposed−current| ≤ tol`, rejects "spontaneous growth beyond slack". `kernel/src/evals.rs:764,796,824` — conserved quantity `Σx²` as a Lyapunov invariant guard (noether-guard pattern). **CORRECTED 2026-07-19 (SYNTHESIS §4; Research A §4.2):** these are **Lyapunov / dissipation guards that certify the *downhill consequence*, NOT an implementation of `F=−∇U`.** They verify a *supplied* potential `V` is non-increasing along a *supplied* update `f`; there is no `∇`, and no requirement that `f = −∇V`. "V non-increasing" is a **necessary-not-sufficient** signature of gradient flow — a plain contraction `x↦0.9x` (not a gradient flow of the energy it checks) also passes (`noether.rs:153-191`). The module's own HONESTY NOTE (`noether.rs:17-24`) pre-emptively refuses this over-reading. The genuine `F=−∇U` correspondence lives at the **Laplacian layer** (`incidence.rs`: `L = BᵀWB = div(W·grad)`, so `∇E = Lx` for Dirichlet energy `E=½xᵀLx`, and a heat step `x ← x − ηLx` IS gradient descent on it) — NOT at these guards. |
| CLT / √N convergence | `X̄ₙ →d N(μ,σ²/n)` | `kernel/src/stats.rs:35,116-124` — `within_clt_envelope`, `error·√n < asymptotic_se·z`. Directly implements prompt §4.1's formula, already shipped (E2 blueprint, commit `6bd181a02`, Wilson lower bound 0.7575 on the 12-query oracle). |
| Standard error of the mean | `SE = s/√n` (Bessel-corrected) | `kernel/src/stats.rs:63` — verbatim. |
| Wilson score interval | (not in prompt; kernel-only) | `kernel/src/stats.rs:91-99,230,254` — closed form `n/(n+z²)` at `p̂=1`; deliberately does not degenerate like Wald. |

## §4. Control theory / signals family (prompt §B1–B5)

| Equation | Prompt form | Existing kernel equivalent |
|---|---|---|
| LTI convolution | `y(t)=x(t)*h(t)`, `H(s)=L{h(t)}` | **Not implemented.** No transfer-function/Laplace surface in this codebase. |
| Closed-loop gain | `T(s)=L(s)/(1+L(s))`, `S(s)=1/(1+L(s))`, `T+S=1` | Conceptually closest existing pattern: `WorkerSlots` semaphore (degrade-closed `Busy`, A4 fix commit `5ef8fbb78`) and `TokenBucket::refill_locked` (`kernel/src/token_bucket.rs:67-77`: `tokens=(tokens+refill_rate·elapsed).min(capacity)`, saturating both directions) — both are **feedback-regulated capacity controllers**, structurally a P-controller on a bounded resource, but not derived from or checked against `T+S=1`. No literal transfer-function math exists to compare. |
| Gain/Phase margin | `GM=1/|L(jω_pc)|`, `PM=180°+∠L(jω_gc)` | **Not implemented.** Flagged prompt-only. |
| PWM duty cycle | `V_avg=(D/100)·Vcc` | **Not implemented** — no embedded/MCU surface in this codebase (dowiz is a delivery-OS backend + wgpu client, not firmware). Flagged as almost certainly out-of-scope; confirm in synthesis pass rather than silently drop. |

## §5. AI-engineering / self-improvement family (prompt §3, §D1–D2)

| Concept | Prompt form | Existing dowiz equivalent |
|---|---|---|
| Self-Harness (Weakness Mining → Harness Proposal → Proposal Validation, non-regressive acceptance) | 3-stage loop, model-specific | `docs/design/spectral-energy-flow-evolution-2026-07-16/BLUEPRINT-E3-self-harness-loop-for-llm-harness.md` — **already blueprinted 2026-07-16**, Phase-A advisory-only, Phase-B hard-blocked on P06 key_V (now CLOSED 2026-07-18/19 per commit `58987d79d` / `5ef8fbb78` — re-check whether E3 Phase-B is now unblockable, this is a live status question for the research pass). |
| Eval-and-Optimizer loop (Microsoft Foundry pattern) | Run rubrics → pass? → ship / Optimizer → candidate fixes → Scores → Promote best | Structurally same shape as this repo's own `librarian`/`ratchet-critic`/`cause-critic` HARNESS council (Stage 8/10) — advisory store, deterministic promotion, non-regressive by design. Not cited as identical, only as the same *shape* of loop already running here under a different name. |
| AIDE² recursive self-improvement (Worker/Improver outer-loop rewriting inner-loop agent) | outer-loop rewrites inner-loop scaffold | **Not implemented as such.** Closest analogue is the Markov attractor loop-signal system (`kernel/src/markov.rs`, tool-outcome entropy/escape-mass/Lyapunov, advisory/fail-open) — same *family* (agent watching its own trajectory statistics) but does not rewrite its own harness. Flagged as genuinely new territory for the synthesis pass. |

## §6. Decision-framework family (prompt §5.1) — no code equivalent, process-only

RICE = `Reach × Impact × Confidence ÷ Effort`. MoSCoW, Eisenhower, Pareto (80/20), OKR, Kano — none of these have or need a kernel equivalent; they are candidate **prioritization lenses for the roadmap-update pass itself** (task #7), not code to wire in. Flagged here so the synthesis pass doesn't waste cycles looking for a "MoSCoW module."

## §7. Neural-field / GPU-rendering family (prompt Document 4)

Already fully extracted with citations in
`docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md` §4
(this is the source the prompt's Document 4 is a byte-identical copy of). Key equations for
completeness, all already sitting in that file and already reconciled against dowiz's wgpu-sole /
zero-JS-math house rules in `R-LM-living-memory-visualization-architecture.md`:

- LIF: `τ_m dV/dt = −(V−V_rest) + R·I`
- Izhikevich: `v'=0.04v²+5v+140−u+I`, `u'=a(bv−u)`, reset `v←c, u←u+d` on `v≥30mV`
- Hodgkin-Huxley: `C_m dV/dt = −(g_Na m³h(V−E_Na)+g_K n⁴(V−E_K)+g_L(V−E_L))+I`
- Cable equation: `(a/2R_i)∂²V/∂x² = C_m∂V/∂t + I_ion`

**Cross-link already made by the 2026-07-16 pass, worth restating:** `R-LM-living-memory-visualization-architecture.md:22-37` establishes that this visualization needs **NO new dynamics equations at all** — positions come from `L=D−A`'s low eigenvectors (§2 above), activity from `Csr::personalized_pagerank` (§2 above), health from `spectral::graph_spectrum` (§2 above). The Izhikevich/LIF/HH spiking math in this section is **third-party reference material that was NOT adopted** for the actual dowiz implementation, precisely because the existing spectral/PPR primitives already cover positions+activity+health without needing a separate neuron-simulation layer. This is the single most important synthesis finding to carry forward: don't re-import spiking-neuron ODEs the prior pass already declined.

## §8. Statistics / ML formulas (prompt §D5, §F2) — cross-reference table

| Formula | Prompt | Kernel |
|---|---|---|
| Entropy `H=−Σp·log₂p` | §D5 | `markov.rs:179-189` (see §3 above) |
| Euclidean distance `d=√Σ(xᵢ−yᵢ)²` | §D5 | **CORRECTED 2026-07-19:** intentionally absent, not a gap. Retrieval is graph-diffusion (hop-distance), not vector-L2 (`retrieval/diffusion.rs:274`; `retrieval/index.rs:80` "No Bloom filter, no compression ⇒ bitwise reproducibility"); PPR-diffusion has no L2 step to add, and adding one would break bit-determinism. |
| Softmax `P(yᵢ)=e^zᵢ/Σe^zⱼ` | §D5 | **CORRECTED 2026-07-19 (prior "not found" was a false negative — shallow grep of `retrieval/*.rs` only):** IMPLEMENTED — `attention.rs:22-36` (row-max-stable, fixed summation order, bit-reproducible native/wasm), `simd.rs:36,164` (SoA SIMD batch, asserted bit-identical to scalar). |
| Cross-entropy `L=−Σyᵢlog(ŷᵢ)` | §D5 | **CORRECTED 2026-07-19 (prior "not found" was a false negative):** IMPLEMENTED as the log-loss of `online.rs`'s `NaturalLogistic` learner — `online.rs:144` sigmoid `σ(t)=1/(1+e⁻ᵗ)`, `online.rs:167-177` "gradient of the log-loss is the prediction error `(y−p)`". |
| CLT / t-Test / ANOVA family | §4.1, §F2 | `stats.rs` implements the CLT envelope + Wilson interval only (§3 above); no t-Test/ANOVA/Chi-Square surface exists or is needed (no experiment-design surface in this codebase). |

**Honest gap note — RESOLVED 2026-07-19 (Research A §1, Research B §D5, SYNTHESIS §1).** The prior
"not found anywhere in the current kernel" claim above was a shallow-grep false negative (it grepped
only `retrieval/*.rs`). Corrected term by term against the operator's closing framing ("where data
should be use information calculation gain with entropy as mismatch on internal digital euclidean
distance stabilized by softmax equation"):
- **Softmax** — IMPLEMENTED (`attention.rs:22-36`, `simd.rs:36,164`). Not a gap.
- **Cross-entropy** — IMPLEMENTED as `online.rs` log-loss (`online.rs:167-177`). Not a gap.
- **Cosine similarity** — IMPLEMENTED (`leak_gate.rs:28-47`), used as a dedup / anti-contamination gate
  at threshold `0.9`, NOT as a retrieval ranker (a distinction the synthesis must not blur).
- **Euclidean / L2 distance** — genuinely and **intentionally** absent. The retrieval layer is declared
  "vectorless, deterministic" (`retrieval/mod.rs:1`); ranking is PPR-diffusion + BM25/trigram fusion,
  which needs no L2 metric and no softmax normalization (PPR mass already sums to 1, `ppr.rs:15-16`).
  Forcing softmax-over-L2 into this path would be a determinism regression, not an improvement.
- **Information gain** (`H(prior) − H(posterior)`) — the ONE genuinely useful missing primitive: entropy
  already exists (`markov.rs:179-189`, `intake.rs:406-443`); the *gain* (difference) is not yet a named
  quantity. This — not softmax/euclidean — is the honest home for the operator's "information calculation
  gain with entropy as mismatch" phrase, as a scoring / active-selection signal (see SYNTHESIS §2).

---

## Provenance

Built 2026-07-19 in worktree `research/equations-thermo-eigenvector-2026-07-19`
(`/root/dowiz-wt-eq-thermo-gpu`), branched from `main`. Kernel citations verified live via `grep`/
`Read` against `kernel/src/*.rs` at branch point (main @ `5a97e1f6f`), not the Repowise index.
Companion files: `OPERATOR-PROMPT-VERBATIM-2026-07-19.md` (source text), `TOPICS-INDEX-2026-07-19.md`
(full theme/topic enumeration, non-equation content).
