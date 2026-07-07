# Verified-by-Math (VbM) — universal validation rule (operator standing rule, 2026-07-07)

> Governing rule, non-negotiable, applies to EVERY task, agent, and lane. Extends the Mandatory Proof
> Rule and §0·GP (ground truth over proxy). Operator directive: *"only verified with math is truly
> validated, no reliance on false/positive metrics."*

## The three questions (ask them of every "done")

A change/feature/fix/measurement is **validated** only when all three hold:

1. **Does it work?** — the behaviour is exercised end-to-end against reality (the real DOM, the real
   bytes, the real running service), not asserted in prose. This is the *ground-truth* leg.
2. **Can it be proven with math?** — there is a *deterministic* artifact that decides pass/fail: an
   assertion, a count, a precision/recall number against a hand-derived oracle, a red→green guardrail.
   "Math" here = a reproducible computation with a defined threshold, not a vibe or a screenshot.
3. **Can the proof (the math / the telemetry) be falsified?** — there exists an input under which the
   proof FAILS, and that failure is observable. A test that cannot go red, a metric that reads "green"
   whether or not the thing works, telemetry that cannot distinguish success from failure — these are
   **false-positive metrics** and do not count as validation. Every proof must carry its own red case.

If any leg is missing, the task is **NOT validated** — regardless of how confident, how reviewed, or
how many green checks it shows.

## Why (the failure this prevents)

The harness has repeatedly been bitten by proofs that could not fail: a gate registered but disarmed
(400+ blind ALLOWs), a `body.length > N` render "proof" that passes on an error page, CERTIFIED loops
whose reports never existed, advisory metrics with no red path. Each was *green* and *worthless*. A
proof's value is exactly its ability to go red on a real defect. Falsifiability is not a nicety — it
is the definition of a proof.

## Practically (what this means when you build)

- **Guardrails/armaments**: ship a RED case (the input that must trip it) *and* a GREEN over-block
  guard (the legitimate neighbour that must NOT trip). The RED case is the falsifiability proof.
- **Behaviour changes**: a programmatic assertion that fails when the code is wrong (Mandatory Proof
  Rule) — and demonstrate it failing on the pre-fix code (red) before it passes (green).
- **Telemetry / probes / comparison reports**: define the metric's failure threshold and show the
  metric moving — a before/after with a number that *could* have been worse. A report that can only
  say "better" is not falsifiable; publish the raw counts and the oracle.
- **Retrieval / ranking / activation**: score against a hand-derived oracle (precision/recall), and
  include queries the system is expected to MISS, so a spurious 100% is impossible.

## Deterministic enforcement

`scripts/guardrail-falsifiable-proof.mjs` (in `run-armaments.sh`, so it runs in pre-commit) parses
`run-armaments.sh` for every proof the enforced suite relies on and asserts each is falsifiable:

- it has a reachable failure path (`process.exit(1|2)`) — it *can* fail; and
- if it is an armament (uses `check()` / `--self-test`), it asserts at least one FAILURE outcome — it
  is not an all-green tautology.

The meta-armament is itself falsifiable: `--self-test` proves it FLAGS a synthetic all-green proof and
a no-failure-path proof, and PASSES a red+green armament and a live-invariant guardrail. (A meta-gate
that could not fail would violate its own rule.)

This is the retrieval/telemetry dual of the same rule: the living-knowledge eval
(`spikes/living-knowledge/`) scores against an oracle that includes expected-misses, and the
comparison probe publishes raw before/after counts — so both are falsifiable, not vanity green.
