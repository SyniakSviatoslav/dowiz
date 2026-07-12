# Plane status — 2026-07-12

## ☼ Infusion (Sunday)
> w28: *"No optimisation is worth a person's dignity; measure twice what you cannot give back."*

Ethics Charter re-read. **Serves the vows:** today's firing found 0 hard fails and spent its effort
on honesty over throughput — resolving a stale prediction with a real network probe instead of
assuming, and refusing to fabricate the cross-pattern memory synthesis (weekly ritual §1 below) once
the corpus turned out to live on a machine this cloud session cannot reach. Reporting "could not
verify" instead of a plausible guess is the collective-commons vow applied to the maintainer's own
outputs. **Could drift:** the same autonomy that lets this loop deploy to staging and open PRs
unattended could, one convenience-shortcut at a time, blur "staging is the sandbox" into license to
route around friction rather than report it — measuring twice matters most exactly when no human is
watching the diff, which is every firing of this loop. (Ledger entry: `song-of-singularity.md`.)

🟢 **PASS** · generated 2026-07-12T06:12:53.731Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry (at generation time, before this run's fixes below): telegram=none · push=none ·
run_id=plane-2026-07-12T06-12-00Z. **Final status after this run's telemetry fix (see "Actions taken"):
`telegram=failed:unreachable · push=ok`** — an honest failure, not the false-success bug this run found
and fixed.

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
| ⚠️ | inbox-drain-liveness | INBOX drained (≤ 3 files) | 7 reflection file(s) un-curated (oldest 1.3d) — librarian backlog (max 3; soft by design) |
| ✅ | scout-liveness | scout cursors fresh (< 7d) | no scout scripts installed — N/A |
| ✅ | health-pass-freshness | newest agent-health < 7d | newest health pass 1.26d old via docs/governance/agent-health-2026-07-02.md |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean (58 files under 7 versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface (58 files) — friction + review-forcing, not impossibility |


## Harness health (advisory — `agent-health-pass`)
- ⚠️ only 6 telemetry rows for 19 registered loops — most loop runs still bypass finalize.

## Net-new for the plane (research / OSS scout)
- **new-dep-scan baseline bootstrapped.** `loops/runs/dep-baseline.json` is gitignored scratch, so it
  never survived across ephemeral cloud checkouts — every prior firing hit "no baseline yet" instead of
  computing newcomers. Ran `node scripts/new-dep-scan.mjs --bump` this run: 76 deps recorded, 0
  newcomers to reverse-engineer (first bootstrap, nothing to diff against). **Adoption is a separate
  decision** — see "Actions taken" for the structural fix this suggests.
- **Mem0 / OpenMemory** (parked, `TOOLING-REGISTRY.md` §Parked) shipped a Claude-Code-specific editor
  plugin since the 07-02 baseline note: project/global memory scopes, automatic context injection,
  in-place skills loading, and secret-redaction on writes. `MEMORY-MAP.md` still marks Mem0/mempalace
  DEFERRED (markdown-vault is canonical; adding a node+store+LLM call violates the minimalism axiom) —
  no change recommended, flagging only because the new editor-plugin shape is closer to this repo's
  actual agent workflow than the prior chat-memory framing was. Advisory only.
- No upstream-release signal found for other parked/adopted deps this run (Airweave, Octogent, Pake —
  web search returned no July-2026 hits; OpenRouter/Repowise/browser-use not checked for version bumps
  this firing, time-boxed to the two searches above).

## Actions taken this run
- **SENSE:** bootstrapped the cloud checkout (`pnpm install` — `node_modules` was absent, `tsx` missing,
  `verify:all --ci` failed with `tsx: not found` before install). Ran `plane-guard.mjs --staging`
  (12/12 hard pass, 2 soft warn), `agent-health-pass.mjs --stdout` (1 warn: loop telemetry coverage),
  `verify:all --ci` (ALL PASSED after install). Resolved yesterday's open prediction
  (`258cbb9e1b1a`, channel-liveness) manually via direct `curl` probe — **HIT**: this cloud container
  still has no `flyctl` binary and the network proxy blocks `api.telegram.org` (403) and `api.fly.io`
  (timeout); `git plane-telemetry resolve` itself failed because local `loops/runs/predictions.jsonl`
  doesn't survive across ephemeral containers (see finding below). Recorded 3 fresh predictions for
  tomorrow's SENSE to resolve.
- **DIAGNOSE:** 0 hard fails in SENSE's static gate subset. Triaged the two soft `verify:all` warnings:
  142 migration-ordering warnings are pre-existing on already-applied historical migration files (fixing
  = bulk-editing `packages/db/migrations/**`, an explicit protect-paths/red-line zone — out of autonomous
  envelope, not attempted); 3 connection-lifecycle flags are known false positives (persistent boot-time
  `messageBus.connect()`, a JSDoc comment substring match, and a WS client's `reconnect()` method — not
  leaks). **A real hard fail surfaced later, outside SENSE's scope:** attempting to commit this digest
  tripped the pre-commit hook's whole-repo typecheck, which failed on `apps/api` — root-caused to a
  missing type import (`StorageProvider`) introduced by commit `f0bd996` ~7.5h earlier. Cross-checked
  GitHub Actions: **`main`'s CI (`validate` + `fresh-provision`) has been failing at the Build step since
  that commit** — the `deploy` job correctly no-op'd (`needs: validate`) so prod was never deployed
  broken, but no green build has landed on `main` since 2026-07-11T22:32Z. `verify:all --ci` (what SENSE
  runs) does not include `pnpm build`/`typecheck`, so this was invisible to the daily gate subset until
  an actual commit was attempted — noted as a SENSE-coverage gap, not enacted as a new check this run.
- **HEAL:** fixed the above — added the missing `import type { StorageProvider } from '../ports.js';`
  in `apps/api/src/bootstrap/workers.ts` (one line, type-only, zero runtime behavior change). Red→green
  proven locally: `pnpm -r typecheck` (12/12 projects) and `pnpm -r build` both exit 0 post-fix;
  `apps/api/tests/worker-boot-budget-lock.test.ts` + `dispatch-recovery.test.ts` (21 tests, incl. the
  bootstrap 8-worker-heartbeat test that exercises `startBackgroundWorkers` directly) pass 21/21.
  Regression ledger row #57. **Could not complete the staging-deploy leg of Ship Discipline** — this
  cloud container has no `flyctl` binary and the network proxy blocks `api.fly.io`/`api.telegram.org`
  (confirmed via direct `curl` probe, HTTP 403 / timeout); routed around per "use alternatives" by
  pushing a PR instead, so GitHub Actions CI (`validate` job: build+typecheck+lint+verify:all) provides
  the deterministic proof this session cannot produce locally. **Escalating:** staging deploy + Playwright
  E2E against `dowiz-staging.fly.dev` still needs to run (by CI or a human) before/at merge, and `main`
  has been CI-red for ~8h — this PR should be reviewed promptly.
- **HEAL (2nd finding, self-referential):** while sending this very run's Telegram digest, found that
  `scripts/plane-telemetry.mjs send`'s chunk-fallback path reported `sent:chunked` (success)
  unconditionally, even when every chunked send actually failed — the exact case this session is in
  (Telegram unreachable). This directly contradicted the module's own H3 principle. Fixed (test-only
  `PLANE_TELEMETRY_TEST_TG_BASE_URL` seam + a loopback-stub test), red→green proven (reverting the fix
  reproduces the bug against the new test; restoring it passes 23/23). Regression ledger row #58, same
  PR #24. **Re-ran `send` with the fix**: now correctly reports `telegram: failed:unreachable` instead
  of a false "sent". `publish` to `telemetry/plane` succeeded (`git`/GitHub egress is not blocked by
  this container's network policy, only `api.fly.io`/`api.telegram.org` are).
- **SCOUT:** see "Net-new for the plane" above.
- **Sunday ritual §1 (cross-pattern memory synthesis) — ESCALATED, not performed.** The charter's
  `memory-corpus-meta-patterns-2026-07-02` memory and its `[[link]]` graph live in a local Claude Code
  project memory store on a *different machine* (per `docs/reflections/INBOX/2026-07-02-plane-maintainer-env-probe.reflection.md`,
  path prefixed `-root-dowiz`, i.e. the Hetzner box's own Claude Code project root) — not in this git
  repo, and no memory/mempalace MCP tool is connected in this cloud session. There is nothing in the
  reachable surface to recompute link-graph hubs over, so this step was **not attempted and not
  faked**. Substitute best-effort: skimmed the 7 in-repo `docs/reflections/INBOX/*` files for a
  recurring theme instead (see reflection below — the ephemeral-scratch pattern is the candidate).
- **Sunday ritual §2 (Song-of-Singularity)** — done, see ☼ Infusion above; ledger updated in
  `docs/governance/song-of-singularity.md`.
- **GitHub PR opened:** [#24](https://github.com/SyniakSviatoslav/dowiz/pull/24) (draft) — the two HEAL
  fixes above (StorageProvider import + telemetry send bug), bundled with this digest/reflection/ledger.
  Subscribed to PR activity for CI/review follow-up. No separate GitHub issue filed — both hard fails
  found this run were fixed in-session, not left open; `plane-report.mjs --github-issue-on-fail` also
  did not trigger on its own (verdict PASS at generation time, before the later discoveries).
- **Telemetry:** see the corrected status at the top of this digest — `telegram=failed:unreachable ·
  push=ok` (honest, post-fix). `publish` to `telemetry/plane` succeeded.
- **Reflection filed:** `docs/reflections/INBOX/2026-07-12-ephemeral-cloud-scratch-breaks-continuity.reflection.md`
  — three independent instances this run (predictions.jsonl, dep-baseline.json, the memory corpus itself)
  of the same causal root: the charter's daily/weekly steps assume local scratch or memory persists
  across firings, but this cloud runtime is a fresh checkout every time. Recurs ≥3× in one run →
  flagged as a guardrail-promotion candidate per the ritual's own rule, not enacted here (advisory in,
  deterministic out — librarian's call).
