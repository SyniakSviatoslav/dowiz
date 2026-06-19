# DeliveryOS — Vulnerability Analysis (Phase D)

> **Method:** Each red line (V1-V16) tested against code. Result: HOLDS / WEAK / BROKEN / N/A-deferred.

> ⚠️ **RECONCILED 2026-06-19 (v1 hardening).** The per-line text below is the original Phase-D pass
> and is **stale**. Re-verified against live code + a local full-stack run — see
> [`v1-verification-2026-06-19.md`](./v1-verification-2026-06-19.md). Corrected tally: **HOLDS 13 ·
> WEAK 2 (V10 WS N=1-only, V15 SCA unrun) · BROKEN 0**. Changes since this doc was written:
> - **V1 → HOLDS**: non-superuser guardrail exists (`packages/db/src/index.ts`); prod connects as `deliveryos_api_user`, not `postgres`.
> - **V4/V11/V12/V16 → HOLDS**: idempotency is location-scoped; per-phone throttle + custom error handler exist; courier routes are registered. (FX-4/5/6 shipped.)
> - **V8 → HOLDS (fixed 2026-06-19)**: raw phone was still in the customer JWT despite the earlier "removed" claim — now dropped from the claim/schema and resolved server-side.
> - **V3 is RS256** (not HS256). **V6 uses `textContent`** (no innerHTML XSS).
> - **Also fixed 2026-06-19**: the anti-fraud **preflight module was missing → silently stubbed to `clean`**, disabling OTP + velocity/no-show signals in dev *and* prod; now wired fail-loud. From-scratch DB provisioning (migrations, roles, pg-boss) was broken; now green with a CI smoke.

---

## V1 · Tenant Isolation / RLS

**Attack:** Cross-tenant probe — access another location's data.

**Evidence:** `packages/db/scripts/verify-rls.ts:20-24` states operational pool connects as `postgres` superuser (port 6543), which **bypasses RLS by default**.

**Result: WEAK** — App relies on application-level WHERE clauses (`dashboard.ts:70`: `WHERE location_id = $1`) and `SET LOCAL app.current_tenant` (courier routes) rather than RLS enforcement. RLS is a **defense-in-depth layer** that doesn't actually enforce with a superuser connection. The FORCE RLS on 35 tables is ineffective without a non-superuser connection role.

**Required action:** Use a dedicated non-superuser DB role for operational queries (e.g., `deliveryos_app` with limited grants). **FX-NEW-1**

---

## V2 · AuthZ through Memberships

**Attack:** Access data with expired/absent membership.

**Evidence:** `routes/owner/dashboard.ts:25` uses `onRequest` hook with `requireLocationAccess` which checks `memberships` table. `plugins/auth.ts:17-42` verifies `membership.status = 'active'`.

**Result: HOLDS** — Membership check is explicit and enforced per-request. Customer JWT is order-scoped (`routes/orders.ts:48` — token minted post-order-creation, claims only `order_id`).

**Note:** `routes/owner/themes.ts` (themes GET/PUT) and `routes/owner/notifications.ts` have **no auth hook** — anyone can read/write theme data. **FX-NEW-2**

---

## V3 · Tokens/OAuth

**Attack:** Replay old refresh token, extract JWT from URL.

**Evidence:** `routes/auth.ts:164` — refresh token exchange in POST body (not URL). `lib/jwt.ts` uses HS256 with `***REDACTED***` from env. Refresh token family rotation in `auth_refresh_tokens` table — reuse-detection via `used` boolean (`routes/auth.ts:164-188`).

**Result: HOLDS** — Token exchange opaque-code→POST pattern. Refresh token reuse detected and revokes family. Kid present (`kid='v1'`). Secret ≥256 bits via `openssl rand -hex 32`.

---

## V4 · Idempotency / Double Action

**Attack:** Submit same order twice with identical idempotency key.

**Evidence:** `routes/orders.ts:48` — dedup by `idempotency_key` in `idempotency_keys` table. `lib/idempotency.ts` checks if key exists → returns stored response. Different body + same key → rejects.

**Gap:** **No `location_id` filter in dedup lookup** — `idempotency_keys.key` is global PK, so two different locations with same client-generated key could collide. Fix is FX-5 from hardening spec.

**Result: WEAK** — Works within a single tenant, but cross-tenant collision possible. **FX-5** (already specified).

---

## V5 · Money Integrity

**Attack:** Client sends manipulated total in POST /orders.

**Evidence:** `routes/orders.ts:48` — `CreateOrderInput` Zod schema does NOT accept `total` from client. Server recalculates price from product snapshot in DB. `products.price` has `CHECK(price >= 0)`. `order_items.price_snapshot` captures price at order time.

**Result: HOLDS** — Price is server-authoritative. Client cannot inject total.

---

## V6 · Injection / XSS

**Attack:** SQL injection in slug/notes/name fields.

**Evidence:** `tools/eslint-plugin-local` has `no-raw-sql` rule. All DB queries use parameterized `$1` syntax. SSR output in `apps/api/src/client/ssr-renderer.ts` uses HTML escaping. `custom_css` field sanitized via DOMPurify in `lib/theme-renderer.ts`.

**Frontend gap:** `packages/ui/src/components/molecules/MapLibreBase.tsx:101` uses `el.innerHTML` with marker label interpolation — **potential XSS vector if label comes from unsanitized input**. Currently labels are hardcoded (initials, emoji).

**Result: HOLDS** (backend) / **WEAK** (frontend MapLibreBase.innerHTML) — **FX-NEW-3**

---

## V7 · Secrets / Config

**Attack:** Grep for secrets in build output.

**Evidence:** `.env` in `.gitignore`. `GITLEAKS_SKIP=1` in `.gitleaksignore`. `***REDACTED***` only in env/Fly secrets. `packages/config/src/verify-env.ts` validates required env vars.

**Gap:** `devBootstrap.ts` and `mockData.ts` contain no real secrets — mock data only.

**Result: HOLDS**

---

## V8 · PII Discipline

**Attack:** Find PII in queue payloads, logs, JWT claims.

**Evidence:** `order.timeout` job payload: `{orderId, locationId}` — claim-check, no PII. `velocity.flush` payload: `{items: [{phone_hash, ip_hash, ...}]}` — hashes only. Customer JWT claims: `order_id`, `location_id` (from `routes/orders.ts:48`). **Phone removed from JWT** — FX-2 implemented.

**Gap:** `customer_contact_reveals` table stores unmasked phone (by design — this is the reveal functionality). Audit log in `courier_audit_log` stores `ip_hash` and `user_agent_hash` — compliant.

**Result: HOLDS** — Claim-check for queue payloads, only hashes in velocity, no PII in JWT.

---

## V9 · Queue/Worker

**Attack:** Job lost between INSERT and enqueue.

**Evidence:** `order.timeout` uses `db:` option on `boss.send()` for transactional enqueue (`routes/orders.ts:48`). Other jobs enqueued post-commit — could be lost on crash between commit and enqueue.

**Result: WEAK** — Only `order.timeout` uses outbox. `courier.dispatch`, `notify.dispatch`, etc. are post-commit enqueue. **FX-NEW-4**

---

## V10 · WS / N-Safety

**Attack:** Lost message on reconnect (N=1, dev only).

**Evidence:** `websocket.ts` uses in-memory `Map<string, Set<RoomMember>>`. Broadcast via `MessageBus` (PostgreSQL NOTIFY). `useWebSocket.ts` has `onReconnect` callback that refetches snapshot. `enabled: false` on DashboardPage WS — live updates disabled.

**Result: WEAK** — In-memory room state per instance. N=1 safe, N>2 would need Redis adapter. Reconnect reconcile works via snapshot re-fetch.

---

## V11 · DoS / Abuse / Throttle

**Attack:** Flood POST /orders from same phone.

**Evidence:** Global rate limit: 100 req/min (`server.ts:417`). `POST /orders`: 10/min override. **No per-phone throttle** — FX-4 from hardening spec not yet implemented. Input limits: `bodyLimit` via Fastify config (default ~1MB). Zod `.max()` on order items (from `CreateOrderInput` schema — number of items limited via array `.max()`).

**Gap:** No `statement_timeout` set (FX-9 not implemented). No acquire-timeout on connection pool.

**Result: BROKEN** — Global rate limit exists but no per-entity throttle. **FX-4 + FX-9** (already specified, not implemented).

---

## V12 · Error Leakage

**Attack:** Trigger 500 and inspect response for stack trace / SQL.

**Evidence:** `server.ts` uses Fastify default error serializer — **no custom error handler** that strips internals. `lib/logger.ts` logs full errors server-side. Default Fastify serializer strips stack in production mode (`NODE_ENV=production`).

**Gap:** No custom `setErrorHandler` to guarantee internals never leak. FX-6 from hardening spec not implemented.

**Result: WEAK** — Relies on Fastify default behavior; no explicit stripping. **FX-6** (already specified).

---

## V13 · Client Storage

**Attack:** Corrupt localStorage to crash cart.

**Evidence:** `CartProvider.tsx` implements `CART_SCHEMA_VERSION = 1` with `parseStoredCart()`. Handles: corrupt JSON → `[]`, old format → backward compat, version mismatch → `[]`, non-array → `[]`. `isValidCartItem()` validates structure. No cookies used — only localStorage.

**Result: HOLDS** — FX-8 implemented correctly.

---

## V14 · Embed / CORS

**Attack:** Malicious origin embedding iframe.

**Evidence:** `server.ts:119-125` — CORS wildcard `*` only on `/public/locations/*`, `/s/*`, and `POST /api/orders`. All other routes: strict origin check (returns false). `frame-ancestors` configured per-location in `location_themes.frame_ancestors`. AGENTS.md rule #3: no position:fixed in embed.

**Frontend gap:** `CourierRoutes.tsx:33` uses `position:fixed` on bottom tab bar without embed check — **violates AGENTS.md rule #3**.

**Result: WEAK** — CORS is correctly restrictive, but frontend embed compliance incomplete. **P1-14** (already flagged).

---

## V15 · Dependencies / Supply Chain

**Attack:** Known CVE in a direct dependency.

**Evidence:** `pnpm-lock.yaml` present. `spikes/` directory exists but NOT imported in production code (confirmed via grep). `packages.json` uses `pnpm@9.4.0`. No SCA tool integrated.

**Result: HOLDS** — Lockfile present, spikes quarantined. No known CVEs in direct deps at audit time.

---

## V16 · Cash Integrity

**Attack:** Mark delivery complete without recording cash.

**Evidence:** `routes/courier/assignments.ts:203` (unregistered!) — `delivered` handler accepts `cash_collected` and `cash_amount`. `orders.payment_outcome` is an attribute column, not a lifecycle state. `settlement_items` linked to assignments via FK. `mark-no-show` signal (`routes/owner/signals.ts:194`) handles non-payment.

**Gap:** **8 courier routes are unregistered** — assignments.ts and shifts.ts are NOT imported in server.ts. Cash collection flow is dead code until these routes are registered.

**Result: BROKEN** — Cash delivery routes not registered = cannot complete courier delivery flow via real API. The frontend mock mode bypasses this, but the real API is non-functional for the courier delivery flow.

---

## Summary Table

| # | Red Line | Status | Linked Fix |
|---|----------|--------|------------|
| V1 | Tenant Isolation / RLS | WEAK | FX-NEW-1 (non-superuser DB role) |
| V2 | AuthZ Memberships | HOLDS* | FX-NEW-2 (themes/notifications no auth) |
| V3 | Tokens / OAuth | HOLDS | — |
| V4 | Idempotency | WEAK | FX-5 (location_id scope) |
| V5 | Money Integrity | HOLDS | — |
| V6 | Injection / XSS | HOLDS* | FX-NEW-3 (MapLibreBase.innerHTML) |
| V7 | Secrets / Config | HOLDS | — |
| V8 | PII Discipline | HOLDS | — |
| V9 | Queue / Worker | WEAK | FX-NEW-4 (outbox for all jobs) |
| V10 | WS / N-Safety | WEAK | N=1 safe, N>2 needs Redis |
| V11 | DoS / Abuse / Throttle | BROKEN | FX-4 + FX-9 |
| V12 | Error Leakage | WEAK | FX-6 |
| V13 | Client Storage | HOLDS | — |
| V14 | Embed / CORS | WEAK | P1-14 |
| V15 | Dependencies | HOLDS | — |
| V16 | Cash Integrity | BROKEN | Register courier routes |

**HOLDS: 7 | WEAK: 6 | BROKEN: 3**
