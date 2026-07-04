# Plane status — 2026-07-04

🟢 **PASS** · generated 2026-07-04T06:03:14.240Z by `scripts/plane-report.mjs`
Charter & autonomy envelope: [plane-maintainer-agent.md](./plane-maintainer-agent.md)
Telemetry: telegram=none · push=none · run_id=plane-2026-07-04T06-03-00Z

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
| ⚠️ | inbox-drain-liveness | INBOX drained (≤ 3 files) | 7 reflection file(s) un-curated (oldest 0.0d) — librarian backlog (max 3; soft by design) |
| ✅ | scout-liveness | scout cursors fresh (< 7d) | no scout scripts installed — N/A |
| ✅ | health-pass-freshness | newest agent-health < 7d | newest health pass 0.00d old via docs/governance/agent-health-2026-07-02.md |
| ✅ | advisory-forever | prediction ledger / plane-events never wired as gate input | enumerated surface clean (42 files under 7 versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2) |
| ✅ | ingestion-authority | inbox output never piped into exec/auto-apply | no inbox→exec/auto-apply coupling on the enumerated surface (42 files) — friction + review-forcing, not impossibility |


## Harness health (advisory — `agent-health-pass`)
- ⚠️ only 6 telemetry rows for 19 registered loops — most loop runs still bypass finalize.

## Net-new for the plane (research / OSS scout)
- **Dep baseline seeded.** `node scripts/new-dep-scan.mjs` found no `loops/runs/dep-baseline.json` yet
  (first-ever run of this script in this checkout) — 76 deps in tree, nothing to diff against. Ran
  `--bump` to record the baseline; future firings will get real newcomer diffs. Nothing to
  reverse-engineer this run (seeding, not a newcomer signal).
- **`TOOLING-REGISTRY.md` "Parked (with triggers)" check** — Headroom · Mem0/OpenMemory · Airweave ·
  Octogent · Pake. No trigger fired:
  - The one *documented* trigger (LLM wiki-gen backfill, §"Deferred/follow-ups") needs a non-rate-limited
    paid OpenRouter/BYOK lane; this checkout has no such lane configured — unchanged, still parked.
  - Mem0: worth a note, not a trigger — `mem0ai` (the SDK, `apps/api/src/lib/memory.ts`) is already a
    long-standing direct dependency (server-side agent memory via Ollama), separate from "OpenMemory"
    the parked item (the standalone MCP memory server / browser extension). 2026 upstream: Mem0 shipped
    a token-efficient memory algorithm (hierarchical extraction, +23–30pt on temporal/multi-hop recall)
    and an OpenMemory Chrome extension — advisory only, would need the privacy-gate ingest contract
    (§2.2) before any adoption since it touches user-authored text.
  - Airweave: actively maintained (5–10 new connectors/month per upstream), still pre-privacy-gate —
    no change to parked status.
  - Octogent / Pake / Headroom: no material 2026 signal found this pass.

## Actions taken this run
- Ran `pnpm install` (node_modules was missing in this checkout) so `verify:all --ci` could run at all —
  surfaced as net-new/updated deps in the lockfile diff (`maplibre-gl`, several `@opentelemetry/*`,
  `@axe-core/playwright`, `@lhci/cli`, `eslint`/`typescript-eslint` majors, others) — not code changes,
  just install-time resolution; flagging so a human notices the lockfile moved.
- `node scripts/plane-report.mjs --github-issue-on-fail` → 🟢 PASS (12/12 hard, 2 soft warns); no
  GitHub issue opened (nothing hard-failed).
- `pnpm verify:all --ci` → ✅ ALL PASSED (soft-advisory notes only: connection-lifecycle flagged 3
  `.connect()` sites — spot-checked all 3, none look like real leaks: `server.ts:228`
  `messageBus.connect()` is a boot-time singleton, `order-persistence.ts:13` is inside a `/** */` doc
  comment (the heuristic matched a comment, not code), `client/status/ws.ts:69` is a browser-side WS
  reconnect method — not touching; migration-ordering 142 warnings, non-blocking, 157 migrations still
  monotonic).
- **No hard fails found → nothing to Heal.** No staging deploy attempted this run (there was nothing to
  fix). `FLY_API_TOKEN` is present in this checkout, for the record, in case a future firing needs it.
- **Telegram channel: correction.** At 06:03Z the `plane-report.mjs` send got `HTTP 403` from the
  egress proxy (`connect_rejected … host: api.telegram.org:443`) — read at the time as a standing org
  policy block and written up as such in a reflection. At 06:12Z the REPORT step's
  `plane-telemetry.mjs send` call to the **same host** succeeded (`sent:chunked`), and the proxy's
  `recentRelayFailures` shows no new entry since 06:03. So this was a **transient** blip, not a
  persistent policy denial — the reflection has been corrected in place rather than left wrong on
  record (see `docs/reflections/INBOX/2026-07-04-telegram-egress-blocked-not-unset.reflection.md` for
  the full correction note). Net effect: telegram digest for this run did go out.
- **Dispatched the `librarian` agent** to drain the reflections INBOX (7 files, over the healthy-backlog
  threshold of 3 per `inbox-drain-liveness`). See its own commit/report for what got promoted vs
  archived vs discarded.
- **Housekeeping note (not actioned):** this checkout's detached `HEAD` is ~250 commits ahead of
  `origin/main` (local-only history, e.g. `c8b2d5a`..`b05b7a5`..older) while the local `main` ref still
  points at `origin/main`'s tip (`7eaf78c`). Out of scope for this agent to reconcile (merging that much
  history to `main` is exactly the kind of irreversible, human-gated action the autonomy envelope
  reserves for a person); flagging so a human can confirm this is the intended state of the cloud
  checkout and not lost/orphaned integration work.
- Predictions recorded for calibration (3): `inbox-drain-liveness` backlog outcome, telegram-egress
  persistence, and detached-HEAD-divergence persistence — see `plane-telemetry.mjs inbox` next firing.
