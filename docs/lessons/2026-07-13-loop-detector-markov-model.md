---
TRIGGER: tools/loop-signals/**
CAUSE: >
  A per-error-signature scalar counter (loop-detector.sh's original N=3 rule) models the WRONG
  object. "Stuck" is a property of the DYNAMICS of the tool-event sequence (a recurrent orbit that
  never reaches a progress state), not of any single repeated error signature. Two blind spots
  followed: a limit cycle across ≥2 distinct signatures (each counter keeps resetting before it
  hits N=3), and high-entropy churn that never repeats a signature but also never reaches a green
  run. A second, separate defect: counting every successful Bash call as "progress" let benign
  reads (ls/cat/grep) inflate the escape signal and mask real churn.
ACTION: >
  When adding or changing a stuck/loop-detection signal under tools/loop-signals/** (or its wiring
  into .claude/hooks/loop-detector.sh) → if the failure mode you're catching is a SEQUENCE (a limit
  cycle spanning ≥2 signatures, or non-progressing churn) rather than one repeated event, model the
  tool-event stream as a process (Markov chain / recurrent class / entropy rate), not a
  per-signature scalar counter. Classify each event as progress vs. probe before counting it toward
  an escape/occupancy signal (ls/cat/grep are probes, not progress) — treating every successful
  call as progress hides churn. New signals stay ADVISORY: they feed the existing N=3 counter, they
  never replace or override it.
LINK: tools/loop-signals/markov_attractor.py ; tools/loop-signals/test_markov_attractor.py ; .claude/hooks/loop-detector.sh ; docs/regressions/REGRESSION-LEDGER.md row 18 (commit a6a299b4)
SCOPE: Stuck/loop-detection signal design under tools/loop-signals/** and its wiring into loop-detector.sh ONLY. Not other detectors.
STATUS: active
---

# Loop-stuck detection needs a process model, not a per-signature counter

Source: reflection `docs/reflections/ARCHIVE/2026-07-13-markov-attractor-loop-signal.reflection.md`
(REGRESSION-LEDGER row 18).

`loop-detector.sh` originally counted only CONSECUTIVE failures on ONE identical error signature
(N=3) — a 0th-order model with two blind spots: a limit cycle bouncing between ≥2 distinct
signatures (each counter keeps resetting before it reaches N=3), and high-entropy churn that never
repeats a signature but also never reaches a green run ("busy, going nowhere"). Both waste agent
budget on a dead path while looking, to a per-signature counter, like nothing is wrong.

The fix (`tools/loop-signals/markov_attractor.py`) models the tool-outcome stream as a first-order
Markov chain — transition matrix, stationary distribution, entropy rate, escape-mass (occupancy of
a progress state) — and wires the result ADVISORY into `.claude/hooks/loop-detector.sh` (fail-open,
augments rather than replaces the N=3 counter). It also fixed a second defect: the original
"progress" signal counted every successful Bash call, so benign reads (ls/cat/grep) could mask
non-progressing churn; the model now separates progress events from probe events. Proof: 8/8 tests
red on limit-cycle/churn fixtures, green on healthy rhythms.

The regression itself is already fully guardrailed (ledger row 18 — the implementation has its own
red→green test suite). This lesson exists as a forward-looking, pre-edit pointer for the NEXT
person who touches `tools/loop-signals/**`: when a detector "counts," ask what OBJECT it models — a
scalar counter often can't see a structural/temporal pattern. Reach for a process-level model when
the failure mode is a sequence, not a single repeated event. (Precedent for a lesson coexisting
with its guardrail: REGRESSION-LEDGER row 13, the CSS-comment `*/`-in-prose class — its
`e2e/tests/paper-skin-tokens.spec.ts` guardrail and pre-edit lesson stood side by side until the
lesson's trigger path was removed by the 2026-07-15 JS/TS drop.)
