# DeliveryOS Deployed Audit Findings
## `dowiz.fly.dev` · ENV: staging · 2026-06-05T21:33Z

---

## VERDICT: **NO-GO**

13 of 39 checks RED | 9 BLOCKERS | Core order creation endpoint (POST /api/orders) returns 404 — the critical client path cannot complete. Owner auth, fallback, Cloudflare edge, and Albanian i18n are all absent or non-functional.

---

## BLOCKERS (9)

| ID | Layer | Target | Expected | Actual | Repro |
|----|-------|--------|----------|--------|-------|
| F-001 | EDGE | `dowiz.org` | Static site serves HTTPS (E8a) | DNS does not resolve; no HTTP response | `curl https://dowiz.org/` — no output |
| F-002 | EDGE | SPA root | HSTS header present (E2b) | `Strict-Transport-Security` absent on all responses | `curl -I https://dowiz.fly.dev/` |
| F-003 | EDGE | SPA root | Security headers: CSP, X-Content-Type-Options, X-Frame-Options (E2c) | ZERO security headers on SPA index.html | `curl -I https://dowiz.fly.dev/` — only Cache-Control + Content-Type |
| F-004 | EDGE | All assets | Cache: immutable/long TTL on CSS/JS (E4c) | ALL assets served with `Cache-Control: public, max-age=0` | `curl -I https://dowiz.fly.dev/assets/index-*.js` |
| F-005 | EDGE | Cloudflare edge | CDN/WAF in front of Fly.io origin (E3d) | No Cloudflare; direct Fly.io access. Spec requires Cloudflare edge | `server: Fly/0c81dcd5` on every response; no `cf-ray`/`cf-cache-status` |
| F-006 | APP | `POST /api/orders` | Create order with idempotency key (A1d, A6) | Returns **404 Not Found** — endpoint not deployed | `POST /api/orders` with valid JSON body → 404 |
| F-007 | APP | Owner auth | Magic-link or Google OAuth login (E6a, A2a) | `POST /api/auth/magic-link/send` → 404; `GET /api/auth/google` → 404 | Both endpoints return 404 |
| F-008 | APP | SSR menu i18n | Albanian (`sq`) locale rendered (A8d) | SSR defaults to `lang="uk"` (Ukrainian); `data-text-sq` attributes missing; locale dropdown shows UK/EN/RU | `curl https://dowiz.fly.dev/s/demo` → `<html lang="uk">` |
| F-009 | APP | Fallback phones | Each location must have fallback phone configured (E7) | Health: **0/59 locations** have fallback phone (0% coverage) | `/health` → `"fallback":{"detail":"0/59 locations have fallback phone configured (0%)"}` |

---

## MAJOR (8)

| ID | Layer | Target | Expected | Actual | Repro |
|----|-------|--------|----------|--------|-------|
| F-010 | EDGE | SSR menu | Complete security headers (E2d) | CSP present but `script-src 'unsafe-eval'` + missing `X-Content-Type-Options` + no HSTS | `curl -I https://dowiz.fly.dev/s/demo` |
| F-011 | APP | Cart/Checkout SSR | Server-rendered content, not empty shell (A1b, A1c) | Both render `<p>Loading...</p>` — require JS to populate | `curl https://dowiz.fly.dev/s/demo/cart` |
| F-012 | APP | Cart/Checkout brand colors | Consistent brand vars with menu page | Cart/Checkout use `#e63946`/`#f8f9fa`; Menu uses `#ea4f16`/`#121212` — different config sources | Compare menu SSR vs cart SSR CSS variables |
| F-013 | APP | Error code UX | Distinct UX per status code (A5) | 401/404/500 share generic JSON format; no 403/422/429 differentiation visible | Test each error path |
| F-014 | APP | `theme.css` endpoint | Public theme CSS (E8) | Returns **500 Internal Server Error** | `GET /public/locations/demo/theme.css` → 500 |
| F-015 | APP | `manifest.webmanifest` | PWA manifest (A8) | Returns **500 Internal Server Error** | `GET /s/demo/manifest.webmanifest` → 500 |
| F-016 | APP | `POST /api/telemetry` | Telemetry collection | Returns **500 Internal Server Error** | `POST /api/telemetry` → 500 with `correlationId: "unknown"` |
| F-017 | APP | i18n `en` locale | Consistent English product translations (A8e) | Inconsistent — some products have `data-text-en`, most don't; menu categories only have `data-text-uk` | Compare English SSR: `curl "https://dowiz.fly.dev/s/demo?locale=en"` |

---

## MINOR (4)

| ID | Layer | Target | Expected | Actual | Repro |
|----|-------|--------|----------|--------|-------|
| F-018 | APP | Demo tenant menu | Clean test data | 3 duplicate "Pizzas" categories with duplicate products | `GET /public/locations/demo/menu` → 3 identical categories |
| F-019 | DATA | `correlationId` | Traceable error correlation | 500 errors return `"correlationId":"unknown"` — not useful for debugging | theme.css / manifest / telemetry errors |
| F-020 | APP | Rate-limit headers | Present for write endpoints (E5a) | Rate-limit headers present but `/api/orders` returns 404 so cannot verify throttle behavior | `x-ratelimit-*` headers on SSR responses |
| F-021 | DATA | Domain list | Validate `dowiz.org` DNS | `dowiz.org` does not resolve — all canonical URLs in SSR pages point to unreachable domain | Check `<link rel="canonical" href="https://dowiz.org/s/demo">` in SSR output |

---

## DEGRADED (from /health)

| Component | Status | Detail |
|-----------|--------|--------|
| **Settlement** | degraded | `relation "settlements" does not exist` |
| **Backup restore** | degraded | `last_verified_at: null`, `last_result: "failed"`, never verified |
| **Fallback** | degraded | `0/59 locations have fallback phone configured (0%)` |
| **Overall** | **degraded** | Postgres, workers, MessageBus, R2, Telegram all OK |

---

## GREEN / PASSING (14 checks)

- E1a: TLS valid (Let's Encrypt `*.fly.dev`, expires 2026-07-21)
- E2a: HTTP→HTTPS redirect (301)
- E2e: Zero cookies on all endpoints
- E3a: /health returns 200 with honest `degraded` status
- E3b: /health granular (PG, Redis, workers separate)
- E3c: /health degraded for non-critical (not 503)
- E4b: `menu_version` header present for cache invalidation
- E4d: Embed mode has distinct rendering (`?embed=1`)
- E5b: No bot challenges on public SSR/embed pages
- A1a: SSR menu page renders full HTML with CSS variables + JSON-LD
- A8a/A8b/A8c: Responsive grid (mobile-first 2→3→4 columns)
- D1a: Cross-tenant API returns 401 (auth gate before tenant gate)
- D2a: No secrets in JS bundle (312KB scanned)
- D2b: Source maps not exposed (404)
- D2c: No PII in /health output
- D3a: Zero cookies confirmed on all routes
- D5b: No PII in cached SSR menu responses

---

## MISSING / NOT DEPLOYED

| Component | Impact | Reference |
|-----------|--------|-----------|
| Cloudflare CDN/WAF edge | Entire edge layer (E2–E5, cache, security) missing | spec v3.1 §2: "Cloudflare перед усім" |
| POST /api/orders | Critical client path cannot complete | contract-map: orders.ts |
| Owner auth (magic-link, Google) | Cannot test any owner flows | contract-map: auth endpoints |
| OTP send/verify | Customer verification not functional | contract-map: customer/otp.ts |
| static site (`dowiz.org`) | Legal pages, OAuth domain verification unreachable | spec E8 |
| Fallback phones | Offline degradation non-functional for all 59 locations | /health |
| Settlement system | Settlement table not created | /health: `relation "settlements" does not exist` |
| Albanian (sq) i18n | Primary market locale not rendered in SSR | spec: sq = default |

---

## RECOMMENDED NEXT ACTIONS

1. **BLOCKER → Deploy Cloudflare edge** with Full-strict SSL, HSTS, security headers, asset caching with immutable TTLs
2. **BLOCKER → Deploy `POST /api/orders`** — core order creation pipeline
3. **BLOCKER → Deploy owner auth** — magic-link + Google OAuth scaffold
4. **BLOCKER → Add Albanian (`sq`) translations** to SSR menu templates
5. **BLOCKER → Configure fallback phones** for all test locations
6. **MAJOR → Fix `/api/telemetry`, `theme.css`, `manifest.webmanifest` 500 errors**
7. **MAJOR → SSR-render cart and checkout pages** (not just empty SPA shells)
8. **After above: re-run full audit for Phase 2 app flows (A1–A8)**

---

*dowiz / DeliveryOS · Deployment Audit Findings · read-only acceptance audit against dowiz.fly.dev (staging) · VERDICT: NO-GO*
