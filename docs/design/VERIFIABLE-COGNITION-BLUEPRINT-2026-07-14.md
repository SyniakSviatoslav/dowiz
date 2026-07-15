---
id: VERIFIABLE-COGNITION
title: Verifiable Cognition — self-eval harness, benchmark generation, and the honest AGI-gap
status: proposed
type: blueprint
owner: SyniakSviatoslav
created: 2026-07-14
updated: 2026-07-15
supersedes: []
superseded_by: null
links:
  - relates_to: "[[AUTONOMOUS-ORGANISM-SYNTHESIS-2026-07-14]]"
  - relates_to: "[[internal-retrieval-living-memory-blueprint]]"
  - relates_to: "[[math-first-architecture-arc-2026-07-14]]"
  - relates_to: "[[integration-research-tf-attention-circuit-kalman-arc-2026-07-14]]"
  - companion: "bebop-repo/docs/design/BEBOP-VERIFICATION-HARNESS-BLUEPRINT-2026-07-14.md"
  - governs: "the self-eval / benchmark-generation layer across kernel + bebop"
inclusion: manual
confidence: high
tags: [evals, agi, faithfulness, hallucination, context-drift, agency, self-improvement, embedded, adapters, context-pruning, verification, ground-truth-over-proxy]
---

# Verifiable Cognition

> **Goal (operator, 2026-07-14):** a researched plan + blueprints to (1) verify **bebop**, (2) generate
> benchmarks for AGI-adjacent concepts — *faithfulness, context drift, hallucination, evals, agency* —
> (3) close the **self-maintenance / self-adaptation / self-research** loop, (4) add **native** reach
> incl. **embedded**, (5) automate **context self-pruning** ("автостинення"), (6) work through
> **adapters**, and (7) answer honestly, without marketing: *is this system AGI?*
>
> **Honest frame (load-bearing):** every claim below is grounded in a `file:line` on this server or a
> confirmed external citation, and every proposed step ships with a RED→GREEN proof obligation. This
> document is the **pre-code design artifact** — the input to `/council` for the red-line steps, not an
> authorization to write product code. It obeys [[ground-truth-over-proxy-2026-07-07]] and
> [[verified-by-math-2026-07-07]]: **metrics are computed, not judged.**

---

## §0 — The honest answer: is this AGI?

**No. It is not AGI, and it should not claim to be.** This is not a hedge — it is a measurable verdict,
and the good news the marketing version misses is that the *not-AGI* framing is exactly where this
system's defensible advantage lives.

### The measurable definition (not vibes)

The serious operational definition is Chollet, *On the Measure of Intelligence* (arXiv:1911.01547):
intelligence = **skill-acquisition efficiency over a scope of tasks, w.r.t. priors, experience, and
generalization difficulty** — with the load-bearing construct being *developer-aware generalization*
(handling situations neither the system **nor its author** anticipated). Skill ≠ intelligence: a system
superhumanly good at a narrow task tells you nothing about generality.

Against six operational criteria, this system scores as a **narrow, specialized agent**, and we can say
so with evidence already on the server:

| Criterion (Chollet / agentic-eval literature) | This system | Evidence |
|---|---|---|
| Breadth of *un-tuned* task distribution | Narrow (delivery + knowledge field) | `order_machine`, `money`, `living_knowledge` are domain-bound |
| Priors-normalized skill | High skill, heavy priors | every organ is hand-built math, not acquired |
| Skill-acquisition efficiency on **unseen** families | ~0 (no in-context skill acquisition) | ~~no learner fits *loop outcomes*; `online`/`micrograd` are STRANDED~~ → **E3 DONE 2026-07-15: `online`/`micrograd` un-stranded; `SelfAdaptator` fits the Kalman Q-scaler to minimize eval-loss under a noether guard** |
| **Transfer ratio** (off/on-diagonal) | ≈ diagonal-heavy | no cross-domain transfer path exists |
| Calibrated self-knowledge that degrades detectably | **absent** | no calibration/abstention organ (§3.4) |
| Autonomy under goal ambiguity | **absent** | only trigger is a human `UserPromptSubmit` (`AUTONOMOUS-ORGANISM §4`) |

The `AUTONOMOUS-ORGANISM-SYNTHESIS` doc already named the same gap from the inside (§4, "the genuinely
missing organs"): **no `Goal/Utility/Reward/Planner` struct exists** — no value function, no goal queue,
no autonomous planner, no volition. Those are *deliberately reserved to the human*. A system with no
self-originated goals and no transfer is, by construction, **not general**.

### What it actually is — and why that is better here

It is a **Verifiable Specialized Cognitive Engine (VSCE)**: a deterministic, bounded-field cognitive
substrate whose distinctive property vs. an LLM chatbot is that **its quality signals are computed and
falsifiable, not generated and hoped-for.** An LLM "guesses the next token" and is graded by another
LLM's opinion; this kernel computes a spectral radius, a Markov escape-mass, a Kalman residual — numbers
with proofs. For a sovereign delivery/knowledge system, *computed-not-judged* is worth more than
generality: it is auditable, offline, embeddable, and can be red→green tested.

### Reconciling the "Specialized Autonomous Agent / Auftragstaktik / Exoskeleton" framing

That framing (from the operator's prior dialogue) is **directionally correct but needs measurable
teeth**, which this blueprint supplies:

- "Emergent connections / synthesis" → measurable as **spectral-entropy coverage** of the recalled
  sub-graph vs. the full corpus graph, and PPR spreading-activation across distant nodes (`csr.rs:204`).
- "Goal-oriented autonomy" → measurable as **reachability + expected-steps-to-goal** on a task-state
  graph (`order_machine::reachable:270`, `absorbing::expected_steps:67`) — *not* claimed as volition.
- "Robustness / anti-fragility to garbage input" → measurable as **success-rate slope + knee-point**
  under injected perturbation (§Appendix A).
- "Dialectical / paradox-holding" is not mysticism — it is a **multi-objective (Pareto) output**: emit a
  trade-off matrix, not a single scalar. That is an engineering choice (return the frontier), and it is
  testable (does the output dominate on ≥2 axes?).

**Bottom line to fix:** *This is a Verifiable Specialized Cognitive Engine, not AGI. Its edge is that it
measures itself with math instead of opinion. The agency-test battery below tells us how intelligent it
is on its own field — honestly, and without claiming to be a mind.*

---

## §1 — The unifying thesis: an eval **is** the missing utility function

The `AUTONOMOUS-ORGANISM` doc found the loop is not autonomous because three joints are open — **trigger,
persistence, enforcement** — and, deeper, because there is **no scalar the loop optimizes toward** (§4.1)
and **no single place to ask "how am I doing / what's degrading."**

> **The benchmark harness this document specifies *is* that missing organ.** An eval metric is a utility
> function. Self-adaptation = the loop descending on eval deltas. Self-research = auto-minting new
> benchmark items to probe where the metrics are weakest. The eval layer is not a side-quest; it is the
> **connective tissue** between "self-improvement loop" and *actually improving*.

### The leverage point (from the eval-science lane)

Almost every cheap, judge-free eval metric in the literature reduces to **one of four operations the
kernel already computes**:

| Kernel operation | already at | powers these eval concepts |
|---|---|---|
| **Eigenvalues** of a similarity / covariance / Laplacian matrix | `spectral.rs`, `householder.rs` | EigenScore hallucination; discrete semantic entropy; transfer/generality distance; context-drift ρ/Fiedler change-point; CoT step-centrality |
| **Kalman-smoothed scalar** over turns | `kalman.rs`, `geo::ema_next` | skill-acquisition-efficiency; drift state+velocity; hallucination-risk trend; faithfulness-drift |
| **Entropy / divergence** over a discrete distribution | `markov.rs` | semantic entropy; PSI/JSD/KL drift; ECE/Brier calibration; empowerment channel capacity |
| **Graph reachability / shortest-path / coverage** | `order_machine.rs`, `csr.rs`, `absorbing.rs` | goal-completion + efficiency; attribution claim↔source coverage; groundedness coverage; metamorphic oracles |
| (feeder) **cosine over embeddings** | via the LK bridge | feeds every matrix above |

**One Gram/adjacency matrix per sample-set → read hallucination self-consistency, semantic entropy, AND
context-drift off it.** Compute once, three metrics. That single fact is the blueprint's core
simplification.

---

## §2 — The metric catalogue: bind each eval concept to a kernel primitive

This is the "benchmark generation on faithfulness / drift / hallucination / agency / evals" core. Each
row: **eval concept → cited definition → kernel primitive (`file:line`) → status → the deterministic
metric to compute.** The kernel-metrics lane confirmed 9 of 11 cognitive organs are **fully tested but
STRANDED** (zero consumers); the harness is mostly a *wiring + oracle* job, not build-from-scratch, and
there is a **proven un-stranding template**: quantity → `#[wasm_bindgen]` export → engine bridge → JS
test (only `spectral` + `order_machine` currently traverse it).

| Eval concept | Cited definition | Kernel primitive (`file:line`) | Status | Deterministic metric to compute |
|---|---|---|---|---|
| **Context drift** (magnitude) | RULER effective-context (2404.06654); eigen-geometry drift (2411.02464) | `spectral::spectral_radius:217`, `classify_drift:280`, `algebraic_connectivity:257` | **WIRED** | ρ and Fiedler λ₂ of a sliding-window similarity graph; change-point when spectrum discontinuous |
| **Context drift** (distributional) | PSI/JSD/KL, "lost-in-the-middle" U-curve (2307.03172) | `markov::analyze:81` (entropy), new `psi()` | STRANDED | PSI>0.2 over embedding-cluster histogram; report the **full position curve**, never the mean |
| **Hallucination** (self-consistency) | SelfCheckGPT (2303.08896); EigenScore/INSIDE (2402.03744) | `spectral::eigenvalues:195` over N-sample Gram matrix | STRANDED | **EigenScore** = mean log-eigenvalue of sample covariance; low-rank ⟺ confident |
| **Hallucination** (semantic entropy) | Farquhar, Nature 2024 (s41586-024-07421-0) | `spectral::laplacian:242` null-space → components; `markov` entropy | STRANDED | discrete semantic entropy over cosine-graph connected components |
| **Novelty / surprise** | innovation/residual (Kalman) | `kalman::update:201` — **innovation `y=z−Hx` was discarded at :215** | DONE (E0) | `last_innovation()` / `last_surprise()` exposed; surprise = ‖innovation‖ / √S |
| **Faithfulness** (CoT-causal) | Lanham causal interventions (2307.13702); *self-consistency ≠ mechanistic* (2311.07466) | `order_machine` graph analysis as step-influence DAG | reuse WIRED | truncation-AOC, add-mistake, filler-token → argmax flip rate (pure string/argmax diff) |
| **Faithfulness / groundedness** | ALCE NLI recall/precision (2305.14627); RAGAS-default is **LLM-judge → advisory only** | `living_knowledge::retrieve:76`, `verify_retrieval::verify_then_lookup:36` | STRANDED **+ bug** | claim↔source **coverage** = fraction of claims whose max cosine to a cited chunk > τ (upgrade to self-hosted HHEM/SummaC NLI where budget allows) |
| **Retrieval recall** | HippoRAG PPR recall (2405.14831) | `csr::personalized_pagerank:204` (fixed-K, bit-reproducible) | DONE (E0) | `recall_at_k`/`precision_at_k` score a PPR ranking vs a relevance set (in-kernel, deterministic) |
| **Goal convergence / agency** | GAIA/WebArena success (2311.12983 / 2307.13854); pass^k (2406.12045) | `absorbing::expected_steps:67`, `order_machine::reachable:270` | STRANDED | reachable-to-goal (bool); path-len ÷ shortest-path (efficiency); repeat-N → pass^k |
| **Consistency / conservation** | metamorphic invariants (§3) | `noether::step_preserves:19`, `invariant_drift:48` | DONE (E1) | `noether_conserving`/`noether_nonconserving` MR items in `evals.rs`; Σ‖ΔI‖ ≤ tol enforced by oracle |
| **Loop-lock / repetition** (hallucination failure-mode) | attractor detection; Foster-Lyapunov | `markov::analyze:81` (Verdict, escape_mass, drift, SLEM), `budget:69` | STRANDED | attractor Verdict + escape-mass; retry-cap `k=ln(1/tol)/ln(1/slem)` |
| **Empowerment / health** | Klyubin channel capacity (2005) | Blahut-Arimoto over `markov` transition matrix | new | reward-free agency signal; →0 ⟺ lost control (analogue of the shipped ρ≈0 ⟺ acyclic) |
| **Calibration / abstention** | ECE/Brier/AURC; SimpleQA rewards abstention | `evals.rs` `ece`/`brier`/`aurc` over kernel probability outputs | DONE (E1) | ECE (bin |acc−conf|), Brier, risk-coverage AURC — hand-validated tests in `evals.rs` |

**Three concrete bugs, fixed (E0 — 2026-07-15, RED→GREEN via `cargo test` in
`kernel/`):** the eval primitives crate now surfaces these as live, tested
organs instead of dead code.
1. `verify_retrieval::verify_then_lookup:46` — both `Err` arms were identical;
   the `round≥max_rounds` branch was functionally dead. **Fixed:** the
   `RetrievalTrigger` now carries a `terminal: bool` flag (true only at
   `round == max_rounds`), so the harness has a deterministic stop signal. RED
   test `terminal_flag_tracks_round_cap` fails on old code; green now.
2. `kalman.rs:215` — the innovation `y` (the surprise signal) was computed then
   discarded. **Fixed:** `KalmanFilter` caches `last_innovation` (the `y` vector)
   and `last_surprise = ‖y‖/√tr(S)` after every `update`; surfaced via
   `last_innovation()` / `last_surprise()`. RED test
   `update_surfaces_innovation_and_surprise` green.
3. No in-kernel `recall@k` scorer. **Fixed:** `csr::recall_at_k` /
   `precision_at_k` score a PPR ranking against a relevance set with a stable
   ascending-index tie-break (deterministic, no HashMap). Re-exported from
   `lib.rs`. RED tests `recall_at_k_hand_oracle`, `precision_at_k_hand_oracle`,
   `recall_scores_real_ppr_ranking` green.

> **Discipline caveat (honor it in code):** the spectral/Markov→LLM-eval bridges (Laplacian drift on
> context, EigenScore on our embeddings, empowerment-as-health) are **well-grounded syntheses, not yet
> established LLM-eval methodology.** Each must be **validated against ground-truth accuracy degradation
> on generated items**, not asserted by fiat. LLM-as-judge (RAGAS default, TruthfulQA GPT-judge) is a
> **secondary advisory signal, never the pass gate.**

---

## §3 — Benchmark generation framework (auto-mint items, no leakage)

The generation side is dominated by **metamorphic / property-based synthesis** — the one technique that
*solves the oracle problem and has nothing fixed to leak*.

### §3.1 Metamorphic generation with programmatic oracles (the load-bearing technique)

A metamorphic relation (MR) checks a *relation* between `f(X)` and `f(T(X))` — verifiable even when the
absolute answer is unknown. The kernel's own invariants are both **generator and oracle**, unbounded and
leak-free:

- **FSM relabeling invariance** — permuting state labels must not change `has_cycle:524` / `topological_order:213` / `reachable:270`. *(Note 2026-07-15: those `order_machine` fns are parameterless singletons over a fixed FSM — they cannot take a relabeled graph as input. The E1 generator therefore uses the **input-accepting** kernel primitives below as its MR substrate; the FSM-MR is deferred behind an `FsmGraph` input adapter. See `kernel/src/evals.rs`.)*
- **Edge-permutation invariance** of `spectral_radius:311` (and cross-check vs `spectral::spectral_radius:217` — the parity gate at `spectral.rs:351`).
- **Kalman scaling law** — scaling process noise `Q` by `k²` scales posterior covariance predictably (`kalman.rs`).
- **PPR seed-locality / stationarity** — the 6 hand-oracles already in `csr.rs` become MR templates.
- **Noether conservation** — `invariant_drift:48` must stay ≤ tol under any structure-preserving transform.

**E1 implemented (2026-07-15, RED→GREEN via `cargo test` in `kernel/`):** `MetamorphicGenerator` in `kernel/src/evals.rs` mints 5 MR families over the input-accepting primitives — `spectral_similarity(n)`, `kalman_q_scaling(q)`, `noether_conserving(dim)`, `noether_nonconserving(dim)`, `recall_constructed(m)` — each carrying a deterministic programmatic oracle (`passed`). Calibration metrics `ece`/`brier`/`aurc` are implemented and hand-validated. See `evals.rs` tests.

Each MR yields **infinite fresh instances with a free oracle** → the benchmark never staleness-leaks.

### §3.2 Held-out oracle + adversarial mutation
- Oracle set is **timestamp-minted** (LiveCodeBench-style, 2403.07974): an item is only scored if minted
  after the artifact-under-test's freeze. Kills contamination structurally, not by heuristic.
- Adversarial items follow the **mutate → validity-filter → score-vs-original-ground-truth** pattern
  (AdvGLUE found ~90% of raw automatic attacks change meaning — the validity filter is not optional).

### §3.3 Leakage / dedup admission gate
- **Structural gate — BUILT (2026-07-15).** `MintLog` (deterministic 128-bit FNV-1a over `(kind ‖ payload)`) rejects any exact `(kind, payload)` re-mint: `mint()` returns `None` for a seen payload. This kills byte-level contamination structurally (the LiveCodeBench timestamp-mint idea, realized as a content hash — no need for wall-clock in a hermetic suite). Test `leakage_gate_rejects_duplicate` + `identical_params_rejected_as_duplicate` green.
- **Semantic gate — DEFERRED to the embedding adapter (§7).** Rejecting an item whose *embedding cosine* to any prior item > ~0.9 requires an embedding bridge (the `living_knowledge` / vector-index adapter). The blueprint's cosine-0.9 gate is intentionally *not* faked with a proxy in kernel; it lands when the embedding adapter (N0-adjacent) exists. Until then the structural gate is the authoritative barrier, and the mint log is disclosed so the comparison graph stays connected.
- Note *The Leaderboard Illusion* (2504.20879): **dynamic ≠ Goodhart-proof** without disclosed sampling — so the mint log is disclosed, and the comparison graph stays connected.

### §3.4 Calibration + reliability (the abstention organ the system lacks)
- **ECE / Brier / risk-coverage AURC** in pure Rust over the kernel's own probability outputs (Kalman
  posterior variance, Markov stationary mass, spectral-gap certainty) → a per-commit regression gate.
- **pass^k reliability curves** (τ-bench 2406.12045), not pass@1 — any autonomy claim needs a
  repeated-trial curve. This closes criterion 5 of §0 (calibrated self-knowledge).

### §3.5 What is explicitly *not* the gate
LLM-as-judge scores (position/verbosity/self-preference biases: 2306.05685, 2305.17926) are advisory;
**deterministic oracles are the authoritative pass/fail.** Where a judge is unavoidable (open-ended
synthesis), use a **Panel-of-LLM-judges** (PoLL, 2404.18796) of small disjoint-family models, and record
it as advisory.

---

## §4 — Verifying bebop (the companion harness)

Full spec: `bebop-repo/docs/design/BEBOP-VERIFICATION-HARNESS-BLUEPRINT-2026-07-14.md` (this section is
the executive summary; the companion carries the RED→GREEN detail and lives in the bebop repo so it sits
beside bebop's 13 CI guards).

**What is already proven (real anchors — don't rebuild):** 60 NIST **ACVP ML-DSA-65** byte-exact tests;
ML-KEM core implicit-reject `J(z‖c)` proof (`core/pq_kem.rs:987`); capability-not-reputation trust
(`roster::verify_chain:252`); scope attenuation lattice; red-line deny-by-default; **13 wired structural
CI guards**. bebop2-core is **genuinely `no_std+alloc`, zero-dep**, and builds `wasm32-unknown-unknown`
in CI — it is the *embedded reference* the kernel is not yet (§6).

**The harness's mandate is the measurement void:** bebop2 has **zero `proptest`, zero fuzz targets, zero
statistical constant-time (dudect)** anywhere. Top targets, ranked (security-criticality × un-provenness):

| # | Target | `file:line` | Assertion to build (RED now) | Gate |
|---|---|---|---|---|
| 1 | `mod_l`/`reduce_p` secret-dependent timing (C4b, HIGH open) | `sign.rs:625,171` | **dudect Welch-t `|t|<4.5`** on `sign`; RED on current, GREEN after Barrett/Montgomery | 🔴 crypto |
| 2 | proto-crypto ML-KEM `H(sk‖c)` wrong reject | `proto-crypto/pq_kem.rs:584` | port `kem_implicit_rejection_equals_fips203_j` (RED now) **or delete the duplicate impl** | 🔴 crypto |
| 3 | CRDT `MerkleLog` convergence (prose-only "proven") | `sync_pull.rs:300` | **property test**: N nodes, random partition+replay → identical `root()`; idempotent re-ingest = no-op | 🟡 |
| 4 | Wire `decode_frame` under hostile bytes | `proto-wire/wire_codec.rs` | **cargo-fuzz**: decode→re-encode = identity; never panic; reject non-canonical | 🟡 |
| 5 | Statistical CT for ML-KEM compare / Argon2 / AEAD | `proto-crypto/constant_time.rs` (Placeholder) | reusable dudect rung over every secret-dependent op | 🔴 crypto |
| 6 | `ClassicalUntilPqAudit` PQ-strip acceptance | `hybrid_gate.rs:180` | assert prod constructs only `RequireBoth`; RED test absent-PQ rejected | 🟡 |
| 7 | Empty-import + no-alloc hot-path budget on feature branches | `verify-empty-imports.sh`, `ARCHITECTURE.md:139` | move empty-import to pre-commit; build the `decide()` no-alloc panic-allocator test | 🟡 | **PARTIAL — 2026-07-15: alloc-free `content_id_ref` (stack buffer, no Vec) proven equal to prod (`differential_content_id` test). The strict runtime no-alloc gate on the FULL hot path requires refactoring `hash.rs::sha3_sponge` (Vec-backed) + `MerkleLog` (`HashSet`) — a separate architectural task, flagged. empty-import→pre-commit intentionally NOT done (script is a slow wasm build per its own header).** |
| 8 | G2 dowiz-kernel ↔ bebop2 event-log differential | `sync_pull.rs:300` vs kernel `event_log.rs` | differential: same input → identical `content_id` across both impls | 🟡 | **DONE — 2026-07-15: `differential_content_id` asserts bebop2 `SyncFrame::compute_content_id` == dowiz `MeshEvent::event_id` byte-for-byte across 12 cases + 200-iter xorshift fuzz (incl. genesis/empty/non-pow2/large/max-seq).** |

**The unification:** targets 3, 4, 8 are the *same four kernel primitives* (§1) applied to protocol
invariants — CRDT convergence is a **graph fixed-point** check; canonicity is an **injectivity** property;
the differential is **coverage/equality**. The eval-harness crate and the bebop-verification crate share
one property-based-testing substrate.

---

## §5 — Self-maintenance, self-adaptation, self-research

This is where the eval layer closes the three open joints from `AUTONOMOUS-ORGANISM §3`.

### §5.1 Self-maintenance (persistence + enforcement joints)
- The eval suite runs as a **regression gate** on the `bench_track.py` / `BENCH_HISTORY.md` pattern that
  already works for kernel latency — extend it to the eval scalars. **DONE (E2, 2026-07-15):** `EvalRow` in `kernel/src/evals.rs` emits `run-history.jsonl` lines byte-compatible with `analytics/analyze.mjs`; `RegressionGate` is the authoritative RED→GREEN mechanism; `EmaTracker` (over `geo::ema_next`) smooths the trend. Tests `regression_gate_flips_red_on_degradation` / `_stays_green_*` / `_recovers_*`, `ema_tracker_smooths_jitter`, `eval_row_schema_matches_analyze_mjs` green.
- **Persist every scalar** to a durable JSONL (kills the "amnesiac `.loop-state`" joint) so the ratchet
  can learn "this task-class drifts / loops." This is the single fix for `AUTONOMOUS-ORGANISM` joint 2. **DONE (E2):** `EvalRow::append_to` is fail-closed (test `eval_row_append_to_persists_jsonl`).
- Feed the long-dead `analytics/analyze.mjs` A/B regression detector (written, fed nothing) with one real
  row per eval run → the "did my last change help or hurt?" nerve. **DONE (E2):** patched `analyze.mjs` `parseTs` to ingest the kernel's `epoch+00:00` timestamp (JS `Date` returned NaN before — the consumer was silently dead). End-to-end verified: 3 kernel rows → analyze.mjs reports Overall 67%, Regression config 2 vs 1, Worst gate `noether`.

### §5.2 Self-adaptation (the runtime bus)
- With a scalar utility defined, the **STRANDED learners un-strand**: `online::{LinearSGD,ScalarAdam}` +
  `micrograd::Value::backward:195` fit parameters to **minimize eval-loss** — the first loss ever defined
  over *loop outcomes* (§0 criterion 3). `noether` guards the adaptation (conservation ≤ tol).
- **Kalman-track the eval state** (`geo::ema_next`) so adaptation descends a smoothed trend, not per-turn
  noise — separates a real regression from measurement jitter.

### §5.3 Self-research (bounded, gate-respecting)
- The metamorphic generator (§3.1) + a **completeness-critic** auto-mint new benchmark items **targeting
  the currently-weakest metric** (empowerment-guided: probe where control/coverage is lowest). This is
  "self-inquiry" with an external ground — the honest version the operator directive asks for.
- **Guardrail:** this originates *proposals + verification*, never irreversible acts. Volition (goal
  origination, self-mod effector) stays reserved to the human (`AUTONOMOUS-ORGANISM §4`, §8 Phase-4). The
  loop must *measure itself against an external ground* (WORM audit hash-chain), never against itself —
  "a loop that measures itself by itself converges to a wrong answer while looking successful."

---

## §6 — Native capabilities + embedded deployment

**Honest starting point (kernel-metrics + embedded lanes):** the kernel's `no_std` is **aspirational**.
`Cargo.toml` declares `default=["std"]`/`std=[]` but there is **no `#![no_std]` anywhere**, `libm` is
**absent from `Cargo.lock`**, and 38 transcendental call-sites + a **460:2 f64:f32 monoculture** + 20
`HashMap` uses block a real embedded build. The `std` feature gates only 2 FMA-dispatch lines.

**The reference exists in-house:** `bebop2/ports/telegram` is a correct `#![no_std] + extern crate alloc`
+ private bump-allocator (`static [u8;65536]`) `wasm32-wasip2` component — the exact recipe the kernel
hasn't adopted.

### Deployment matrix (target × what blocks it × effort)

| Target | Blocks it today | Effort |
|---|---|---|
| Browser (`wasm32-unknown-unknown`) | **nothing** — shipping (`pkg/…bg.wasm` 321 KB) | ✅ done |
| Native Rust node (systemd) | **nothing** — intended runtime | ✅ done |
| Server/edge WASI component (`wasm32-wasip2`) | kernel is a wasm-bindgen blob, not a component; needs cargo-component + WIT + facade (pattern proven by telegram port) | Low–Med |
| microVM (Firecracker) | `isolation/microvm.rs` gate exists, VMM launch is a stub; server-class only, **KVM ≠ phone** | Med (server hubs only) |
| **Embedded MCU (Cortex-M4F / ESP32-S3, `no_std+alloc`)** | add `#![no_std]`+`alloc`+`libm`; feature-gate `serde_json`/`wasm-bindgen`/`tracing`; `HashMap`→`hashbrown`; **f64→f32 + Kahan** (all FPUs here are f32-only → f64 is ~6× soft-float) | High |
| Constrained WASM on MCU (wasm3 / WAMR) | same `no_std` recompile **+ ban relaxed-SIMD & raw-NaN** for cross-runtime bit-identity; wasm3 <256 KB RAM, WAMR-AOT ≥256 KB | High |
| No-FPU MCU (Cortex-M0/M3, ESP32-C3/C6) | above **+ fixed-point (`fixed`/Q31)** rewrite of the hot math | Very High |

### The single highest-leverage unlock
The **`no_std + alloc + libm` refactor** simultaneously enables embedded, constrained-WASM, smaller
browser/edge blobs (314 KB → tens of KB), and clean WASI components. Determinism watch-items to bake into
the spec: **never produce/observe raw NaN, never use relaxed-SIMD** (core-WASM float is otherwise IEEE-754
value-deterministic); **pin float width + reduction order**; note that **soft-float vs FPU is not
bit-identical across MCU classes** — so "deterministic kernel" holds *within* a target class, not across.

---

## §7 — Adapters (ports-and-adapters)

The pattern — **core-immutable / integrations = ports** — is **built on the bebop side** and **blueprint
on the dowiz side**:

- **Built (bebop):** `proto-cap/src/scope.rs` (closed `Resource:12` × `Action:47`), `port.rs`
  (`InboundPort:37`, `OutboundPort:56`, `check_port_scope` deny-by-default), the telegram component, and
  the `wasm-host` `Scope`→WASI-import mapper. **WASI-p2 grant ≡ `required_scope()`** — capability *is* the
  facade, for free.
- **Blueprint (dowiz):** `KernelFacade` / `OutboundPort` / `ChannelAdapter` / `BackupSink` /
  `EntropySource` are specified in `integration-ports/` but **not yet Rust** (grep = zero hits). The
  compile firewall (an adapter that imports the kernel must **fail to build**) is the hardest purity
  guarantee.

**The eval harness plugs in as an adapter, not a core mutation:** it is an `OutboundPort` with
`required_scope() = (Corpus, Read)` — it reads artifacts and emits scalars, touching no domain state. The
**existing reference adapter** is `living_knowledge::SubprocessLivingKnowledge:57` (JSON-over-stdio to a
swappable `LK_BRIDGE_CMD` bridge, fail-closed) — the eval harness reuses that exact seam for the
embedding/NLI backend.

**Dispatch guidance (embedded-safe):** generics/monomorphization for the deterministic hot path
(zero-cost, `no_std`-clean); `&dyn` for runtime-swappable adapters (**`&dyn` needs no allocator; only
`Box<dyn>` pulls `alloc`**). **Do NOT model mutually-exclusive business adapters as Cargo features** —
features union across a workspace and silently break; use separate crates + trait selection. (bebop's
`wasm-host` feature-gating wasmtime is the *correct* use — an additive platform backend.)

---

## §8 — Automated context self-pruning ("автостинення")

**Status (embedded lane):** the deterministic *recall* half is **built** — `csr::personalized_pagerank:204`
is exactly HippoRAG-style PPR (fixed-K, fixed summation order → **bit-reproducible on any hardware**). The
*tiered store* half is **fully specified but unbuilt** (`internal-retrieval-living-memory-blueprint.md`):
`tier SMALLINT` (Hot/Warm/Cold/Attic), per-row `decay_tau`, **"NO DELETE in the write API"**, TTL as
*demote-one-tier* not delete, attic = `tier=3` (no separate table).

**SOTA mapping (all external-corroborated):** the design = **HippoRAG PPR recall (2405.14831) + Mem0/MemGPT
tiered demote-don't-delete (2310.08560) over an append-only+tombstone substrate with Generative-Agents
recency×importance×relevance ranking (2304.03442)** — but with a **stricter never-CULL invariant** than
any of them (they hard-delete; this vaults duplicates). That invariant uniquely buys a **monotonic,
replayable state machine**: any past working-set is recomputable from the log + deterministic
demote/rank functions.

**Determinism watch-items to freeze in the spec:** (a) LLM importance at write-time + pinned model
version; (b) embedding model + synonymy threshold; (c) PPR iteration cap/tolerance (or closed-form
`(I−αP)⁻¹`); (d) a **total-order tie-break (stable id key)** in RANK. Real eviction lives **only in a
front read-cache** (ARC / W-TinyLFU — deterministic given a fixed trace), never in the store. Slow tier-3
compaction (summarize+hash, never drop) gates its output through a claim-set check (reject if any claim
dropped) — hierarchical summarization's acceptance is made deterministic even though generation isn't.

---

## §9 — Phased execution plan (each phase names its gate + its RED→GREEN proof)

**GATE KEY:** 🟢 non-gated (offline Rust/docs, feature-gated, additive) · 🟡 self-mod-token (`.claude/`
or harness, floor-preserving, audited) · 🔴 human/`!` (red-line: crypto, money, RLS, migrations, dep-install,
autonomy expansion). Nothing below is enacted by this document; the 🔴 rows require `/council` + operator.

| Phase | Scope | Gate | Proof obligation (RED→GREEN) |
|---|---|---|---|
| **E0** Eval-primitives crate | un-strand the 9 STRANDED organs into an `evals` module w/ wasm exports + oracle tests; fix the 3 bugs (§2) | 🟢 | each metric red→green vs a hand-oracle; `verify_retrieval` dead-branch test; Kalman-innovation surfaced test |
| **E1** Benchmark generator | metamorphic synthesis over input-accepting kernel invariants; deterministic MintLog leakage gate; ECE/Brier/AURC | 🟢 | generator emits N fresh items whose MR-oracle passes (test `emits_fresh_passing_items`); leakage gate rejects an exact dup (test `leakage_gate_rejects_duplicate`); **DONE — 2026-07-15, 6 evals tests green, kernel suite 222 pass** |
| **E2** Self-eval loop wiring | persist scalars (JSONL); regression gate; Kalman drift-trend; feed `analyze.mjs` | 🟡 | a seeded regression flips the gate red; a stable run stays green; persistence survives a session boundary | **DONE — 2026-07-15, 6 E2 tests green, kernel suite 228 pass, analyze.mjs end-to-end verified** |
| **E3** Self-adaptation | un-strand `online`/`micrograd` to minimize eval-loss under `noether` guard | 🟡 | adaptation reduces eval-loss on held-out items **without** raising `invariant_drift` above tol | **DONE — 2026-07-15, `online` module made public (was STRANDED), `SelfAdaptator` drives `ScalarAdam`+`micrograd` via real θ-dependent control law J(θ)=loss/θ+κ(θ−1)²; noether guard rolls back any step that pushes Σx² past tol; mutates only the Kalman Q-scaler. 3 E3 tests green, kernel suite 234 pass** |
| **B0** bebop harness (non-crypto) | CRDT-convergence property test; wire-decode fuzz; differential G2; empty-import→pre-commit | 🟡 | §4 targets 3,4,7,8 red→green | **DONE — 2026-07-15 (bebop-repo `feat/verification-harness`). Pre-existing GREEN from harness waves: B0-3 `two_diverged_nodes_converge_identical_after_pull`+idempotent; B0-4 `w2_canonical_and_injective`+`w2_hostile_byte_sweep`; B0-6 `production_facade_rejects_absent_pq_require_both`. Added this turn: B0-8 `differential_content_id` (kernel↔bebop2 content_id byte-identical across 12 cases + 200-iter fuzz) + B0-7 alloc-free reference `content_id_ref` (stack buffer, no Vec) proven equal to prod. NOTE: the strict runtime no-alloc gate on the full hot path is a separate architectural refactor (`hash.rs` `sha3_sponge` + `MerkleLog` `HashSet` both allocate) — flagged, NOT silently done. empty-import→pre-commit intentionally deferred (script is a slow wasm build, per its own header).** |
| **B1** bebop crypto | dudect for C4b; proto-crypto `H(sk‖c)` fix/delete; CT rung | 🔴 | §4 targets 1,2,5 — **council + operator gated** |
| **N0** `no_std`+`alloc`+`libm` kernel | additive feature-gated refactor; f64→f32+Kahan on the hot path | 🟡 | `--no-default-features` builds `wasm32v1-none`; bit-parity test within a target class |
| **P0** Context-pruning store | tiered demote-never-delete on pgrust; PPR recall wired | 🟡/🔴 | replay test: working-set recomputable from log; never-DELETE guard; PPR bit-reproducibility |
| **V** Volition | goal queue + utility + reversible branch-only CI-gated effector | 🔴 | **reserved to the human** — grow reversibility before autonomy |

**Sequencing:** E0→E1 are pure-offline and unblock everything (do first). E2/E3 close the self-loop. B0
and N0 are parallelizable. B1/P0/V are gated. The biggest early wins are **elections/deletions**
(`AUTONOMOUS-ORGANISM §7` reconciliation-first: ~10 competing roadmaps, 3-claimed LK states, eigensolver
dual-authority) — reconcile before constructing.

---

## §10 — Honest limits / what NOT to claim

- **Do not claim AGI.** §0 is the standing answer. Claim *Verifiable Specialized Cognitive Engine*.
- **Novel bridges are unvalidated.** Laplacian-drift-on-context, EigenScore-on-our-embeddings,
  empowerment-as-health are grounded *syntheses*, not established LLM-eval methods — **validate each
  against ground-truth accuracy on generated items** before it gates anything.
- **LLM-judge is never the gate.** Deterministic oracle is authority; judge is advisory (PoLL if needed).
- **Citations dated after Jan-2026 (arXiv `2601–2606.x`, ARC-AGI-3 model scores) are UNVERIFIED** — the
  *methods* they name are corroborated by older confirmed sources, but treat their exact numbers as
  unconfirmed (Appendix B). The confirmed foundational set is load-bearing; the rest is directional.
- **f64→f32 loses cross-class bit-determinism.** "Deterministic" holds within a target class, not across
  MCU FPU variants; state this wherever the determinism claim is made.
- **No fake-green.** A well-proven FAIL/BLOCKED is a successful run. Never trust a doc's "DONE" —
  re-verify against `cargo test`/`git` (`AUTONOMOUS-ORGANISM §7`; the corpus has same-day false-greens).
- **Keep the human as volition + effector gate.** This layer originates proposals + verifies + learns
  *within the gates*; irreversible acts always gate to a human.

---

## Appendix A — Agency-test battery (maps the operator's prior "agency tests" to kernel metrics)

| Prior "agency test" (dialogue) | Measurable version | Kernel primitive | Pass criterion |
|---|---|---|---|
| **Emergent connections** (novel synthesis) | spectral-entropy coverage of recalled sub-graph ÷ full-corpus graph; PPR spreading-activation across distant seeds | `csr::personalized_pagerank:204`, `spectral::eigenvalues:195` | coverage ratio ↑ vs. a pure-lexical baseline; connects nodes >2 hops apart |
| **Goal-oriented autonomy** | reachable-to-goal + expected-steps ÷ shortest-path (efficiency); pass^k over repeats | `order_machine::reachable:270`, `absorbing::expected_steps:67` | path exists; efficiency ≥ threshold; pass^k reported (not pass@1) |
| **Robustness / anti-fragility** (garbage input) | success-rate **slope + knee-point** under injected perturbation | corrupt fraction of graph edges / truncate Markov window | shallow slope, late knee; ignores noise, holds structure |
| **Cross-domain synthesis** (L1) | transfer ratio = off/on-diagonal on a task-graph; Laplacian-spectrum distance as x-axis | `spectral::laplacian:242` | transfer-decay curve over *provable* structural distance |
| **Adversarial meta-analysis** (L2, argue the opponent then refute) | multi-objective Pareto output (holds both sides) + `verify_then_lookup` on each claim | `verify_retrieval:36` | output dominates on ≥2 axes; each claim survives bounded re-verify |
| **Strategic autonomy / graceful degradation** (L3, memory-sector loss) | resource-loss `success(resource)` slope; demote-never-delete replay after "damage" | §8 store + `absorbing` | working-set recomputable from log; degradation graceful not cliff |
| **Dialectical / paradox-holding** | return the trade-off matrix, not a scalar | multi-objective frontier | frontier emitted; no premature collapse to one side |

*Empowerment* (Blahut-Arimoto over the Markov transition matrix) is the single reward-free scalar that
summarizes "is it still in control of its field" → 0 ⟺ lost control, the direct analogue of the shipped
ρ≈0 ⟺ acyclic result.

## Appendix B — Citation ledger

**Confirmed / foundational (cite directly):** Chollet 1911.01547 · ARC-AGI (arcprize.org) · Lanham
2307.13702 · Turpin 2305.04388 · Anthropic reasoning-faithfulness 2505.05410 · Parcalabescu & Frank
2311.07466 · ALCE 2305.14627 · AIS 2112.12870 · SelfCheckGPT 2303.08896 · semantic-entropy Nature 2024
(s41586-024-07421-0) · EigenScore/INSIDE 2402.03744 · FActScore 2305.14251 · TruthfulQA 2109.07958 ·
HaluEval 2305.11747 · FEVER 1803.05355 · Lost-in-the-Middle 2307.03172 · RULER 2404.06654 · LongBench
2308.14508 · Zheng LLM-judge 2306.05685 · Wang position-bias 2305.17926 · PoLL 2404.18796 · Guo
calibration 2017 · LiveCodeBench 2403.07974 · Leaderboard-Illusion 2504.20879 · GAIA 2311.12983 ·
WebArena 2307.13854 · τ-bench 2406.12045 · METR 2503.14499 · empowerment (Klyubin 2005) · options (Sutton
1999) · HippoRAG 2405.14831 · MemGPT 2310.08560 · Generative Agents 2304.03442 · eigen-geometry drift
2411.02464.

**Verify before load-bearing use (post-cutoff / unconfirmed exact numbers):** all arXiv `2601–2606.x`
IDs (ReliabilityBench, Agent-Contracts, AgentNoiseBench, "Beyond pass@1", PushBench/QGP, etc.), ARC-AGI-2
2025 leaderboard figures, and ARC-AGI-3 per-model scores. The *methods* are corroborated by the confirmed
set above; the *figures* are not independently verified in this pass.
