# ERROR-RETRO: Error Cluster Analysis

> Generated: 2026-06-11 · Source: mempalace (dowiz wing) + git log + failure-mode-ledger + docs/harness/episodes/
> Method: cluster by root cause, not symptom. Prioritize by frequency × impact.

---

## Cluster table

| ID | Root cause | Symptom examples (`n`=incidents) | Frequency | Impact | Existing protection | Mechanically catchable? |
|---|---|---|---|---|---|---|
| **C1** | API contract not validated against live response | `attributes.kcal`←`attributes.bom[].kcal` (n=1); slug from name←API slug (n=1); snake_case←camelCase on request (n=2) | 4+ | Silent wrong render | §13.1 textual rule | ✅ — automated contract test per endpoint |
| **C2** | Method/interface assumed from name, not verified | `storage.upload()`→`put()` in spa-proxy.ts + themes.ts (n=2); StorageProvider method list not read | 2 | 500 on upload | §13.1 textual rule | ✅ — grep caller vs interface + type-level check |
| **C3** | Env var typos / missing from deploy | `GROQ_API_KE`←`GROQ_API_KEY` on Fly (n=1); `COURIER_PII_ENCRYPTION_KEY` missing at startup (n=1); health 503 from Telegram bot token issues (n=1) | 3 | Production outage | `verify:env` script exists but NOT in CI | ✅ — add verify:env to CI pipeline |
| **C4** | Migration ordering drift / idempotency | Out-of-order migrations blocking `migrate:up` (n=2 in 2026-06-11); column already existing after runtime workaround (n=1) | 3+ | Deploy blocked | None | ✅ — script checks migration TSort + idempotent DDL |
| **C5** | Test anti-patterns (permissive assertions) | `expect([200,201,400,500]).toContain(status)` (n=6); `waitForTimeout` (n=3); auth-only without body validation (n=4) | 13+ | Tests pass even when code is broken | §13.2 textual rules only | ✅ — ESLint rule `no-permissive-status` |
| **C6** | CI pipeline missing verification gates | E2E Playwright not in CI (n=∞); verify:env/verify:rls/verify:secrets not in CI (n=∞) | Ongoing | Bugs reach production | None | ✅ — add steps to CI |
| **C7** | `@ts-nocheck` hiding type errors | `health.ts` had `@ts-nocheck` hiding Telegram API type mismatch (n=1); route files with suppressed errors (n=2) | 3 | Silent type drift | ESLint `no-ts-nocheck` at warn level | ✅ — promote to error or add CI gate |
| **C8** | Route/middleware misconfig (CLOSED) | Double-prefixed routes (n=2); duplicate settings routes (n=1); subdomain asset 404 (n=1) | 4 | 404/500 on routes | research-first.md protocol | ✅ — ESLint route-prefix rule exists (require-auth-hook) |
| **C9** | Dependency version mismatch crash | fastify-type-provider-zod v6 with Zod v3 (n=1); pdf-parse v2 native deps (n=1); pdfjs dynamic import in esbuild (n=1) | 3 | Server crash at startup | None | ⚠️ Partial — verify:env at startup |
| **C10** | No composite verification script | Multiple `verify:*` scripts, no single `verify:all` (n=continuous) | Ongoing | Papercut, skipped checks | None | ✅ — add `verify:all` script |

---

## Priority ranking

| Rank | ID | Rationale | Target guard level |
|---|---|---|---|
| 1 | **C6** | CI gates block deployment of broken code at the merge point. Without this, all other guards are advisory. | 4 — CI gate |
| 2 | **C5** | 13+ anti-pattern instances make tests unreliable. Permissive tests hide real regressions. | 2 — ESLint rule |
| 3 | **C3** | Env var typos cause production outages. verify:env exists but has no mechanical enforcement. | 4 — CI gate |
| 4 | **C4** | Migration ordering breaks deploys with no warning. Easy to catch pre-merge. | 3 — script + CI gate |
| 5 | **C1/C2** | Contract + interface mismatches cause silent wrong behavior. §13.1 rules exist but no mechanical guard. | 3 — E2E contract test |
| 6 | **C7** | `@ts-nocheck` neutralizes TypeScript. ESLint rule exists at warn; promote to CI error. | 2 — ESLint rule (promote) |
| 7 | **C10** | No `verify:all` means verify scripts get skipped. | 3 — script |
| 8 | **C9** | Dependency mismatch — one-off, harder to guard mechanically. | 5 — textual rule add |
| 9 | **C8** | Already CLOSED via research-first.md. No additional guard needed. | — |

---

## Clusters excluded from this round

| Cluster | Reason |
|---|---|
| C8 | Already CLOSED by research-first.md protocol. Existing ESLint `require-auth-hook` covers part. |
| C9 | One-off dependency version mismatches; guard would require runtime dep validation, disproportionate. |
