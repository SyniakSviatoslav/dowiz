# Plane status — 2026-07-02

🔴 **FAIL** · generated 2026-07-02T19:38:23.506Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: telegram=none · push=none · run_id=plane-2026-07-02T19-38-00Z

> Generated with the SENSE-step fix from PR #8 applied locally (not yet merged) — the version
> of `plane-report.mjs` on `main` at this run's start silently rendered "gate output
> unparseable" here instead of the table below; see PR #8 and the note in Actions-taken.

## 11-pattern gate (`plane-guard`)
9/12 hard checks pass · 0 soft warn(s)

| | pattern | check | detail |
|---|---|---|---|
| ❌ | P4 advisory→authority | wired: scripts/guardrail-gate-armament.mjs | guardrail script MISSING |
| ❌ | P5 fix-the-class (ratchet) | wired: scripts/guardrail-ledger-integrity.mjs | guardrail script MISSING |
| ✅ | P6 red-line topology | wired: .claude/hooks/red-line-doubt-gate.sh | present + wired in verify:all |
| ✅ | P7 council-before-code | wired: .claude/hooks/serious-gate.sh | present + wired in verify:all |
| ❌ | P9 subtractive | wired: scripts/guardrail-license.mjs | guardrail script MISSING |
| ✅ | P10 data-sovereignty | wired: scripts/compliance-gate.ts | present + wired in package.json (CI privacy-gate) |
| ✅ | P3 dark-first | launch flags default OFF | all *_ENABLED default false (allow-on: FUNNEL_INGEST_ENABLED) |
| ✅ | P1/P2 verify-artifact | no commit/deploy piped to tail\|head\|grep | no masked-exit-code pipes in tracked scripts |
| ✅ | P8 prod↔staging | migration numbering monotonic | 140 migrations, monotonic |
| ✅ | P11 feedback-contract | autonomy envelope documented | docs/governance/plane-maintainer-agent.md present |
| ✅ | telemetry-liveness | newest telemetry event < 3d | newest event 0.01d old |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface — friction + review-forcing, not impossibility |

### ❌ Hard fails — carry-forward
- **P4 advisory→authority**: guardrail script MISSING
- **P5 fix-the-class (ratchet)**: guardrail script MISSING
- **P9 subtractive**: guardrail script MISSING

Root cause + full detail: [issue #9](https://github.com/SyniakSviatoslav/dowiz/issues/9). Short
version: `verify-all.ts` references 9 guardrail scripts that don't exist on `main`. 6 of them
(including these 3) live on the unmerged `chore/design-system-prune` branch (1327 files / -245k
+72k lines vs. `main` — too large to merge unilaterally); 3 more (`guardrail-ledger-integrity.mjs`,
`loops-registry-sync.mjs`, `agent-health-pass.mjs`) don't exist on any branch despite
`REGRESSION-LEDGER.md` row #48 claiming red→green proof for them. One of the 6
(`guardrail-definer-search-path.mjs`, row #33) is a SECURITY DEFINER / RLS-privilege-escalation
guardrail — red-line adjacent, routed to Council per the charter rather than freehand-merged.

Also observed (not gated, advisory): `verify:event-wiring` (`ci: true` in `verify-all.ts`)
requires a `.env` this fresh checkout doesn't have — its own `--ci` classification says the CI
subset needs "no provisioned env/DB," so this one step contradicts that; `contrast audit` warns
3/7 themes (Royal Gold primary-on-surface below AA-normal, Coral Breeze missing a color def);
`connection lifecycle` flags 2 potential leak sites for manual review. None fixed this run —
out of scope vs. the two findings above, flagged here for visibility only.

## Harness health (advisory — `agent-health-pass`)
- ⚠️ `scripts/agent-health-pass.mjs` does not exist on `main` (see issue #9) — this section always
  silently reads "no warnings" regardless of real harness health, because the missing-script error
  produces no `⚠️ **...**`-shaped output for the digest's regex to find. Not a warning-free harness;
  an unmeasured one. Filed as part of issue #9 rather than a second issue.

## Net-new for the plane (research / OSS scout)
- `node scripts/new-dep-scan.mjs` had no prior baseline (first-ever run of this script) — bumped
  it to the current 73-dep set (`loops/runs/dep-baseline.json`, committed this run so the next
  firing can actually diff against it — previously nothing under `loops/runs/` was git-tracked,
  so every fresh cloud checkout would have re-triggered "no baseline yet" forever).
- `TOOLING-REGISTRY.md` parked-with-trigger list (Headroom · Mem0/OpenMemory · Airweave · Octogent
  · Pake): no trigger condition observed as newly met this cycle. No upstream-release check run
  against adopted deps this cycle (scope: this run's time went to the SENSE root-cause chase
  above) — carry forward to the next firing.

## Actions taken this run
- **Fixed & PR'd**: [`#8`](https://github.com/SyniakSviatoslav/dowiz/pull/8) —
  `scripts/plane-report.mjs`'s `capture()` treated a genuinely-empty `stderr` as falsy and
  substituted `execSync`'s generic error message, corrupting otherwise-valid `plane-guard --json`
  output and masking the hard-fail table above behind "gate output unparseable" on every run
  where the gate actually fails cleanly (i.e. always, given the state below). Red→green unit test
  included. Not deployed to staging — `scripts/` never ships in the runtime image (builder-stage
  only); N/A with reasoning in the PR body.
- **Escalated**: [issue `#9`](https://github.com/SyniakSviatoslav/dowiz/issues/9) — the 9 missing
  guardrail scripts above. Not fixed directly: unmerged-branch scale (1327 files) + one red-line
  RLS-adjacent script + 3 unverifiable ledger claims, all outside the autonomy envelope's
  reversible/in-envelope/non-red-line bar.
- **SCOUT**: `new-dep-scan.mjs --bump` (first baseline, 73 deps) — committed alongside this digest.
- Staging deploy skipped this run: nothing staging-deployable was fixed (the one fix is a
  non-runtime tooling script; the escalated finding needs a human decision, not a deploy).
