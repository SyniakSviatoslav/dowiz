# BLUEPRINT — Items 70 · 71 · 72: The Digital-Twin Trilogy (halves A · B′ · B)

> The roadmap frames these three as halves of one arc, so they share one doc with three
> clearly-headed sections: **70** state-mirroring digital twin, half (A) — REAL, near-term; **71**
> cost-aware eqc-rs rewrite-extraction, half (B′) — the one honestly-scoped step toward (B),
> operator-gated; **72** auto-optimizing digital twin, half (B) — LONG-TERM ASPIRATION, EXPLICITLY
> NOT ready. Section 72 is deliberately **not an execution plan** — it documents the entry gate the
> roadmap itself insists on.

- **Date:** 2026-07-19 · **Tier:** §K composition (70/71 real; 72 aspirational) · **Status:**
  BLUEPRINT (planning artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K items
  70–72 (lines 1200–1246) + §K dependency line (1252–1253); `tools/eqc-rs/src/lib.rs` (the extraction
  machinery item 71 extends); `docs/audits/hardening/{HOT-PATHS.tsv,CHECKLIST.md}`. Ground truth for
  code citations: this worktree at HEAD `6701bbb6f`.
- **Upstream (landed, cited live):** the spectral/graph kit reused AS-IS — `spectral::{classify_drift,
  spectral_radius, DriftClass}` (`kernel/src/hydra.rs:22`; `DriftClass::{Damped,Resonant,Unstable}`),
  `markov::{analyze, analyze_detailed}` (`kernel/src/markov.rs:284,110`), the Laplacian
  (`kernel/src/incidence.rs:107`, `kernel/src/spectral_laplacian.rs:83`), CSR parity
  (`csr.rs`, item-7 exhaustion precedent); the `eqc-rs` extraction machinery (`tools/eqc-rs/src/lib.rs`
  — `Expr` :56, `eval` :208, `emit_proof_program` :442).
- **Upstream (gating):** items 67 + 68 (cost oracle — twin leaf-cost source); item 62 (relational
  FDR lineage — call matrix source), no blueprint yet.

---

## 0. The binding forced-metaphor guard (Anu/Ananke) — read this first, applies to all three

Carried exactly from the roadmap (lines 1210–1213): the spectral machinery answers **GRAPH-level
questions only** — convergence, flow, bottleneck, drift. **Per-leaf cost comes from
enumeration/interval ONLY.** The twin must never present a spectral quantity as an individual
function's cycle count. This is not a stylistic footer — it is a structural design constraint on the
API surface (§70.3 step 5, §70.4 proof). A "quality router" of functions by a spectral score is
exactly the kind of unrepresentable-by-construction thing the kernel already forbids elsewhere
(routing enums omit `Ord`); the twin honors the same discipline.

---

# Item 70 — State-mirroring digital twin, half (A): REAL, near-term

## 70.1 Scope / goal

NOT a new subsystem. The twin is the **composition of three already-real/already-scoped pieces**:

1. **Per-function cost oracle** — item 67's buckets + item 68's tables/intervals/distributions
   (leaf-level cost, enumerated/interval only).
2. **Aggregate call-graph layer, reusing `spectral.rs`/`markov.rs`/`csr.rs` AS-IS** — no new
   machinery:
   - `ρ(A)` of the frequency-weighted call matrix decides whether total propagated cost converges
     (`c = (I−A)⁻¹·c_self`), by applying the *existing* `classify_drift` `Damped/Resonant/Unstable`
     verdict to the call matrix (the exact pattern `hydra.rs` already uses to reject a divergent
     mutation — `hydra.rs:53` "Returns Unstable if the proposed mutation would diverge");
   - Laplacian diffusion for where cost concentrates (bottlenecks);
   - `markov::analyze` over discretized cost-tier tokens for resource-regime drift.
3. **The `eqc-rs` precedent** (equation → proven-faithful Rust mirror) as the template that "real
   behavior mirrored by real math" already works here.

**Deliverable.** Given `(action, inputs)` → its bucket + value/interval/distribution + evidence
pointer; and (via `ρ(A)`) the propagated aggregate answer.

**Non-goals.** No optimization/search (that is item 72). No new spectral machinery. No per-leaf cost
from a spectral value (the §0 guard). No new dependency.

## 70.2 Current state (grounded)

| Piece | Citation (live HEAD) | Reuse posture |
|---|---|---|
| `classify_drift` + `DriftClass{Damped,Resonant,Unstable}` + `spectral_radius` | `kernel/src/spectral.rs` (imported at `hydra.rs:22`; variants at `hydra.rs:505,516`) | AS-IS on the call matrix |
| ρ(A) applied to a graph to decide divergence (the exact pattern) | `kernel/src/hydra.rs:53,221` (mutation-proposal adjacency → `Unstable` rejected) | precedent for the call-matrix verdict |
| `markov::analyze` / `analyze_detailed` | `kernel/src/markov.rs:284,110` | AS-IS over cost-tier tokens |
| Laplacian (diffusion / bottleneck) | `kernel/src/incidence.rs:107`, `kernel/src/spectral_laplacian.rs:83` | AS-IS for concentration |
| CSR + exhaustive parity idiom | `csr.rs` (item-7 `laplacian_dense_vs_spmv_parity_exhaustive_small`) | AS-IS |
| Leaf cost buckets/tables | items 67/68 (this arc) | consumer |
| Call matrix (frequency-weighted edges) | item 62 relational FDR lineage (`span_id`/`parent_span_id`) | consumer — the matrix source |
| eqc-rs faithful-mirror precedent | `tools/eqc-rs/src/lib.rs:442` (`emit_proof_program`) | template |

## 70.3 Implementation plan (numbered)

1. **Assemble the leaf oracle lookup.** A `fn cost_of(action, inputs) -> CostAnswer` returning
   `{bucket, value|interval|distribution, evidence_pointer}` sourced entirely from items 67/68's
   HOT-PATHS classification. An action with no classification returns the **forbidden-state error**,
   never a guess (the §0/coverage discipline).
2. **Build the frequency-weighted call matrix `A`** from item 62's relational FDR lineage
   (`span_id`/`parent_span_id` edges) — off-line/consumer-side, on the P3 plane (never a decision
   input).
3. **Compute the aggregate verdict** by applying `classify_drift`/`spectral_radius` to `A` AS-IS:
   `Damped` ⇒ propagated cost converges (`c = (I−A)⁻¹·c_self` well-defined); `Resonant`/`Unstable` ⇒
   report the divergence honestly rather than a finite number.
4. **Bottleneck + regime layers** via the existing Laplacian diffusion and `markov::analyze` over
   discretized cost-tier tokens.
5. **Enforce the forced-metaphor guard structurally** (§0). No per-leaf API method derives from a
   spectral value — the leaf answer comes only from `cost_of` (step 1); the spectral layer exposes
   only graph-level verdicts (converges/bottleneck/regime). Grep-checkable naming: no function named
   like `cycle_count_of(fn)` may read `spectral_radius`/`classify_drift`.

## 70.4 Required proofs (CHECKLIST 5-point) + acceptance

- **Item 1 (oracle):** **coverage-complete over every HOT-PATHS action** — an unclassifiable query
  returns the forbidden-state error (not a guess); a **differential check on ORACLE-EXACT functions**
  (the twin's stated cost class matches a fresh measurement within the stated noise interval);
  **ρ(A) verdict validated on a synthetic recursive call graph with known divergence** (red→green both
  directions — a divergent graph yields `Unstable`, a contracting one `Damped`).
- **Item 5 (structural proof):** the forced-metaphor guard asserted structurally — no per-leaf API
  derives from a spectral value (reviewed + doc-ruled + grep-checkable naming). This is the §0 guard
  as a test, not a footnote.
- **Items 2/3/4:** N/A — the twin is a P3 consumer analysis over already-gated math; the leaf math
  (`spectral`/`markov`/`csr`) carries its own coverage under its existing HOT-PATHS rows; adding no new
  secret-timing or branch-free-crypto path.
- **Falsifiable acceptance:** (a) every HOT-PATHS action resolves to a bucket or the forbidden-state
  error; (b) the EXACT differential check passes within the noise interval; (c) `ρ(A)` classifies a
  known-divergent and a known-convergent synthetic graph correctly; (d) grep proves no leaf API reads
  a spectral value. **Falsifier for (d):** any per-function cycle-count method sourced from
  `spectral_radius` → FAIL (Anu/Ananke guard breach).

## 70.5 Dependency gate (honest)

**After {item 67 + item 68}, with item 62 feeding the call matrix.** Items 67/68 are blocked on
57 (see the cost-oracle blueprint); **item 62 has no blueprint yet** (spec-level — the FDR relational
lineage that produces the call matrix). So **item 70 is blocked on 67, 68, and 62**. The spectral/markov
machinery it reuses is fully landed, so the *graph-verdict* layer can be prototyped against a synthetic
matrix ahead of 62, but the real twin ships after all three.

---

# Item 71 — Cost-aware eqc-rs rewrite-extraction, half (B′): the one honestly-scoped near-term step

## 71.1 Scope / goal

Give `eqc-rs` codegen a cost-aware extraction over a **SMALL, HAND-CURATED, FINITE** set of
provably-equivalent algebraic rewrites, choosing the cheaper form by lower op-count at codegen time
and REUSING `emit_proof_program` to prove the chosen form still equals the `Expr::eval` reference.
Equality-saturation's "extraction picks the cheapest equivalent" idea at **toy scale**: **no e-graph,
no SMT, no SAT, zero new dependency** — honestly "constant folding plus strength reduction with a
proof," NOT a superoptimizer, and it must never be described as one (roadmap line 1230).

**The finite rule set (curated, closed):**
- strength reduction: `a*2 → a+a`
- factoring: `a*b + a*c → a*(b+c)`
- constant folding: `Num(k1) ⊕ Num(k2) → Num(k1 ⊕ k2)`

## 71.2 Current state (grounded)

`eqc-rs` is a landed, tested equation→Rust compiler (`tools/eqc-rs/src/lib.rs`):
- `Expr` tree (`:56`) with `Sum`/`Prod`/`Pow`/`Num`/`Sym` — the rewrite operates on this tree BEFORE
  emission.
- `Expr::eval` (`:208`) — the independent tree-walking interpreter that is the proof reference.
- `emit_proof_program` (`:442`) — already emits a self-contained Rust program asserting the emitted
  code equals the `Expr::eval` reference across samples; compiled by real `rustc`, self-asserting (the
  proof IS the artifact). This is the machinery item 71 reuses to prove a *rewritten* form still
  equals the reference.
- Emission paths (`emit_f64_rust` :360, `emit_int_checked_rust` :393, `emit_fixed_rust` :413) — the
  rewrite chooses a cheaper `Expr` before these run; the emitters are untouched.

## 71.3 Implementation plan (numbered)

1. **Define an op-count cost metric on `Expr`** — a pure `fn op_count(&Expr) -> u64` counting
   arithmetic nodes (the documented cost metric, recorded in the eqc-rs README per line 1233). This is
   the *only* cost model — no timing, no e-graph.
2. **Implement the finite rewrite set as pure `Expr → Expr` functions**, each applied at authoring
   time, each preserving mathematical equality by construction (the three rules above; the set is
   closed and hand-audited).
3. **Extraction = pick the lower `op_count` form.** After applying a rule, keep the rewritten `Expr`
   iff its `op_count` is strictly lower; otherwise keep the original. Deterministic, terminating (the
   rule set is finite and monotone-decreasing on op-count).
4. **Prove the chosen form via `emit_proof_program`.** For every emitted case where a rewrite fired,
   the generated proof program (`:442`) asserts the chosen form still equals the `Expr::eval`
   reference (`:208`) — compiled by real `rustc`, green exit is the proof.
5. **No-rule-applies is byte-identical** to today's output — the rewrite is a no-op on trees no rule
   matches (§71.5 acceptance).

## 71.4 Required proofs (CHECKLIST 5-point) + acceptance

- **Item 1 (oracle) + Item 5 (proof program):** per rule, an emitted case where the cheaper form is
  demonstrably chosen (`op_count` strictly lower) **with its proof program green** (compiled by real
  rustc, self-asserting — this is the strongest oracle form the repo has: an independent-reference
  equality proof). The `emit_proof_program` reuse means the equivalence is machine-checked, not
  hand-argued.
- **Item 3 (debug cross-check):** the rewrite's equality is additionally cross-checked at
  authoring/test time by evaluating both forms under `Expr::eval` on the sample set.
- **Items 2/4:** N/A — authoring-time codegen, no runtime secret timing, no branch-free crypto.
- **Falsifiable acceptance:** (a) each of the 3 rules has a case where the cheaper form is chosen +
  proof program green; (b) a no-rule-applies input emits **byte-identical** output to today; (c) the
  op-count metric is documented in the eqc-rs README; (d) the full eqc-rs suite is green; (e)
  `cargo tree` unchanged (no new dep). **Falsifier for (b):** any drift in output on a
  no-rule-applies input → FAIL (the rewrite is not conservative).

## 71.5 Dependency gate + operator-decision (honest)

- **Independent of items 67–70.** It extends the standalone `eqc-rs` tool and needs none of the cost
  oracle / twin work.
- **OPERATOR-GATED whether to build at all** (roadmap line 1223): "offered as the smallest grounded
  step, not a commitment." This blueprint scopes it; **it does not authorize building it.** The
  operator rules go/no-go. It is the honest smallest step toward (B) — and the entry criterion (i) for
  item 72.

---

# Item 72 — Auto-optimizing digital twin, half (B): LONG-TERM ASPIRATION, EXPLICITLY NOT READY

## 72.0 Why this section is NOT an execution plan

The roadmap marks item 72 as carrying **"no proof conditions and no schedule — deliberately"** (line
1241). Writing an implementation plan for it would be exactly the fabricated-roadmap-promise the item
was created to avoid. This section therefore documents, honestly, **what it is, why it is not ready,
and what would have to be true first** — and stops there. That is the correct deliverable for item 72.

## 72.1 What it is (scoped honestly)

"Always finds a shorter/faster version of any action" is **automated superoptimization** — a real,
hard, active research field: STOKE (stochastic search), Souper (SMT synthesis), egg/egglog (equality
saturation with cost-model extraction). Its machinery — exponential search spaces, e-graph/SMT
engines — is **antithetical TODAY** to a zero-dep deterministic kernel (roadmap lines 1238–1240).

## 72.2 Why it is NOT ready (the gate, stated as fact)

The roadmap's entry criteria — **all three required before any work begins** (lines 1242–1245):

1. **Item 71 landed with measured wins** demonstrating extraction value on real kernel math. (Item 71
   is itself only *scoped* here and operator-gated — not built. So criterion (i) is unmet.)
2. **An explicit operator ruling** accepting the tooling/determinism cost for a *bounded* target
   domain. (Not given.)
3. **A fresh research pass** — this item is a pointer, not a plan. (Not done.)

**Current status against the gate:** 0 of 3 criteria met. Item 71 is unbuilt (and itself unauthorized);
no operator ruling exists accepting superoptimizer tooling into the zero-dep kernel; no research pass
has been commissioned. Item 72 is therefore **NOT READY** and this blueprint writes no plan for it — by
design.

## 72.3 What the DECISIONS.md / MANIFESTO tension is (the real blocker, named)

Beyond the three procedural criteria, item 72 sits against the repo's structural invariants: a
superoptimizer's engine (e-graph/SMT/SAT) is a heavy external dependency, and DECISIONS.md D0's
"reliability-over-latency" plus the zero-dep / serde-free / deterministic-core discipline mean any
such tooling must stay CI-time-only (like Kani, terminal-state-(c) in the item-25 procedure) and its
*output* must remain hand-inspectable, proven Rust — the eqc-rs discipline. Whether even a bounded,
CI-time, proof-carrying superoptimizer is worth its determinism/tooling cost is precisely the operator
ruling criterion (ii) reserves. This blueprint records the tension; it does not resolve it.

## 72.4 The honest one-line status

**Named direction, zero commitment — the honest opposite of a fabricated roadmap promise.** Until all
three entry criteria hold, item 72 remains a pointer on the roadmap, not a dispatched item.

---

## Cross-item dependency summary (this doc)

| Item | Gate | Status |
|---|---|---|
| 70 | after {67 + 68}, call matrix from 62 | BLOCKED on 67, 68, 62 (spectral/markov machinery landed; graph layer prototypable) |
| 71 | independent + OPERATOR-GATED whether to build at all | READY to build (extends live eqc-rs) — awaits operator go/no-go |
| 72 | entry-gated on {71 landed w/ wins, operator ruling, fresh research} | NOT READY — 0/3 criteria met; no plan written by design |
