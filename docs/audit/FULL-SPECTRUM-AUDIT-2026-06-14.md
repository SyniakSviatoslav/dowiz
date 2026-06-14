# Full-Spectrum Audit Report — DeliveryOS

> **Date:** 2026-06-14  
> **Target:** `https://dowiz.fly.dev`  
> **Method:** Live API probes + curl + existing Playwright E2E (57 tests) + static code analysis  
> **Coverage:** 18 frontend surfaces, 35 API flows, 3 roles (owner/courier/customer), full codebase scan  

---

## Executive Summary

| Metric | Count |
|--------|-------|
| **Total findings** | **27** |
| **🔴 P0 (blocker)** | 5 |
| **🔴 P1 (critical)** | 5 |
| **🟠 P2 (high)** | 8 |
| **🟡 P3 (medium)** | 6 |
| **🔵 P4 (low)** | 3 |
| **Health** | degraded (backup_restore failed, fallback 0%) |
| **E2E tests** | 49/57 pass · **8 fail** (all on auth + SSR issues) |

---

## 🔴 P0 — Blockers (breaks core functionality)

### B1. Auth 500 on ALL endpoints — JWT env vars missing/deployed

| | |
|---|---|
| **Symptom** | `POST /auth/mock`, `POST /auth/local/login`, `POST /orders` all return 500 with `correlationId: "unknown"` |
| **Root cause** | 2 compounding bugs: (1) `packages/platform/src/auth/jwt.ts:6` calls `loadEnv()` at module scope — crashes on import if `***REDACTED***` missing; (2) `.env.example:8` documents `***REDACTED***` (HS256) but code at `packages/config/src/index.ts:11-12` requires `***REDACTED***` + `***REDACTED***` (RS256). Fly.io likely has the old env var name. |
| **Impact** | Every authenticated flow is broken — owner dashboard, courier app, customer order history, all admin screens |
| **Fix** | (a) `flyctl secrets set ***REDACTED***=... ***REDACTED***=...`; (b) Make `loadEnv()` lazy in jwt.ts; (c) Update `.env.example` |
| **Evidence** | `apps/api/src/server.ts:687,800` — `signAuthToken()` called without try/catch; `jwt.ts:6` — module-scope env |

### B2. SSR menu returns SPA shell — Preact renderer never implemented

| | |
|---|---|
| **Symptom** | `GET /s/demo` returns `<div id="root"></div>` — no SSR content, no JSON-LD, no OG tags, no hreflang |
| **Impact** | Menus invisible to search engines, no social preview, slow initial load for customers |
| **Root cause** | `ssr.ts:7` calls `reply.sendFile('index.html')` which serves the static SPA shell. Despite AGENTS.md claiming "SSR was dormant — now active" (fixed 2026-06-13), the actual code was NEVER changed. The `ssr-renderer.ts` file referenced in AGENTS.md does not exist. |
| **Fix** | Rewrite SSR handler to: query DB by slug, render products/categories with Preact, inject JSON-LD/OG/hreflang |
| **Evidence** | `apps/api/src/routes/public/ssr.ts:7` — `reply.sendFile('index.html')` |

### B3. `db.connect()` outside try/catch in orders.ts — connection leak

| | |
|---|---|
| **Symptom** | If connection pool exhausted, `db.connect()` throws uncaught error; `finally` block also errors on undefined `client` |
| **Impact** | Orders silently fail under load; connection leaks crash the pool; the internal catch has wrong response shape (no `code`/`correlationId`) |
| **Root cause** | `orders.ts:68` — `const client = await db.connect()` is 1 line BEFORE the inner `try` block at line 69 |
| **Fix** | Move `db.connect()` inside the try/catch; add connection error handling |
| **Evidence** | `apps/api/src/routes/orders.ts:68-69` |

### B4. CDN images all broken — `cdn.dowiz.org` returns 404

| | |
|---|---|
| **Symptom** | All product images fail to load across every surface. Network log: `Failed to load resource` |
| **Impact** | Zero product images visible. Menu looks broken. Admin branding preview also broken. |
| **Root cause** | `cdn.dowiz.org` DNS or Cloudflare Worker not configured. API stores `image_key` as `products/{locationId}/{uuid}.webp` referencing the CDN subdomain. |
| **Fix** | Configure CDN CNAME or fall back to direct R2 URLs |
| **Evidence** | `curl -I https://cdn.dowiz.org/products/test.webp` → `404 Not Found`; FE-RADAR-REPORT F1 |

### B5. Correlation ID always `"unknown"` — property name mismatch

| | |
|---|---|
| **Symptom** | Every 500 response shows `correlationId: "unknown"`. Cannot trace errors. |
| **Root cause** | `server.ts:216` stores ID in `request.headers['x-correlation-id']` but `server.ts:576` reads `(request as any).correlationId` — a property that is NEVER set |
| **Fix** | Change line 576 to read `request.headers['x-correlation-id']` |
| **Evidence** | `apps/api/src/server.ts:576` vs `:216` |

---

## 🔴 P1 — Critical

### C1. `uk` locale entirely missing; `en` locale truncated (~50 missing keys)

| | |
|---|---|
| **What** | `packages/ui/src/lib/i18n.ts` has `sq` (~730 keys) and partial `en` (~680 keys) but **zero `uk` keys**. The `uk` section header exists but no entries follow. |
| **Impact** | Ukrainian restaurant owners and customers see empty/missing labels |
| **Fix** | Translate all 730 keys for `uk` locale. Complete missing `en` keys (~50 from `sq` not in `en`). |

### C2. 14+ hardcoded English strings bypassing i18n `t()` system

| | |
|---|---|
| **What** | Courier LoginPage (`Email`/`Password` labels), CourierInvitePage (5 dual-language labels), FlowTestPage (3 selects), DeliveryPage (`Call` link), OrderStatusPage (`Not Found`) |
| **Impact** | These strings never localize. Albanian/Ukrainian owners see English text. |
| **Fix** | Replace every hardcoded string with `t('key', 'fallback')` |

### C3. 17 unlabeled form elements in admin settings (known from FE-RADAR)

| | |
|---|---|
| **What** | Admin settings page renders `<input>` fields without `<label>` or `aria-label`. Screen readers cannot identify any settings fields. |
| **Evidence** | FE-RADAR-REPORT-v2 C1 — 17 violations |
| **Fix** | Add `<label>` or `aria-label` to every settings input |

### C4. 110 touch targets below 44px WCAG minimum on mobile menu (known)

| | |
|---|---|
| **What** | Category tabs, quantity +/- buttons, filter chips all smaller than 44px. Critical for couriers in motion. |
| **Evidence** | FE-RADAR-REPORT-v2 C5 — 110 elements |
| **Fix** | Increase `min-width`/`min-height` to 44px on all interactive elements |

### C5. 88 ARIA role parent violations on menu page (known)

| | |
|---|---|
| **What** | ARIA `tab`/`listitem`/`option` roles used without required parent `role="tablist"` / `role="list"` containers |
| **Evidence** | FE-RADAR-REPORT-v2 C4 — 88 violations |
| **Fix** | Wrap elements in proper role containers |

---

## 🟠 P2 — High

### H1. 3 duplicate ErrorBoundary implementations

| | |
|---|---|
| **What** | `packages/ui/src/components/Fallback.tsx:26` + `Status.tsx:68` + `apps/web/src/main.tsx:9` — three copies. Only 1 top-level usage, no page-level boundaries. A crash in one page breaks navigation entirely. |
| **Fix** | Consolidate to one canonical component; add page-level boundaries |

### H2. 3 duplicate ALLERGEN_COLORS maps

| | |
|---|---|
| **What** | Identical 8-color hex map repeated in `MenuManagerPage.tsx`, `MenuPage.tsx`, and `ClientUI.tsx` |
| **Fix** | Extract to shared constant in `packages/ui/src/` |

### H3. ~15 hardcoded hex colors in production code

| | |
|---|---|
| **Examples** | `#ea4f16` in CSS keyframes (`index.css:152,268,324,360`), `#000000`/`#ffffff` in SettingsPage, `#22c55e`/`#f59e0b` in CouriersPage, `#0088cc` Telegram icon, etc. |
| **Fix** | Replace with CSS var references or `color-mix()` |

### H4. No `prefers-color-scheme` auto-dark mode detection

| | |
|---|---|
| **What** | Dark mode is implemented via CSS variables (6 presets) but no system preference detection. User must manually cycle themes. |
| **Fix** | Add `prefers-color-scheme` media query → auto-apply dark preset when system is dark |

### H5. Settlement endpoint returns 500

| | |
|---|---|
| **What** | `GET /api/owner/locations/:locId/settlements` → 500. Blocks owner from viewing settlement history. |
| **Evidence** | FLOW-RADAR-REPORT #1 |
| **Fix** | Investigate SQL error in period boundaries; add try/catch with empty array fallback |

### H6. Backup restore test degraded

| | |
|---|---|
| **What** | `/health` shows `backup_restore.status = "degraded"`, `last_result = "failed"`, `stale = true`. R2 backup exists but restore path is unverified. |
| **Fix** | Debug BackupVerifyWorker logs, test manual restore |

### H7. 51 empty test categories cluttering public menu

| | |
|---|---|
| **What** | Menu API returns ~51 categories with names like `Test-Cat-1781098561058`, `E2E-Cat-1781095951339` — all empty (zero products). These clutter the customer menu and look unprofessional. |
| **Fix** | Clean up test data from demo location; hide empty categories from public menu |

### H8. No loading skeleton on slow-3g for menu page (known)

| | |
|---|---|
| **What** | Menu page shows blank/empty while API loads. No skeleton/spinner found. |
| **Fix** | Add Suspense or loading state boundary around menu data fetch |

---

## 🟡 P3 — Medium

### M1. Courier login minimal content (89 chars)

| | |
|---|---|
| **Symptom** | `/courier/login` renders essentially empty page (89 chars body) across all viewports |
| **Evidence** | FE-RADAR-REPORT F5 |
| **Fix** | Debug route component; ensure login form renders |

### M2. Branding preview iframe returns 404

| | |
|---|---|
| **Symptom** | Admin branding page cannot show live preview of theme changes |
| **Evidence** | FE-RADAR-REPORT F2 |
| **Fix** | Add/branding-preview route or remove broken preview |

### M3. Order status page shows 3x console errors for fake IDs

| | |
|---|---|
| **Symptom** | Fake order UUID returns 401 (not 404) because auth hook fires before 404 check. Frontend only handles `err.status === 404`, so 401 hits generic error path. |
| **Fix** | Better error code ordering (auth after entity check for public endpoints); handle 401 in frontend |

### M4. 0% fallback phone coverage

| | |
|---|---|
| **What** | `0/150 locations have fallback phone configured (0%)`. Feature exists but owners don't know about it. |
| **Fix** | Add fallback phone prompt to onboarding flow or dashboard readiness checklist |

### M5. framer-motion shared variants (`motion.ts`) unused by components

| | |
|---|---|
| **What** | `motion.ts` defines `fadeIn`, `scaleIn`, `slideUp`, `staggerContainer` variants but individual components (`DeliveryPage.tsx:178-195`, `MenuPage.tsx:483`) use inline `animate` props instead |
| **Fix** | Reference shared variants for consistency |

### M6. Menu price modifier uses raw `toLocaleString()` bypassing PriceDisplay

| | |
|---|---|
| **What** | `MenuPage.tsx:744` — `{mod.price_delta.toLocaleString()}` — uses raw number formatting instead of `PriceDisplay`/`formatMoney`. Minor money contract violation. |
| **Fix** | Use `PriceDisplay` for all monetary values including modifier price deltas |

---

## 🔵 P4 — Low

### L1. FE-RADAR known a11y issues unresolved (color contrast, button names, aria-live)

35 contrast violations across 6 surfaces, icon-only buttons missing aria-label, no aria-live for dynamic updates.

### L2. Courier task cards missing "Add" button in SSR

E2E test `button[aria-label="Add"]` not found on SSR-rendered product cards. Interactive elements not rendered in SSR output.

### L3. E2E test helper bugs found

`checkTouchTargets()` returns `undefined` instead of array; `checkAriaLive()` returns `boolean`; `authRes.status()` called as function instead of property.

---

## Health & Infrastructure State

| Component | Status | Detail |
|-----------|--------|--------|
| Postgres | ✅ ok | 4-6ms latency |
| Workers | ✅ ok | All 4 workers healthy |
| MessageBus | ✅ ok | NOTIFY/LISTEN working |
| Telegram bot | ✅ ok | `dowizbot_bot` responding |
| R2 storage | ✅ ok | Bucket reachable |
| Settlement | ✅ ok | Last period end: 2026-06-11 |
| Anonymizer | ✅ ok | Last run: 2026-06-03 |
| Backup | ✅ ok | Hourly backups running |
| **Backup restore** | ❌ degraded | Last result: failed, stale: true |
| **Fallback** | ❌ degraded | 0/150 locations configured (0%) |
| **Auth (all forms)** | ❌ BROKEN | 500 on every auth endpoint |
| **Login endpoint** | ❌ BROKEN | 500 on POST /auth/local/login |
| **CDN images** | ❌ BROKEN | 404 on cdn.dowiz.org |
| **SSR menu** | ❌ BROKEN | SPA shell only, no content |
| **Order creation** | ❌ BROKEN | 500 (connection outside try/catch + auth) |

---

## Backlog (ranked by business impact)

| # | Priority | Item | Effort | Depends On |
|---|---|---|---|---|
| 1 | **P0** | Fix JWT env vars on Fly.io + lazy load in jwt.ts | 1h | Fly.io access |
| 2 | **P0** | Fix SSR menu — rewrite handler with Preact rendering | 4-6h | Preact + DB schema |
| 3 | **P0** | Fix db.connect() try/catch in orders.ts | 30m | — |
| 4 | **P0** | Configure CDN or direct R2 URLs for images | 2h | Cloudflare/R2 |
| 5 | **P0** | Fix correlation ID property name mismatch | 15m | — |
| 6 | **P1** | Complete `uk` locale translation (730 keys) | 4-8h | Translator |
| 7 | **P1** | Convert 14+ hardcoded strings to `t()` | 2h | — |
| 8 | **P1** | Add labels to 17 unlabeled form inputs in settings | 2h | — |
| 9 | **P1** | Fix 110 touch targets < 44px on mobile | 1-2h | — |
| 10 | **P1** | Fix 88 ARIA parent role violations on menu | 1h | — |
| 11 | **P2** | Consolidate 3 ErrorBoundary implementations | 1h | — |
| 12 | **P2** | Extract ALLERGEN_COLORS to shared constant | 30m | — |
| 13 | **P2** | Replace 15 hardcoded hex colors with CSS vars | 2h | — |
| 14 | **P2** | Add prefers-color-scheme auto dark mode | 1h | — |
| 15 | **P2** | Fix settlement 500 error | 1-2h | — |
| 16 | **P2** | Debug backup restore verification | 2-4h | R2 access |
| 17 | **P2** | Clean up 51 empty test categories from demo menu | 30m | — |
| 18 | **P2** | Add loading skeleton to menu page | 1-2h | — |
| 19 | **P3** | Fix courier login page (89 chars body) | 1h | — |
| 20 | **P3** | Fix branding preview iframe 404 | 1h | — |
| 21 | **P3** | Better 401 handling in OrderStatusPage | 1h | — |
| 22 | **P3** | Add fallback phone prompt to onboarding | 1h | — |
| 23 | **P3** | Use shared framer-motion variants across components | 1h | — |
| 24 | **P3** | Fix modifier price formatting → use PriceDisplay | 15m | — |
| 25 | **P4** | Fix color contrast violations (35 across 6 surfaces) | 2-3h | — |
| 26 | **P4** | Fix E2E test helper type bugs | 30m | — |
| 27 | **P4** | Add aria-live regions for dynamic updates | 1h | — |

---

## Key Recommendations (in order)

1. **Fix auth first** — everything authenticated is dead. The JWT env var fix takes <1h.
2. **Fix SSR menu** — it's the customer-facing front door. Search engines see nothing.
3. **Fix orders.ts** — it will crash under any real load. 30m fix.
4. **Fix CDN images** — menus look broken with zero product images.
5. **Fix correlation ID** — 15m fix that unlocks error tracing.
6. **Then tackle the i18n/a11y backlog** — P1 items block inclusivity and accessibility.
7. **Clean up test data** — 51 empty "Test-Cat-" categories make the demo look abandoned.
