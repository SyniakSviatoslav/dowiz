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
           body for a 4th time. (4) At REPORT, `plane-telemetry.mjs send` printed
           `telegram: sent:chunked` — but a direct `curl` to `api.telegram.org:443` immediately
           before and after still returned 403. Traced the false-positive to `cmdSend`'s
           chunked-fallback loop: it discards each `tgApi()` call's boolean result via
           `.catch(() => {})` and hard-sets `status='sent:chunked'` once the loop merely
           *finishes*, not once anything *succeeds*. Verified this exact bug is already fixed in
           unmerged PR #18 (counts `chunksOk`, yields `failed:chunk_send` when zero succeed, adds
           a regression test — ledger #57) — did not duplicate the fix, corrected the digest text
           instead and left #18 to land it.
WHERE:     scripts/plane-telemetry.mjs (PREDICTIONS_PATH → loops/runs/predictions.jsonl,
           gitignored; `cmdSend`'s chunked-fallback loop); scripts/new-dep-scan.mjs (same
           loops/runs/ pattern, PR #14 unmerged); docs/governance/plane-maintainer-agent.md
           (N=3 loop-budget clause); issue #19; PR #18.
WHY:       Two structural roots, not one — and they compound. (A) Every piece of the maintainer
           agent's own working state (`predictions.jsonl`, dep-baseline) is written to
           `loops/runs/`, which is gitignored by design (meant to be ephemeral scratch) — but
           the *cloud* runtime is itself ephemeral per-session (fresh checkout, no persistent
           volume), so anything that should survive across daily firings has exactly one durable
           home: the `telemetry/plane` git branch. `resolve` and `new-dep-scan` both assume
           local state persists across invocations; on this runtime that assumption is false
           every time, silently, unless something explicitly re-hydrates at the top of each run.
           (B) Separately, and more consequentially: **the fixes for both of today's other real
           bugs already exist** — PR #14 (dep-baseline persistence) and PR #18 (telegram
           false-success) are correct, reviewed-by-nobody, and sitting in a 7-PR backlog with
           "zero review activity for days" (today's own confirmed prediction). So the actual
           lever today isn't more code — three consecutive days of maintainer-agent runs have
           already produced correct fixes for these exact recurring symptoms. The system's
           bottleneck is downstream of code: nothing merges, so every fresh cloud checkout
           re-discovers the same already-solved problems from scratch. A daily agent that can
           open PRs but not get them reviewed is structurally capped at "diagnose the same thing
           repeatedly," never "close it."
CONFIDENCE: high
NEXT-TIME: (a) `plane-telemetry.mjs resolve`/`predict` should auto-fetch+hydrate
           `loops/runs/predictions.jsonl` from `origin/telemetry/plane` at the start of the
           command (mirroring what `inbox`/`digest`/`query` already do via `fetchBranch()`)
           instead of requiring the caller to know to do it manually — a recurring (3rd time
           observed) gap, a guardrail/code-fix candidate, not just a lesson to remember.
           (b) The higher-leverage NEXT-TIME is human, not agent-side: PRs #13/#14/#18 are each
           narrow, low-risk, already-proven fixes for bugs this same daily run keeps
           re-diagnosing — merging them would retire 3 recurring findings in one pass. This is
           worth naming explicitly in the digest each day until it happens, not just re-finding
           the same bugs. (c) The N=3 egress escalation (issue #19) is a recurrent doubt per the
           charter's doubt-escalation ladder — if it recurs past today unresolved, the next step
           per the ladder is Triadic Council / stronger-model escalation, not a 4th issue.
           (d) If a 4th distinct "already-fixed-in-an-unmerged-PR" rediscovery happens tomorrow,
           that itself crosses a pattern threshold worth a librarian-curated guardrail proposal:
           the digest should auto-cross-reference open PR titles against today's findings before
           the agent starts re-diagnosing.
LINK:      docs/governance/plane-maintainer-agent.md ; docs/governance/model-calibration.md §3 ;
           scripts/plane-telemetry.mjs ; scripts/new-dep-scan.mjs ; issue #19 ; PR #13 ; PR #14 ;
           PR #18 ; [[memory-corpus-meta-patterns-2026-07-02]]
