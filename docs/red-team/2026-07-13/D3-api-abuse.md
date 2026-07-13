# D3 — API Abuse & Auth Red-Team (dowiz)

**Date:** 2026-07-13
**Operator:** adversarial API-abuse specialist (red-team lane D3)
**Targets:** `https://dowiz-staging.fly.dev` (primary, probed) · `https://dowiz.fly.dev` (prod, read-only/gentle)
**Contract source:** `/root/dowiz/attic/apps-api/src/routes/**` (the TS Fastify API — moved to `attic/` on branch `feat/decentralized-pq-protocol`, but this is what is deployed live)
**Method:** derived the route/auth model from code, then probed live with `curl`. Non-destructive: no DoS, no data exfil, no writes. One prod login was performed with a *known documented test credential* to prove token issuance; the token was **not** reused against any data endpoint.

---

## 1. Bottom line

The **authorization** surface is solid: every owner/admin/courier/customer endpoint returns `401` unauthenticated, JWT is RS256-only with `alg=none` and garbage tokens rejected, the dev-login/mock-auth backdoors fail closed on prod (`404`), CORS default is restrictive, no `.env`/`.git`/source-maps/swagger are exposed, and the error handler suppresses stack traces. Method/verb/path-normalization tricks did not bypass auth.

The **posture is dragged down by one critical issue and a cluster of medium/low abuse vectors**:

- 🔴 **CRITICAL:** a **known, weak, repo-documented credential** (`test@dowiz.com` / `test123456`) is a **live, working OWNER login on PRODUCTION** — it returns a real RS256 owner access-token + refresh-token scoped to a real prod location.
- 🟠 **MEDIUM:** rate limits key on `req.ip`, which behind the Fly proxy is a **shared/global bucket** (no `trustProxy`, no `Fly-Client-IP` keying on the global + login limiters) → global login-lockout and global request-budget DoS, plus weakened per-attacker isolation.
- 🟠 **MEDIUM:** the public order-create limiter keys on the **attacker-controlled** `body.customer.phone` → rotate the field to bypass the 5/min throttle (order spam).
- 🟡 **LOW–MED:** anonymous Telegram webhook on **staging** (`POST /webhook/telegram/` → `200`, secret empty); design relies on URL-path secrecy + an optional header that is bypassable by omission. Prod is protected.
- 🟡 **LOW:** unauthenticated `/health` enumerates full internal subsystem topology and is `rateLimit:false` over ~11 DB queries + external calls (recon + amplification). User-enumeration oracle on login. CSP missing on the SPA shell / API JSON (present but weak on SSR pages).

Verdict: **prod is not wide open, but the test-account credential is a full-owner foothold that must be closed immediately.** Everything else is hardening.

---

## 2. Endpoint probe table (observed live, staging unless noted)

| Endpoint | Method | Auth required? | Observed (unauth) | Finding |
|---|---|---|---|---|
| `/api/owner/locations/:id/dashboard` | GET | yes | `401` | OK — guarded |
| `/api/owner/locations/:id/gdpr/export` | GET | yes | `401` | OK — guarded |
| `/api/courier/me` | GET | yes | `401` | OK — guarded |
| `/api/courier/assignments` | GET | yes | `401` | OK — guarded |
| `/api/courier/shifts` | GET | yes | `401` | OK — guarded |
| `/api/customer/orders` | GET | yes | `401` | OK — guarded |
| `/api/admin/backups` | GET | yes | `401` | OK — guarded (admin has its own preHandler; not in the prefix list) |
| `/api/admin/notification-audit` | GET | yes | `401` | OK — guarded |
| `/api/admin/fallback` | GET | yes | `404` | route/method differs; not reachable as GET |
| `/api/orders` | POST | no (public storefront) | `415` (no CT); `400` (bad body) | **F3** rate-limit bypass via body-controlled key |
| `/api/auth/local/login` | POST | no | `401` invalid / **`200` valid** | **F1 (prod)** weak test cred → owner token; **F6** enum oracle; rate-limited (F-good) |
| `/api/dev/mock-auth` | POST | dev-flag | `404` (prod & stg no-secret) | OK — fails closed; `200` on stg only *with* the known `x-dev-auth-secret` |
| `/webhook/telegram/` | POST | url-secret | **stg `200`**, **prod `404`** | **F4** anonymous forged updates on staging (secret empty) |
| `/health` | GET | no | `200` + topology | **F5** info disclosure + `rateLimit:false` amplification |
| `/livez` | GET | no | `200` | OK — liveness only |
| `/api/customer/locations/:slug/otp/send` | POST | no (pre-auth) | `404` (unknown slug) | OK — OTP globally disabled + per-phone + DB throttle + lockout |
| `/s/:slug` (SSR menu) | GET | no | `200` + CSP | **F7** CSP present but weakened (`unsafe-inline`/`unsafe-eval`) |
| `/` (SPA shell, static) | GET | no | `200`, **no CSP** | **F7** CSP absent on shell + API JSON |
| `/.env` `/.git/*` `*.js.map` `/openapi.json` `/swagger.json` | GET | — | `404` | OK — no exposure |

---

## 3. Findings

### F1 — 🔴 CRITICAL: Known weak test credential is a live OWNER login on PRODUCTION
- **Endpoint:** `POST https://dowiz.fly.dev/api/auth/local/login`
- **Type:** Broken authentication / weak & documented credential (OWASP API2:2023).
- **Reproduction (observed live):**
  ```bash
  curl -s -X POST -H 'Content-Type: application/json' \
    -d '{"email":"test@dowiz.com","password":"test123456"}' \
    https://dowiz.fly.dev/api/auth/local/login
  ```
  Observed response `200`, body keys `["access_token","refresh_token","userId","activeLocationId"]`:
  - `access_token`: valid RS256 JWT (`{"alg":"RS256","kid":"2"}`), payload `role: owner`, `exp`/`iat` set, 660 chars
  - `refresh_token`: present (7-day rotating family)
  - `userId`: `00bf019a-49a3-4c16-817f-15554a4274a4`
  - `activeLocationId`: `3625d9b3-e53d-48e7-9d7b-84abf68326f5`
  - Control: wrong password → `401`; unknown email → `401 INVALID_CREDENTIALS`. Confirms this is the **real argon2 path** (not the dev bypass, which is off on prod), i.e. a genuinely valid stored credential.
- **Why it is critical:** `test@dowiz.com` / `test123456` is documented in project memory/fixtures (`docs`/memory: "test@dowiz.com/test123456 via POST /api/auth/local/login") and almost certainly in git history — it is effectively public. It grants a full **owner** token, which unlocks the owner API for location `3625d9b3…`: dashboard, order confirm/reject, customer contact reveal, **GDPR export of that location's customers**, settlements, menu mutation, storefront pause. If any real customer PII/orders flow through this location, this is a data-exposure, not just a demo foothold.
- **Impact:** Full account takeover of a production owner account with a guessable, published password. Owner-scoped data read + mutation on prod.
- **Fix (do immediately):**
  1. Disable or rotate `test@dowiz.com` on the **prod** DB (`UPDATE users SET password_hash = NULL` or delete the seed) — do not ship seeded human-owner logins to prod.
  2. Remove the plaintext credential from fixtures/docs/memory and rotate anything derived from it.
  3. Add a boot-guard / migration assertion that no seed/demo owner with a known password exists in a prod-tagged environment.
  4. Enforce a password policy (length/entropy) on `users.password_hash` creation so `test123456`-class passwords cannot be set.

### F2 — 🟠 MEDIUM: Rate limits key on `req.ip` behind Fly → shared/global bucket
- **Where:** global limiter `server.ts` `fastify.register(fastifyRateLimit,{max:100,timeWindow:'1 minute'})` (no `keyGenerator`); login limiter `routes/auth/local.ts:39` (`max:5/min`, no `keyGenerator`); also `routes/customer/track.ts:34` (`req.ip`). No `trustProxy` is set on the Fastify constructor anywhere in the codebase.
- **Type:** Improper rate-limit keying / availability (OWASP API4:2023).
- **Evidence:**
  - Rate limiting **is present and fires** (CONFIRMED live): a burst of login POSTs returned `429` with `retry-after: 26` and `x-ratelimit-remaining: 0`.
  - The code itself proves the authors know `req.ip` is not the client behind Fly: `routes/public/access-requests.ts` reads `Fly-Client-IP` for *its* keyGenerator and logs `"Fly-Client-IP missing … rate-limit degraded to a shared bucket"`. The global + login limiters do **not** do this → they fall back to `req.ip` = the Fly proxy peer address, which is shared across clients.
- **Impact:**
  - **Global login lockout:** the 5/min login limit is effectively global; an attacker making 5 login attempts/min can block *all* users from logging in.
  - **Global request-budget DoS:** the 100/min global cap is shared, so ~100 req/min from one source throttles the entire app.
  - Conversely, brute-force is capped globally (a mild positive) but at the cost of availability.
- **Status:** limiter presence CONFIRMED live; shared-bucket amplification PLAUSIBLE (strong code + author-comment evidence; not isolated live because it needs a second source IP).
- **Fix:** set Fastify `trustProxy: true` (Fly sets `X-Forwarded-For`) **or** give the global + credential limiters a `keyGenerator` that reads `Fly-Client-IP` (as `access-requests` already does). Then per-client isolation is restored.

### F3 — 🟠 MEDIUM: Public order-create throttle bypassable via attacker-controlled key
- **Endpoint:** `POST /api/orders` (public storefront route, no token by design).
- **Code:** `routes/orders.ts:70` — `keyGenerator: req => req.body?.customer?.phone ? 'phone:'+phone : req.ip`.
- **Type:** Rate-limit bypass / resource abuse (OWASP API4/API6).
- **Reproduction:** the limiter is keyed on a value **in the request body**. Rotating `customer.phone` (or omitting it and rotating source) yields a fresh 5/min bucket per value, so an attacker can submit far more than 5 orders/min by cycling the phone field. (Not exercised against prod to avoid writing order rows; endpoint confirmed reachable — `POST /api/orders` returns `415` without a content-type and `400` on a bad body, i.e. it is live and public.)
- **Impact:** Order spam / inventory & notification abuse (each order fans out Telegram notifications + kernel pricing + DB writes) against a real location.
- **Fix:** key the order limiter on `Fly-Client-IP` (server-observed), not on a client-supplied body field. Keep an *additional* per-phone soft cap, but never let a body field be the sole/first key. Consider a proof-of-work or captcha on anonymous order-create.

### F4 — 🟡 LOW–MEDIUM: Anonymous Telegram webhook on staging; weak webhook auth design
- **Endpoint:** `POST /webhook/telegram/${TELEGRAM_BOT_SECRET}` (`routes/telegram-webhook.ts:36`).
- **Type:** Missing/weak webhook authentication (OWASP API2/API8).
- **Reproduction (observed live):**
  ```bash
  curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'Content-Type: application/json' \
    -d '{"update_id":1}' https://dowiz-staging.fly.dev/webhook/telegram/
  # -> 200   (staging: TELEGRAM_BOT_SECRET is empty, route mounts at /webhook/telegram/, NO validation)
  curl ... https://dowiz.fly.dev/webhook/telegram/    # -> 404  (prod: secret set, empty path does not exist)
  ```
- **Design weaknesses (code):**
  1. When `telegramBotSecret` is empty (staging), `if (telegramBotSecret)` is falsy → **all** validation is skipped and any anonymous POST is processed (`{ok:true}`).
  2. Even when the secret *is* configured, the `x-telegram-bot-api-secret-token` header check only rejects a **present-but-wrong** header; a **missing** header is logged and **processed anyway** (backward-compat). So the only real gate is knowledge of the URL-path secret.
- **Impact (staging):** an attacker can inject forged `message` / `callback_query` updates. Business-logic authority checks (`owner_notification_targets` linkage by `chatId` + membership) blunt most actions, but forged updates still drive outbound `api.telegram.org` calls (`answerCallbackQuery`/`sendMessage`) and DB work, and the `/start login_<uuid>` path could bind an attacker-chosen `telegram_user_id` to a login token if a pending token UUID is known (unguessable in practice). Prod is not exposed.
- **Fix:** fail **closed** when the bot secret is unset (do not mount the webhook, or `500`/reject). Always require and validate the `x-telegram-bot-api-secret-token` header (constant-time compare) rather than treating a missing header as trusted. Add a rate limit to the webhook route.

### F5 — 🟡 LOW: Unauthenticated `/health` topology disclosure + amplification
- **Endpoint:** `GET /health` (both staging and prod), `routes/health.ts:65`, `config:{ rateLimit:false }`.
- **Type:** Information disclosure + DoS amplification (OWASP API7/API4).
- **Reproduction (observed live, prod):**
  ```json
  {"status":"degraded","checks":{"postgres":{"status":"ok","latencyMs":4},"workers":{...},
   "messageBus":{"status":"ok"},"telegram":{"status":"ok","latencyMs":35},"r2":{"status":"ok"},
   "settlement":{...},"anonymizer":{...},"backup":{...},"backup_restore":{...},
   "fallback":{"status":"degraded"},"free_tier":{"status":"ok"}}}
  ```
- **Impact:** Any anonymous caller learns the full internal architecture (Postgres, Redis/message-bus, Telegram, Cloudflare R2/S3, settlement/anonymizer/backup subsystems), per-check latencies (timing recon), and current **degraded** operational state. The endpoint runs ~11 sequential DB queries + a Telegram `getMe` + an R2 `HeadBucket` **with rate-limiting explicitly disabled**, so repeated calls are a cheap DB/load amplifier. (The payload is already minimized to `{status,latencyMs}` — no driver text — which is good; the residual issue is the subsystem enumeration + un-limited cost.)
- **Fix:** require an ops token (or internal-network restriction) for the rich `/health`; keep `/livez` public for Fly. If it must stay public, collapse to a single `{status}` and re-enable a modest rate limit.

### F6 — 🟡 LOW: User-enumeration oracle on login
- **Endpoint:** `POST /api/auth/local/login`.
- **Type:** Account enumeration (OWASP API3).
- **Reproduction (observed live):**
  - Unknown email → `{"code":"INVALID_CREDENTIALS","error":"Invalid email or password"}` (`401`).
  - Existing OAuth-only account (`test@dowiz.com` on staging, no password hash) → `{"code":"WRONG_AUTH_METHOD","error":"Account uses another sign-in method"}` (`401`).
- **Impact:** the distinct `WRONG_AUTH_METHOD` response confirms an account exists **and** reveals its sign-in method, letting an attacker enumerate valid accounts / target OAuth phishing.
- **Fix:** return a single uniform `INVALID_CREDENTIALS` message for all failure modes (no-user, wrong-password, OAuth-only). Handle the "wrong method" hint only after successful auth or via a separate authenticated flow.

### F7 — 🟡 LOW: Inconsistent / weak Content-Security-Policy
- **Type:** Missing security header / weak CSP (OWASP API8).
- **Observed live:**
  - SPA HTML shell `GET /` (static) → **no** `Content-Security-Policy` header (only `X-Frame-Options`, nosniff, referrer-policy).
  - Protected API JSON (`/api/owner/…` `401`) → **no** CSP.
  - SSR menu `GET /s/:slug` → CSP present but weakened: `script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io`, `style-src … 'unsafe-inline'`.
- **Root cause (code):** `securityHeadersPlugin` (`lib/security/headers.ts`) is registered with `fastify.register(...)` **without** a `fastify-plugin` wrapper, so its `onRequest`/`onSend` CSP hooks are encapsulated to that (route-less) child context and never apply to the sibling-registered API routes or the static SPA shell. SSR pages set CSP inline in their handler, which is why only they carry it — and that inline CSP uses `unsafe-inline`/`unsafe-eval`, which largely defeats CSP's XSS protection.
- **Impact:** the SPA shell (where the app's JS executes) and API responses ship without CSP; where CSP does apply it does not meaningfully constrain inline/eval script. Defense-in-depth gap, not a direct exploit.
- **Fix:** wrap `securityHeadersPlugin` with `fastify-plugin` (or set CSP in the root `onRequest` alongside the other baseline headers) so CSP reaches all responses incl. the static shell; drop `'unsafe-inline'`/`'unsafe-eval'` in favour of nonces/hashes (the code already supports a `nonce` param).

---

## 4. Missing / weak controls (summary)

| Control | State | Action |
|---|---|---|
| **Prod credential hygiene** | ❌ known weak owner cred works on prod (F1) | remove/rotate `test@dowiz.com`; password policy; prod seed boot-guard |
| **Rate-limit client keying** | ⚠️ `req.ip` = shared Fly proxy on global+login+track limiters (F2) | `trustProxy:true` or `Fly-Client-IP` keyGenerator everywhere |
| **Order-create throttle key** | ⚠️ keyed on body `customer.phone` (F3) | key on server-observed IP; body field only as secondary |
| **Webhook auth** | ⚠️ URL-secret only; header optional; fails **open** when secret empty (F4, staging) | fail closed when secret unset; always validate header |
| **`/health` exposure** | ⚠️ topology + un-rate-limited (F5) | auth-gate or minimize; re-enable limit |
| **Uniform auth errors** | ⚠️ `WRONG_AUTH_METHOD` enumeration (F6) | single generic message |
| **CSP** | ⚠️ absent on shell/API; weak on SSR (F7) | fastify-plugin wrap; nonce; drop unsafe-inline/eval |
| **CORS** | ✅ restrictive default (no reflected origin, `credentials:false`); `ACAO:*` only on public menu/order (no credentials) | none |
| **Baseline headers** | ✅ nosniff, `X-Frame-Options: SAMEORIGIN`, referrer-policy on all; HSTS on prod | none (add HSTS on staging if desired) |
| **JWT** | ✅ RS256-only, `alg=none` + garbage rejected (`401`) | none |
| **Dev/test backdoors** | ✅ dev-login + `/api/dev/mock-auth` fail closed on prod (`404`) | none |
| **Authorization** | ✅ all owner/admin/courier/customer endpoints `401` unauth | none |
| **Error handling** | ✅ no stack traces / 500 internals leaked | none |
| **Info disclosure** | ✅ no `.env`/`.git`/source-maps/swagger | none |
| **Verb/path tampering** | ✅ TRACE/case/trailing-slash/double-slash/`%2e` did not bypass auth | none |

---

## 5. Notes on method / caveats
- Deployed live server is the TS Fastify API whose source now lives in `attic/apps-api/` (the branch checked out at `/root/dowiz` has since restructured, but staging/prod still serve this build). All findings are against the **deployed** behavior.
- `CONFIRMED` = observed in a live response this session. `PLAUSIBLE` = code-confirmed, not fully isolated live (F2 shared-bucket, F3 bypass, F4 forgery side-effects) to stay within non-destructive/gentle limits.
- No data was created, mutated, or exfiltrated. The single prod login (F1) captured only the token *shape*; the token was discarded, not replayed against data endpoints.
