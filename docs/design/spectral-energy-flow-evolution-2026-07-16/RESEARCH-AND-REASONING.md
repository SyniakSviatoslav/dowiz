# Spectral / Energy-Flow Evolution — Research & Reasoning (2026-07-16)

> Task: classify 10 concept clusters from the operator's reference package against the LIVE
> code of the energy-spectral-circuit-flow architecture. Discipline applied throughout: V6
> metaphor rule (a concept "connects" only if a named, computed, falsifiable thing in the
> codebase would be extended or corrected) and Hermetic P2 (never propose a new unification
> on top of an already-fractured primitive without naming that as the precondition).
> Branch: `feat/spectral-energy-flow-evolution` (worktree, off merged `feat/harness-llm-backend`).
> No code was written or edited; this is planning/research only.

---

## §1 — Live-code grounding (what was actually read, and what it actually is)

**`kernel/src/spectral.rs` (657 lines).** A general non-symmetric eigensolver: Faddeev-LeVerrier
charpoly + Durand-Kerner roots as fallback, Householder fast path for n ≤ 32
(`eigenvalues`, :195-214). Exposes `spectral_radius` (:217), `slem` (:222), `spectral_gap` (:235),
`graph_energy` E = Σ|λ| (:246), Fiedler `algebraic_connectivity` (:302), and the DMD-style
`DriftClass {Damped, Resonant, Unstable}` with `classify_drift` on ρ-vs-unit-circle (:342-352)
and a single-authority `wire_code` (:333). It carries its OWN dense Laplacian `L = D − A`
(:287-297) — one of the multiple implementations the Hermetic audit flagged. Tests are
hand-oracle Verified-by-Math (K₃ energy = 4, P₃ Fiedler = 1, nilpotent DAG ρ = 0).

**`kernel/src/csr.rs` (849 lines).** Deterministic CSR graph; row-stochastic normalize;
LEFT-orientation `spmv` with fixed summation order; synchronous Jacobi personalized PageRank
with fixed iteration count (byte-reproducible, :228-264); `laplacian_spmv` with three
`LaplacianKind`s (Unnormalized / Normalized / RandomWalk, :279-359); in-kernel `recall_at_k` /
`precision_at_k` (:387-427). Two facts matter downstream: (a) the conservation tests
(:794-815) assert `L·1 = 0` — Σ of the output is zero for any topology — which is literally
Kirchhoff's current law expressed as a Laplacian row-sum invariant; (b) this branch already
carries the audit quick-win #19 fix (serialize→re-read boundary test, :516-550).

**`kernel/src/impedance.rs` (102 lines).** Read closely, as instructed. It is NOT a
Kirchhoff/circuit model. It is a queueing/transmission-line *lens*: `reflection_coefficient(ρ, k)
= min(ρ²·k, 0.99999)` where ρ = λ/μ (offered/service load) and k is burstiness (:19-28), plus a
two-pole `FlowGate {Admit, Backpressure}` on `ρ_eff < 1 − margin` (:32-47). Its own docstring
(:5-8) explicitly rejects literal impedance matching: "the plan is explicit that literal
'max-power-transfer impedance matching' misleads here." There is no composition law of any kind —
no R_eq, no series/parallel reduction, no node/loop equations. Grep confirms **zero production
callers**: the only reference outside the module is the `pub mod` line (`lib.rs:56`). It is
itself a stranded organ — which, under P2 ("a dead canonical primitive is a violation"), argues
*against* piling more circuit theory onto it, not for.

**`kernel/src/harmonic.rs` (163 lines).** Harmonic centrality H(v) = Σ 1/d(u,v) by BFS, ∞⁻¹ = 0
for disconnected pairs (:26-65), with a bit-parity test against the hermes agent-kernel
reference (:132-162). A healthy example of the P2 parity-pin discipline done right.

**`kernel/src/kalman.rs` (452 lines).** Full n-D predict/correct Kalman filter over the `Mat`
public API; Gauss-Jordan inverse fail-closed on singular S (:91-139); the innovation and the
dimensionless surprise ‖y‖/√tr(S) are surfaced (:227-237, E0 fix) for the self-eval loop; a
`set_q_scaler` self-adaptation knob (:283-288). The EMA-is-steady-state-KF equivalence is proven
by hand-derived Riccati oracle (:331-389).

**`kernel/src/noether.rs` (128 lines).** The conserved-quantity verifier: `step_preserves(x0,
update, invariant, steps, tol)` (:19) and `invariant_drift` (:42). Its non-vacuousness proof is
exactly the physics case relevant here: explicit Euler on a harmonic oscillator GAINS energy and
the checker catches it at tight tol (:87-97). This is the ready-made harness for any
"energy decreases along the flow" claim — currently never pointed at the engine's field
integrator.

**`kernel/src/event_log.rs` (746 lines).** Content-addressed hash-chain event log: in-kernel
SHA3-256 (:30), `event_id = H(prev ‖ actor ‖ seq ‖ payload)` as the idempotency key (:148),
`append` with local-first tip chaining where a Duplicate is a *structural* no-op (:293-312),
`commit_after_decide` running the kernel Law before persist (:339-361), and the HYDRA-G2
spectral drift gate `commit_after_decide_drift_gate` (:389-419) that rejects Unstable (ρ > 1)
mutations pre-persist. Notably, the Hermetic audit's #1 HIGH finding (infallible
`EventStore::insert`) is FIXED on this branch: `insert` returns `Result<(), StoreError>`
(:167-205), the Law pole and the Store pole are distinct types (`CommitError::{Rejected, Store}`,
:263-268), and a `FaultyStore` fault-injection suite proves no fake `Committed` on a failed
fsync (:442-468, :702-745). One correction to the task brief: `fold_transitions` does not live
here — it is in `order_machine.rs:140-153` (the deterministic reducer "the WS event bus replays
against"), consumed by `analytics.rs::reduce_anomalies` (:13-22). The lineage story is therefore:
content-addressed log (event_log) + per-subsystem fold reducers (order_machine, intake) +
`hydra::boot_verify` (:253) replaying the log after restart.

**`kernel/src/causal.rs` (2330 lines).** A full Pearl stack: back-door / front-door / IV
adjustments (:60, :139, :219), linear counterfactuals via abduction-action-prediction (:274),
a d-separation oracle (:309), the Shpitser-Pearl ID/IDC identifiability algorithm with hedge
witnesses (:589-960), and `empirical_identify` from raw sample rows. The decisive find for this
task: the test `empirical_converges_to_analytic_as_n_grows` (:2226-2256) **already encodes the
CLT** as a falsifiable gate — it derives the estimator's true asymptotic standard error
(`se_factor`, no magic constant) and asserts `error·√N < 6σ·se_factor` across N = 200…200,000.
The CLT is in the codebase; it is just imprisoned inside one test module.

**`kernel/src/online.rs` (627 lines).** The gradient-descent half of the "one ∇" story already
exists — and is *past* plain gradient descent: `LinearSGD` on the micrograd tape (:28),
deterministic `ScalarAdam` (:82), and the info-geometry upgrades `NaturalLogistic` (:177) and
`LinearGaussNatural` (:251) with Fisher preconditioning, a fail-soft manifold guard (:159), and
a scale-invariance test proving raw SGD degrades under input rescaling while the natural
gradient does not (:520-575). The monotone-descent test shape (`adam_descends_on_x2`, :359:
"loss must not increase") is the exact test shape the gradient-unification cluster generalizes.

**`kernel/src/rng.rs` (272 lines).** SplitMix64 → PCG64, pinned to published reference vectors
(:155-171), rejection-sampled `next_index`, `sample_categorical` (:117) — the seeded Monte-Carlo
substrate `causal.rs` already draws from. The module doc carries the audit's honest
reproducibility-scope doctrine (:20-29): integer stream is cross-platform bit-identical;
transcendental float paths are per-target only. The #19 serialize-reread test landed here too
(:203-231).

**`kernel/src/money.rs` (442 lines).** Checked integer arithmetic on minor units: currency-typed
`Money` with fail-closed cross-currency add (:57-87), half-up i128 tax (:94-112), and an
order-total mirror that degrades to `None` rather than fabricate a number (:219-241 — the audit's
#11 `unwrap_or(0)` pole-collapse is fixed here; tax overflow → `tax_total: None`, pinned by
:429-441). **Honest note: there is no circuit/gradient/flow resemblance in money, and that is by
design** — `motion.rs:159-168` ("money is NEVER a field channel", FE-09) makes the separation an
enforced invariant, not an omission. No cluster should be connected to money.rs.

**`engine/src/field_frame.rs` (343 lines).** The physics-UI operator equation
`M·U̇ = −ΓU̇ − c²·L·U + S`, integrated semi-implicitly (:10-14, :143-160), with a CFL-style
fail-closed stability assert (:59-72) and dt pinned to the kernel's `DT_STABLE` (:47-52 plus the
mirror-pin test :213-219 — audit finding #10 is fixed on this branch). Its Laplacian is a
5-point grid stencil with Neumann edges (:92-107). Two observations that matter: (a) this is a
**fourth** dowiz-side Laplacian implementation (dense `spectral.rs:287`, CSR `csr.rs:307`, grid
`field_frame.rs:92`, plus bebop's `field.rs:82` per the audit); (b) the stencil returns
`left+right+up+down − 4u`, i.e. **−(D−A)U — the NEGATIVE of the graph-convention Laplacian** the
kernel returns. The sign conventions across the seam are opposite and pinned by nothing. The
equilibrium test (:273-316) proves boundedness and convergence but does NOT assert energy
monotonicity — the exact property class `noether.rs` exists to check.

**`engine/src/motion.rs` (169 lines).** Critically-damped spring per property (ζ, ω derived from
tension/friction, :29-63), presets with falsifiable overshoot tests (ζ=1 monotone, ζ<1 bounces),
and `heat_kernel_delay = d/√α` (:87-92) — the stagger-by-graph-distance piece of the ONE-Laplacian
thesis.

**Additional files read because the reasoning demanded them.** `kernel/src/evals.rs`: the
metamorphic eval mint with a duplicate-rejection leakage gate (:806-818), `EvalRow::append_to`
(an execution-trace log with an appender and **no reader**), brier/ece/aurc point estimators
(no uncertainty attached), `RegressionGate` — RED on K consecutive smoothed degradations beyond
tol (:545-623), and `SelfAdaptator` (:676-764) — a propose→guard→apply loop over the Kalman
Q-scaler where `propose_step` never mutates, a Noether invariant accepts/rejects, and rejection
rolls back to the last accepted θ. `kernel/src/hydra.rs`: the closed-loop organism (`candidate_drift`
:56, `boot_verify` :253, durable `FileEventStore` :743). `kernel/src/ports/llm.rs` +
`llm-adapters/` (compose/dispatch/quirks/cache/transport) — the harness scaffold this branch's
parent built; `token_bucket.rs` the budget primitive it draws on.
`docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md` in full —
P2 correspondence rule (§1), RC-1 self-certification, findings #8 (Laplacian ≥4×, no parity),
#19, #21 (per-subsystem fold), #26 (M7 heal unimplemented). Live-tree check against finding #8:
`csr::laplacian_spmv` **now has a production caller** — `engine/src/bridge.rs:125`
(`VertexBridge::apply_field`, W20) — so the "zero callers" half of #8 is stale on this branch;
the "≥4 implementations, no cross parity, opposite sign conventions" half fully stands.
⚠ CORRECTED (decorrelated verification, `RESEARCH-VERIFICATION.md`): `apply_field` is real,
non-test, non-feature-gated wiring — but is only reached from tests today (`scene.rs` does not
call it yet); "production caller" means "wired public API," not "on a live runtime loop." Does
not change the finding-#8 assessment.
`docs/design/ARCHITECTURE.md` §0 (SCOPE RULE, M-series) — dev-time gates are canonical-repo
fences; zero-dep protocol boundary (M6) governs any "borrowed pattern" decision.

---

## §2 — Classification of the 10 clusters

| # | Cluster | Bucket | Justification (file:line or absence) |
|---|---------|--------|--------------------------------------|
| 1 | Gradient unification (field = −∇potential) | **STRONG** (narrowed) | Both halves already live: gradient flow on a loss (`online.rs:359` monotone-descent test) and diffusion `c²·L·U` (`field_frame.rs:143-160`); the missing computed objects are (a) a Dirichlet-energy Lyapunov check on the field integrator — `noether.rs:87` proves the checker catches exactly this bug class and is never pointed at `field_frame`; (b) the discrete gradient/incidence factorization `L = Bᵀ W B` as the parity-bind for the 4-way-fractured Laplacian (Hermetic #8) incl. the live sign-convention split (`field_frame.rs:103` = −(D−A) vs `csr.rs:316-325` = +(D−A)). The "SGD ≡ gravity, one operator" prose is rejected (V6): micrograd's ∇ is over a computation tape, not the state graph. |
| 2 | Central Limit Theorem | **STRONG** | Already partially implemented but imprisoned: `causal.rs:2226-2256` derives the true asymptotic SE and gates on `error·√N < 6σ` — test-only, not a library primitive. Absent everywhere it is needed: `evals.rs:331-430` (brier/ece/aurc, no CI), `csr.rs:387` recall@k (the stranded "recall@5 = 1.0 on 29 questions" claim carries no interval), `RegressionGate` tol hand-tuned (`evals.rs:557`). No Wilson/Clopper-Pearson/bootstrap anywhere (grep: 0 hits). |
| 3 | Apache Spark's 9 concepts | **BORROWED PATTERN, NOT DEPENDENCY** | One concept matches a real partial reimplementation: fault-tolerance-via-lineage ≡ content-addressed log + `fold_transitions` (`order_machine.rs:140`) + `hydra::boot_verify` (`hydra.rs:253`) — state is recomputed from the event chain, never copied. The pattern's completion is the shared replay/projection seam the audit already names (finding #21). The other 8 concepts (RDD, shuffle, partitioning, lazy DAG, Catalyst, DataFrames) are rejected: no distributed dataset, kernel is deliberately eager/deterministic; caching is already an independently-evidenced P0, Spark adds no new claim. Spark itself is never a dependency (M6 zero-dep). |
| 4 | Self-Harness (arXiv:2606.09498) | **STRONG** | The 3-stage loop exists in embryo at parameter scale: trace log = `EvalRow::append_to` (`evals.rs:487-498`, appender with zero readers — a named gap); proposal = `SelfAdaptator::propose_step` (never mutates, `evals.rs:707-748`); non-regressive acceptance = `RegressionGate` (`evals.rs:545`) + Noether guard + rollback-to-accepted (`evals.rs:739-744`). Extension = lift this discipline from one Kalman scalar to the actual LLM harness (`llm-adapters/` compose/quirks/dispatch). Hard precondition flagged in §3: without the audit's key_V independent verifier (finding #2, zero code hits), "validation" is self-certification (RC-2). |
| 5 | AIDE² recursive self-improvement (public/private score) | **WEAK / merges into #4** | Its one non-duplicated idea — a hidden score deciding survival — is already structurally present: the Noether guard accepts/rejects independently of the eval-loss the Adam tape minimizes (`evals.rs:714-744`). The Worker/Improver two-agent split adds no distinct falsifiable code change beyond cluster 4's loop; source is a self-reported vendor result. Inspiration only; not a separate blueprint candidate. |
| 6 | Quantum steering trust bound K ≥ H(B\|E) − H(B\|A) | **WEAK / rejected** | No code hit: no QKD, no conditional-entropy key-rate anywhere; mesh trust is signed capability, explicitly never reputation/scoring (ARCHITECTURE.md M12; event_log.rs CI guard "identity, NOT a score"). There is no computable H(B\|E) in this system — the connection cannot name a computed criterion (V6 fail). |
| 7 | Parallel vs Series circuits (KCL/KVL, R_eq) | **WEAK / rejected with precision** | KCL already lives in the codebase as the Laplacian conservation invariant `L·1 = 0` / `Σ out = 0` (`csr.rs:794-815`) — Kirchhoff at every node is the row-sum-zero property, tested. `impedance.rs` is queueing, not circuits, its docstring rejects the literal metaphor (:5-8), and it has zero callers (`lib.rs:56` only) — extending a stranded organ violates P2's dead-primitive rule. R_eq composition / effective resistance (= L⁺ quadratic form) is real mathematics compatible with the existing spectral stack, but no named consumer exists (closest: unimplemented M7 mesh-heal, audit #26). Recorded as inspiration; rejected as a blueprint candidate until a consumer names it. |
| 8 | Decision-prioritization frameworks (Pareto, RICE, Kano…) | **WEAK / rejected** | No code hit found; these are operator planning heuristics, not architecture. Nothing in kernel/engine computes or should compute a RICE score. V6 fail. |
| 9 | Claude Skills authoring guide | **WEAK / rejected as architecture** | Dev-tooling documentation for the agent layer (`.claude/` skills exist), zero contact with the energy-spectral-flow architecture. Possibly useful for skill hygiene; not a blueprint candidate here. |
| 10 | Speculative physics (Gödel CTCs, paradoxes, ER=EPR, volatility trading) | **WEAK / rejected** | The only tempting bridge — Novikov self-consistency ↔ replay idempotency (`event_log.rs:305` Duplicate no-op) — is word-play: the event log already has a precise, tested vocabulary and gains nothing from the metaphor. Volatility trading: no stochastic-finance code exists; money is checked integers (money.rs), and kelly_fraction tiering (ARCHITECTURE E15) is an already-decided separate item. No code hit for any member of this cluster. |

Tally: 3 STRONG (1, 2, 4), 1 BORROWED PATTERN (3), 6 WEAK/rejected (5, 6, 7, 8, 9, 10).

---

## §3 — Concrete sketches for STRONG and BORROWED-PATTERN findings

### 3.1 Gradient unification → (a) Dirichlet-energy Lyapunov gate, (b) incidence factorization as the Laplacian parity-bind

**What changes.**
(a) A thin `dirichlet_energy` helper computed THROUGH the existing operator — e.g.
`Csr::dirichlet_energy(&self, x) = ½·x·laplacian_spmv(x, Unnormalized)` — no new Laplacian
(P2: this is a function *of* the primitive, not a fifth implementation). Engine-side, a new test
(no integrator change) that treats one `FieldFrame::step` as the `update` and the discrete
damped-wave energy `E(U, U̇) = ½‖U̇‖² + ½c²·⟨U, L_grid U⟩ − ⟨S, U⟩` as the `invariant`, driven by
`noether::invariant_drift` — connecting two existing organs that have never met.
(b) A discrete gradient/divergence pair over an edge list (`grad: node-field → edge-flow`,
`div = gradᵀ`), plus a property test asserting `laplacian_spmv(x, Unnormalized) == div(W·grad(x))`
and `field_frame::laplacian(u) == −div(grad(u))` on a lattice edge list — pinning the sign
convention explicitly.

**New falsifiable claims.** (1) For Γ > 0 and fixed S, the field integrator's energy is
non-increasing per step within a stated scheme tolerance — a claim the current test suite
(`field_frame.rs:273-316`: bounded + converges) cannot express, and exactly the bug class
`noether.rs:87` proves detectable (explicit Euler gains energy; a wrong sign or an unstable
scheme would too). (2) Every Laplacian implementation equals div∘W∘grad on its own graph under
ONE sign convention — after which adding a fifth unbound `L` fails a test instead of shipping.

**Existing gap it connects to.** Hermetic finding #8 / RC-1 (the ONE-Laplacian claim
"design-true, code-false"), finding #22 (per-module `from_edges` hand-rolling), and the live,
currently-unpinned sign split found in this reading (`field_frame.rs:103` is −(D−A);
`csr.rs:316` is +(D−A) — a future caller crossing that seam without the pin gets anti-diffusion,
i.e. divergence).

**Audit precondition, stated plainly.** This extension is real, but building the energy gate on
`field_frame`'s own stencil while the kernel's L differs in sign would add one more hand-mirrored
seam (RC-4). The factorization parity test (b) — or at minimum a sign-pin test — is the
precondition for (a) to be trustworthy across the kernel↔engine boundary. Part (b) IS the
audit's own fix for #8, arrived at from the physics side.

**Open question for the blueprint pass.** Where does the canonical edge-list live (Csr, a plain
`&[(usize, usize, f64)]`, or `mat.rs`), and is bebop's `field.rs:82` bound in the same pass or by
a cross-repo fixture (the `TG_MIN_GAP_S` problem, finding #18, says cross-repo comment-pins
don't hold)?

**Explicit rejection within the cluster (V6).** No code will claim "SGD and heat flow are one
operator." The honest shared structure is the *test shape*: potential decreases monotonically
along the flow — already asserted for Adam (`online.rs:359-369`), newly asserted for the field.
Same theorem schema, deliberately different operators.

### 3.2 CLT → an uncertainty primitive for the eval/verification layer

**What changes.** Promote the CLT logic that already exists inside `causal.rs`'s test module
(:2240-2246 derives `se_factor` from first principles) into callable kernel primitives —
`mean_se(samples)`, `wilson_interval(k, n, z)` (exact-leaning binomial for small n), and
optionally a seeded bootstrap over `rng.rs` for non-analytic estimators. Location per CLAUDE.md
edit-don't-create bias: extend `evals.rs` (the consumer) unless the blueprint pass decides a
`stats.rs` module is warranted; either way ONE implementation, with the causal.rs test rewritten
to call it (P2: the √N law currently exists only as test-local code — a proto-duplicate waiting
to happen).

**New falsifiable claims.** (1) Every reported eval scalar carries an interval: "recall@5 = 1.0,
n = 29" becomes "recall@5 = 1.0, 95% Wilson lower bound ≈ 0.88" — the stranded
living-knowledge headline number becomes an honest, falsifiable statement (its own memory arc
already flags "NEXT bigger oracle"; the CI quantifies exactly why). (2) `RegressionGate` tol
derived from the metric's SE (red = degradation > z·SE) makes the gate's false-positive rate a
stated, testable quantity instead of a hand-tuned constant. (3) The causal empirical pipeline's
convergence gate becomes a reusable library call pinned by the same test.

**Existing gap it connects to.** `evals.rs` brier/ece/aurc are point estimates with no
uncertainty; `verify_retrieval.rs` retries on a bounded loop with no statistical stop rule; the
BRAIN-TOPOLOGY self-watch note ("no falsifiable statistic") names the repo-wide habit this
directly counteracts.

**Open question.** Which bound per surface: CLT/normal needs moderate n and finite variance;
n = 29 wants Wilson/Clopper-Pearson; EMA-smoothed streams violate iid (block bootstrap over
`rng.rs`, or a conservative Hoeffding bound?). The blueprint must pick per-surface and must keep
everything seeded/deterministic (P6) — bootstrap through `Rng`, never ambient entropy.

### 3.3 Self-Harness → the propose/validate/accept loop lifted to the LLM harness

**What changes.** Three pieces, all mapped to named code. (i) *Weakness mining*: a deterministic
reducer over the `EvalRow` JSONL (`evals.rs:487-498`) clustering failures by `(kind, params)` —
giving the appender-only trace log its first reader (the same half-pendulum shape as audit #27).
(ii) *Harness proposal*: proposals are DATA — a config struct over the enumerable harness knobs
in `llm-adapters` (retry policy in `quirks.rs`, prompt/template selection in `compose.rs`,
`TokenBucket` budget, model routing per `TaskClass` in `ports/llm.rs`) — mirroring
`SelfAdaptator::propose_step`'s propose-never-mutate discipline. (iii) *Non-regressive
acceptance*: a proposal is applied only if the frozen, mint-log-pinned `MetamorphicGenerator`
suite plus `RegressionGate` stay green; rejection rolls back to the last accepted config exactly
as `accepted_theta` does (`evals.rs:739-744`). The leakage gate (`evals.rs:806-818`) already
prevents the proposer from contaminating the eval set with duplicates.

**New falsifiable claim.** For every accepted harness-config change, pass rate on the frozen
suite is ≥ the previous accepted config's, measured and logged; a rejected change is provably
reverted. This is the paper's "non-regressive acceptance" as a testable kernel property rather
than a slogan.

**Existing gap it connects to.** The verifiable-cognition arc's "9/11 organs stranded" finding —
this wires evals → harness; the zero-reader JSONL; the fact that the branch this worktree forks
from is literally named `feat/harness-llm-backend` and built the scaffold with no tuning loop.

**Audit precondition, stated plainly (the biggest one in this document).** Hermetic RC-2/P7: if
the same session proposes AND validates, acceptance is self-certification — the exact failure
shape the audit found in the hermes done-gate (finding #2) and `FalseClaimMeter` (finding #7).
The paper's loop is only as honest as the independence of its validator. Until the audit's key_V
independent re-execution path exists (today: zero code hits outside docs), this loop must ship
**advisory-only** (signals, never auto-apply), consistent with the Markov-attractor
loop-signals precedent (advisory/fail-open). Auto-acceptance is gated on key_V, full stop.

**Open question.** Proposal space: an enumerable config lattice keeps determinism and lets
validation be exhaustive-ish; free-form LLM-written scaffold diffs (the paper's full form)
reintroduce self-certification at maximum strength and would additionally violate the
red-line self-mod scope that `SelfAdaptator`'s comments codify (`evals.rs:650-656`). The
blueprint must choose the lattice first and treat free-form as a separately-gated later stage,
if ever.

### 3.4 Spark lineage (borrowed pattern) → a shared replay/projection seam, natively

**What dowiz already has (the partial reimplementation that qualifies this as
borrowed-pattern).** Content-addressed hash-chained events (`event_log.rs:148`), structural
replay idempotency (:305), decide-before-persist (:339), typed durability faults (:167-205,
landed H1 fix), per-subsystem fold reducers (`order_machine.rs:140`, `analytics.rs:13-22`,
intake), and post-restart re-derivation (`hydra.rs:253 boot_verify`). That IS
fault-tolerance-via-lineage: lost projections are recomputed from the transformation log, never
copied. Spark is not a dependency and never will be (M6); the *lesson* being borrowed is that
lineage scales only when replay is first-class, not hand-rolled per consumer.

**What changes.** When (and only when) the third fold consumer appears — the audit's own
trigger, finding #21 — extract a `Projection` seam (`fn apply(&mut State, &MeshEvent) ->
Result<…>`) plus one `replay(log, projection)` driver, with snapshot-as-checkpoint
(`snapshot_root` already exists per the P6 audit) and replay-since-tip.

**New falsifiable claims.** (1) Replay determinism for arbitrary projections: state after crash
+ replay is byte-identical to uninterrupted state (extends the `boot_verify` shape; connects to
audit #4, the restore drill that has never run). (2) Snapshot/replay parity: every snapshot is
periodically re-derived from full replay and compared — because a snapshot is a *cache of
lineage*, and an unverified cache is the self-certification pattern wearing a performance hat.

**Explicitly rejected members of the cluster.** RDD/partitioning/shuffle (no distributed
dataset — mesh nodes exchange signed events, not partitions), lazy DAG + Catalyst (the kernel is
eager and deterministic by design; laziness trades auditability for optimization headroom this
scale doesn't need), DataFrames (no relational layer in kernel). Caching/persistence is real but
already the 4×-corroborated P0 on its own evidence — attributing it to Spark adds no new claim.

**Open question.** Whether `MeshEvent.payload` (opaque bytes) is the right replay unit for
projections that today fold typed `OrderStatus` streams — the blueprint must decide the
decode-at-the-seam vs typed-event-enum question before the trait is worth writing.

---

## §4 — Honest closing verdict

On the whole, this concept package is **mostly inspiration with a real, narrow architectural
core**. Three of ten clusters survive the V6 test, and even those three survive only in
*narrowed* form: gradient unification survives as "one energy functional, one factorized
operator, one Lyapunov test" — not as the poetic "one ∇ across four physics domains"; CLT
survives as "promote a √N gate the kernel already wrote into a library primitive and attach
intervals to every eval number" — the theorem is already in the tree at `causal.rs:2229`; and
Self-Harness survives because this repo independently built the same three-stage loop at
parameter scale before reading the paper — the extension is a scale-up of existing discipline,
with the blunt caveat that its acceptance stage is worthless without the independent verifier
the Hermetic audit already demanded for unrelated reasons. The single borrowed pattern (Spark
lineage) is legitimate precisely because dowiz already reimplemented two-thirds of it without
naming it. Everything else — steering bounds, circuit composition laws, prioritization
frameworks, skills-authoring, CTCs, volatility — either has no computed criterion in this
codebase (six clusters) or, in the instructive case of cluster 7, the codebase *already made the
honest rejection itself*: `impedance.rs`'s docstring refuses the literal circuit metaphor, and
KCL already exists as a tested Laplacian invariant with nothing left to add. The package's real
value is not the new physics vocabulary; it is that three of its items point at gaps the
Hermetic audit found from the other direction (fractured Laplacian, claims without intervals,
self-certified validation) — where two independent routes converge on the same fix, that fix is
probably load-bearing. The rest should be filed as reading, not roadmap.

*Files cited are live on `feat/spectral-energy-flow-evolution` as of 2026-07-16. No code was
written or edited; no commits were made.*
