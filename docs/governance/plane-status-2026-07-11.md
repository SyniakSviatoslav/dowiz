# Plane status — 2026-07-11

🟢 **PASS** · generated 2026-07-11T06:13:33.438Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: telegram=none · push=ok · run_id=plane-2026-07-11T06-13-00Z

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
| ⚠️ | prediction-resolution-liveness | predictions resolved (backlog ≤ 0) | 1/1 prediction(s) unresolved (oldest 0.0d) — the resolve half never ran (backlog>0; soft by design) |
| ⚠️ | inbox-drain-liveness | INBOX drained (≤ 3 files) | 8 reflection file(s) un-curated (oldest 0.3d) — librarian backlog (max 3; soft by design) |
| ✅ | scout-liveness | scout cursors fresh (< 7d) | no scout scripts installed — N/A |
| ✅ | health-pass-freshness | newest agent-health < 7d | newest health pass 0.26d old via docs/governance/agent-health-2026-07-02.md |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean (58 files under 7 versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface (58 files) — friction + review-forcing, not impossibility |


## Harness health (advisory — `agent-health-pass`)
- ⚠️ only 6 telemetry rows for 19 registered loops — most loop runs still bypass finalize.

## Net-new for the plane (research / OSS scout)
- `new-dep-scan.mjs` had no baseline yet (first run since the scanner was built) — bumped it
  (76 deps recorded). No newcomers to reverse-engineer this run; future runs will report actual
  diffs against this baseline.
- No `TOOLING-REGISTRY.md` "Parked (with triggers)" entry (Headroom / Mem0-OpenMemory / Airweave
  / Octogent / Pake) has a fired trigger this run — not re-researched today (advisory, no
  adoption action).
- `pnpm outdated`: all dev-tooling only, no production-dep upgrades pending. Notable major-version
  bumps sitting available (advisory only, no adoption decision made): `typescript` 5.9.3→7.0.2,
  `eslint` 9.39.4→10.7.0, `lint-staged` 15.5.2→17.0.8, `@types/node` 22→26.
- `pnpm audit --prod`: 2 **moderate** advisories, both transitive, both un-actioned this run
  (see escalation below) — `uuid@9.0.1` (<11.1.1 vulnerable) via `apps/api > mem0ai@3.0.7 > uuid`;
  `@opentelemetry/core@1.30.1` (<2.8.0 vulnerable, 28 paths) via `apps/api > @sentry/node`.

## Actions taken this run
- **Fixed (staging-independent, governance-plane only):** `scripts/plane-guard.mjs` gained a new
  `channel-liveness` soft check (`--staging`-gated) that deterministically surfaces when
  `api.telegram.org` / `api.fly.io` egress is blocked, instead of each run silently rediscovering
  it by hand. Root cause: this cloud checkout's network policy denies both hosts at the CONNECT
  layer (confirmed via `$HTTPS_PROXY/__agentproxy/status`) — recurred silently across
  `run-20260707T0603` (telegram) and `run-20260710T0603` (fly.io) before this run named it.
  Red→green proven (`git stash` → 0 warn rows; restored → 2 named `⚠️` rows). Ledger #57.
  `pnpm verify:all --ci` re-run clean after the change (ALL PASSED). Reflection filed:
  `docs/reflections/INBOX/2026-07-11-channel-liveness-network-policy-block.reflection.md`.
  Committed on `plane-maintainer/channel-liveness-guard-20260711` (feature branch, not `main`);
  PR opened for operator review.
- **Escalated, not fixed (out of this run's autonomy envelope):**
  1. **HEAL/staging-deploy is unavailable from this cloud checkout.** `flyctl` is not installed
     and `api.fly.io` is network-policy-blocked (HTTP 403 at CONNECT) — even with
     `FLY_API_TOKEN`/`STAGING_DATABASE_URL` present in env, there is no path to
     `flyctl deploy -a dowiz-staging` or `flyctl proxy` from here. No product-code fix was
     attempted this run for exactly this reason (verify:all/plane-guard both PASS — nothing
     needed the deploy step regardless). **Operator action needed** if HEAL capability is
     wanted from this env: allowlist `api.fly.io` (+ `api.telegram.org` for the Telegram
     channel) in the environment's network policy, or run the maintainer routine from an
     environment that already has that egress.
  2. **2 moderate `pnpm audit` findings** (`uuid`, `@opentelemetry/core` — see above) were
     **not** patched this run. Both are transitive, both are production-reachable, and a
     `pnpm overrides` bump would be a dependency change to `apps/api`'s production tree —
     per Ship Discipline that requires commit→staging-deploy→Playwright-validate to count as
     "done," and the staging deploy is exactly the capability blocked above. Fixing it
     half (commit-only, no deploy proof) would violate the Mandatory Proof Rule. Left for a
     run where staging deploy is reachable, or for direct operator action.
  3. **Telegram push failed** (`HTTP 403`, same network-policy block) — this digest's Telegram
     one-line verdict did not reach the operator's phone this run. The committed digest (this
     file) is the channel of record.
  4. **GitHub reports 8 Dependabot alerts on `main`** (1 high, 3 moderate, 4 low) — wider than
     the 2 moderate `pnpm audit --prod` surfaced (likely includes dev-dep paths). Not enumerated
     or actioned this run (no Dependabot-alert-listing tool available in this session) — see
     `https://github.com/SyniakSviatoslav/dowiz/security/dependabot` directly.

## PR
[#23 — plane-guard: add channel-liveness soft check for fly.io/telegram egress](https://github.com/SyniakSviatoslav/dowiz/pull/23) (draft; subscribed for CI/review follow-up; a ~1h check-in is scheduled)
