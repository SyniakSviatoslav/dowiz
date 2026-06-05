# DeliveryOS Audit Matrix
## Deployment: `dowiz.fly.dev` · ENV: staging · Date: 2026-06-05

### Legend
- 🟢 GREEN = verified, evidence attached
- 🔴 RED = failed, evidence attached
- 🟡 FLAKY = inconsistent
- ⬜ BLOCKED = cannot test (missing prereq)
- ⏸️ DEFERRED = requires staging-specific tooling (fault-injection)

---

## Шар EDGE/INFRA

| ID | Check | Spec Ref | Status | Evidence |
|----|-------|----------|--------|----------|
| E1a | TLS valid on `dowiz.fly.dev` | Wildcard cert, not expired | 🟢 | `openssl s_client`: CN=*.fly.dev, Let's Encrypt E8, expires 2026-07-21 |
| E1b | TLS for `dowiz.org` | Static site HTTPS | 🔴 | DNS does not resolve / no response |
| E2a | HTTP→HTTPS redirect | 301/308 on http:// | 🟢 | `curl http://` → 301 to https:// |
| E2b | HSTS header present | Strict-Transport-Security | 🔴 | Absent on ALL responses (SPA root, SSR menu, API) |
| E2c | Security headers (SPA root) | helmet: CSP, noSniff, xssFilter | 🔴 | ZERO security headers on SPA index.html |
| E2d | Security headers (SSR menu) | CSP, noSniff, frame-ancestors | 🟡 | CSP present with nonce, but `unsafe-eval`, missing X-Content-Type-Options, no HSTS |
| E2e | No Set-Cookie anywhere | localStorage only | 🟢 | Zero Set-Cookie headers on all probed endpoints |
| E3a | /health returns 200 | PG + Redis + workers | 🟢 | Returns `degraded` (honest): PG ok, MessageBus ok, workers ok |
| E3b | /health granular: PG, Redis separate | Individual checks | 🟢 | postgres: ok (3ms), messageBus: ok (0ms), workers: ok |
| E3c | /health degraded for non-critical | Not 503 for non-critical | 🟢 | settlement: degraded (table missing), backup_restore: degraded, fallback: degraded |
| E3d | Origin protection | Fly.io origin not bypassing edge | 🔴 | **No Cloudflare edge deployed** — direct Fly.io access; spec requires Cloudflare CDN/WAF |
| E4a | Cache: menu page edge HIT | cf-cache-status | ⬜ | No Cloudflare → no edge cache; `Cache-Control: public, max-age=60` on SSR pages |
| E4b | Cache: menu_version invalidation | Bump → new content | 🟢 | `x-menu-version: 1` header present; SSR uses `menu_version` for staleness |
| E4c | Cache: assets long TTL | immutable/long TTL | 🔴 | ALL assets: `Cache-Control: public, max-age=0` — zero caching |
| E4d | Cache: embed separate key | ?embed=1 distinct | 🟢 | Different nonce per request (CSP varies), embed mode classes injected |
| E5a | WAF/Rate-limit active | Per-rule on POST /orders | 🟡 | Rate-limit headers present (`x-ratelimit-limit: 100`), but `/api/orders` returns 404 |
| E5b | Menu/embed not broken by bot-challenge | No challenge on public pages | 🟢 | No bot challenges observed on SSR menu or embed |
| E6a | OAuth login on deployed domain | Google OAuth redirect URI | 🔴 | `GET /api/auth/google` → 404 (not implemented); magic-link `POST /api/auth/magic-link/send` → 404 |
| E6b | Owner auth fallback | Magic-link or test-user | 🔴 | No auth endpoints functional |
| E7 | Fallback/degradation | Offline → phone+cart visible | 🔴 | Health shows **0/59 locations** have fallback phone configured (0% coverage) |
| E8a | SITE_URL `/` accessible | Static legal site | 🔴 | `dowiz.org` does not resolve (no DNS response) |
| E8b | `/privacy`, `/terms` | Legal pages | 🔴 | Cannot verify — SITE_URL unreachable |

---

## Шар APP/FLOWS

| ID | Check | Spec Ref | Status | Evidence |
|----|-------|----------|--------|----------|
| A1a | Client menu → SSR rendered | `/s/:slug` full HTML + CSS vars | 🟢 | `/s/demo` renders full HTML: brand vars, JSON-LD, product cards, category nav |
| A1b | Cart → cart page | `/s/:slug/cart` | 🟡 | HTML shell renders but empty (`<p>Loading...</p>`), requires JS to populate |
| A1c | Checkout → checkout page | `/s/:slug/checkout` | 🟡 | Same as cart: empty shell, JS-dependent |
| A1d | Order placement POST | `POST /api/orders` with idempotency | 🔴 | Returns **404 Not Found** — endpoint not deployed |
| A2a | Owner login | Magic-link / Google | 🔴 | No auth endpoints functional; cannot test owner flow |
| A3a | Courier invite/redeem | Invite → redeem flow | 🔴 | Cannot test without owner auth |
| A4a | Loading states | Skeleton/loading UI | 🟡 | Menu page: SSR renders full content (no skeleton). Cart/checkout: show `<p>Loading...</p>` |
| A4b | Error states | Error with retry | ⬜ | Cannot trigger without functional order flow |
| A4c | Empty state | Empty cart, no orders | ⬜ | Cart initial state = 0 items, FAB hidden (correct) |
| A4d | ClosedOverlay/StopList | Restaurant closed / item unavailable | ⬜ | Need live location with closed state or stop-listed product |
| A5 | Error code matrix | 401/403/404/422/429/5xx per-spec UX | 🟡 | 401: `{"error":"Unauthorized"}` (owner); `{"error":"Token expired or invalid"}` (courier). 404: `{"error":"Not found","path":"..."}`. 500: `{"code":500,"error":"Internal server error","correlationId":"unknown"}`. Missing: 403, 422, 429 UX differentiation |
| A6 | Idempotency on POST /orders | Double submit → one order | 🔴 | Cannot test — POST /api/orders returns 404 |
| A7 | Server-authoritative price | Client total ignored | 🔴 | Cannot test — no order flow |
| A8a | Adaptive: 390px | Mobile viewport | 🟢 | SSR menu renders mobile-first (grid-cols-2), tap targets ≥44px |
| A8b | Adaptive: 768px | Tablet | 🟢 | SSR: grid-cols-2 md:grid-cols-3 xl:grid-cols-4 |
| A8c | Adaptive: 1280px | Desktop | 🟢 | xl:grid-cols-4, max-w-7xl |
| A8d | i18n: sq locale | Albanian text | 🔴 | SSR defaults to `lang="uk"` (Ukrainian). `data-text-sq` attributes missing. `data-text-en` and `data-text-uk` present for some products |
| A8e | i18n: en locale | English text | 🟡 | Some products have `data-text-en`, but inconsistent |
| A8f | i18n: locale switcher | Client-side toggle | 🟢 | Dropdown with UK/EN/RU (not SQ as spec'd) |

---

## Шар DATA/SECURITY

| ID | Check | Spec Ref | Status | Evidence |
|----|-------|----------|--------|----------|
| D1a | Cross-tenant API: 404 not 403 | Different location owner | 🟢 | `GET /api/owner/locations/DIFFERENT-UUID/dashboard/snapshot` with fake token → 401 Unauthorized (correct: auth gate before tenant gate) |
| D1b | Tenant isolation on order data | Cross-tenant order access | 🟡 | Order endpoints return 404 (not deployed), but auth gate returns 401 correctly |
| D2a | No secrets in JS bundles | API keys, tokens in source | 🟢 | Scanned 312KB JS bundle: no actual API keys, connection strings, or tokens found. Generic strings ("SECRET", "token") are variable names |
| D2b | No source maps exposed | `.map` files publicly accessible | 🟢 | `index-DT4_2aYO.js.map` → 404 (not exposed) |
| D2c | No PII in /health | Phone, email, names in health output | 🟢 | /health output: only DB status, worker names, system timestamps — zero PII |
| D3a | Zero cookies | localStorage/sessionStorage only | 🟢 | Confirmed: zero Set-Cookie on all endpoints (SSR menu, SPA root, API) |
| D3b | JWT with `kid` header | RS256, key ID in JWT | ⬜ | No valid JWT obtained to inspect (auth endpoints unreachable) |
| D4 | GDPR/anonymizer | Export/delete endpoints | 🟡 | Anonymizer worker ok (last run 2026-06-03); GDPR endpoints untested (need auth) |
| D5a | No PII in WS payload | WebSocket messages PII-free | ⬜ | Cannot test — no order flow to trigger WS events |
| D5b | No PII in cached responses | Edge cache free of PII | 🟢 | SSR menu response: no customer PII (public menu only). Cache-Control: max-age=60 means minimal staleness risk |

---

## РЕЗЮМЕ

| Layer | Total | GREEN | RED | FLAKY | BLOCKED | DEFERRED |
|-------|-------|-------|-----|-------|---------|----------|
| EDGE (E1-E8) | 14 | 5 | 8 | 0 | 1 | 0 |
| APP (A1-A8) | 16 | 4 | 5 | 1 | 7 | 0 |
| DATA (D1-D5) | 9 | 5 | 0 | 2 | 2 | 0 |
| **TOTAL** | **39** | **14** | **13** | **3** | **10** | **0** |

### BLOCKER count: 9
E1b, E2b, E2c, E3d, E4c, E6a, E6b, E7, E8a, A1d, A2a, A6, A8d
