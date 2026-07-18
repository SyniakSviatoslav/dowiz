# BLUEPRINT P32 — Hydraulic-Loop-v2 wiring: self-tuning control loops, connection not construction (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). This phase IS
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1's **P32** (sub-letters
> P32a–P32d). The arc's pattern, per §10.5.1: nearly everything was BUILT and TESTED, almost
> nothing was WIRED — P32's work is connection. This blueprint **formalizes and cross-links**;
> the math lives in the source docs and is reused, never re-derived:
> `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` + `BLUEPRINTS.md` (exact filenames,
> verified `ls` this pass — the plan file is `HYDRAULIC-LOOP-v2-PLAN.md`, not "PLAN.md").
> Lower urgency than P34/P37/P38/P40/P41/P45 — nothing here is on the critical path.
>
> **Headline ground-truth findings of this pass:** (i) resonator's zero-call-site status is
> **confirmed still true** (fresh grep, §0 row 3) — P32b stands as charted; (ii) P32c's framing
> is **stale in a precise way**: `OnlineDMD` now HAS a real consumer (`instrument_panel.rs`
> reads `spectral_radius()` into a K-band alarm) but that consumer is itself uncalled outside
> tests — the strand moved one level up the chain (§0 row 5); (iii) **P06 key_V is CLOSED**
> (`58987d79d`, real bebop2-kv sign/verify) — any inherited "P06 blocks P32d-adjacent
> self-harness work" premise is stale (§0 row 8).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `76167336a7b2ed31fee38d7161d109d462763643`
(docs-only working-tree modifications, zero `.rs` diffs) and `bebop-repo` `main` @
`e56ba6a35258ced76752510625511f37a6367a77` (clean). Bebop paths relative to
`/root/bebop-repo/`; dowiz paths relative to `/root/dowiz/`.

| # | Claim | Fresh `file:line` (this pass) | Inherited cite (§10.5.1) | Status |
|---|---|---|---|---|
| 1 | P32a ledger — BP-08 intake, the wiring template | dowiz `kernel/src/intake.rs` = **779 lines** (fresh `wc`); registered `kernel/src/lib.rs:85`; consumer `kernel/src/loops.rs:11` `use crate::intake::{admit, …}` with BP-08 named in `loops.rs:4` doc | §10.5.1: "744 lines, `lib.rs:84`, `loops.rs:11`" | **DRIFT (grew)** — +35 lines, registration +1 line; consumer cite exact MATCH. Still the one hydraulic organ wired end-to-end — the template for P32b/c |
| 2 | P32a ledger — bebop crates side | `crates/bebop/src/lib.rs`: `entropy_ledger` (BP-06) `:29`, `governor` (BP-05) `:37`, `instrument_panel` (BP-19) `:39`, `orthogonality` (BP-10) `:54`, `persistence` (BP-09) `:58`, `renormalizer` (BP-11) `:67`; `loop_runtime.rs:28` `use crate::orthogonality::goodhart_alarm` (+ doc `:12` naming it in the L6 SENSE lane) | §10.5.1: governor.rs:180-233, orthogonality consumed at loop_runtime.rs:421 | registrations fresh; the `:421` consumption line and `governor.rs:180-233` internals inherited (not re-walked — registration + import verified suffice for a ledger row) |
| 3 | P32b — resonator registered, ZERO call sites | `bebop2/core/src/lib.rs:328` `pub mod resonator;` (**MATCH exact**); `grep -rn "resonator::" bebop2/ crates/ --include="*.rs"` excluding `resonator.rs` → **0 hits** (fresh, this pass) | §10.5.1: same claim | **CONFIRMED STILL TRUE** — the swarm has NOT wired it since §10.5.1 was drafted |
| 4 | P32b — resonator's live API + the metric seam | `bebop2/core/src/resonator.rs`: `pub trait Metric<S>` `:169`, `L2Metric` + `impl Metric<Vec<f64>>` `:174-175`, `run_resonator<S: Clone, M: Metric<S>>` `:211`, `rollback_to_best` `:312`, `DriftAccumulator` `:125`; `bebop2/core/src/algebra.rs:55` `pub fn geodesic_distance(a: &[f64], b: &[f64]) -> f64` — callers outside `algebra.rs`: **0** (fresh grep; `:215,:239-251` are its own tests) | §10.5.1: "algebra.rs:56 geodesic_distance" | **DRIFT −1 line** on the fn; API cites new (needed for §3.2's trait-impl spec) |
| 5 | P32c — **STALE framing: DMD now has a consumer, one level short of live** | `bebop2/core/src/lib.rs:316` `pub mod dmd` (MATCH); `bebop2/core/src/field.rs:328` is a **doc comment** ("Reused by `dmd` (BP-07 POD covariance)" — Jacobi reuse note, not a call; matches §10.5.1's "referenced not called"); **NEW:** `crates/bebop/src/instrument_panel.rs:19` `use bebop2_core::dmd::OnlineDMD`, `:88` `report(…, dmd: &OnlineDMD, …)`, `:100` `let k_dmd = dmd.spectral_radius()`, `:126` K-band alarm `k_dmd ∈ [K_BAND_LO, K_BAND_HI]`; **BUT** `InstrumentPanel`/`report` has **0 non-test callers** repo-wide (fresh grep) and `loop_runtime.rs` has **0** `dmd` hits | §10.5.1: "only *referenced* — not called — from `field.rs:328`" | **DRIFT** — a real dmd→panel edge landed since; the panel itself is stranded. §3.3's DoD is re-anchored to the live chain |
| 6 | P32c — DMD internally consumes BP-03 | `bebop2/core/src/dmd.rs:332,387,397` call `lyapunov::eigenvalues_general`; `bebop2/core/src/lyapunov.rs:395` `pub fn eigenvalues_general(a: &[f64], n) -> Vec<Complex>` | not in §10.5.1 | **NEW** — evidence that BP-03 (a P33b audit item) is built AND consumed; handed to P33b's seed table, not judged here |
| 7 | P32d — no multi-model-voting code | no critic/voting module found (consistent with §10.5.1's "zero code found"; not exhaustively re-grepped — P32d is PLANNED, the design note is the first deliverable either way) | §10.5.1: PLANNED | MATCH |
| 8 | P32d context — **P06 key_V CLOSED** | dowiz `tools/ci-truth/src/v1.rs:405+` `HybridSigner` (`signed() -> true` `:416`; shells `bebop2-kv sign/verify`; fail-closed empty sig on error); commit `58987d79d` "P06 key_V HybridSigner — real bebop2-kv sign/verify + TLV sig field", landed **today**, ancestor of HEAD | prior sessions: "P06 blocks H3/E3-Phase-B" (memory) | **CLOSED** — the cross-model-critic's neighborhood (self-harness Phase-B, E3) is no longer P06-blocked; P32d's own dependency was never P06 (see §3.4) |
| 9 | P32d soft-dep — LlmBackend exists on main | dowiz `llm-adapters/src/` (`lib.rs`, `ollama.rs`, `cache.rs`, `dispatch.rs`, …); `cache.rs:16` imports `LlmBackend` from the crate root; `cache.rs:31` `CachingBackend<B: LlmBackend, S>` | §10.5.1: "may reuse AGENT's LlmBackend once wired" | **SHARPENED** — the trait + Ollama adapter are ON MAIN; what remains owned by P40/P41 is consumer wiring, so P32d's implementation dependency is thinner than §10.5.1 implied |
| 10 | BP-22 stays resolved | `agent-governance/resonator.ts` — no TS port present (per P32a ledger; not re-verified beyond the standing deletion commit trail) | §10.5.1: RESOLVED DIFFERENTLY | inherited-accepted (closed item, zero build) |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — the four sub-letters, and what P32 deliberately does NOT own

**P32's single sentence:** connect the already-built hydraulic control-loop organs — resonator
(+ arccos metric) and OnlineDMD — into real live consumers using BP-08's intake→loops pattern
as the template, and add the one never-built item (cross-model critic) as design-note-first;
construction is finished, only connection remains.

| Sub | Absorbs | Status (fresh, §0) | Character |
|---|---|---|---|
| P32a | BP-05, BP-06, BP-08, BP-09, BP-10, BP-22 | DONE | index-only, zero build |
| P32b | BP-01, BP-02 | WIRING-GAP (rows 3-4, confirmed) | trait impl + 1 call site + round-trip test |
| P32c | BP-07 | WIRING-GAP, re-anchored (row 5) | complete the dmd→panel→loop chain |
| P32d | cross-model critic (unnumbered, from the 7 math corrections) | PLANNED (row 7) | design note → minimal advisory impl |

**Anti-scope (binding):**
- **Do not touch P32a organs.** In particular `governor.rs::default_legacy()` is a deliberate
  RED regression fixture — never "modernized" (§10.5.1's explicit rule).
- **Do not rebuild resonator or redesign the `Metric` trait** (P32b wires only). If no existing
  loop genuinely benefits from resonator's output, **stop and report that as a finding, not a
  failure** — §10.5.1's own escape hatch, kept verbatim.
- **Do not rebuild or extend OnlineDMD** — rank-1 RLS is done; no higher-rank updates, no
  forecasting, no new spectral features.
- **Do not build an AI-council framework** (P32d): one loop output, minimal voting, advisory
  only; critic output NEVER gates anything deterministic (GROUND-TRUTH-over-PROXY, standing).
- **No courier/agent scoring or reputation** anywhere in these loops — trust = signed
  capability, standing rejection.
- P32b ∥ P32c are parallel lanes, but they share a consumer landscape
  (`loop_runtime.rs`/`field.rs`/`instrument_panel.rs`) — **sequential landing, parallel prep**
  (§10.5.1's coordination rule, kept).
- BP-03/04/11/12..21/23 status classification belongs to **P33b**, not here (row 6's BP-03
  evidence is handed over, not adjudicated).
- Repo routing: all P32b/P32c code edits land in `/root/bebop-repo` (push remote `openbebop`),
  never in `/root/dowiz` — standing rule (`cross-branch-todo-map-2026-07-10`).

---

## 2. Predefined types & constants (standard item 4)

P32a needs none. P32b/P32d name theirs before implementation; P32c introduces no new types
(its work is instantiation + call sites of existing ones):

```rust
// ── P32b: the ONLY new production type (bebop2/core/src/algebra.rs or resonator.rs) ──
/// BP-02's own name, reused: the arccos/geodesic metric as a resonator Metric.
/// A newtype impl, NOT a free function sitting nearby (DoD-1's exact phrasing).
pub struct AngularMetric;
impl Metric<Vec<f64>> for AngularMetric {
    fn distance(&self, a: &Vec<f64>, b: &Vec<f64>) -> f64 {
        crate::algebra::geodesic_distance(a, b)   // algebra.rs:55 — reused, not reimplemented
    }
}
// (Exact trait-method name/signature per resonator.rs:169-172 at implementation time —
//  the trait is NOT redesigned to fit this impl; the impl bends to the trait.)

// ── P32d: design-note-first; the note must predefine (names fixed here) ──
/// One critiqued output type (pilot): the governor's PID correction for a tick.
pub struct CritiqueInput { pub loop_id: &'static str, pub tick: u64, pub value: f64 }
/// Advisory verdict from ≥2 decorrelated judges. NEVER consumed by a gate.
pub struct CriticSignal { pub agree: bool, pub judges: [JudgeId; 2], pub note: String }
```

Constants: none new. K-band bounds for the DMD alarm already exist
(`instrument_panel.rs:126`); P32c reuses them.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 P32a — DONE+wired ledger (zero build)

The §10.5.1 one-line ledger stands, with §0 rows 1-2's fresh cites (intake now 779 lines,
`lib.rs:85`; bebop registrations at their fresh lines). BP-08's intake→loops chain remains the
**wiring template**: a built organ, a registration, and a REAL consumer that changes behavior —
that three-part shape is what P32b/P32c must reproduce. Falsifier for this section: any P32a
file diff attributed to P32 (§5).

### 3.2 P32b — Resonator + arccos metric wiring (BP-01 + BP-02)

**Spec:** §2's `AngularMetric` + ONE real call site. Candidate consumers in §10.5.1's
preference order, re-validated against §0: (1) `crates/bebop/src/loop_runtime.rs` — already
consumes orthogonality/persistence, the natural L-lane home; (2) dowiz `kernel/src/loops.rs`
via the BP-08 pattern. The wiring decision (which loop, which resonator output feeds which
input) is the only new design, and it is one paragraph in the PR, not a new doc — the math is
`HYDRAULIC-LOOP-v2-PLAN.md` BP-01/BP-02's, reused.

**RED tests (written first):**
1. `angular_metric_is_resonator_metric` — compiles only when `AngularMetric: Metric<Vec<f64>>`
   (trait-impl proof; RED today: no such impl exists, §0 row 4).
2. `loop_round_trip_through_resonator` — drive the chosen consumer loop for N ticks with
   resonator in the chain; assert on the **event sequence** of loop decisions (not just final
   state — standard item 3), reusing resonator's existing checkpoint/rollback semantics
   (`resonator.rs:115,312`). RED: no call site exists (grep `resonator::` = 0 hits, §0 row 3).
3. **Decorative-wiring guard (DoD-4, kept verbatim from §10.5.1):** with resonator's
   contribution zeroed, at least one loop-level test result DIFFERS — wiring that changes
   nothing is not wiring.

**Adversarial:** (i) feed `run_resonator` a reference trajectory that `DriftAccumulator::is_chaotic`
(`resonator.rs:157`) flags — assert the consumer loop takes the rollback path
(`rollback_to_best`, `:312`), not the divergent one; (ii) metric teeth: swap `AngularMetric`
for `L2Metric` in a test-local copy and assert at least one ranking/decision differs on a
fixture where angular ≠ euclidean ordering (proves the arccos metric is load-bearing, not
interchangeable decoration).

### 3.3 P32c — Online DMD wiring (BP-07) — re-anchored on the live chain

**The §10.5.1 DoD assumed the gap was at `field.rs:328`; §0 row 5 shows the live gap is one
level up.** The chain today: `dmd.rs` (built, tested) → `instrument_panel.rs::report` reads
`spectral_radius()` into `k_dmd` + K-band alarm (**edge exists**) → nothing calls `report` in
a non-test path (**edge missing**). Re-anchored spec, preserving §10.5.1's intent exactly
("mode estimates inform something live"):

1. **The missing edge:** a non-test code path (preference: `crates/bebop/src/loop_runtime.rs`,
   which already aggregates the L-lane instruments, §0 row 2) instantiates `OnlineDMD`, updates
   it with real field-state samples across ticks, and calls `InstrumentPanel::report` with it —
   completing dmd → panel → loop.
2. **A downstream decision reads DMD output** (§10.5.1 DoD-2, kept): the K-band alarm result
   (`instrument_panel.rs:126`) must feed a real branch in the consumer (throttle, rollback,
   log-escalate — whichever the loop already has; no new actuator invented).
3. `field.rs:328`'s doc-comment reuse note stays a comment — §10.5.1's "reference becomes a
   call" phrasing is satisfied at the panel seam, where the live consumer actually is; forcing
   a second call site into `field.rs` for literalism would be decorative wiring.

**RED tests:** round-trip per §10.5.1 DoD-3, kept: feed a synthetic signal with a known
dominant mode through the WIRED path (loop → dmd update → panel → alarm) and assert the
consumer sees it — reusing `dmd.rs`'s existing fixtures (e.g. the `new_from_snapshots` shape
already used in `instrument_panel.rs:208` tests). RED today: no non-test caller exists.
**Adversarial:** (i) a signal whose dominant mode sits OUTSIDE the K-band must trip the alarm
branch (and the branch must be observable in the loop's event sequence); (ii) teeth: freeze the
DMD updates (never call update) and assert the round-trip test FAILS — guards against a panel
that reports a stale constant.

### 3.4 P32d — Cross-model critic (design-note-first)

**Unchanged from §10.5.1 in shape; two premises refreshed:** (i) P06 key_V is CLOSED (§0
row 8) — no inherited blocker language applies to anything in this lane; (ii) the LlmBackend
trait + Ollama adapter are on main (§0 row 9) — the implementation's soft dependency is only
"P40/P41 wire consumers", and the **design note proceeds now** regardless.

**Deliverable 1 — the design note** (short, standard item 18-compliant), predefining: the
critiqued output (pilot = one `CritiqueInput` per governor tick, §2), the decorrelation
constraint (≥2 judges on different models/providers — the `research-verifier` precedent:
decorrelated verifier on a different provider), and the advisory-only integration point
(signal logged/ledgered; NEVER a gate — GROUND-TRUTH-over-PROXY).
**Deliverable 2 — minimal implementation:** one loop output critiqued by 2 decorrelated
judges; disagreement surfaces as a `CriticSignal` in the log.
**RED test (DoD-3, kept verbatim):** a deliberately corrupted loop output (e.g. a PID
correction with flipped sign) triggers a critic disagreement signal in a test.
**Adversarial:** (i) judge-collusion probe: both judges given the SAME model/provider must be
rejected at construction (decorrelation is a typed constraint, not a convention);
(ii) gate-leak probe: grep-level assertion that no deterministic gate imports the critic module
— advisory posture enforced structurally, not by review vigilance.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

- **Hazard-safety as math (6):** the loops being wired GOVERN the agent's own machinery, so the
  hazard is a control loop acting on stale/decorative signals. Each wiring carries a
  structural guard: P32b's zeroed-contribution differ-test and metric teeth; P32c's
  frozen-update teeth; P32d's typed decorrelation + the no-gate-import assertion. Stability
  itself is the source docs' math (Jury-stable PID, Lyapunov/drift accumulators, Tikhonov
  well-posedness in `admit()`) — consumed, not re-proven here.
- **Scaling axes (8):** these are per-tick control loops on one node: state is O(loop-state)
  (resonator checkpoints: O(N_ticks) worst case — its own `rollback_to_best` bounds usage;
  OnlineDMD: rank-1, O(r²) memory, r small and fixed). Break point: none at product scale —
  the loops run per-agent-node, not per-order; stated so nobody "scales" them speculatively.
- **Linux-discipline verdicts (9):** wiring existing organs through an existing consumer
  pattern (BP-08's) = **ALREADY-EQUIVALENT**; the decorative-wiring differ-tests =
  **REINFORCES** (the repo's teeth-first test culture); P32d's decorrelated advisory critic =
  **EXTENDS** (new machinery, justified by the arc's own math-correction history — 7 real
  corrections found by cross-model review).
- **Isolation/bulkhead (11):** all P32b/c wiring is inside bebop-repo's existing crates —
  dowiz kernel untouched (the only dowiz-side P32 artifact is this doc). P32d's judges run
  out-of-process (LlmBackend adapters); a judge failure degrades to "no signal", never blocks
  a loop tick (advisory = fail-open for the signal, while the loops themselves stay
  fail-closed on their own invariants).
- **Mesh awareness (12):** **node-local, all of it** — these loops govern a single agent
  node; no transport, no gossip, no payload budget. One line, honestly N/A beyond it.
- **Rollback vocabulary (13):** P32b consumes resonator's checkpoint/rollback — that is
  **Snapshot-Re-entry** (best-checkpoint diode, cheap regenerative recovery), claimed
  precisely because the mechanism exists at `resonator.rs:115,312`. P32c/P32d claim only
  **Self-Termination** flavor guards (alarms/refusals); no Self-Healing redundancy math is
  claimed anywhere in P32.
- **Error-propagation gates (14):** the differ-tests and teeth tests land in the crates' own
  `cargo test` (CI-gated); P32d's no-gate-import grep is the smart-index for the advisory
  boundary. The bug class "wired but decorative" becomes a named CI RED in both P32b and P32c.
- **Living memory (15):** resonator checkpoints and DMD mode history are time-ordered
  loop-local state, not stored knowledge; no retrieval-arc coupling. Stated, not padded.
- **Tensor/spectral reuse (16):** DMD's eigen-machinery uses `lyapunov::eigenvalues_general`
  (§0 row 6) and field.rs's shared Jacobi (`field.rs:328`'s actual point) — one spectral
  authority per concept, preserved by wiring instead of duplicating.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

§10.5.1's DoD kept 1:1; P32c items re-anchored per §0 row 5 with intent preserved.

| Item | §10.5.1 | RED (fails before) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|---|
| P32a | — | n/a | ledger cites match live source (§0 rows 1-2) | falsified by any P32a-organ diff attributed to P32 |
| P32b-1 | DoD-1 | no `Metric` impl exists (§0 row 4) | `AngularMetric: Metric<Vec<f64>>` via `geodesic_distance` (impl, not free fn) | `grep -n "impl Metric" bebop2/core/src/` shows the impl |
| P32b-2 | DoD-2 | `grep -rn "resonator::" --include="*.rs"` outside resonator.rs = 0 (§0 row 3) | same grep ≥ 1 hit in a live loop consumer | the grep, run verbatim |
| P32b-3 | DoD-3 | no round-trip test | `loop_round_trip_through_resonator` green (event-sequence assertion) | named test in bebop `cargo test` |
| P32b-4 | DoD-4 | — | zeroed-resonator run makes ≥1 loop-level test DIFFER; metric-swap teeth recorded | falsified by differ-test passing with contribution zeroed |
| P32c-1 | DoD-1 (re-anchored) | `InstrumentPanel::report` has 0 non-test callers; `loop_runtime.rs` has 0 dmd hits (§0 row 5) | a non-test path instantiates OnlineDMD, updates it with real samples, calls `report` | `grep -rn "OnlineDMD\|instrument_panel" crates/bebop/src/loop_runtime.rs` non-empty, non-test |
| P32c-2 | DoD-2 | K-band alarm feeds nothing | a real consumer branch reads the alarm/mode output | grep-verifiable consumer + branch visible in event sequence |
| P32c-3 | DoD-3 | no wired-path round trip | synthetic known-mode signal seen by the consumer through the full chain; frozen-update teeth recorded | named test; falsified by teeth not failing when updates frozen |
| P32d-1 | DoD-1 | no design note | note exists: critiqued output, decorrelation constraint, advisory-only point (§3.4 names fixed) | doc grep |
| P32d-2 | DoD-2 | no critic code | one output × 2 decorrelated judges; disagreement → logged `CriticSignal`, never a gate | no-gate-import grep clean |
| P32d-3 | DoD-3 | — | corrupted loop output triggers disagreement signal in a test | named RED-provable test |

Regression rows (item 17) for `docs/regressions/REGRESSION-LEDGER.md` on completion:
"P32b resonator load-bearing wiring — guardrail: zeroed-contribution differ-test";
"P32c DMD live-chain round trip — guardrail: frozen-update teeth test";
"P32d advisory boundary — guardrail: no-gate-import assertion". Ratchet rule verbatim:
red→green proof before any "done".

---

## 6. Benchmark plan (item 10) — light, per the lower-urgency framing

The loops are per-tick, per-node, off any product hot path. One measurement per wiring,
recorded not budgeted: (i) P32b — consumer-loop tick cost before/after resonator in the chain
(criterion or a timed test in the crate; the delta IS the resonator tax); (ii) P32c — same
shape for the dmd-update+report tick. Expectation to falsify: both taxes are microseconds-class
(rank-1 RLS update is O(r²); resonator tick is a metric eval + checkpoint bookkeeping) and
irrelevant at loop rates. If either tax surprises upward, that is a finding for the source arc,
not a P32 optimization license (`performance-priority-over-minimal-change-2026-07-17` is scoped
to perf arcs; this is not one). P32d: judge latency is out-of-process LLM latency — advisory,
async, explicitly unmeasured against loop budgets because it never sits on a loop tick.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1 (the P32 charter; kept 1:1,
re-anchored where §0 shows drift) · `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md`
+ `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` (BP-01/02/07 math + the full BP list — used
for *design*, never for *status*: its §status header block `BLUEPRINTS.md:70-85` is stale-RED
for items long landed, same class as mesh-real's table, P34 §0 row 20) ·
`BLUEPRINT-P33-core-ledger-hygiene.md` (P33b owns BP-03/04/11/12..21/23 classification; §0
row 6's BP-03 evidence handed there) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (structural
template; teeth-first discipline precedent) · `docs/regressions/REGRESSION-LEDGER.md` (item
17). Memory: `hydraulic-loop-v2-arc-2026-07-13` (7 math fixes; resonator=dead-code finding —
now confirmed-still-dead §0 row 3) · `harness-llm-backend-and-hermetic-remediation-2026-07-17`
(LlmBackend lineage; its "P06 blocks H3" line is superseded by §0 row 8) ·
`ground-truth-over-proxy-2026-07-07` (P32d's advisory-only posture) ·
`model-role-division-research-vs-reasoning.md` + the `research-verifier` agent definition
(decorrelation precedent) · `cross-branch-todo-map-2026-07-10` (repo routing: bebop files →
`/root/bebop-repo`) · `worktree-remote-push-collision-avoidance-2026-07-18` (push cadence).
Supersedes: nothing — §10.5.1 remains the charter; this file corrects its two stale premises
(P32c chain level, P06) with dated evidence.

---

## 8. Hermetic principles honored (item 20)

- **P6 CAUSE-AND-EFFECT:** every wiring must demonstrably change behavior (differ-tests,
  frozen-update teeth) — a wire that transmits no cause is rejected by the DoD itself.
- **P7 GENDER (paired creation, no self-certification):** P32d is this principle as
  machinery — loop outputs reviewed by decorrelated external judges, never by the loop's own
  model; and the critic itself cannot self-certify into a gate (structural no-gate-import).
- **P3 VIBRATION** (the one non-decorative extra): these loops exist to keep the system's
  operating point inside stable bands (K-band, Jury stability, drift accumulators) — P32 wires
  the sensors to the actuators; it does not re-derive the band math.
- (Others not load-bearing; not claimed, per Anu/Ananke.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 10 rows, fresh cites both repos, 2 stale-premise corrections (rows 5, 8), 1 confirmed-still-true (row 3), 1 new cross-phase evidence handoff (row 6) |
| 2 DoD | §5 — RED→GREEN per item, §10.5.1 kept 1:1, P32c re-anchor justified from §0 |
| 3 spec/TDD | §2 spec first; §3 RED tests precede code; event-sequence assertions (§3.2-2, §3.3) |
| 4 predefined types | §2 — `AngularMetric`, `CritiqueInput`, `CriticSignal`; P32a/c need none (stated) |
| 5 adversarial tests | §3.2 (chaotic-drift rollback, metric-swap teeth), §3.3 (out-of-band alarm, frozen-update teeth), §3.4 (collusion probe, gate-leak probe) |
| 6 hazard-safety as math | §4 — stale/decorative-signal hazard with structural guards; stability math consumed from source docs |
| 7 links | §7 |
| 8 scaling axes | §4 — O(r²)/checkpoint bounds; explicit no-break-point-at-product-scale statement |
| 9 Linux discipline | §4 — three verdicts |
| 10 benchmarks | §6 — per-wiring tax measured, honestly scoped as recorded-not-budgeted |
| 11 isolation | §4 — bebop-crates-only wiring; judge failure degrades to no-signal |
| 12 mesh awareness | §4 — node-local, honest one-line N/A |
| 13 rollback vocabulary | §4 — Snapshot-Re-entry claimed ONLY where the mechanism exists (resonator); rest Self-Termination |
| 14 error-propagation gates | §4 — differ/teeth tests CI-gated; no-gate-import smart index |
| 15 living memory | §4 — loop-local state, honest N/A |
| 16 tensor/spectral reuse | §4 — one spectral authority preserved (eigenvalues_general, shared Jacobi) |
| 17 regression ledger | §5 tail — three named rows |
| 18 agent instructions | §10 |
| 19 reuse-first | §1 anti-scope (wire, never rebuild; the §10.5.1 stop-and-report escape hatch kept) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repos: `/root/dowiz` (this doc) and `/root/bebop-repo` (ALL P32b/P32c code; push to
`openbebop`, never `origin` — it is archived read-only). P32b and P32c are parallel-preparable
but land SEQUENTIALLY (shared consumer files). Re-verify §0's rows 3 and 5 before starting —
this repo's swarm closes gaps fast, and P32c's framing already drifted once between §10.5.1
and this file.

1. **T1 (P32b).** Re-run the resonator grep (§0 row 3) — if non-zero, someone wired it; stop
   and re-audit. Else: implement `AngularMetric` (§2, bending to `resonator.rs:169`'s trait as
   it exists), pick the consumer loop (preference: `crates/bebop/src/loop_runtime.rs`; state
   the one-paragraph wiring decision in the PR), write §3.2's three RED tests FIRST, wire,
   green. Run both adversarials; record the metric-swap teeth. If no existing loop genuinely
   benefits — STOP and file the finding (sanctioned outcome, §1). Acceptance: §5 P32b rows 1-4.
2. **T2 (P32c, after T1 lands or on its prepared branch).** Re-run §0 row 5's greps. Complete
   the chain: `loop_runtime.rs` instantiates `OnlineDMD`, updates per tick with real field
   samples, calls `InstrumentPanel::report`; route the K-band alarm into an EXISTING branch.
   Write the round-trip RED test first; run both adversarials (out-of-band alarm,
   frozen-update teeth). Do not add DMD features. Acceptance: §5 P32c rows 1-3.
3. **T3 (P32d, independent lane).** Write the design note (§3.4's fixed names; decorrelation
   as a typed constraint; advisory-only). Then the minimal impl: 2 judges over the governor-
   tick pilot via the existing `llm-adapters` LlmBackend (§0 row 9) — no new provider
   machinery. Land the corrupted-output RED test + both adversarial probes. Acceptance: §5
   P32d rows 1-3.
4. **T4 (close-out).** Append the three REGRESSION-LEDGER rows (§5); update §10.5.1's P32
   statuses; hand any BP-03 status evidence encountered (§0 row 6) to P33b's table rather than
   classifying it yourself. Push after every milestone; fetch before every push, never force.

**Stop-and-flag conditions:** (i) any P32a organ diff (incl. `default_legacy()`);
(ii) resonator/OnlineDMD internals changed (wiring-only phases); (iii) critic output consumed
by any deterministic gate; (iv) a §0 row failing re-verification; (v) the P32b "no loop
benefits" finding — file it, do not force a consumer into existence.
