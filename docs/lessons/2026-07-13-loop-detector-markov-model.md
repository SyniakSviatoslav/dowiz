---
TRIGGER: tools/loop-signals/**
CAUSE: >
  A per-error-signature scalar counter (loop-detector.sh's N=3 rule) models the WRONG object.
  "Stuck" is a property of the DYNAMICS of the tool-event sequence (a recurrent orbit that never
  reaches a progress state), not of any single repeated error signature — so a limit cycle across
  ≥2 signatures (each counter keeps resetting) and high-entropy non-progressing churn were both
  invisible to it. A second blindness: counting every successful Bash call as "progress" let
  benign reads (ls/cat/grep) inflate the escape signal and mask real churn.
ACTION: >
  When extending a stuck/loop detector under tools/loop-signals/** (or wiring a new signal into
  .claude/hooks/loop-detector.sh) → if the failure mode is a SEQUENCE (limit cycle across ≥2
  signatures, non-progressing churn) rather than a single repeated event, model the tool-event
  stream as a process (Markov chain / recurrent-class / entropy-rate), not a per-signature scalar
  counter. Classify each event as progress vs probe (ls/cat/grep are probes, not progress) before
  counting it toward an escape/occupancy signal — treating every successful call as progress hides
  churn. New signals stay ADVISORY: feed the existing N=3 counter, never replace or override it.
LINK: tools/loop-signals/markov_attractor.py ; tools/loop-signals/test_markov_attractor.py ; .claude/hooks/loop-detector.sh ; REGRESSION-LEDGER row 18 (commit a6a299b4)
SCOPE: Stuck/loop-detection signal design under tools/loop-signals/** and its wiring into loop-detector.sh ONLY. Not other detectors.
STATUS: active
---

# Loop-stuck detection needs a process model, not a per-signature counter

Source: reflection `docs/reflections/ARCHIVE/2026-07-13-markov-attractor-loop-signal.reflection.md`
(ledger #18).

`loop-detector.sh` originally only counted CONSECUTIVE failures on ONE identical error signature
(N=3). That is a 0th-order model and has two blind spots: a limit cycle bouncing between ≥2
distinct signatures (each counter keeps resetting before it hits N=3) and high-entropy churn that
never repeats a signature but also never reaches a green run ("busy, going nowhere"). Both waste
agent budget on a dead path while looking, to a per-signature counter, like fine.

The fix (`tools/loop-signals/markov_attractor.py`) models the tool-outcome stream as a first-order
Markov chain — transition matrix, stationary distribution, entropy rate, escape-mass (occupancy of
a progress state) — and wires the result ADVISORY into `.claude/hooks/loop-detector.sh` (fail-open,
augments rather than replaces the N=3 counter). It also fixed a second defect: the original
"progress" signal counted every successful Bash call, so benign reads (ls/cat/grep) could mask
non-progressing churn; the model now separates progress events from probe events.

When adding a new stuck/loop signal: ask what OBJECT the detector models. A scalar counter can't
see a structural/temporal pattern — reach for a process-level model when the failure mode is a
sequence, not a single repeated event.
