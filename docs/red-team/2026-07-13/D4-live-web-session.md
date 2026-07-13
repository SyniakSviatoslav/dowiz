# D4 — Live Web Red-Team Session (browser + curl)

**Date:** 2026-07-13
**Targets:** `https://dowiz-staging.fly.dev` (primary, interactive) · `https://dowiz.fly.dev` (prod, gentle read-only)
**Operator:** hostile-rival browser session (non-destructive: no DoS, no defacement, no real-customer-data exfil)
**Tooling:** playwright-test MCP (browser drive/JS eval/network/DOM) + curl (headers, bundles, API). The `browser-use` MCP was unstable (CDP init failures) and was abandoned in favour of playwright-test MCP — all live browser evidence below is from playwright-test.

---

## 1. Bottom line — what a rival learns/gains from the live client

- **A working owner account on PRODUCTION.** The documented test credential `test@dowiz.com` / `test123456` logs in via `POST /api/auth/local/login` on **both staging and prod** and returns a valid `role: owner` JWT. This is the single most damaging takeaway — a trivially-guessable, publicly-documented password grants owner-scoped API access to a live prod location. **CONFIRMED.**
- **The entire API + feature surface, for free.** JS bundles are readable (no source maps, but the code is plain) and disclose the full owner/courier/customer/public endpoint contract plus ~15 `VITE_*` feature flags (the unreleased-feature roadmap). **CONFIRMED.**
- **No real secrets in the client.** No Supabase service-role key, no anon key, no private tokens, no internal hostnames leaked. The only "SECRET"/"password" hits are React internals and login-form field labels. Secrets-in-client verdict: **CLEAN.** **CONFIRMED.**
- **XSS defence holds on the public storefront.** React auto-escaping is intact, no `dangerouslySetInnerHTML`/`eval`/`innerHTML` in app code, and reflected (query-param) + search-box marker payloads did **not** execute. **CONFIRMED.**
- **But the security-header posture is inconsistent and the CSP is weak** — CSP is applied only to `/s/:slug` storefront routes, not to the SPA shell (`/`, `/admin/*`) where the owner CRM actually runs, and the policy itself uses `unsafe-inline` + `unsafe-eval`. Combined with bearer-tokens-in-localStorage, any future XSS is high-impact.
- **Trust-cue smell present:** the shipped courier-earnings surface animates money from 0 → value (400 ms ease-out-cubic count-up). Not on the public menu (static prices there). **CONFIRMED in bundle.**

---

## 2. Findings

### F1 — Publicly-documented test owner account is live on PRODUCTION · Severity: HIGH
- **Where:** `POST https://dowiz.fly.dev/api/auth/local/login` (and staging).
- **Evidence:**
  - Prod: `POST /api/auth/local/login {"email":"test@dowiz.com","password":"test123456"}` → `HTTP 200`, body contains `access_token`; decoded JWT payload `role: owner`, `kid: 2`, TTL 1 day, `userId 00bf019a-49a3-4c16-817f-15554a4274a4`, `activeLocationId 3625d9b3-e53d-48e7-9d7b-84abf68326f5`.
  - Staging: same creds → `HTTP 200`, `role: owner`, `kid: dev`, TTL 7 days, location `28239442-63a1-431e-8cab-2e4ed64ab8e7`.
  - The password is documented in project memory ("Test owner fixture") and in this engagement's brief; it is also trivially guessable.
- **Impact:** Any rival who reads the repo/docs or guesses the password obtains an owner-role session token on prod. Owner APIs (`/api/owner/*` — orders, customers, menu, analytics, courier management) are auth-gated but this credential *is* valid auth. At minimum it is a foothold into the owner CRM for that prod location; the `/api/auth/local/login` "local" password path existing on prod at all bypasses the normal customer-OTP/owner-OAuth flow. *(Not exercised against data endpoints — non-destructive rule. Impact is owner-scoped access; exact reachable data not enumerated.)*
- **Fix:** Remove/disable the local-password test account and ideally the `/api/auth/local/login` route in prod. If required for E2E, gate it behind a non-prod env flag (e.g. `DEV_AUTH_SECRET` present only on staging) and fail-closed in prod; rotate/retire the `test@dowiz.com` account.

### F2 — Session token in localStorage (no HttpOnly cookie) + long TTL on staging · Severity: MEDIUM
- **Where:** `apiClient` chunk (`/assets/apiClient-Y7ZD009M.js`); live localStorage.
- **Evidence:** Bundle stores `dos_access_token` / `dos_refresh_token` in `localStorage` (mirrored to `sessionStorage`) and sends `Authorization: Bearer ${token}`; no `Set-Cookie` on login (login returns the token in the JSON body). Staging JWT TTL = 7 days (`exp-iat = 604800s`), `kid: dev`. Live check on the public storefront: `document.cookie === ""`, tokens absent until authenticated.
- **Impact:** Tokens are readable by any JavaScript, so a single XSS anywhere in the app yields full owner-session theft with no cookie protections (no HttpOnly/Secure/SameSite). Staging's 7-day TTL widens the window. (Prod is better: 1-day TTL, real `kid: 2`.)
- **Fix:** Prefer an HttpOnly + Secure + SameSite=Lax session cookie for the owner session; if bearer-in-JS is retained, shorten staging TTL to match prod and tighten CSP (F3) so XSS can't reach the token.

### F3 — CSP is inconsistently applied and permissive · Severity: MEDIUM
- **Where:** all HTML/asset responses.
- **Evidence:**
  - CSP header present on `/s/:slug` **only**. Absent on the SPA shell `GET /` and on `GET /admin` (200 SPA shell — the owner CRM) and on JS assets. `curl -D-` on `/admin` returns only `x-frame-options`, no `content-security-policy`.
  - The policy that *is* served: `script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io`.
- **Impact:** The most sensitive pages (owner/courier admin) render with no CSP at all. Where CSP exists, `unsafe-inline` + `unsafe-eval` largely nullify it as an XSS control, and `cdn.tailwindcss.com` (Tailwind Play CDN) is a full JS runtime — a supply-chain or config-injection foothold executes script.
- **Fix:** Serve one hardened CSP on every HTML response (`/`, `/admin/*`, `/s/*`). Drop `unsafe-inline`/`unsafe-eval` in favour of nonces/hashes; self-host Tailwind (the bundle already self-hosts Tabler icons — do the same for Tailwind and remove the CDN allowance).

### F4 — Missing HSTS on staging · Severity: LOW
- **Where:** all staging responses.
- **Evidence:** Prod sends `strict-transport-security: max-age=31536000; includeSubDomains`; staging sends none on any route checked.
- **Impact:** Staging is downgrade/SSL-strip-eligible on first contact. Low (staging), but staging is used for validated pre-prod E2E.
- **Fix:** Emit HSTS on staging too (can use a shorter max-age without preload).

### F5 — Full API contract + feature-flag roadmap disclosed in client bundle · Severity: LOW / INFO
- **Where:** `/assets/*.js` chunks (AdminRoutes, CourierRoutes, ClientRoutes, useWebSocket, apiClient, publicApi).
- **Evidence:** Extracted endpoint map includes `/api/owner/menu/products`, `/api/owner/customers`, `/owner/locations/:id/couriers`, `/owner/locations/:id/notifications/telegram/connect-init`, `/courier/assignments/:id/{accept,reject,picked-up,delivered}`, `/customer/locations/:id/otp/{send,verify}`, `/api/claim/request`, etc. Feature flags exposed: `VITE_TMA_ENABLED`, `VITE_TG_CATEGORY_GATING`, `VITE_ACCESS_GATE_PUBLIC_ENABLED`, `VITE_MEDIA_RICH_ENABLED`, `VITE_UNDO_REDO_ENABLED`, `VITE_MENU_CHARACTERISTICS_{ENABLED,FILTER,COMPARISON}`, `VITE_MENU_ALLERGEN_FILTER`, `VITE_PULL_TO_REFRESH_ENABLED`, `VITE_PAPER_SKIN`, `VITE_WS_BASE_URL`, `VITE_TILE_STYLE_URL`, `VITE_API_BASE_URL`.
- **Impact:** Normal for an SPA, but it hands a rival the complete attack surface and the unreleased-feature roadmap. Positive: source maps return **404** (not exposed) — good hygiene.
- **Fix:** No action required for correctness; ensure no `VITE_*` flag gates a *security* control (flags observed are UX-only). Treat the endpoint list as public and rely on server-side authz (which holds — see Positive Controls).

### F6 — Money count-up animation on courier earnings (trust-cue smell) · Severity: LOW
- **Where:** `/assets/CourierRoutes-U1fVTptk.js` (courier earnings surface).
- **Evidence:** Shipped component: `const [a,b]=useState(reduce?t:0); ... const u=Math.min((Date.now()-v)/d,1),p=1-Math.pow(1-u,3); b(Math.round(m+(t-m)*p)) ...` — animates a money value from `0` → `t` over `d=400ms` with ease-out-cubic via `requestAnimationFrame`, respecting reduced-motion (`reduce` jumps straight to `t`). Live probe on the **public** storefront showed static prices (menu cards and product-modal unit price did not tween across 55 animation-frame samples) — the count-up is on the auth-gated earnings surface, not the public menu.
- **Impact:** Financial figures visibly counting up from 0 is the known project trust-cue smell (money should not appear to "tick up"). Not a security hole. Respects reduced-motion, which mitigates.
- **Fix:** Render financial totals statically, or only animate genuine deltas — never `0 → value` on money.

### F7 — sw.js served immutable for 1 year · Severity: LOW / INFO
- **Where:** `GET /sw.js`.
- **Evidence:** `cache-control: public, max-age=31536000, immutable`. Body is a no-op worker: `addEventListener('install', skipWaiting); addEventListener('activate', clients.claim)` — **no fetch handler, no caching**.
- **Impact:** No cache-poisoning / stale-auth / scope risk because the worker caches nothing. But `immutable` on a service worker is an anti-pattern (browsers cap SW update checks at 24h regardless).
- **Fix:** Serve `sw.js` with `Cache-Control: no-cache` for cleanliness.

### F8 — HEAD /admin 404 vs GET /admin 200 · Severity: INFO
- **Evidence:** `HEAD /admin → 404`, `GET /admin → 200 text/html` (SPA shell). Minor SPA-fallback method-handling inconsistency; not security-relevant (noted so it isn't mistaken for a server-side admin guard — it is not one).

### Positive controls verified (defence that held)
- **No secrets in client** — grep of `index.js` + `vendor.js` + all 16 route chunks for supabase/service_role/anon-key/bearer/eyJ.../AIza/sk_/whsec found only React internals and form labels. **CONFIRMED CLEAN.**
- **XSS blocked** — query-param payload `?q=<img src=x onerror=...>` and a search-box `<img onerror>` payload both failed to execute (`window.__xss__`/`__xss2__` stayed undefined; raw string only present as escaped text). No `dangerouslySetInnerHTML`/`eval`/`document.write`/`innerHTML` in app chunks (13 `dangerouslySetInnerHTML` hits are all in `vendor.js` = React). **CONFIRMED.**
- **Server-side authz holds** — unauth `GET /api/owner/menu/products`, `/api/owner/customers`, `/api/owner/locations/:id/couriers` all → `401 {"error":"Unauthorized"}`. No client-side-only guard leak. **CONFIRMED.**
- **Clickjacking blocked** — `X-Frame-Options: SAMEORIGIN` on every response + `frame-ancestors 'self'` in the storefront CSP. **CONFIRMED.**
- **Login rate-limited** — `x-ratelimit-limit: 5` on `/api/auth/local/login`; `100` on storefront routes. **CONFIRMED.**
- **No source maps** — `*.js.map` → 404. **CONFIRMED.**

---

## 3. Header / cookie table

### Security headers by route

| Header | staging `/` (SPA shell) | staging `/s/:slug` | staging `/admin` (SPA) | staging `/assets/*.js` | prod `/` | prod `/s/:slug` |
|---|---|---|---|---|---|---|
| `strict-transport-security` | **absent** | **absent** | **absent** | absent | `max-age=31536000; includeSubDomains` | present |
| `content-security-policy` | **absent** | present (weak) | **absent** | absent | **absent** | present (weak) |
| `x-frame-options` | SAMEORIGIN | SAMEORIGIN | SAMEORIGIN | SAMEORIGIN | SAMEORIGIN | SAMEORIGIN |
| `x-content-type-options` | nosniff | nosniff | nosniff | nosniff | nosniff | nosniff |
| `referrer-policy` | strict-origin-when-cross-origin | same | same | same | same | same |
| `access-control-allow-origin` | — | `*` | — | — | — | `*` |
| `permissions-policy` | absent | absent | absent | absent | absent | absent |
| `cross-origin-*-policy` (COOP/COEP/CORP) | absent | absent | absent | absent | absent | absent |
| `cache-control` | no-store | no-store | no-store | `public, max-age=31536000, immutable` | no-store | no-store |

**CSP value (storefront routes only):**
`default-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org https://plausible.io; frame-ancestors 'self'`

### Cookies

| Context | Cookies | Notes |
|---|---|---|
| Public storefront (anon) | **none** (`document.cookie === ""`) | No session/tracking cookie set for anon visitors. |
| Authenticated owner | **none** | Auth is bearer-token in `localStorage` (`dos_access_token`, `dos_refresh_token`) + `sessionStorage` mirror. Login returns the token in the JSON body — no `Set-Cookie`. See F2. |

localStorage on the public storefront held only non-sensitive UX state: `dos_cart_<slug>`, `dos_menu_prefs_<slug>`. No PII, no tokens (unauthenticated).

---

## 4. Secrets-in-client verdict

**CLEAN — no critical secret exposure.**
- No Supabase service-role key, no anon key, no private API tokens, no `DEV_AUTH_SECRET`, no internal/private hostnames in `index.js`, `vendor.js`, or any of the 16 lazy chunks.
- Source maps not served (`.js.map` → 404).
- Only client config exposed is expected `VITE_*` public feature flags + public map/tile/analytics/CDN hosts (F5), none of which are secrets.
- The one credential-shaped risk is **not** a leaked key but a **shipped weak account** (F1) — a configuration/account-hygiene problem, not a secret-in-bundle problem.

---

## Assets
- `assets/staging-storefront-dubin-sushi.png` — live staging storefront (`/s/dubin-sushi`), static menu prices.

## Method notes / non-destructive compliance
- Prod touched read-only: header reads, one login (own test account, not used against data endpoints), storefront render. No writes, no data enumeration, no OTP sends, no DoS.
- Staging login token was obtained but not used to mutate any location data.
- `browser-use` MCP failed to initialise CDP; pivoted to playwright-test MCP for all live browser evidence.
