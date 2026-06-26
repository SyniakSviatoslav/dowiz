# DeliveryOS — Project Blueprint & Migration / Restructure / Simplification Guide

> Whole-system blueprint of `dowiz` / DeliveryOS, **grounded in the source code as it stands today** (verified by direct reads of the authoritative files, not inference). Purpose: enable a large migration / restructure / simplification with zero loss and zero failures.
>
> - **Captured:** 2026-06-25 · **Branch:** `feat/mvp-sensor-seams` (HEAD `d400f488`) · root package `deliveryos-monorepo`, `pnpm@9.4.0`, Node `>=22`
> - **Prod:** `https://dowiz.fly.dev` (Fly app `dowiz`, region `fra`) · **Staging:** `https://dowiz-staging.fly.dev` (`dowiz-staging`)
> - **Scale:** ~197k NLOC · 3 apps · 7 packages · **142 migration files** (head `1790000000067_bom-seams`) · ~83 app tables · 15 ADRs · 20 regression guardrails · 16 custom lint rules
>
> **Source-of-truth note:** structural facts, env schema, pools, state machine, routes, lint rules, table names, and deploy config below were read directly from source. The few items that can only be confirmed against the live database (exact column types, row counts) are marked _(verify against live DB)_. Before any destructive data step, treat a fresh `pg_dump --schema-only` of prod as canonical.

---

## Table of Contents
1. [Executive Summary](#1-executive-summary)
2. [Repository Topology](#2-repository-topology)
3. [Technology Stack](#3-technology-stack)
4. [Backend — apps/api](#4-backend--appsapi)
5. [Frontend — apps/web](#5-frontend--appsweb)
6. [Worker — apps/worker](#6-worker--appsworker)
7. [Shared Packages](#7-shared-packages)
8. [Data Layer](#8-data-layer)
9. [Auth & Identity](#9-auth--identity)
10. [Security Posture](#10-security-posture)
11. [Integrations](#11-integrations)
12. [Feature Flags](#12-feature-flags)
13. [Environment Variables (real schema)](#13-environment-variables)
14. [Build & Deployment](#14-build--deployment)
15. [CI/CD & Quality Gates](#15-cicd--quality-gates)
16. [Invariants / Red-Lines](#16-invariants--red-lines)
17. [ADRs](#17-adrs)
18. [Testing](#18-testing)
19. [Tech Debt, Hotspots & Clutter](#19-tech-debt-hotspots--clutter)
20. [Project Status](#20-project-status)
21. [Migration Playbook](#21-migration-playbook)
22. [Appendices](#22-appendices)

---

## 1. Executive Summary

**DeliveryOS** (`dowiz`) is a **multi-tenant SaaS for restaurant/store ordering + delivery**, Albania-first (locales `sq`/`en`/`uk`). An owner onboards (often menu-first via CSV/PDF upload), gets a branded storefront at `/s/:slug`, takes cash-on-delivery orders, dispatches couriers with live GPS tracking, and gets real-time alerts via Telegram + Web Push.

Three audiences on one backend: **Customer** (anonymous storefront), **Owner/Admin** (`/admin/*`), **Courier** (`/courier/*`).

**One-sentence architecture:** a Fastify API (single process that *also* runs ~20 background workers) + a separate pg-boss worker process, both bundled to single-file CJS by esbuild, serving a React 18 / Vite SPA from the same container, on Supabase Postgres with **forced RLS** tenant isolation, deployed on Fly.io, using Redis (Upstash) for pub/sub + ephemeral state and Cloudflare R2 for media/backups.

New verticals land **schema-first as inert flag-gated "seams"** (media, stock/BOM, sensor-bus) so prod stays dark while code ships. The repo is heavily hardened (20 regression guardrails, 15 ADRs, compliance/GDPR tooling, a self-improvement harness).

---

## 2. Repository Topology

pnpm workspace (`pnpm-workspace.yaml`): `apps/*`, `packages/*`, `tools/*`, `spikes/*`. **No Turbo** — `pnpm -r build` (topological). Node `>=22`, pnpm `9.4.0` via corepack.

```
apps/
  api/     Fastify backend (~44.7k LOC) — routes + ~20 inline workers + WS
  web/     React 18 + Vite SPA (~17.6k LOC) — admin/client/courier in one bundle
  worker/  pg-boss consumer process (thin: createSessionPool → registerHandlers → heartbeat)
packages/
  config/        @deliveryos/config — Zod EnvSchema + loadEnv() + boot guards
  core/          @deliveryos/core — preflight evaluator
  db/            @deliveryos/db — operational+session pools, 142 migrations, seed
  domain/        @deliveryos/domain — order state machine (assertTransition) + errors (pure)
  platform/      @deliveryos/platform — RedisMessageBus/PgMessageBus, PgBossQueueProvider, RoutingProvider, JWT (jose), TenantProvider
  shared-types/  @deliveryos/shared-types — contracts (public/customer/owner/courier), queue-names, allergens
  ui/            @deliveryos/ui — design system, i18n catalog, theme/palette/paperSkin, hooks
tools/
  eslint-plugin-local/   16 custom lint rules
  skillspector/          agent-skill security scanner
e2e/  docs/  compliance/  scripts/  loops/  openspec/  spikes/  .agents/ .claude/ .opencode/
Dockerfile  fly.toml  eslint.config.js  tsconfig.base.json  tsconfig.migrations.json
[root clutter — see §19]
```

**Package dependency graph** (bottom → top): `config`, `domain`, `shared-types` have no internal deps → `db`(config) → `platform`(config,db) → `core` → `ui`(shared-types) → `api`(all) / `worker`(config,db,platform,shared-types) / `web`(ui,shared-types). Clean and layered — these `@deliveryos/*` boundaries are the natural seams for any restructure.

---

## 3. Technology Stack

| Area | Tech (verified versions where read) |
|---|---|
| Runtime/lang | Node ≥22, TypeScript 5.6 (strict, ES2022, `moduleResolution: bundler`) |
| Monorepo | pnpm 9.4 workspaces, no Turbo |
| Backend | **Fastify** (cors, static, multipart, rate-limit plugins), port 8080 |
| Frontend | **React 18.3**, **Vite 6.0**, **react-router-dom 7.5**, **TailwindCSS 3.4**, **framer-motion 12.40**, **maplibre-gl 5.24** + **react-map-gl 8.1**, **three 0.184**, **zod 3.23** |
| DB | **PostgreSQL 16** (Supabase), node-pg-migrate (TS), `pg` driver |
| Queue | **pg-boss** (queue-in-Postgres, ADR-0001), isolated `pgboss` schema |
| Pub/sub + ephemeral | **Redis** (ioredis, Upstash) — `RedisMessageBus`; worker uses `PgMessageBus` (NOTIFY) |
| Auth | **jose** RS256 JWT, **argon2** |
| Storage | **Cloudflare R2** via `@aws-sdk/client-s3` + `lib-storage`; local FS fallback |
| Images | **sharp** |
| Maps/tiles | MapLibre + OpenFreeMap (`VITE_TILE_PROVIDER` free/self) |
| Routing/ETA | OpenRouteService / self / haversine (`RoutingProvider`, default `ors`, haversine degrade) |
| Notifications | **Telegram Bot API**, **Web Push (VAPID)**. WhatsApp/Baileys **removed** (migrations 020→043) |
| LLM (menu OCR) | OpenCode Zen → OpenRouter → Groq → OpenAI → zero-dep heuristic |
| Email | Resend (operator lead alerts only) |
| Observability | Sentry (PII-scrubbed), Pino; OTel/Langfuse deps present |
| Build | **esbuild** single-file CJS (`scripts/build-apps.ts`) |
| Host | **Fly.io** Docker, 2 processes (web 512MB / worker 256MB), region `fra` |
| CI | GitHub Actions (ci, visual, skill-security) + Husky pre-commit |
| Lint | ESLint 9 flat + `eslint-plugin-local` (16 rules) + Prettier + jsx-a11y |
| E2E | Playwright 1.60 (pinned container for visual) |

---

## 4. Backend — apps/api

Entry `apps/api/src/server.ts` (99th-pct churn). Builds Fastify, listens `{ port, host: '0.0.0.0' }` (8080).

### 4.1 Plugins (registration order in `server.ts`)
cors → static (serves the SPA from `dist/public`) → multipart → rate-limit → **authPlugin** → **securityHeadersPlugin** → healthRoutes → all feature routes. Auth decorates `verifyAuth` + `requireRole` (`plugins/auth.ts`).

### 4.2 Health
- `/livez` — cheap liveness (Fly healthcheck). `/health` — readiness (~11 DB queries + external calls), never the Fly probe (would drop live WS on restart).

### 4.3 Routes (59 route files under `routes/`, real list)
- **public/** `ssr.ts`, `menu.ts`, `theme.ts`, `branding-preview.ts`, `client-flow.ts`, `pwa.ts`, `seo.ts`, `telemetry.ts`, `funnel.ts`, `vapid.ts`, `rates.ts`, `fallback-config.ts`, `access-requests.ts` (registered only when `ACCESS_GATE_PUBLIC_ENABLED`)
- **auth** `auth.ts` (Google OAuth, Telegram login, refresh, logout), `auth/local.ts` (argon2 + dev-bypass)
- **owner/** (28 files) activation, alerts, categories, courier-invites, couriers, dashboard, dwell-settings, fallback, gdpr, locations, menu-availability, menu-import, menu-translate, modifier-groups, notifications, onboarding, order-meta, product-media, products, promotions, push, reveal-contact, settlements, signals, themes
- **courier/** assignments, auth, me, settlements, shifts; + top-level `couriers.ts`
- **customer/** orders, otp, push, track
- **admin/** backups, fallback, notification-audit
- **dev/** `mock-auth.ts` (gated by dev-guard)
- top-level: `orders.ts` (**red-line hotspot, 999 lines**: create txn, fee ladder, BigInt tax, idempotency, velocity gate phone+IP, sensor capture), `order-messages.ts`, `spa-proxy.ts` (**hotspot, SSR↔hydration**), `telegram-webhook.ts`, `health.ts`

### 4.4 Realtime
Fastify decorated `wss` (WebSocket server); room-based subscribe (`location:{id}:orders`, `order:{id}`, `delivery:{id}`). **Claim-check**: bus/WS payloads carry IDs only; consumers RLS-read full context (keeps PII out of Redis). Graceful shutdown closes all `wss.clients` before exit.

### 4.5 Order state machine (`@deliveryos/domain/order-machine.ts` — exact)
States: `PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP`. Transitions: `PENDING→{CONFIRMED,REJECTED,CANCELLED}`, `CONFIRMED→{PREPARING,IN_DELIVERY}`, `PREPARING→READY`, `READY→{IN_DELIVERY,PICKED_UP}`, `IN_DELIVERY→DELIVERED`. **Terminal:** `DELIVERED, PICKED_UP, REJECTED, CANCELLED`. **`SCHEDULED` is scaffold-disabled** (`ScaffoldDisabledError`); `assertTransition` throws on same-status/scaffold/illegal.

### 4.6 Workers — run in BOTH processes (the duplication to clean up)
`server.ts` starts ~20 workers **inline in the API process**: notify dispatch/customer-status/telegram-send (via `queue.work`), courierDispatch, courierCron, courierEvents, settlementCron, backupCron, backupVerify, dwellMonitor, orderTimeoutSweep, lifecycleHandlers, anonymizerRetention, gdprErasure, signalRaiser, velocityFlush, ratesRefresh, accessRequestNotify, accessRequestRetention, livenessChecker, freeTierWatch (scheduled `0 * * * *`). The **separate `apps/worker` process** (`createSessionPool` → `registerHandlers` → `Heartbeat`) registers handlers too. Telegram poller is disabled (webhook active).

### 4.7 Money
Integer minor units; BigInt arithmetic; tax half-up; **delivery fee server-authoritative** + client mirror `estimateOrderTotal()` + CI parity guardrail (ADR-0005, regression #17); `CASH_AMOUNT_TOO_LOW` 422 backstop.

---

## 5. Frontend — apps/web

React 18.3 + Vite 6 SPA, served by the API from `dist/public`. `vite.config.ts`: dev port 5173, proxies `/api`,`/public`,`/auth`,`^/s/` → `VITE_PROXY_TARGET || http://localhost:3000`; chunks `map` (maplibre ~1MB isolated) + `vendor` (react+react-dom+framer-motion kept together to avoid circular-init). Aliases `@ui`/`@deliveryos/ui`, `@shared-types`/`@deliveryos/shared-types`.

**Routing (`main.tsx`):** `/`→`/start` · `/start` (menu-first onboarding) · `/login` · `/privacy` · `/auth/callback` · `/s/:slug/*` (storefront, SSR-proxy) · `/branding-preview/:slug/*` · `/admin/*` · `/courier/*` · `/courier-invite/:inviteId` · `*` 404. Lazy bundles, Framer Motion transitions, providers (I18n/Currency/Theme/Tour/ErrorBoundary).

**Pages:** admin (Dashboard live orders+WS, **MenuManagerPage** hotspot, Branding, Couriers, CRM, Settings, Analytics lazy, Activation publish-gate, SupplyLibrary, Promotions, Login); client (**MenuPage** hotspot, Checkout w/ map+E.164+OTP+push, OrderStatus live tracking); courier (Tasks, Delivery GPS 12s + swipe, Earnings, History, Shift); **MenuFirstOnboarding** hotspot.

**Data layer:** `lib/apiClient.ts` (base `VITE_API_BASE_URL||/api`, Web-Locks-serialized token refresh, Zod, `X-Idempotency-Key`, 10s timeout, `safeStorage`); `lib/useWebSocket.ts` (single shared client, `VITE_WS_BASE_URL`||`/ws`, backoff+jitter, resume on online/focus); `lib/CartProvider.tsx`+`cartReconcile.ts` (versioned localStorage, reconcile-to-menu).

**Design/i18n:** `@deliveryos/ui` tokens (`--brand-*`), `derivePalette()`, paper/Moebius skin (`data-skin="paper"`). **i18n SSOT** `packages/ui/src/lib/i18n-catalog.ts` (key-major, all of sq/en/uk; pre-commit parity gate; add via `scripts/i18n-add.ts`). `messenger.ts` builds telegram/**whatsapp** (`wa.me`)/viber **customer click-to-chat deep-links** from E.164 (unrelated to the removed WhatsApp notification channel). Maps via `lib/tileConfig.ts` (`VITE_TILE_STYLE_URL`/`VITE_TILE_PROVIDER`).

**PWA:** `apps/api/public/sw.js` (cache-first statics, skip `/api`,`/ws`), `manifest.json`, Web Push from Checkout.

---

## 6. Worker — apps/worker

Thin entry: `createSessionPool()` → `PgBossQueueProvider.start()` → `registerHandlers(queue,pool,messageBus)` → `Heartbeat(pool).start()` → `setupShutdown`. Uses `PgMessageBus` (publish-only via NOTIFY). Fly `worker` process, 256MB. **Note the overlap with §4.6** — the same job handlers also run inline in the API process; a restructure should make one shared job registry the single source of truth.

---

## 7. Shared Packages

| Package | Role / key exports | Migration relevance |
|---|---|---|
| `config` | Zod `EnvSchema` (real list §13) + `loadEnv()` + `assertDevAuthDisabledInProd()` boot guard | the env contract; any new runtime must satisfy or replace it |
| `domain` | `ORDER_STATUSES`, `TRANSITIONS`, `assertTransition`, `isTerminal`, errors — **pure, no deps** | safest module; move money math here (ADR-0005 isomorphic mirror) |
| `db` | `createOperationalPool()` / `createSessionPool()`, migrations, seed | RLS enforcement + pool guard live here (red-line) |
| `platform` | `RedisMessageBus`/`PgMessageBus`, `PgBossQueueProvider`, `RoutingProvider`, JWT (`verifyAuthToken`), `TenantProvider` | provider plug-points for swapping infra |
| `shared-types` | contracts (`public/customer/owner/courier`), `queue-names`, `allergens`, legacy order types | the API contract surface (FE + BE import) |
| `ui` | atoms/molecules/admin/client/courier, theme/palette/paperSkin, i18n catalog+provider, hooks (geolocation, delivery-eta, courier-marker, breakpoint, haptics) | consumed only by `apps/web` |
| `core` | `evaluatePreflight` (order readiness) | small |

---

## 8. Data Layer

### 8.1 Migrations
- **142 TS files** in `packages/db/migrations/`, node-pg-migrate, run vs `DATABASE_URL_MIGRATIONS` with `--no-check-order`, `tsconfig.migrations.json` (NodeNext). Two timestamp eras (`1780310044710…` core build-out → `1790000000001…0067` hardening). Head `1790000000067_bom-seams`.
- **Forward-only:** `down()` bodies are intentional no-ops; correctness via staging-first + boot guard. (E.g. WhatsApp: `…020_notification_channel_whatsapp` added, `…043_remove-whatsapp-channel` removed — both stay.)
- **Boot guard / release_command:** `fly.toml release_command = "dist/migrate/index.cjs"` applies `001..head` pre-traffic; failure aborts rollout. The API bundle bakes `__EXPECTED_MIGRATION_HEAD__` and FATAL-exits if the DB head is behind (the 2026-06-20 schema-drift fix).
- **pg-boss bootstrap (delicate):** `…0006/0008/0009/0011` create the isolated `pgboss` schema, transiently grant→revoke CREATE from the runtime role, run the installer under `noTransaction()`+COMMIT (avoids a `pg_namespace` deadlock), and **pre-create all queues** (from `shared-types/queue-names`) so the runtime (no CREATE on `pgboss`) finds `createQueue()` idempotent. The `read_public_menu` SQL fn: defined in `…0022_read_public_menu`, evolved by `…055` (primary-media), `…063` (availability), `…064` (perf).

### 8.2 Tables — **84 DDL matches: ~83 app tables + the `pgboss` schema** (names verified from migration source; column details _verify against live DB_)
**Tenancy/identity:** users, organizations, locations, memberships, auth_refresh_tokens, phone_otp, customer_otp_sessions, domain_verifications.
**Menu:** categories, products, modifier_groups, modifiers, product_modifier_groups, order_item_modifiers, product_translations, category_translations, modifier_translations, modifier_group_translations, menu_versions, menu_schedules, location_themes, theme_versions, product_media, delivery_tiers, recurring_orders, reservations, import_sessions.
**Recipes/inventory (BOM seams):** ingredients, recipe_components.
**Orders:** orders, order_items, order_status_history, order_messages, order_ratings, order_routes, order_sensor_events, idempotency_keys, delivery_trace.
**Customers/signals:** customers, customer_signals, velocity_events, customer_devices, customer_track_grants, customer_contact_reveals, free_tier_snapshots.
**Couriers:** couriers, courier_locations, courier_invites, courier_sessions, courier_shifts, courier_assignments, courier_positions, courier_payouts, courier_audit_log, courier_cash_ledger, courier_dispatch_queue.
**Settlement/commerce:** settlement_items, settlement_audit_log, exchange_rates, promotions.
**Notifications:** owner_notification_targets, notification_outbox_audit, notification_prefs_audit, telegram_connect_tokens, telegram_login_tokens, telegram_action_nonces, access_requests, location_alerts.
**Analytics (intentionally NO RLS):** analytics_events, analytics_abuse_log, analytics_cwv, funnel_events, upload_audit.
**GDPR/infra:** gdpr_erasure_requests, anonymization_audit_log, backup_metadata, backup_audit_log, ops_worker_heartbeat, webhook_endpoints, api_keys.
**Queue:** `pgboss.*` (job table + per-queue partitions).

### 8.3 RLS (red-line — verified in `packages/db/src/index.ts` + verify-rls.ts)
Every tenant table `ENABLE` + `FORCE ROW LEVEL SECURITY`. Helpers `app_current_user()` (reads `current_setting('app.user_id', true)::uuid`) and `app_member_location_ids()` (SECURITY DEFINER, active memberships). Public read policies (`USING(true)`) on categories/products/locations; anonymous INSERT on orders/customers gated by `app_current_user() IS NULL`. Most tables key on `app.user_id`; courier tables on `app.current_tenant` _(known duality — confirm both are set in their paths)_. Set per request with `SET LOCAL` inside a transaction.

### 8.4 Pools (`packages/db/src/index.ts` — exact)
- **Operational** (`createOperationalPool`): `DATABASE_URL_OPERATIONAL` (:6543 Supavisor transaction mode, **no prepared-statement cache**), `max=OPERATIONAL_POOL_SIZE` (default 20), `statement_timeout 10s`, and a **connect-time guard that destroys the connection + throws if `current_user === 'postgres'`** (RLS-bypass prevention).
- **Session** (`createSessionPool`): `DATABASE_URL_SESSION` (:5432), `max=3`, `statement_timeout 30s` — workers/DDL/analytics.

### 8.5 Seed
`packages/db/scripts/seed.ts` — Owner A/B (RLS fixtures), courier, `test@dowiz.com`/`test123456` owning demo "Dubin & Sushi" (Durrës, slug `demo`, published, ALL minor_unit 0, flat fee 200, free-delivery 2000). Idempotent; argon2 hashes; courier PII encrypted.

### 8.6 Data-layer risks
🔴 RLS bypass if `DATABASE_URL_OPERATIONAL` points at a superuser/BYPASSRLS role (guard throws — keep it) · forgotten `SET LOCAL` → silent 0-rows / context bleed if `SET` used instead · new table missing `FORCE` · `app.user_id` vs `app.current_tenant` split. ⚠️ forward-only = no rollback (staging-first mandatory) · `--no-check-order` dependency order · pg-boss bootstrap deadlock if the noTransaction/COMMIT pattern is disturbed · queue-name drift vs `…0011` · analytics/anonymization prod↔staging drift.

---

## 9. Auth & Identity

- **JWT:** RS256 via jose (`platform`), `verifyAuthToken` in `plugins/auth.ts`. `JWT_PRIVATE_KEY`/`PUBLIC_KEY`/`KID` (all required); **dev keys `JWT_DEV_*` accepted only off-prod** (ADR-0003).
- **Access 24h, refresh 7d rotating** (hashed, family-based); `POST /auth/refresh` (5/min) preserves `activeLocationId`, re-derives role; `POST /auth/logout`. **Per-request owner check** (`auth.ts:149`): `SELECT 1 FROM memberships WHERE location_id=$1 AND user_id=$2 AND role='owner' AND status='active'` — index-backed insider-removal gate (regression #16).
- **Roles:** owner (userId, activeLocationId), courier (sub=courierId, **`jti` → `courier_sessions` row check**, activeLocationId), customer (orderId, locationId, customerId).
- **Login:** Telegram (token mint→bot poll, single-use), Google OAuth (PKCE, **gated off** by `GOOGLE_OAUTH_ENABLED`), local email/password (argon2).
- **Dev-login (ADR-0003, once a live prod backdoor):** triple-gated — `loadEnv()` FATAL-throws on prod if any dev knob set (`assertDevAuthDisabledInProd`, `config/index.ts:211`); requires BOTH `ALLOW_DEV_LOGIN='true'` AND `DEV_AUTH_SECRET`; dev-kid segregation; + Fly release_command asserts prod NODE_ENV.
- **Customer OTP:** `OTP_ENABLED` off (send is a console.log scaffold); per-location `require_phone_otp` only active when global flag on.

---

## 10. Security Posture
RLS (§8.3) · IP velocity gate (daily-salted `client_ip_hash` + `phone_hash`, `VELOCITY_THRESHOLD_1H/24H`→429, cron-raised signals; regression #20) · token-bucket rate limiting (`fastifyRateLimit` + per-route) · security-headers plugin (strict CSP w/ allowlists for fonts/tiles/OSRM/R2/nonce, HSTS prod, nosniff, SAMEORIGIN) · PII: claim-check on bus/WS, hashed IP/phone, **encrypted courier PII** (`COURIER_PII_ENCRYPTION_KEY`), GPS active-delivery-only, minimized Telegram bodies, Sentry `beforeSend` scrub, Pino redaction · anonymizer retention cron + GDPR erasure (`anonymization_audit_log` append-only, 0 PII) · `/compliance` SoT + CI `compliance-gate.ts` (blocks undocumented PII/subprocessor/raw-PII-on-logs/missing-DPIA; escape `// compliance-gate:allow`) · gitleaks + `verify-secrets`.

---

## 11. Integrations

| Integration | Use | Code | Key env | Degrade |
|---|---|---|---|---|
| Telegram | owner login, alerts, webhook | `notifications/adapters/telegram.ts`, `routes/telegram-webhook.ts` | `TELEGRAM_BOT_TOKEN/SECRET/USERNAME` (optional in schema) | 401/403 disable, 429 backoff |
| Google OAuth | owner sign-in (gated off) | `routes/auth.ts` | `GOOGLE_CLIENT_ID/SECRET` (**required**), `GOOGLE_OAUTH_ENABLED` | FE button hidden + backend 404 |
| Tiles | delivery map | `lib/tileConfig.ts` | `VITE_TILE_PROVIDER/STYLE_URL` | OpenFreeMap default |
| ORS routing | per-leg ETA baseline | `platform/RoutingProvider` | `ROUTING_PROVIDER/BASE_URL/API_KEY` | **haversine** |
| LLM menu OCR | PDF/photo → menu | `lib/ai-ocr-parser.ts` | Zen→OpenRouter→Groq→OpenAI keys; `LLM_ADAPTER/PROVIDER/ENDPOINT` | **heuristic** parser |
| Cloudflare R2 | media + backups | `lib/r2-storage.ts` | `R2_ENDPOINT/ACCESS_KEY_ID/SECRET_ACCESS_KEY/BUCKET/PUBLIC_URL` | local FS in dev |
| Web Push | customer push | `notifications/adapters/webpush.ts`, `routes/public/vapid.ts` | `VAPID_PUBLIC_KEY/PRIVATE_KEY` (**required**), `VAPID_SUBJECT` | silent if unsubscribed |
| Sentry | errors | `lib/sentry.ts` | `SENTRY_DSN` (optional) | disabled if unset |
| Resend | operator lead email | access-requests | `RESEND_API_KEY`, `WAITLIST_NOTIFY_EMAIL` | silent |
| Supabase / Fly / Upstash | DB / host / pub-sub | `db`, `fly.toml`, `platform` | `DATABASE_URL_*`, `FLY_*`, `REDIS_URL` | pool/boot guards |
| ~~WhatsApp/Baileys~~ | **removed** (P0-2, migrations 020→043, `config:44`) | — | — | only `.env.example` is stale (see §19) |

---

## 12. Feature Flags

| Flag | Default | Where read | Effect |
|---|---|---|---|
| `ALLOW_DEV_LOGIN` (+`DEV_AUTH_SECRET`) | false | config schema | master dev-auth gate; FATAL on prod |
| `GOOGLE_OAUTH_ENABLED` / `VITE_GOOGLE_OAUTH_ENABLED` | false | schema / **vite build** | Google sign-in (route + button) |
| `OTP_ENABLED` | false | schema | customer phone OTP |
| `MEDIA_RICH_ENABLED` / `VITE_MEDIA_RICH_ENABLED` | false | schema / **vite build** | rich media (ADR-0002; storefront read also gated by `plan='business'`) |
| `FUNNEL_INGEST_ENABLED` | **true** | schema | funnel sensor kill-switch (204 off) |
| `ACCESS_GATE_PUBLIC_ENABLED` / `VITE_ACCESS_GATE_PUBLIC_ENABLED` | false | schema / **vite build** | public register-interest CTA + route registration (**launch blocker STOP-1**) |
| `ACCESS_GATE_INVITE_GATING_SHIPPED` | false | schema | scarcity-copy CI gate companion |
| `BACKUP_ENABLED`, `DWELL_TIER3_ENABLED`, `RESTORE_VERIFY_FULL_HASH` | false | schema | ops toggles |
| `TG_CATEGORY_GATING`, `TG_STOREFRONT_ACTION` | (unset) | **`process.env` direct, NOT in Zod schema** | Telegram notification category prefs / storefront action |
| `VITE_TG_CATEGORY_GATING` | false | vite build | preference-centre UI render |
| `VITE_TILE_PROVIDER` / `VITE_TILE_STYLE_URL` | free / OpenFreeMap | vite build | map source |

> **Build-baked `VITE_*` flags ⇒ staging and prod need different Docker builds** (Dockerfile `ARG`s default false; staging passes true). Simplification opportunity: move user-visible gates to a runtime `/config` fetch so one image promotes staging→prod.

---

## 13. Environment Variables

**Authoritative = `packages/config/src/index.ts` `EnvSchema`.** Required (Zod `min(1)` / non-optional): `NODE_ENV`, `APP_BASE_URL`, `DATABASE_URL_OPERATIONAL/SESSION/MIGRATIONS`, `REDIS_URL`, `JWT_PRIVATE_KEY/PUBLIC_KEY/KID`, **`GOOGLE_CLIENT_ID/SECRET`**, **`VAPID_PUBLIC_KEY/PRIVATE_KEY`**, **`IP_HASH_SALT`**. (`PORT` defaults 8080.)

Optional / defaulted, grouped:
- **Dev-auth (off-prod only):** `DEV_AUTH_SECRET`, `ALLOW_DEV_LOGIN`(false), `DEV_LOGIN_EMAIL`, `DEV_LOGIN_PASSWORD`, `JWT_DEV_KID/PRIVATE_KEY/PUBLIC_KEY`.
- **OAuth/login:** `GOOGLE_OAUTH_ENABLED`(false), `TELEGRAM_BOT_TOKEN/SECRET/USERNAME`.
- **Flags:** `OTP_ENABLED`(false), `MEDIA_RICH_ENABLED`(false), `FUNNEL_INGEST_ENABLED`(true).
- **LLM:** `OPENROUTER_API_KEY/MODEL/ENDPOINT`, `OPENCODE_ZEN_API_KEY/MODEL/ENDPOINT`, `GROQ_API_KEY/MODEL/ENDPOINT`, `OPENAI_API_KEY/MODEL/ENDPOINT`, `LLM_ADAPTER/PROVIDER/ENDPOINT`, `MEM0_EMBED_MODEL/LLM_MODEL/OLLAMA_URL`.
- **Storage/backup:** `R2_ACCESS_KEY_ID/SECRET_ACCESS_KEY/ENDPOINT/BUCKET/PUBLIC_URL`, `BACKUP_ENABLED`(false), `BACKUP_ENCRYPTION_KEY`, `BACKUP_POOL_SIZE`(2), `BACKUP_*_CRON`, `BACKUP_*_RETENTION_*`, `BACKUP_PII_FIELDS`(default list), `DATABASE_URL_ADMIN`, `RESTORE_VERIFY_CRON`, `RESTORE_VERIFY_FULL_HASH`(false), `RESTORE_POOL_SIZE`(2).
- **Pool/perf:** `OPERATIONAL_POOL_SIZE`(20).
- **Dwell/signals/OTP/velocity:** `DWELL_CRON`, `DWELL_TIER2_DELAY_MS`(30000), `DWELL_TIER3_DELAY_MS`(90000), `DWELL_TIER3_ENABLED`(false), `DWELL_BATCH_THRESHOLD`(10), `SIGNAL_RAISE_CRON`(*/5), `OTP_SEND_RATE_LIMIT`(3), `OTP_VERIFY_RATE_LIMIT`(5), `OTP_TTL_MS`(300000), `VELOCITY_WINDOW_1H_S`(3600)/`24H_S`(86400), `VELOCITY_THRESHOLD_1H`(3)/`24H`(10).
- **Push:** `VAPID_SUBJECT`(push@deliveryos.app).
- **Anonymizer:** `ANONYMIZER_RETENTION_CRON`(0 3 * * *), `ANONYMIZER_RETENTION_BATCH_SIZE`(100), `R2_RETENTION_OVERRIDE_DAYS`.
- **Observability/worker:** `SENTRY_DSN`, `LOG_LEVEL`(info), `GIT_SHA`, `WORKER_HEARTBEAT_INTERVAL_MS`(15000), `WORKER_LIVENESS_CHECK_MS`(60000), `WORKER_LIVENESS_STALE_MS`(60000), `WORKER_CRITICAL_LIST`(dispatcher,settlement-cron,dwell-monitor,anonymizer-retention), `FLY_MACHINE_ID`, `HOSTNAME`, `RENDER_GIT_COMMIT`.
- **Routing:** `ROUTING_PROVIDER`(ors), `ROUTING_BASE_URL`(openrouteservice), `ROUTING_API_KEY`.
- **Rates/translation:** `RATES_CRON`(0 * * * *), `TRANSLATION_PROVIDER/ENDPOINT`.
- **Courier ops:** `COURIER_PII_ENCRYPTION_KEY`, `COURIER_ACCEPT_WINDOW_MS`, `CANCEL_AFTER_DISPATCH_WINDOW_MS`, `COURIER_DISPATCH_MAX_ATTEMPTS`, `COURIER_DISPATCH_RETRY_MS`, `COURIER_GPS_MAX_DIST_KM`.
- **Access gate:** `ACCESS_GATE_PUBLIC_ENABLED`(false), `ACCESS_GATE_INVITE_GATING_SHIPPED`(false), `RESEND_API_KEY`, `WAITLIST_NOTIFY_EMAIL`, `PRIVACY_NOTICE_VERSION`(2026-06-20), `ACCESS_REQUEST_RETENTION`(12 months), `ACCESS_REQUEST_RETENTION_CRON`, `ACCESS_REQUEST_RECONCILE_CRON`(*/15), `ACCESS_REQUEST_NOTIFY_MAX_ATTEMPTS`(10).
- **Read via `process.env` directly (NOT in schema):** `TG_CATEGORY_GATING`, `TG_STOREFRONT_ACTION`.
- **Vite build-time (in Dockerfile/FE):** `VITE_ACCESS_GATE_PUBLIC_ENABLED`, `VITE_TG_CATEGORY_GATING`, `VITE_MEDIA_RICH_ENABLED`, `VITE_GOOGLE_OAUTH_ENABLED`, `VITE_TILE_PROVIDER`, `VITE_TILE_STYLE_URL`, `VITE_API_BASE_URL`, `VITE_WS_BASE_URL`, `VITE_PROXY_TARGET`, `VITE_BASE_URL` (E2E).
- **⚠️ Stale (not in schema):** `WHATSAPP_ENABLED`, `WHATSAPP_AUTH_DIR` — present only in `.env.example`; channel removed in code.

> Generate the live list with `flyctl secrets list -a dowiz` / `-a dowiz-staging` and diff against this register — **lose nothing** in the move.

---

## 14. Build & Deployment

**Pipeline:** `pnpm -r build` (packages + `apps/web` via `tsc && vite build`) → `tsx scripts/build-apps.ts` (esbuild → single-file CJS `dist/api/server.cjs`, `dist/worker/index.cjs`, `dist/migrate/index.cjs` + per-migration cjs; injects `__EXPECTED_MIGRATION_HEAD__`; externals `argon2,sharp,@aws-sdk/*,@smithy/*,pg-native,fsevents`). `dist/public` = `apps/api/public` overlaid with `apps/web/dist`.

**Dockerfile (2-stage, node:22-slim):** builder runs pnpm install (frozen) → `-r build` → `build-apps.ts` (`ARG VITE_ACCESS_GATE_PUBLIC_ENABLED`, `VITE_TG_CATEGORY_GATING`, default false). Runtime copies `dist/` then **`npm install argon2 sharp @aws-sdk/client-s3 @aws-sdk/lib-storage` UNPINNED** (line 42) — native modules. ENTRYPOINT `node`.

**Fly (`fly.toml`):** app `dowiz`, region `fra`. Processes `web=dist/api/server.cjs` (512MB, http 8080, force_https, `/livez` check 15s/3s), `worker=dist/worker/index.cjs` (256MB). `release_command="dist/migrate/index.cjs"` pre-traffic. Staging `dowiz-staging` (separate Supabase DB, `VITE_*` build args true). `flyctl` at `~/.fly/bin`.

**Risks:** large single bundle (full rebuild/change) · unpinned runtime native installs · release_command coupling · build-baked flags (staging≠prod images) · heavy pre-commit (build+docker → `--no-verify` temptation) · transaction-mode pool forbids prepared-statement cache.

---

## 15. CI/CD & Quality Gates

**GitHub Actions:** `ci.yml` — validate (lint, lint:gates, `-r build`, `-r typecheck`, verify:migrations, verify:secrets, compliance:gate) · fresh-provision (PG16+Redis7: create `dowiz_migrator`/`dowiz_app` roles, migrate, seed, boot, `/health` 200) · deploy (main only: pre-migrate vs `DATABASE_URL_MIGRATIONS` → `flyctl deploy --remote-only` → Playwright smoke vs prod). `visual.yml` — pinned `playwright:v1.60.0-jammy`, compare mode. `skill-security.yml` — SkillSpector `--no-llm` SARIF.

**Husky pre-commit:** lint-staged → i18n parity → typecheck → build → flyctl config validate → docker build check.

**`eslint-plugin-local` (16 real rules, all registered as `warn` in `eslint.config.js`):** `no-raw-sql`, `no-hardcoded-string`, `no-hardcoded-color`, `no-hardcoded-tailwind-color`, `no-arbitrary-tailwind`, `no-arbitrary-font-size`, `no-raw-form-control`, `no-insecure-random`, `no-direct-websocket`, `no-process-exit`, `no-ts-nocheck`, `no-raw-any`, `no-duplicate-import`, `no-empty-catch`, `no-mock-in-prod`, `no-permissive-status-assertion`. (No `require-auth-hook` — that was an earlier inference; it does not exist.) Plus core complexity caps.

**Verify scripts:** verify:migrations, verify:secrets, verify:rls, verify:i18n-coverage, verify:contrast, verify:event-wiring, verify:schema-queries, verify:connection-lifecycle, verify:launch, compliance-gate, guardrail-owner-active-membership, guardrail-spike-boundary, backup verify/drill/list.

---

## 16. Invariants / Red-Lines

Carry the **enforcement mechanism**, not just the behavior. Changing any of these requires a new ADR + Council.

| # | Invariant | Enforcement |
|---|---|---|
| I1 | Money = integer minor units, no floats | `CHECK(>=0)`; `check-money.mjs`; `fee-parity.test.ts` (#17); BigInt tax |
| I2 | RLS FORCE tenant isolation | every tenant table FORCE; operational-pool non-superuser connect-guard (`db/src/index.ts:35`); `verify-rls.ts`; cross-tenant E2E |
| I3 | RS256 JWT w/ kid, reject alg=none | `platform` jose; dev-kid segregation; security-regression E2E |
| I4 | Forward-only migrations | no edits to applied files; boot guard FATAL on head mismatch; release_command |
| I5 | Idempotency keys | `idempotency_keys` PK `(location_id,key)`; `ON CONFLICT DO NOTHING`; concurrent-POST E2E |
| I6 | No PII leakage (claim-check) | IDs-only on bus/WS/queue; log redaction; Sentry scrub; compliance-gate |
| I7 | Dev-login fail-closed | `assertDevAuthDisabledInProd` FATAL + flag+secret + dev-kid + release_command; ADR-0003; #1 |
| I8 | Owner revocation ≤24h | 24h access + logout + per-request `status='active'` (`auth.ts:149`); ADR-0004; #16 |
| I9 | auth/money/RLS/`packages/db/migrations/`/bulk-edit = red-line globs | Mandatory Proof Rule (staging Playwright) + ledger row + Council |

**Monotonic ratchet:** `docs/regressions/REGRESSION-LEDGER.md` (20 rows) never weakens; no cheat-green.

---

## 17. ADRs

`docs/adr/` (15): **0001** queue-in-postgres · **0002** product-media-seam (inert, `MEDIA_RICH_ENABLED`) · **0003** dev-login-fail-closed (I7) · **0004** owner-token-revocation (I8) · **0005** delivery-fee-source-of-truth (server + mirror + parity gate, I1) · **0006** courier-status-display-model · **0007** stock-decrement-in-order-txn (inert seam) · **0008** bom-recipe-polymorphic-seam (`ingredients`/`recipe_components`) · **0009** sensor-bus-event-log-and-promised-window (no unit leak on terminal paths) · **ADR-GEO-SEAMS** (routing/eta/tiles) · **ADR-NOTIFICATION-CONSOLIDATION** (Telegram+push, **no native WhatsApp** — basis for the removal) · **ADR-p0-privacy-hardening** (GPS guard, remove WhatsApp/Baileys, PII minimization, TG detail) · **ADR-soft-access-gate** (launch blocker STOP-1) · **ADR-golive-remediation** · **ADR-TELEGRAM-NOTIFICATIONS-ACTIONS**. Plus `docs/decisions/CHALLENGE-LOG.md`.

---

## 18. Testing
**Proof model (Mandatory Proof Rule):** UI change → Playwright vs staging (`VITE_BASE_URL=https://dowiz-staging.fly.dev`) real-DOM assertions; API-only → ≥1 `request.*`. Public at `/s/:slug`, owner at `/admin/*`. **`e2e/`:** tests (~28 `flow-*`), lifecycle-e2e, journeys, personas (23), visual (180 deterministic), chaos, driver, rites; `e2e/MATRIX.md`. **`apps/api/tests/`:** stage tests `test-stage1..36`, `fee-parity.test.ts`, `eta-synthesis.test.ts`, `pii-leak-detector.test.ts`, phase5 rls-adversarial/jwt-rotation/integrity. **Reliability gate:** `/reliability-gate` traces one order L0–L11 → GO/NO-GO.

---

## 19. Tech Debt, Hotspots & Clutter

**Churn hotspots:** `ui/src/lib/i18n.ts` · `MenuManagerPage.tsx` · `MenuPage.tsx` · `routes/spa-proxy.ts` · `server.ts` · `routes/orders.ts` (999). **Worst health:** `routes/courier/shifts.ts` (1.0/10). **Prior-defect (−2.0):** `public/sw.js`, `routes/orders.ts`, `routes/customer/otp.ts`, `routes/owner/courier-invites.ts`, `routes/owner/gdpr.ts`.

**api↔worker duplication:** ~20 workers run inline in the API process AND in `apps/worker` — consolidate into one shared job registry.

**Root clutter (committed):** ~22 screenshots (`admin-*.png`, `checkout-*.png`, `login.png`, …) + `dubin-logo.jpg`; one-off scripts (`fix.js/.cjs/.mjs`, `fix-schedule.cjs`, `fix_ui.py`, `fix_spacing.py`, `analyze.mjs`, `eval_runs.py`, `clean-rls.ts`, `get-hash.sh`, `ignore-api.mjs`, `ignore-workers.mjs`); logs (`api-*.log`, `lint-errors*.txt`); `dev-hub.html`, `entities.json`; generated dirs (`audit/`, `.audit/`, `.polish/`, `.playwright-mcp/`, `playwright-report/`, `test-results/`, `temp/`, `dist/`, `graphify-out/`, `load/`, `.harness-backups/`). Heavy agent surface (`.agents/`, `.claude/`, `.opencode/`, `loops/`, `metric-core/`, `audit-sentinel/`, `spikes/`) — review for staleness.

**Confirmed-resolved (not debt):** WhatsApp/Baileys notification channel **is removed in code** (migrations `…020`→`…043`, `config:44`); **only `.env.example` still mentions `WHATSAPP_ENABLED`/Baileys** → delete those 5 lines. `messenger.ts`'s `wa.me` is a separate customer deep-link (keep).

**Verify-against-live-DB:** §8.2 column-level details (types, nullability) — dump prod schema before any data restructure.

---

## 20. Project Status
- **Prod (dowiz.fly.dev):** auth (Google/RS256/refresh), menu CRUD + CSV/PDF import + versioning, branding/themes + auto-brand, storefront (SSR `/s/:slug`, cart, checkout, tracking), owner dashboard (lifecycle, dispatch, dwell), courier app (invite→shift→GPS→complete), cash cycle + settlements, Telegram + Web Push, anonymizer + GDPR, encrypted R2 backups, anti-fake signals.
- **Dark/flag-gated on prod (off):** `MEDIA_RICH_ENABLED`, `DWELL_TIER3_ENABLED`, `GOOGLE_OAUTH_ENABLED`, `OTP_ENABLED`, `ACCESS_GATE_PUBLIC_ENABLED`, `ALLOW_DEV_LOGIN` (FATAL on prod). `FUNNEL_INGEST_ENABLED` on.
- **On branch (`feat/mvp-sensor-seams`, not prod):** sensor-bus §1.1 runtime proven on staging; §1.2–1.4 + §4 IP gate landed (ledger #18–20); migs 066/067 on staging; ADR-0007/0008/0009.
- **Pending / launch blockers:** owner-onboarding-invite-gating (gates soft-access-gate) · ADR-0004/0005/0006 merges · stock runtime (0007) · live courier presence (0006 B) · rich media Phases 2–5 · official WhatsApp Cloud API (replace removed Baileys) · real LLM menu parsing · billing/Stripe.
- **Deploy model:** push to `main` → CI deploys prod (release_command migrates pre-traffic); prod changes need approval/merge.

---

## 21. Migration Playbook

> Goal: large migration / restructure / simplification with **zero data loss, zero prod failure**. The system's own tools — forward-only migrations, the boot guard, RLS, flag-gated seams, the regression ratchet, staging-first ship discipline — are the safety net. Use them; never route around them.

### 21.0 Pin the scope first (use `AskUserQuestion` if unclear)
1. **Cleanup/simplification** (lowest risk) — clutter, `.gitignore`, stale `.env.example`, consolidations. No behavior change.
2. **Code restructure** — extract hotspots, unify the api↔worker job registry, move money to `@deliveryos/domain`, retire spent flags. Behavior-preserving.
3. **Infra/platform** — swap host/DB/bundler/framework behind the `@deliveryos/platform` abstractions. High blast radius.
4. **Schema/data** — restructure tables / tenancy / money-locale model. Touches red-lines.

### 21.1 Principles
- Preserve every §16 invariant **with its enforcement mechanism**. If the new structure can't host the guardrail, that's a blocker.
- Forward-only, **staging-first**: migrate staging DB → deploy dark → validate → prod. Never edit an applied migration.
- Flag everything not ready (default off). Deploying dark to verify ≠ launching.
- Proof or it didn't happen (staging Playwright / curl / test). Green typecheck ≠ proof.
- Red-line changes go through `/council` before code.
- One reversible, independently-shippable step at a time (the boot guard aborts a half-applied rollout safely).

### 21.2 Freeze a ground-truth baseline (before touching anything)
1. Tag HEAD; `pg_dump` prod + staging; drill a restore (`pnpm backup:drill`).
2. `pg_dump --schema-only` prod → canonical schema (supersedes §8.2 column detail).
3. `flyctl secrets list -a dowiz` / `-a dowiz-staging`; diff vs §13; vault every value. The Zod boot guard FATAL-ing on a missing var is your completeness check.
4. Capture a green baseline: full E2E + visual + stage tests + `pnpm typecheck` on staging; archive reports (your regression oracle).
5. Record migration head (`…0067`) and the branch↔main divergence; decide merge order first.
6. Resolve §19 discrepancies (delete stale `.env.example` WhatsApp lines) before building on them.

### 21.3 Recommended sequence (lowest-risk first)
- **Phase A — simplify (no behavior change):** fix `.gitignore`, `git rm --cached` the clutter (§19), move keep-worthy screenshots to `docs/`, consolidate/delete root one-off scripts, delete stale `.env.example` WhatsApp lines. Proof: `pnpm -r build && -r typecheck && lint`; full staging E2E unchanged-green.
- **Phase B — behavior-preserving restructure:** split `orders.ts` (fee ladder / idempotency / lifecycle / sensor) along ADR seams; thin the menu pages; **unify the api↔worker job registry into one shared module**; move money math to `@deliveryos/domain` (keep `fee-parity.test.ts` red→green); retire spent flags with a documented lifecycle. Proof: per-extraction commit with its guardrail/E2E green; visual snapshots unchanged.
- **Phase C — infra/schema (only if in scope):** swap providers behind `@deliveryos/platform`; for DB/host moves stand up the target, replay `001..head`, validate on a staging clone, cut over with the boot guard as the net.

### 21.4 Top failure modes (what breaks if careless)
RLS bypass (operational `DATABASE_URL` → superuser; the connect-guard throws only if kept) · forgotten `SET LOCAL app.user_id`/`app.current_tenant` (silent 0-rows, or bleed if `SET` not `SET LOCAL`) · schema drift (code ahead of DB — keep release_command + boot guard) · pg-boss bootstrap (disturbing noTransaction/COMMIT or the pre-created queue list → fresh-DB deadlock) · money floats · two `@deliveryos/domain` copies diverging between api/worker · promoting a staging image (VITE_*=true) to prod · removing `ALLOW_DEV_LOGIN` from the guards · unpinned native installs drifting on rebuild.

### 21.5 Guardrail strategy
Keep `eslint-plugin-local`, the verify scripts, compliance-gate and the ledger **active throughout** — never `--no-verify`; fix slow gates instead. New failure mode mid-migration → add a red→green guardrail + ledger row (ratchet). Run fresh-provision after **every** migration (bare DB → migrate → seed → boot → `/health` 200).

### 21.6 Data migration
Canonical schema = live `pg_dump`. Preserve append-only semantics (`anonymization_audit_log`, `courier_cash_ledger`, `*_audit_log`, `order_status_history`) and encrypted PII (courier bytea + `_hash`; never drop `_hash` lookups; re-key only with a planned re-encrypt). Run backfills in the session pool with correct tenant context (or migrator + explicit `WHERE location_id`). Carry `read_public_menu` + its indexes (storefront hot path). Keep the analytics tables' deliberate no-RLS decision.

### 21.7 Secrets & integrations
Move every §13 secret into the target store before first boot (boot guard = completeness check). Rotate high-value secrets in the move: `JWT_*` (bump `JWT_KID` — invalidates outstanding tokens, users re-auth), ensure `DEV_AUTH_SECRET`/`JWT_DEV_*`/`ALLOW_DEV_LOGIN` absent on prod, Telegram token, R2 keys, `BACKUP_ENCRYPTION_KEY` (with re-encrypt plan), `IP_HASH_SALT`. Re-test each degrade path (routing→haversine, LLM→heuristic, Sentry off, push unsubscribed).

### 21.8 Post-migration acceptance
- [ ] All §16 invariants enforced (guardrails green, ledger intact).
- [ ] Secret diff vs §13 = zero missing; boot guard passes.
- [ ] Fresh-provision: bare DB → `001..head` → seed → boot → `/health` 200.
- [ ] Operational pool `current_user ≠ postgres`; cross-tenant RLS E2E green.
- [ ] Money parity + check-money green.
- [ ] Dev-login FATAL on prod confirmed; dev knobs absent; JWT kid rotated.
- [ ] Full E2E + visual + stage tests green vs archived baseline.
- [ ] release_command/boot-guard drift protection in place on target.
- [ ] Compliance re-validated; compliance-gate green.
- [ ] Flag defaults verified (prod dark where required).
- [ ] Rollback drill-tested (DB snapshot restore + previous image).

### 21.9 Highest-value, lowest-risk simplifications (do regardless of scope)
1. `.gitignore` hygiene + remove committed clutter (§19).
2. Delete stale `.env.example` WhatsApp lines.
3. Move money math to `@deliveryos/domain`.
4. Document flag lifecycle; delete spent flags.
5. Split top hotspots along ADR seams (guardrails intact).
6. Unify the api↔worker job registry.
7. Pin runtime native-module versions in the Dockerfile.
8. (Optional) Move user-visible `VITE_*` gates to a runtime `/config` fetch (one image staging→prod).

---

## 22. Appendices

### A. Key file map
| Concern | File |
|---|---|
| API entry | `apps/api/src/server.ts` |
| Auth plugin (status='active', courier jti) | `apps/api/src/plugins/auth.ts` |
| Orders (red-line) | `apps/api/src/routes/orders.ts` |
| SPA/SSR proxy | `apps/api/src/routes/spa-proxy.ts` |
| Web entry | `apps/web/src/main.tsx` |
| API client / WS client | `apps/web/src/lib/apiClient.ts` · `useWebSocket.ts` |
| Env schema + boot guard | `packages/config/src/index.ts` |
| Pools + RLS guard | `packages/db/src/index.ts` |
| Migrations (142) | `packages/db/migrations/*.ts` |
| read_public_menu | `…0022_read_public_menu.ts` (+ 055/063/064) |
| Order state machine | `packages/domain/src/order-machine.ts` |
| Platform (JWT/bus/queue/routing/tenant) | `packages/platform/src/*` |
| i18n SSOT | `packages/ui/src/lib/i18n-catalog.ts` |
| Build bundler / migrator | `scripts/build-apps.ts` · `scripts/migrate-runner.ts` |
| Deploy | `fly.toml` · `Dockerfile` |
| Lint rails | `tools/eslint-plugin-local/src/index.js` · `eslint.config.js` |
| Compliance gate | `scripts/compliance-gate.ts` |
| Regression ledger | `docs/regressions/REGRESSION-LEDGER.md` |
| ADRs | `docs/adr/*` |

### B. Commands
```bash
pnpm -r build                 # build all packages + web
pnpm -r typecheck
pnpm lint  /  pnpm lint:gates
pnpm migrate:up               # node-pg-migrate vs DATABASE_URL_MIGRATIONS
pnpm seed                     # demo fixture (sushi-durres)
pnpm dlx tsx scripts/build-apps.ts          # esbuild single-file bundles
flyctl deploy -a dowiz-staging --remote-only
VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test <spec> --reporter=list
```

### C. Provenance
- **Read directly from source this pass:** env schema (`config/index.ts`), pools (`db/src/index.ts`), state machine (`domain/order-machine.ts`), `server.ts` (routes + worker starts + listen), `plugins/auth.ts`, `vite.config.ts`, lint rules (`eslint-plugin-local` + `eslint.config.js`), Dockerfile/fly.toml, real table names (84 DDL matches from migrations), migration count/head, web deps.
- **Verify against live DB before destructive use:** exact column types/nullability in §8.2; row-level data.
- **Single-owner repo (bus factor 1):** pair this with the author's review for Phase C/D.

---
*End of blueprint. Regenerate after major structural change by re-reading the authoritative files above and reconciling §8.2 against a fresh `pg_dump --schema-only`.*
