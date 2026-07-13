# Reflection: plane telemetry egress + calibration ledger + closed loop (council → build → bootstrap)

## CONTEXT
Operator directive: (1) expand dowiz-maintainer probe/job reporting into structured, analyzable,
searchable Telegram telemetry; (2) integrate the three-skills-one-cycle model (адаптація/зв'язок/
переконання + prediction-vs-fact meta-principle) into the meta-loop harness as operational
mechanisms; (3) close the loop — maintainer output visible/storable/adjustable locally so its
findings can drive local change. Ran the full Triadic Council (3 rounds to convergence), then a
3-lane parallel build, then lead integration (real branch bootstrap + trigger wiring).

## DECISIONS
- Council before code (serious: third-party egress + systemic harness change). 3 rounds; the
  Breaker round-2 pass was the highest-value step of the session (see WHY).
- Durability pivoted from "commit telemetry to main" to an append-only orphan `telemetry/plane`
  branch written by git plumbing with an in-emitter fail-closed secret-scan — dissolved the
  pre-commit-hook / no-commit-to-main / PR-merge-dependency trilemma in one move.
- All three counsel constraints were encoded as artifacts, not prose: advisory-forever +
  ingestion-authority plane-guard checks (honestly worded "friction, not impossible"),
  uncertainty-first inbox ordering as a stable contract, mirror-never-stick verbatim in the ADR.
- Shipped trigger-prompt update BEFORE the scripts were committed — then caught that the cloud
  checkout reads GitHub, so the whole uncommitted governance batch was invisible to the cloud
  agent; assembling a clean ship branch became mandatory, not optional.

## WHERE
scripts/plane-telemetry.mjs (+.test.mjs), scripts/plane-guard.mjs (3 checks),
scripts/plane-report.mjs (lifecycle events + status line), docs/governance/model-calibration.md,
docs/governance/plane-maintainer-agent.md (6 charter deltas), docs/adr/ADR-plane-telemetry-and-
calibration.md, docs/design/plane-telemetry-principles/*, REGRESSION-LEDGER row 49,
remote: trigger trig_01DgtaGih6VQVRNsKfgKMVBh prompt v2, origin/telemetry/plane bootstrapped
(e080cacc), first prediction e957e06090d8 recorded.

## WHY-causal
Two causal roots surfaced:
1. **The "durable local file" illusion.** The design assumed the writer's filesystem persists;
   the primary writer is an ephemeral cloud box. The root is a category error — conflating
   "written to disk" with "durable" — the same class as verify-artifact-not-proxy: durability is
   a property of the STORE (git remote), not the WRITE. Only an adversarial round with the real
   deployment topology in scope caught it; the architect alone had marked it solved.
2. **The uncommitted-toolchain blind spot.** The trigger prompt referenced scripts by path while
   every referenced script sat uncommitted on this box. Cause: two prior sessions deferred
   committing (collision risk) and the deferral state became invisible background — each later
   step (trigger update) silently assumed the repo the cloud sees == the repo on disk here.
   Remote consumers bind to the REMOTE state; every remote-facing wiring step must verify its
   referenced artifacts exist remotely (same lesson as get-before-update, applied to file paths).

## CONFIDENCE
High on local proofs (15/15 tests incl. red arms, 12/12 hard checks, real publish + inbox
round-trip). Medium on the cloud half until tomorrow's 06:06 run: prediction e957e06090d8
(confidence 0.6) says the first cloud publish succeeds without secret-scan abort or push failure —
resolution of that prediction is itself the first calibration data point.

## NEXT-TIME
- When a design says "durable/local/persistent", ask WHERE the writer runs before accepting it.
- Before any remote-config update that references repo paths (trigger prompts, CI, webhooks):
  `git ls-remote` / check the referenced files exist on the branch the remote consumer reads.
- Deferred commits are debt with a fuse: record WHO unblocks them and re-check at every
  remote-facing step, or the deferral silently poisons downstream wiring.

## LINK
- [[plane-maintainer-agent-2026-07-02]] · [[memory-corpus-meta-patterns-2026-07-02]]
  (verify-artifact-not-proxy, advisory-vs-authority, council-before-code)
- docs/design/plane-telemetry-principles/ (proposal, breaker ROUND 1-3, counsel, resolution)
- docs/adr/ADR-plane-telemetry-and-calibration.md · REGRESSION-LEDGER #49
