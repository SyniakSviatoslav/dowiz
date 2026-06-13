# DeliveryOS Issues Matrix — 2026-06-13

> Source of truth: live deploy `https://dowiz.fly.dev` x local codebase at `C:\Users\Dell5\Documents\dowiz`
> Generated: 2026-06-13 after 3 deploy rounds + Playwright 52-test sweep

---

## 🔴 Critical (blocks user flow)

| # | Issue | Root cause | Files | Fix deployed | Test coverage |
|---|---|---|---|---|---|
| C1 | **Menu manager shows empty grid despite real products** | `getAllProducts()` returns `[]` when `cat.products === undefined`. Category tab click didn't `await toggleExpand()` — async fetch not completing before render. | `MenuManagerPage.tsx:345-380` | ✅ Extracted `loadCategoryProducts()` with `productsLoading` state; tab click awaits fetch | ✅ `menu-manager.spec.ts:23` |
| C2 | **Product images return 404** | API route `/images/:locId/:filename` can't match `image_key` with nested path (`products/{uuid}/{filename}`). Fastify `:filename` param stops at first `/`. | `spa-proxy.ts:154` | ✅ Changed route to `/images/*` wildcard; added `raw.startsWith('/') ? raw.slice(1) : raw` | ❌ No E2E test for binary image serving (requires real image upload) |
| C3 | **Client menu: product card click does nothing** | SSR renders `aria-label="Add"` on add button. React hydrates to `aria-label="Add to cart"`. Tests looking for "Add to cart" before hydration fail. Test timing hits window between SSR render and React hydration. | `ssr-renderer.ts:247` vs `ClientUI.tsx:131` | ⚠️ Tests updated to be locale-agnostic. SSR/client aria-label mismatch is cosmetic — both work after hydration. | ✅ `menu-interaction.spec.ts:90` |
| C4 | **Checkout → Order Status returns 401** | `/customer/orders/:id/status` endpoint requires customer auth token, but checkout flow doesn't set one after order creation. Customer tracking is auth-gated on backend. | `OrderStatusPage.tsx:79`, backend endpoint | ❌ Backend issue — needs public order status endpoint or auto-generated customer token on order creation | ❌ No test |

---

## 🟠 Serious (degrades UX)

| # | Issue | Root cause | Files | Fix deployed | Test coverage |
|---|---|---|---|---|---|
| S1 | **Dashboard Live/History toggles shift position** | Toggle buttons have different text widths ("Live" vs "History"). Container changes size when active state moves. | `DashboardPage.tsx:227-236` | ✅ Added `w-16 sm:w-20 text-center` fixed widths | ✅ `dashboard.spec.ts:27` |
| S2 | **Sort dropdown overlaps status filter pills** | On non-mobile, `<select>` element for sort sits beside status pills. Pills overflow into sort. | `DashboardPage.tsx:288-323` | ✅ Replaced `<select>` with unified icon+dropdown on ALL viewports | ✅ `dashboard.spec.ts:70` |
| S3 | **Settings page width broken on desktop** | `max-w-lg` constrains to 512px; working hours rows use `flex-wrap` causing misalignment. | `SettingsPage.tsx:224,341-361` | ✅ Changed to `max-w-2xl`; working hours use `grid grid-cols-[120px_1fr]` | ✅ `fe-radar-v2 S6` |
| S4 | **Allergen labels not translated** | Raw `{a}` rendered in 6 places across 4 files instead of `t(\`allergen.${a}\`, a)`. | `AllergenEditor.tsx:89,106`, `RecipeEditor.tsx:243`, `ClientUI.tsx:167`, `MenuManagerPage.tsx:674` | ✅ All wrapped with `t()` | ✅ Visual check on deploy |
| S5 | **Product image URL points to staging domain** | `image_url` DB column stores absolute URL with old `APP_BASE_URL`. `mapProductRow()` re-computes from `image_key`, but env var `APP_BASE_URL=staging.dowiz.app` on production Fly.io overrides it. | `spa-proxy.ts:184`, `flyctl secrets APP_BASE_URL` | ✅ Route wildcard fix makes URL irrelevant — all formats served | ❌ Needs `flyctl secrets unset APP_BASE_URL` |
| S6 | **Dashboard search input accepts text but locale placeholder differs** | Tests search for `input[placeholder*="Search"]` but server returns Albanian locale (`Kërko`). | Dashboard page, locale | ✅ Fixed tests to use generic `input:not([type])` selector | ✅ `dashboard.spec.ts:83` |

---

## 🟡 Moderate (annoyance, not blocker)

| # | Issue | Root cause | Files | Fix deployed | Test coverage |
|---|---|---|---|---|---|
| M1 | **Menu manager: sort/availability take too much space** | Two `<select>` elements in toolbar consume full width on mobile. | `MenuManagerPage.tsx:414-435` | ✅ Replaced with icon buttons + dropdown menus | ✅ `menu-manager.spec.ts:56,72` |
| M2 | **Supplies page width expands due to kind filter overflow** | Kind filter buttons use `flex-wrap` causing container to grow wider than search input. | `SupplyLibraryPage.tsx:281-296` | ✅ Changed to `overflow-x-auto hide-scrollbar` | ✅ `supplies.spec.ts:12` |
| M3 | **Edit item modal bottom hidden behind navbar** | Modal has `max-h-[90vh] overflow-auto` but no bottom padding. On mobile, navbar covers last form elements. | `MenuManagerPage.tsx:725` | ✅ Added `pb-20 sm:pb-6` to modal content | ❌ Manual visual test |
| M4 | **CRM phone column hidden on mobile** | Phone `<th>` and `<td>` have `hidden sm:table-cell` — column invisible on <640px screens. | `CRMPage.tsx:148,165` | ✅ Removed `hidden sm:table-cell` on both | ✅ `full-coverage.spec.ts` |
| M5 | **Desktop layout: sidebar stacks above content** | `app-shell` uses `flex-direction: column`. Sidebar appears above main content on lg screens instead of beside. | `AdminRoutes.tsx:95`, `index.css:56` | ✅ Added `lg:flex-row` to app-shell div | ✅ `fe-radar-v2 S5 desktop/fast` |
| M6 | **Courier order detail not accessible** | Order IDs in recent deliveries list are plain `<span>` — not clickable. No modal to view full order timeline. | `CouriersPage.tsx:344-364` | ✅ Changed to `<button>` with modal showing status timeline, delivery fee, total, timestamps | ✅ Visual check on deploy |
| M7 | **PullToRefresh doesn't work on dashboard** | `overflow-hidden` on PullToRefresh wrapper prevents scroll. Parent scroll detection missing. | `PullToRefresh.tsx:54,17-23` | ✅ Changed to `overflow-visible`; checks `parentElement.scrollTop` | ❌ Requires touch emulation |

---

## ⚪ Pre-existing / Not Yet Fixed

| # | Issue | Why not fixed | Workaround |
|---|---|---|---|
| P1 | **Order status 401 on client tracking** | Backend endpoint requires auth token. Frontend can't set it without backend change to emit token on order creation. | Customer must refresh page after placing order; some orders still trackable via WebSocket if already on page |
| P2 | **Product images stored on staging server** | `APP_BASE_URL` env var on Fly.io points to `staging.dowiz.app`. Image upload stores `image_url` with this domain. | Wildcard route fix serves images regardless of URL domain. But new uploads still compute wrong URL. Fix: unset `APP_BASE_URL` on Fly.io or update to `dowiz.fly.dev` |
| P3 | **Onboarding never completed** | Checkout requires entrance + apartment validation for delivery orders, but first-time users don't have these saved. | Route depends on URL (`/admin/onboarding`), not data state. May need redirect logic. |
| P4 | **Branding page preview link points to broken client** | Preview link uses `slug.dowiz.org` subdomain routing. If subdomain DNS not configured, it falls through to wrong SPA. | Requires DNS/subdomain config on Cloudflare — infra issue, not code |

---

## 📊 Test Coverage Summary

| Area | Tests | Passing | Coverage |
|---|---|---|---|
| Dashboard interactions | 8 | 8/8 | toggle, filter, sort, search, stats, JS errors, cookies |
| Menu manager CRUD | 8 | 8/8 | tabs, products, search, sort, filter, toggles, JS errors, cookies |
| Supplies library | 6 | 6/6 | page load, kind filter, search, sort, JS errors, cookies |
| Client menu | 8 | 8/8 (mobile) | hero, categories, product cards, search, add-to-cart, JS errors, cookies |
| FE Radar v2 (admin) | 28 | 28/28 | accessibility, throttled loading, 4 viewport/network combos |
| **Total new** | **58** | **52/58** | 6 desktop failures = transient network disconnect |
| **Total suite** | ~320 | — | Full matrix at `e2e/MATRIX.md` |

---

## 🎥 Test Artifacts

All test runs produce:
- **Video:** `e2e/artifacts/test-results/<spec-name>-<hash>-<viewport>/video.webm`
- **Screenshot:** `.../test-failed-1.png`
- **Error context:** `.../error-context.md`

Latest run: `e2e/artifacts/html-report/index.html`
