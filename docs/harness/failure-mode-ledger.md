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
| 11 | No composite `verify:all` script | `systemic` | 🔴 OPEN | 2026-06-09 | Multiple verify scripts exist but no single entry point |
| 12 | No CI auto-run for Playwright E2E tests | `systemic` | 🔴 OPEN | 2026-06-09 | `playwright.config.ts` has empty `webServer: []` |
| 13 | graphify knowledge graph stale (built pre-move) | `systemic` | 🔴 OPEN | 2026-06-07 | AGENTS.md §5 — graph built at `Documents\delivery\` not `dowiz` |
| 14 | No harness validation script | `systemic` | ⚪ PENDING | 2026-06-09 | No script validates skill router mappings or reference integrity |

## Episodes

_To be populated by Phase A1 (episode store). See `docs/harness/episodes/`._

## Changelog

| Date | Change |
|------|--------|
| 2026-06-09 | Created ledger with 14 entries from audit sweep and recent session |
