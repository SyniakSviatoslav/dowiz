# Failure-Mode Ledger

> Priority queue for the harness self-improvement loop (`.agents/rules/harness-self-improvement.md` Phase B).
> Populated by DIAGNOSE output from past episodes. Tag: `systemic` = all models hit, `model-specific` = one model.

## Status key

| Status | Meaning |
|--------|---------|
| ✅ CLOSED | Fixed by a harness edit with deterministic evidence |
| 🔴 OPEN | Root cause identified, not yet fixed |
| ⚪ PENDING | Reported but not yet diagnosed |

---

## Current entries

| # | Failure mode | Tag | Status | First seen | Evidence / artifacts |
|---|---|---|---|---|---|
| 1 | Double-prefixed routes (e.g. `/api/courier/api/courier/...`) | `systemic` | ✅ CLOSED | Pre-2026-06-07 | `.agents/rules/research-first.md` §A (route prefix check) |
| 2 | Duplicate routes/utilities written without checking existing patterns | `systemic` | ✅ CLOSED | Pre-2026-06-07 | `.agents/rules/research-first.md` (full protocol) |
| 3 | Inconsistent auth patterns across similar routes | `systemic` | ✅ CLOSED | Pre-2026-06-07 | `.agents/rules/research-first.md` §A (auth hook check) |
| 4 | Multiple error response formats (`{ error }` vs `{ message }` vs raw string) | `systemic` | ✅ CLOSED | Pre-2026-06-07 | `.agents/rules/research-first.md` §A (error format check) |
| 5 | Subdomain middleware rewriting static assets to `/s/:slug` causing 404 | `systemic` | ✅ CLOSED | 2026-06-09 | `apps/api/src/server.ts:198` — file extension exclusion |
| 6 | Old Preact SSR served instead of new React SPA at `/s/:slug` | `systemic` | ✅ CLOSED | 2026-06-09 | `apps/api/src/routes/public/ssr.ts` — replaced with `reply.sendFile('index.html')` |
| 7 | Zod v3/v4 validator compiler crash on 500s | `systemic` | ✅ CLOSED | 2026-06-09 | Fastify validator/serializer compiler replaced with Zod v3-safe impl |
| 8 | E2E matrix: 73/126 RED (58% failing) | `systemic` | 🔴 OPEN | Pre-2026-06-07 | `e2e/MATRIX.md` — needs systematic remediation |
| 9 | Per-phone order throttle missing (FX-4) | `systemic` | 🔴 OPEN | 2026-06-07 | AGENTS.md §9 item 2 |
| 10 | DB role guardrail — operational pool connects as superuser | `systemic` | 🔴 OPEN | 2026-06-07 | AGENTS.md §9 item 3 |
| 11 | No composite `verify:all` script | `systemic` | ✅ CLOSED | 2026-06-09 | `scripts/verify-all.ts` + `pnpm verify:all` — 2026-06-11 |
| 12 | No CI auto-run for Playwright E2E tests | `systemic` | ✅ CLOSED | 2026-06-09 | `.github/workflows/ci.yml` — post-deploy E2E smoke + regression — 2026-06-11 |
| 13 | graphify knowledge graph stale (built pre-move) | `systemic` | 🔴 OPEN | 2026-06-07 | AGENTS.md §5 — graph built at `Documents\delivery\` not `dowiz` |
| 14 | No harness validation script | `systemic` | ⚪ PENDING | 2026-06-09 | No script validates skill router mappings or reference integrity |
| 15 | Permissive test assertions (`expect([200,400,500]).toContain(x)`) | `systemic` | ✅ CLOSED | 2026-06-11 | ESLint rule `local/no-permissive-status-assertion` |
| 16 | Migration ordering drift (numeric prefix ≠ alpha order) | `systemic` | ✅ CLOSED | 2026-06-11 | `scripts/verify-migrations.ts` — exits 1 on ordering error |
| 17 | No CI verify gates before deploy | `systemic` | ✅ CLOSED | 2026-06-11 | CI validate job now runs `verify:migrations` + `verify:secrets` |
| 18 | Schema-query mismatch — SQL references column that doesn't exist | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern A — verify columns before writing JOINs |
| 19 | Library API drift — assuming installed version matches docs/prior experience | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern B — check installed version's API before use |
| 20 | Incomplete event wiring — missing links in event chain | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern C — verify publisher→handler→locale→render→type |
| 21 | Connection lifecycle leak — connect() without matching close() | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern D — audit connection lifecycle |
| 22 | Resilience gap — no rate-limiter/circuit-breaker/dedup on IPC channels | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern E — every pg-boss send needs singletonKey |
| 23 | Backward compat blindspot — strict validation breaks existing producers | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern F — start lenient, add strict after telemetry |
| 24 | Topology ignorance — assuming all DB connections are equivalent | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern G — maintain port map with pool type |
| 25 | Permission assumption — assuming runtime role has unverified privileges | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern H — verify privileges at startup |
| 26 | Missing pre-flight check — depending on infrastructure that doesn't exist | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern I — verify external deps at startup |
| 27 | Code duplication — repeating same API call pattern 3+ times | `systemic` | 🔴 OPEN | 2026-06-09 | `NX-RETRO.md` §2 Pattern J — extract helper on third repeat |
| 28 | Notification event dedup missing (no singletonKey on send) | `systemic` | ✅ CLOSED | 2026-06-09 | `server.ts:438-445` — dedupKey from event:entity_id:location_id |
| 29 | Missing per-queue explicit creation (createQueue not called) | `systemic` | ✅ CLOSED | 2026-06-09 | `server.ts:260-269` — all 10 queues explicitly created |
| 30 | PgBossQueueProvider.boss field private (blocked external access) | `systemic` | ✅ CLOSED | 2026-06-09 | `queue-provider.ts:18` — changed to public |
| 31 | answerCallbackQuery called after processing (loading spinner) | `systemic` | ✅ CLOSED | 2026-06-09 | `telegram-webhook.ts` — moved to top of action handler |

## Episodes

_To be populated by Phase A1 (episode store). See `docs/harness/episodes/`._

## Changelog

| Date | Change |
|------|--------|
| 2026-06-09 | Created ledger with 14 entries from audit sweep and recent session |
| 2026-06-11 | Closed #11 (verify:all), #12 (CI E2E), #15 (permissive assertion rule), #16 (migration ordering), #17 (CI verify gates). Added #15-17 from retro analysis. |
| 2026-06-12 | NX audit: added #18-31 (10 OPEN failure modes from 10 error patterns + 4 closed). See `docs/harness/retro/NX-RETRO.md` and episode `docs/harness/episodes/2026-06-12--nx-audit.md`. |
