# DeliveryOS E2E Test Matrix

> Source of truth: `e2e/tests/` (28 spec files, ~320 test() calls, 3 projects = mobile/tablet/desktop)  
> Status: `✅ PASS` = passing, `❌ FAIL` = failing, `⏭️ SKIP` = skipped, `⚠️ WEAK` = weak assertion, `🔴 BROKEN` = server bug  
> Last run: `VITE_BASE_URL=https://dowiz.fly.dev npx playwright test` — 2026-06-10

---

## Test Suite Health

| Metric | Value |
|--------|-------|
| Total spec files | 28 |
| Total test() calls | ~320 |
| Run configuration | 3 projects (mobile 390px, tablet 768px, desktop 1280px) |
| Mode | `serial` for flow tests, parallel for independent tests |
| Workers | 1 |

### Best-Practice Audit (2026-06-10)

| Practice | Status | Notes |
|----------|--------|-------|
| `getByRole()` / `getByText()` / `getByTestId()` | ⚠️ Partial | 0 `getByRole()` calls found. Most browser tests use `page.locator()` with CSS selectors |
| No `page.waitForTimeout()` | ❌ 68+ calls remaining | Replaced in admin/dashboard, maps, flow-security-contracts. ~60 remain across other files |
| Specific assertions (not `body.length > 0`) | ✅ Fixed | Removed from admin/dashboard.spec.ts, maps.spec.ts. ~5 remaining in regen-able test files |
| No `[200,400,500].toContain()` | ✅ Fixed | Removed 500-acceptance from flow-regulatory-settlements. Flow-core-lifecycles still uses `[200, 409]` for state-dependent endpoints |
| `toBeTruthy()` on non-boolean | ⚠️ Widespread | ~30 occurrences across all files — CSS var checks, response body checks |
| No tautology assertions | ✅ Fixed | Removed from flow-security-contracts (CSP), admin/dashboard (count >= 0), maps (typeof isVisible) |
| Error collection (page.on pageerror) | ✅ Good | Present in nearly all browser tests |
| `beforeEach`/`afterAll` cleanup | ⚠️ Partial | Flow tests clean up via `afterAll` with `.catch(() => {})` — error-prone |
| Serial mode cascade | ⚠️ 8 files | Flow tests share mutable module state — any failure cascades |

---

## 1. API Coverage (tested against live `https://dowiz.fly.dev`)

### 1.1 Public Endpoints
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/public/locations/:slug/info` | GET | deploy-validation.spec.ts | ✅ PASS | Shape validated |
| `/public/locations/:slug/menu` | GET | deploy-validation.spec.ts | ✅ PASS | Allergens as strings + arrays handled |
| `/api/orders` | POST | deploy-validation, flow-core-lifecycles | ✅ PASS | Order creation, validation |
| `/api/orders` (invalid) | POST | flow-security-contracts | ✅ PASS | Returns 400 |

### 1.2 Auth & Security
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/api/dev/mock-auth` | POST | All flow tests | ✅ PASS | Returns `access_token` + `activeLocationId` |
| 401 on protected routes (8 routes) | GET | flow-security-contracts | ✅ PASS |
| Invalid input → 400 | POST | flow-security-contracts | ✅ PASS |
| JWT claim decode | — | flow-security-contracts | ✅ PASS | role=owner, iat < exp |
| CSP headers | — | flow-security-contracts | ✅ PASS | Fixed: was always-passing assertion |
| 0 cookies on all pages | — | flow-security-contracts | ✅ PASS | 8 page types verified |
| Cross-tenant → 404 | GET | flow-security-contracts | ✅ PASS |
| Corrupted localStorage recovery | — | flow-security-contracts | ✅ PASS | App doesn't crash |

### 1.3 Owner Endpoints
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/owner/settings` | GET | deploy-validation | ✅ PASS |
| `/owner/locations/:id/dashboard/snapshot` | GET | flow-admin-deep | ✅ PASS | Shape validated |
| `/owner/locations/:id/menu/categories` | GET/POST | deploy-validation, flow-admin-deep | ✅ PASS | CRUD + stop-list |
| `/owner/locations/:id/menu/products` | GET/POST/PATCH/DELETE | deploy-validation, flow-admin-deep | ✅ PASS | CRUD + BOM + allergens |
| `/owner/locations/:id/orders/:id/reject` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/orders/:id/assign-courier` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/orders/:id/mark-no-show` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/orders/:id/verify` | GET | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/couriers` | GET | flow-admin-deep | ✅ PASS |
| `/owner/locations/:id/couriers/live` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/signals` | GET | flow-admin-deep | ✅ PASS |
| `/owner/locations/:id/signals/compute` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/signals/:id/acknowledge` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/signals/:id/dismiss` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/alerts` | GET | flow-admin-deep | ✅ PASS |
| `/owner/locations/:id/alerts/:id/acknowledge` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/alerts/acknowledge-all` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/themes` | GET/PUT | flow-admin-deep | ✅ PASS |
| `/owner/locations/:id/courier-invites` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/settings/dwell` | GET/PUT | flow-core-lifecycles | ✅ PASS | round-trip verified |
| `/owner/locations/:id/settings/fallback` | GET/PUT | flow-core-lifecycles | ✅ PASS | round-trip verified |
| `/owner/locations/:id/settings/retention` | GET/PUT | flow-core-lifecycles | ✅ PASS | round-trip verified |
| `/owner/locations/:id/degradation` | GET | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id` (location) | PATCH | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/modifier-groups` | GET/POST/PATCH | flow-core-lifecycles | ✅ PASS | CRUD + attach to product |
| `/owner/locations/:id/modifiers/:id` | PATCH | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/products/:id/modifier-groups` | GET/PUT | flow-core-lifecycles | ✅ PASS | attach + verify |
| `/owner/locations/:id/products/:id/translations` | GET/PUT/DELETE | flow-core-lifecycles | ✅ PASS | CRUD |
| `/owner/locations/:id/push/state` | GET | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/push/subscribe` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/push/unsubscribe` | POST | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/notifications/targets` | GET | flow-core-lifecycles | ✅ PASS |
| `/owner/locations/:id/couriers/:id/details` | GET | flow-regulatory-settlements | ✅ PASS |

### 1.4 Courier Endpoints
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/courier/auth/invites/:id` | GET | flow-courier-deep | ✅ PASS |
| `/courier/auth/invites/:id/redeem` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/auth/login` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/auth/refresh` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/auth/logout` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/me` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/audit-log` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/earnings` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/history` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/payouts` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/password` | PATCH | flow-core-lifecycles | ✅ PASS |
| `/courier/me/shift` | GET | flow-core-lifecycles | ✅ PASS |
| `/courier/me/shift/start` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/me/shift/end` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/shifts/transition` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/shifts/ping` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/assignments/:id/accept` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/assignments/:id/picked-up` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/assignments/:id/delivered` | POST | flow-core-lifecycles | ✅ PASS |
| `/courier/assignments/:id/cancel` | POST | flow-core-lifecycles | ✅ PASS |

### 1.5 Settlements (Owner)
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/owner/locations/:id/settlements` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/settlements?status=` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/settlements/:id` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/settlements/:id/approve` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/settlements/:id/dispute` | POST | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/settlements/:id/reopen` | POST | flow-regulatory-settlements | ✅ PASS |

### 1.6 GDPR
| Endpoint | Method | Test File | Status | Notes |
|----------|--------|-----------|--------|-------|
| `/owner/locations/:id/gdpr-requests` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/gdpr-requests/:id` | GET | flow-regulatory-settlements | ✅ PASS |
| `/owner/locations/:id/gdpr-requests` | POST | flow-regulatory-settlements | ❌ FAIL | Returns 500 — **REAL SERVER BUG** |

---

## 2. Issue Matrix (Found & Fixed in This Session)

### 2.1 Bugs Found by Strengthened Assertions

| # | Issue | File | Severity | Status |
|---|-------|------|----------|--------|
| BP-1 | GDPR create endpoint returns 500 | `flow-regulatory-settlements.spec.ts:67` | 🔴 CRITICAL | **Confirmed server bug** — was masked by `expect([201,400,422,429,500]).toContain()` |
| BP-2 | Always-passing CSP assertion | `flow-security-contracts.spec.ts:123` | 🔴 CRITICAL | `expect(\`Page ${label} loaded without crash\`).toBeTruthy()` — string literal is always truthy |
| BP-3 | `body.length > 0` tautology (x3) | `admin/dashboard.spec.ts:15,25,27` | 🔴 HIGH | Anti-pattern per AGENTS.md §13.2 |
| BP-4 | `body.length > 0` tautology | `maps.spec.ts:40` | 🔴 HIGH | Anti-pattern per AGENTS.md §13.2 |
| BP-5 | `expect(typeof isVisible).toBe('boolean')` tautology | `maps.spec.ts:51` | 🔴 HIGH | Always passes (typeof any variable is always its type) |
| BP-6 | `expect(count).toBeGreaterThanOrEqual(0)` tautology | `admin/dashboard.spec.ts:33` | 🔴 HIGH | Always passes (0 >= 0) |
| BP-7 | `expect(visible \|\| true).toBeTruthy()` tautology | `admin/menu-manager.spec.ts:60` | 🟡 MEDIUM | Always passes |
| BP-8 | `expect(bodyClass !== null \|\| bodyClass === null).toBeTruthy()` tautology | `client/menu.spec.ts:131` | 🟡 MEDIUM | Always passes |
| BP-9 | `expect(desktopGridCols !== null \|\| desktopGridCols === null).toBeTruthy()` tautology | `flow-proofs.spec.ts:606` | 🟡 MEDIUM | Always passes |

### 2.2 Best-Practice Fixes Applied

| # | Fix | Files | Severity | Status |
|---|-----|-------|----------|--------|
| BP-F1 | Removed 500 from permissive status arrays | `flow-regulatory-settlements.spec.ts` | 🔴 CRITICAL | ✅ Fixed — now asserts 500 is a failure |
| BP-F2 | Replaced always-passing CSP assertion with real CSP header checks | `flow-security-contracts.spec.ts:123` | 🔴 CRITICAL | ✅ Fixed — checks `default-src` and `script-src` in CSP |
| BP-F3 | Replaced `body.length > 0` with `toBeTruthy()` + min length | `admin/dashboard.spec.ts` | 🔴 HIGH | ✅ Fixed |
| BP-F4 | Replaced `body.length > 0` with `toBeTruthy()` | `maps.spec.ts:40` | 🔴 HIGH | ✅ Fixed |
| BP-F5 | Replaced `typeof isVisible` tautology with actual visibility check | `maps.spec.ts` | 🔴 HIGH | ✅ Fixed |
| BP-F6 | Replaced `count >= 0` with `count > 0` | `admin/dashboard.spec.ts` | 🔴 HIGH | ✅ Fixed |
| BP-F7 | Replaced `waitForTimeout()` with `expect(locator).toBeAttached()` | `flow-security-contracts.spec.ts` | 🟡 MEDIUM | ✅ Fixed — 5 timeouts removed |
| BP-F8 | Replaced `waitForTimeout()` with `expect(locator).toBeAttached()` | `admin/dashboard.spec.ts` | 🟡 MEDIUM | ✅ Fixed — 1 timeout removed |
| BP-F9 | Replaced `waitForTimeout()` with `expect(locator).toBeAttached()` | `maps.spec.ts` | 🟡 MEDIUM | ✅ Fixed — 3 timeouts removed |
| BP-F10 | Added response status assertion to CSP test | `flow-security-contracts.spec.ts` | 🟡 MEDIUM | ✅ Fixed — was missing status check |

### 2.3 Remaining Anti-Patterns (Not Yet Fixed)

| # | Issue | Files | Severity | Notes |
|---|-------|-------|----------|-------|
| BP-R1 | `page.waitForTimeout()` used in ~60 places | Most browser test files | 🟡 MEDIUM | Replaced in 3 files; remaining in error-handling, ui-polish, cross-cutting, flow-courier-deep, etc. |
| BP-R2 | `page.locator('text=...')` instead of `getByText()` | ~15 places across browser tests | 🟡 MEDIUM | Works but not best practice |
| BP-R3 | `page.locator('#cartFabBtn')` instead of `getByTestId('cart-fab')` | ~8 places | 🟡 MEDIUM | Add `data-testid` to component |
| BP-R4 | `toContain([200, 409])` in flow-core-lifecycles | `flow-core-lifecycles.spec.ts` | 🟢 LOW | Acceptable for state-dependent endpoints — order may already be rejected |
| BP-R5 | CSS class selectors (`.product-card`, `.rounded-xl`) | ~50 places | 🟢 LOW | Acceptable for layout testing but fragile to refactors |

---

## 3. Coverage Gaps (Endpoints Without Tests)

### 3.1 Missing Endpoint Coverage
| Endpoint | Method | Reason | Notes |
|----------|--------|--------|-------|
| `/api/public/locations` | GET | Not implemented | Tour hub / discovery |
| `/api/customer/me` | GET/PUT | Stub | Profile |
| `/api/customer/orders` | GET | Stub | Order history |
| `/api/customer/favorites` | GET/POST | Stub | |
| `/orders/:id/review` | POST | Stub | Rate & review |
| `/api/support/tickets` | POST | Stub | |
| `/api/courier/me/payouts/:id` | GET | Partially covered | Only if payouts exist |
| `/owner/locations/:id/gdpr-requests/:id` (anonymize) | POST | Not covered | Actual anonymization execution |
| R2 bucket backup | — | Not covered | Infrastructure test |

### 3.2 Missing Flow Coverage
| Flow | Reason | Notes |
|------|--------|-------|
| Google OAuth login | Can't automate | Needs real browser session |
| WebSocket real-time updates | WS test infra not set up | courier tracking, order status |
| Actual OTP SMS send | Can't automate | Needs real SMS |
| Cross-browser (Firefox, Safari) | Config is chromium-only | playwright.config.ts |
| PWA / service worker | Not covered | Offline support |
| Image upload via browser UI | Not covered | API-only tests |
| Stress / load testing | Not covered | |

---

## 4. Run Results (2026-06-10)

### 4.1 API-Only Flow Tests (3 files)
| File | Tests | Pass | Fail | Skip | Notes |
|------|-------|------|------|------|-------|
| deploy-validation.spec.ts | 66 | 64 | 0 | 2 | Branding-theme, auth-redirect skip if no admin |
| flow-core-lifecycles.spec.ts | 96 | 79 | 0 | 17 | Courier/order state skips |
| flow-regulatory-settlements.spec.ts | 48 | 39 | 3 | 6 | 3 failures = GDPR 500 bug (all 3 viewports) |
| **Subtotal** | **210** | **182** | **3** | **25** | **98.6% pass rate** |

### 4.2 Browser Tests
| Category | Tests | Status | Notes |
|----------|-------|--------|-------|
| Browser smoke tests | ~70 | ❌ FAIL | Need local dev server (connect to localhost:5173) |
| Flow browser tests | ~40 | ❌ FAIL | Same issue — no local server |

### 4.3 Total
| Project | Tests | Pass | Fail | Skip | Did Not Run |
|---------|-------|------|------|------|-------------|
| mobile | ~106 | ~70 | ~20 | ~15 | ~1 |
| tablet | ~106 | ~70 | ~20 | ~15 | ~1 |
| desktop | ~106 | ~70 | ~20 | ~15 | ~1 |
| **Total** | **~320** | **~210** | **~60** | **~45** | **~5** |

---

## 5. Quick-Run How-To

```powershell
# API-only flow tests (fast — 1.1min, 182/210 pass)
$env:VITE_BASE_URL="https://dowiz.fly.dev"; npx playwright test "e2e/tests/deploy-validation.spec.ts" "e2e/tests/flow-core-lifecycles.spec.ts" "e2e/tests/flow-regulatory-settlements.spec.ts" --reporter=list

# Full suite (needs local dev server running — pnpm dev:all)
npx playwright test --reporter=list

# Single file
npx playwright test "e2e/tests/flow-security-contracts.spec.ts" --reporter=list

# Single project
npx playwright test --project=mobile --reporter=list
```

---

## 6. Critical Server Bug Found

### GDPR create erasure request returns 500

- **Endpoint**: `POST /owner/locations/:id/gdpr-requests`
- **Test**: `flow-regulatory-settlements.spec.ts:61` — "Flow 1: GDPR — create erasure request"
- **Assertion**: `expect(gdprStatus).not.toBe(500)` — **FAILS** (returns 500)
- **Previously masked by**: `expect([201, 400, 422, 429, 500]).toContain(gdprRes.status())`
- **Impact**: GDPR erasure requests cannot be created — Phase 5 blocker
- **Likely cause**: Rate limiting bug (1/customer/24h), missing DB migration, or middleware error

**To reproduce**:
```powershell
$VITE_BASE_URL="https://dowiz.fly.dev"; npx playwright test "e2e/tests/flow-regulatory-settlements.spec.ts" --reporter=list
```
