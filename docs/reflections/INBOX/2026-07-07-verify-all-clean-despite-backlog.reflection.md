# Reflection: predicted verify:all would show backlog rot; it didn't

## CONTEXT
Daily plane-maintainer run `run-20260707T0603`. Same-run calibration test (predict then
resolve within one firing, not across days): recorded prediction `4e7cc95b71bf` —
"`pnpm verify:all --ci` will surface the same pre-existing lint/type gaps as prior runs
rather than a clean pass, given the 6-PR backlog of unmerged fixes" (confidence 0.55) —
then ran the check in the same SENSE step. Result: exit 0, `✅ ALL PASSED`. Resolved `miss`.

## DECISIONS
- Ran the prediction and its resolution inside the same SENSE step rather than deferring
  to tomorrow, to get one clean same-day calibration data point instead of only carrying
  forward cross-day predictions (which the last 3 runs' misses show are mostly really
  "will a human review my PR" predictions in disguise, per
  `2026-07-06-predictions-assumed-merge-that-never-happened`).
- Did not chase or "fix" the miss — a wrong prediction about system state is exactly what
  the ledger exists to catch; recorded it plainly.

## WHERE
- `docs/governance/plane-status-2026-07-07.md` (digest, this run)
- `loops/runs/predictions.jsonl` (local working copy, materialized from `origin/telemetry/plane`
  tip `e0fb00fe8b19` for this session — see WHY-causal below)

## WHY-causal
The premise conflated two unrelated things: "N PRs are unreviewed/unmerged" and "the gate
that runs against `main`/this checkout is degraded." The 9 open drafts are almost all
**narrow, additive fixes** to `scripts/plane-telemetry.mjs`/dep-scan/digest tooling — none
of them touch `apps/**`/`packages/**` runtime code, migrations, or existing gate logic. An
unmerged PR backlog says nothing about the state of the branch it targets; `verify:all`
checks the tree in front of it, not the review queue behind it. Root cause of the *miss*:
I anchored the prediction on a salient, visible signal (a growing PR count) instead of on
the actual causal chain to the thing being predicted (gate correctness in this checkout).
This is the same class of error as `predictions-jsonl-durability`'s partial and
`inbox-drain-librarian`'s miss from `run-20260705T0602`/`run-20260706T0603` — all three
predicted "my own unmerged work will already have taken effect" or "an unrelated backlog
signal predicts gate health" rather than checking the actual mechanism.

Separately (not a miss, but a friction worth naming): `scripts/plane-telemetry.mjs resolve`
reads only the local `loops/runs/predictions.jsonl` working copy, which does not exist on a
fresh checkout — `predict`/`resolve` assume same-session continuity that an ephemeral daily
container doesn't have. I had to manually `git fetch` the `telemetry/plane` branch and
`git show <tip>:telemetry/predictions.jsonl > loops/runs/predictions.jsonl` before `resolve`
would find yesterday's prediction IDs at all. This is the exact gap PR #13's title names
("publish must not drop tip-only files it never had locally") from the write side; today's
finding is the same gap from the **read** side — `resolve`/`inbox` have no shared "pull the
branch into local scratch" helper, so every fresh session either duplicates this manual
`git show` step or silently fails to resolve anything (which is arguably how 9/15 predictions
are still sitting unresolved per today's `plane-guard` soft warn).

## CONFIDENCE
High on the causal read for the miss itself (directly observed: verify:all output, PR file
lists via `list_pull_requests`). Medium on the resolve/read-side gap being the same root as
the write-side PR #13/#11 — plausible from the code (`cmdResolve` only calls
`readJsonl(PREDICTIONS_PATH())`, no `fetchBranch()`/branch-read call, unlike `cmdInbox`) but
not confirmed by reading #13's/#11's actual diffs.

## NEXT-TIME
- When predicting "will gate X pass," anchor on gate X's actual inputs (the tree, the
  guardrail scripts, prior gate history), not on adjacent-but-unrelated backlog signals like
  PR count or review latency.
- `resolve`/`inbox` for `plane-telemetry.mjs` should share one "materialize local scratch from
  `origin/telemetry/plane` tip" helper so a fresh daily checkout doesn't need a manual
  `git fetch` + `git show` before `resolve` can find anything — candidate ratchet for the
  librarian/ratchet-critic (extends the same fix family as PR #11/#13, read-side rather than
  write-side).

## LINK
- [[memory-corpus-meta-patterns-2026-07-02]]
- `docs/reflections/ARCHIVE/2026-07-06-...` (predictions-assumed-merge-that-never-happened, once archived)
