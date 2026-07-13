# Plane status — 2026-07-13

🟢 **PASS** · generated 2026-07-13T06:09:01.426Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: telegram=none · push=none · run_id=plane-2026-07-13T06-09-00Z

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
| ⚠️ | prediction-resolution-liveness | predictions resolved (backlog ≤ 0) | 3/3 prediction(s) unresolved (oldest 0.0d) — the resolve half never ran (backlog>0; soft by design) |
| ⚠️ | inbox-drain-liveness | INBOX drained (≤ 3 files) | 7 reflection file(s) un-curated (oldest 2.3d) — librarian backlog (max 3; soft by design) |
| ✅ | scout-liveness | scout cursors fresh (< 7d) | no scout scripts installed — N/A |
| ✅ | health-pass-freshness | newest agent-health < 7d | newest health pass 2.25d old via docs/governance/agent-health-2026-07-02.md |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean (58 files under 7 versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface (58 files) — friction + review-forcing, not impossibility |


## Harness health (advisory — `agent-health-pass`)
- ⚠️ only 6 telemetry rows for 19 registered loops — most loop runs still bypass finalize.

## Net-new for the plane (research / OSS scout)
`node scripts/new-dep-scan.mjs` — 76 deps in tree (freshly re-baselined this run, see Actions below),
0 true newcomers vs the baseline. No trigger-matched `TOOLING-REGISTRY.md` candidates surfaced; web
scout skipped this run (time budget spent on the 3 in-envelope fixes below).

## Actions taken this run
**3 draft PRs opened, all in-envelope (scripts/docs + one type-import fix), none touching red-lines:**

1. **[#25](https://github.com/SyniakSviatoslav/dowiz/pull/25) fix(api): restore missing `StorageProvider` import breaking main's build.**
   `apps/api/src/bootstrap/workers.ts` used the type without importing it (added by `f0bd996`,
   2026-07-11, pushed straight to `main` with no PR). CI run
   [29170665863](https://github.com/SyniakSviatoslav/dowiz/actions/runs/29170665863) failed on that
   commit and nobody followed up — **`main`'s `pnpm build`/`pnpm typecheck` were red for 36+ hours**
   before this run found it (while running `pnpm build` to unblock its own `pnpm typecheck`, which
   failed on an unbuilt `packages/config` in this fresh cloud checkout — separate, unrelated cause).
   Proof: `pnpm build` + `pnpm typecheck` both exit 0 across all 13 workspace projects.
2. **[#26](https://github.com/SyniakSviatoslav/dowiz/pull/26) fix(governance): plane-telemetry `resolve` must see branch-only predictions.**
   `cmdResolve` only read the local `loops/runs/predictions.jsonl`, which never survives this
   maintainer's ephemeral container between firings — every `resolve --prediction-id` against a
   prior day's prediction failed. Hit on both 2026-07-11 and 2026-07-12 (silently worked around
   both times, per those runs' own telemetry) before being root-caused and fixed here. Ledger #57 +
   new test, RED→GREEN proven (stash/restore). This PR's own CI inherits #25's build break (not
   caused by this PR — commented on the PR with the explanation).
3. **[#27](https://github.com/SyniakSviatoslav/dowiz/pull/27) fix(governance): `dep-baseline.json` must survive an ephemeral container.**
   Same shape as #26 — `scripts/new-dep-scan.mjs`'s baseline lived at a gitignored path, so SCOUT's
   newcomer-detection has been a permanent no-op in this cloud environment (confirmed 2 consecutive
   runs). `.gitignore` carve-out added (matching the existing `registry.json` exception) + ledger #58
   + new test, RED→GREEN proven. Stacked on #25 (needed for this branch's own pre-commit typecheck
   to pass against currently-broken `main`) — retarget to `main` once #25 merges.

**Calibration:** resolved 1/1 resolvable prediction from today (`verify-all-ci-static-gate`, hit).
Attempted to resolve all 4 outstanding predictions from 2026-07-11/07-12 (the trigger for fix #2
above) — **all 4 are now permanently unresolvable**: those runs called `predict` *after* their first
telemetry emit, so the M1 anti-backdating check correctly refuses them forever. This is itself a
process-ordering finding (not a bug in the resolve fix) — recorded below, not auto-fixed. 2 more
predictions from today (`staging-deploy-egress`, `prediction-resolve-ordering`) remain open for
tomorrow's SENSE step, which — unlike the last 2 runs — called `predict` *before* any other
telemetry emit this run, so they should resolve cleanly.

**Not attempted / escalated:**
- **Staging deploy** (Ship Discipline step 2) — could not run for any of the 3 fixes above.
  `api.fly.io` is unreachable from this cloud container (proxy `CONNECT` 403, `flyctl` binary
  absent), confirmed the 3rd consecutive maintainer run. Both secrets (`FLY_API_TOKEN`,
  `STAGING_DATABASE_URL`) are present — this is a network-egress limitation, not a missing-secret
  one. Build/typecheck/unit-test proof substitutes where possible; PRs are left as drafts pending
  human deploy + Playwright validation.
- **Direct push to `main` bypassing CI/review** — `f0bd996` (root cause of #25) landed on `main`
  despite its own CI run failing. This is a one-instance finding (not yet N≥3 recurrent), so no new
  guardrail was added this run; flagging for human awareness. Branch protection requiring a passing
  CI check before merge to `main` would close this class structurally — recommend as a follow-up,
  not enacted here (GitHub repo settings are outside this agent's autonomy envelope).
- **Librarian backlog** — 7 un-curated reflection files in INBOX (soft warn, unchanged this run;
  librarian curation is a separate triggered agent, not run automatically here).
