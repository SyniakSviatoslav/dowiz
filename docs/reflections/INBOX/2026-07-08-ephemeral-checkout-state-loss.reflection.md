CONTEXT:   Daily plane-maintainer run (run-20260708T0603). SENSE/calibration step: resolve
           yesterday's open predictions (e0b456576c07, 1917c9a68a08 from run-20260707T0603),
           then run `new-dep-scan.mjs`. HEAL step: check the staging-deploy precondition
           (fly.io/telegram egress), which has now failed identically 3 days running.
DECISIONS: (1) `plane-telemetry.mjs resolve` failed "prediction not found" on both carried-over
           predictions — `loops/runs/predictions.jsonl` is gitignored/local-only, and this cloud
           session is a fresh checkout with no local state from yesterday's session. Worked
           around by `git show origin/telemetry/plane:telemetry/predictions.jsonl >
           loops/runs/predictions.jsonl` to hydrate the local file from the durable branch
           before resolving (safe: publish's mergeContent() dedups by exact line, so hydrating
           then publishing again is a no-op union, not a duplicate). (2) `new-dep-scan.mjs`
           showed the exact same symptom — "no baseline yet, 76 deps" — for the same root cause
           (baseline also lives under gitignored `loops/runs/`), already diagnosed and fixed in
           unmerged PR #14; did not re-fix, just re-bumped per the charter step and left it to
           #14. (3) fly.io + api.telegram.org 403 recurred a 3rd consecutive day (predicted and
           confirmed hit each time) — crossed the charter's N=3 loop-budget line, escalated via
           issue #19 (proof-first + own-stake) instead of re-logging it inside an unrelated PR
           body for a 4th time.
WHERE:     scripts/plane-telemetry.mjs (PREDICTIONS_PATH → loops/runs/predictions.jsonl,
           gitignored); scripts/new-dep-scan.mjs (same loops/runs/ pattern, PR #14 unmerged);
           docs/governance/plane-maintainer-agent.md (N=3 loop-budget clause); issue #19.
WHY:       One structural root explains two separate-looking symptoms: every piece of the
           maintainer agent's own working state (`predictions.jsonl`, dep-baseline) is written
           to `loops/runs/`, which is gitignored by design (it's meant to be ephemeral scratch)
           — but the *cloud* runtime is itself ephemeral per-session (fresh checkout, no
           persistent volume), so anything that should survive across daily firings has
           exactly one durable home: the `telemetry/plane` git branch (or another committed
           path). `predictions.jsonl` already has a publish path to that branch; the dep-
           baseline (PR #14, per yesterday's ledger) apparently doesn't yet, or isn't merged.
           The pattern is: local-only state + ephemeral runtime = state loss every single day,
           silently, unless something explicitly re-hydrates from the durable branch at the
           top of each run. `resolve` and `new-dep-scan` both assume local state persists
           across invocations; on this runtime that assumption is false every time.
CONFIDENCE: high
NEXT-TIME: (a) `plane-telemetry.mjs resolve`/`predict` should auto-fetch+hydrate
           `loops/runs/predictions.jsonl` from `origin/telemetry/plane` at the start of the
           command (mirroring what `inbox`/`digest`/`query` already do via `fetchBranch()`)
           instead of requiring the caller to know to do it manually — this is a recurring
           (3rd time observed) gap, a guardrail/code-fix candidate, not just a lesson to
           remember. (b) Once merged, confirm PR #14 gives `new-dep-scan.mjs` the same
           branch-durable baseline story so it stops reporting "no baseline yet" every fresh
           checkout. (c) The N=3 egress escalation (issue #19) is a recurrent doubt per the
           charter's doubt-escalation ladder — if it recurs past today unresolved, the next
           step per the ladder is Triadic Council / stronger-model escalation, not a 4th issue.
LINK:      docs/governance/plane-maintainer-agent.md ; docs/governance/model-calibration.md §3 ;
           scripts/plane-telemetry.mjs ; scripts/new-dep-scan.mjs ; issue #19 ; PR #14 ;
           [[memory-corpus-meta-patterns-2026-07-02]]
