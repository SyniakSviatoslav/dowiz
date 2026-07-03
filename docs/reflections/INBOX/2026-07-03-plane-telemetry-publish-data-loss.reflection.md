# Reflection: plane-telemetry publish silently deleted predictions.jsonl on a fresh checkout

## CONTEXT
First real firing of the plane-maintainer routine from a genuinely fresh cloud container (no
`node_modules`, no `loops/runs/*` scratch — matches the "first cloud maintainer run" predictions
already sitting in the inbox from 2026-07-02). SENSE step: ran `node scripts/plane-report.mjs
--github-issue-on-fail`, which internally calls `plane-telemetry.mjs publish`. That publish
(commit `070bc69`) succeeded (no secret-scan abort, no push failure) — but silently rewrote the
`telemetry/plane` branch tree to DROP `telemetry/predictions.jsonl` entirely, because this
checkout had no local `loops/runs/predictions.jsonl` to begin with.

## DECISIONS
- Diagnosed via direct git archaeology (`git ls-tree` before/after the publish commit) rather than
  trusting `plane-guard`'s soft warning ("no loops/runs/predictions.jsonl — … soft by design") —
  that warning is designed to read as *intentional silence* (H3), and it nearly masked a real bug.
- Read `collectPublishFiles()`/`cmdPublish()` source directly: it builds the new branch tree
  **only** from files that exist in **local** scratch, merging branch-vs-local per file it already
  knows about — any branch file with no local counterpart is never even considered, so it's
  dropped by omission on the next push. The code's own comment ("branch content ∪ unseen local
  lines — append-only union") was aspirational, not what the code did for missing files.
- Fixed by adding `remoteTelemetryNames(tip)` (git ls-tree at the remote tip) and unioning it into
  the file set before building blobs — any name that exists remotely but not locally now carries
  forward unchanged.
- Proved red→green: added a test to `plane-telemetry.test.mjs` that publishes a prediction from
  "box A", then publishes an unrelated event from a **fresh** "box B" checkout with no local
  predictions.jsonl, and asserts predictions.jsonl survives. Confirmed it fails on the pre-fix
  code (stashed the fix, ran the test — `predictions.jsonl was DELETED`), then confirmed it
  passes with the fix restored. Full suite 23/23 green.
- Recovered the actual lost data from the last known-good commit (`373ca35`) on `telemetry/plane`
  and republished with the fixed script — verified via a fresh `git fetch` + `git ls-tree` that
  both files now coexist on the real branch, all 4 historical + new predictions present.
- Filed `docs/regressions/REGRESSION-LEDGER.md` #53.

## WHERE
- `scripts/plane-telemetry.mjs` (`collectPublishFiles`, `cmdPublish` — new `remoteTelemetryNames`)
- `scripts/plane-telemetry.test.mjs` (new red→green test)
- `docs/regressions/REGRESSION-LEDGER.md` #53
- `loops/runs/predictions.jsonl` (recovered locally, republished)

## WHY-causal
The root cause is a **local-vs-durable scope mismatch**: the branch is documented and intended as
"the durable record... not the ephemeral box" (charter, telemetry emission section), but the
publish code was written and tested (until today) only from repeat-writer boxes that always had
the full local history already on disk (a persistent Hetzner box, or same-session test fixtures
that write-then-publish in one process). Nobody had exercised publish from a checkout that is
missing a file the branch already has — which is exactly what "the box is ephemeral, the branch is
durable" *guarantees will happen* on every genuinely fresh cloud firing. The existing test suite's
own bootstrap/race tests (R3-7) prove unioning EVENTS across two boxes, but no test covered a
box publishing while genuinely missing a **different file type** (predictions vs events) that a
prior box had already written. Append-only was asserted in prose and in one code comment, but not
covered by a test for the asymmetric-file case — so the gate that should have caught this (the
regression ledger's own "red→green proof required" discipline) had a gap in what it exercised.

## CONFIDENCE
High. Reproduced deterministically (stash/pop the fix, same test, opposite result), and confirmed
against the real remote branch (not just the test fixture) — `git ls-tree` before/after this run's
real publish shows the exact same failure mode and the exact same fix closing it.

## NEXT-TIME
- A soft/advisory plane-guard warning ("no X — soft by design") is a hypothesis about *why* a file
  is missing, not a proof that it's fine — when a warning's stated reason ("silence made visible")
  could equally describe a bug, check the git history of the referenced path before accepting the
  advisory framing.
- Any script that treats "local scratch" and "durable branch" as two write sources to merge must
  be tested from a **partial/fresh** local state, not just from a full-history writer — the
  asymmetric case (branch has it, local doesn't) is exactly the case an ephemeral-box architecture
  guarantees in production and is easy to omit from a test suite built by a persistent-box author.
- **Process note (self-critical):** this run also ran predict-before-sense out of order — I
  resolved yesterday's predictions and recorded today's *after* already running `plane-report`/
  `verify:all` (i.e., after already knowing today's outcomes), which defeats the calibration
  ledger's purpose for today's 3 new predictions (they're hindsight, not foresight). They'll sit
  unresolved by M1's ordering-friction design (their `ts_predicted` is after this run's first
  event) — left as-is rather than fudged. Next-time: call `plane-telemetry predict` for the day's
  targets as the FIRST tool call of the run, before any check that could reveal the answer.

## LINK
- [[plane-maintainer-agent-2026-07-02]]
- [[plane-telemetry-closed-loop-2026-07-02]]
- REGRESSION-LEDGER.md #53
