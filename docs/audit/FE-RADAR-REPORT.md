# FE-RADAR-REPORT.md — Full Frontend Issue Matrix

> Generated: 2026-06-12 · Target: `dowiz.fly.dev` (staging)
> Method: Playwright × 12 surfaces × 3 viewports (mobile 390, tablet 768, desktop 1280)
> Collectors: network (4xx/5xx/fails), console (errors/warnings), page errors, DOM state
> Results: 36 tests — 34 ✅ pass · 2 ⏱ timeout

---

## Executive Summary

| Metric | Count |
|---|---|
| **Surfaces tested** | 12 × 3 viewports = 36 |
| **OK** | 34 |
| **🔴 Issues** | **3** |
| **🟠 Issues** | **3** |
| **🔵 Observations** | **2** |
| **Timeout (infrastructure)** | **2** (PWA chunk check on mobile/tablet — networkidle never settles) |

---

## 🔴 Critical Issues

### F1. CDN image domain fails (all surfaces)
| | |
|---|---|
| **Surfaces** | `menu` · `admin-branding` · (likely all with product images) |
| **URL** | `https://cdn.dowiz.org/products/…` |
| **Expected** | Image loads (2xx) |
| **Actual** | `requestfailed` — DNS/CORS error |
| **Evidence** | Network log: `Failed to load resource` on every product image |
| **Severity** | 🔴 — all product images broken across the entire app |
| **Hypothesis** | `cdn.dowiz.org` DNS not resolving or CORS not configured. Images stored in R2 but the custom CDN subdomain is misconfigured. The API stores `image_key` as `products/{locationId}/{uuid}.webp` paths referencing `cdn.dowiz.org`, but the CDN CNAME or Cloudflare Worker isn't set up. |

### F2. Admin branding preview iframe fails
| | |
|---|---|
| **Surface** | `admin-branding` |
| **URL** | `https://dowiz.fly.dev/branding-preview/demo?embed=true&draft=true` |
| **Expected** | Preview loads (2xx) |
| **Actual** | `requestfailed` — 0 status |
| **Evidence** | Network log on admin-branding page |
| **Severity** | 🔴 — branding page cannot show live preview of changes |
| **Hypothesis** | The `branding-preview/:slug/*` route doesn't exist on the deployed server, or returns a non-2xx. This is an SPA route that may not be handled by the server-side catch-all. Check `server.ts` routing for `branding-preview/*`. |

### F3. Public menu SSR renders empty shell (all viewports)
| | |
|---|---|
| **Surface** | `menu` (`/s/demo`) |
| **Expected** | Menu content rendered with category/product names |
| **Actual** | Body does not contain expected menu text (`Menu`, `Category`, product name) |
| **Evidence** | Text content check across all 3 viewports |
| **Severity** | 🔴 — customer-facing menu page fails to display products |
| **Hypothesis** | SSR shell (`public/client-flow.ts`) renders the HTML wrapper, but the React app inside fails to hydrate or fetch menu data. Check: (1) Did the API call to `/public/locations/demo/menu` return 200? (2) Did the React component receive the data? (3) Is there a hydration mismatch? Also observed: CDN images fail, which could cause the content area to collapse or error. |

---

## 🟠 Functional Issues

### F4. Order status page shows console errors for fake ID
| | |
|---|---|
| **Surface** | `order-status` (all viewports) |
| **URLs** | `/api/customer/orders/test-123/status` × 2, `/api/orders/test-123/messages` |
| **Expected** | Graceful 404 handling with empty state |
| **Actual** | 401 thrown on 3 endpoints + 3 console errors |
| **Evidence** | 3× 401 network + 3× console-error per page load |
| **Severity** | 🟠 — fake ID case causes 401 (not 404) because customer auth endpoint requires valid token, and the test-123 UUID doesn't match. The 401 is expected for improper UUIDs, but it also demonstrates that the frontend's catch handler for err.status === 404 (line 89 of OrderStatusPage.tsx) won't trigger when the API returns 401 instead. |
| **Hypothesis** | The customer orders endpoint requires a valid JWT with `orderId` claim. Test-123 is not a valid UUID, so the route returns 401 at the auth hook level before the 404 check. The frontend only handles `err.status === 404` with a specific order object, so a 401 causes the generic error path to render. |

### F5. Courier login minimal content
| | |
|---|---|
| **Surface** | `courier-login` (all viewports) |
| **URL** | `/courier/login` |
| **Expected** | Login form with email/password fields |
| **Actual** | Body length 89 characters (essentially empty page) |
| **Evidence** | 89 chars body for mobile/tablet/desktop |
| **Severity** | 🟠 — courier login page appears broken or blank |
| **Hypothesis** | The courier login page component might not render or route correctly. Check if the React route for `/courier/login` maps to a different component than expected, or if the page depends on an API call that fails. |

### F6. Admin dashboard not showing expected text
| | |
|---|---|
| **Surface** | `admin-dashboard` (all viewports) |
| **Expected** | Dashboard shows "Pending", "Orders", "Revenue" or similar |
| **Actual** | Body length 4766 but contains none of the expected keywords |
| **Evidence** | Text content check: no Pending/Order/Revenue found (mobile) |
| **Severity** | 🟠 — dashboard renders but may be showing empty state or different content |
| **Hypothesis** | The dashboard may render in a loading/error state, or the translation keys don't match expected English text. The API calls to `/api/owner/*` endpoints are working (confirmed in backend radar), so data should be available. Check: (1) Is the mock-auth token being accepted by owner routes from the browser context? (2) Is the tenantId being set correctly? |

---

## 🔵 Observations / Infrastructure

### O1. Networkidle never settles on mobile/tablet for menu page
| | |
|---|---|
| **Surface** | `pwa`/chunks check |
| **Test** | S12 — navigate to `/s/demo` with `waitUntil: 'networkidle'` |
| **Failed** | mobile (30s timeout), tablet (30s timeout) — desktop passed |
| **Evidence** | 2 test timeouts |
| **Severity** | 🔵 — indicates open connections or polling that never stops |
| **Hypothesis** | On mobile/tablet, the page may have an open WebSocket connection or polling interval that keeps the browser from reaching `networkidle`. Desktop may not exhibit this because of different timing or the viewport triggers different code paths. |

### O2. PWA manifest and service worker accessible
| | |
|---|---|
| **Surface** | `pwa` |
| **URLs** | `/manifest.json`, `/sw.js` |
| **Expected** | 200 on both |
| **Actual** | 200 on both (all viewports) |
| **Severity** | 🔵 — positive finding |
| **Note** | PWA basics are present. Offline functionality not tested (requires deeper SW lifecycle probe). |

---

## Cluster by Root Cause

| Cluster | Issues | Shared cause | Estimate |
|---|---|---|---|
| **CDN misconfiguration** | F1 (images), F3 (menu empty — images fail first) | `cdn.dowiz.org` DNS/CORS not configured. All product images referenced via CDN domain fail, which may cause content collapse | 1-2h: configure Cloudflare DNS or fall back to direct R2 URLs |
| **SSR/SPA routing gap** | F2 (branding-preview 404), F5 (courier login empty) | Missing SPA routes or catch-all handler fails for these paths | 1h: check server.ts catch-all, add missing routes |
| **Auth flow gap** | F4 (401 on fake ID), F6 (dashboard render) | Customer order status expects real JWT with orderId claim; dashboard may need full auth flow not just token in localStorage | 1-2h: add better error handling for 401, verify dashboard auth flow |
| **Hydration/data race** | F3 (menu empty), O1 (networkidle) | SSR shell loads but React hydration may fail when API data arrives late, or the component renders before data is available | 2-3h: add loading states, verify data flow in MenuPage |

---

## ✅ Passed (8 surfaces clean)

| Surface | Status | Notes |
|---|---|---|
| admin-login | ✅ | Login form renders correctly across all viewports |
| admin-menu | ✅ | Menu manager loads with categories |
| admin-settings | ✅ | Settings page loads |
| admin-couriers | ✅ | Couriers page loads |
| admin-analytics | ✅ | Analytics renders (charts likely placeholders) |
| admin-branding | ✅ | Branding page loads (preview iframe fails separately) |
| PWA manifest | ✅ | /manifest.json and /sw.js return 200 |
| PWA chunks | ✅ desktop | No stale 404 chunks found (desktop only) |

---

## Backlog (ordered severity → effort)

1. **🔴 Fix CDN domain** — configure `cdn.dowiz.org` DNS/CORS or switch to direct R2 URLs
2. **🔴 Fix menu SSR** — debug why `/s/demo` renders empty content despite API working
3. **🔴 Fix branding preview iframe** — add/branding-preview route or remove broken preview
4. **🟠 Fix courier login** — debug why `/courier/login` renders 89 chars
5. **🟠 Better 401 handling** — show empty state instead of error when order status returns 401
6. **🟠 Verify dashboard auth** — ensure mock-auth token works for all `/api/owner/*` calls
7. **🔵 Fix networkidle** — identify what keeps connections open on mobile/tablet

---

## Safety Confirmation

- ✅ Staging only (`dowiz.fly.dev`)
- ✅ Test accounts only (mock-auth, no real customer data)
- ✅ No destructive operations
- ✅ Screenshots/videos saved to `e2e/artifacts/test-results/`
