# Security Audit — dowiz/DeliveryOS — 2026-07-03

READ-ONLY audit. Findings only; no code changed. Scope: JWT/auth middleware, the
`withTenant` tenant seam, RLS policies, WebSocket auth, `spa-proxy.ts`, the rate
limiter, owner/courier/customer route guards, raw `db.query` IDOR surface, input
validation, secrets handling, and SSRF/injection in menu-import + URL fetches.

> **Standing posture note (read first).** The `dowiz_api` DB role currently holds
> **BYPASSRLS** (the NOBYPASSRLS flip is council-deferred — *known, not re-reported*).
> Consequence: **RLS is inert right now** — the *active* cross-tenant control is the
> explicit `WHERE location_id = …` predicate on each raw query, not the policies.
> Therefore: (a) the **RLS findings below are latent** — they become live the instant
> the flip lands (or if any query runs on a non-BYPASSRLS role), and several of them
> mean *the flip will not actually achieve isolation*; (b) the **raw-query / route-guard
> findings are live today** because predicates are the only thing standing.

Excluded as already-known (per tasking): NOBYPASSRLS deferral; C2 `set_config(...,false)`
session-GUC leak (`onboarding.ts:75`, `spa-proxy.ts:771`); JWT-in-URL WS deprecation;
`deliveryos_api_user` rotated→`dowiz_api`; the shipped Tier-1 authz fixes; 8 dependabot alerts.

---

## Severity tally

| Severity | Count | Findings |
|----------|-------|----------|
| CRITICAL | 1  | F1 (cross-tenant order-status write) |
| HIGH     | 3  | F2 (GDPR cross-tenant PII erasure), F3 (`owner/couriers` missing role gate), F4 (`owner/courier-invites` missing role gate) |
| MED      | 5  | B1 (SSRF rebind), C1 (rate-limit key collapse), F5 (signals cross-tenant), F6 (telegram fail-open), F7 (invite-redeem ATO) |
| LOW      | 8  | C2, D1, F8–F13 |
| LATENT (RLS, post-flip) | 3 HIGH + 8 MED/LOW | Section E |

Live-today findings are F1–F7 (+ B1, C1); the Section E RLS gaps are latent under the
current BYPASSRLS posture. "VERIFIED SOUND" surfaces are recorded so they aren't re-walked.

---

## A. Auth / JWT / tenant seam — VERIFIED SOUND (no findings)

Recorded so the next auditor doesn't re-walk them:

- `packages/platform/src/auth/jwt.ts` — RS256 only; kid selected from the header **then**
  the signature is still verified against the trusted key; `alg=none` rejected; dev-kid
  accepted only when `NODE_ENV!=='production'` **and** a dev keypair is present; claims
  strict-parsed via Zod (`AuthToken` discriminated union, all ids `uuid`). No forgeable path.
- `apps/api/src/plugins/auth.ts` — courier tokens re-bound to a live `courier_sessions`
  row + membership every request; owner location access re-checks `status='active'`
  membership (ADR-0004), 404 (not 403) on cross-tenant to avoid existence leaks.
- `apps/api/src/plugins/dev-guard.ts` — `/dev|/api/dev` fail **closed**: require both
  `ALLOW_DEV_LOGIN` and a constant-time-compared `DEV_AUTH_SECRET`; 404 (not 401) on prod.
- `packages/platform/src/auth/tenant.ts` `withTenant` sets `app.user_id` (transaction-local);
  owner-menu tables (`products`/`categories`/`locations`/`import_sessions`) scope via
  `app_member_location_ids()` which reads `app.user_id` — so the GUC matches. Courier-ops
  tables use `app.current_tenant`, set by the courier routes. **Two intentional tenancy
  models, each consistent with its code** — not a mismatch.
- `apps/api/src/routes/auth/local.ts` — argon2 verify + timing-safe dev compare; inherits
  the global `clientIp` limiter (5/min). `courier/auth.ts` invite-redeem: row-locked invite,
  argon2 code+password, encrypted PII columns. Sound.

---

## B. SSRF — brand-extractor (menu-import `website` + `/api/owner/brand/generate`)

### B1. MED — SSRF via DNS-rebinding: per-hop `assertPublicUrl` re-resolves the hostname instead of pinning the validated IP
- `apps/api/src/lib/brand-extractor.ts:234` (`assertPublicUrl`), `:247-280` (`fetchText`),
  `:283` (`extractFromWebsite`); reached from `routes/owner/menu-import.ts:564`
  (`commit`, owner-auth) and `routes/spa-proxy.ts:616` (`/api/owner/brand/generate`, owner-auth).

**Violated invariant:** a server-side fetch must never reach an internal/link-local address.
**What's there (good):** protocol allowlist, `localhost` block, private/link-local/ULA IP
block (incl. `169.254`, `::1`, `fc/fd/fe80`), **manual redirect handling with
`assertPublicUrl` re-run on every hop**, 6 s timeout, 1.5 MB cap. This is a strong guard.
**Residual (real, and self-acknowledged in the code comment at `:255-257`):**
`assertPublicUrl` does a `dns.lookup`, but `fetch(currentUrl, …)` is then called with the
**hostname**, so Node re-resolves DNS independently. An attacker who controls
`evil.example.com` can answer the validation lookup with a public IP and the fetch lookup
with `169.254.169.254` / a Fly **6PN `.internal`** address (TOCTOU DNS rebind) → the fetch
lands on an internal target. Some content reflects back (og:image URL → `logoUrl`, `<title>`
→ name), so it is not purely blind.
**Exploit:** owner account → set venue `website`/brand URL to an attacker domain that rebinds
→ server fetches an internal Fly service / metadata; leaks partial content into the theme.
**Fix direction:** resolve once, then fetch against the **pinned IP** with a `Host` header
(undici `Agent`/`lookup` pin), or an egress allowlist — closing the lookup↔fetch gap the
comment already flags as a follow-up.

Menu-import LLM path itself is clean: OCR text is **PII-redacted before** the model
(`ai-ocr-parser.ts:456`), LLM output is **Zod-validated** (`:601`) before any DB write, and
`paddleOcr` uses `execFileSync(py, [script, tmp])` (array args — **no shell injection**).
All import DB writes are parameterized and carry `location_id = $1`.

---

## C. Rate limiting

### C1. MED — Per-phone / per-IP route limiters are non-functional: `keyGenerator` reads `req.body`, but `@fastify/rate-limit@10.3.0` runs on the `onRequest` hook (before body parsing) → key collapses to `req.ip`
- `apps/api/src/routes/orders.ts:76` — `keyGenerator: req => req.body?.customer?.phone || req.body?.phone ? … : req.ip`
- `apps/api/src/routes/customer/otp.ts:36` and `:114` — `req.body?.phone || req.ip`
- (plugin default hook confirmed: `node_modules/.pnpm/@fastify+rate-limit@10.3.0/…/index.js:11`
  `const defaultHook = 'onRequest'`; none of these routes override `hook`.)

**Violated invariant:** a per-target throttle must key on the target. At `onRequest` the body
is unparsed, so `req.body` is `undefined` → every request hits the `req.ip` fallback.
**Compounding:** Fastify is initialised with **no `trustProxy`** (`server.ts:82`), so `req.ip`
is the **Fly edge socket** address, not the client — the exact collapse `lib/client-ip.ts`
was written to avoid. These routes also **bypass the `clientIp()` single-source** that the
#9 hardening established for the global/auth/funnel limiters.
**Impact:** the intended per-phone anti-abuse keys never engage; requests share a Fly-edge
bucket → (a) legit users get false 429s (availability), (b) the per-phone SMS-bomb /
order-spam control is not enforced at the HTTP layer.
**Mitigation present:** OTP has **DB-backed backstops** that DO work — a `COUNT(*)` per-phone
gate on send (`otp.ts:55-63`) and an `attempts` counter on verify (`otp.ts:152,170-175`) —
so OTP brute-force/bomb is still bounded; and OTP is globally disabled (`OTP_ENABLED`). The
`POST /orders` phone throttle has **no** equivalent backstop → order-spam is effectively
unthrottled per-phone. `customer/track.ts:34` and `courier/shifts.ts:316` likewise key on
raw `req.ip` / `authorization`, diverging from `clientIp()`.
**Fix direction:** set `hook:'preHandler'` on these route limiters (body available) or key on
`clientIp(request)`; add a per-phone DB backstop to `POST /orders`.

### C2. LOW — Rate limiter is in-memory / per-instance; effective limit scales with machine count
- `apps/api/src/server.ts:360` registers `@fastify/rate-limit` with **no `redis` store**;
  `fly.toml` has `auto_stop_machines=false` + multiple `[[vm]]` blocks.

**Impact:** with N running machines the real ceiling is `max × N` (requests load-balance
across instances, each with its own LRU). Brute-force/flood surfaces (login, access-request,
funnel) weaken proportionally to horizontal scale. Latent while single-machine.
**Fix direction:** back the limiter with the already-present Redis (`fastify.redis`) so the
counter is shared cross-instance.

---

## D. WebSocket (`apps/api/src/websocket.ts`) — VERIFIED SOUND, one low note

Room authz is tight: customer → only `order:<own orderId>`; owner → `ownerRoomVerdict`
(live `status='active'` membership, order rooms JOIN'd on the order's location); courier →
tri-state binding check; **fan-out re-authz guards** re-derive owner + courier membership on
every frame (revocation ≤TTL). No cross-tenant subscribe or relay path found.

### D1. LOW — No per-connection WS message-rate cap → authenticated DB-query amplification
- `websocket.ts:388` (`subscribe`) → each `order:` subscribe triggers a
  `courierRoomVerdict`/`ownerRoomVerdict` DB round-trip; a logged-in courier/owner can flood
  `subscribe`/`unsubscribe` frames with no throttle → connection-pool pressure (authenticated DoS).
**Fix direction:** cap messages/sec per socket (cheap token bucket on the `ws.on('message')` path).

---

## E. RLS policy gaps (LATENT under BYPASSRLS — become live at the NOBYPASSRLS flip)

Authoritative sweep of `packages/db/migrations/*.ts`. **The active control today is the
explicit WHERE predicate** (Section F); these describe what RLS will (fail to) do post-flip.
The two GUC families (`app.user_id` owner vs `app.current_tenant` courier) are consistent
between code and policies — no GUC-name mismatch.

- **HIGH — anonymous policies on `orders`/`order_items`/`customers` are unscoped.**
  `migrations/1780338981783_anonymous_orders.ts:5,9` and
  `1780338981782_customer-anonymous-update.ts:6,10` use `USING (app_current_user() IS NULL)`
  with no per-order/customer scoping (and the customer UPDATE has **no `WITH CHECK`**). The
  public storefront pool sets no `app.user_id` → `app_current_user()` is NULL → these
  permissive policies are `TRUE` for **every row in every tenant**, and (permissive =
  OR-combined) they nullify the sibling `tenant_isolation`. Post-flip, any query on the public
  pool lacking an explicit predicate reads all tenants' orders + customer PII / updates any
  customer. **RLS provides zero isolation in the most-exercised context.**
- **HIGH — `couriers` has NO RLS at all.** `1780421029538_couriers.ts:5-18` never
  `ENABLE`s RLS (only sibling `courier_locations` gets it). Table holds `password_hash`,
  `email_encrypted/hash`, `phone_*`, `full_name_encrypted`. On the shared role any tenant
  context reads every courier's credential hashes/PII and can `UPDATE`/deactivate competitors'
  couriers. No flip fixes it — RLS is simply off.
- **HIGH — `courier_sessions` has NO RLS at all.** `1780421032856_courier-sessions.ts:5-21`.
  Holds `token_hash`, `family_id`, `active_location_id`. Cross-tenant read of session token
  hashes / forge/extend sessions.
- **MED — anonymous `INSERT WITH CHECK (app_current_user() IS NULL)` lacks `location_id`
  scoping** (`1780315000000_customer-rls.ts:6-7,16-17`): an anonymous checkout can insert an
  order/customer into **any** tenant (only app-layer slug validation prevents it).
- **MED — `public_select ON locations USING (true)`** (`1780338909301_public-locations-rls.ts:7`)
  exposes every tenant's `phone`/`custom_domain`/`lat,lng`/`status` on any connection.
- **MED — `customer_contact_reveals` NO RLS** (`1780421100062_fallback-config.ts:15`) — a
  privacy audit table, `location_id`-scoped but unprotected.
- **MED — secret/PII ops tables `FOR ALL USING(true) WITH CHECK(true)`**
  (`access_requests` `…041:49`, `provision_grants` `…069:55`, `claim_invites` `…071:42`,
  `acquisition_sources` `…068:63`): protection is grant-only; since **all requests share one
  role**, an INSERT-capable context can self-mint a `provision_grants`/`claim_invites` row
  (it knows the plaintext token) and drive self-provision/claim.
- **MED/LOW — `backup_metadata` + `backup_audit_log` are `ENABLE`d but never `FORCE`d**
  (`…048:21-22`, `…049:15-16`) — the only two such tables; the table-owner role bypasses,
  and `…050`'s `FOR SELECT TO authenticated USING(true)` lets any authenticated principal
  read all backup metadata/audit.
- **LOW — `telegram_login_tokens` NO RLS + no anon/authenticated REVOKE**
  (`…031_telegram-owner-login.ts:16`) — mints an owner JWT; inconsistent with sibling token tables.
- **LOW — `upload_audit` NO RLS** (`…063_hardening-seam.ts:9`).
- **LOW — `anonymous_select ON idempotency_keys USING (key IS NOT NULL)` ≈ `USING(true)`**
  (`1780315000000_customer-rls.ts:27-28`) — cross-tenant metadata read.
- **LOW/INFO — `theme_versions_owner_write` still on the Supabase GUC**
  (`1780338982030_theme_versions.ts:33-42`, `current_setting('request.jwt.claim.sub')`,
  `TO authenticated`, no `WITH CHECK`) — the phase-1 re-key missed it; inert for `dowiz_app`,
  fails **closed** post-flip (availability, not leak).
- **INFO — `app_member_location_ids()` admits ANY membership role**
  (`1780310071220_core-identity.ts:76-80`) — owner-named write policies don't filter
  `role='owner'`; a within-tenant (not cross-tenant) privilege concern.

**Verified non-findings (don't re-flag):** base `tenant_isolation` `FOR ALL USING(…)` with no
`WITH CHECK` is safe (Postgres reuses `USING` as the check); `analytics_*`/`exchange_rates`
RLS-off is documented design; public menu `products`/`categories` `USING(true)` is intended
public content.

---

## F. LIVE cross-tenant / broken-access-control findings (ranked)

Verified against source (both the raw-query sweep of all ~1,155 `.query(` sites and the
route-guard sweep of all 66 route modules converged on the courier-role and logout items;
the author re-read F1–F7 directly). **These are exploitable today** — the active tenant
boundary is the explicit SQL predicate + preHandler, and these paths are missing one.

### F1. CRITICAL — Cross-tenant order-status write (`PATCH /api/orders/:id/status`)
- `apps/api/src/routes/orders.ts:840-905` — guard `[verifyAuth, requireRole(['owner'])]`
  (`:841`); the read is `SELECT id,status,location_id,type FROM orders WHERE id = $1`
  (`:862-864`) with **no membership JOIN**; `locationId` is then taken from the order's own
  row (`:871`); `withTenant` (`:860`) is inert under BYPASSRLS.

**Invariant violated:** a mutation must bind the target row to the caller's tenant.
**Exploit:** owner of tenant A, with a normal owner JWT →
`PATCH /api/orders/{tenantB_order_uuid}/status {"status":"CANCELLED"}`. `assertOwnerTargetAllowed`
only restricts *which* transitions (PENDING→CANCELLED and IN_DELIVERY→CANCELLED are allowed),
never *whose* order → cancels/mutates another tenant's live orders. Competitor sabotage; red-line
(order lifecycle). The **GET** sibling (`:747`) was already hardened with a `JOIN memberships`
(and even comments that a bare `WHERE id=$1` leaks cross-tenant under BYPASSRLS) — the PATCH was
left behind.
**Fix direction:** read via `FROM orders o JOIN memberships m ON m.location_id=o.location_id
WHERE o.id=$1 AND m.user_id=$2 AND m.role='owner' AND m.status='active'`; 404 before any transition.

### F2. HIGH — Cross-tenant, irreversible customer-PII erasure via unvalidated GDPR `customerId`
- Entry `apps/api/src/routes/owner/gdpr.ts:48` — `let resolvedCustomerId = customerId || null;`
  takes the body `customerId` **verbatim**; only the `phone` branch (`:50-53`) is location-scoped.
  The INSERT (`:81-86`) writes `gdpr_erasure_requests(location_id=<my loc>, customer_id=<attacker>)`.
- Sink `apps/api/src/lib/anonymizer/index.ts:118-141` — `SELECT … FROM customers WHERE id=$1
  FOR UPDATE` then `UPDATE customers SET phone='anon_'||…, name=NULL, marketing_opt_in=false,
  anonymized_at=now() WHERE id=$1` — **no `location_id` predicate** (`locationId` at `:131` feeds
  only the audit row/bus). Worker `workers/anonymizer-gdpr.ts` drains the row.

**Invariant violated:** an FK supplied by the caller (`customer_id`) must be proven same-tenant;
`withTenant`/RLS never checks that a child FK belongs to the tenant.
**Exploit:** owner A `POST /api/owner/locations/{A}/gdpr-requests {"customerId":"<tenant-B customer
UUID>"}` → passes `requireLocationAccess` (owns A) → worker **irreversibly** anonymizes tenant B's
customer (name→NULL, phone→random) + writes a bogus erasure/audit trail. HIGH (impact is
CRITICAL-class irreversible cross-tenant PII destruction; rated HIGH only because the target must
be a known customer v4 UUID, not app-enumerable). `anonymizeOrder` (`index.ts:195/210`) has the same
unscoped shape (not currently client-reachable — fix for symmetry).
**Fix direction:** validate `SELECT 1 FROM customers WHERE id=$customerId AND location_id=$locationId`
(404 otherwise) when `customerId` is supplied directly; add `AND location_id=$2` to the anonymizer.

### F3. HIGH — `owner/couriers.ts` has no `requireRole(['owner'])` → a *customer* reads decrypted courier PII; a *courier* runs privileged mutations
- `apps/api/src/routes/owner/couriers.ts:14-15` — hooks are only `verifyAuth` +
  `requireLocationAccess`; `requireRole` is **not imported** (`:6`). Every other owner file adds it.

**Why exploitable:** `requireLocationAccess` (`plugins/auth.ts:127-140`) admits a **customer**
whose JWT `locationId` matches and a **courier** whose `activeLocationId` matches — not just owners.
**Read exploit (most practical):** a customer of location L → `GET /api/owner/locations/L/couriers`
→ full courier roster with **decrypted `full_name`** + masked phone/email + stats (`:28-62`). No id
guessing — the customer already holds `locationId=L`.
**Mutate exploit:** a courier of L → `PATCH /api/owner/locations/L/couriers/:courierId` →
deactivate/suspend co-workers + revoke their sessions (`:99-116`); the audit row is mis-attributed
`actor_kind='owner'` (`:107`). Also exposes `.../couriers/:courierId/details` (unmasked customer
name/phone/address for the last 20 deliveries), `.../couriers/live` (live GPS+phone),
`.../orders/:orderId/route` (GPS trail). Within-tenant cross-**role** BAC, with a cross-tenant *side
effect*: `UPDATE couriers SET status … WHERE id=$3` and the session-revoke hit the **global** courier
account, so deactivating a courier shared across A+B disables them for B too.
**Fix direction:** `import { requireRole }` and add
`fastify.addHook('preValidation', requireRole(['owner']));` between the two existing hooks; and scope
the `couriers`/`courier_sessions` mutations to the `courier_locations` relationship, not the global row.

### F4. HIGH — `owner/courier-invites.ts` has no `requireRole(['owner'])` → non-owner mints/revokes courier invites
- `apps/api/src/routes/owner/courier-invites.ts:20-21` — `verifyAuth` + `requireLocationAccess`,
  no `requireRole` (`:6-7`).

**Exploit:** a courier (or customer) of `:locationId` → `POST .../courier-invites` with an
attacker-chosen `role` (`:27-30`, no allow-list) → mints a working courier invite + code (account
injection); `DELETE .../:inviteId` revokes pending invites (onboarding DoS). Chains with F7 into ATO.
**Fix direction:** add `fastify.addHook('preValidation', requireRole(['owner']));` (import it first)
and allow-list `role`.

### F5. MED — `owner/signals.ts /compute` leaks arbitrary customer reputation cross-tenant
- Entry `apps/api/src/routes/owner/signals.ts:105-123` — `GET /:locationId/signals/compute?customer_id=<uuid>`
  passes the client `customer_id` straight into `computeSignals(db, …)` (raw `db`).
- Sink `apps/api/src/lib/signals/compute.ts:85-88` — `SELECT no_show_count, completed_count,
  last_no_show_at FROM customers WHERE id = $1` — **no `location_id` predicate**.

**Exploit:** owner A `GET /api/owner/{A}/signals/compute?customer_id=<tenant-B customer uuid>` →
returns B's behavioural counters + row existence. `requireLocationAccess` never touches `customer_id`.
**Fix direction:** add `AND location_id = $2` to the `compute.ts` lookup and thread `locationId` in.

### F6. MED — Telegram webhook fails OPEN when `TELEGRAM_BOT_SECRET` is unset
- `apps/api/src/server.ts:531` (`env.TELEGRAM_BOT_SECRET || ''`) + `routes/telegram-webhook.ts:36`
  (registers at `/webhook/telegram/${secret}` → `/webhook/telegram/` when empty) + `:49-61` (the
  `if (secretToken && secretToken !== …)` reject is skipped for a missing header; empty-secret path
  skips the check entirely). Config makes the secret `optional()`.

**Exploit (anonymous, only if the secret is unset):** inner authority is bound to the sender's
**numeric Telegram id** (`:239-248`), which is enumerable, not secret → forge a `callback_query`
with `from.id` = a linked owner's Telegram id → confirm/reject that tenant's orders, open shifts,
pause storefront. Also: the header compare at `:50` is a non-constant-time `!==` (F11).
**Fix direction:** require the secret at boot; 404/refuse to register when empty; `timingSafeEqual`.

### F7. MED — Courier invite-redeem overwrites an existing courier's password (ATO primitive)
- `apps/api/src/routes/courier/auth.ts:89-94` — `INSERT INTO couriers (…) VALUES (…)
  ON CONFLICT (email_hash) DO UPDATE SET password_hash = EXCLUDED.password_hash`. The redeem never
  compares the caller-supplied `email` (`:47`) to the invite's intended recipient
  (`invited_email_hash`); the only proof required is the argon2 invite `code` (`:68`).

**Exploit:** a holder of any valid, unused invite+code redeems it with a **victim courier's email**
→ resets that courier's `password_hash` to the attacker's value and grants membership; the response
returns a live JWT for the account. Combined with F4 (a courier can now mint invites) this is a full
account-takeover chain. MED (gated on a valid invite+code).
**Fix direction:** reject when `sha256(email) != invite.invited_email_hash`; never silently reset an
existing account's password on `ON CONFLICT`.

---

## G. Additional lower-severity / latent hardening notes

- **F8 · LOW — Courier `/logout` revokes any session with no token proof.**
  `apps/api/src/routes/courier/auth.ts:479-507` — public route; takes
  `sessionId = refresh_token.split('.')[0]` and runs `UPDATE courier_sessions SET revoked_at=now()
  WHERE id=$1` with **no** `argon2.verify` of the token secret (contrast `/refresh` at `:412`).
  Anyone who learns a courier's session UUID (it is the JWT `jti` / first refresh-token segment)
  force-logs-out that courier → targeted session DoS. *Fix:* `argon2.verify(token_hash, parts[1])`
  before revoking. (Both sweeps flagged this.)
- **F9 · LOW — `customer/push.ts` keys on `user.userId`, which is undefined for customer tokens.**
  `routes/customer/push.ts` (~`:35-53,72-74`) — customer claim has only `sub`; `pg` coerces to
  `NULL` → subscribe inserts orphaned `customer_id=NULL` rows and **unsubscribe never matches**
  (customer can't opt out — GDPR-adjacent). Fails **closed** (no cross-user read). *Fix:* use `user.sub`.
- **F10 · LOW — `owner/menu-translate.ts:10` `requireLocationAccess` bound to the wrong param.**
  Route param is `:id` but the guard reads `params.locationId` → always 400 (fails **closed**). The
  in-handler `SELECT … FROM locations WHERE id=$1` has no membership binding, so renaming `:id`→
  `:locationId` **without** also adding a predicate would create a direct cross-tenant IDOR. *Fix:*
  rename the param **and** keep a real ownership predicate.
- **F11 · LOW — `owner/products.ts` translation/modifier subroutes scope tenant only via RLS.**
  `routes/owner/products.ts:187,228,245,264,296` operate on a bare `:id` with no
  `WHERE location_id=:locationId` tie; closed **today only** by FORCE-RLS on the child tables →
  becomes cross-tenant IDOR the instant RLS is relaxed or a child table ships without FORCE. *Fix:*
  add `AND EXISTS (SELECT 1 FROM products WHERE id=$id AND location_id=$locationId)`.
- **F12 · LOW — `menu-import` "anonymous" import reachable by any bearer token.**
  `routes/owner/menu-import.ts:196` — no `verifyAuth` (intended pre-account) but sits under
  `/api/owner/`, so the presence gate blocks true-anon callers yet lets any courier/customer token
  trigger the expensive AI-OCR path (Redis-only 5/min). Resource abuse, not a data breach.
- **F13 · LOW — `owner/settlements.ts:301` `/settlements/regenerate` recomputes ALL locations.**
  Correctly owner+location gated, but `worker.handleGenerate()` recomputes every tenant (comment at
  `:314`) → an owner of one tenant triggers a global recompute. Tenant-isolation smell.
- **C2 · LOW** and **D1 · LOW** — see Sections C and D (in-memory per-instance limiter; unthrottled
  WS message DB-amplification).
- **INFO — `otp.ts:112-212`** verified OTP token is not bound to the stored `order_intent_hash`
  (latent; OTP globally disabled + attempts capped). **INFO — `order-messages.ts`** role dispatch has
  no final `else` (safe only because the `AuthToken` union is exhaustive — brittle default-allow if a
  4th role is added).

### Surfaces confirmed CLEAN (ranked sweeps, no exploitable IDOR/authz gap)
Admin/platform plane (`lib/platform-admin.ts` → real `verifyAuth`→`requirePlatformAdmin`, server-side
allow-list, one register); acquisition/provisioning (`/internal`, timing-safe `PROVISION_OPS_SECRET`,
fail-closed 404, mints no owner JWT); money webhook (`payments-webhook.ts`, HMAC `timingSafeEqual`
before any ledger write); token minting (`auth.ts`, `auth/local.ts` — role always server-derived);
claim flow (recipient-email match, single-use opaque token); spa-proxy inline owner routes
(`getLocationId`/`getOwnerContext` verify owner + live membership); the whole
`/api/owner/locations/:locationId/*` group (categories/products/modifier-groups/menu-availability/
themes/notifications/dashboard/refunds/promotions/product-media/gdpr-list/reveal-contact/…) —
`verifyAuth`+`requireRole(['owner'])`+`requireLocationAccess` with location-scoped SQL; courier data
routes (me/settlements/assignments/shifts — every query bound to `courier_id=user.sub` /
`location_id=activeLocationId`); customer/orders (`WHERE o.id=$1 AND o.customer_id=$2`),
customer/track+otp (secret `token_hash` / location+phone); all public menu/SSR/theme/funnel/telemetry
routes (by-design public storefront data).
