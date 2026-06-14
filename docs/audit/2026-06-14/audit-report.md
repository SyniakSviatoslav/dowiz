# Audit Report — 2026-06-14

> **Full project verification run.** Metric-core: 5/7 pass (score 0.71). **1 blocker, 3 major, 4 minor, 8 advisory, 6 pass.**
> See `audit-report.json` for structured data.

---

## Metric-Core Gates

| Check | Result | Duration |
|---|---|---|
| `tsc` | ✅ PASS | 24.0s |
| `lint` | ⚠️ FAIL (109 errors, 11433 warnings) | 11.3s |
| `check-money` | ✅ PASS — 0 violations | 0.1s |
| `check-rls` | ✅ PASS — 0 violations | 0.1s |
| `check-contracts` | ✅ PASS (placeholder) | 0.02s |
| `playwright-smoke` | ❌ FAIL — E2E_BASE_URL not set | 3.5s |
| `verify-env` | ✅ PASS | 1.9s |

**Score: 0.71. Gating failed: playwright-smoke (needs staging URL).** Lint is soft gate (pre-existing).

---

## 🔴 Blockers (human gate required)

### F-001: Analytics tables missing RLS — cross-tenant data leak
- **Domain:** RLS / tenant-isolation
- **Evidence:** `packages/db/migrations/1790000000012.ts`
- **Detail:** Migration 1790000000012 creates `analytics_events`, `analytics_abuse_log`, `analytics_cwv` with `location_id` but **zero RLS policies and zero FORCE RLS**. Any role can read any tenant's analytics data. The telemetry endpoint inserts without tenant context.
- **Fix:** Add RLS policies + FORCE RLS to all 3 tables.
- **→ HUMAN GATE: requires migration change, do not auto-fix.**

---

## 🟠 Major

### F-002: Raw PII (phone + name) in Telegram notifications
- **Domain:** Security/PII
- **Evidence:** `apps/api/src/notifications/workers/index.ts:487`, `locales.ts:64`
- **Detail:** Unhashed phone number and full customer name sent to Telegram (third-party) in order notifications. `maskPhone()` helper exists but is bypassed.
- **Fix:** Mask phone with `maskPhone()` in `fetchOrderDetails()`. Omit or mask customer name.
- **→ HUMAN GATE: PII compliance.**

### F-003: Notifications divide price by 100 — wrong totals
- **Domain:** Money/rounding
- **Evidence:** `apps/api/src/notifications/locales.ts:39`
- **Detail:** `(i.price * i.quantity) / 100` — assumes minor-unit pricing. A 500 ALL × 2 order shows as 10.00 instead of 1000 ALL in Telegram/SMS.
- **Fix:** Remove `/100`. Use `(i.price * i.quantity).toFixed(0)` or `formatMoney()`.
- **→ HUMAN GATE: affects production notifications.**

### F-004: money.ts is dead code with @ts-nocheck
- **Domain:** Money/rounding
- **Evidence:** `apps/api/src/lib/money.ts:1`
- **Detail:** Money utility library has @ts-nocheck and zero imports across the codebase. Dead code with suppressed type errors.
- **Fix:** Remove @ts-nocheck, fix types, use the functions or delete the file.

---

## 🟡 Minor

### F-005: auth.ts still has @ts-nocheck (missed in structural sweep)
- **Domain:** Security
- **Evidence:** `apps/api/src/plugins/auth.ts:1`
- **Fix:** Remove @ts-nocheck, fix type errors.

### F-006: IP_HASH_SALT should be required in production
- **Domain:** Security
- **Evidence:** `packages/config/src/index.ts: IP_HASH_SALT: z.string().optional()`
- **Fix:** Change to `z.string().min(1)`.

### F-007: EUR conversion uses floating-point arithmetic
- **Domain:** Money/rounding
- **Evidence:** `packages/shared-types/src/utils.ts:34-36`
- **Fix:** Use scaled-integer (BigInt) arithmetic for EUR conversion.

### F-008: SSR/JSON-LD/WebPush use ad-hoc formatting instead of formatMoney
- **Domain:** Money/rounding
- **Evidence:** `ssr-renderer.ts:233`, `jsonld-builder.ts:76-96`, `webpush.ts:60`
- **Fix:** Replace with `formatMoney(price, 'ALL')`.

---

## ⚪ Advisory

| ID | Domain | Detail | Fix |
|---|---|---|---|
| F-009 | RLS | Operational pool connects as superuser (bypasses RLS) | Create dedicated non-superuser role |
| F-010 | RLS | exchange_rates missing non-tenant documentation | Add comment to migration |
| F-011 | Frontend | Dark mode absent from entire app (1 rule found) | Add `dark:` variants everywhere |
| F-012 | Frontend | Button/Input missing hover/active/disabled states | Add state classes |
| F-013 | Frontend | StatusBadge uses hardcoded Albanian labels | Replace with `t()` |
| F-014 | i18n | E2E tests use hardcoded English selectors | Use regex for multi-locale |
| F-015 | i18n | `client.recommended` key empty in all 3 locales | Remove or fill |
| F-016 | Security | Turnstile plugin defined but never wired | Register in server.ts |

---

## ✅ Verified Pass (no violations)

| Check | Detail |
|---|---|
| `tsc` | Zero type errors across 12 workspace projects |
| `check-money` | 0 violations — all prices use integer ALL |
| `check-rls` | 0 violations — all 30+ tenant tables have FORCE RLS |
| `verify-env` | All ~100 env vars verified |
| Migration ordering | 87 migrations with consistent epoch prefixes |
| Migration safety | 3 most recent are additive (no destructive DDL) |
| pg-boss v10 | Array callback wrapper correct |
| MessageBus | No silent drops — pool fallback active |
| NOBYPASSRLS | deliveryos_api_user correctly has NOBYPASSRLS |
| `SET LOCAL` tenant context | 19 calls across routes — all with local scope |
| No secrets in code | Zero API keys/tokens in .ts/.tsx files |
| RS256 JWT + Bearer | Auth plugin verifies correctly |
| Embed mode | Implemented: ClientLayout detects `?embed=true` |
| Loading/error states | Consistent across all pages (SkeletonBase + EmptyState) |
| Responsive layout | Dashboard uses sm/md breakpoints correctly |
| i18n key coverage | All 3 locales have matching ~724 keys |

---

## MISSING / Infrastructure Gaps

- **E2E_BASE_URL** — playwright-smoke gate requires staging URL. Cannot run E2E in this environment.
- **contract-map check** — placeholder only. Real check needs to be wired (`npm run check:contracts`).
- **Turnstile plugin** — defined but not wired in server.ts.

---

## Feeding the Loop

- MemPalace reflections written: `docs/audit/2026-06-14/mempalace-reflections.jsonl`
- Held-out candidates: `docs/audit/2026-06-14/held-out-candidates.json`
- Run recorded: `analytics/record-run.mjs`

---

*Generated: 2026-06-14 · Audit scope: full project · 16 findings (1 blocker, 3 major, 4 minor, 8 advisory)*
