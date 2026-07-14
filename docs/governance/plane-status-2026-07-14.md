# Plane status — 2026-07-14

🟢 **PASS** · generated 2026-07-14T06:03:49.984Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: telegram=none · push=none · run_id=plane-2026-07-14T06-03-00Z

## 11-pattern gate (`plane-guard`)
12/12 hard checks pass · 2 soft warn(s)

| | pattern | check | detail |
|---|---|---|---|
| ✅ | P4 advisory→authority | wired: scripts/guardrail-gate-armament.mjs | present + wired in verify:all |
| ✅ | P5 fix-the-class (ratchet) | wired: scripts/guardrail-ledger-integrity.mjs | present + wired in verify:all |
| ✅ | P6 red-line topology | wired: .claude/hooks/red-line-doubt-gate.sh | present + wired in verify:all |
| ✅ | P7 council-before-code | wired: .claude/hooks/serious-gate.sh | present + wired in verify:all |
| ✅ | P9 subtractive | wired: scripts/guardrail-license.mjs | present + wired in verify:all |
| ✅ | P10 data-sovereignty | wired: scripts/compliance-gate.ts | present + wired in package.json (CI privacy-gate) |
| ✅ | P3 dark-first | launch flags default OFF | all *_ENABLED default false (allow-on: FUNNEL_INGEST_ENABLED) |
| ✅ | P1/P2 verify-artifact | no commit/deploy piped to tail|head|grep | no masked-exit-code pipes in tracked scripts |
| ✅ | P8 prod↔staging | migration numbering monotonic | 157 migrations, monotonic |
| ✅ | P11 feedback-contract | autonomy envelope documented | docs/governance/plane-maintainer-agent.md present |
| ✅ | telemetry-liveness | newest telemetry event < 3d | newest event 0.00d old via loops/runs/plane-events-2026-07.jsonl |
| ⚠️ | prediction-resolution-liveness | predictions resolved (backlog ≤ 0) | no loops/runs/predictions.jsonl — calibration mirror has a prediction half and no fact half (H3: silence made VISIBLE; soft by design) |
| ⚠️ | inbox-drain-liveness | INBOX drained (≤ 3 files) | 7 reflection file(s) un-curated (oldest 3.3d) — librarian backlog (max 3; soft by design) |
| ✅ | scout-liveness | scout cursors fresh (< 7d) | no scout scripts installed — N/A |
| ✅ | health-pass-freshness | newest agent-health < 7d | newest health pass 3.25d old via docs/governance/agent-health-2026-07-02.md |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean (42 files under 7 versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface (42 files) — friction + review-forcing, not impossibility |


## Harness health (advisory — `agent-health-pass`)
- ⚠️ only 6 telemetry rows for 19 registered loops — most loop runs still bypass finalize.

## Net-new for the plane (research / OSS scout)
- `node scripts/new-dep-scan.mjs` had no baseline yet (first run of this script to actually
  succeed end-to-end) — 76 deps recorded via `--bump`. No newcomers to reverse-engineer this run
  (nothing to compare against). Future runs will diff against this baseline.
- Checked `TOOLING-REGISTRY.md`'s parked-with-triggers list (Headroom, Mem0/OpenMemory, Airweave,
  Octogent, Pake) — no trigger conditions changed since last review; no action.

## Actions taken this run

**🔴 Escalation — read this first: the PR review queue is the actual bottleneck.**
`list_pull_requests` shows **21 open PRs, 20 of them drafts opened by this exact routine between
2026-06-18 and 2026-07-13 — zero merged in that window.** At least 4 bug classes have TWO
independent duplicate-fix PRs each (the same bug rediscovered, root-caused, tested, and PR'd twice
because the first PR was never reviewed): publish-drops-files (#11/#13), resolve-doesn't-see-
branch (#21/#26), StorageProvider import (#24/#25), dep-baseline persistence (#14/#27). Full
reflection: `docs/reflections/INBOX/2026-07-14-pr-review-bottleneck-and-duplicate-fixes.reflection.md`.
Recommend a batch review session, starting with #25 (1-line fix, `mergeable_state: clean`, unblocks
`main`'s CI which has been red for 3 days) and #13 (governance-script-only, re-verified live today).

**Found + fixed this run (governance-plane only, no product-code touched):**
1. **`main`'s build has been broken for 3 consecutive days** (`f0bd996`, `apps/api/src/bootstrap/
   workers.ts:38` uses `StorageProvider` without importing it). Not new — already has two proven,
   unmerged fix PRs (#24, #25). Re-verified live (`pnpm build` still fails identically on current
   `main`) and posted a re-confirmation + duplicate-PR triage comment on #25.
2. **`scripts/plane-telemetry.mjs`'s `cmdPublish` silently drops perpetual files (e.g.
   `predictions.jsonl`) from the `telemetry/plane` branch tip** when a run publishes without a
   local copy — confirmed LIVE (today's tip `209fc7e` had already lost `predictions.jsonl`, a
   2nd real occurrence of the exact bug PR #13 described on 2026-07-05). Rediscovered
   independently, wrote a red→green regression test, found PR #13 already had the identical fix —
   merged current `main` into its branch (merge commit, no force-push; `guard-bash.sh` correctly
   blocked my first `--force-with-lease` attempt) and pushed the update: 24/24 tests green,
   `verify:all --ci` ALL PASSED. Then used the fixed local checkout to `publish` today's run for
   real, which additionally **recovered the 8 historical predictions lost from the live branch tip**
   (merged with today's 3 new ones — 10 unique predictions now back on `origin/telemetry/plane`).
3. Environment gap (not product code): this fresh cloud container had no `node_modules` and no
   built `packages/config`/other workspace `dist/` output — `pnpm verify:all --ci` and `pnpm
   typecheck`/`pnpm build` both failed until `pnpm install` and `pnpm build` were run once. This is
   the same "ephemeral-container starts with incomplete state" class already reflected on
   2026-07-13; not re-reflected again today since no new guardrail angle emerged.

**Not attempted / explicitly out of scope this run:**
- No staging deploy: no code change in THIS run's own branch is product-surfaced (this digest +
  reflection are docs-only; the telemetry fix lives on PR #13, already governance-script-only and
  proof-carrying without a staging leg per its own PR description).
- Telegram push: `HTTP 403` from `plane-report.mjs`'s internal `send` call this morning (see
  telemetry line above). Not investigated further this run — time-boxed in favor of the PR-backlog
  finding above, which is higher-value. Flagged as a predict target for tomorrow's calibration.
- Did not attempt to resolve the 7 unresolved historical predictions recovered from the branch
  (2026-07-11 through 2026-07-13) — per the 2026-07-13 reflection, most are permanently
  unresolvable under the M1 anti-backdating rule (predicted after that day's first emitted event),
  and fabricating a resolution for the rest without direct memory of that day's actual outcome
  would violate the calibration ledger's honesty requirement. Left unresolved, visible in the
  ledger as-is.
- Did not curate the 7-file reflections INBOX backlog (librarian is triggered, not part of this
  daily loop by default) — noted as a soft warn above, unchanged this run.
