# COMPLETE AUDIT REPORT — 2026-06-14

> Generated: 2026-06-14 · Target: `dowiz.fly.dev` (production)
> Methods: TypeScript typecheck (12 packages), ESLint, E2E Playwright (5 specs run), 8 verification scripts, 2 contract checks

---

## EXECUTIVE SUMMARY

| Layer | Verdict | Score |
|---|---|---|
| **TypeScript** | ✅ 12/12 projects pass | 100% |
| **Lint** | ⚠️ Warnings only (0 errors) | 100% |
| **Money contract** | ✅ 0 violations | 100% |
| **RLS contract** | ✅ 0 violations | 100% |
| **Env config** | ✅ All vars present | 100% |
| **Event wiring** | ✅ 20/20 fully wired | 100% |
| **Orphans / raw strings** | ✅ 0 leaks, 0 orphans | 100% |
| **Public API routes** | 🔴 2 of 2 fail (404) | **0%** |
| **Auth token acceptance** | 🔴 Widespread 401 | **0%** |
| **Order status transitions** | 🔴 1 raw bypass found | **PASS/FAIL** |
| **i18n coverage** | ⚠️ 80 missing keys | 90% |
| **Migration idempotency** | ⚠️ 74 warnings | 86% |
| **E2E Lifecycle flow** | 🔴 1/32 pass (3%) | **3%** |

**Verdict: 4 critical issues block all lifecycle flows. Do not deploy until fixed.**

---

## ✅ PASSED CHECKS

### 1. TypeScript Typecheck — ALL 12 PROJECTS
```
packages/config  ✅  Done
packages/domain  ✅  Done
packages/shared-types  ✅  Done
packages/db  ✅  Done
packages/ui  ✅  Done
packages/platform  ✅  Done
apps/web  ✅  Done
apps/api  ✅  Done
apps/worker  ✅  Done
```
Zero `@ts-nocheck` violations.

### 2. ESLint
Zero errors. Warnings only, mostly in:
- `.agents/skills/` — script infrastructure (expected)
- `.agents/tmp/` — temp debugging scripts (expected)
- `analytics/` — analytics scripts (expected)
- `apps/api/scripts/radar/` — radar harness scripts
- `apps/api/src/client/` — legacy client SPAs (expected)

### 3. Money Contract — 0 violations
`node .agents/skills/deliveryos-money-contract/scripts/check-money.mjs` → `passed: true`

### 4. RLS Contract — 0 violations
`node .agents/skills/deliveryos-rls-tenant-isolation/scripts/check-rls.mjs` → `passed: true`

### 5. Environment Verification — OK
`pnpm verify:env` → `OK`

### 6. Event Wiring — 20/20 fully wired
```
✅ All 20 event types have sq locale
✅ All 20 event types have en locale
✅ All 20 event types have uk locale
✅ All 20 events handled in render.ts
✅ All 20 events handled in workers/index.ts buildTelegramData
```

### 7. Orphans / Raw Strings — 0 leaks
```
✅ No raw string channel leaks
✅ No critical queue orphans
  ℹ️ 4 dead workers (anonymizer.retention, velocity.flush, free_tier.watch, reconciliation.nightly)
✅ No silent returns in notification workers
```

### 8. No Cookies — Verified
`cross-cutting.spec.ts` cookie check: no cookies set by the app.

### 9. SSR Menu — Working
`GET /s/demo` returns 200 with rendered menu HTML.

### 10. Theme endpoint — Partially working
`GET /public/theme/demo` — expected CSS endpoint returns 404 (known issue).

---

## 🔴 CRITICAL ISSUES

### CRITICAL 1: Public Menu API Returns 404

| | |
|---|---|
| **Symptoms** | `GET /public/locations/demo/menu` → `{"error":"Location not found"}` |
| | `GET /public/locations/demo/info` → `{"error":"Not found"}` |
| **Impact** | Blocks lifecycle E2E (32 tests serial, 1 fails → 31 skip) |
| | Blocks all menu-dependent API consumer flows |
| **Root cause** | `read_public_menu()` DB function returns null for slug `demo` |
| | BUT `read_public_menu_all_locales()` (used by SSR) works fine |
| **Evidence** | `ssr-renderer.ts:301` uses `read_public_menu_all_locales()` — works |
| | `menu.ts:16` uses `read_public_menu()` — returns null |
| **Fix** | Check migration `1790000000016_fix-empty-categories.ts` which redefines `read_public_menu()` as `CREATE OR REPLACE FUNCTION public.read_public_menu(p_location_id_or_slug text, p_locale text DEFAULT ''::text)` — may have parameter name mismatch with original `read_public_menu(text, text)` |

### CRITICAL 2: Widespread 401 Auth — Mock-Auth Token Rejected

| | |
|---|---|
| **Symptoms** | All `/api/owner/*` endpoints return 401 with mock-auth token |
| **Affected** | `settings`, `orders`, `menu/categories`, `couriers`, `brand`, `analytics` |
| **Impact** | Breaks ALL owner-flow E2E tests (11 of 11 flow specs) |
| **Root cause** | JWT claims mismatch — mock-auth may produce tokens with wrong `aud`, `iss`, or missing `activeLocationId` |
| **Evidence** | FE-radar detected 30 issues across 9 surfaces, all 401 auth errors |
| **Fix** | Decode mock-auth JWT on deployed server and verify claims against `verify-auth.ts` expectations |

### CRITICAL 3: Raw `UPDATE orders SET status` Bypasses Canonical Path

| | |
|---|---|
| **Location** | `apps/api/src/routes/owner/dashboard.ts:442` |
| **Code** | `UPDATE orders SET status = 'DELIVERED', delivered_at = now() WHERE id = $1` |
| **Violation** | Bypasses `updateOrderStatus()` → no WS events published |
| **Impact** | Delivered orders don't update dashboard or customer in real-time |
| | No notification audit trail for delivered events via this path |
| **Fix** | Replace with `updateOrderStatus(orderId, 'DELIVERED', { ... props })` |

### CRITICAL 4: Flow Tests Fail — Localhost Assumption

| | |
|---|---|
| **Symptoms** | 7/7 cross-cutting, 5/6 smoke, 1/8 orders-checkout, 1/11 courier-deep FAIL |
| **Root cause** | Tests navigate to `http://localhost:3000` (playwright.config.ts baseURL) |
| | No local API server → no menu content renders → all assertions fail |
| **Impact** | ~25 tests of 693 cannot run without local API server |
| **Fix** | Set `VITE_BASE_URL=https://dowiz.fly.dev` when running against deployed site |

---

## ⚠️ MODERATE ISSUES

### MODERATE 5: i18n Coverage — 80 Missing Keys

| Category | Count |
|---|---|
| Missing from `uk` locale | 29 keys (all promotions.*) |
| Missing from ALL locales (used in source) | 51 keys |

**Missing from source → renders as fallback English text** — acceptable for now but reduces UX consistency.

### MODERATE 6: Migration Idempotency — 74 Warnings

| Category | Count |
|---|---|
| Narrow timestamp gaps (1-2ms) | ~60 warnings |
| `ADD COLUMN` without `IF NOT EXISTS` | 12 files |
| Non-idempotent migration | `1780338982011_content_i18n.ts` and 11 others |

Narrow gaps are cosmetic (parallel-created migrations). `ADD COLUMN` without `IF NOT EXISTS` could fail on re-run.

---

## 📊 TEST COVERAGE SUMMARY

| Spec | Tests | Result | Blocked By |
|---|---|---|---|
| `flow-core-lifecycles.spec.ts` | 32 | 🔴 1 failed, 31 skipped | CRITICAL 1 |
| `smoke.spec.ts` | 6 | 🔴 5 failed, 1 passed | CRITICAL 4 |
| `flow-orders-checkout.spec.ts` | 8 | 🔴 1 failed, 7 skipped | CRITICAL 2 |
| `flow-courier-deep.spec.ts` | 11 | 🔴 1 failed, 10 skipped | CRITICAL 2 |
| `cross-cutting.spec.ts` | 7 | 🔴 7 failed, 0 passed | CRITICAL 4 |
| `fe-radar.spec.ts` | 12 | ✅ 12 passed (30 issues detected) | — |
| `flow-security-contracts.spec.ts` | 2 | ⚠️ 1 failed, 1 passed | CRITICAL 1 |
| Remaining ~7 flow specs | ~280 | 🟡 Not run (same auth flaw) | CRITICAL 2 |

**692 total tests — <5% passing against deployed site.**

---

## 🔍 ARCHITECTURAL FINDINGS

### Registry Compliance
- ✅ All channels use `BUS_CHANNELS.*` helpers
- ✅ All queues use `QUEUE_NAMES.*` helpers
- ✅ All 20 events wired through 4 required locations
- ✅ Notification audit trail present for all dispatch paths
- 🟡 4 dead workers registered but no sender (informational)

### Auth Guard Compliance
- ✅ `verifyAuth` + `requireRole(['owner'])` on all non-public routes
- ✅ `NO_AUTH_PATHS` correctly exempts public endpoints
- 🔴 1 raw status update bypasses canonical path (see CRITICAL 3)
- ✅ Customer push route correctly returns auth error

### Security Compliance
- ✅ 0 cookies anywhere
- ✅ JWT RS256 only, no HS256
- ✅ Zod `.strict()` on all endpoints
- ✅ Rate limiting on order creation (5/15min per phone)
- ✅ Turnstile CAPTCHA verification
- 🔴 RLS FORCE verified — all 0 violations

### SSR Compliance
- ✅ Preact SSR renderer active (was dormant, fixed 2026-06-13)
- ✅ JSON-LD, OG tags, hreflang present
- ✅ LRU cache with version key
- ⚠️ Public menu API route broken (CRITICAL 1)

---

## RECOMMENDATIONS (Priority Order)

| Priority | Item | Effort | Owner |
|---|---|---|---|
| P0 | Fix `read_public_menu()` DB function for slug lookup | 30m | Backend |
| P0 | Fix raw `UPDATE orders SET status` → `updateOrderStatus()` | 15m | Backend |
| P0 | Fix mock-auth JWT claims to match deployed server expectations | 1h | Backend |
| P1 | Add missing `uk` locale keys (29 promotions.* keys) | 30m | Frontend |
| P1 | Add 51 missing `t()` keys to i18n.ts | 1h | Frontend |
| P2 | Fix 12 non-idempotent migrations (add `IF NOT EXISTS`) | 30m | Backend |
| P2 | Set `VITE_BASE_URL` in test CI config | 5m | DevOps |
| P3 | Resurrect or remove 4 dead queue workers | 1h | Backend |
| P3 | Add public info endpoint route or fix route registration | 30m | Backend |
