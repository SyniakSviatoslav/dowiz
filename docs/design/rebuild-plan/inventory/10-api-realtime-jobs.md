# 10 — API + Realtime + Jobs: Inventory & Rebuild Map (Lane A)

- **Date:** 2026-07-04 · **Lane:** A (backend behavioral surface) · **Status:** inventory complete, decisions rendered
- **Scope:** every HTTP route, WS message/event, job/queue/cron, server-side integration, and boot/lifecycle
  behavior in `apps/api` + `apps/worker`, mapped to its target in the decided stack
  (`06-complete-rebuild-stack.md`: Rust / axum / tokio / sqlx, OpenAPI 3.1 SSOT via utoipa,
  axum-WS + PgListener, Postgres-backed job queue — decided in §D below, DB schema UNCHANGED).
- **Machine-verifiability contract:** every census table carries its exact extraction command + count;
  re-run the command and diff against the table to re-derive the map. Deltas are explained inline, never silent.
- **🔴 rows** = money / auth / RLS / WS-authz / order-state-machine → **council-before-port** (Triadic Council
  + red→green proof per `06-complete-rebuild-stack.md` council note).

## 0. Census summary (re-derivable)

| Census | Extraction command (run from repo root) | Count |
|---|---|---|
| HTTP route registrations | `cd apps/api && grep -rnE "^\s*(fastify\|app\|server\|f\|instance)\.(get\|post\|put\|patch\|delete\|all\|head\|options\|route)\(" src --include="*.ts" \| grep -vE "\.test\.\|\.spec\." \| wc -l` | **236** |
| — style blind-spot check | same grep with `\s*<` (generic-typed), `\.route\({`, chained non-line-start variants, all `= 0` | 0 missed |
| Non-route HTTP surface | `grep -rn "setNotFoundHandler\|setErrorHandler" apps/api/src --include="*.ts"` → server.ts:443 (error envelope ADR-0010), server.ts:841 (SPA fallback) | 2 handlers |
| WS inbound message types | `grep -nE "msg\.type ===" apps/api/src/websocket.ts` | **5** |
| WS outbound event types | bus→room event census in §2 — 24 in-frame `type` values from 33 `messageBus.publish()` sites + 2 synthetic eviction frames; the 34 internal `BUS_CHANNELS.*` topics are censused separately (never reach a browser) | **24** |
| WS room kinds | room-constructor census in §2 (`order:<id>`, `location:<id>:dashboard`, `location:<id>:couriers`, `courier:<sub>`, `courier:<id>:shift` — last one has no FE consumer) | **5** |
| Job queues (live) | queue census in §3 — 33 name constants, 3 dead (`dwell.escalate` broken/dead, `order.pending_aging` + `settlement.cron`-as-queue never implemented) | **30** |
| Cron schedules | `.schedule()` census in §3 (all UTC; 1 raw grep hit is a comment) | **23** |
| Transactional-enqueue sites | §3 — `apps/api/src/lib/order-persistence.ts:158-173` (2 enqueues inside the POST /orders txn; verified at pg-boss driver level) | **1 file / 2 enqueues** |
| Server-side integrations | §4 — 16 areas censused; 13 active, 3 dead/inert (Telegram poll-mode, Turnstile plugin never registered, `adapters/push.ts` scaffold) | **16 (13 active)** |
| Env vars | §5 — 80 Zod `EnvSchema` fields + `grep -rhoE "process\.env\.[A-Z_0-9]+" apps/api/src apps/worker/src packages/db/src packages/config/src \| sort -u \| wc -l` = 48 raw reads (~20 unvalidated "shadow" vars) | **80 schema + 48 raw** |
| Feature flags (server + VITE mirrors) | `grep -rhoE "[A-Z_]*_ENABLED\b" --include=*.ts --include=*.tsx apps packages \| sort -u \| wc -l` | **35** |
| Proof universe | `find e2e -name "*.spec.ts" \| wc -l` = 174 (170 `e2e/tests`, 3 `e2e/visual`, 1 `e2e/lifecycle-e2e`) + `find apps/api -name "*.test.ts" -not -path "*node_modules*" \| wc -l` = 124 unit/int + `apps/api/e2e/api-integrity.spec.ts` | 174 E2E + 124 unit |

## 0.1 Target crate tree (proposal — Lane A)

```
crates/
  api/                       # single binary; workers co-hosted behind a role flag (mirrors apps/api + apps/worker)
    src/
      main.rs                # boot sequence (§5 mapping)
      boot/{env.rs, schema_guard.rs, pools.rs, shutdown.rs, flags.rs}
      extractors/{auth.rs, tenant.rs, rate_limit.rs, correlation.rs}   # 🔴 auth/tenant = council
      error.rs               # ADR-0010 envelope as axum IntoResponse
      routes/
        public/…  owner/…  courier/…  customer/…  admin/…  auth/…  dev/…   # 1:1 with TS files (§1 tables)
        webhooks/{telegram.rs, payments.rs}                                # 🔴 signature verification
      ssr/                   # spa-proxy successor; most behavior migrates to Astro shell (§1.3 split)
      ws/{mod.rs, auth.rs, rooms.rs, protocol.rs, pg_fanout.rs}            # 🔴 ADR-0013 tri-state authz
      jobs/                  # one module per queue (§3); runtime = §D queue pick
      integrations/{telegram.rs, push.rs, email.rs, r2.rs, otp.rs, plisio.rs,
                    imaging.rs, ocr.rs, pdf.rs, translate.rs, rates.rs, turnstile.rs}
      notifications/         # event→registry→render→adapter→retry→audit pipeline (§4.2)
      observability.rs       # /health /livez /metrics
```

utoipa note: every route handler gets `#[utoipa::path]`; request/response structs derive `ToSchema`;
the OpenAPI 3.1 doc is the SSOT the FE client is generated from — Zod schemas in §1 tables are the
source contracts to transcribe (not re-derive).

**Image-topology consequence of the §7 decisions** (libvips FFI + pdfium need glibc; tesseract's
transitive deps alone exceed the whole 15–25 MB scratch budget): split into **two images** —
`crates/api` stays scratch/static (~15–25 MB: all routes + WS + non-media jobs), and a
`crates/media-worker` on Debian-slim carries libvips + pdfium + tesseract and consumes the
menu-import / imaging job queues. This preserves the stack doc's RSS goal for the hot path while
keeping media features whole. (Alternative if one image is preferred: everything on Debian-slim,
~+80–120 MB image size, same RSS.)

## 0.2 Document map

| § | Contents | Rows/items |
|---|---|---|
| 1.1 | HTTP routes — owner | 113 rows |
| 1.2 | HTTP routes — courier + customer + core (incl. order/assignment state-machine map) | 51 rows |
| 1.3 | HTTP routes — public + admin + dev + webhooks + spa-proxy + infra, plus non-route HTTP surface (static/CORS/limits/error+404 handlers) and cache-TTL table | 72 rows |
| 2 | WebSocket protocol: connection/auth, inbound, outbound, rooms, fan-out (LISTEN/NOTIFY + claim-check), liveness, Rust ws/ mapping | 5 in / 24 out / 5 rooms |
| 3 | Jobs/queues/cron: queue census, transactional enqueue, worker topology, cron census, failure semantics | 30 queues / 23 crons |
| 4 | Server-side integrations + full dependency classification | 16 areas (13 active) |
| 5 | Boot/lifecycle: env preflight, schema guard, pools, boot order, shutdown, flags, error envelope | 80+48 env / 35 flags |
| 6 | Synthesis: 🔴 register (≈121 items → 8 proposed councils), orphaned/dead surface, pre-existing bugs found, honesty gaps | — |
| 7 | Decisions (researched + cited 2026-07-04): job queue, imaging, OCR, PDF | 4 verdicts |


---

<!-- ============ §1.1 HTTP ROUTES — OWNER (113) ============ -->

# OWNER route census — Rust rebuild map

**Extraction command:**
```
cd /root/dowiz/apps/api && grep -rnE "^\s*(fastify|app|server|f|instance)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\."
```
filtered to `routes/owner/*.ts`.

**Expected total: 113 · Actual row count: 113 · Delta: 0.** No false positives — every grep hit is a live registration; all 27 files matched their expected per-file counts exactly.

Prefix resolution source: `apps/api/src/bootstrap/routes.ts::registerCoreRoutes` (main registration block) + `apps/api/src/server.ts` (tail registrations for `product-media.ts` and `refunds.ts`, both dynamically imported after `registerCoreRoutes` resolves). Auth guard shorthand:
- **OWNER+LOC** = `verifyAuth` → `requireRole(['owner'])` → `requireLocationAccess` (either as `addHook('onRequest', …)`, `preValidation:[…]`, or `preHandler:[…]` — functionally identical middleware chain)
- **OWNER** = `verifyAuth` + `requireRole(['owner'])` only — location resolved *inside* the handler from JWT `activeLocationId`/DB membership (`getOwnerLocationId`/`getLocationId`/`getOwnerLocation` helpers), not from the URL
- **PUBLIC** = no auth hook at all

---

## routes/owner/products.ts — prefix: self-prefixed (registered with no `prefix` option; every path is absolute in-file)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | products.ts:15 | POST | `/api/owner/locations/:locationId/products` | OWNER+LOC | body: name/price/prep_time_minutes(default 15)/available/image_key/attributes/sort_order `.strict()` | 201 product row | none (Zod 400 only) | global | — | | crates/api/src/routes/owner/products.rs | tests/flow-ui-owner-crud.spec.ts |
| 2 | products.ts:52 | GET | `/api/owner/locations/:locationId/products` | OWNER+LOC | querystring: cursor/limit/category_id/available `.strict()` | 200 `{data:[]}` cursor-paged | none | global | — | | products.rs | flow-ui-owner-crud.spec.ts |
| 3 | products.ts:99 | GET | `.../products/:id` | OWNER+LOC | params | 200 product row | 404 NOT_FOUND | global | — | | products.rs | flow-ui-owner-crud.spec.ts |
| 4 | products.ts:117 | PATCH | `.../products/:id` | OWNER+LOC | body partial `.strict()` | 200 row | 400 NO_UPDATES, 404 | global | — | | products.rs | flow-ui-owner-crud.spec.ts |
| 5 | products.ts:168 | DELETE | `.../products/:id` | OWNER+LOC | params | 204 | 404 | global | — | | products.rs | flow-ui-owner-crud.spec.ts |
| 6 | products.ts:187 | PUT | `.../products/:id/translations/:locale` | OWNER+LOC | body name/description `.strict()` | 200 translation row | 400 UNSUPPORTED_LOCALE, 404 | global | — | | products.rs | flow-onboarding-parsing.spec.ts (locale coverage) |
| 7 | products.ts:239 | GET | `.../products/:id/translations` | OWNER+LOC | params | 200 `{data:[]}` | none (parent-existence not checked → silent empty) | global | — | | products.rs | needs-new-E2E |
| 8 | products.ts:263 | DELETE | `.../products/:id/translations/:locale` | OWNER+LOC | params | 204 | 404 | global | — | | products.rs | needs-new-E2E |
| 9 | products.ts:289 | PUT | `.../products/:id/modifier-groups` | OWNER+LOC | body array `{group_id,sort_order}` | 200 `{success:true}` | 404 NOT_FOUND (product), 400 INVALID_GROUP | global | — | | products.rs | flow-modifiers-promotions.spec.ts, flow-ingredients.spec.ts |
| 10 | products.ts:346 | GET | `.../products/:id/modifier-groups` | OWNER+LOC | params | 200 `{data:[]}` | none | global | — | | products.rs | flow-modifiers-promotions.spec.ts |
| 11 | products.ts:372 | GET | `/api/owner/menu/products` (JWT alias, no locationId in path) | OWNER | querystring category_id `.strict()` | 200 array (mapProductRow) | 401 UNAUTHORIZED | global | — | | products.rs | flow-ui-admin-menumanager.spec.ts |
| 12 | products.ts:396 | POST | `/api/owner/menu/products` | OWNER | body `.strip()` (name/price/prep_time/taste/recipeLines/attributes …) | 201 mapped row | 401 | global | — | | products.rs | flow-ui-admin-product-bom.spec.ts |
| 13 | products.ts:440 | PATCH | `/api/owner/menu/products/:productId` | OWNER | body `.strip()` | 200 mapped row | 401, 404 | global | — | | products.rs | flow-ui-admin-product-bom.spec.ts |
| 14 | products.ts:501 | DELETE | `/api/owner/menu/products/:productId` | OWNER | params | 204 | 401, 404 | global | — | | products.rs | flow-ui-owner-crud.spec.ts |

**Subtotal products.ts = 14** (expected 14 ✓)

---

## routes/owner/categories.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 15 | categories.ts:20 | POST | `/api/owner/locations/:locationId/categories` | OWNER+LOC | body name/sort_order/image_key `.strict()` | 201 category | none | global | — | | crates/api/src/routes/owner/categories.rs | flow-ui-owner-crud.spec.ts |
| 16 | categories.ts:51 | GET | `.../categories` | OWNER+LOC | querystring cursor/limit `.strict()` | 200 `{data:[]}` | none | global | — | | categories.rs | flow-ui-owner-crud.spec.ts |
| 17 | categories.ts:88 | GET | `.../categories/:id` | OWNER+LOC | params | 200 category | 404 | global | — | | categories.rs | flow-ui-owner-crud.spec.ts |
| 18 | categories.ts:112 | PATCH | `.../categories/:id` | OWNER+LOC | body `.strict()` | 200 category | 400 NO_UPDATES, 404 | global | — | | categories.rs | flow-ui-owner-crud.spec.ts |
| 19 | categories.ts:149 | DELETE | `.../categories/:id` | OWNER+LOC | params | 204 | 409 CATEGORY_NOT_EMPTY, 404 | global | — | | categories.rs | flow-ui-owner-crud.spec.ts |
| 20 | categories.ts:189 | GET | `/api/owner/menu/categories` (JWT alias) | OWNER | none | 200 array w/ product_count | 401 | global | — | | categories.rs | flow-ui-admin-menumanager.spec.ts |
| 21 | categories.ts:211 | POST | `/api/owner/menu/categories` | OWNER | body name `.strict()` | 201 | 401 | global | — | | categories.rs | flow-ui-admin-menumanager.spec.ts |
| 22 | categories.ts:233 | DELETE | `/api/owner/menu/categories/:id` | OWNER | params | 204 | 401, 409, 404 | global | — | | categories.rs | flow-ui-owner-crud.spec.ts |

**Subtotal categories.ts = 8** (expected 8 ✓)

---

## routes/owner/settlements.ts — prefix `/api/owner/locations` (routes.ts:131) · 🔴 MONEY (courier payouts) on every row

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 23 | settlements.ts:14 | GET | `/api/owner/locations/:locationId/settlements` | OWNER+LOC (top-level addHook) | querystring status/courier_id/period | 200 `{payouts:[]}` masked courier name | **swallows all errors → `{payouts:[]}`** (try/catch with no rethrow — silent-empty on real 5xx) | global | — | 🔴 money | crates/api/src/routes/owner/settlements.rs | flow-regulatory-settlements.spec.ts |
| 24 | settlements.ts:75 | GET | `.../settlements/:id` | OWNER+LOC | params | 200 `{payout,items}` | 404 | global | — | 🔴 money | settlements.rs | flow-regulatory-settlements.spec.ts |
| 25 | settlements.ts:110 | POST | `.../settlements/:id/approve` | OWNER+LOC | params | 200 success | 409 CONFLICT | 30/min | — | 🔴 money | settlements.rs | flow-regulatory-settlements.spec.ts |
| 26 | settlements.ts:162 | POST | `.../settlements/:id/pay` | OWNER+LOC | body payment_reference/payment_method | 200 success | 409 | 30/min | — | 🔴 money | settlements.rs | flow-regulatory-settlements.spec.ts |
| 27 | settlements.ts:206 | POST | `.../settlements/:id/dispute` | OWNER+LOC | body reason/items | 200 success | 409 | 10/min | — | 🔴 money | settlements.rs | flow-regulatory-settlements.spec.ts |
| 28 | settlements.ts:257 | POST | `.../settlements/:id/reopen` | OWNER+LOC | body reason | 200 success | 409 | 10/min | — | 🔴 money | settlements.rs | flow-regulatory-settlements.spec.ts |
| 29 | settlements.ts:301 | POST | `.../settlements/regenerate` | OWNER+LOC | body referenceDate | 200 success | none (dynamic-imports SettlementCronWorker; comment admits it processes **all locations**, not just this one) | 5/5min | — | 🔴 money + cross-tenant blast radius | settlements.rs | deploy-validation.spec.ts |

**Subtotal settlements.ts = 7** (expected 7 ✓)

---

## routes/owner/modifier-groups.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 30 | modifier-groups.ts:14 | POST | `/api/owner/locations/:locationId/modifier-groups` | OWNER+LOC | body name/min_select/max_select/required/display_type | 201 group | none | global | — | | crates/api/src/routes/owner/modifier_groups.rs | flow-modifiers-promotions.spec.ts |
| 31 | modifier-groups.ts:48 | GET | `.../modifier-groups` | OWNER+LOC | params | 200 `{data:[]}` w/ modifier_count | none | global | — | | modifier_groups.rs | flow-modifiers-promotions.spec.ts |
| 32 | modifier-groups.ts:73 | PATCH | `.../modifier-groups/:id` | OWNER+LOC | body partial `.strict()` | 200 group | 400 VALIDATION_FAILED, 404 | global | — | | modifier_groups.rs | flow-ingredients.spec.ts |
| 33 | modifier-groups.ts:117 | DELETE | `.../modifier-groups/:id` | OWNER+LOC | params | 204 | 404 | global | — | | modifier_groups.rs | flow-modifiers-promotions.spec.ts |
| 34 | modifier-groups.ts:136 | POST | `.../modifier-groups/:groupId/modifiers` | OWNER+LOC | body name/price_delta/available/sort_order | 201 modifier | 404 NOT_FOUND (group ownership folded into INSERT) | global | — | | modifier_groups.rs | client/modifier-display-type.spec.ts |
| 35 | modifier-groups.ts:172 | PATCH | `.../modifiers/:id` | OWNER+LOC | body partial `.strict()` | 200 modifier | 400 VALIDATION_FAILED, 404 | global | — | | modifier_groups.rs | flow-ingredients.spec.ts |
| 36 | modifier-groups.ts:214 | DELETE | `.../modifiers/:id` | OWNER+LOC | params | 204 | 404 | global | — | | modifier_groups.rs | flow-modifiers-promotions.spec.ts |

**Subtotal modifier-groups.ts = 7** (expected 7 ✓)

---

## routes/owner/dashboard.ts — prefix `/api/owner/locations` (routes.ts:132) · 🔴 order state-machine on 5 of 7 rows

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 37 | dashboard.ts:23 | GET | `/api/owner/locations/:locationId/dashboard/snapshot` | OWNER+LOC | querystring status/limit/cursor (hand-parsed, no Zod) | 200 snapshot (orders/counts/activeDeliveries/cursor) | none explicit | global | — | | crates/api/src/routes/owner/dashboard.rs | flow-ui-admin-dashboard.spec.ts |
| 38 | dashboard.ts:193 | POST | `.../orders/:orderId/confirm` | OWNER+LOC | none | 200 `{id,status,statusUpdatedAt}` | 404 (order not found), 403 (assertOwnerTargetAllowed — illegal transition) | 30/min | — | 🔴 order-state | dashboard.rs | flow-core-lifecycles.spec.ts |
| 39 | dashboard.ts:203 | POST | `.../orders/:orderId/reject` | OWNER+LOC | body reason? | 200 | 404, 403 | 30/min | — | 🔴 order-state | dashboard.rs | flow-core-lifecycles.spec.ts |
| 40 | dashboard.ts:215 | POST | `.../orders/:orderId/assign-courier` | OWNER+LOC | body `{courierId}` (hand-checked) | 200/201 assignment | 400 VALIDATION_FAILED, 404 (order/courier), 409 CONFLICT (bad status) | 10/min | `COURIER_OFFER_HANDSHAKE_ENABLED` (offer-vs-force-accept branch) | 🔴 order-state + courier dispatch | dashboard.rs | ws-courier-assignment.spec.ts |
| 41 | dashboard.ts:379 | POST | `.../orders/:orderId/pickup` (owner proxy for courier) | OWNER+LOC | none | 200 | 404, 409 (×2) | 10/min | — | 🔴 order-state | dashboard.rs | flow-ui-order-lifecycle.spec.ts |
| 42 | dashboard.ts:447 | POST | `.../orders/:orderId/deliver` (owner proxy) | OWNER+LOC | body payment_outcome/cash_collected/cash_amount `.strict()` | 200 | 404, 409, 422 (CompletionError) | 10/min | — | 🔴 order-state + money (cash-as-proof) | dashboard.rs | deliver-v2-cancel-revert.spec.ts |
| 43 | dashboard.ts:539 | GET | `.../orders/:orderId/verify` | OWNER+LOC | params | 200 order+items+assignments+auditLogs | 404 | 30/min | — | | dashboard.rs | flow-order-lifecycle-trace.spec.ts |

**Subtotal dashboard.ts = 7** (expected 7 ✓)

---

## routes/owner/promotions.ts — self-prefixed (no requireLocationAccess anywhere — location resolved via `getLocationId()` JWT/membership helper)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 44 | promotions.ts:36 | GET | `/api/owner/promotions` | OWNER | querystring is_active/type/limit/offset `.strict()` | 200 `{promotions,total}` | none | global | — | | crates/api/src/routes/owner/promotions.rs | admin/promotions.spec.ts |
| 45 | promotions.ts:91 | POST | `/api/owner/promotions` | OWNER | body code/type/discount_value/… `.strict()` | 201 promotion | none (no unique-code handling → 500 on dup) | global | — | | promotions.rs | flow-modifiers-promotions.spec.ts |
| 46 | promotions.ts:143 | POST | `/api/owner/promotions/validate` | OWNER | body code/order_subtotal/product_ids `.strict()` | 200 `{valid,discount_amount}` (never errors — always 200 w/ valid:false) | none | global | — | | promotions.rs | flow-ui-settings-promotions.spec.ts |
| 47 | promotions.ts:224 | GET | `/api/owner/promotions/:id` | OWNER | params | 200 promotion | 404 | global | — | | promotions.rs | admin/promotions.spec.ts |
| 48 | promotions.ts:253 | PATCH | `/api/owner/promotions/:id` | OWNER | body partial `.strict()` | 200 promotion | 400 VALIDATION_FAILED, 404 | global | — | | promotions.rs | admin/promotions.spec.ts |
| 49 | promotions.ts:329 | DELETE | `/api/owner/promotions/:id` | OWNER | params | 200 `{success:true}` (inconsistent — not 204) | 404 (via `reply.status(404).send`, not `sendError` — inconsistent envelope) | global | — | | promotions.rs | flow-ui-settings-promotions.spec.ts |

**Subtotal promotions.ts = 6** (expected 6 ✓)

---

## routes/owner/signals.ts — prefix `/api/owner/locations` (routes.ts:135)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 50 | signals.ts:20 | GET | `/api/owner/locations/:locationId/signals` | OWNER+LOC | querystring status/kind/limit/cursor | 200 `{signals,nextCursor}` masked PII | none | global | — | | crates/api/src/routes/owner/signals.rs | flow-simpl-s1-velocity-frictionless.spec.ts |
| 51 | signals.ts:105 | GET | `.../signals/compute` | OWNER+LOC | querystring phone_hash/ip_hash/customer_id | 200 `{signals,computedAt}` | none | global | — | | signals.rs | flow-sensor-geofence.spec.ts |
| 52 | signals.ts:129 | POST | `.../signals/:signalId/acknowledge` | OWNER+LOC | params | 200 | 404 | global | — | | signals.rs | needs-new-E2E |
| 53 | signals.ts:167 | POST | `.../signals/:signalId/dismiss` | OWNER+LOC | body reason? `.strict()` | 200 | 404 | global | — | | signals.rs | needs-new-E2E |
| 54 | signals.ts:198 | POST | `.../orders/:orderId/mark-no-show` | OWNER+LOC | params | 200 `{success,customerId}` | 404, 400 (no customer on order) | global | — | 🔴 order-state (drives CANCELLED via updateOrderStatus) | signals.rs | flow-core-lifecycles.spec.ts |

**Subtotal signals.ts = 5** (expected 5 ✓)

---

## routes/owner/product-media.ts — registered directly in **server.ts** (not routes.ts) with `prefix: '/api/owner'` (server.ts:816); no `requireLocationAccess` — location from `getOwnerLocation()` DB-membership helper

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 55 | product-media.ts:82 | POST | `/api/owner/menu/products/:productId/media/presign` | OWNER | body kind/items[] (hand-validated: mime allow-list, sha256, per-file+per-location budget) | 200 `{uploads,expiresIn}` | 401, 404, 400 (×3: invalid kind/sha/mime/bytes), 413 (file/budget), 503 (R2 unconfigured or SDK import fail) | 10/min | dark unless `R2_BUCKET`/`R2_ENDPOINT` env set (else 503) | | crates/api/src/routes/owner/product_media.rs | needs-new-E2E (MediaManager.tsx is the FE caller — apps/web/src/components/admin/MediaManager.tsx) |
| 56 | product-media.ts:178 | POST | `.../media/confirm` | OWNER | body storageKey/mimeType/bytes/… | 201 `{id,sortOrder}` | 401, 404, 400 (×3), key-prefix check | global | R2 magic-byte recheck is a no-op when storage absent (staging) | | product_media.rs | needs-new-E2E |
| 57 | product-media.ts:271 | POST | `.../media/:mediaId/set-primary` | OWNER | params | 200 `{ok,changed}` | 401, 404 | global | — | | product_media.rs | needs-new-E2E |
| 58 | product-media.ts:303 | POST | `.../media/reorder` | OWNER | body `{order:string[]}` | 200 `{ok:true}` | 401, 400 | global | — | | product_media.rs | needs-new-E2E |
| 59 | product-media.ts:330 | PATCH | `.../media/:mediaId` | OWNER | body `{available:boolean}` | 200 `{id,available}` | 401, 400, 404 | global | — | | product_media.rs | needs-new-E2E |

**Subtotal product-media.ts = 5** (expected 5 ✓)

---

## routes/owner/onboarding.ts — prefix `/api/owner` (routes.ts:142)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 60 | onboarding.ts:35 | POST | `/api/owner/onboarding/start` | OWNER (no locationId yet — creates one) | body name/phone/slug/currency/locales/anonymous_import_id `.strict()` | 201 `{locationId,slug,onboardingState,seeded}` | 409 SLUG_TAKEN | 3/min | — | 🔴 auth/tenant bootstrap (SECURITY DEFINER `bootstrap_owner()` fn — first-membership chicken/egg) | crates/api/src/routes/owner/onboarding.rs | onboarding-wizard-retired.spec.ts, onboarding-e2e.spec.ts |
| 61 | onboarding.ts:144 | GET | `/api/owner/onboarding/:locationId/state` | requireLocationAccess (added per-route, not addHook) | params | 200 state | 404 | global | — | | onboarding.rs | menu-first-onboarding.spec.ts |
| 62 | onboarding.ts:174 | POST | `.../step/complete` | requireLocationAccess | body `{step}` | 200 | 404, 400 (step mismatch, plain `reply.status().send`, not sendError) | global | — | | onboarding.rs | onboarding-e2e.spec.ts |
| 63 | onboarding.ts:247 | POST | `.../step/:stepNum/skip` | requireLocationAccess | params coerced | 200 | 400 (not skippable), 404, 400 (STEP_ALREADY_COMPLETED) | global | — | | onboarding.rs | onboarding-e2e.spec.ts |
| 64 | onboarding.ts:315 | GET | `.../complete` | requireLocationAccess | params | 200 `{slug,dashboardUrl}` | 404, 400 ONBOARDING_INCOMPLETE | global | — | | onboarding.rs | flow-onboarding-auth.spec.ts |

**Subtotal onboarding.ts = 5** (expected 5 ✓)

---

## routes/owner/notifications.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 65 | notifications.ts:16 | GET | `/api/owner/locations/:locationId/notifications/targets` | OWNER+LOC | params | 200 `{targets:[]}` | none | global | — | | crates/api/src/routes/owner/notifications.rs | notif-categories.spec.ts |
| 66 | notifications.ts:32 | GET | `.../notifications/status` | OWNER+LOC | params | 200 `{channels,anyActive,telegramConnected}` | none | global | — | | notifications.rs | real-notifications.spec.ts |
| 67 | notifications.ts:54 | POST | `.../notifications/telegram/connect-init` | OWNER+LOC | params | 200 `{deepLink,token}` | none | global | — | | notifications.rs | telegram-full-flow.spec.ts |
| 68 | notifications.ts:81 | POST | `.../notifications/test` | OWNER+LOC | body targetId? `.strict()` | 200 `{enqueued}` | none | global | — | | notifications.rs | notification-events.spec.ts |
| 69 | notifications.ts:118 | PUT | `.../notifications/targets/:targetId` | OWNER+LOC | body status/prefs/locale `.strict()` | 200 `{success:true}` | 404 NOT_FOUND | global | — | | notifications.rs | notif-categories.spec.ts |

**Subtotal notifications.ts = 5** (expected 5 ✓)

---

## routes/owner/gdpr.ts — prefix `/api/owner/locations` (routes.ts:140) · 🔴 PII/erasure on every row

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 70 | gdpr.ts:33 | POST | `/api/owner/locations/:locationId/gdpr-requests` | OWNER+LOC | body customerId/phone/reason (refine: one required) | 201 `{requestId,status}` | 404 (cross-tenant customerId — logged as `cross_tenant_attempt` then masked as plain 404), 409 CONFLICT, 429 RATE_LIMIT (24h cooldown) | 30/min | — | 🔴 PII/irreversible erasure + cross-tenant IDOR guard | crates/api/src/routes/owner/gdpr.rs | soft-access-gate.spec.ts |
| 71 | gdpr.ts:139 | GET | `.../gdpr-requests` | OWNER+LOC | querystring status/limit/cursor | 200 `{requests,nextCursor}` (customerId masked via `maskName` — questionable since it's a UUID not a name) | none | global | — | 🔴 PII | gdpr.rs | flow-regulatory-settlements.spec.ts |
| 72 | gdpr.ts:199 | GET | `.../gdpr-requests/:requestId` | OWNER+LOC | params | 200 request+auditLogs | 404 | global | — | 🔴 PII | gdpr.rs | flow-regulatory-settlements.spec.ts |
| 73 | gdpr.ts:257 | GET | `.../settings/retention` | OWNER+LOC | params | 200 `{retentionDays}` | 404 | global | — | 🔴 compliance policy | gdpr.rs | needs-new-E2E |
| 74 | gdpr.ts:272 | PUT | `.../settings/retention` | OWNER+LOC | body `{retentionDays:30-2555}` `.strict()` | 200 | 404 | global | — | 🔴 compliance policy | gdpr.rs | needs-new-E2E |

**Subtotal gdpr.ts = 5** (expected 5 ✓)

---

## routes/owner/couriers.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 75 | couriers.ts:22 | GET | `/api/owner/locations/:locationId/couriers` | OWNER+LOC (F3-fixed: `requireRole` was missing) | none | 200 `{couriers:[]}` decrypted+masked PII | none explicit (rollback+rethrow → 500) | global | — | | crates/api/src/routes/owner/couriers.rs | flow-ui-courier-full.spec.ts |
| 76 | couriers.ts:79 | PATCH | `.../couriers/:courierId` | OWNER+LOC | body status/role (untyped) | 200 `{success:true}` | 404 NOT_FOUND (membership) | global | — | 🔴 auth (deactivate/suspend revokes `courier_sessions`) | couriers.rs | courier-room-authz-isolation.spec.ts |
| 77 | couriers.ts:147 | GET | `.../couriers/live` | OWNER+LOC | none | 200 `{success,couriers}` live map | none | global | — | | couriers.rs | dashboard-courier-pins.spec.ts |
| 78 | couriers.ts:205 | GET | `.../orders/:orderId/route` | OWNER+LOC | params | 200 `{orderId,courierId,points}` | 404 | global | — | | couriers.rs | needs-new-E2E |
| 79 | couriers.ts:251 | GET | `.../couriers/:courierId/details` | OWNER+LOC | params | 200 `{shifts,earnings,history}` | none | global | — | | couriers.rs | flow-ui-courier-full.spec.ts |

**Subtotal couriers.ts = 5** (expected 5 ✓)

---

## routes/owner/menu-availability.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 80 | menu-availability.ts:22 | PATCH | `/api/owner/locations/:locationId/kitchen-busy` | OWNER+LOC | body `{busy_until: iso\|null}` `.strict()` | 200 `{id,kitchenBusyUntil}` | 404 | global | — | | crates/api/src/routes/owner/menu_availability.rs | ui-improvements.spec.ts |
| 81 | menu-availability.ts:76 | GET | `.../menu-schedules` | OWNER+LOC | params | 200 `{data:[]}` | none | global | — | | menu_availability.rs | needs-new-E2E |
| 82 | menu-availability.ts:95 | POST | `.../menu-schedules` | OWNER+LOC | body product_id XOR category_id + mode/window `.strict()` | 201 schedule | 400 (must pick exactly one target), 404 (FK-ownership fold — explicit R2-1/15th-IDOR fix in comments) | global | — | 🔴 RLS/tenant-scoping (documented prior IDOR) | menu_availability.rs | needs-new-E2E |
| 83 | menu-availability.ts:142 | DELETE | `.../menu-schedules/:id` | OWNER+LOC | params | 204 | 404 | global | — | | menu_availability.rs | needs-new-E2E |

**Subtotal menu-availability.ts = 4** (expected 4 ✓)

---

## routes/owner/themes.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 84 | themes.ts:17 | GET | `/api/owner/locations/:locationId/theme` | OWNER+LOC | params | 200 `{theme,cssHash,version}` | 404 | global | — | | crates/api/src/routes/owner/themes.rs | flow-ui-admin-branding.spec.ts |
| 85 | themes.ts:46 | PUT | `.../theme` | OWNER+LOC | body colors/font_family(enum ALLOWED_FONTS)/frame_ancestors `.strict()` | 200 `{cssHash,version,warnings}` | none explicit (rollback+rethrow) | global | `frame_ancestors` feeds CSP — no validation on origin format shown | | themes.rs | flow-ui-admin-branding.spec.ts |
| 86 | themes.ts:119 | POST | `.../theme/logo` | OWNER+LOC | multipart file | 200 `{logo_url}` | 400 VALIDATION_FAILED (no file) | global | — | | themes.rs | ux1-storefront-links.spec.ts |

**Subtotal themes.ts = 3** (expected 3 ✓)

---

## routes/owner/push.ts — prefix `/api/owner/locations` (routes.ts:136)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 87 | push.ts:23 | POST | `/api/owner/locations/:locationId/push/subscribe` | OWNER+LOC | body subscription `.strict()` | 200 `{ok:true}` | none | 10/min | — | | crates/api/src/routes/owner/push.rs | flow-core-lifecycles.spec.ts |
| 88 | push.ts:66 | POST | `.../push/unsubscribe` | OWNER+LOC | params | 200 `{ok:true}` | none | global | — | | push.rs | needs-new-E2E |
| 89 | push.ts:81 | GET | `.../push/state` | OWNER+LOC | params | 200 `{subscribed,status?,lastError?}` | none | global | — | | push.rs | needs-new-E2E |

**Subtotal push.ts = 3** (expected 3 ✓)

---

## routes/owner/menu-import.ts — prefix `/api/owner` (routes.ts:124) · mixed auth (one PUBLIC route) · 🔴 bulk-edit on commit

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 90 | menu-import.ts:24 | POST | `/api/owner/menu/import/preview` | OWNER (no LOC — `getLocationId()` DB helper) | multipart file (≤10MB) + mode/config | 200 `{import_session_id,summary,issues,draft_preview}` | 401, 400 (missing file / bad mode / bad config JSON), 413, 404 (location), 400 (unsupported source) | 5/min | — | | crates/api/src/routes/owner/menu_import.rs | menu-first-onboarding.spec.ts |
| 91 | menu-import.ts:173 | POST | `/api/owner/menu/import/anonymous` | **PUBLIC — no auth at all** (pre-account onboarding front door; IP + 5/min rate-limited only) | multipart file (≤10MB) | 200 `{anonymous_import_id,summary,draft_preview}` | 503 (no redis), 400 (missing/bad type), 413 | 5/min | requires `fastify.redis` (503 if absent) | 🔴 unauthenticated file-parsing surface (AI-OCR) — DoS/abuse vector | menu_import.rs | groq-import-proof.spec.ts |
| 92 | menu-import.ts:231 | POST | `/api/owner/menu/import/commit` | OWNER (no LOC — `getLocationId()`) | body import_session_id/commit_token/force/website `.strict()` | 200 `{menu_version,counts,branding_generated}` | 401, 404, 409 (×2: already-committed-mismatch, FK/dup-key), 410 EXPIRED, 422 LOW_CONFIDENCE_REQUIRES_FORCE, 400, 500 (missing column) | global | `mode==='replace'` deletes categories/products not in the draft (blocked 409 if historical orders exist) | 🔴 bulk-edit (mode=replace mass-deletes menu rows) | menu_import.rs | flow-onboarding-parsing.spec.ts |

**Subtotal menu-import.ts = 3** (expected 3 ✓)

---

## routes/owner/fallback.ts — prefix `/api/owner/locations` (routes.ts:138)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 93 | fallback.ts:21 | GET | `/api/owner/locations/:locationId/settings/fallback` | OWNER+LOC | params | 200 fallback config | 404 | global | — | | crates/api/src/routes/owner/fallback.rs | flow-offline-phone-fallback.spec.ts |
| 94 | fallback.ts:43 | PUT | `.../settings/fallback` | OWNER+LOC | body phone/showPhoneOnError/showPhoneOnOffline/wsRetry* `.strict()` | 200 `{success,config}` | 404 | global | — | | fallback.rs | flow-offline-phone-fallback.spec.ts |
| 95 | fallback.ts:68 | GET | `.../degradation` | OWNER+LOC | params | 200 `{deadChannels,channels,…}` | 404 | global | — | | fallback.rs | flow-core-lifecycles.spec.ts |

**Subtotal fallback.ts = 3** (expected 3 ✓)

---

## routes/owner/courier-invites.ts — self-prefixed · 🔴 mints access credentials

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 96 | courier-invites.ts:27 | POST | `/api/owner/locations/:locationId/courier-invites` | OWNER+LOC (F4-fixed: `requireRole` was missing) | body role/email/ttl_hours (hand-checked, not Zod) | 200 `{inviteId,code,deepLink,expiresAt}` (code returned once, argon2-hashed at rest) | 400 VALIDATION_FAILED, 400 INVALID_ROLE (role allow-listed to `'courier'` only — can't mint an owner) | global | — | 🔴 auth (invite = latent credential) | crates/api/src/routes/owner/courier_invites.rs | flow-ui-courier-invite.spec.ts |
| 97 | courier-invites.ts:87 | GET | `.../courier-invites` | OWNER+LOC | params | 200 `{invites:[]}` | none | global | — | | courier_invites.rs | flow-courier-deep.spec.ts |
| 98 | courier-invites.ts:104 | DELETE | `.../courier-invites/:inviteId` | OWNER+LOC | params | 200 `{success:true}` (always 200 even if no row matched — no 404) | none | global | — | | courier_invites.rs | flow-ui-invite-onboarding.spec.ts |

**Subtotal courier-invites.ts = 3** (expected 3 ✓)

---

## routes/owner/alerts.ts — prefix `/api/owner/locations` (routes.ts:133)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 99 | alerts.ts:16 | GET | `/api/owner/locations/:locationId/alerts` | OWNER+LOC | querystring status/kind/limit/cursor | 200 `{alerts,nextCursor}` masked PII | none | global | — | | crates/api/src/routes/owner/alerts.rs | flow-admin-deep.spec.ts |
| 100 | alerts.ts:106 | POST | `.../alerts/:alertId/acknowledge` | OWNER+LOC | params | 200 | 404 (via `reply.status(404).send`, inconsistent envelope) | global | — | | alerts.rs | flow-regulatory-settlements.spec.ts |
| 101 | alerts.ts:151 | POST | `.../alerts/acknowledge-all` | OWNER+LOC | body kind? | 200 `{acknowledged:n}` | none | global | — | 🔴 bulk-edit (unbounded bulk UPDATE across all active alerts) | alerts.rs | real-notifications.spec.ts |

**Subtotal alerts.ts = 3** (expected 3 ✓)

---

## routes/owner/activation.ts — prefix `/api/owner` (routes.ts:143)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 102 | activation.ts:58 | GET | `/api/owner/activation/:locationId/status` | requireLocationAccess (per-route) | params | 200 gate object (menuConfirmed/notificationsConnected/fulfillmentReady/missing[]) | 404 | global | — | | crates/api/src/routes/owner/activation.rs | flow-onboarding-auth.spec.ts (401-without-token assertion) |
| 103 | activation.ts:72 | POST | `.../activation/:locationId/pickup` | requireLocationAccess | body `{enabled:boolean}` | 200 gate object | 404 | global | — | | activation.rs | needs-new-E2E |
| 104 | activation.ts:89 | POST | `.../activation/:locationId/publish` | requireLocationAccess | params | 200 `{published,slug,url}` | 404, 422 NOT_READY_TO_PUBLISH, 500 | global | — | 🔴 draft→live transition (flips order-acceptance capability site-wide) | activation.rs | flow-onboarding-auth.spec.ts |

**Subtotal activation.ts = 3** (expected 3 ✓)

---

## routes/owner/refunds.ts — registered directly in **server.ts** with `prefix: '/api/owner'` (server.ts:822) — ⚠️ see cross-cutting note #1 · 🔴 MONEY on both rows

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 105 | refunds.ts:17 | GET | `/api/owner/:locationId/refunds` (⚠️ missing `locations/` segment vs. every sibling route) | OWNER+LOC | params | 200 `{refunds:[]}` | none (dark: empty array when flag off) | global | `PAYMENTS_PREPAID_ENABLED` (dark otherwise) | 🔴 money (crypto refund ledger) | crates/api/src/routes/owner/refunds.rs | needs-new-E2E (dark feature, no FE wired yet per crypto-payments-build memory) |
| 106 | refunds.ts:43 | POST | `/api/owner/:locationId/refunds/:paymentId/sent` | OWNER+LOC | body txRef? | 200 `{ok,orderId}` | 404 (flag off → 404; payment not found → 404) | global | `PAYMENTS_PREPAID_ENABLED` | 🔴 money (marks payment refunded — idempotent via unique `(provider,provider_payment_id,type)`) | refunds.rs | needs-new-E2E |

**Subtotal refunds.ts = 2** (expected 2 ✓)

---

## routes/owner/dwell-settings.ts — prefix `/api/owner/locations` (routes.ts:134)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 107 | dwell-settings.ts:22 | GET | `/api/owner/locations/:locationId/settings/dwell` | OWNER+LOC | params | 200 `{dwellThresholds}` | 404 | global | — | | crates/api/src/routes/owner/dwell_settings.rs | flow-core-lifecycles.spec.ts |
| 108 | dwell-settings.ts:41 | PUT | `.../settings/dwell` | OWNER+LOC | body dwellThresholds (nested bounds 10-3600/7200s) | 200 `{dwellThresholds}` | 404 | global | — | | dwell_settings.rs | flow-core-lifecycles.spec.ts |

**Subtotal dwell-settings.ts = 2** (expected 2 ✓)

---

## routes/owner/reveal-contact.ts — prefix `/api/owner/locations` (routes.ts:139) · 🔴 PII exposure

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 109 | reveal-contact.ts:15 | POST | `/api/owner/locations/:locationId/orders/:orderId/reveal-customer-contact` | OWNER+LOC | body reason? `.strict()` | 200 `{orderId,customerId,name,phone}` (plaintext PII returned + audited) | 404 (order), 404 (no customer on order) | 10/min | — | 🔴 PII (deliberate plaintext reveal, audited in `customer_contact_reveals`) | crates/api/src/routes/owner/reveal_contact.rs | needs-new-E2E (FE caller: apps/web/src/pages/admin/CRMPage.tsx) |

**Subtotal reveal-contact.ts = 1** (expected 1 ✓)

---

## routes/owner/order-meta.ts — prefix `/api/owner/locations` (routes.ts:137)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 110 | order-meta.ts:13 | PATCH | `/api/owner/locations/:locationId/orders/:orderId/metadata` | OWNER+LOC | body `{test_order?:boolean}` `.strict()` | 200 `{ok:true}` | 404 | global | — | | crates/api/src/routes/owner/order_meta.rs | needs-new-E2E (FE caller: apps/web/src/pages/admin/ActivationPage.tsx) |

**Subtotal order-meta.ts = 1** (expected 1 ✓)

---

## routes/owner/menu-translate.ts — prefix `/api/owner` (routes.ts:125)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 111 | menu-translate.ts:10 | POST | `/api/owner/locations/:id/menu/translate` | OWNER+LOC (preHandler) | body target_locales/force/entity_filter `.strict()` | 200 `{translated,skipped_due_to_manual,degraded}` | 404 (location), 400 (unsupported locale, no targets), 500 INTERNAL (catch) | **1/minute per location** (tightest limit in the census) | `degraded` flag surfaces in response when the translation provider itself degrades | | crates/api/src/routes/owner/menu_translate.rs | needs-new-Rust-unit — **no FE caller found** (grep of apps/web/src for the route hit nothing; likely unwired/dead from the owner UI) |

**Subtotal menu-translate.ts = 1** (expected 1 ✓)

---

## routes/owner/menu-confirm.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 112 | menu-confirm.ts:10 | POST | `/api/owner/locations/:locationId/products/:productId/confirm-allergens` | OWNER+LOC | params only | 200 `{confirmed:true}` (via `reply.code(200)`, not the `sendError`/plain-object convention used elsewhere) | 404 `{error:'PRODUCT_NOT_FOUND'}` (via `reply.code(404).send`, inconsistent envelope shape vs. `sendError(404,'NOT_FOUND',…)` used everywhere else) | global | flips `allergens_confirmed` only — never touches `source` (preserves the AI-vs-owner provenance flag gating the C2 read-gate) | 🔴 food-safety/liability gate (drives what allergen data reaches the public storefront) | crates/api/src/routes/owner/menu_confirm.rs | needs-new-Rust-unit — **no FE caller found** (grep for `confirm-allergens`/`allergens_confirmed` in apps/web/src returned nothing — likely unwired from MenuManagerPage/CRM despite ADR-0014 shipping the data model) |

**Subtotal menu-confirm.ts = 1** (expected 1 ✓)

---

## routes/owner/locations.ts — self-prefixed

| # | File:Line | Method | Full path | Auth guard | Req schema | Response | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 113 | locations.ts:9 | PATCH | `/api/owner/locations/:locationId` | OWNER+LOC | body (locale/name/phone/currency/delivery_fee_flat/min_order_value/free_delivery_threshold/delivery_radius_km/tax_rate/lat/lng/delivery_address) `.strict()` | 200 location row | 400 VALIDATION_FAILED (empty body / default_locale∉supported_locales), 404 | global | — | 🔴 money-adjacent (`tax_rate`, `delivery_fee_flat`, `free_delivery_threshold` are pricing inputs consumed by the order-total calc — not itself a transaction, but a direct upstream of one) | crates/api/src/routes/owner/locations.rs | ui-improvements.spec.ts |

**Subtotal locations.ts = 1** (expected 1 ✓)

---

# TOTAL: 113 rows (expected 113, delta 0)

- 🔴 red-line rows: **32** — settlements.ts (7), dashboard.ts (5 of 7: confirm/reject/assign-courier/pickup/deliver), signals.ts mark-no-show (1), onboarding.ts start (1), gdpr.ts (all 5), couriers.ts PATCH (1), menu-availability.ts POST menu-schedules (1), menu-import.ts anonymous+commit (2), courier-invites.ts POST (1), alerts.ts acknowledge-all (1), activation.ts publish (1), refunds.ts (2), reveal-contact.ts (1), menu-confirm.ts (1), locations.ts PATCH (1). *(A handful — order-meta.ts, menu-availability GET/DELETE — were judged non-red on a strict reading of the four categories; flagged inline where the call is close.)*
- `needs-new-E2E` / `needs-new-Rust-unit` rows: **32** — concentrated in product-media.ts (5/5), gdpr.ts retention (2/5), refunds.ts (2/2), reveal-contact.ts (1/1), order-meta.ts (1/1), menu-translate.ts (1/1, likely dead), menu-confirm.ts (1/1, likely dead), menu-availability.ts schedules (3/4), plus scattered singles (products translations ×2, signals ack/dismiss ×2, couriers route ×1, push unsubscribe/state ×2, activation pickup ×1).

---

## Cross-cutting notes

1. **Routing inconsistency (porting hazard, HIGH):** `refunds.ts` is the only owner file registered with `prefix: '/api/owner'` while using an in-file relative path of `/:locationId/refunds`. Every other location-scoped owner route (settlements, dashboard, alerts, dwell-settings, signals, push, order-meta, fallback, reveal-contact, gdpr, and the self-prefixed files) resolves to `/api/owner/locations/:locationId/...`. Refunds therefore actually mounts at **`/api/owner/:locationId/refunds`** — missing the `locations/` segment. This is either a live bug (FE would 404 if it ever calls the "correct" path) or the FE happens to call the actual (inconsistent) path — either way, `utoipa`'s OpenAPI 3.1 SSOT will surface this immediately once schemas are declared; decide during the Rust port whether to fix the URL (breaking) or preserve it verbatim (documented wart). No FE caller was found for either refunds route, consistent with the "dark behind `PAYMENTS_PREPAID_ENABLED`" status in memory — low blast radius to fix now, before anything depends on the wrong shape.

2. **Three different auth-guard idioms do the identical thing.** `addHook('onRequest', fastify.verifyAuth)` (dashboard/settlements/signals/gdpr/notifications/themes/alerts/push/order-meta/fallback/reveal-contact/dwell-settings/refunds), `preValidation:[server.verifyAuth,…]` (products/categories/modifier-groups/locations/menu-confirm/promotions), and `preHandler:[fastify.verifyAuth,…]` (product-media/menu-import/menu-translate/onboarding/activation) are functionally the same chain but stylistically forked across the codebase — collapse to ONE axum middleware/extractor stack (`crates/api/src/extractors/auth.rs` + `tenant.rs`) rather than porting the three idioms as three patterns. Two files (`couriers.ts`, `courier-invites.ts`) have inline comments (F3, F4) documenting that `requireRole(['owner'])` was **missing** in a past version and got patched in — treat every owner route's role check as a red-line invariant to re-verify explicitly in Rust tests, not just port-and-trust.

3. **Two distinct "location resolution" strategies coexist and must not be conflated during the port:** (a) URL-param `:locationId` + `requireLocationAccess` middleware (majority of routes — the middleware checks the caller is an active owner member of that exact `locationId`), vs. (b) JWT-derived "the owner's one location" via `getOwnerLocationId`/`getLocationId`/`getOwnerLocation` helpers with **no locationId in the URL at all** (promotions.ts, menu-import.ts, product-media.ts, and the `/api/owner/menu/*` alias routes in products.ts/categories.ts). Both re-verify the JWT's `activeLocationId` against a **live** `memberships` row (ADR-0004 P-d pattern: "a removed/downgraded owner holds a valid ≤24h token") rather than trusting the JWT claim — this DB-recheck-on-every-request pattern is the actual security boundary and must be preserved 1:1 in the axum extractor, not simplified away as "just decode the JWT."

4. **Error envelope is NOT uniform** — most handlers use `reply.sendError(code, 'SNAKE_CASE', 'message')`, but at least 4 routes (promotions DELETE, alerts acknowledge, menu-confirm both branches) use bare `reply.status(n).send({error:...})` or `reply.code(n).send(...)` with a different JSON shape. `utoipa`'s single response-schema-per-status-code model will force a decision here — recommend normalizing to one envelope during the Rust port rather than replicating the inconsistency, and flag it as an inline-fix candidate (cosmetic/contract-shape, not business logic) per the Task-Exit Rule.

5. **`withTenant()` (Postgres `SET app.user_id` → RLS) is the load-bearing tenant boundary**, not the JS-level `location_id = $N` WHERE clauses layered on top — those are defense-in-depth. In `sqlx`, this maps to: every pooled connection used for an owner-scoped query must run `SELECT set_config('app.user_id', $1, true)` (or the transaction-scoped equivalent) before the real query, inside the same logical transaction/connection checkout — this is the single riskiest thing to get wrong in the Rust port (a pooled connection that leaks `app.user_id` across requests, or a query issued on a fresh connection that skips the `set_config`, silently reopens every RLS hole this codebase spent multiple audit rounds closing). Several routes (couriers.ts, product-media.ts confirm-bytes path) also call `SELECT set_config('app.current_tenant', $1, true)` — a **second**, apparently legacy/parallel tenant-context variable — confirm during the port whether `app.user_id` and `app.current_tenant` are both still read by live RLS policies or whether one is dead weight.

6. **Two dead-looking routes found by cross-referencing the FE:** `menu-translate.ts` (POST `/locations/:id/menu/translate`) and `menu-confirm.ts` (POST `.../confirm-allergens`) have **no caller anywhere in `apps/web/src`** (verified by grep) despite both being fully-built, non-trivial, DB-writing endpoints — `menu-confirm.ts` in particular is described in its own header comment as safety-critical (allergen liability). Worth a quick human check before porting: either they're called from a surface I didn't grep (e.g., a script, an internal admin-only page not under `pages/admin/`) or they're genuinely orphaned and the Rust port is a good moment to either wire them up or intentionally drop them (flag as **escalate**, not inline-fix, per Task-Exit Rule — this touches a safety/liability field).

7. **Rate limits are inconsistent in a way that maps to real risk, mostly correctly** — `menu-translate.ts` (1/min, LLM-cost-bounding), `onboarding.ts:start` (3/min, tenant-creation), `menu-import.ts` preview/anonymous (5/min, AI-OCR cost + the anonymous route is fully unauthenticated), `settlements.ts regenerate` (5/5min, triggers a global cron-worker sweep) are all tightly bounded; but several equally sensitive routes have **no rate limit at all** (global only): `gdpr.ts` GET/PUT retention, all of `couriers.ts` PATCH (session revocation), `courier-invites.ts` POST (mints credentials), `menu-availability.ts` POST schedules. Not necessarily wrong (global limits may suffice) but worth an explicit per-route rate-limit decision table when porting to axum (e.g. `tower-governor` per-route layers) rather than assuming "global" is a deliberate choice everywhere it appears.

8. **`sqlx` compile-time query checking will surface every dynamic-SQL route immediately** — `products.ts`, `categories.ts`, `modifier-groups.ts`, and `locations.ts` all build `UPDATE ... SET ${dynamicClauses}` strings from `Object.entries(updates)` (guarded by `/* eslint-disable local/no-raw-sql */`). These cannot be ported as `sqlx::query!` macros verbatim (no compile-time-checkable dynamic column list) — plan for either `sqlx::QueryBuilder` (runtime-built, still parameterized) or an explicit enum-of-updatable-fields per entity. This is pure mechanical translation work, not a correctness risk (params are already positionally bound, no injection), but it's the single most common pattern-mismatch between the Fastify/pg style and idiomatic sqlx and will touch ~15 rows.

9. **`utoipa` OpenAPI SSOT will need one schema per Zod object** — most routes already have Zod (portable to `utoipa`/`serde` derive macros 1:1), but `dashboard.ts` snapshot (hand-parsed querystring, no Zod at all), `couriers.ts` (fully untyped `request.body as any` throughout), and `alerts.ts`/`signals.ts` (Zod only on some routes) will need net-new schema authoring during the port, not just translation — budget extra time for these three files specifically.


---

<!-- ============ §1.2 HTTP ROUTES — COURIER + CUSTOMER + CORE (51) ============ -->

# COURIER + CUSTOMER + CORE route census — Rust rebuild map

**Extraction command:**
```
cd /root/dowiz/apps/api && grep -rnE "^\s*(fastify|app|server|f|instance)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\."
```
Full command output: 236 registrations across the whole `apps/api/src` tree (owner/public/admin/dev/webhook routes excluded from this partition — they belong to other census parts).

**Expected total (this partition):** 51
**Actual count (this partition):** 51 — **MATCH, delta = 0**

Prefix resolution source: `/root/dowiz/apps/api/src/bootstrap/routes.ts` (`registerCoreRoutes`), cross-checked against `apps/api/src/server.ts:525` (`await registerCoreRoutes(...)`). All 5 example prefixes in the task spec verified correct against the live file:
- `courierAuthRoutes` → `{ prefix: '/api/courier/auth', db: pool }` (line 126)
- `courierMeRoutes` → `{ prefix: '/api/courier', db: pool }` (line 127)
- `customerOrderRoutes` → `{ prefix: '/api/customer', db: pool, messageBus }` (line 130)
- `orderRoutes` → `{ prefix: '/api', db: pool, messageBus, queue }` (line 96)
- `authRoutes` → `{ prefix: '/api' }` (line 90); `localAuthRoutes` → `{ prefix: '/api', db: pool }` (line 94, dynamic import)
- Additionally verified: `courierAssignmentsRoutes`/`courierShiftsRoutes`/`courierSettlementRoutes` → all `{ prefix: '/api/courier', ... }`; `customerOtpRoutes`/`customerTrackRoutes`/`customerPushRoutes` → all `{ prefix: '/api/customer', ... }`; `orderMessageRoutes` → **no prefix option** (self-embeds `/api/orders/...` in its own path strings); `courierRoutes` (routes/couriers.ts) → **no prefix option** (registered bare at line 95, `fastify.register(courierRoutes)`) — its one route literally is `/couriers/invites`, NOT under `/api`.

Auth guard decorators (shared across all routes below): `fastify.decorate('verifyAuth', ...)`, `fastify.decorate('softVerifyAuth', ...)`, `fastify.decorate('requireRole', ...)` — all registered in `apps/api/src/plugins/auth.ts:163-165`. Rust target for these: `crates/api/src/extractors/auth.rs`. Tenant/membership joins (ADR-0004 live-membership re-check pattern) map to `crates/api/src/extractors/tenant.rs`.

---

## COURIER (27 routes, 5 files)

### `routes/courier/assignments.ts` — 9 routes (subtotal 9)
File-level hooks (line 18-19): `preValidation: [verifyAuth, requireRole(['courier'])]` on every route in this file.

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | assignments.ts:74 | GET | `/api/courier/me/assignments` | courier JWT (file-hook) | none | `{success, assignments: Task[]}` (assigned/accepted/picked_up only) | 500 (unhandled→global) | global (100/min) | — | — | `crates/api/src/routes/courier/assignments.rs` | tests/courier/full-coverage.spec.ts, tests/flow-ui-courier-core.spec.ts |
| 2 | assignments.ts:102 | GET | `/api/courier/assignments/:id` | courier JWT (file-hook) | params `{id: uuid}` | single `Task` shape, scoped `WHERE courier_id=$2` | 404 NOT_FOUND | global | — | — | same | tests/courier/full-coverage.spec.ts |
| 3 | assignments.ts:125 | POST | `/api/courier/assignments/:id/accept` | courier JWT (file-hook) | params `{id: uuid}` | `{success:true}` | 500 (service throws on invalid) | global | `COURIER_OFFER_HANDSHAKE_ENABLED` (offered-path branch) | 🔴 STATE (offered/assigned→accepted; order→CONFIRMED/IN_DELIVERY) | same | tests/courier/full-coverage.spec.ts, tests/courier/offer-timer.spec.ts |
| 4 | assignments.ts:178 | POST | `/api/courier/assignments/:id/reject` | courier JWT (file-hook) | params `{id: uuid}` | `{success:true}` | 404 ASSIGNMENT_NOT_FOUND_OR_NOT_ASSIGNED | global | — | 🔴 STATE (assigned→rejected; re-enqueue dispatch) | same | tests/courier/full-coverage.spec.ts |
| 5 | assignments.ts:239 | POST | `/api/courier/assignments/:id/picked-up` | courier JWT (file-hook) | params `{id: uuid}` | `{success:true}` | 404 ASSIGNMENT_NOT_FOUND_OR_NOT_ACCEPTED | global | — | 🔴 STATE (accepted→picked_up; order→IN_DELIVERY) | same | tests/courier/full-coverage.spec.ts, tests/flow-sensor-delivery-baseline.spec.ts |
| 6 | assignments.ts:292 | POST | `/api/courier/assignments/:id/delivered` | courier JWT (file-hook) | body `{payment_outcome?, cash_collected?, cash_amount?}` strict | `{success:true}` | 404 ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP, 409 PREPAID_NOT_PAID, 422 CASH_AMOUNT_MISMATCH | global | crypto-prepaid branch gated `payment_method==='crypto'` | 🔴 STATE + MONEY (picked_up→delivered/cancelled; cash-as-proof HOLD via `completeDelivery`) | same | tests/deliver-v2-cancel-revert.spec.ts, tests/ux4-tips.spec.ts, tests/flow-core-lifecycles.spec.ts |
| 7 | assignments.ts:413 | POST | `/api/courier/assignments/:id/cancel` | courier JWT (file-hook) | params `{id}`, body `{reason: string}` strict | `{success:true, requeued}` | 404 ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS, 410 CANCEL_WINDOW_EXPIRED | global | `CANCEL_AFTER_DISPATCH_WINDOW_MS` env (5min default) | 🔴 STATE (accepted/picked_up→terminal via `releaseBindingAndReoffer`; order revert) | same | tests/deliver-v2-cancel-revert.spec.ts |
| 8 | assignments.ts:482 | POST | `/api/courier/assignments/:id/abort` | courier JWT (file-hook) | params `{id}`, body `{reason?}` strict | `{success:true, requeued}` | 404 ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS | global | none (no time gate, unlike /cancel) | 🔴 STATE (accepted/picked_up→terminal, en-route no-time-gate exit) | same | **needs-new-E2E** (only `decline` proven in offer-timer.spec.ts, not `abort`) |
| 9 | assignments.ts:535 | POST | `/api/courier/assignments/:id/decline` | courier JWT (file-hook) | params `{id: uuid}` | `{success:true}` | 404 ASSIGNMENT_NOT_FOUND_OR_NOT_OFFERED | global | offer-handshake path only | 🔴 STATE (offered→offered_expired; re-enqueue; customer order untouched) | same | tests/courier/offer-timer.spec.ts (§6c decline test) |

**Hazard note — courier/shifts.ts is skipped here; assignments.ts hazards:** every mutating route hand-rolls its own `BEGIN`/`SELECT set_config('app.current_tenant',...)`/`FOR UPDATE`/`COMMIT`/`ROLLBACK` block with no shared transaction helper — 7 near-identical copies of the same tenant-scoping + row-lock ritual. In Rust this is exactly the shape a `with_tenant_tx` extractor/helper should collapse; porting each handler literally would reproduce the duplication. `/accept` (line 125-175) has a **dual code path** (offered-handshake vs legacy pre-handshake `acceptCourierAssignment` service) gated by row content, not a flag check — both paths must be preserved or explicitly deprecated in the rebuild, not silently dropped.

### `routes/courier/me.ts` — 6 routes (subtotal 15)
File-level hooks (line 32-33): `preHandler: [verifyAuth, requireRole(['courier'])]`.

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | me.ts:36 | GET | `/api/courier/me` | courier JWT (file-hook) | none | profile w/ `decryptPII`'d name + masked email/phone | 404 NOT_FOUND | global | — | — (PII decrypt/mask — not state-machine but privacy-sensitive) | `crates/api/src/routes/courier/me.rs` | tests/flow-ui-courier-core.spec.ts, tests/courier-room-authz-isolation.spec.ts |
| 11 | me.ts:76 | PATCH | `/api/courier/me/messenger` | courier JWT (file-hook) | body `{messenger_kind?: enum, messenger_handle?}` strict | `{messenger_kind, messenger_handle}` | none explicit (relies on Zod 400 via schema validation) | global | — | — | same | **needs-new-E2E** |
| 12 | me.ts:94 | GET | `/api/courier/me/audit-log` | courier JWT (file-hook) | none | `{logs: [...]}` last 50 | — | global | — | — | same | tests/flow-core-lifecycles.spec.ts |
| 13 | me.ts:110 | PATCH | `/api/courier/me/password` | courier JWT (file-hook) | manual Zod: `{current_password, new_password (min 12)}` strict | `{success:true}`; revokes all `courier_sessions` | 400 VALIDATION_FAILED, 404 NOT_FOUND, 400 (invalid current password) | global | — | 🔴 AUTH (password change + full session revoke) | same | tests/flow-core-lifecycles.spec.ts |
| 14 | me.ts:177 | GET | `/api/courier/me/earnings` | courier JWT (file-hook) | none | today/week/month cash+tips summary + last 20 payouts | — | global | — | 🔴 MONEY (earnings/payout figures) | same | tests/courier/full-coverage.spec.ts, tests/flow-core-lifecycles.spec.ts (12 spec files touch this) |
| 15 | me.ts:249 | GET | `/api/courier/me/history` | courier JWT (file-hook) | none | delivery history, masked customer name (`maskStr`), last 50 | — | global | — | — | same | tests/flow-security-contracts.spec.ts, tests/flow-core-lifecycles.spec.ts |

### `routes/courier/shifts.ts` — 5 routes (subtotal 20) — ⚠ WORST HEALTH SCORE FILE (1.0/10 per CLAUDE.md biomarker)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 16 | shifts.ts:15 | GET | `/api/courier/me/shift` | `preValidation:[verifyAuth, requireRole(['courier'])]` (per-route, not file-hook) | none | `{isActive, startedAt, elapsedSeconds, shiftId, status, stats}` | — | global | — | — | `crates/api/src/routes/courier/shifts.rs` | tests/flow-ui-courier-full.spec.ts |
| 17 | shifts.ts:60 | POST | `/api/courier/me/shift/start` | same pattern | body `{lat?, lng?}` (soft-parsed, non-strict) | `{success, status, shiftId, startedAt}` | — | global | P0-1: no GPS write at shift-open (privacy) | 🔴 STATE (shift offline→available via `openShift`) | same | tests/flow-core-lifecycles.spec.ts, tests/courier-room-authz-isolation.spec.ts |
| 18 | shifts.ts:111 | POST | `/api/courier/me/shift/end` | same pattern | none | `{success, status:'offline'}` | 409 ACTIVE_DELIVERY_EXISTS | global | — | 🔴 STATE (available/on_delivery→offline; blocked if active delivery) | same | tests/flow-core-lifecycles.spec.ts |
| 19 | shifts.ts:173 | POST | `/api/courier/shifts/transition` | same pattern | body `{to: enum['offline','available'], lat?, lng?}` strict | `{success, status, shiftId}` | 400 (Zod), 409 CANNOT_GO_OFFLINE_WITH_ACTIVE_ORDER, 409 ACTIVE_DELIVERY_EXISTS, 400 GPS_REQUIRED, 409 INVALID_TRANSITION | global | — | 🔴 STATE (generalized transition endpoint — supersedes #17/#18 semantics) | same | tests/flow-core-lifecycles.spec.ts |
| 20 | shifts.ts:305 | POST | `/api/courier/shifts/ping` | same pattern | body `{lat, lng, accuracy_meters?}` strict | `{success, gps_stored, reason?}` | 400 (Zod), 400 GPS_OUT_OF_RANGE, 409 NO_ACTIVE_SHIFT | **per-route: 1 req / 10s keyed by Authorization header** (not IP — explicit carrier-NAT fix) | P0-1 hard gate: GPS stored only while on active delivery | 🔴 STATE-ADJACENT (writes `courier_positions` + fires `courier_geofence_enter` sensor event; WS fan-out `COURIER_POSITION_UPDATED`) | same | tests/flow-sensor-geofence.spec.ts, tests/flow-core-lifecycles.spec.ts, tests/dashboard-courier-pins.spec.ts |

**Hazard note (shifts.ts, worst-health file):** three overlapping ways to reach the same state exist simultaneously — `/me/shift/start`+`/me/shift/end` (convenience routes, line 60/111) AND the generalized `/shifts/transition` (line 173) both drive the identical `courier_shifts.status` column with **duplicated** business rules (the "no offline with active delivery" 409 check is copy-pasted in both `/me/shift/end` and `/shifts/transition`'s `to==='offline'` branch). A Rust port must pick ONE canonical transition function (likely mirroring `/shifts/transition`) and make the two convenience routes thin wrappers, or the duplication will drift further. `/shifts/ping` (line 305) is the most complex handler in the file: GPS range-check → active-shift check → conditional position INSERT (privacy gate) → best-effort SAVEPOINT-wrapped geofence sensor event → unconditional heartbeat UPDATE → conditional WS publish — 5 sequential DB round-trips inside one non-batched flow, all in the file scoring 1.0/10.

### `routes/courier/auth.ts` — 5 routes (subtotal 25)
Registered with explicit prefix `/api/courier/auth`; every route here is **pre-auth** (no verifyAuth hook) except where noted.

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 21 | auth.ts:23 | POST | `/api/courier/auth/invites/:inviteId/redeem` | public (invite-code gated) | manual Zod `{email, code, password(min12), full_name, phone?}` | `{jwt, refreshToken, courier:{id, masked_email, full_name, locations}}` | 400 VALIDATION_FAILED, 410 INVITE_INVALID, 401 INVALID_CODE | 5/15min | — | 🔴 AUTH (mints courier JWT + session; argon2 code verify) | `crates/api/src/routes/courier/auth.rs` | tests/courier-room-authz-isolation.spec.ts, tests/flow-core-lifecycles.spec.ts |
| 22 | auth.ts:159 | GET | `/api/courier/auth/invites/:inviteId` | public (no auth) | none | `{id, role, locationName, isValid, isExpired, isUsed, isRevoked}` | 404 NOT_FOUND | global | — | — (two-pass RLS-tenant-discovery pattern — worth noting for Rust RLS parity) | same | tests/flow-ui-invite-onboarding.spec.ts |
| 23 | auth.ts:219 | POST | `/api/courier/auth/login` | public | manual Zod `{email, password, location_id?}` | `{jwt, refreshToken, activeLocationId, role}` | 400, 401 INVALID_CREDENTIALS, 403 COURIER_DEACTIVATED, 403 NOT_AUTHORIZED_FOR_LOCATION, 403 NO_LOCATION_ASSIGNED | 5/15min | — | 🔴 AUTH (argon2 + timing-safe dummy-verify on miss; JWT+session mint) | same | tests/courier/full-coverage.spec.ts, tests/flow-courier-deep.spec.ts |
| 24 | auth.ts:354 | POST | `/api/courier/auth/refresh` | public (bearer of refresh token) | manual Zod `{refresh_token}` strict | `{jwt, refreshToken}` | 401 INVALID_REFRESH_TOKEN, 401 SESSION_NOT_FOUND, 401 REFRESH_REUSED (family-revoke), 401 REFRESH_EXPIRED, 401 COURIER_DEACTIVATED | 10/1min | — | 🔴 AUTH (session rotation + reuse-detection family-revoke — ADR-0004-adjacent for couriers) | same | tests/flow-core-lifecycles.spec.ts |
| 25 | auth.ts:479 | POST | `/api/courier/auth/logout` | public (best-effort token) | manual Zod `{refresh_token}` strict, fails soft | `{success:true}` (always) | none (soft-fails to success) | global | — | 🔴 AUTH (session revoke) | same | tests/flow-core-lifecycles.spec.ts |

**Note:** courier auth has NO live ADR-0004-style per-request membership re-check on refresh (unlike owner `/api/auth/refresh` which re-derives from `memberships` table) — courier refresh only checks `couriers.status !== 'active'`, not per-location membership liveness. Flag for the security council when porting.

### `routes/courier/settlements.ts` — 2 routes (subtotal 27)
File-level hooks (line 8-9): `onRequest: [verifyAuth, requireRole(['courier'])]`.

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 26 | settlements.ts:12 | GET | `/api/courier/me/payouts` | courier JWT (file-hook) | querystring `{status?: enum}` | `{payouts: [...]}` | — | global | — | 🔴 MONEY (payout ledger read) | `crates/api/src/routes/courier/settlements.rs` | tests/flow-regulatory-settlements.spec.ts, tests/flow-courier-deep.spec.ts |
| 27 | settlements.ts:51 | GET | `/api/courier/me/payouts/:id` | courier JWT (file-hook) | params `{id: uuid}` | `{payout, items}` (items strictly exclude orderId/assignmentId/phone) | 404 NOT_FOUND | global | — | 🔴 MONEY (payout detail; deliberately narrow PII surface) | same | tests/flow-regulatory-settlements.spec.ts |

**COURIER subtotal: 27/27 ✓**

---

## CUSTOMER (8 routes, 4 files)

### `routes/customer/orders.ts` — 3 routes (subtotal 3)
File-level hooks (line 18-19): `onRequest: [verifyAuth, requireRole(['customer'])]`.

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 28 | orders.ts:21 | GET | `/api/customer/orders/:orderId/status` | customer JWT (file-hook), scoped `customer_id=$2` | params `{orderId: uuid}` | rich status/tracking payload (route, ETA range, promised window, masked courier contact) | 500 INTERNAL (caught) | global | — | — (read-only, but surfaces order+courier PII — masked) | `crates/api/src/routes/customer/orders.rs` | tests/flow-order-lifecycle-trace.spec.ts, tests/flow-customer-track-link.spec.ts |
| 29 | orders.ts:219 | POST | `/api/customer/orders/:orderId/rating` | customer JWT (file-hook) | params `{orderId}`, body `{rating: int 1-5, feedback?: max1000}` | `{success, rating, feedback}` upsert | 404 NOT_FOUND, 409 NOT_DELIVERED, 409 RATING_WINDOW_CLOSED, 500 | global | 24h edit window (business rule, not a flag) | — | same | **needs-new-E2E** (no direct `orders/.*rating` hit found; broad `customer orders` matches are for status/checkout, not rating) |
| 30 | orders.ts:259 | POST | `/api/customer/orders/:orderId/cancel` | customer JWT (file-hook), scoped `customer_id=$2` | params `{orderId}`, body `{reason: string 5-500}` | `{success:true}` | 403 FORBIDDEN, 409 CANCEL_NOT_ALLOWED_STATUS, 410 CANCEL_WINDOW_EXPIRED, 500/typed-rethrow | global | `CANCEL_AFTER_DISPATCH_WINDOW_MS` (5min default) | 🔴 STATE + MONEY (IN_DELIVERY→CANCELLED via `updateOrderStatus`; records `refund_due` obligation for paid crypto orders) | same | tests/deliver-v2-cancel-revert.spec.ts, tests/flow-order-lifecycle-trace.spec.ts |

### `routes/customer/push.ts` — 2 routes (subtotal 5)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 31 | push.ts:21 | POST | `/api/customer/push/subscribe` | inline check `user.role==='customer'` (no file-hook — manual, not via `requireRole`) | body `{subscription:{endpoint, keys:{p256dh,auth}}, opted_in}` strict | `{ok:true}` upsert `customer_devices` | 401 UNAUTHORIZED | 10/1min | — | — | `crates/api/src/routes/customer/push.rs` | **needs-new-E2E** (only the OWNER push route `/api/owner/locations/:id/push/subscribe` is tested — NOT this customer route) |
| 32 | push.ts:64 | POST | `/api/customer/push/unsubscribe` | inline check `user.role==='customer'` | none | `{ok:true}` | 401 UNAUTHORIZED | 5/1min | — | — | same | **needs-new-E2E** (same gap) |

### `routes/customer/otp.ts` — 2 routes (subtotal 7)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 33 | otp.ts:34 | POST | `/api/customer/locations/:slug/otp/send` | public (pre-order) | strict `{phone: E.164 regex, order_intent:{items[], total, currency}}` | `{otp_token, expires_in_ms}` | 404 NOT_FOUND, 400 OTP_NOT_REQUIRED, 429 OTP_RATE_LIMIT | 3/15min keyed by phone | `OTP_ENABLED` env (globally OFF — CLAUDE.md: "OTP_ENABLED off; integer tax") | 🔴 AUTH-ADJACENT (pre-order phone verification gate) | `crates/api/src/routes/customer/otp.rs` | **needs-new-E2E** (checkout.spec.ts has a TODO(needs_staging) stub only; no live send/verify assertion) |
| 34 | otp.ts:112 | POST | `/api/customer/locations/:slug/otp/verify` | public | strict `{phone, code: 6-digit, otp_token, order_intent_hash}` | `{verified_token, expires_in_ms}` | 404 NOT_FOUND, 400 OTP_NOT_REQUIRED, 410 OTP_EXPIRED, 429 OTP_LOCKOUT, 401 INVALID_TOKEN, 401 (invalid code, non-envelope) | 5/15min keyed by phone | `OTP_ENABLED` (OFF) | 🔴 AUTH-ADJACENT | same | **needs-new-E2E** (same gap) |

### `routes/customer/track.ts` — 1 route (subtotal 8)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 35 | track.ts:28 | POST | `/api/customer/track/exchange` | public/pre-auth (registered in NO_AUTH_PATHS; opaque-code bearer) | strict `{code: string 20-64}` | `{token}` (customer JWT minted via `issueCustomerToken`) | 410 TRACK_LINK_EXPIRED (uniform for unknown/expired/gone), 500 INTERNAL | 10/1min by IP | — | 🔴 AUTH (mints customer JWT from an opaque tracking-link code; runs on operational/BYPASSRLS pool with explicit `WHERE token_hash=$1`) | `crates/api/src/routes/customer/track.rs` | tests/flow-customer-track-link.spec.ts, tests/flow-geo-tracking.spec.ts, tests/flow-sensor-eta-window.spec.ts |

**CUSTOMER subtotal: 8/8 ✓**

---

## CORE (16 routes, 5 files)

### `routes/auth.ts` — 8 routes (subtotal 8) — prefix `/api`

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 36 | auth.ts:34 | GET | `/api/auth/google` | public | none | 302 redirect to Google OAuth (PKCE state in Redis, 10min TTL) | 404 (flag off) | 10/1min | `GOOGLE_OAUTH_ENABLED` | 🔴 AUTH | `crates/api/src/routes/auth.rs` | tests/flow-onboarding-auth.spec.ts, tests/api-real.spec.ts |
| 37 | auth.ts:62 | GET | `/api/auth/google/callback` | public (state-token bound) | query `{code, state}` | 302 redirect w/ one-time opaque code (60s Redis TTL) | 404 (flag off), 400 VALIDATION_FAILED (bad state/nonce/token exchange) | 10/1min | `GOOGLE_OAUTH_ENABLED` | 🔴 AUTH (mints owner access+refresh token pair, upserts `users`) | same | tests/flow-onboarding-auth.spec.ts |
| 38 | auth.ts:173 | POST | `/api/auth/exchange` | public (opaque-code bearer) | strict `{code: uuid}` | `{access_token, refresh_token}` (from Redis one-shot code) | 400 VALIDATION_FAILED | 10/1min | — | 🔴 AUTH | same | tests/flow-onboarding-auth.spec.ts |
| 39 | auth.ts:191 | POST | `/api/auth/telegram/start` | public | none | `{token, botUsername, deepLink}` (5min TTL) | — | 10/5min | — | 🔴 AUTH (pre-login token mint) | same | tests/flow-onboarding-auth.spec.ts, tests/menu-first-onboarding.spec.ts |
| 40 | auth.ts:202 | GET | `/api/auth/telegram/poll` | public (token bearer) | query `{token: uuid}` | `{status}` or `{status:'authenticated', access_token, refresh_token}` | 404 (unknown), 410 (expired/consumed) | 120/5min | — | 🔴 AUTH (single-use atomic authenticated→consumed flip; mints owner tokens) | same | tests/flow-onboarding-auth.spec.ts |
| 41 | auth.ts:235 | POST | `/api/auth/refresh` | public (refresh-token bearer) | strict `{refresh_token, active_location_id?: uuid}` | `{access_token, refresh_token}` | 401 UNAUTHORIZED (invalid/expired/reuse), 401 OWNER_REVOKED, 409 (soft concurrent-refresh) | 5/1min | — | 🔴 AUTH — **ADR-0004 live-membership re-check IS HERE** (line 293-301: re-derives owner authority from `memberships` on every refresh; closes privilege-roll-forward) | same | tests/owner-fixes-batch.spec.ts |
| 42 | auth.ts:325 | POST | `/api/auth/logout` | `preHandler:[verifyAuth]` | none | 204 No Content; deletes ALL `auth_refresh_tokens` for user | 401 UNAUTHORIZED | 10/1min | — | 🔴 AUTH (ADR-0004 P-b: real server-side logout, all-devices) | same | tests/prod-adr0004-smoke.spec.ts, tests/owner-revocation.spec.ts |
| 43 | auth.ts:339 | POST | `/api/auth/courier/activate` | public (invite-code bearer) | strict `{code: min4, phone: min6, name}` | `{access_token, refresh_token}` (courier role, 7d token) | 400 VALIDATION_FAILED (Invalid/Used/Expired code, mapped from `activate_courier()` DB fn errors) | 5/1min | — | 🔴 AUTH (SECURITY DEFINER `activate_courier()` fn — first courier membership can't self-satisfy RLS) | same | **needs-new-E2E** (no direct hit in any spec) |

### `routes/orders.ts` — 3 routes (subtotal 11) — prefix `/api` — ⚠ KNOWN UNTESTED HOTSPOT per CLAUDE.md biomarkers

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 44 | orders.ts:72 | POST | `/api/orders` | public (anonymous storefront checkout) | `CreateOrderInput` (shared-types) | `{id, locationId, status, subtotal, total, ..., authToken?, payment?, trackUrl?, preflight}` | 400 VALIDATION_FAILED, 404 NOT_FOUND, 409 NOT_PUBLISHED/VENUE_CLOSED, 422 (hard_block/PRODUCT_UNAVAILABLE/PRODUCT_NOT_FOUND/MIN_ORDER_NOT_MET/NOT_DELIVERABLE/CASH_AMOUNT_TOO_LOW/IDEMPOTENCY_KEY_REUSED), 429 PHONE_THROTTLE/IP_THROTTLE, 503 SERVICE_UNAVAILABLE, 409 IDEMPOTENCY_CONFLICT, 500 | 5/1min keyed by phone (custom errorResponseBuilder) | `ENFORCE_VENUE_HOURS`, `OTP_ENABLED`, `PAYMENTS_PREPAID_ENABLED`+`PAYMENTS_CRYPTO_ENABLED` (crypto fork) | 🔴 MONEY + STATE (order creation — pricing/tax/idempotency/preflight-signals/OTP/crypto-charge all fused into ONE 650-line handler) | `crates/api/src/routes/core/orders.rs` | tests/flow-order-creation.spec.ts, tests/flow-orders-checkout.spec.ts, tests/flow-simpl-s1-velocity-frictionless.spec.ts |
| 45 | orders.ts:729 | GET | `/api/orders/:id` | `preHandler:[softVerifyAuth]` (anonymous-permitting, but P2-ANONORDER hardened: 401 for unrecognized role) | params `{id}` (manual UUID regex, not Zod) | order + items, shape varies per role (owner: membership-JOIN; courier: `courierReadVerdict`; customer: `orderId`-pinned) | 400 VALIDATION_FAILED (bad UUID), 401 UNAUTHORIZED, 404 NOT_FOUND, 503 SERVICE_UNAVAILABLE (courier verdict UNAVAILABLE, fail-closed), 500 | global | — | 🔴 RLS/AUTHZ (3-way branch: ADR-0004 owner live-membership JOIN / ADR-0013 courier live-binding verdict / customer orderId-pin — the exact multi-tenant read-authz matrix that must port bit-for-bit) | same | tests/flow-order-lifecycle-trace.spec.ts, tests/client/order-stepper.spec.ts |
| 46 | orders.ts:858 | PATCH | `/api/orders/:id/status` | `preHandler:[verifyAuth, requireRole(['owner'])]` | `StatusUpdateInput` (shared-types) | `{id, status, dispatched?, reason?}` | 400 VALIDATION_FAILED, 403 FORBIDDEN, 404 (membership-JOIN miss, logged as possible cross-tenant attempt), 409 ASSIGNMENT_ACTIVE/USE_DELIVER_FLOW (M6/CC-1 guard against DELIVERED/PICKED_UP via PATCH), 500 | global | — | 🔴 STATE — **the owner-driven status-transition endpoint**: `assertOwnerTargetAllowed` gate + "honest dispatch" fork (IN_DELIVERY target on a delivery order tries to find a courier BEFORE advancing; no-op with `dispatched:false` if none free, never silently strands) | same | tests/flow-security-contracts.spec.ts, tests/golive-remediation.spec.ts |

**Hazard note (orders.ts — untested hotspot, per CLAUDE.md "apps/api/src/routes/orders.ts — untested hotspot" biomarker):** `POST /orders` (line 72-726) is a single ~650-line handler that fuses: venue-hours gate, OTP header + server-side OTP verification, preflight risk-signal computation (phone/IP velocity throttles + signal state), idempotency-key replay handling, product/modifier availability + pricing computation, tax computation (inclusive/exclusive), delivery-fee tier resolution, customer upsert, order+items persistence, post-commit MessageBus fan-out (3 publishes), customer JWT issuance, and a crypto-prepaid payment-provider charge fork — all inside one transaction with a hard `SET LOCAL statement_timeout=4500`. Despite being flagged as the worst-tested file with the most business logic surface area of any route in this census, e2e coverage is comparatively broad (12+ spec files touch order creation) but almost entirely **happy-path or single-fault-injection** — there is no evidence of a test matrix crossing venue-closed × OTP-required × idempotency-replay × crypto-fork simultaneously, which is exactly the combinatorial space a Rust rewrite risks silently diverging on. Recommend: this file is the single highest-priority target for a full behavioral characterization test suite BEFORE porting, not after.

### `routes/order-messages.ts` — 3 routes (subtotal 14) — no prefix option (self-embeds `/api/orders/...`)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 47 | order-messages.ts:32 | POST | `/api/orders/:orderId/messages` | `preValidation:[verifyAuth]` (any role; per-role tenant check inline) | `SendMessageRequest` (shared-types) + per-preset `paramsSchema` | `{success, message}` (201) | 400 VALIDATION_FAILED/UNKNOWN_PRESET, 404 NOT_FOUND (uniform for owner-not-member / courier-not-bound / customer-not-owner), 409 (preset-not-allowed-for-status / NO_COURIER_ASSIGNED / NOT_LEAVE_AT_DOOR / NOT_CASH_PAYMENT) | global | preset registry (`PRESET_REGISTRY`) gates which messages are sendable per role+status | 🔴 RLS/AUTHZ (ADR-0013 N3: courier check is **assignment-scoped SEND binding** via `courierCanSendOrder`, not location-wide — the exact WS-adjacent authz tightening from ADR-0013) | `crates/api/src/routes/core/order_messages.rs` | tests/cr8-order-messages.spec.ts, tests/courier-room-authz-isolation.spec.ts |
| 48 | order-messages.ts:124 | GET | `/api/orders/:orderId/messages` | `preValidation:[verifyAuth]` | none | `{success, messages: [...]}` | 404 NOT_FOUND (same 3-way tenant check) | global | — | 🔴 RLS/AUTHZ (courier READ binding via `courierCanReadOrder` — wider than send: incl. `offered`) | same | tests/cr8-order-messages.spec.ts |
| 49 | order-messages.ts:161 | POST | `/api/orders/:orderId/messages/read` | `preValidation:[verifyAuth]` | none | `{success:true}` | 404 NOT_FOUND (same check) | global | — | 🔴 RLS/AUTHZ (same read-binding check) | same | tests/cr8-order-messages.spec.ts |

### `routes/couriers.ts` — 1 route (subtotal 15) — no prefix option (registered bare, `fastify.register(courierRoutes)`)

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 50 | couriers.ts:8 | POST | `/couriers/invites` ⚠ **not under `/api`** | `preHandler:[verifyAuth, requireRole(['owner'])]` | strict `{locationId: uuid}` | `{code}` (invite code, 7-day expiry) | 403 FORBIDDEN, 404 NOT_FOUND (explicit live-membership-ownership predicate — comment cites the BYPASSRLS-pool cross-tenant risk explicitly), 500 | global | — | 🔴 RLS (explicit `WHERE user_id=$1 AND location_id=$2 AND role='owner' AND status='active'` — hand-rolled ADR-0004-style check with an inline comment explaining WHY: BYPASSRLS pool would otherwise let an owner mint an invite for another tenant's location) | `crates/api/src/routes/core/couriers.rs` | **needs-new-E2E / possibly DEAD** — no FE reference to `/couriers/invites` (grep of `apps/web/src` returns zero hits); the courier-invite UI/E2E (`flow-ui-courier-invite.spec.ts`) exclusively calls the DIFFERENT owner-scoped route `POST /api/owner/locations/:locationId/courier-invites` (`owner/courier-invites.ts`). This route appears to be a superseded/orphaned duplicate. |

### `routes/auth/local.ts` — 1 route (subtotal 16) — prefix `/api`

| # | File:Line | Method | Full path | Auth guard | Req schema (Zod) | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 51 | auth/local.ts:36 | POST | `/api/auth/local/login` | public | `{email (rejects reserved TLD), password}` | `{access_token, refresh_token?, userId, activeLocationId}` | 401 INVALID_CREDENTIALS, 401 WRONG_AUTH_METHOD, 500 INTERNAL (argon2 failure), 503 SERVICE_UNAVAILABLE (pool checkout guard) | 5/1min | dev-bypass path gated by `devLoginAllowed(env)` (ADR-0003 — inert on prod: requires BOTH `ALLOW_DEV_LOGIN='true'` AND `DEV_AUTH_SECRET` set, boot-guard D forbids both on prod) | 🔴 AUTH (dual-path: flag-gated `signDevToken` dev bypass vs real argon2 `signAuthToken`; role/activeLocationId resolution via `memberships`→`organizations` fallback) | `crates/api/src/routes/auth/local.rs` | tests/admin-platform-authz.spec.ts, tests/deploy-validation.spec.ts, tests/cross-tenant-realtime-qa.spec.ts (18 spec files touch this — most-used login path) |

**CORE subtotal: 16/16 ✓**

---

## Grand total: 27 (courier) + 8 (customer) + 16 (core) = **51/51 — MATCH**

No FALSE-POSITIVE rows in this partition — every grepped registration is a real, reachable Fastify route (including the orphaned-looking `/couriers/invites`, which is real code, just apparently unreferenced by the FE).

---

## Cross-cutting notes

**State-machine map — every order/assignment status-transition endpoint, from→to (the 🔴 core of this census):**

*Courier assignment lifecycle (`courier_assignments.status`):*
1. `POST /api/courier/assignments/:id/accept` — `offered`→`accepted` (handshake path, order→`IN_DELIVERY`) **or** `assigned`→`accepted` (legacy path, order→`CONFIRMED`)
2. `POST /api/courier/assignments/:id/reject` — `assigned`→`rejected` (re-enqueue to dispatch queue)
3. `POST /api/courier/assignments/:id/picked-up` — `accepted`→`picked_up` (order→`IN_DELIVERY`)
4. `POST /api/courier/assignments/:id/delivered` — `picked_up`→`delivered` (order→`DELIVERED`) **or** →terminal-cancel (order→`CANCELLED`, no-cash tail via `payment_outcome`)
5. `POST /api/courier/assignments/:id/cancel` — `accepted`/`picked_up`→terminal (5-min accept-regret window; order reverts via `releaseBindingAndReoffer`)
6. `POST /api/courier/assignments/:id/abort` — `accepted`/`picked_up`→terminal (no time gate, en-route emergency exit; same rail as #5)
7. `POST /api/courier/assignments/:id/decline` — `offered`→`offered_expired` (re-enqueue; customer order untouched)

*Courier shift lifecycle (`courier_shifts.status`):*
8. `POST /api/courier/me/shift/start` / `POST /api/courier/shifts/transition {to:'available'}` — `offline`→`available`
9. `POST /api/courier/me/shift/end` / `POST /api/courier/shifts/transition {to:'offline'}` — `available`→`offline` (blocked 409 if `on_delivery` or active assignment exists)
10. (implicit, driven by assignment accept) — `available`→`on_delivery`

*Order lifecycle (`orders.status`):*
11. `PATCH /api/orders/:id/status` (owner-driven, generic) — any legal transition per `assertOwnerTargetAllowed`; `IN_DELIVERY` target triggers "honest dispatch" (finds courier BEFORE advancing, or stays put + `dispatched:false`); `DELIVERED`/`PICKED_UP` targets are BLOCKED (409) if an active/undelivered courier binding exists — those states are reachable ONLY via the courier-side `/delivered` route or the owner-proxy `/deliver` route (owner/dashboard.ts, out of this partition).
12. `POST /api/customer/orders/:orderId/cancel` — `IN_DELIVERY`→`CANCELLED` (customer-driven, 5-min window from pickup; records `refund_due` for paid crypto orders)
13. (implicit) `POST /api/orders` — creates at `PENDING`

**Totals (verified by direct count against the table, not estimated):** 🔴 red-line rows = **39 of 51** — courier: 7/9 assignments.ts rows (all mutating transitions; the 2 GET reads are not red-line) + 2/6 me.ts rows (password, earnings) + 4/5 shifts.ts rows (all mutating transitions; the 1 GET read is not) + 4/5 auth.ts rows (login/redeem/refresh/logout; the invite-details GET is not) + 2/2 settlements.ts rows = 19; customer: 1/3 orders.ts row (cancel) + 0/2 push.ts + 2/2 otp.ts + 1/1 track.ts = 4; core: 8/8 auth.ts rows + 3/3 orders.ts rows + 3/3 order-messages.ts rows + 1/1 couriers.ts row + 1/1 auth/local.ts row = 16. **needs-new-E2E count = 9**: assignments `/abort` (#8); courier/me.ts `/me/messenger` (#11); customer orders `/rating` (#29); customer push `/subscribe`+`/unsubscribe` (#31, #32); customer otp `/send`+`/verify` (#33, #34); core `auth/courier/activate` (#43); core `couriers.ts /couriers/invites` (#50, flagged as possibly-dead code, not merely untested).


---

<!-- ============ §1.3 HTTP ROUTES — PUBLIC + ADMIN + DEV + WEBHOOKS + SPA-PROXY + INFRA (72) ============ -->

# Route Census — PUBLIC + ADMIN + DEV + WEBHOOK + SPA-PROXY + INFRA
Rust rebuild map input · dowiz/DeliveryOS → Rust/axum/tokio/sqlx, utoipa OpenAPI 3.1, DB unchanged.

**Extraction command (authoritative, run from `apps/api`):**
```
grep -rnE "^\s*(fastify|app|server|f|instance)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\."
```
Full repo run: 216 total registrations across the whole API. **This document's scope** (spa-proxy + public + dev + admin + acquisition + server.ts-inline-dev + health + telegram-webhook + payments-webhook + metrics) = **expected 72**.

**Actual (verified by isolating my file set from the same grep, counted per file below): 72 / 72 — exact match, zero delta.**

Per-file actual counts (all confirmed against live `grep -n` output, not the index):
`spa-proxy.ts`=18 · `public/client-flow.ts`=4 · `public/seo.ts`=3 · `public/menu.ts`=3 · `public/claim.ts`=3 · `public/telemetry.ts`=2 · `public/voice-config.ts`=1 · `public/vapid.ts`=1 · `public/theme.ts`=1 · `public/ssr.ts`=1 · `public/rates.ts`=1 · `public/pwa.ts`=1 · `public/funnel.ts`=1 · `public/fallback-config.ts`=1 · `public/branding-preview.ts`=1 · `public/access-requests.ts`=1 (public subtotal 25) · `dev/mock-auth.ts`=6 · `admin/backups.ts`=3 · `admin/fallback.ts`=2 · `admin/notification-audit.ts`=1 (admin subtotal 6) · `modules/acquisition/route.ts`=9 · `server.ts`=3 (inline dev routes only) · `routes/health.ts`=2 · `routes/telegram-webhook.ts`=1 · `routes/payments-webhook.ts`=1 · `lib/metrics.ts`=1.

**18+25+6+9+3+2+1+1+1 = 72.** No false positives to mark — every grep hit in this file set is a real Fastify route registration (no commented-out code, no test-file leakage since `.test.`/`.spec.` are excluded).

---

## ⚠️ Critical framing correction — `spa-proxy.ts` is NOT the SSR/bot-detection file

The task brief assumed `routes/spa-proxy.ts` contains bot-UA detection, SSR-vs-shell branching, redirects, and slug-based tenant resolution. **Having read all 885 lines, none of that lives here.** That logic is in `routes/public/ssr.ts` (`isBot`, `renderMenuPage`, `renderShadowPreview`) and `routes/public/client-flow.ts` (`serveSpaShell`) — both in the **public/** section below, not spa-proxy.

`spa-proxy.ts` is a legacy-named grab-bag of **owner-plane JWT-authenticated API endpoints** (analytics, brand/theme CRUD, settings, courier-invites, onboarding, customer CRM) plus **two dynamic media-serving routes** (`/images/*`, `/media/*` — R2/local-fs proxies with traversal guards, not literal static files) and **one public slug-keyed theme reader** (`/api/public/theme/:slug`) and **one public unauthenticated upload** (`/api/public/entry-photo`). Tenant resolution here is **JWT-derived `locationId`** (via `getLocationId`/`getOwnerContext`, re-verified against a live `memberships` row per ADR-0004 — never the baked JWT claim alone), not slug-based routing. There are no redirects and no bot-UA branches anywhere in this file.

The deep-dive columns below are filled with what's *actually* present (cache headers/TTL, tenant resolution, claim-check pattern) and explicitly marked N/A where the brief's assumed concern doesn't apply to this file.

---

## Table: `routes/spa-proxy.ts` (18) — deep dive

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | spa-proxy.ts:158 | GET | `/images/*` | none (public); traversal guard on wildcard key (`..`, `\0`, `\\` rejected) | none (raw wildcard param) | binary webp, `Cache-Control: public,max-age=31536000,immutable` | 400 INVALID_KEY, 404 NOT_FOUND | global 100/min | none | — | dynamic proxy over `storage.get(key)` (R2/local-fs) — **not** literal static; needs a custom axum handler, not `tower_http::ServeDir`. Target: `crates/api/src/routes/media/images.rs` | `e2e/tests/flow-client-product-images.spec.ts`, `flow-ui-images.spec.ts` |
| 2 | spa-proxy.ts:184 | GET | `/media/*` | none (public); same traversal guard | none | binary (webp/jpeg/png/mp4 by ext), 1y immutable cache | 400 INVALID_KEY, 404 NOT_FOUND | global 100/min | MEDIA_RICH_ENABLED gates *content* upstream, not this proxy | — | same as #1; content-type sniffed from key extension | needs-new-E2E (no spec hits `/media/` directly — product-media E2E cover the upload side, not this GET) |
| 3 | spa-proxy.ts:213 | POST | `/api/owner/menu/products/:productId/image` | global Bearer-presence gate (server.ts:421, `/api/owner/` prefix) + `getLocationId()` (JWT verify + live `memberships` re-check, ADR-0004) | multipart file (sharp-processed: resize 800×800, webp q82); content-hashed key (sha256×12) | `{imageUrl, imageKey}` | 401 UNAUTHORIZED, 400 VALIDATION_FAILED (no file / bad image), 500 (store/db failure) | global 100/min | none | tenant-scoping (owner-only write) | `crates/api/src/routes/owner/products.rs` (image sub-resource) | `e2e/tests/flow-client-product-images.spec.ts`, `flow-ui-images.spec.ts` |
| 4 | spa-proxy.ts:268 | POST | `/api/public/entry-photo` | none (public, anonymous, pre-order) | multipart image ≤8MB, must be `file` field + image mimetype | `{key, url}` | 400 VALIDATION_FAILED, 413 FILE_TOO_LARGE | **8/min** (explicit, overrides global) | none | **claim-check pattern**: key is unguessable (`entry-photos/<uuid>.webp`), never linked to an order until the order references it, only revealed to the assigned courier during the active delivery (P0 privacy hardening, UX-3) | `crates/api/src/routes/public/entry_photo.rs` | `e2e/tests/ux3-entry-photo.spec.ts`, `e2e/visual/courier-path.visual.spec.ts` |
| 5 | spa-proxy.ts:296 | GET | `/api/owner/analytics` | global Bearer gate + `getOwnerContext()` + `withTenant(db, userId, …)` RLS scoping | none | revenue/orders/avgOrderValue/deliveryTime/chart/topProducts/geoLocations/heatmap (7d/30d aggregates) | 401 UNAUTHORIZED | global 100/min | none | tenant-scoping (RLS) | `crates/api/src/routes/owner/analytics.rs` | `e2e/tests/flow-ui-analytics-supplies.spec.ts`, `flow-ui-validation.spec.ts` |
| 6 | spa-proxy.ts:375 | GET | `/api/owner/analytics/product-orders` | global Bearer gate + `getLocationId()` | query `?name=` required | last-50 orders for a product name | 401, 400 VALIDATION_FAILED (missing name) | global 100/min | none | tenant-scoping | `crates/api/src/routes/owner/analytics.rs` | `flow-ui-analytics-supplies.spec.ts` |
| 7 | spa-proxy.ts:393 | GET | `/api/owner/orders` | global Bearer gate + `getLocationId()` | query `?status=` optional | last-50 orders incl. items/courier-name(masked)/rating/PII-masked signals | 401 | global 100/min | none | tenant-scoping; decrypts+masks courier PII (`decryptPII`) | `crates/api/src/routes/owner/orders.rs` | `e2e/visual/owner-path.visual.spec.ts`, `flow-ui-proof-comprehensive.spec.ts` |
| 8 | spa-proxy.ts:452 | GET | `/api/owner/couriers` | global Bearer gate + `getLocationId()` + explicit `BEGIN`/`set_config(app.current_tenant)`/`COMMIT` tx | none | courier roster w/ status, deliveries, avg rating (PII decrypted server-side only for display) | 401 | global 100/min | none | tenant-scoping (RLS GUC), PII decrypt (`decryptPII`, best-effort try/catch) | `crates/api/src/routes/owner/couriers.rs` | `e2e/tests/flow-regulatory-settlements.spec.ts`, `owner-path.visual.spec.ts` |
| 9 | spa-proxy.ts:506 | GET | `/api/public/theme/:slug` | none (public) | slug param | `{primaryColor,bgColor,textColor,logoUrl,locationName,headingFont,bodyFont,supportedLocales}` | 404 NOT_FOUND | global 100/min | none | — the **one** slug-resolved route in this file | `crates/api/src/routes/public/theme.rs` | `e2e/tests/ux1-storefront-links.spec.ts`, `flow-ui-admin-branding.spec.ts` |
| 10 | spa-proxy.ts:528 | GET | `/api/owner/brand` | global Bearer gate + `getOwnerContext()` + `withTenant` | none | full theme row incl. Google Place/social fields | 401 | global 100/min | none | tenant-scoping | `crates/api/src/routes/owner/brand.rs` | `flow-ui-admin-branding.spec.ts`, `flow-ui-images.spec.ts` |
| 11 | spa-proxy.ts:562 | PUT | `/api/owner/brand` | global Bearer gate + `getOwnerContext()` | Zod `.strict()` `brandSchema` (hex-color regex, font-id allowlist regex, Instagram/Facebook URL allowlist) | updated theme row | 401, 400 (Zod parse throw → global error handler → VALIDATION_FAILED) | global 100/min | none | tenant-scoping; SET-vs-COALESCE font semantics (explicit null clears, undefined keeps) | `crates/api/src/routes/owner/brand.rs` | `flow-ui-admin-branding.spec.ts` |
| 12 | spa-proxy.ts:616 | POST | `/api/owner/brand/generate` | global Bearer gate + `getOwnerContext()` | `{website?, logoDataUrl?}` (logo capped ~3MB base64, no SSRF — data-URL only) | suggested `{primaryColor,bgColor,textColor,logoUrl,name,headingFont,bodyFont,sources}` (not persisted) | 401, 422 no_signal | global 100/min | none | SSRF-guard note (no server-side fetch of arbitrary logo URL) | `crates/api/src/routes/owner/brand.rs` | needs-new-E2E (no spec directly exercises `/brand/generate`) |
| 13 | spa-proxy.ts:667 | GET | `/api/owner/settings` | global Bearer gate + `getLocationId()`, soft-fallback via `isValidOwnerToken()` for fresh owner (no location yet) | none | location settings incl. fee/tax/hours/geo | 401 (only if token wholly invalid — a valid ownerless owner gets `{id:null}` 200, ADR "O1") | global 100/min | none | tenant-scoping | `crates/api/src/routes/owner/settings.rs` | `e2e/tests/flow-ui-admin-settings.spec.ts`, `flow-ui-proof-comprehensive.spec.ts` |
| 14 | spa-proxy.ts:701 | PUT | `/api/owner/settings` | global Bearer gate + `getLocationId()` | Zod `.strip()` `settingsSchema` (coerced numeric fields, currency enum) | updated settings | 401, 400 (Zod), 404 NOT_FOUND | global 100/min | none | tenant-scoping; money fields are integer minor-units (ADR-0005) | `crates/api/src/routes/owner/settings.rs` | `flow-ui-admin-settings.spec.ts` |
| 15 | spa-proxy.ts:742 | POST | `/api/owner/courier-invites` | global Bearer gate; soft-fallback via `isValidOwnerToken()` for fresh owner | `{phone?}` | `{link, code, phone, pending?}` | 401 (only if token wholly invalid) | global 100/min | none | tenant-scoping (link embeds locationId subdomain) | `crates/api/src/routes/owner/courier_invites.rs` (note: **separate** from `routes/owner/courier-invites.ts`, which is a *different* file with its own 3 routes — naming collision to resolve in Rust) | `e2e/tests/flow-ui-courier-invite.spec.ts` |
| 16 | spa-proxy.ts:758 | POST | `/api/owner/onboarding` | global Bearer gate + `getOwnerUserId()` (owner-role only, location optional) | ad-hoc body (name/phone/slug/lat/lng/menu_items/…) | `{success, slug, url}` | 401, 400 VALIDATION_FAILED, 409 SLUG_TAKEN | global 100/min | none | **tenant provisioning**: brand-new-owner path calls `bootstrap_owner()` SECURITY DEFINER (self-bootstraps first membership under `app.user_id` GUC, since the owner can't yet satisfy the member RLS policy) | `crates/api/src/routes/owner/onboarding.rs` | `e2e/tests/menu-first-onboarding.spec.ts`, `flow-ui-invite-onboarding.spec.ts` |
| 17 | spa-proxy.ts:838 | GET | `/api/owner/customers` | global Bearer gate + `getLocationId()` | none | customer CRM list, phone masked (`maskStr`) | 401 | global 100/min | none | tenant-scoping; PII-masked phone | `crates/api/src/routes/owner/customers.rs` | `e2e/tests/flow-ui-analytics-supplies.spec.ts`, `audit-fix-data-integrity.spec.ts` |
| 18 | spa-proxy.ts:856 | GET | `/api/owner/customers/:id/analytics` | global Bearer gate + `getLocationId()` | none | per-customer order history/preferences/heatmap | 401 | global 100/min | none | tenant-scoping (customer id + locationId both bound in WHERE) | `crates/api/src/routes/owner/customers.rs` | `flow-ui-analytics-supplies.spec.ts` |

**spa-proxy.ts subtotal: 18/18.**

### spa-proxy.ts deep-dive answers (per brief's requested axes)

| Axis | Finding |
|---|---|
| Bot-UA detection | **N/A to this file.** Lives in `public/ssr.ts` (`isBot()`) — see public/ table below. |
| SSR-vs-shell decision | **N/A to this file.** Lives in `public/ssr.ts` + `public/client-flow.ts` (`serveSpaShell()`). |
| Redirects | **None found in spa-proxy.ts.** No `reply.redirect()` call anywhere in the 885 lines. |
| Cache headers/TTLs | Only rows #1/#2 set cache headers: 1-year immutable (`/images/*`, `/media/*` — content-hashed keys make this safe). All `/api/owner/*` and `/api/public/theme/:slug`, `/api/public/entry-photo` responses are uncached (default no-cache JSON). |
| Claim-check behavior | Present **only** at row #4 (`/api/public/entry-photo`) — unguessable key minted pre-order, revealed to the courier only during active delivery. |
| Tenant slug resolution | Present **only** at row #9 (`/api/public/theme/:slug`, direct `WHERE slug = $1`). Every other row resolves tenant via **JWT `locationId`**, live-re-verified against `memberships` (ADR-0004) — never a route param. |

**3-line summary (spa-proxy Rust-vs-Astro split), as requested:**
1. **Nothing in `spa-proxy.ts` is SSR or belongs in an Astro shell** — it is 100% JSON API (owner CRUD/analytics/onboarding + two media-proxy GETs + one public theme GET + one public upload POST); all 18 registrations port to **Rust axum handlers** (`crates/api/src/routes/owner/*.rs`, `crates/api/src/routes/media/*.rs`, `crates/api/src/routes/public/{theme,entry_photo}.rs`), none to `crates/api/src/ssr/`.
2. The **actual** bot-UA/SSR-vs-shell/redirect logic that the brief expected lives in `public/ssr.ts` and `public/client-flow.ts` — those two files are the real `crates/api/src/ssr/` candidates, and in the Astro rebuild the bot-branch (`renderMenuPage`, `renderShadowPreview`) is the part that plausibly moves to an **Astro SSR route**, while `serveSpaShell` (human path) stays a thin Rust handler that serves the SPA shell HTML (or is replaced entirely by Astro's own routing once the storefront becomes an Astro app).
3. Net effect: rename/re-scope the module — Rust rebuild should **not** create a `crates/api/src/ssr/spa_proxy.rs`; instead split `spa-proxy.ts`'s content into the owner/media/public route modules above, and treat `ssr.ts`+`client-flow.ts` as the true SSR/Astro-handoff surface (documented next).

---

## Table: `routes/public/*` (25)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 19 | client-flow.ts:15 | GET | `/s/:slug/cart` | none | slug param | SPA shell HTML (CSP + per-tenant frame-ancestors) via `serveSpaShell` | (delegates to shell renderer) | global 100/min | none | — | **`crates/api/src/ssr/client_flow.rs`** (SSR/spa-proxy bucket — this IS the real SSR-adjacent file) | `e2e/tests/api-real.spec.ts` (`/s/:slug` family) |
| 20 | client-flow.ts:16 | GET | `/s/:slug/checkout` | none | slug param | SPA shell HTML | — | global 100/min | none | — | `crates/api/src/ssr/client_flow.rs` | as above |
| 21 | client-flow.ts:17 | GET | `/s/:slug/order/:id` | none | slug+id params | SPA shell HTML | — | global 100/min | none | — | `crates/api/src/ssr/client_flow.rs` | `e2e/tests/flow-customer-track-link.spec.ts` |
| 22 | client-flow.ts:18 | GET | `/s/:slug/orders/:orderId` (legacy alias) | none | slug+orderId params | SPA shell HTML | — | global 100/min | none | — | `crates/api/src/ssr/client_flow.rs` (keep as a redirect/alias in Rust, not a duplicate handler) | needs-new-E2E (legacy alias untested directly) |
| 23 | seo.ts:45 | GET | `/robots.txt` | none | none | robots directives incl. AI-bot allowlist (GPTBot/ClaudeBot/PerplexityBot/…) + sitemap link | 200 always | global 100/min | none | — | `crates/api/src/ssr/seo.rs` or static text asset (content is host-dependent — needs a handler, not pure static) | `e2e/tests/api-real.spec.ts:114` |
| 24 | seo.ts:90 | GET | `/sitemap.xml` | none | none | sitemap **index** → sharded children (50k-URL shard cap) | 500 on DB failure | global 100/min | none | — | `crates/api/src/ssr/seo.rs` | needs-new-E2E |
| 25 | seo.ts:121 | GET | `/sitemap-locations-:shard.xml` | none | shard param (parsed int) | per-shard `<urlset>` w/ hreflang alternates; excludes shadow tenants (`org.owner_id IS NOT NULL`) | 404 (empty shard), 500 | global 100/min | none | privacy (never lists unconsented shadow tenants — P6-2 B2) | `crates/api/src/ssr/seo.rs` | needs-new-E2E |
| 26 | menu.ts:231 | GET | `/public/locations/:locationIdOrSlug/menu` | none | locale query normalized (bounded charset ≤12) | full public menu incl. media-rich image resolution | 404 NOT_FOUND | global 100/min | MEDIA_RICH_ENABLED (gates media URL fill-in) | tenant-scoping (slug/id lookup) | `crates/api/src/routes/public/menu.rs` | `e2e/tests/menu.spec.ts`, `menu-load.spec.ts`, `storefront-smoke.spec.ts` |
| 27 | menu.ts:312 | GET | `/public/locations/:slug/info` | none | slug param | venue status (open/closed/busy), fees, hours, geo — mirrors order-total math (ADR-0005) | 404, 503 SERVICE_UNAVAILABLE (stale-cache exhausted) | global 100/min | none | tenant-scoping | `crates/api/src/routes/public/menu.rs` | `e2e/tests/storefront-characteristics.spec.ts`, `client/venue-state.spec.ts` |
| 28 | menu.ts:418 | GET | `/public/locations/:slug/products/:productId/media` | none | slug+productId | lazy product-media array (image/video/spin) | (empty array on any gate-fail, never errors) | global 100/min | MEDIA_RICH_ENABLED **+** location plan=='business' (`mediaServingAllowed`) | tenant-scoping | `crates/api/src/routes/public/menu.rs` | needs-new-E2E (media-rich feature dark; no live spec) |
| 29 | claim.ts:17 | POST | `/api/claim/accept` (mounted w/ `prefix:/api`) | `verifyAuth` (any authenticated user — no owner/membership role required yet) | Zod `{token}` (16-256 chars) | `{org_id, location_id, reauth:true}` | 400, 401 UNAUTHENTICATED, 409 ALREADY_CLAIMED, 401 INVALID_OR_EXPIRED_TOKEN, 403 CONTACT_MISMATCH/CONTACT_REQUIRED, 422 | **10/min** | none | 🔴 auth + tenant transfer (ownership mint) | `crates/api/src/routes/public/claim.rs` | `e2e/journeys/STAGING_AUDIT_2026-06-23.md`; `tests/claim-rls.test.ts`, `flow-simpl-claim-gf2g.test.ts` |
| 30 | claim.ts:49 | POST | `/api/claim/request` | none (public) | Zod `{slug}` (lowercase-alnum-hyphen) | `202 {requested:true}` uniform ack (never reveals shadow-tenant existence) | 400 | **5/min** | none | anti-enumeration | `crates/api/src/routes/public/claim.rs` | needs-new-E2E (unit-only coverage) |
| 31 | claim.ts:69 | POST | `/api/claim/decline` | none (token-only, no auth — "the restaurant can erase without an account") | Zod `{token}` | `{erased:true}` | 400, 401 (ClaimError code) | **10/min** | none | 🔴 destructive (hard-erase preview) | `crates/api/src/routes/public/claim.rs` | `tests/claim-notice.test.ts` |
| 32 | telemetry.ts:37 | POST | `/api/telemetry` | none (public) | Zod batch (≤20 events + ≤10 CWV metrics) | `202 {accepted, count}` (accepted:false on parse fail, still 202 — never blocks client) | none (always 202) | **disabled** (`rateLimit:false`) | none | — | `crates/api/src/routes/public/telemetry.rs` | `e2e/tests/api-real.spec.ts:98` |
| 33 | telemetry.ts:84 | POST | `/api/telemetry/abuse` | none (public) | loose (no Zod schema — raw body fields) | `202 {accepted}` | none (always 202) | disabled | none | — (note: weaker validation than #32 — no schema, worth tightening in Rust) | `crates/api/src/routes/public/telemetry.rs` | needs-new-E2E |
| 34 | voice-config.ts:11 | GET | `/api/public/voice-config` | none (public) | none | `{enabled: isVoiceEnabled()}` | none | global 100/min | VOICE (ADR-0015 kill-switch) | — deliberately placed under `/api/` so the storefront service-worker (cache-first) never pins it | `crates/api/src/routes/public/voice_config.rs` | `tests/voice-flag.test.ts` (unit); needs-new-E2E for the route itself |
| 35 | vapid.ts:5 | GET | `/api/push/vapid-public-key` | none (public) | none | `{publicKey}` | 404 NOT_FOUND (unconfigured) | global 100/min | VAPID_PUBLIC_KEY env presence | — | `crates/api/src/routes/public/vapid.rs` | `e2e/tests/api-real.spec.ts:91` |
| 36 | theme.ts:10 | GET | `/public/locations/:locationId/theme.css` | none (public) | locationId (uuid or slug) + optional `?hash=` | raw CSS text | (falls back to `DEFAULT_CSS` on any DB error — never 500s) | global 100/min | none | tenant-scoping (slug/uuid lookup) | `crates/api/src/routes/public/theme.rs` | `e2e/tests/api-real.spec.ts`, `flow-ui-images.spec.ts` |
| 37 | ssr.ts:18 | GET | `/s/:slug` | none | slug param | **bot** → SSR menu HTML (`renderMenuPage`) or shadow-preview HTML (`renderShadowPreview`, generic OG); **human** → SPA shell (`serveSpaShell`) | (falls through to real-tenant path if migration 070 absent, `42883`) | global 100/min | none | 🔴 privacy — shadow tenants (unconsented scraped listings) forced `noindex` + generic OG regardless of branch | **`crates/api/src/ssr/menu_page.rs`** — the true SSR/Astro-handoff file | `e2e/tests/api-real.spec.ts`, `flow-customer-checkout-render.spec.ts`, `ssr-jsonld-price.test.ts`, `ssr-escaping.test.ts` |
| 38 | rates.ts:14 | GET | `/v1/rates` | none (public) | none | `{base:'ALL',target:'EUR',rate,fetchedAt}` | 500 INTERNAL (query throw) | global 100/min | none | — static ALL→EUR fallback (0.0099) when the hourly worker table is empty, 300s cache on fallback only | `crates/api/src/routes/public/rates.rs` | `e2e/tests/flow-ui-proof-comprehensive.spec.ts` |
| 39 | pwa.ts:7 | GET | `/s/:slug/manifest.webmanifest` | none (public) | slug param | per-tenant PWA manifest (name/theme_color/icons) | (falls back to generic manifest on DB error, never 500s) | global 100/min | none | — | `crates/api/src/routes/public/pwa.rs` | `e2e/tests/api-real.spec.ts:80` |
| 40 | funnel.ts:34 | POST | `/api/funnel` | none (public, anonymous) | Zod `.strict()` `{locationId(uuid), sessionRef, eventType(enum), shownEtaLoMin?, shownEtaHiMin?}` | `204` uniform (always, regardless of validity/enabled-state — anti-enumeration) | none (uniform 204) | **60/min** per real IP | FUNNEL_INGEST_ENABLED (kill-switch, in-handler) | tenant-scoping (RLS GUC `app.current_tenant`) — pure observation, never gates an order | `crates/api/src/routes/public/funnel.rs` | `e2e/tests/flow-sensor-funnel.spec.ts` |
| 41 | fallback-config.ts:9 | GET | `/api/public/locations/:slug/fallback-config` | none (public) | Zod params `{slug}` | `{phone, showPhoneOnError, showPhoneOnOffline}` | 404 NOT_FOUND | global 100/min | none | tenant-scoping | `crates/api/src/routes/public/fallback_config.rs` | `e2e/tests/flow-offline-phone-fallback.spec.ts` |
| 42 | branding-preview.ts:6 | GET | `/branding-preview/:slug` | none (public) | slug param (unused in handler body — serves static shell regardless) | `index.html` w/ permissive iframe-embed CSP (`frame-ancestors *`) | — | global 100/min | none | 🔴 deliberately relaxed CSP/X-Frame-Options for iframe-embed preview — worth a Rust-side comment/ADR pointer | `crates/api/src/routes/public/branding_preview.rs` | `e2e/tests/flow-ui-admin-branding.spec.ts`, `behavioral-proof.spec.ts` |
| 43 | access-requests.ts:58 | POST | `/api/access-requests` | none (public); **registered only when `ACCESS_GATE_PUBLIC_ENABLED=true`** (else 404 via `setNotFoundHandler`) | Zod `ControlFields.strict()` (consent must be literal `true`, honeypot `website` must be empty) + separate lenient email parse | `200 {ok:true}` uniform (new/duplicate/honeypot/bad-email all byte-identical) | none (uniform 200) | **5/min** per real IP | ACCESS_GATE_PUBLIC_ENABLED (registration-level) | 🔴 anti-enumeration PII-capture; timing-parity `cheapNoOp` DB round-trip on the no-op path | `crates/api/src/routes/public/access_requests.rs` | `tests/access-requests.test.ts`, `tests/access-request-workers.test.ts` |

**public/ subtotal: 25/25** (client-flow 4 + seo 3 + menu 3 + claim 3 + telemetry 2 + voice-config 1 + vapid 1 + theme 1 + ssr 1 + rates 1 + pwa 1 + funnel 1 + fallback-config 1 + branding-preview 1 + access-requests 1).

---

## Table: `routes/dev/mock-auth.ts` (6) — 🔴 must NEVER ship enabled

All 6 routes are gated **globally** by `server.ts`'s `onRequest` hook (line 405-427) calling `isDevRequestAuthorized(url, header, env)` from `plugins/dev-guard.ts`: **fails closed** — 404 (not 401, to not even reveal existence) unless **both** `ALLOW_DEV_LOGIN==='true'` **and** a `x-dev-auth-secret` header exactly matches `env.DEV_AUTH_SECRET` (constant-time compare). This is the single choke-point that closed the 2026-06-22 prod dev-login backdoor (ADR-0003) — verified by `tests/dev-guard.test.ts`.

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 44 | mock-auth.ts:14 | POST | `/dev/mock-auth` | 🔴 dev-guard (see above) | `{role?, locationSlug?, locationId?, synthetic?}` | signed dev JWT (`signDevToken`, dev-keypair — prod verifier rejects it) | 409 SYNTHETIC_COURIER_MISSING | none (dev-gated instead) | ALLOW_DEV_LOGIN + DEV_AUTH_SECRET | 🔴 auth-bypass minter | `crates/api/src/routes/dev/mock_auth.rs` — **recommend: feature-gate out of release binary entirely (`#[cfg(feature = "dev-routes")]`), not just runtime-gated** | `e2e/lifecycle-e2e/auth.setup.ts` + ~80 specs (see `mock-auth` grep hit-list) |
| 45 | mock-auth.ts:122 | POST | `/dev/create-assignment` | 🔴 dev-guard | `{orderId, courierId, locationId}` | `{assignmentId, shiftId}` + WS publish (`assignment.created`, `task_assigned`) | 400 VALIDATION_FAILED | none | same | 🔴 test-only order/courier mutation | `crates/api/src/routes/dev/mock_auth.rs` | `e2e/lifecycle-e2e/critical-lifecycle.spec.ts`, `flow-core-lifecycles.spec.ts`, `ws-courier-assignment.spec.ts` |
| 46 | mock-auth.ts:184 | POST | `/dev/seed-telegram-target` | 🔴 dev-guard | `{locationId, userId?, address?}` | `{targetId, address}` | 400 VALIDATION_FAILED | none | same | 🔴 | `crates/api/src/routes/dev/mock_auth.rs` | `e2e/helpers/notifHelpers.ts` (helper used across notif specs) |
| 47 | mock-auth.ts:204 | POST | `/dev/repair-test-owner` | 🔴 dev-guard | `{email?, slug?, locationId?}` | membership-before/after diagnostic | 404 NOT_FOUND (user/location) | none | same | 🔴 mutates real fixture membership | `crates/api/src/routes/dev/mock_auth.rs` | referenced by fixture docs (`Test owner fixture (sushi=demo)` memory); no direct spec assertion found — needs-new-E2E |
| 48 | mock-auth.ts:583 | POST | `/dev/seed-visual-state` | 🔴 dev-guard | none | deterministic 3-venue + synthetic-courier fixture (`{open,closed,busy,stoplistProductId,orderId,syntheticCourierId,syntheticAssignmentId}`) | — | none | same | 🔴 | `crates/api/src/routes/dev/mock_auth.rs` | `e2e/visual/harness.ts` (`VisualFixtures` contract consumer) |
| 49 | mock-auth.ts:584 | POST | `/api/dev/seed-visual-state` (alias) | 🔴 dev-guard | same as #48 | same handler (`seedVisualHandler` shared) | — | none | same | 🔴 | same file — **duplicate registration of the same handler under 2 paths; collapse to one in Rust** | same |

**dev/mock-auth.ts subtotal: 6/6.**

**Duplicate-implementation flag (cross-cutting):** `mock-auth.ts:14` (`/dev/mock-auth`) and `server.ts:549` (`/api/dev/mock-auth`) are two **independently maintained, near-identical** implementations of the same owner/courier JWT minter (same synthetic-courier re-derive pattern, same upsert logic, copy-pasted). Same for `/dev/create-assignment` (mock-auth.ts:122) vs `/api/dev/create-assignment` (server.ts:653). This is real duplication risk (a security fix to one copy can silently miss the other) — **the Rust rebuild should collapse these to one handler registered at both paths**, not port both bodies.

---

## Table: `routes/admin/*` (6) — ADR-admin-platform-authz (B4)

**Auth guard (applies to all 6, NOT repeated per-file):** the parent `routes/admin/index.ts` is the **sole** thing mounted at `prefix:/api/admin` (enforced by an ESLint rule, `local/no-admin-register-outside-plane`). It registers, in this load-bearing order:
1. `fastify.verifyAuth` (`onRequest`) — populates `request.user` from the Bearer JWT.
2. `requirePlatformAdmin` (`onRequest`) — 403 non-admin, 503 fail-closed if the platform-admin check itself errors.

Defense-in-depth: a **root-instance** gate (`registerAdminPlaneGate`, `server.ts:829-830`, called via `lib/platform-admin.ts`) applies the same check structurally to **any** route matched under `/api/admin/*` — child, sibling, or future — independent of the parent plugin, closing a BOLA class even if a future route forgets to nest under the plane. Child route files (`backups.ts`, `fallback.ts`, `notification-audit.ts`) carry **no** per-file `verifyAuth`/`requireRole` — by design, per the ADR (a per-file owner check would incorrectly 403 a legitimate platform-admin).

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 50 | backups.ts:13 | GET | `/api/admin/backups` | platform-admin (parent+root gate) | query `{type?,status?,limit?}` | backup list + restore-drill result per row | — | global 100/min | none | 🔴 admin-authz | `crates/api/src/routes/admin/backups.rs` | `e2e/tests/admin-platform-authz.spec.ts` |
| 51 | backups.ts:73 | POST | `/api/admin/backups/verify` | platform-admin + single-flight advisory-lock (409 if a drill is in-flight) | `{backupId?}` uuid-regex-validated | drill result | 400 VALIDATION_FAILED, 409 drill_in_progress | **3/5min** | none | 🔴 admin-authz + DR-drill (destructive/weaponizable if unguarded) | `crates/api/src/routes/admin/backups.rs` | `admin-platform-authz.spec.ts` |
| 52 | backups.ts:100 | GET | `/api/admin/backups/dr-report` | platform-admin + single-flight | none | fleet-wide full-hash DR report | 409 drill_in_progress | **3/5min** | none | 🔴 admin-authz | `crates/api/src/routes/admin/backups.rs` | `admin-platform-authz.spec.ts` |
| 53 | fallback.ts:13 | GET | `/api/admin/fallback/health` | platform-admin | none | per-location fallback-channel coverage overview | — | global 100/min | none | 🔴 admin-authz | `crates/api/src/routes/admin/fallback.rs` | `admin-platform-authz.spec.ts` |
| 54 | fallback.ts:47 | POST | `/api/admin/fallback/r2-check` | platform-admin | none | `{totalLocations, withFallbackPhone, coveragePct}` | — | global 100/min | none | 🔴 admin-authz | `crates/api/src/routes/admin/fallback.rs` | `admin-platform-authz.spec.ts` |
| 55 | notification-audit.ts:17 | GET | `/api/admin/notification-audit` | platform-admin | Zod `.strict()` query `{event, locationId?, status?, sinceMinutes}` | grouped audit counts (event/status/channel/count) — PII-free by design | 500 (generic, `err.message` NOT leaked per ADR §F6) | global 100/min | none | 🔴 admin-authz | `crates/api/src/routes/admin/notification_audit.rs` | `admin-platform-authz.spec.ts` |

**admin/ subtotal: 6/6.**

---

## Table: `modules/acquisition/route.ts` (9) — mounted `prefix:/internal`

**What it does:** the P6-1/P6-2 internal/ops surface for the "acquisition → shadow-tenant provisioning → claim" pipeline (scraping a restaurant's public listing → minting a shadow storefront → inviting the real owner to claim it). **Auth guard:** a single `onRequest` hook (line 56-60) checks `provisionOpsAuthorized(header, opsSecret)` against `PROVISION_OPS_SECRET` (env) — **fail-closed 404** (existence-hiding, not 401/403) when the secret is unset or doesn't match. Deliberately **decoupled** from the dev-login family (breaker finding B4 — the ops surface must survive independently of the dev-auth secret rotating/being disabled) and mounted **outside** `/api/dev` so the dev-guard does not (and must not) apply here.

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 56 | route.ts:62 | POST | `/internal/acquisition` | 🔴 ops-secret (404 fail-closed) | Zod `{place_id}` | idempotent lifecycle-row create (repeat place_id → same row) | 400 VALIDATION_FAILED | 30/min | PROVISION_OPS_SECRET presence | 🔴 ops-secret gate | `crates/api/src/routes/internal/acquisition.rs` | `e2e/tests/p6-provision-verify.spec.ts`, `tests/acquisition-state-machine.test.ts` |
| 57 | route.ts:77 | POST | `/internal/acquisition/extract` | 🔴 ops-secret | Zod `{acquisition_source_id, website_url}` | AI-parsed menu extraction (SSRF-guarded website fetch → PII-redacted parse) | 400, 503 EXTRACTION_UNAVAILABLE (no parser wired) | 10/min | PROVISION_OPS_SECRET | 🔴 SSRF-guard + ops-secret | `crates/api/src/routes/internal/acquisition.rs` | `tests/extraction-orchestrator.test.ts` |
| 58 | route.ts:90 | POST | `/internal/acquisition/provision/mint` | 🔴 ops-secret | Zod `{acquisition_source_id}` | one-time provisioning token (plaintext returned ONCE, only hash stored) | 400, 409 (ProvisionError code) | 30/min | same | 🔴 single-use secret mint | `crates/api/src/routes/internal/acquisition.rs` | `tests/acquisition-service.test.ts` |
| 59 | route.ts:107 | POST | `/internal/acquisition/provision/spine` | 🔴 ops-secret | Zod `{acquisition_source_id, token, name, slug(regex), phone?}` | `{org_id, location_id}` — writes the shadow tenant spine via `provision_shadow` RLS policy | 400, 409 | 30/min | same | 🔴 tenant creation | `crates/api/src/routes/internal/acquisition.rs` | `tests/acquisition-service.test.ts` |
| 60 | route.ts:130 | POST | `/internal/acquisition/provision/hard-delete` | 🔴 ops-secret | Zod `{acquisition_source_id}` | `{deleted:true}` | — | 30/min | same | 🔴 destructive hard-delete (GDPR erase path for unclaimed shadows) | `crates/api/src/routes/internal/acquisition.rs` | `e2e/scripts/provision-claim-shadow.mjs` |
| 61 | route.ts:142 | POST | `/internal/acquisition/claim/verify` | 🔴 ops-secret | Zod `{acquisition_source_id}` | `{verified:true}` | 400, 409 (ClaimError) | 30/min | same | 🔴 | `crates/api/src/routes/internal/acquisition.rs` | needs-new-E2E |
| 62 | route.ts:159 | POST | `/internal/acquisition/claim/mint` | 🔴 ops-secret | Zod `{acquisition_source_id, invited_contact?, base_url?}` | single-use claim-invite token + Art-14 GDPR first-contact notice; **token rides the URL fragment (`#token=`), never the query string** (anti-Referer-leak) | 400, 409 | 30/min | same | 🔴 token-in-fragment (leak-safe transport) | `crates/api/src/routes/internal/acquisition.rs` | `e2e/journeys/STAGING_AUDIT_2026-06-23.md` |
| 63 | route.ts:186 | POST | `/internal/acquisition/complaint` | 🔴 ops-secret | Zod `{place_id, note?}` | `{recorded:true}` (structured log only) | 400 | 20/min | same | 🔴 | `crates/api/src/routes/internal/acquisition.rs` | needs-new-E2E |
| 64 | route.ts:199 | POST | `/internal/acquisition/retention/sweep` | 🔴 ops-secret | `{abandoned_ttl_days?}` | retention-sweep result (reaps expired grants/invites, hard-deletes past-TTL unclaimed shadows) | — | 6/min | same | 🔴 GDPR Art-5(e) — cron-driven | `crates/api/src/routes/internal/acquisition.rs` | needs-new-E2E |

**acquisition subtotal: 9/9.**

---

## Table: `server.ts` inline dev routes (3) — 🔴 duplicates of mock-auth.ts

Same global dev-guard as the `dev/mock-auth.ts` table above (isDevPath matches `/api/dev/` too).

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 65 | server.ts:549 | POST | `/api/dev/mock-auth` | 🔴 dev-guard | same shape as mock-auth.ts:14, **plus** a `fresh:true` mode (mints a brand-new ownerless owner for onboarding-wizard E2E) not present in the mock-auth.ts twin | signed dev JWT | 409 SYNTHETIC_COURIER_MISSING | none | ALLOW_DEV_LOGIN+DEV_AUTH_SECRET | 🔴 duplicate-of-#44 (see cross-cutting note above) — **the `fresh` mode is the one behavioral difference to preserve when collapsing** | `crates/api/src/routes/dev/mock_auth.rs` | `e2e/tests/flow-ui-invite-onboarding.spec.ts` and ~80 more (mock-auth grep hit-list) |
| 66 | server.ts:653 | POST | `/api/dev/create-assignment` | 🔴 dev-guard | `{orderId, courierId, locationId}` | `{assignmentId}` + WS publish, **also inserts a throwaway courier row inline** (mock-auth.ts's twin assumes the courier already exists) | 400, 500 | none | same | 🔴 duplicate-of-#45 | same file target | `e2e/tests/ws-courier-assignment.spec.ts`, `dev/create-assignment` grep hit-list |
| 67 | server.ts:701 | POST | `/api/dev/seed-data` | 🔴 dev-guard | `{slug?, name?, phone?}` | seeds a demo location + 4 categories + 5 products (+ default theme) — **no twin in mock-auth.ts** | 500 | none | same | 🔴 | `crates/api/src/routes/dev/seed_data.rs` | `e2e/tests/seed.spec.ts` |

**server.ts subtotal: 3/3.**

---

## Table: `routes/health.ts` (2)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 68 | health.ts:61 | GET | `/livez` | none | none | `{status:'ok', timestamp}` — **liveness only**, deliberately does NOT touch Postgres (Fly's 3s machine-health-check budget; a DB blip must never restart the machine and sever live WS) | none | **explicit `rateLimit:false`** | none | — | `crates/api/src/observability.rs` | `e2e/tests/prod-smoke.spec.ts` |
| 69 | health.ts:65 | GET | `/health` | none (unauthenticated by design — payload is minimal, no driver internals) | none | 11 sequential checks: postgres(critical)/workers/redis/telegram/r2/settlement/anonymizer/backup/backup-restore/fallback-coverage/free-tier — `{status, timestamp, checks:{name:{status,latencyMs}}}` | **503** if any check is `down` (currently only postgres timeout can produce `down`); 200 w/ `degraded` for soft-check failures | **explicit `rateLimit:false`** | BACKUP_ENABLED gates r2+backup-drift+restore-test sub-checks; TELEGRAM_BOT_TOKEN presence gates telegram sub-check | — readiness/ops surface, not money/auth, so no 🔴 | `crates/api/src/observability.rs` | `e2e/tests/api-real.spec.ts`, `deploy-validation.spec.ts`, `telegram-webhook.spec.ts` |

**health.ts subtotal: 2/2.** Semantics note for Rust: `/livez` must stay a **zero-dependency** handler (no DB pool touch) — this is exactly the incident class (2026-06-XX "service falling down") the comment documents; port it as a pure `async fn` with no `Extension<PgPool>` in its signature, not just a fast happy path through the same handler as `/health`.

---

## Table: `routes/telegram-webhook.ts` (1) — 🔴 webhook signature

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 70 | telegram-webhook.ts:36 | POST | `` /webhook/telegram/${telegramBotSecret} `` (secret embedded **in the path itself**, not a header) | 🔴 `x-telegram-bot-api-secret-token` header compared to `env.TELEGRAM_BOT_SECRET` **when configured**; **soft-fails open** if the header is absent entirely (logged warning, request still processed — "backward compat with webhooks set without secret_token") | raw Telegram Update JSON (manually parsed, not Zod-validated) | **always `{ok:true}`** — even on internal processing failure (Telegram-retry-suppression: "best-effort, off critical-path") | none surfaced to Telegram (401 only on secret **mismatch**, not on missing header) | global 100/min (no override — worth flagging: Telegram's servers could get 429'd under burst) | TG_CATEGORY_GATING, TG_STOREFRONT_ACTION (gate individual callback actions, not the route) | 🔴 webhook-signature (partial — path-secret + optional header, soft-fail-open on missing header is a **weaker posture than payments-webhook.ts's hard fail-closed**, worth hardening in Rust) | `crates/api/src/routes/webhooks/telegram.rs` | `e2e/tests/telegram-webhook.spec.ts`, `telegram-webhook-test.spec.ts`, `telegram-full-flow.spec.ts`, `tests/notifications/telegram-webhook-storefront.test.ts` |

**telegram-webhook.ts subtotal: 1/1.**

---

## Table: `routes/payments-webhook.ts` (1) — 🔴 money source-of-truth

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 71 | payments-webhook.ts:13 | POST | `/webhook/payments/plisio` | 🔴 **HMAC signature, hard fail-closed**: `provider.verifyWebhook(rawBody, payload)` — a forged/garbled body is **401**, never silently 200'd (contrast w/ telegram-webhook's soft-fail-open) | raw body (`config:{rawBody:true}`) + provider-specific payload | `{ok:true}` on success/no-op; the **sole writer** of `payment_status='paid'/'failed'` on `orders`/`payments` | 401 invalid signature, 400 missing ref, 500 (real error → **lets Plisio retry**, deliberately not swallowed) | global 100/min (no override) | 🔴 **dark unless `PAYMENTS_CRYPTO_ENABLED`** (404 otherwise, checked first line of handler) + provider-name check | 🔴🔴 **money + webhook-signature** — idempotent via `payment_events` composite-unique insert-wins (not check-then-act); tenant resolved via a DEFINER function (`payment_location_by_provider_ref`) **without** a member context, then `set_config(app.current_tenant)` for the rest of the tx; atomically emits `refund_due` in the same tx when a payment completes against an already-cancelled order (LC6 pay-after-cancel) | `crates/api/src/routes/webhooks/payments.rs` | `tests/refund-due-spine.test.ts`, `tests/money-spine-fixture.ts` (no live e2e — Plisio webhook is inherently hard to E2E without a sandbox provider; unit/integration is the honest proof surface here) |

**payments-webhook.ts subtotal: 1/1.**

---

## Table: `lib/metrics.ts` (1)

| # | File:Line | Method | Full path | Auth guard | Req schema | Response (1-line) | Errors | Rate-limit | Flags | 🔴 | Rust target | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 72 | metrics.ts:134 | GET | `/metrics` | 🔴 constant-time bearer-token match against `METRICS_TOKEN` (`crypto.timingSafeEqual`); **dark by default** — 404 (not 401) when `METRICS_TOKEN` is unset, so the endpoint's existence is hidden until an operator opts in | none | Prometheus text-exposition v0.0.4: `http_requests_total`, `http_request_duration_seconds_{bucket,sum,count}` (9 latency buckets, 0.025s–10s), `ws_messages_out_total`, pool-saturation gauges (`pg_pool_operational_*`, `pg_pool_session_*`), `pgboss_jobs_pending`, process RSS/heap/uptime | 401 Unauthorized (token present but wrong), 404 (token unset) | none (excluded from its own `onResponse` recorder — `route !== '/metrics'` guard prevents self-scrape pollution) | METRICS_TOKEN presence | 🔴 token-gated (constant-time compare — timing-attack-aware) | `crates/api/src/observability.rs` (use the `metrics`/`prometheus` crate's exposition encoder rather than hand-rolled string-building) | `tests/metrics.test.ts` |

**metrics.ts subtotal: 1/1.**

---

## Grand total check

18 (spa-proxy) + 25 (public) + 6 (dev/mock-auth) + 6 (admin) + 9 (acquisition) + 3 (server.ts) + 2 (health) + 1 (telegram-webhook) + 1 (payments-webhook) + 1 (metrics) = **72 = 72 expected. Zero delta, zero false positives to mark.**

---

## Non-route HTTP surface table (server.ts + plugins/) — infra, not a route registration

| Concern | File:Line | Behavior | Rust target |
|---|---|---|---|
| Static file mount | `server.ts:156-161` | `@fastify/static` root=`apps/api/public`, prefix `/`, `cacheControl:true`, `maxAge:'365d'` | `tower_http::services::ServeDir` + `ServeFile` layer, same maxAge |
| Cache-header override (onSend) | `server.ts:167-176` | Post-static hook: `text/html` → `no-cache,no-store,must-revalidate`; `text/css`/`application/javascript` → `public,max-age=31536000,immutable`. Comment notes fastify-static may bypass `onSend` for piped sends — only the SPA-fallback `reply.sendFile()` path reliably triggers it | axum middleware layer (`tower::Layer`) applied uniformly — no piping-bypass caveat in axum/tower-http since responses always flow through the tower service stack |
| Subdomain rewrite | `server.ts:180-185` | `resolveSubdomainRewrite(hostname, url)` — `margherita.dowiz.org/<path>` → internal `/s/margherita` rewrite (tenant routes only); mutates `request.raw.url` | axum middleware rewriting `Uri` before router dispatch (a `map_request` layer) |
| Correlation-ID | `server.ts:187-204` | Server-authoritative: `genReqId: crypto.randomUUID()`, `requestIdHeader:false` (inbound header NEVER trusted as the id — ADR-0010 A1/B6). Inbound `x-correlation-id` demoted to a sanitized `clientTraceId` (regex `^[A-Za-z0-9._-]{1,128}$`, else `undefined`) for WS/log stitching only | `tower_http::request_id::MakeRequestUuid` + custom extractor for the sanitized client trace id |
| Security headers (global) | `server.ts:126-136` | Unconditional `onRequest`: HSTS (prod only), `X-Content-Type-Options:nosniff`, `X-Frame-Options:SAMEORIGIN` (**skipped** when `?embed=true` — widget-embed use case), `Referrer-Policy` | axum middleware, conditional on query param |
| Security headers (route-scoped CSP) | `lib/security/headers.ts` (`securityHeadersPlugin`, registered `server.ts:396`) | `onRequest`: full CSP+frame-ancestors+Permissions-Policy for `/api/admin`, `/api/owner`, `/api/customer`, `/api/courier`, `/api/orders`, `/api/telemetry`, `/api/push`, `/auth/`, `/couriers/`. `onSend` fallback: applies the same CSP to any `text/html`/`application/json` response that doesn't already carry a CSP header (idempotent guard: `if (reply.getHeader('Content-Security-Policy')) return`). A **second** `onSend` hook rewrites any bare-string 404 body to a fixed JSON shape | tower middleware layer, path-matched via axum's `MatchedPath` |
| CORS | `server.ts:140-154` | Global default: **deny all** cross-origin (`origin:(o,cb)=>cb(null,false)` unless no Origin header at all, `credentials:false`). Then a **second** `onRequest` hook **overrides** to `Access-Control-Allow-Origin:*` for `/public/locations/*`, `/s/*`, and `POST /api/orders*` (storefront widget embeds need cross-origin reads/writes) | `tower_http::cors::CorsLayer` — needs a **per-path conditional layer** (axum doesn't have a single drop-in for "deny by default, allow-* for these 3 prefixes" — implement as a custom `tower::Layer` wrapping `CorsLayer`, or two nested routers) |
| Dev-path gate | `server.ts:405-427` (`isDevPath`/`isDevRequestAuthorized`, `plugins/dev-guard.ts`) | See dev/mock-auth table above — 404 fail-closed unless `ALLOW_DEV_LOGIN==='true'` **and** matching `x-dev-auth-secret` (constant-time) | axum middleware / route-group guard; **recommend compiling dev routes out of release builds** rather than porting the runtime gate verbatim |
| Auth-prefix 401 gate | `server.ts:399-427` | `AUTH_PREFIXES = ['/api/owner/','/api/courier/','/api/customer/']` → bare Bearer-presence check (401 if missing) **before** any route-specific auth runs; `NO_AUTH_PATHS` carve-outs (`/api/courier/auth/`, `/api/customer/track/exchange`) plus a regex carve-out for pre-auth OTP send/verify | axum middleware — a cheap "is there a bearer token at all" pre-check ahead of the real `verifyAuth` extractor, preserved as a fast-reject layer |
| `setErrorHandler` | `server.ts:443-517` | ONE structured error envelope (`{code,message,fields?,correlationId,retryAfterMs?,status,error}`; legacy `error` string retained for un-migrated FE). Handles AJV validation, Zod validation (via custom validator compiler), `ApiError` instances, PG/driver codes (never leaked raw — `isContractCode` allowlist), 5xx → generic message + Sentry capture w/ correlationId tag, `retry-after` header on rate-limit | axum's `IntoResponse` for a shared `ApiError` enum + a fallback `tower::Layer`/`Handler` for panics/unmatched errors — utoipa can generate the OpenAPI error schema from the same enum |
| `setNotFoundHandler` | `server.ts:841-852` | GET + `text/html` accept **or** matches `SPA_ROUTES` prefix list (`/admin`,`/courier`,`/dashboard`,`/s/`,`/login`,`/branding-preview`,`/privacy`) → serve `index.html` (SPA fallback); everything else → the ONE error envelope, `404 NOT_FOUND` | axum `fallback()` handler with the same prefix-match branch |
| Custom validator/serializer compiler | `server.ts:106-124` | Zod v3 native `safeParse` (not the AJV-JSON-schema path) — needed because `fastify-type-provider-zod@6.x` requires Zod v4 but the codebase pins v3.25.x | N/A in Rust — `serde`+`validator`/`garde` or `utoipa`'s native Zod-equivalent; no version-mismatch shim needed |
| Multipart limits | `server.ts:355-358` | `@fastify/multipart`: `fileSize: 10MB`, `throwFileSizeLimit:true` (global); **per-route override** at `spa-proxy.ts:271` (`entry-photo`: 8MB) | `axum::extract::Multipart` + a `DefaultBodyLimit::max(10*1024*1024)` layer, with a per-route override via a route-specific layer for entry-photo |
| Body limit | `server.ts:91` | Fastify constructor `bodyLimit: 10MB` (comment: "individual routes can override via route config") | `tower_http::limit::RequestBodyLimitLayer` |
| Global rate-limit | `server.ts:360-376` | `@fastify/rate-limit`: **100/min default**, keyed by `clientIp(request)` (Fly-Client-IP header **only** — never client-injectable X-Forwarded-For, `lib/client-ip.ts`). `errorResponseBuilder` throws an `ApiError` (not a plain body) so the 429 routes through the ONE `setErrorHandler` envelope w/ `retryAfterMs`/`x-ratelimit-*` | `tower_governor` or a custom `tower::Layer` keyed on a `ConnectInfo`-derived real-IP extractor (Fly-specific header, not `X-Forwarded-For`) — this Fly-Client-IP-only keying is a **security-relevant detail to preserve exactly**, not a generic IP extractor |
| Boot-time schema guard | `server.ts:224-226` (`assertSchemaCurrent`) | Fails fast (process exit) if the DB schema head doesn't match the build — prevents booting against un-migrated DB | Rust boot-time `sqlx::migrate!` check or equivalent version-row assertion |
| Worker-boot budget | `server.ts:333-350` | `Promise.race` against a 3s budget (`WORKER_BOOT_BUDGET_MS`) so Fly's `/livez` check (15s interval, no grace period) never fails a deploy because background workers (notification queue, telegram poller, etc.) are slow to start — workers continue in the background past the budget | Rust: `tokio::select!` race between worker-init future and a `tokio::time::sleep(3s)`, same non-blocking-listen intent |
| Uncaught-exception/rejection guards | `server.ts:73-80` | Global `process.on('unhandledRejection'/'uncaughtException')` → log + Sentry-capture + **keep serving** (never exit) — a single dropped promise must not sever every live WebSocket | Rust: this class of bug mostly can't happen the same way (no uncaught-panic-equivalent silently swallowing) — panics in a spawned task are isolated per-task by tokio; the equivalent guard is a `catch_unwind` + structured log around per-connection/per-task futures, not a process-global hook |

---

## Cross-cutting notes for the Rust rebuild

### SSR strategy — Astro handoff

- **True SSR concerns** (bot-UA detection, SSR-vs-shell branching, shadow-tenant honest-preview rendering, JSON-LD/OG tag emission) live in exactly **two** files: `routes/public/ssr.ts` (`/s/:slug`) and `routes/public/client-flow.ts` (`/s/:slug/{cart,checkout,order/:id,orders/:orderId}`). Both delegate to `lib/spa-shell.ts` (`isBot`, `serveSpaShell`) and `lib/ssr-renderer.ts`/`lib/preview-render.ts` for the bot branch.
- **Astro-handoff candidate:** the **bot branch** (`renderMenuPage`, `renderShadowPreview`) is exactly the workload Astro's SSR/islands model targets — static-ish HTML with dynamic data injection, no client JS needed for a crawler. In the rebuild, this branch is the strongest candidate to move to an **Astro route** (`src/pages/s/[slug].astro` with `Astro.request.headers` bot-UA check), while the human branch (`serveSpaShell`) either stays a thin Rust redirect/passthrough to the SPA build or is subsumed entirely once the storefront itself becomes the Astro app (no separate "shell" concept needed).
- **Everything else registered under `/s/:slug/*`** (cart, checkout, order, legacy orders alias) is **100% the human branch** — always `serveSpaShell`, never bot-branched — so these 4 routes are pure "serve the SPA shell HTML" and migrate identically whether the target is a thin Rust handler or a redirect into Astro's own routing.
- **`spa-proxy.ts` is NOT part of this SSR surface** (see the critical framing correction at the top) — despite its name and the module-convention note "`SSR/spa-proxy → crates/api/src/ssr/`", none of its 18 routes belong in `crates/api/src/ssr/`. Rebuild planning should route this file's content to `crates/api/src/routes/owner/*.rs` / `routes/public/*.rs` / `routes/media/*.rs` instead, and reserve `crates/api/src/ssr/` for `ssr.ts` + `client-flow.ts` only.

### Cache TTL table (every `Cache-Control` found across this file set)

| Route | Cache-Control | TTL | Rationale (from source comments) |
|---|---|---|---|
| `/images/*`, `/media/*` (spa-proxy) | `public, max-age=31536000, immutable` | 1 year | content-hashed keys — a changed image is a new URL, so immutable-forever is safe |
| static assets (`.css`/`.js`, server.ts onSend) | `public, max-age=31536000, immutable` | 1 year | build-hashed asset filenames |
| static HTML (server.ts onSend) | `no-cache, no-store, must-revalidate` | 0 | SPA shell must always re-fetch (index.html is the app-version pointer) |
| `/public/locations/:id/menu` | `public, max-age=60, stale-while-revalidate=300` | 60s fresh / 300s SWR | HTTP-cache layer on top of the in-process 30s/300s app-level cache (menu.ts) — two independent cache layers |
| `/public/locations/:slug/info` | no explicit header (relies on in-process cache only: 30s fresh) | 30s (app-level) | shares the storefront-blink fix cache with `/menu` |
| `/public/locations/:slug/products/:id/media` | `public, max-age=60, stale-while-revalidate=300` | 60s / 300s SWR | lazy media payload, same policy as menu |
| `/public/locations/:locationId/theme.css` | `public, max-age=60` (no hash) / `public, max-age=31536000, immutable` (hashed) | 60s or 1y | hash-qualified theme versions are immutable; the "latest" pointer is short-cached |
| `/s/:slug/manifest.webmanifest` | `public, max-age=3600` | 1 hour | PWA manifest, low churn |
| `/robots.txt` | `public, max-age=86400` | 1 day | rarely changes |
| `/sitemap.xml`, `/sitemap-locations-:shard.xml` | `public, max-age=3600` | 1 hour | |
| `/v1/rates` (fallback path only) | `public, max-age=300` | 5 min | only set when the hourly-refresh worker's table is empty — short so the real worker value is picked up fast |
| `/api/public/voice-config` | `no-store` | 0 | deliberate — an emergency VOICE kill-switch must never be delayed by any shared/edge cache |
| menu payload (in-process, app-level, not an HTTP header) | fresh 30s / stale-but-served 300s / stale-on-error ceiling 3600s (1h) | see menu.ts `MENU_CACHE_*_MS` constants | the storefront-blink fix — collapses a connection-pool-starvation burst into one DB execution per (slug,locale) |
| info-row cache (in-process) | fresh 30s (shares `MENU_CACHE_TTL_MS`) | 30s | same starvation-guard rationale as the menu cache |

**Rust porting note on the two-tier menu cache:** `menu.ts`'s in-process `Map`-based cache (fresh/stale/stale-on-error, FIFO-evicted at 500 entries, keyed `slug::locale`) is the load-bearing fix for a real production incident (pool starvation → blank storefront). Port it as a per-instance `moka` or hand-rolled `DashMap`-based cache with the same three-tier freshness semantics — do not drop it as "just add an HTTP cache header," since the HTTP `Cache-Control` header and the app-level cache solve **different** problems (edge/browser caching vs. protecting the operational DB pool from a request burst).


---

<!-- ============ §2 WEBSOCKET PROTOCOL ============ -->

# WebSocket Protocol Census — Rust Rebuild Map Input

Scope: `apps/api/src/websocket.ts` (531 lines, read in full), `apps/api/src/bootstrap/messaging.ts`,
`apps/api/src/lib/courier-room-authz.ts`, `apps/api/src/lib/courier-relay-guard.ts`,
`packages/platform/src/message-bus.ts`, `apps/api/src/lib/registry.ts`, `apps/web/src/lib/useWebSocket.ts`
+ 4 FE consumer pages, `apps/api/src/client/status/ws.ts` (orphan client), ADR-0013 addendum draft +
escalation doc, `docs/regressions/REGRESSION-LEDGER.md` rows 5/7/39/40/42, e2e specs, unit test files.

Target: axum WebSocket + tokio broadcast + sqlx PgListener. Tri-state authz port = 🔴 council-gated.

---

## 1. Connection & auth

- **Endpoint / upgrade mechanism**: NOT a fastify WS plugin. `apps/api/src/websocket.ts:192` —
  `const wss = new WebSocketServer({ server: fastify.server })` (raw `ws` package, no `path` option).
  Because no `path` is given, `ws` intercepts **every** HTTP upgrade request on the shared HTTP server —
  there is no path-based routing at the WS layer; a client can upgrade at any URL (`/ws`, `/ws/orders/:id`,
  literally anything) and lands in the same `wss.on('connection', ...)` handler
  (`apps/api/src/websocket.ts:329`). Wired in `apps/api/src/server.ts:857` inside `fastify.ready()`, after
  `fastify.printRoutes()`. Graceful shutdown: `server.ts:860-867` (`onClose` hook) closes every client with
  code `1012` ("Server restarting") then `wss.close()`. Health metric: `server.ts:266` —
  `fastify.wss.clients.size`.
- **FE connect URL**: `apps/web/src/lib/useWebSocket.ts:6` —
  `VITE_WS_BASE_URL || ${protocol}//${host}/ws` (`wss:` iff `location.protocol === 'https:'`).
- **Auth handshake — DUAL path, both live today** (extraction: `grep -n "urlToken\|msg.type === 'auth'" apps/api/src/websocket.ts`):
  1. **URL query-param** (`?token=`): `websocket.ts:338-355`. Parsed on `connection`, `verifyAuthToken(urlToken)`
     resolved async into `authPromise`; on success sends `{type:'auth_success', role}`, calls
     `logTokenDeprecation()` (`websocket.ts:179-181`, `#5`) + `logAuthSuccess('url', ip)` (`websocket.ts:187-189`).
  2. **First-message auth**: `websocket.ts:373-385` — if not yet authenticated, first inbound message must be
     `{type:'auth', token}`; `verifyAuthToken(msg.token)`, same `auth_success` reply + `logAuthSuccess('message', ip)`.
  3. Every subsequent message handler starts with `if (authPromise) { await authPromise; authPromise = null; }`
     (`websocket.ts:368-371`) — serializes message processing behind any in-flight URL-auth so a
     pipelined `subscribe` sent immediately after open can't race ahead of auth.
  4. **`authTimeout`**: 5000ms (`websocket.ts:357-362`) — `ws.close(1008, 'Authentication timeout')` if not
     authenticated within 5s of connect. This is the "5s auth budget" the ws-token-in-url escalation cites
     as a constraint on any auth-channel redesign.
- **FE reality vs. "deprecation" framing (finding)**: `useWebSocket.ts:48-64` — the client **always** sets
  `?token=` on the URL (`url.searchParams.set('token', token)` at line 50) whenever a token exists in
  storage, **and** on `onopen` also sends `{type:'auth', token}` as a message. Since the URL-token
  `authPromise` almost always resolves before the message arrives (message processing awaits it), the
  explicit `auth` message is a dead branch on the server (falls through every `if` to
  `console.warn('[WS] Unknown message type...', 'auth')`, `websocket.ts:491`) — i.e. **in practice the FE
  is on the URL-token path today**, not the message-auth path; `?token=` is NOT actually deprecated in
  enforcement, only usage-logged (`logTokenDeprecation`, role-only, never the token/sub).
  Anonymous flow: if no stored token, `onopen` sends `subscribe` directly with no auth — this only works
  for rooms that don't require `isAuthenticated` server-side... but server-side EVERY message path requires
  `isAuthenticated` (falls to the `!isAuthenticated` branch and closes 1008 unless `msg.type==='auth'`).
  So an unauthenticated client sending `subscribe` first is rejected — customer/guest tracking still
  requires a token (issued to the customer at order-creation, presumably a short-lived per-order JWT).
- **`?token=` deprecation state**: **PROPOSED, not built.** `docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md`
  status = "PROPOSED — awaiting operator approval"; R1 (Sec-WebSocket-Protocol subprotocol dual-accept) is
  not yet implemented. Only the P1 mitigation shipped: `redactUrlSecrets()` strips `?token=` from the Pino
  `req.url` serializer (ledger row 42, `apps/api/src/lib/logger.ts`) — the log-leak is closed, but the
  URL-token **auth channel itself** (still visible in browser devtools/history) is unchanged and dual-accepted.
  A second, **currently-orphaned** WS client exists purely on the URL-token path with no message fallback:
  `apps/api/src/client/status/ws.ts` (`StatusWSClient`, connects to `/ws/orders/:id?token=...`) — grep
  confirms **zero import sites** anywhere in `apps/api` or `apps/web` (`grep -rn "StatusWSClient" apps` →
  only the class definition itself). It is referenced by the escalation doc as "the order-status widget"
  but is not currently wired into any route/page — flag as dead-or-unmounted code, not a live protocol
  surface, though the addendum's R1/R2 plan treats it as a client to migrate.
- **Per-connection state** (`websocket.ts:329-336`): `isAuthenticated: boolean`, `user: AuthToken | null`,
  `authPromise: Promise<void> | null`, `clientIp` (`x-forwarded-for` or `socket.remoteAddress`),
  `(ws as any).isAlive` (heartbeat flag, seeded `true` at connect). Global maps: `rooms: Map<room, Set<RoomMember>>`,
  `userBySocket: Map<WebSocket, AuthToken>`, `roomHandlers: Map<room, handler>` (one messageBus subscription
  per room, tracked so it can be unsubscribed — see §4/§5).

## 2. Inbound message census (client → server)

Extraction: `grep -nE "msg\.type ===" apps/api/src/websocket.ts` → **5 matches** (auth handled separately
above as it's pre-authentication; counting all `msg.type` branches gives 5 distinct types after auth: not
counting `auth` itself as a "post-auth" type, but it IS a distinct wire message type — **6 total distinct
`type` values the server parses**, listed below).

| type | payload | authz check | handler file:line | Rust target | proof |
|---|---|---|---|---|---|
| `auth` | `{type:'auth', token: string}` | none (this establishes identity) — `verifyAuthToken(msg.token)` | `websocket.ts:374-384` | `protocol::ClientMsg::Auth{token}` in the connection-init state machine before the room-registry loop starts | `apps/api/tests/websocket-authz.test.ts` (indirect, via owner-verdict tests), `e2e/tests/cross-tenant-realtime-qa.spec.ts` |
| `subscribe` | `{type:'subscribe', room: string}` | role-branched: customer → exact match `order:${user.orderId}`; owner → `location:*`/`order:*` prefix + `ownerCanAccessRoom` (🔴 tri-state, `ownerRoomVerdict`); courier → `courier:${sub}` exact match OR `order:*` + `courierRoomVerdict` (🔴 ADR-0013 tri-state); anything else denied | `websocket.ts:388-438` | `rooms::subscribe(room, member)` behind an `authz::verdict(room, &user)` gate returning the same tri-state enum | `apps/api/tests/courier-room-authz.test.ts` (12/12), `e2e/tests/courier-room-authz-isolation.spec.ts` |
| `unsubscribe` | `{type:'unsubscribe', room: string}` | none (any member of a room may remove themselves — no cross-member unsubscribe possible, keyed by `m.ws === ws`) | `websocket.ts:440-456` | `rooms::unsubscribe(room, conn_id)` | needs-new (no dedicated unsubscribe e2e found; covered incidentally by `useWebSocket.ts` unmount cleanup, not asserted server-side) |
| `client_location` | `{type:'client_location', payload:{lat:number, lng:number}}` — range-validated (`-90..=90`, `-180..=180`) | `user.role === 'customer'` only; relayed to courier members of `order:${user.orderId}` **through the courier relay guard** (tri-state revalidation, not raw send) | `websocket.ts:458-476` | `protocol::ClientMsg::ClientLocation{lat,lng}` → `fanout::relay_to_couriers(order_id, frame)` reusing the same guard as bus events | `e2e/tests/courier-room-authz-isolation.spec.ts` ("POSITIVE CONTROL — courier-1 BOUND... receives the guarded customer-GPS relay") |
| `client_location_stop` | `{type:'client_location_stop'}` | `user.role === 'customer'` only; same guarded relay path | `websocket.ts:478-489` | same as above | covered by same spec (implicitly; no dedicated stop-frame assertion found — flag needs-new for an explicit stop-frame test) |
| *(unrecognized)* | any other `type` | n/a — logged, silently ignored | `websocket.ts:491` `console.warn('[WS] Unknown message type from:', ...)` | serde-tagged enum with `#[serde(other)]` fallback → log + no-op, never an error frame | n/a |

Count reconciliation: grep found 5 `msg.type ===` comparisons (subscribe/unsubscribe/client_location/
client_location_stop are inside the post-auth block; `auth` is checked via `msg.type === 'auth'` too but
sits inside the `!isAuthenticated` branch at line 374, which the `msg.type ===` grep DOES catch — so the
grep's 5 lines are: 374 (auth), 388 (subscribe), 440 (unsubscribe), 458 (client_location), 478
(client_location_stop) = **5 lines, 5 distinct types**, matching the table above (6th row is the
"anything else" catch-all, not a distinct wire type).

## 3. Outbound event census (server → client, over the bus/rooms)

Two genuinely distinct namespaces that must **not** be conflated:

**(A) `BUS_CHANNELS.*`** (`apps/api/src/lib/registry.ts:1-45`) — internal message-bus topic names (dotted,
e.g. `order.created`, `courier.stale_heartbeat`). Extraction: `grep -roE "messageBus\.publish\(BUS_CHANNELS\.[A-Z_]+" apps/api/src --include='*.ts' | sed 's/.*BUS_CHANNELS\.//' | sort -u` → **34 distinct channels**.
Most of these are consumed **only** by non-WS subscribers — `apps/api/src/bootstrap/messaging.ts`
(Telegram/pg-boss fan-out via `registerNotifySubscriptions`), `courier-events.ts` (re-emits onto room
channels, see B below), workers doing internal bookkeeping. **None of these 34 channel names are
themselves WS room names** — a browser never does `subscribe` to `order.created`; it subscribes to a
room (`order:<id>`, `location:<id>:dashboard`, etc.) and receives a re-published, room-scoped event whose
`type` field may or may not reuse the same string (e.g. `order.status`, `order.created` DO get re-emitted
as in-frame `type` values onto `dashboardChannel`/`orderChannel` — see B — but `WORKER_FAILED`,
`BACKUP_FAILED`, `LIVENESS_CHECK_FAILED`, `OTP_SENT/VERIFIED`, `SETTLEMENT_*`, `MENU_*`, `SHIFT_STARTED`
(the raw channel, not the `courier.shift_updated` room re-emit), `ALERT_WORKER_LIVENESS`, `CUSTOMER_*`,
`ANONYMIZER_GDPR_FAILED` are **pg-boss/Telegram-only, never reach a browser**).

**(B) Room-targeted frame `type` values** — the actual WS wire events. Extraction:
`grep -roE "type:\s*'[a-zA-Z_.]+'" apps/api/src --include='*.ts'` filtered to publish calls targeting
`orderChannel()`/`dashboardChannel()`/`courierChannel()`/`shiftChannel()`/raw `` `courier:${id}` ``/
`` `location:${id}:dashboard` `` strings, **plus** the two synthetic eviction notices the relay guards send
directly (not via the bus). Distinct count: **24** (list below) + the WS-protocol-level frames
(`auth_success`, `subscribed`, `error`) which are connection-management, not domain events.

| event `type` | emitted from (file:line) | room/channel | payload summary | consumed by (FE) | Rust target (tokio broadcast topic) |
|---|---|---|---|---|---|
| `order.created` | `routes/orders.ts:611` (as `order.status` on `order:<id>`!) and `:624` (as `order.created` on `dashboardChannel`, `data`-wrapped) | `order:<id>` (as `order.status`), `location:<id>:dashboard` (as `order.created`) | claim-check: id/status/total/currency/timestamp — **zero PII** (P0-3) | `DashboardPage.tsx:141` (`order.created` on dashboard room) | `topic::location_dashboard(loc_id)` |
| `order.status` | `orderStatusService.ts:261` (flat, `order:<id>`), `:278` (`data`-wrapped, `dashboardChannel`), `owner/dashboard.ts:363,427`, `order-timeout-sweep.ts:96,99`, `orders.ts:612` | `order:<id>` (flat shape) **and** `location:<id>:dashboard` (data-wrapped shape — different envelope per target room, same type string) | `orderId/status/locationId/timestamp` (+`statusAtField/statusAt` on the order-room flat variant) | `OrderStatusPage.tsx:267` (order room), `DashboardPage.tsx:149` (dashboard room, via `mergeDelta`) | `protocol::Event::OrderStatus{..}` — needs ONE canonical envelope; today two shapes alias the same `type` |
| `order.route` | `workers/courier-events.ts:180` (`publishRouteOnce`) | `order:<id>` | `{orderId, polyline, durationSeconds, distanceMeters}` — claim-checked once per leg via `claimOnce` (Redis, cross-instance dedup) | `OrderStatusPage.tsx:239` (draws polyline) | `topic::order(order_id)` |
| `order.courier_updated` | `courier-events.ts:206` (position tick), `:236` (assignment transition) | `order:<id>` | `{orderId, courierName (masked), phoneMasked, position, status}` — **D1: no single-number ETA**, PII pre-masked server-side | `OrderStatusPage.tsx:247`, `DeliveryPage.tsx` (implicitly via courier's own `order:<id>` room — see below) | same topic as order.status |
| `order.message` | `routes/order-messages.ts:114` | `order:<id>` | `{id, ...}` chat message row | `OrderStatusPage.tsx:290`, `DeliveryPage.tsx:165` | same topic |
| `binding_changed` | `routes/courier/assignments.ts:465,521` (abort/decline) | `order:<id>` | `{orderId}` — **deliberately carries NO `courierId`** (ledger row 40: this is *why* event-eviction can't target the evictee, forcing fan-out-time revalidation instead) | none directly — it's a "something changed, don't trust cache" signal; superseded by the relay guard's `binding_revoked` for the actual evictee | `protocol::Event::BindingChanged{order_id}` |
| `assignment_aborted` | `courier/assignments.ts:466,522` | `location:<id>:dashboard` | `{orderId}` | owner dashboard (assignment list refresh) | dashboard topic |
| `offer_declined` | `courier/assignments.ts:564` | `location:<id>:dashboard` | `{orderId}` | owner dashboard | dashboard topic |
| `offer_sent` | `owner/dashboard.ts:337` | `location:<id>:dashboard` | `{orderId}` | owner dashboard | dashboard topic |
| `offer_expired` | `courier-offer-sweep.ts:100` | `location:<id>:dashboard` | `{orderId}` | owner dashboard | dashboard topic |
| `assignment_expired` | `courier-offer-sweep.ts:150` | `location:<id>:dashboard` | `{orderId}` | owner dashboard | dashboard topic |
| `assignment.created` | `lib/dispatch.ts:54`, `server.ts:692`, `dev/mock-auth.ts:163` | `location:<id>:dashboard` | `{orderId, courierId}` | owner dashboard | dashboard topic |
| `task_assigned` | `lib/dispatch.ts:55`, `server.ts:688`, `dev/mock-auth.ts:168`, `owner/dashboard.ts:365` | `courier:<courierId>` | `{payload:{id, orderId, status:'assigned', courierId}}` | `TasksPage.tsx:89` (new task card + ping sound) | `topic::courier(courier_id)` |
| `task_offered` | `owner/dashboard.ts:336` | `courier:<courierId>` | `{id, orderId, assignmentId, courierId}` | courier task inbox (offer-handshake variant) | courier topic |
| `courier.position_updated` | `courier-events.ts:163` (also a `BUS_CHANNELS` internal name, re-emitted onto the room) | `location:<id>:couriers` | `{courierId, position:{lat,lng}}` | `DashboardPage.tsx:169` (live map pin) | `topic::location_couriers(loc_id)` |
| `courier.assignment_status_changed` | `courier-events.ts:230` | `location:<id>:couriers` | `{courierId, orderId, status}` | owner live map (status badge) | couriers topic |
| `courier.shift_updated` | `routes/courier/shifts.ts:94,156,241,288` (start/close/2 transition paths) | `location:<id>:couriers` | `{courierId, status:'available'\|'offline'}` | `DashboardPage.tsx:171` (drops offline courier pin) | couriers topic |
| `dwell.alert_created` | `workers/dwell-monitor.ts:99` | `location:<id>:dashboard` | `data:{alertId, orderId, kind, dwellSeconds, severity}` | owner dashboard alert banner | dashboard topic |
| `dwell.alert_acknowledged` | `owner/alerts.ts:142,182` | `location:<id>:dashboard` | ack payload | owner dashboard | dashboard topic |
| `dwell.escalation_tier_changed` | `dwell-escalation.ts:83,102` (via `publishEvent` helper) | `location:<id>:dashboard` | `data:{alertId, orderId, kind, tier}` | owner dashboard | dashboard topic |
| `preflight.signal_raised` | `signal-raiser.ts:113` | `location:<id>:dashboard` | signal payload | owner dashboard | dashboard topic |
| `preflight.signal_acknowledged` / `preflight.signal_dismissed` | `owner/signals.ts:159,190` | `location:<id>:dashboard` | ack/dismiss payload | owner dashboard | dashboard topic |
| `customer.contact_revealed` | `owner/reveal-contact.ts:64` (raw string `` `location:${id}:dashboard` ``, not the `dashboardChannel()` helper — same string, different construction site) | `location:<id>:dashboard` | reveal audit payload | owner dashboard | dashboard topic |
| `gdpr.erasure_completed` | `workers/anonymizer-gdpr.ts:134` | `location:<id>:dashboard` | erasure confirmation | owner dashboard (compliance banner, if wired) | dashboard topic |
| `membership_revoked` | `websocket.ts:280` — **not via messageBus.publish; a direct `member.ws.send` from `createOwnerRelayGuard`'s `evict()`** | direct-to-evictee, any `location:`/`order:` room | `{type:'error', error:'membership_revoked'}` (note: wire-level `type` is `'error'`, `error` field carries the reason string) | no dedicated FE handler found — falls into generic `msg.type==='error'` handling (`OrderStatusPage.tsx:232` returns early on any `error` type without distinguishing reason) | `protocol::Event::Evicted{reason}` — flag: FE should surface reason-specific UX, doesn't today |
| `binding_revoked` | `websocket.ts:260` — same pattern, `createCourierRelayGuard`'s `evict()` | direct-to-evictee | `{type:'error', error:'binding_revoked'}` | same generic `error` handling | same as above |

**Count reconciliation**: 24 distinct in-frame `type` values found via targeted grep +
manual read of all 25 `messageBus.publish(...)` call sites feeding room channels (dashboardChannel×13,
orderChannel×9, courierChannel×6, shiftChannel×1, raw `` `courier:${id}` ``×3, raw
`` `location:${id}:dashboard` ``×1 — 33 call sites, fewer distinct types because several call sites reuse
the same `type` string, e.g. `order.status` appears at 5 call sites, `courier.shift_updated` at 4).
`shiftChannel()` (`courier:<id>:shift`) is published to (`shiftService.ts:60`, `type:'shift.opened'`) but
**grep of `apps/web/src` found zero FE subscription to any `courier:<id>:shift` room** — this channel
currently has no WS consumer (dead-on-the-wire; either an orphaned code path or a channel intended for a
not-yet-built shift-history widget). Flag as a protocol surface that could not be fully pinned down to a
live consumer.

Separately, `ops.reconciliation_drift` and `ops:order_timeout_lag` (`order-timeout-sweep.ts:56,160`,
`reconciliation.ts:103`) are published as raw string channels but have **no room-authz gate and no FE
consumer** — these are operational/internal telemetry channels riding the same `MessageBus.publish` API,
not part of the customer-facing WS protocol; exclude from the outbound census proper but note them as
"ops-only bus traffic sharing the transport."

## 4. Rooms/channels model

Naming scheme (verified against both server-side authz branches and FE `room:` construction —
`apps/web/src/pages/{admin/DashboardPage,client/OrderStatusPage,courier/{TasksPage,DeliveryPage}}.tsx`):

| room pattern | who joins | who authorizes | FE constructor |
|---|---|---|---|
| `order:<orderId>` | customer (own order only), owner (via order→location join), courier (bound only) | see below | `OrderStatusPage.tsx:227`, `DeliveryPage.tsx:153` |
| `location:<locationId>:dashboard` | owner only | `ownerRoomVerdict` (🔴) | `DashboardPage.tsx:136` |
| `location:<locationId>:couriers` | owner only | `ownerRoomVerdict` (🔴, same predicate — `.startsWith('location:')` matches both dashboard/couriers suffixes since the verdict fn does `room.split(':')[1]`, suffix-agnostic) | `DashboardPage.tsx:163` |
| `courier:<courierSub>` | courier, own task-feed only | exact string match `room !== \`courier:${user.sub}\`` (`websocket.ts:411`) — **no DB check**, identity-only | `TasksPage.tsx:86` |
| `courier:<courierId>:shift` | *(nobody — no FE subscriber found)* | n/a | none found |

**Join authz per role** (`websocket.ts:392-433`):
- **customer**: hard-pinned to exactly `order:${user.orderId}` (the JWT's own bound order id) — any other
  room string, including a different `order:<id>`, is `Forbidden room`. Zero DB query (JWT-derived).
- **owner**: room must start with `location:` or `order:`; then 🔴 `ownerCanAccessRoom` →
  `ownerRoomVerdict(fastify.db, ownerId, room)` (`websocket.ts:35-63`) — tri-state:
  - `location:<locId>` → `SELECT 1 FROM memberships WHERE user_id=$1 AND location_id=$2 AND role='owner' AND status='active'`
  - `order:<orderId>` → `SELECT 1 FROM orders o JOIN memberships m ON m.location_id=o.location_id WHERE o.id=$1 AND m.user_id=$2 AND m.role='owner' AND m.status='active'`
  - 0 rows → `DENY`; query throw → `UNAVAILABLE`; **both map to subscribe-refusal** (`ownerCanAccessRoom`
    only admits on a live `ALLOW`, `websocket.ts:316-322`) — subscribe fails closed on either negative.
  - `status='active'` in both branches is the **#4 fix** (ADR-0004 revocation mirror) — the order-room
    query previously omitted it, letting a de-activated owner still subscribe (`websocket.ts:32-33`
    comment documents this as the pre-fix gap).
- **courier** (🔴 **ADR-0013**): `room.startsWith('courier:')` → exact-self-match only (no DB);
  `room.startsWith('order:')` → 🔴 `courierRoomVerdict(fastify.db, sub, activeLocationId, room)`
  (`apps/api/src/lib/courier-room-authz.ts:85-91`) → delegates to `courierReadVerdict` →
  `courierBindingVerdict` (`courier-room-authz.ts:32-66`): opens an explicit `BEGIN`, sets
  `app.current_tenant` via `set_config(...)` (RLS-scoping — **NOBYPASSRLS-sound**, works whether or not
  the DB role currently has `BYPASSRLS`), then
  `SELECT 1 FROM courier_assignments WHERE order_id=$1 AND courier_id=$2 AND status = ANY($3) LIMIT 1`
  against `BINDING_READ_STATUSES = ['offered','assigned','accepted','picked_up']`, `COMMIT`s. Any other
  room (including bare `location:*`) → `DENY` with **zero DB access** (`courier-room-authz.ts:85-91`).
  - Tri-state mapped at the WS layer (`websocket.ts:416-427`): `UNAVAILABLE` → `{type:'error', error:'Service temporarily unavailable', retryable:true}` — socket stays OPEN, no `ws.close` (a DB blip must never fleet-deny/reconnect-storm couriers, Breaker H1/NEW-A). `DENY` → `Forbidden room`, also no close.
- Both owner and courier gates are **fail-closed on ambiguity**: courier `UNAVAILABLE` is retryable-soft at
  subscribe (keeps socket open) but the underlying REST wrappers (`courierCanReadOrder`/`courierCanSendOrder`,
  `courier-room-authz.ts:98-106`) collapse `UNAVAILABLE`→`false` (404) — the retryable distinction is
  WS-only, intentionally (`courier-room-authz.ts:94-96` comment).

**Fan-out re-authz (the "admission isn't enough" half of ADR-0013, 🔴)**: admission authorizes the
*subscribe*, but a member can be un-bound/de-activated *after* joining and — absent re-checking — would
keep receiving frames until disconnect (Breaker C1, ledger row 40). Two parallel guards close this, both
sharing the identical shape:
- `createCourierRelayGuard` (`apps/api/src/lib/courier-relay-guard.ts`) — gates every frame to every
  courier member of an `order:<O>` room (the bus handler at `websocket.ts:227-232`, plus the two direct
  `client_location`/`client_location_stop` relays at `websocket.ts:471,485`). Relay-only-on-fresh-ALLOW
  (10s absolute TTL, no refresh-on-read), `UNAVAILABLE` withholds without evicting until a **60s wall-clock
  ceiling since first UNAVAILABLE** (dominant) or `maxUnavail=120` count (secondary, tuned to fire after
  the wall at ~1Hz GPS rate) — the ceiling is **in-memory only**, holding even under total DB starvation.
  `DENY` → evict + `binding_revoked`.
- `createOwnerRelayGuard` (`websocket.ts:102-171`, `#4`) — the owner mirror, added because the courier
  guard originally re-validated ONLY couriers, so a revoked owner kept streaming `order:`/`location:`
  frames until disconnect. Same absolute-TTL/no-refresh-on-read cache (10s default), `DENY` → evict +
  `membership_revoked`; `UNAVAILABLE` → withhold only, **no ceiling** (comment at `websocket.ts:96`
  explains: owners have no GPS-rate ceiling need — a stated, honest residual, "OR-9": eviction happens
  within ≤TTL, not literally zero-window).
- Non-courier/non-owner members (and non-order/non-owner rooms) are relayed **directly**, no revalidation
  — admission is treated as authoritative for those paths (`courier-relay-guard.ts:112-119`).
- **Drift guardrail**: `local/no-raw-courier-ws-send` ESLint rule (ledger row 40) bans any raw
  `member.ws.send` at a courier-joinable fan-out site outside the guard — keyed on the SITE, not
  `role==='courier'`, specifically so a future 4th raw-send site can't reopen the leak.

## 5. Multi-instance fan-out (Pg LISTEN/NOTIFY)

`packages/platform/src/message-bus.ts` — `PgMessageBus` (aliased as `RedisMessageBus`, i.e. **Redis is not
actually used**; the export name is a historical/compat alias — `message-bus.ts:243`).

- **Transport**: one dedicated `listenerClient` (`pool.connect()`, held open) issues `LISTEN "<channel>"`
  per distinct channel on first subscriber (`message-bus.ts:190-199`); `publish()` always `NOTIFY`s on the
  **shared pool**, never the listener client (`message-bus.ts:118-121` comment: doing so previously
  serialized every publish onto one connection and raced the reconnect logic).
- **8KB claim-check**: `MAX_NOTIFY_BYTES = 7800` (`message-bus.ts:23`, margin under Postgres's hard 8000-byte
  NOTIFY payload cap). `serializeForNotify()` (`message-bus.ts:140-154`): if the JSON exceeds 7800 bytes,
  emits a slimmed frame `{_truncated:true, type, data:{id, _truncated:true}}` — preserving only `type` and
  `data.id ?? data.order_id ?? data.orderId` — and logs a warning. **Consumer-side re-fetch semantics**:
  grep confirms no dedicated `_truncated` handler in `websocket.ts` or the FE — the truncated frame still
  carries the real `type`, so existing FE handlers that only read `type` + a top-level id (e.g.
  `order.status` handlers that call `fetchOrder()`/`fetchOrders()` on receipt regardless of payload
  completeness — `OrderStatusPage.tsx:287`, `DashboardPage.tsx:143`) incidentally self-heal via their
  existing authenticated refetch calls. **No explicit `_truncated`-aware branch exists anywhere** — this is
  an implicit, not designed, recovery path. Flag for the Rust rebuild: make the refetch-on-truncation
  behavior explicit rather than relying on incidental refetch timers.
- **Reconnect**: capped-exponential backoff (`1000 * 2^n`, max 30000ms), retried **indefinitely**
  (`message-bus.ts:101-114`) — a prior 5-attempt cap left an instance silently realtime-dead after Pg blips
  (comment at `message-bus.ts:95-100`); a single `reconnecting` flag prevents the `'error'` and `'end'`
  listener-client events from stacking overlapping timers. On reconnect, re-`LISTEN`s every
  previously-registered channel (`message-bus.ts:80-84`).
- **Handler isolation**: `dispatch()` (`message-bus.ts:168-181`) catches both sync throws and rejected
  promises per-handler so one bad subscriber can't crash the process (a documented prior incident: a
  handler referencing a non-existent column threw, took down the whole API for every tenant).
- **Payload logging**: channel name + byte length only, never the raw payload (`message-bus.ts:51`, P0-3).

## 6. Liveness

- **Heartbeat**: server-side `setInterval` every 30000ms (`websocket.ts:287-297`) — pings every client,
  terminates any whose `isAlive` flag is still `false` from the *previous* tick (i.e. a full missed
  ping/pong round-trip, ~30-60s to reap a zombie); `ws.on('pong', ...)` resets `isAlive=true`.
- **Room GC**: separate `setInterval` every 60000ms (`websocket.ts:300-309`) sweeps empty rooms (belt,
  since `deleteRoom` is also called eagerly on last-member-leave / disconnect — `websocket.ts:203-210,
  498-517` — P1-WSDUP fix, ledger row: re-creating a room after GC previously stacked a second
  `messageBus.subscribe` on the same channel, causing N× event delivery) and calls
  `relayGuard.sweep()`/`ownerRelayGuard.sweep()` (drop expired ALLOW cache entries — heap hygiene only,
  eviction itself is frame-driven, not sweep-driven).
- **FE reconnect** (`useWebSocket.ts:86-113`): reconnects **forever**, capped exponential backoff
  (`2000 * 1.5^n`, max 15000ms) + up to 1000ms jitter. Explicitly does NOT give up (comment: giving up after
  N tries left dashboards silently stale after any blip until manual reload).
- **Resubscribe on reconnect**: `useWebSocket.ts` re-sends `{type:'subscribe', room}` both on raw `onopen`
  (if no token — anonymous) and on receiving `auth_success` (`useWebSocket.ts:70-76`) — i.e. full
  re-subscribe after every reconnect, no session resumption.
  **`onReconnect` callback** in every consumer (`DashboardPage.tsx:155-158`, `TasksPage.tsx:100`,
  `OrderStatusPage.tsx:297-299`, implicitly `DeliveryPage.tsx`) triggers an authenticated REST refetch
  (`fetchOrders`/`fetchTasks`/`fetchOrder`) — **this is the missed-event-recovery mechanism**: there is
  **no server-side replay/backlog/sequence-number** for frames missed while disconnected; recovery is
  entirely "reconnect → resubscribe → client refetches full state via REST." Confirms: no missed-event
  queue, no `Last-Event-ID`-style resumption token exists in this protocol.
- **Unmount**: `useWebSocket.ts:154-161` sends `{type:'unsubscribe', room}` (best-effort, swallowed on
  failure) then `ws.close(1000, 'unmount')` — clean close, no reconnect.

## 7. Rust mapping proposal

```
crates/api/src/ws/
  mod.rs        — axum WebSocket upgrade handler, connection task spawn, per-conn actor loop
  auth.rs       — 🔴 dual-channel auth: URL ?token= (legacy, log-redacted) + first-message {type:"auth"};
                  port verifyAuthToken; DO NOT silently drop the URL path without an operator-approved
                  ADR-0013-addendum R2 (see §1) — currently PROPOSED not ACCEPTED
  protocol.rs   — serde-tagged enums:
                    ClientMsg: Auth{token} | Subscribe{room} | Unsubscribe{room}
                             | ClientLocation{lat,lng} | ClientLocationStop
                    ServerMsg: AuthSuccess{role} | Subscribed{room} | Error{error, retryable: bool}
                             | Event{room, data: EventPayload}   // envelope, mirrors {room, data: msg}
                  EventPayload as a second tagged enum for the 24 in-frame `type`s in §3(B) — collapse
                  the flat-vs-data-wrapped inconsistency (order:<id> gets flat order.status, dashboard
                  gets data-wrapped order.status — same Rust variant, one canonical shape)
  rooms.rs      — Room registry: HashMap<RoomId, HashSet<ConnId>>, RoomId enum {Order(Uuid), Location
                  Dashboard(Uuid), LocationCouriers(Uuid), Courier(Uuid)} — NOT stringly-typed, to kill the
                  raw-string-room class of bugs (courier-shift room orphan in §3 would be caught at
                  compile time by an unused-variant lint)
  authz.rs      — 🔴 tri-state AuthzVerdict{Allow, Deny, Unavailable}; port ownerRoomVerdict +
                  courierRoomVerdict as the SAME trait (RoomAuthz) so WS-subscribe, WS-fanout-revalidate,
                  and REST order-messages share one impl (mirrors the TS "one predicate, three call
                  sites" design intentionally — this is the load-bearing invariant, not incidental)
  fanout.rs     — 🔴 RelayGuard<T>: generic over {Courier, Owner} — bounded LRU allow-cache (10s TTL, no
                  refresh-on-read), ceiling-on-sustained-UNAVAILABLE (courier only, 60s wall / owner: none
                  — port the asymmetry deliberately, it's a documented design choice not an oversight),
                  eviction = drop from room + one {binding_revoked|membership_revoked} frame
  pg_listener.rs — sqlx::postgres::PgListener wrapping LISTEN/NOTIFY, 7800-byte NOTIFY cap enforcement +
                  explicit _truncated envelope (make this a first-class variant, not incidental — see §5),
                  indefinite reconnect w/ capped backoff, per-handler panic/error isolation (tokio::spawn
                  + catch_unwind or Result-returning handlers, mirroring dispatch()'s per-handler isolation)
  heartbeat.rs  — 30s ping/pong reaper (tokio::time::interval + last-pong Instant per conn)
```

**🔴 Council-gated items** (touch the ADR-0013 tri-state authz surface or an unresolved auth-channel decision):
1. `authz.rs` (owner + courier tri-state room verdicts) — the core ADR-0013 port.
2. `fanout.rs` (both relay guards) — the fan-out re-authz invariant (Breaker C1 closure).
3. `auth.rs` — the `?token=` vs subprotocol decision is **already gated in TS** (ADR-0013-addendum-DRAFT,
   status PROPOSED) — the Rust port inherits the same gate; do not resolve it unilaterally during rebuild.
4. Any redesign of the room-naming scheme (RoomId enum) that changes wire-compatible room strings needs
   sign-off since FE `useWebSocket.ts` room construction (`order:${id}`, `location:${id}:dashboard`, etc.)
   is a cross-repo contract, not internal to the WS module.

**Per-item proof coverage**:
- Owner tri-state + fan-out: `apps/api/tests/websocket-authz.test.ts` (13 tests, `#4` prefix).
- Courier tri-state: `apps/api/tests/courier-room-authz.test.ts` (12/12).
- Courier fan-out guard: `apps/api/tests/courier-relay-guard.test.ts` (13/13).
- P1-WSDUP (room GC / duplicate-subscription): `apps/api/tests/websocket-churn.test.ts`.
- Cross-tenant/cross-role isolation, live staging: `e2e/tests/courier-room-authz-isolation.spec.ts` (7 tests:
  positive-control GPS relay, real `/courier/delivery/:id` UI+WS, `location:*` denial, N3 zero-frames WS +
  404 REST, **C1 reassign-eviction** live test).
- Full-stack realtime delta correctness: `e2e/tests/cross-tenant-realtime-qa.spec.ts` (8 tests, 3 roles).
- Channel/wrapping regression (the specific bug this test was written for): `e2e/tests/ws-courier-assignment.spec.ts`.
- **needs-new**: explicit `unsubscribe` server-behavior assertion; explicit `client_location_stop` frame
  assertion (currently only covered incidentally); `_truncated` NOTIFY-overflow consumer behavior (no test
  found at all — the recovery path is implicit/incidental, see §5); `shiftChannel`/`courier:<id>:shift`
  has no test because it has no consumer — needs either a consumer + test, or removal.

---

## Summary counts

- **Inbound message types**: 5 distinct `msg.type` branches post-connection (`auth`, `subscribe`,
  `unsubscribe`, `client_location`, `client_location_stop`) — `grep -nE "msg\.type ===" apps/api/src/websocket.ts` → 5 lines.
- **Outbound event `type`s reaching a WS room**: 24 distinct in-frame values (§3B table), fed by 33
  `messageBus.publish(...)` call sites into 4 room-channel constructors + 2 raw-string room targets, plus 2
  synthetic non-bus eviction notices (`membership_revoked`, `binding_revoked`) sent directly by the relay
  guards. Separately, 34 `BUS_CHANNELS.*` internal topic names exist (§3A) — mostly non-WS (Telegram/pg-boss),
  NOT conflated with the 24 above (some names, like `order.status`/`order.created`, are reused as both an
  internal bus channel name AND a room-frame `type` string — same string, different transport role).
- **Room kinds**: 4 live (`order:<id>`, `location:<id>:dashboard`, `location:<id>:couriers`,
  `courier:<sub>`) + 1 published-but-unconsumed (`courier:<id>:shift`) = 5 distinct patterns.
- **🔴 items**: 4 (owner tri-state verdict + fan-out guard; courier tri-state verdict + fan-out guard,
  ADR-0013; the gated `?token=`→subprotocol auth-channel migration, ADR-0013-addendum PROPOSED; any
  RoomId/wire-string redesign since FE room construction is a cross-repo contract).
- **Protocol surfaces not fully pinned down**:
  1. `courier:<id>:shift` / `shiftChannel()` has a publisher (`shiftService.ts:60`) but zero found FE
     consumer — orphaned channel, unclear if intentional (future feature) or dead code.
  2. `StatusWSClient` (`apps/api/src/client/status/ws.ts`) — zero import sites found anywhere in the repo;
     the ws-token-in-url escalation doc treats it as a live client requiring migration, but it appears
     unmounted today. Could not confirm whether it's genuinely dead or intended-but-not-yet-wired.
  3. `_truncated` NOTIFY-overflow frames have no explicit consumer-side handling — recovery is incidental
     (piggybacks on unrelated refetch timers), not a designed contract; no test exercises this path.
  4. The exact per-order customer JWT issuance (how a guest customer gets a token scoped to
     `user.orderId` for the WS auth) lives outside `websocket.ts`/`messaging.ts` (likely
     `packages/platform/src/auth/jwt.ts` / `routes/customer/orders.ts`) — out of grep scope for this pass,
     flagging as a boundary this census didn't chase down since it's issuance, not the WS protocol itself.


---

<!-- ============ §3 JOBS / QUEUES / CRON ============ -->

# JOBS / QUEUES / CRON Census — pg-boss inventory for Rust rebuild parity bar

READ-ONLY inventory. No code changed. Scope: `apps/api` (pg-boss v10 runtime via
`packages/platform`), `apps/worker` (separate Fly **process group**, same app), and the
notification sub-tree. All line numbers verified against current working tree
(`fix/audit-remediation` branch).

**Queue engine**: `pg-boss` — **v10.4.2 at runtime** (`packages/platform` pins `^10`,
instantiated in `packages/platform/src/queue-provider.ts`), but apps/api compiles its
worker files against **pg-boss `^12` types** (`apps/api`'s own `pg-boss` dep is v12).
This is a live, acknowledged version-skew bridged with `as unknown as PgBoss` casts in
`apps/api/src/bootstrap/workers.ts` (4 sites: lines 140, 155, 159, 169) — flagged in-repo
as a FLAG comment ("type-restore 2026-07-02"). **This skew itself is a fact the Rust
rebuild sidesteps entirely** (no npm package versioning problem in a from-scratch Rust
queue), but it proves the current census was extracted against a moving-types/fixed-
runtime target — verify behavior against v10 semantics specifically (that's what's
actually running), not v12 docs.

---

## 0. Top-line counts (see §7 for full reconciliation)

| Metric | Count |
|---|---|
| **Total distinct queue names** (`QUEUE_NAMES` const + 5 local ad-hoc constants) | **33** (28 in `packages/shared-types/src/queue-names.ts` + 5 local: `courier.offer_sweep`, `order.timeout_sweep`, `acquisition.retention-sweep`, `delivery-trace.retention-sweep`, `health-job`) |
| **Queues actually consumed** (`.work()` registered somewhere reachable from boot) | **30** |
| **Queues registered but dead** (never wired into `startBackgroundWorkers`) | **1** (`dwell.escalate`) |
| **Queue-name constants never used at all** | **2** (`ORDER_PENDING_AGING`, `SETTLEMENT_CRON` as a queue — see §7.4) |
| **Cron schedules** (`.schedule()` calls, excluding 1 comment false-positive) | **23** |
| **Transactional-enqueue sites** (pg-boss `send()` sharing the caller's DB transaction via `{ db }`) | **1 call-site file, 2 enqueues** (`apps/api/src/lib/order-persistence.ts:158-173`, inside the `orders.ts` POST /orders BEGIN…COMMIT) |
| **🔴 money / state-machine / GDPR / backup queues** | **9** (see §0.1) |
| Ambiguous literals requiring classification (Appendix) | **11** |

### 0.1 🔴 red-line queues (money / order state-machine sweep / GDPR / backup+DR)
`settlement.generate`, `reconciliation.nightly`, `order.timeout_sweep` (+ per-order
`order.timeout` in apps/worker), `courier.offer_sweep` (dispatch state-machine sweep),
`anonymizer.gdpr`, `anonymizer.retention`, `backup.hourly`/`backup.daily`/`backup.weekly`/
`backup.monthly` (counted as one 🔴 family), `backup.verify.restore`/`backup.verify.r2`
(one 🔴 family). Refund-due reconciliation lives *inside* `order.timeout_sweep`'s tick
(`app_reconcile_refund_due()`, §5), not as its own queue.

---

## 1. Queue census table

Legend: **W**=`.work()` registered, **S**=`.schedule()` cron, **E**=explicit enqueue site
(`.send`/`.enqueue`) beyond the worker's own retry-requeue, **singletonKey**=Y/N,
**DLQ**=deadLetter configured.

| # | Queue name (constant) | Handler file:line | Enqueued from (file:line) | Retry/backoff | singletonKey | DLQ | Cron (raw / effective default) | Concurrency | 🔴 | Rust target module | Proof |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `notify.dispatch` (`NOTIFY_DISPATCH`) | `apps/api/src/bootstrap/workers.ts:55` → `notifications/workers/index.ts:202` `handleDispatch` | `dwell-monitor.ts:127,136` (immediate+tier2 delay); `dwell-escalation.ts:135` (dead path, see §7.3); `notifications/workers/index.ts:254` (quiet-hours re-hold), `:336` (retry-on-fail); route `owner/notifications.ts:102` (`queue.boss.send('notify.dispatch', …)`, ad-hoc test-send) | pg-boss v10 default (retryLimit=2, delay=0s) — **not overridden** at queue-creation; in-handler retry uses `RetryPolicy.getDelay(attempt)` (custom backoff table, capped at `MAX_RETRIES`-ish via disable-on-exhaustion) | N (per-send `startAfter`, no key) | N | none (on-demand, event-triggered) | 1 (single job at a time per handler invocation; pg-boss default worker concurrency) | — | `crates/worker/notify_dispatch.rs` | `apps/api/tests` — 1 file hits `notify.dispatch` literal |
| 2 | `notify.customer_status` (`NOTIFY_CUSTOMER_STATUS`) | `bootstrap/workers.ts:56` → `notifications/workers/index.ts:99` `handleCustomerStatus` | `courier-offer-sweep.ts:264` (grace-cancel post-commit) | pg-boss default | N | N | none | 1 | 🔴 (touches order status) | `crates/worker/notify_customer_status.rs` | 1 test file |
| 3 | `notify.telegram.send` (`NOTIFY_TELEGRAM_SEND`) | `bootstrap/workers.ts:57` → `notifications/workers/index.ts:345` `handleTelegramSend` | `order-persistence.ts:165` (**transactional**, order.created), `order-timeout-sweep.ts:111`, `apps/worker/src/handlers.ts:66` (order.timeout_cancelled) | pg-boss default at queue level; **in-handler** re-throw with incremented `attempt` up to `MAX_RETRIES=10` then archives (no throw) | Y — `dedupKey` per call-site (`order.created:<id>:<loc>`, `order.timeout_cancelled:<id>:<loc>`) | N (archives to `notification_outbox_audit` status='archived' instead) | none | 1, per-chat rate-limit 1/1.2s + circuit-breaker (5 fail → 60s cooldown) | 🔴 (order lifecycle notifications) | `crates/worker/notify_telegram.rs` | 2 test files |
| 4 | `order.timeout` (`ORDER_TIMEOUT`) | `apps/worker/src/handlers.ts:15` | **`apps/api/src/lib/order-persistence.ts:158` (TRANSACTIONAL — see §2)** | pg-boss default | Y — `singletonKey: order.id` | N | one-shot `startAfter: timeoutAt` (not cron — per-order delayed job) | 1 (apps/worker process) | 🔴 order state machine | `crates/worker/order_timeout.rs` | 4 test files |
| 5 | `order.timeout_sweep` (local const, `SWEEP_QUEUE` in `order-timeout-sweep.ts:9`) | `order-timeout-sweep.ts:29` | self-scheduled only | pg-boss default | Y — `singletonKey: SWEEP_QUEUE` | N | `* * * * *` (hardcoded, not env-configurable) | 1 (advisory lock id=5, cross-instance singleton) | 🔴 safety-net sweep + refund-due reconciler (§5) | `crates/worker/order_timeout_sweep.rs` (tokio cron loop candidate, see §6) | 0 direct; covered indirectly via `dispatch-recovery.test.ts` |
| 6 | `courier.dispatch` (`COURIER_DISPATCH`) | `bootstrap/workers.ts:67` → `courier-dispatch.ts:23` `handleDispatch` | **only** `courier-offer-sweep.ts:173` (pump/drain pass) | pg-boss default (no self-retry — deliberately deleted, see file comment L9-14) | Y — `singletonKey: row.order_id` | N | none (pumped by `courier.offer_sweep` cron, not self-scheduled) | 1 | 🔴 dispatch state machine | `crates/worker/courier_dispatch.rs` | 5 test files |
| 7 | `courier.offer_sweep` (local const `SWEEP_QUEUE`, `courier-offer-sweep.ts:27`) | `courier-offer-sweep.ts:41` (4 passes: offer-expiry, accept-timeout, dispatch drain, grace-cancel) | self-scheduled | pg-boss default | Y — `singletonKey: SWEEP_QUEUE` | N | `* * * * *` (hardcoded, `SWEEP_CRON`) | 1 (advisory lock id=9) | 🔴 dispatch state machine + honest-cancel funnel | `crates/worker/courier_offer_sweep.rs` (tokio cron loop candidate) | 0 direct hits on literal (covered by dispatch-recovery tests indirectly) |
| 8 | `courier.stale_check` (`COURIER_STALE_CHECK`) | `courier-cron.ts:25` `handleStaleCheck` | self-scheduled | pg-boss default | Y | N | `*/2 * * * *` (hardcoded) | 1 | — | `crates/worker/courier_stale_check.rs` | 1 test file |
| 9 | `gps.purge` (`GPS_PURGE`) | `courier-cron.ts:20` `handleGpsPurge` | self-scheduled | pg-boss default | Y | N | `0 3 * * *` (hardcoded) | 1 | — | `crates/worker/gps_purge.rs` | 0 |
| 10 | `settlement.generate` (`SETTLEMENT_GENERATE`) | `settlement-cron.ts:18` `handleGenerate` | self-scheduled (also accepts `referenceDate` payload for manual re-runs) | pg-boss default; whole generation is one atomic `app_generate_settlements()` DB call — a thrown error aborts the entire sweep | Y — `singletonKey: QUEUE_NAMES.SETTLEMENT_GENERATE` | N | `env.SETTLEMENT_CRON \|\| '0 2 * * *'` | 1 | 🔴🔴 MONEY | `crates/worker/settlement_generate.rs` — 🔴 council money-math review before port | 2 test files |
| 11 | `dwell.monitor` (`DWELL_MONITOR`) | `dwell-monitor.ts:21` `run` | self-scheduled | pg-boss default | Y | N | `env.DWELL_CRON \|\| '* * * * *'` | 1 (advisory lock id=2) | — | `crates/worker/dwell_monitor.rs` | 1 test file |
| 12 | `dwell.escalate` (`QUEUE_NAMES.DWELL_ESCALATE` — **undefined constant, see §7.3**) | `dwell-escalation.ts:20` (class never instantiated) | **nowhere** — dead code | n/a | N | N | n/a (never scheduled) | n/a | — | **DO NOT PORT** — dead code; escalation actually flows through `notify.dispatch` directly from `dwell-monitor.ts` | 0 |
| 13 | `anonymizer.retention` (`ANONYMIZER_RETENTION`) | `anonymizer-retention.ts:22` `run` | self-scheduled | pg-boss default | Y | N | `env.ANONYMIZER_RETENTION_CRON \|\| '0 3 * * *'` | 1 (advisory lock id=4) | 🔴 GDPR retention | `crates/worker/anonymizer_retention.rs` | 2 test files |
| 14 | `anonymizer.gdpr` (`ANONYMIZER_GDPR`) | `anonymizer-gdpr.ts:17` `run` | **route** `apps/api/src/routes/owner/gdpr.ts:132` (owner-triggered erasure request) + self `anonymizer-gdpr.ts:156` (exponential-backoff retry, own re-send) | **in-handler** retry: up to 3 attempts, `2^retryCount * 60`s backoff, then `failed` | Y — `singletonKey: QUEUE_NAMES.ANONYMIZER_GDPR` (**note**: this dedups ALL erasure requests to ONE in-flight job globally — see Appendix A11) | N | **none** — on-demand only, not cron | batch of 10 (`LIMIT 10 FOR UPDATE SKIP LOCKED`) per invocation | 🔴🔴 GDPR Art.17 | `crates/worker/anonymizer_gdpr.rs` — 🔴 council | 3 test files |
| 15 | `velocity.flush` (`VELOCITY_FLUSH`) | `bootstrap/workers.ts:133` → `lib/signals/velocity-increment.ts:60` `handleFlush` | `velocity-increment.ts:54` (in-process 5s debounce buffer flush) | pg-boss default **overridden inline**: `retryLimit: 3` passed at send-time (not queue-creation) | Y — `singletonKey: 'velocity.flush'` (**note**: also dedups ALL locations to one job — buffer is batched, so this is intentional, not a bug) | N | none (debounce-triggered, not cron) | 1 | — | `crates/worker/velocity_flush.rs` (or fold into in-process tokio channel, see §6) | 0 |
| 16 | `free_tier.watch` (`FREE_TIER_WATCH`) | `bootstrap/workers.ts:174` (inline closure, not a class) | self-scheduled | pg-boss default | N (**no singletonKey** — only queue named `free_tier.watch`, relies on cron cadence alone; inconsistent with every other cron worker) | N | `0 * * * *` (hardcoded at `bootstrap/workers.ts:182`) | 1 | — | `crates/worker/free_tier_watch.rs` | 1 test file |
| 17 | `signal.raiser` (`SIGNAL_RAISER`) | `signal-raiser.ts:19` `run` | self-scheduled | pg-boss default | Y | N | `env.SIGNAL_RAISE_CRON \|\| '*/5 * * * *'` | 1 (advisory lock id=3) | — | `crates/worker/signal_raiser.rs` | 1 test file |
| 18 | `liveness.check` (`LIVENESS_CHECK`) | `liveness-checker.ts:34` `run` | self-scheduled | pg-boss default | Y | N | `*/${cronSec} * * * * *` where `cronSec = max(floor(env.WORKER_LIVENESS_CHECK_MS/1000), 30)` → default `*/60 * * * * *` (60s, seconds-granularity 6-field cron) | 1 | — (watches red-line workers — "watcher of the watcher") | `crates/worker/liveness_checker.rs` | 2 test files |
| 19 | `backup.hourly` (`BACKUP_HOURLY`) | `backup/index.ts:35` `handleBackup('hourly')` | self-scheduled | pg-boss v10 default OVERRIDDEN via `createQueueWithDefaults`: `retryLimit=3, retryDelay=30s, retryBackoff=true` (platform default) + `policy:'short'`, `deadLetter:true`, `expireInSeconds:3600`; **plus in-handler** 3-attempt retry loop (1/5/15 min delays) BEFORE the pg-boss-level retry ever engages | Y (queue `policy:'short'` — makes bare-`.schedule()`'s implicit singletonKey dedup real) | **Y** — `backup.hourly.dlq` auto-created | `env.BACKUP_HOURLY_CRON \|\| '0 * * * *'` | 1 (advisory lock, per-type hashed key) | 🔴🔴 DR/backup | `crates/worker/backup_hourly.rs` — 🔴 R2 + pg_dump strategy decision (§6) | 1 test file |
| 20 | `backup.daily` (`BACKUP_DAILY`) | `backup/index.ts:36` | self-scheduled | same as #19 | Y | Y (`.dlq`) | `env.BACKUP_DAILY_CRON \|\| '0 3 * * *'` | 1 | 🔴🔴 | `crates/worker/backup_daily.rs` | 0 direct |
| 21 | `backup.weekly` (`BACKUP_WEEKLY`) | `backup/index.ts:37` | self-scheduled | same as #19 | Y | Y | `env.BACKUP_WEEKLY_CRON \|\| '0 4 * * 0'` | 1 | 🔴🔴 | `crates/worker/backup_weekly.rs` | 0 |
| 22 | `backup.monthly` (`BACKUP_MONTHLY`) | `backup/index.ts:38` | self-scheduled | same as #19 | Y | Y | `env.BACKUP_MONTHLY_CRON \|\| '0 5 1 * *'` | 1 | 🔴🔴 | `crates/worker/backup_monthly.rs` | 0 |
| 23 | `backup.verify.restore` (`BACKUP_VERIFY_RESTORE`) | `backup/backup-verify-scheduled.ts:32` → `backup-verify.ts` `runRestoreVerify` | self-scheduled | `createQueueWithDefaults` override (`policy:'short'`, `deadLetter:true`, default retryLimit/delay/backoff) | Y | Y | `env.RESTORE_VERIFY_CRON \|\| '0 4 * * *'` | 1 | 🔴 DR proof | `crates/worker/backup_verify_restore.rs` | 1 test file (`backup.verify` substring) |
| 24 | `backup.verify.r2` (`BACKUP_VERIFY_R2`) | `backup-verify-scheduled.ts:42` → `r2-verify.ts` `runR2Verify` | self-scheduled | same override as #23 | Y | Y | `0 */6 * * *` (hardcoded, no env override) | 1 | 🔴 DR proof | `crates/worker/backup_verify_r2.rs` | 1 test file |
| 25 | `reconciliation.nightly` (`RECONCILIATION_NIGHTLY`) | `reconciliation.ts:39` `run` (12 read-only checks M1-M4,O1-O3,N1,R1,F1,T1,A6) | self-scheduled | pg-boss default; `.catch`-wrapped schedule+createQueue (boot-safety) | Y | N | `'0 3 * * *'` (hardcoded) | 1 | 🔴 money-invariant + drift detection (read-only) | `crates/worker/reconciliation_nightly.rs` | 1 test file |
| 26 | `rates.refresh` (`RATES_REFRESH`) | `rates-refresh.ts:18` `run` | self-scheduled + **one-shot boot kick** `rates-refresh.ts:22` (`startAfter:5`, `.catch`-wrapped) | pg-boss default; both `.send` and `.schedule` are `.catch`-wrapped (boot-safety, queue may not be pre-created) | N (no singletonKey — relies on advisory lock id=8192 only) | N | `env.RATES_CRON \|\| '0 * * * *'` | 1 | — | `crates/worker/rates_refresh.rs` | 0 |
| 27 | `access-request.notify` (`ACCESS_REQUEST_NOTIFY`) | `access-request-notify.ts:36` `handle` | **route** `owner/gdpr.ts`-adjacent access-request submission route (not read directly in this pass; inferred from B6/B8 comment) + `access-request-retention.ts:103` (reconcile re-feed) | genuine-failure path throws → pg-boss retries (default limit); reconcile cron also re-feeds while `notify_attempts < cap` (10) | N — uses **claim-before-send CAS** on the `access_requests` row instead of singletonKey (deliberate: idempotency lives in the DB row, not pg-boss) | N | none (event + reconcile-driven) | 1 | — | `crates/worker/access_request_notify.rs` | 2 test files |
| 28 | `access-request.reconcile` (`ACCESS_REQUEST_RECONCILE`) | `access-request-retention.ts:40` `runReconcile` | self-scheduled | pg-boss default, `.catch`-wrapped | Y | N | `env.ACCESS_REQUEST_RECONCILE_CRON \|\| '*/15 * * * *'` | 1 (advisory lock id=6) | — | `crates/worker/access_request_reconcile.rs` | 0 direct (covered under retention test) |
| 29 | `access-request.retention-sweep` (`ACCESS_REQUEST_RETENTION_SWEEP`) | `access-request-retention.ts:39` `runRetention` | self-scheduled | pg-boss default, `.catch`-wrapped | Y | N | `env.ACCESS_REQUEST_RETENTION_CRON \|\| '0 3 * * *'` | 1 (advisory lock id=5) | 🔴 GDPR-adjacent (12mo consent-expiry auto-erase) | `crates/worker/access_request_retention.rs` | 1 test file |
| 30 | `acquisition.retention-sweep` (local const, `acquisition-retention.ts:13`) | `acquisition-retention.ts:25` `runSweep` | self-scheduled | pg-boss default, `.catch`-wrapped | Y | N | `env.ACQUISITION_RETENTION_CRON \|\| '30 3 * * *'` | 1 (advisory lock id=7) | 🔴 GDPR Art.5(e) | `crates/worker/acquisition_retention.rs` | 1 test file |
| 31 | `delivery-trace.retention-sweep` (local const, `delivery-trace-retention.ts:13`) | `delivery-trace-retention.ts:25` `runSweep` | self-scheduled | pg-boss default, `.catch`-wrapped | Y | N | `env.DELIVERY_TRACE_RETENTION_CRON \|\| '15 4 * * *'` | 1 (advisory lock id=8) | 🔴 GPS/PII anonymize-not-delete | `crates/worker/delivery_trace_retention.rs` | 0 |
| 32 | `health-job` (local literal, `apps/worker/src/handlers.ts:11`) | `handlers.ts:11` | **never enqueued anywhere** — scaffolding/smoke-test only | pg-boss default | N | N | none | 1 | — | not ported (test scaffold, not product surface) | 0 |

---

## 2. Transactional-enqueue sites — THE hard requirement

**Exactly ONE call-site file has genuine same-transaction pg-boss enqueue.** No other
site in the codebase passes pg-boss the `{ db }` option.

### `apps/api/src/lib/order-persistence.ts:154-173` (`insertOrderWithItems`)

```
const txDb = { executeSql: (sql: string, values: any[]) => client.query(sql, values) };

await queue.enqueue(QUEUE_NAMES.ORDER_TIMEOUT, { orderId: order.id, locationId: input.locationId }, {
  singletonKey: order.id,
  startAfter: new Date(input.timeoutAt),
  db: txDb,                                    // ← same pg client as the order INSERT
});

await queue.enqueue(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, { event: 'order.created', … }, {
  singletonKey: dedupKey,
  db: txDb,                                    // ← same pg client
});
```

- **Business write it pairs with**: the order + order_items + modifiers + idempotency-key
  + customer-track-grant INSERT block in `orders.ts` (BEGIN at `orders.ts:112`, this
  function called at `orders.ts:561`, COMMIT at `orders.ts:597`).
- **Mechanism verified at the driver level**: `pg-boss@10.4.2`'s `manager.js` `send()` →
  `createJob()` does `const db = options.db || this.db` then `await db.executeSql(insertJobCommand, values)`
  (confirmed by reading `node_modules/.pnpm/pg-boss@10.4.2/.../src/manager.js:79,103,114`).
  Passing `{ executeSql: (sql, values) => client.query(sql, values) }` makes the
  `INSERT INTO pgboss.job` run on the **caller's own transaction-scoped client**, so a
  ROLLBACK of the order INSERT also rolls back the queue insert — genuine atomicity, no
  outbox-poller needed for this one path.
- **Why it matters for the Rust pick**: this is the ONE place in the whole system that
  needs a queue library/driver capable of "insert a job as part of an arbitrary caller
  transaction" (not just "the library's own connection"). A Postgres-backed Rust queue
  (e.g., a hand-rolled `SKIP LOCKED` table, or a crate) MUST expose an equivalent
  "enqueue using this `&mut PgConnection`/transaction" API, or this one flow needs to be
  re-architected onto a transactional-outbox pattern instead.

### Everywhere else: NOT transactional — three different durability strategies in use instead

1. **Fire-and-forget `.send()` after COMMIT** — the overwhelming majority (e.g., `orders.ts`
   COMMITs at line 597, then publishes messageBus events at 602/611/624 — no pg-boss
   enqueue at all in that block beyond the one at #2 above). A crash between COMMIT and
   the `.send()` call silently drops the side-effect (this is the standard pg-boss usage
   pattern here — accepted risk, mitigated by sweeps for the state-machine-critical paths).
2. **DB-table journal + cron pump** ("poor-man's outbox", NOT pg-boss transactional
   insert) — `courier_dispatch_queue` table: written transactionally alongside the
   business state change (e.g. `courier-offer-sweep.ts:92-99` BEGIN/INSERT/COMMIT), then
   drained by a **separate** 60s cron pass (`drainDispatchQueue`, `courier-offer-sweep.ts:165-184`)
   that calls plain non-transactional `this.boss.send(COURIER_DISPATCH, …)` per row. This
   is the Option-C "durable journal" pattern referenced throughout the file's comments —
   durability comes from the DB row surviving a lost pg-boss job, not from enqueue being
   in-transaction.
3. **In-handler retry-requeue** (not a transactional pattern, just a common idiom) — many
   workers re-`.send()` themselves on failure (`anonymizer-gdpr.ts:156`,
   `notifications/workers/index.ts:336`, `access-request-retention.ts:103`). These are
   plain pool-connection sends, never wrapped in the failing transaction.

**Conclusion for the Rust queue pick**: the codebase does NOT lean on transactional
enqueue as a systemic pattern — only 1 of ~30 queues actually needs it. This *does*
lower the bar (a queue that supports "insert via an arbitrary externally-supplied
executor/transaction" is a nice-to-have, not a hard blocker for 97% of queues), but the
one site that needs it is order creation → order-timeout, which is itself 🔴
(order state machine) — so the Rust pick still needs *a* path to same-transaction
enqueue, even if the majority of queues use fire-and-forget.

---

## 3. Worker runtime topology

### In-process (`apps/api`, Fly process group `web`, `dist/api/server.cjs`, 512mb)
Everything in `apps/api/src/workers/*`, `apps/api/src/notifications/workers/*`, plus the
3 pg-boss workers wired directly in `bootstrap/workers.ts` (`NOTIFY_DISPATCH`,
`NOTIFY_CUSTOMER_STATUS`, `NOTIFY_TELEGRAM_SEND`) and the inline `FREE_TIER_WATCH`
closure. All share the same Fastify process, same `Pool` (`pool` = operational pool,
`backupPool` = separate pool for backup jobs), same `PgBossQueueProvider` instance
(`queue.boss`), same `MessageBus`. Started inside `startBackgroundWorkers()`
(`apps/api/src/bootstrap/workers.ts:49-185`), itself wrapped in a `Promise.race` boot-
budget (`WORKER_BOOT_BUDGET_MS`) in `server.ts` main() — **boot resilience**: if worker
startup hangs or throws, `fastify.listen()` still proceeds so the menu keeps serving
(incident 2026-06-21 regression guard, per file header comment).

### Separate process (`apps/worker`, Fly process group `worker`, `dist/worker/index.cjs`, 256mb)
**NOT a separate Fly app** — confirmed via `/root/dowiz/fly.toml`: single app `dowiz`,
two `[processes]` entries (`web` and `worker`) under two separate `[[vm]]` blocks (web
512mb, worker 256mb), sharing the same deploy/release pipeline and the same
`release_command` migration gate. `apps/worker/src/index.ts` is a standalone `main()`:
own `createSessionPool()`, own `PgBossQueueProvider()` instance (separate pg-boss
client — NOT the same in-memory instance as apps/api's), own `PgMessageBus` (publish-only,
falls back to `pool` NOTIFY since it holds no listener connection). Registers exactly 2
jobs: `health-job` (dead scaffold, never enqueued) and `QUEUE_NAMES.ORDER_TIMEOUT` (the
real per-order timeout canceller, `apps/worker/src/handlers.ts:15-81`).

**Why order.timeout lives in the separate `worker` process and not `apps/api`**: no
comment states this explicitly, but the effect is isolation — the per-order timeout
canceller runs on its own 256mb VM, independent of API traffic load/crashes. The
`order.timeout_sweep` safety-net (recovering lost per-order jobs) deliberately lives
back in `apps/api` (`order-timeout-sweep.ts` comment: "this sweep is the safety net...
and the detector for a stuck/undrained order.timeout queue... Both recovery and
detection live in this one job so the detector cannot lose its host [i.e., if
`order.timeout` jobs get stuck in the `worker` process, the detector must NOT be
co-located with it]").

### `heartbeat.ts` + `shutdown.ts` semantics

**apps/worker** (`apps/worker/src/heartbeat.ts`, `shutdown.ts`):
- `Heartbeat` — single global heartbeat row keyed by `FLY_MACHINE_ID` (or `'local-worker'`),
  `INSERT ... ON CONFLICT (worker_id) DO UPDATE`, 20s interval, `timer.unref()` (never
  keeps process alive).
- Graceful drain order on SIGTERM/SIGINT (`setupShutdown`): **(1)** stop heartbeat timer
  → **(2)** `queue.stop()` (pg-boss `boss.stop({ graceful: true, wait: true })` — drains
  in-flight jobs) → **(3)** `pool.end()` → **(4)** `process.exit(0)`. Fly's `kill_timeout
  = "30s"` (root `fly.toml:44`) bounds this whole sequence.

**apps/api** (`apps/api/src/lib/worker/heartbeat.ts` — `WorkerHeartbeat`, plural/per-job):
- One `WorkerHeartbeat` instance **per critical worker id** (8 total, see
  `heartbeatConfigs` in `bootstrap/workers.ts:111-120`: `dispatcher`, `settlement-cron`,
  `dwell-monitor`, `anonymizer-retention`, `backup-hourly`, `signal-raiser`,
  `courier-stale_check`, `liveness-checker`). Each beats independently every
  `WORKER_HEARTBEAT_INTERVAL_MS` (default 15000ms), writing `(worker_id, instance_id,
  job_name, status='healthy', last_seen_at, last_job_at)` — **cadence-independent**: an
  hourly/nightly worker still beats every 15s (the beat proves the *process*, not the
  *job cadence*, is alive). No explicit `shutdown.ts` equivalent found in apps/api —
  `apps/api`'s shutdown/drain is handled inline in `server.ts` (not read in this pass;
  out of scope for this census but flagged — the 8 heartbeats have no visible `.stop()`
  call site found in this grep pass, worth a follow-up check before Rust-porting the
  drain-order guarantee).
- **Liveness-checker is the "watcher of the watcher"**: `LivenessChecker`
  (`apps/api/src/workers/liveness-checker.ts`) reads `ops_worker_heartbeat`, alerts on
  `CRITICAL_WORKERS` (env `WORKER_CRITICAL_LIST`, default
  `dispatcher,settlement-cron,dwell-monitor,anonymizer-retention,backup-hourly`) going
  stale (`WORKER_LIVENESS_STALE_MS`, default 60s), auto-resolves on recovery. The nightly
  `ReconciliationWorker`'s check **A6** cross-checks a *slightly different* 8-id list
  (`EXPECTED_WORKERS` in `reconciliation.ts:212-213`, includes `courier-stale_check` +
  `liveness-checker` but not `dispatcher`... actually includes `dispatcher` too — the two
  lists are near-identical but maintained as **two separate hardcoded arrays** in two
  files — a Rust port should unify this into one source of truth).

### Env/flag gating in `bootstrap/workers.ts`
- No env flag gates `startBackgroundWorkers()` itself — it always runs (wrapped in the
  boot-budget race for resilience, not for feature-gating).
- Individual workers self-gate: `BackupCronWorker.start()` returns early if
  `env.BACKUP_ENABLED !== 'true'` (`backup/index.ts:28-31`); `BackupVerifyWorker.start()`
  same guard (`backup-verify-scheduled.ts:21-24`); `DISPATCH_OWNER_GRACE_ENABLED`
  (default `'false'`) gates `courier-offer-sweep.ts`'s Pass 4 (grace-cancel) — the sweep
  still runs every minute, but Pass 4 no-ops unless flagged on.
- `assertAccessRequestSchedules()` / `assertDeliveryTraceSchedule()` — called **after**
  `fastify.listen()` in `server.ts:881,883` — fail-fast `process.exit(1)` in
  `NODE_ENV=production` if the expected `pgboss.schedule` rows are missing (visible red
  deploy instead of a silent zombie). Not applied to every cron — only these two
  (newest additions, per commit history a deliberate hardening pattern worth extending).

---

## 4. Cron census (verified against source, not assumed)

| Job | Actual cron expression (source) | Env override | TZ | Singleton mechanism |
|---|---|---|---|---|
| `backup.hourly` | `'0 * * * *'` | `BACKUP_HOURLY_CRON` | UTC (pg-boss default; no explicit TZ option passed anywhere in this codebase) | `singletonKey` + queue `policy:'short'` |
| `backup.daily` | `'0 3 * * *'` | `BACKUP_DAILY_CRON` | UTC | same |
| `backup.weekly` | `'0 4 * * 0'` (Sun 04:00) | `BACKUP_WEEKLY_CRON` | UTC | same |
| `backup.monthly` | `'0 5 1 * *'` (1st @ 05:00) | `BACKUP_MONTHLY_CRON` | UTC | same |
| `backup.verify.restore` | `'0 4 * * *'` | `RESTORE_VERIFY_CRON` | UTC | `singletonKey` + `policy:'short'` |
| `backup.verify.r2` | `'0 */6 * * *'` | **no env override** (hardcoded) | UTC | same |
| `settlement.generate` (aka "settlement-cron") | `'0 2 * * *'` | `SETTLEMENT_CRON` | UTC | `singletonKey` |
| `reconciliation.nightly` | `'0 3 * * *'` | **hardcoded, no env** | UTC | `singletonKey`, `.catch`-wrapped |
| `courier.stale_check` | `'*/2 * * * *'` | **hardcoded** | UTC | `singletonKey` |
| `gps.purge` | `'0 3 * * *'` | **hardcoded** | UTC | `singletonKey` |
| `order.timeout_sweep` | `'* * * * *'` | **hardcoded** (`SWEEP_CRON` const) | UTC | `singletonKey` + advisory lock id=5 |
| `courier.offer_sweep` | `'* * * * *'` | **hardcoded** (`SWEEP_CRON` const) | UTC | `singletonKey` + advisory lock id=9 |
| `dwell.monitor` | `'* * * * *'` | `DWELL_CRON` | UTC | `singletonKey` + advisory lock id=2 |
| `liveness.check` | `*/${cronSec} * * * * *` → default `*/60 * * * * *` (6-field, seconds granularity) | `WORKER_LIVENESS_CHECK_MS` (ms, converted+floored to ≥30s) | UTC | `singletonKey` |
| `signal.raiser` | `'*/5 * * * *'` | `SIGNAL_RAISE_CRON` | UTC | `singletonKey` + advisory lock id=3 |
| `rates.refresh` | `'0 * * * *'` | `RATES_CRON` | UTC | advisory lock id=8192 only (**no singletonKey** — see Appendix A-note) |
| `anonymizer.retention` | `'0 3 * * *'` | `ANONYMIZER_RETENTION_CRON` | UTC | `singletonKey` + advisory lock id=4 |
| `access-request.retention-sweep` | `'0 3 * * *'` | `ACCESS_REQUEST_RETENTION_CRON` | UTC | `singletonKey` + advisory lock id=5 (⚠ **same numeric id=5 as `order-timeout-sweep.ts`'s `SWEEP_LOCK_ID`** — different Postgres advisory-lock namespace risk if both ever run in the same backend process; each currently runs its own `pool.connect()` so likely safe, but worth a Rust-port sanity check, not re-using raw small ints across unrelated workers) |
| `access-request.reconcile` | `'*/15 * * * *'` | `ACCESS_REQUEST_RECONCILE_CRON` | UTC | `singletonKey` + advisory lock id=6 |
| `acquisition.retention-sweep` | `'30 3 * * *'` | `ACQUISITION_RETENTION_CRON` | UTC | `singletonKey` + advisory lock id=7 |
| `delivery-trace.retention-sweep` | `'15 4 * * *'` | `DELIVERY_TRACE_RETENTION_CRON` | UTC | `singletonKey` + advisory lock id=8 |
| `free_tier.watch` | `'0 * * * *'` | **hardcoded**, not read from env at all | UTC | **no singletonKey, no advisory lock** — relies purely on cron cadence + pg-boss's own single-instance dispatch |

**Every cron in this system is UTC** — no worker anywhere passes pg-boss's optional `tz`
schedule option. **Requested-list reconciliation**: every name in the prompt's seed list
is present and verified except `courier.stale_check` (verified as `courier.stale_check`
i.e. exact match) — all matched their real source names 1:1, no renames found.

---

## 5. Failure semantics

### pg-boss v10 runtime defaults (unmodified queues — the majority)
`retryLimit=2`, `retryDelay=0s` (no backoff), **no deadLetter**, default `expireIn`
(pg-boss v10 built-in, not overridden). Per the reliability-audit comment in
`packages/platform/src/queue-provider.ts:21-27`: a transiently-failing job on a bare
`createQueue()` queue is retried twice within milliseconds, then permanently lands in
`failed` with **no salvage path** — this is the state of ~24 of 30 live queues today
(everything except the 6 `backup.*`/`backup.verify.*` queues, which opt into
`createQueueWithDefaults()`).

### `createQueueWithDefaults()` override (`packages/platform/src/queue-provider.ts:70-86`)
Used only by: `backup.hourly/daily/weekly/monthly` (`backup/index.ts:52-59`) and
`backup.verify.restore/r2` (`backup-verify-scheduled.ts:29,36,45`). Sets:
`retryLimit=3, retryDelay=30s, retryBackoff=true` + optional `policy:'short'` (makes
singletonKey-only dedup real — v10 only honors singletonKey dedup when the *queue's*
policy is `short`; every bare `createQueue()` defaults to `standard`, silently making
singletonKey a no-op elsewhere) + optional `deadLetter: true` (auto-creates
`<name>.dlq`, self-referencing FK requires the DLQ queue to exist first — the helper
creates it before the parent).

**Systemic gap flagged in-repo, not yet fixed for the other 24 queues**: only the backup
family got the H1/H2 reliability-audit treatment (2026-07-03). Every retention/sweep/
notify queue still runs on bare v10 defaults (2 retries, 0 backoff, no DLQ) — **this is a
real gap the Rust rebuild should NOT reproduce**: the Rust queue's defaults should ship
the "hardened" backup-queue behavior (backoff + DLQ) as the *baseline* for every queue,
not an opt-in only 6/30 queues currently get.

### On handler throw — three distinct patterns observed
1. **Let pg-boss retry** (majority pattern) — handler throws, pg-boss's retryLimit/delay
   takes over, eventually `failed` (silently, for the 24 un-hardened queues).
2. **Swallow + re-throw with enriched payload** (`notifications/workers/index.ts:513-534`,
   Telegram send) — catches, checks `MAX_RETRIES=10` in-handler, archives to
   `notification_outbox_audit` (status='archived') **without re-throwing** once exhausted
   (deliberately breaks the pg-boss retry loop at a business-level cap tighter than any
   queue-level DLQ would apply).
3. **Swallow + self-requeue with computed backoff, never let pg-boss retry at all**
   (`anonymizer-gdpr.ts:138-165`) — catches, computes `2^retryCount * 60`s, calls its OWN
   `this.boss.send(..., { startAfter: backoff })`, and does NOT re-throw — the original
   job then completes "successfully" (no error surfaces to pg-boss), decoupling GDPR
   retry cadence entirely from pg-boss's built-in retry mechanics. **This means GDPR
   erasure retry logic is 100% application-level, not queue-level** — a fact the Rust
   queue pick can lean on (no dependency on the queue library's own backoff for this
   flow).

### DLQ usage — narrow
Only 6 of 30 live queues have a DLQ (`backup.*` ×4 + `backup.verify.*` ×2, all via
`createQueueWithDefaults(..., { deadLetter: true })`). No consumer of any `.dlq` queue
was found in this pass (i.e., jobs land in `backup.hourly.dlq` etc. but nothing reads
from it yet — presumably manual/ops inspection only). **Rust-port note**: if the Rust
queue ships DLQ as default-on everywhere (recommended above), it needs an explicit
decision on whether anything *consumes* the DLQ or it's purely a manual-inspection sink
— currently the latter, system-wide.

---

## 6. Rust mapping notes

**General shape**: of the 30 live queues, **27 are pure cron sweeps or on-demand single-
job dispatches with zero cross-queue fan-out complexity** — these map cleanly to either
(a) a tokio `tokio_cron_scheduler` (or hand-rolled `tokio::time::interval`) loop that
does its own `pg_try_advisory_lock` + business logic in one `sqlx` transaction, with NO
actual queue-table involvement, or (b) a thin wrapper job row if you want uniform
observability (`ops_worker_heartbeat`-equivalent) across all workers. Given nearly every
cron worker already does its own `pg_try_advisory_lock(id)` for singleton enforcement
(independent of pg-boss's singletonKey), **a plain tokio cron loop + advisory lock is a
legitimate, arguably simpler Rust-native replacement for the ~20 self-scheduled sweep
queues** (#5,7,8,9,10,11,13,17,18,19-24,25,26,28,29,30,31 in §1) — they don't need to be
"queue jobs" at all in Rust; pg-boss is providing cron scheduling + single-flight there,
both of which tokio + `pg_try_advisory_lock` do natively without an intermediary queue
table.

**The queue-table IS load-bearing for the remaining ~10**: anything that is genuinely
event-driven / fan-out / needs durable cross-process handoff:
- `order.timeout` (per-order delayed job, needs `startAfter` + durability across the
  `apps/worker` process — a real queue, and the ONE transactional-enqueue site, §2)
- `notify.dispatch`, `notify.customer_status`, `notify.telegram.send` (event-triggered
  fan-out from many producers, need real per-job retry/backoff, not a cron loop)
- `courier.dispatch` (pumped by the offer-sweep, singleton per order — could arguably
  fold into the sweep's own loop, since the sweep is its ONLY producer, but keeping it
  separate lets dispatch retry cadence differ from the 60s sweep tick if ever needed)
- `anonymizer.gdpr` (on-demand, owner-triggered — needs real durability, a cron loop
  polling a `pending` table column would work equally well and is arguably simpler;
  🔴 council decision)
- `velocity.flush` — candidate to fold entirely into an in-process tokio
  `mpsc`/debounce channel instead of round-tripping through Postgres at all, IF
  cross-instance durability of the 5s buffer is not actually required (needs a
  product decision: is losing an in-flight 5s velocity buffer on a crash acceptable?
  Today's pg-boss version already accepts this risk in-memory before the flush fires).

**Money math** 🔴 (`settlement.generate` → `app_generate_settlements()`): the queue
plumbing itself is trivial (single cron, single DB function call, no payload beyond an
optional `referenceDate`); ALL the actual risk is inside the SECURITY DEFINER SQL
function, which is out of scope for this jobs census — flag for the separate money-math
council/port, not a queue-architecture concern.

**Backup workers** 🔴 (`backup.hourly/daily/weekly/monthly` + `backup.verify.*`): the
Rust port needs an explicit decision on **pg_dump strategy** — today's
`createLogicalDump()` (in `dump.ts`, not read in this pass but referenced) almost
certainly shells out to `pg_dump` against `DATABASE_URL_MIGRATIONS`; Rust has no
built-in equivalent and would need either (a) shell out to `pg_dump` as a subprocess
(same as today, just from Rust), or (b) a native logical-dump implementation via `sqlx`
+ COPY. Also needs an R2/S3 client decision (`aws-sdk-s3` crate vs a lighter S3-
compatible client) to replace `uploadStream`/`uploadJson` (`upload.ts`). The
retry-then-DLQ shape (in-handler 3-attempt loop BEFORE queue-level retry) is worth
preserving as-is — it's a deliberate two-tier retry (fast in-process retries for
transient R2/dump blips, DLQ only for genuinely exhausted attempts).

**GDPR/anonymizer** 🔴: `anonymizer.gdpr`'s current `singletonKey` dedups ALL pending
erasure requests globally to one in-flight job (Appendix A11) — worth revisiting in the
Rust port since it means erasure throughput is serialized to whatever the batch-of-10
`FOR UPDATE SKIP LOCKED` loop processes per invocation; if erasure request volume ever
grows, this becomes a queueing-theory bottleneck baked into the *dedup key choice*, not
a Rust concern per se, but worth NOT blindly reproducing the same key.

**Sequential decision-lens for every queue when porting**: 
1. Is there more than one producer, or delayed/durable-across-restart semantics needed?
   → real queue-table job. 
2. Is it purely "run this on a schedule, single-flight across instances"? → tokio cron
   loop + `pg_try_advisory_lock`, skip the queue table entirely.
21 of 30 queues in this census satisfy (2) and could shed the queue abstraction
altogether in a from-scratch Rust rebuild — a materially smaller surface than "port
pg-boss 1:1."

---

## 7. Appendix — ambiguous literals & classification

Namespace legend: **(a)** pg-boss QUEUE name, **(b)** message-bus EVENT name (WS lane,
out of scope), **(c)** DB table name, **(d)** env var name (config lane, not a queue),
**(e)** dead/unused constant.

| Literal | Where seen | Classification | Note |
|---|---|---|---|
| A1. `settlement.cron` (`QUEUE_NAMES.SETTLEMENT_CRON`) | `queue-names.ts:9`; used only as a heartbeat `jobName` LABEL (`bootstrap/workers.ts:113`) | **(e)** dead-as-a-queue constant | The real queue is `settlement.generate`. `SETTLEMENT_CRON` the *string* is never `.work()`'d or `.schedule()`'d — it only labels a heartbeat row. Do not port as a queue. |
| A2. `SETTLEMENT_CRON` (env var) | `settlement-cron.ts:24` `env.SETTLEMENT_CRON \|\| '0 2 * * *'` | **(d)** env var | **Same string as A1 but a totally different namespace** — coincidental name collision between an unused queue-name constant and a real env var controlling the `settlement.generate` cron expression. Flagged because it's genuinely confusing on first read. |
| A3. `order.pending_aging` (`QUEUE_NAMES.ORDER_PENDING_AGING`) | `queue-names.ts:5` | **(e)** dead/unused constant | Zero references anywhere except its own definition. Never `.work()`'d, `.send()`'d, or `.schedule()`'d. Do not port. |
| A4. `dwell.escalate` (`QUEUE_NAMES.DWELL_ESCALATE`) | `dwell-escalation.ts:20` | **(a)+(e)** — referenced as a queue name but **the constant does not exist** in `queue-names.ts` at all; evaluates to `undefined` at runtime, masked by the file's `// @ts-nocheck` | `DwellEscalationWorker` is also never instantiated in `bootstrap/workers.ts`. Fully dead, broken-if-ever-invoked code. The real dwell-escalation delivery path is `dwell-monitor.ts:127,136` sending straight to `notify.dispatch` with a `startAfter` delay (tier-1 immediate + tier-2 delayed) — `DwellEscalationWorker`'s tier-1/2/3 batching logic is superseded/unused. **Do not port `dwell.escalate`.** |
| A5. `order.created`, `order.confirmed`, `order.rejected`, `order.delivered`, `order.cancelled`, `order.status`, `courier.position_updated`, `courier.stale_heartbeat`, `shift.started`, `shift.closed`, `settlement.approved`, `backup.completed`, `backup.failed`, `dwell.monitor.failed`, `menu.imported`, `otp.sent`, `otp.verified`, `worker.stale`, `worker.failed`, `alert.worker_liveness`, `signal.created`, `gdpr.erasure_completed`, etc. (all of `BUS_CHANNELS` in `apps/api/src/lib/registry.ts:1-45`) | `messageBus.publish(...)` call sites throughout | **(b)** message-bus EVENT names | These are Redis/PgMessageBus pub/sub channels (WS lane) — **explicitly out of this census's scope** per the task framing. Listed here only to show they were seen and correctly excluded, not conflated with queue names despite dot-separated naming that visually resembles queue names. |
| A6. `ops.reconciliation_drift`, `ops:order_timeout_lag` | `reconciliation.ts:103`, `order-timeout-sweep.ts:56,160` | **(b)** message-bus event (ad-hoc, not in `BUS_CHANNELS` const — string literal inline) | Same lane as A5 but not even centrally registered — a minor internal inconsistency (some bus events go through the `BUS_CHANNELS` const, some are raw string literals), not a queue. |
| A7. `settlements`, `backup_metadata`, `ops_worker_heartbeat`, `pgboss.job`, `pgboss.schedule`, `access_requests`, `courier_dispatch_queue`, `gdpr_erasure_requests`, `notification_outbox_audit`, `free_tier_snapshots`, `exchange_rates` | Throughout | **(c)** DB table names | Never queue names. `courier_dispatch_queue` and `gdpr_erasure_requests`/`access_requests` are the "poor-man's outbox" journal tables that a cron *pumps into* a real pg-boss queue (§2 pattern #2) — worth remembering these tables are effectively pre-queue staging, not queues themselves. `pgboss.job`/`pgboss.schedule` are pg-boss's OWN internal tables (queried read-only by `order-timeout-sweep.ts:49`, `reconciliation.ts:236`, and the two boot-assert functions) — meta, not product queues. |
| A8. `health-job` | `apps/worker/src/handlers.ts:11` | **(a)** real queue registration, but **never enqueued** anywhere in the codebase | Scaffolding/smoke-test leftover. Confirmed zero `.send('health-job', ...)` call sites. Recommend NOT porting — dead weight. |
| A9. `velocity.flush` singletonKey | `velocity-increment.ts:54` | (a) real queue, but singletonKey is the **literal string `'velocity.flush'`** (not per-location) | Intentional — the in-process buffer batches ALL locations before one flush call, so global dedup is correct here, unlike the accidental-looking pattern in A11. Listed to distinguish "intentional global singleton" from "accidental global singleton." |
| A10. `rates.refresh` — no singletonKey | `rates-refresh.ts:18-27` | (a) real queue, relies on advisory lock 8192 ONLY for singleton enforcement, no pg-boss-level `singletonKey` | Inconsistent with every other cron worker in the codebase (all others use both belt-and-suspenders: advisory lock + singletonKey). Not necessarily a bug (advisory lock alone is sufficient), but an inconsistency worth normalizing in the Rust port rather than reproducing verbatim. |
| A11. `anonymizer.gdpr` singletonKey = `QUEUE_NAMES.ANONYMIZER_GDPR` (the queue's own name, a constant, not a per-request key) | `anonymizer-gdpr.ts:17` | (a) real queue, **singleton dedup scope is GLOBAL, not per-request** | Unlike `order.timeout` (`singletonKey: order.id`, correctly per-entity) or `notify.telegram.send` (`singletonKey: dedupKey` including entity id), `anonymizer.gdpr`'s singletonKey is the bare queue-name constant — meaning at most ONE GDPR erasure job can be in-flight system-wide at any time (the batch-of-10 loop inside `run()` is how multiple pending requests actually get processed per invocation, not via concurrent jobs). Flagged as a design choice to consciously carry forward or revisit, not a bug — but easy to misread as a per-request key on first pass. |
| A12. `free_tier.watch` singletonKey | absent (bootstrap/workers.ts:174-182) | (a) real queue, **no singletonKey at all** (only cron cadence) | The ONE cron-scheduled queue in the entire census with zero singleton mechanism (no `singletonKey`, no advisory lock). Low risk in practice (hourly cadence, idempotent read+insert snapshot), but structurally an outlier — every other cron worker in this codebase pairs `.schedule()` with a `singletonKey`. |

---

## Summary — answers to the RETURN line

- **Total queue count**: **33** distinct name constants exist in source; **30** are
  actually live (`.work()` reachable from a running process); **1** is dead-but-coded
  (`dwell.escalate`); **2** are pure dead constants never implemented at all
  (`order.pending_aging`, `settlement.cron`-as-queue).
- **Cron count**: **23** actual `.schedule()` calls (one grep hit was a comment, not a
  call — see §0).
- **Transactional-enqueue site count**: **1 file, 2 enqueue calls**
  (`apps/api/src/lib/order-persistence.ts:158` `ORDER_TIMEOUT` + `:165`
  `NOTIFY_TELEGRAM_SEND`, both sharing the `orders.ts` POST /orders transaction via
  pg-boss's `{ db: { executeSql } }` option). This is the **only** place in the codebase
  where a queue insert and a business write share one DB transaction — everywhere else
  is either fire-and-forget-after-commit, a separate DB-table journal drained later by a
  cron pump (`courier_dispatch_queue`), or an in-handler self-requeue on failure. This
  materially lowers (but does not zero out) the bar for the Rust queue's transactional
  requirements.
- **🔴 queue count**: **9 families** — `settlement.generate` (money); `reconciliation.nightly`
  (money-invariant + drift, read-only); `order.timeout`/`order.timeout_sweep` +
  `courier.dispatch`/`courier.offer_sweep` (order/dispatch state machine, 2 families);
  `anonymizer.gdpr` + `anonymizer.retention` (GDPR, 2 families); `backup.hourly/daily/
  weekly/monthly` + `backup.verify.restore/r2` (backup/DR, 2 families); plus 3 more
  GDPR-adjacent retention sweeps (`access-request.retention-sweep`,
  `acquisition.retention-sweep`, `delivery-trace.retention-sweep`) — counting those
  brings the strict 🔴 total to **12 individual queue names** across 9 conceptual
  families, all tagged in §0.1/§1.
- **Ambiguities not fully resolvable from static reading alone**: whether `apps/api`'s 8
  `WorkerHeartbeat` instances have any explicit `.stop()` call in `server.ts`'s shutdown
  path — not confirmed in this pass (`server.ts`'s own shutdown/drain sequence was out
  of the file list given for this census; flagged as a follow-up before Rust-porting the
  drain-order guarantee for the `web` process group specifically, mirroring what was
  confirmed for `apps/worker`'s `shutdown.ts` in §3).

  *(Resolved, not left ambiguous)*: the two hardcoded worker-id arrays are **not**
  drifted duplicates — `CRITICAL_WORKERS` (`liveness-checker.ts:13`, 5 ids: `dispatcher,
  settlement-cron,dwell-monitor,anonymizer-retention,backup-hourly`) is the subset that
  triggers `LivenessChecker`'s real-time (60s) immediate-page-on-newly-stale alert;
  `EXPECTED_WORKERS` (`reconciliation.ts:212-213`, 8 ids: the same 5 **plus**
  `signal-raiser, liveness-checker, courier-stale_check`) is the full roster the nightly
  A6 check expects to have beaten at all in the last hour — confirmed to match the 8
  `heartbeatConfigs` workerIds in `bootstrap/workers.ts:111-120` exactly (verified by
  direct comparison, not just visual similarity). This is intentional two-tier
  layering (narrow real-time-critical vs. broad nightly-completeness), not drift — but
  it IS two independently-maintained hardcoded lists with no shared source constant, so
  a Rust port should unify them into one roster with an explicit "critical" flag per
  entry rather than reproducing two parallel arrays.


---

<!-- ============ §4 SERVER-SIDE INTEGRATIONS ============ -->

# Server-Side Integrations Census — Rust Rebuild Map

Scope: `apps/api` (Fastify server) + `apps/worker` (background worker shell — confirmed to carry **zero** third-party deps of its own; all actual "worker" logic for backup/notifications/rates/anonymizer lives inside `apps/api/src/workers/` and `apps/api/src/notifications/workers/`, run in-process by pg-boss, not by the standalone `apps/worker` app).

Target architecture: Rust/axum. Each integration maps to a `crates/api/src/integrations/<name>.rs` module or an explicit sidecar/ops-binary decision (noted per integration).

Legend: 🔴 = red-line/high-risk item (money, PII, auth secret, destructive DB op, webhook signature). Proof = existing test citation, or `needs-new`.

---

## 1. Telegram — bot + notifications

**Verified active.** `grep -rn "TelegramAdapter" apps/api/src` → 6 real sites; `callTelegramApi`/`sendMessage`/`answerCallbackQuery` → 10+ call sites in `telegram-webhook.ts`. **Both webhook and polling code exist in the repo, but only webhook mode is live**: `server.ts:330-331` constructs `TelegramPoller` but never calls `.start()` (commented "disabled: webhook active"), only `.stop()`s it on shutdown (`server.ts:868`). The webhook route is registered unconditionally at `server.ts:528-533`. There is no env/flag toggle between the two modes — it's a hardcoded dead branch (`notifications/workers/telegram.poll.ts`, `getUpdates` call in `adapters/telegram.ts:58`).

Telegram Bot API endpoints called (raw `fetch`, no SDK): `sendMessage` (adapter dispatch path, `adapters/telegram.ts:23`; webhook reply helpers, `telegram-webhook.ts:727-764`), `getUpdates` (dead polling path, `adapters/telegram.ts:58`), `answerCallbackQuery` (`telegram-webhook.ts:766-793`, ~10 call sites), `editMessageText` (`telegram-webhook.ts:464-469`), `getMe` (health probe, `routes/health.ts:137`).

Message kinds sent (via `event-registry.ts` + `render.ts:70-132`): order created/confirm-reject buttons, confirmed/rejected/delivered/dwell_escalation/ready_for_pickup/timeout_cancelled/dispatch_failed, cash-reconcile discrepancy, delivery flag raised, low rating, order pending-aging confirm/reject, courier assigned (track link), shift started/closed/close-reminder, ops worker-liveness/backup-failed/degradation-changed/test. Inbound bot commands `/start`, `/stop`, `/open`, `/close`, `/store`, `/settings` and callback actions (order confirm/reject-reason, store open/close, pref-set) handled in `telegram-webhook.ts:274-712`. No OTP-over-Telegram path exists.

`chat_id` storage: `owner_notification_targets.address` where `channel='telegram'` (migration `1780348982032`), linked via `location_id`/`user_id`; pre-auth handshake in `telegram_connect_tokens` (migration `1780348982031`) and `telegram_login_tokens` + `users.telegram_user_id` (migration `1790000000031`).

Failure handling: adapter maps 401/403 → `AUTH_OR_BLOCKED` → permanent target `status='disabled'` (`notifications/workers/index.ts:325-326`); 429 → `RATE_LIMIT` honoring `retry-after`; network error caught (`adapters/telegram.ts:40-52`). Dispatch path uses exponential backoff (`retry.ts:15-21`); the Telegram-send path additionally runs its own per-chat circuit breaker + rate limiter + dedup cache (`workers/index.ts:396-489`). Webhook route always returns HTTP 200 to Telegram even on internal failure (best-effort, off critical path, `telegram-webhook.ts:78-90`).

Feature flags: `TG_CATEGORY_GATING` (default off, category-based prefs), `TG_STOREFRONT_ACTION` (default off, store open/close via bot). Telegram itself is not flag-gated — always wired, soft-degrades to no-op if `TELEGRAM_BOT_TOKEN` is empty.

**⚠️ Finding for the rebuild (not fixed — read-only task):** `telegram-webhook.ts:48-61` — when `TELEGRAM_BOT_SECRET` is set but the `x-telegram-bot-api-secret-token` header is **absent**, the handler logs a warning and processes the request anyway ("backward compat"); it 401s only when the header is present but wrong. `e2e/tests/telegram-webhook.spec.ts:53-59` ("missing secret returns 401") asserts behavior that contradicts this — either the test is stale or this is a live fail-open gap. **Rust rebuild must make signature verification unconditionally fail-closed.**

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Webhook endpoint `POST /webhook/telegram/:secret` | `apps/api/src/routes/telegram-webhook.ts:36` | `TELEGRAM_BOT_SECRET` | axum route + raw JSON body extractor | 🔴 | `e2e/tests/telegram-webhook.spec.ts:42-70` |
| Webhook secret verification (fail-open on missing header — bug to fix in rebuild) | `telegram-webhook.ts:48-61` | `TELEGRAM_BOT_SECRET` | constant-time compare, fail-closed unconditionally | 🔴 | `e2e/tests/telegram-webhook.spec.ts:53-59` (currently mismatched vs code) |
| `sendMessage` API call | `notifications/adapters/telegram.ts:23`; `routes/telegram-webhook.ts:742-764` | `TELEGRAM_BOT_TOKEN` | `reqwest`+`serde_json` (no polling SDK needed) | | `apps/api/tests/notifications/telegram.test.ts:18-27` (token-absent path) |
| `getUpdates` polling (disabled) | `notifications/workers/telegram.poll.ts:25-38`, `adapters/telegram.ts:56-68` | `TELEGRAM_BOT_TOKEN` | drop — dead code, do not port | | needs-new |
| `answerCallbackQuery` / `editMessageText` | `telegram-webhook.ts:766-793`, `:464-469` | `TELEGRAM_BOT_TOKEN` | `reqwest` | | needs-new |
| `chat_id` storage | `owner_notification_targets.address` | migration `1780348982032` | Postgres table, same shape | | needs-new |
| Pre-auth connect/login tokens | `telegram_connect_tokens`, `telegram_login_tokens` | migrations `1780348982031`, `1790000000031` | same table, uuid PK | 🔴 (auth) | `e2e/tests/telegram-full-flow.spec.ts` |
| Failure → disable target on 401/403 | `notifications/workers/index.ts:325-326` | — | state machine on target row | | needs-new |
| Retry/backoff (dispatch path) | `notifications/retry.ts:15-21` | — | exponential backoff util | | `apps/api/tests/notifications/telegram.test.ts:8-16` |
| Circuit breaker/rate-limit (Telegram send path) | `notifications/workers/index.ts:396-418` | in-memory maps | in-process `HashMap`+`Instant` | | needs-new |
| `TG_CATEGORY_GATING` flag | `telegram-webhook.ts:19`, `workers/index.ts:49` | `TG_CATEGORY_GATING` | config flag | | `apps/api/tests/notifications/category-gating.test.ts` |
| `TG_STOREFRONT_ACTION` flag | `telegram-webhook.ts:25` | `TG_STOREFRONT_ACTION` | config flag | | `apps/api/tests/notifications/storefront-action.test.ts` |
| Health check (`getMe`) | `routes/health.ts:131-147` | `TELEGRAM_BOT_TOKEN` | reqwest probe | | `e2e/tests/telegram-webhook.spec.ts:32-40` |

---

## 2. Web-push + full notification pipeline architecture

**Verified active.** `WebPushAdapter` has 8 real call sites; `adapters/push.ts` (`PushAdapter`) is a dead "Phase 4 scaffold" (`@ts-nocheck`) never registered with the dispatcher — not a router across push types, just unused code.

**Pipeline flow** (event → registry → render → adapter → retry → audit):
1. **Event fires** — producers enqueue a pg-boss job: `NotifyDispatchJob` (owner/push+telegram-tenant events), `TelegramSendJob` (Telegram-specific fan-out), `CustomerStatusJob` (opt-in customer push) — types in `notifications/workers/index.ts:14-36`.
2. **Registry lookup** — `notifications/event-registry.ts:16-167` (`EVENT_REGISTRY`) maps each of 21 event types to quiet-hours policy, render group, target scope; `getEventCategory`/`isSuppressedByCategory` (`:190-215`) classify transactional/operational/quality.
3. **Dispatch worker** — `handleDispatch` (`workers/index.ts:202-343`): re-fetches target + verifies `status='active'` (`:209-226`), checks prefs (`:229-238`), evaluates quiet hours (`quiet-hours.ts:38-67`, called `:246`), re-fetches order under tenant isolation (`:276-297`).
4. **Render** — `render.ts:61-139` (`renderTelegramMessage`, via `locales.ts`); `push-strings.ts` (`getPushText`) consumed in `adapters/webpush.ts:56-84` (`buildPayload`).
5. **Adapter dispatch** — `notifications/provider.ts:80-94` (`NotificationDispatcher.dispatch`) routes by `target.channel`, wired in `bootstrap/notifications.ts:46-58`.
6. **Retry-on-fail** — exponential backoff (`retry.ts:15-21`) applied at `workers/index.ts:328-337`; immediate permanent disable on `AUTH_OR_BLOCKED`.
7. **Audit write** — `notifications/audit.ts:29-47` (`writeAudit`) → `notification_outbox_audit` table, on every branch (no_target/target_inactive/prefs_disabled/quiet_hours/order_not_found/success/fail), queryable via `routes/admin/notification-audit.ts`.

Web-push specifics: VAPID keys read at `bootstrap/notifications.ts:52-58` (only registers if both keys set) and `workers/index.ts:90-93` (lazy init for customer-status push); `routes/public/vapid.ts:5-9` serves public key only (`GET /api/push/vapid-public-key`, 404 if unconfigured). Config schema: `packages/config/src/index.ts:115-117` (`VAPID_PUBLIC_KEY`/`VAPID_PRIVATE_KEY` required, `VAPID_SUBJECT` defaults `push@deliveryos.app`). Subscription storage: customer subscriptions in `customer_devices` (`opted_in`, `push_subscription`, `vapid_endpoint`, `keys_p256dh`, `keys_auth` — migration `1780421100059`, base table `1780348982033`), written by `routes/customer/push.ts:41-55` (dedup by SHA-256 endpoint fingerprint); owner subscriptions in `owner_notification_targets` (`channel='push'`, address = JSON-stringified subscription — `routes/owner/push.ts:34,56-58`). External endpoint: **not a single fixed URL** — the `web-push` npm library POSTs directly to each subscription's own browser-vendor push-service URL (FCM/Mozilla autopush/etc, stored per-row) using VAPID JWT auth (`adapters/webpush.ts:32-36`, `workers/index.ts:177`). Stale subscriptions pruned on 410/404 (`workers/index.ts:183-191`).

Categories + quiet hours (`notificationPrefsService.ts`): categories are `transactional` (default-on, never suppressed), `operational` (`shift.*` events), `quality` (`rating.low_received`). Stored in `owner_notification_targets.prefs` jsonb; `setCategoryPref` (`:23-72`) does atomic `jsonb_set` under `FOR UPDATE` + same-transaction audit insert into `notification_prefs_audit` (migration `1790000000051`). Quiet-hours window per target: `owner_notification_targets.quiet_hours` jsonb (migration `1790000000053`), evaluated against `locations.timezone` (`quiet-hours.ts:38-67`).

Additional finds: `locales.ts` (Telegram outbound i18n, sq/en/uk, separate from `bot-strings.ts` for inbound replies); `lib/pii-mask.ts` masks customer phone before Telegram rendering (`workers/index.ts:585`); **no Slack/Discord/SMS/Twilio adapter exists** (confirmed by repo-wide grep); **WhatsApp/Baileys channel was fully removed** (migration `1790000000043_remove-whatsapp-channel.ts` — do not port); customer-status push (`workers/index.ts:99-200`) is a third, separate send path sending minimal no-PII push for order-status transitions.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Event registry (21 event types) | `notifications/event-registry.ts:16-167` | — | `enum`+`match` or static map | | needs-new |
| Quiet-hours evaluator | `notifications/quiet-hours.ts:38-67` | `locations.timezone` | `chrono-tz` | | `apps/api/tests/notifications/quiet-hours.test.ts` |
| Retry policy (exp backoff) | `notifications/retry.ts:15-21` | — | small struct, same formula | | `apps/api/tests/notifications/telegram.test.ts:8-16` |
| Audit write | `notifications/audit.ts:29-47` | `notification_outbox_audit` table | `sqlx` insert | | needs-new |
| `NotificationDispatcher.dispatch` (channel routing) | `notifications/provider.ts:87-93` | — | `Box<dyn NotificationProvider>` dispatch | | needs-new |
| VAPID config | `bootstrap/notifications.ts:52-58`, `packages/config/src/index.ts:115-117` | `VAPID_PUBLIC_KEY`/`VAPID_PRIVATE_KEY`/`VAPID_SUBJECT` | `web-push` crate or `web-push-native` | 🔴 (private key) | `apps/api/tests/notifications-bootstrap.test.ts:36-43` |
| `GET /api/push/vapid-public-key` | `routes/public/vapid.ts:5-9` | `VAPID_PUBLIC_KEY` | axum handler | | `e2e/tests/api-real.spec.ts:91-92` |
| Customer subscribe/unsubscribe | `routes/customer/push.ts:21-81` | `customer_devices` table | axum + sqlx | | needs-new |
| Owner subscribe/unsubscribe/state | `routes/owner/push.ts:23-104` | `owner_notification_targets` table | axum + sqlx | | `e2e/tests/flow-core-lifecycles.spec.ts:695-701` |
| `web-push` send-notification call | `notifications/adapters/webpush.ts:32-36`, `workers/index.ts:177-181` | per-subscription endpoint (not fixed) | `web-push` crate `VapidSignatureBuilder` + reqwest | | needs-new |
| Prune on 410/404 | `workers/index.ts:183-191`, `adapters/webpush.ts:40-41` | — | same logic | | needs-new |
| Category prefs (operational/quality) | `notificationPrefsService.ts:23-72` | `owner_notification_targets.prefs`, `notification_prefs_audit` | atomic UPDATE + audit insert, one txn | 🔴 (consent/GDPR) | `apps/api/tests/notifications/prefs-service.test.ts`, `category-gating.test.ts` |
| Dead scaffold `PushAdapter` | `notifications/adapters/push.ts:1-13` | — | drop, do not port | | n/a |
| Admin audit query (PII-free) | `routes/admin/notification-audit.ts:17-54` | `notification_outbox_audit` | axum handler, platform-admin gate | 🔴 (admin authz) | needs-new |

---

## 3. Email adapter (Resend)

**Verified active but narrow.** `EmailAdapter` has exactly 3 real sites: definition (`adapters/email.ts`) and one consumer, `apps/api/src/workers/access-request-notify.ts:7,31,73`. **Not** wired into `event-registry.ts`/`provider.ts`/`workers/index.ts` — it is a separate, direct-call ops-alert path, not part of the tenant notification dispatcher.

Provider: no SDK, raw `fetch` to the **Resend REST API**, `POST https://api.resend.com/emails` (`adapters/email.ts:46`), `Authorization: Bearer ${RESEND_API_KEY}` (`:48-50`), 5s abort timeout (`:59`). No SMTP/nodemailer anywhere. Called from `access-request-notify.ts:31,73` to alert an operator when a new `access_requests` row is submitted — deliberately bypasses the tenant dispatcher (no `locationId`, no need for per-tenant dedup/prefs/quiet-hours/audit). Env: `RESEND_API_KEY` (optional — absent → soft-disable, `{delivered:false, reason:'email-disabled'}`, request still persists).

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Resend send-email call | `notifications/adapters/email.ts:46-60` | `RESEND_API_KEY` | `reqwest` direct to Resend REST API (no SDK today; `lettre` only if switching to SMTP) | 🔴 (API key) | needs-new |
| Missing-key soft-disable | `adapters/email.ts:41-43` | `RESEND_API_KEY` optional | config-gated `Option<Client>` | | needs-new |
| Auth-error surfacing (401/403) | `adapters/email.ts:69-71` | — | typed error variant | | needs-new |
| Rate-limit handling (429) | `adapters/email.ts:73-80` | — | retry-after aware | | needs-new |
| Wiring: access-request alert | `apps/api/src/workers/access-request-notify.ts:31,73-93` | `RESEND_API_KEY` | direct call, not through dispatcher | | `apps/api/tests/access-request-workers.test.ts:44,60,68` (mocked send, not real HTTP) |

---

## 4. R2/S3 storage

**Verified active.** 51 real call-site matches (`r2-storage|R2_BUCKET|S3Client|PutObjectCommand|GetObjectCommand|DeleteObjectCommand|getSignedUrl`) across `apps/api/src` + `packages`. **`apps/worker/src` has zero hits** — the backup worker lives in `apps/api/src/workers/backup/`, not the standalone `apps/worker` app.

Client: `apps/api/src/lib/r2-storage.ts:27-35` — lazily `import('@aws-sdk/client-s3')` → `new S3Client({ endpoint: R2_ENDPOINT, region:'auto', credentials:{R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY} })`. Env: `R2_BUCKET`, `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` — presence of bucket+endpoint gates provider choice (`server.ts:306-308`: R2 if both set, else `LocalFsStorageProvider`).

**Bucket structure: one bucket, key-prefix separated** — product images (`${locId}/${pid}-${hash}.webp`), rich media (`${locId}/${productId}/${subKind}/${sha256_12}.${ext}`), theme logos (`locations/${locationId}/logo.webp`), entry photos (`entry-photos/${uuid}.webp`), menu-import cache (`import_${uuid}.csv`), backups (`dowiz-backups/${NODE_ENV}/${type}/${dateStr}/${backupId}.enc.parts`). `lib/image-key.ts:7-14` enforces DB image fields are always object-storage keys, never inline data:/blob: URLs.

Failure handling: `r2-storage.ts` put/get/delete don't self-retry; errors propagate except `get()` which catches 404/`NoSuchKey` → `null` (`:69-71`). Route call sites catch and 500. Old-key cleanup after image swap is best-effort/swallowed (`spa-proxy.ts:258-260`). Backup orchestrator wraps the whole dump→encrypt→upload sequence in its own outer retry loop (retry lives at job level, not the S3 client).

Call-site breakdown by feature area: product-media (`routes/owner/product-media.ts:142-168`, presigned PUT), themes/branding (`routes/owner/themes.ts:135-136`, via `storage.put` abstraction), menu-import (`routes/owner/menu-import.ts:112`, via same abstraction, 30-min cache), backup (`workers/backup/upload.ts`, `backup-verify.ts`, `r2-verify.ts`, `index.ts`, `scripts/restore.ts`).

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| S3Client construction | `apps/api/src/lib/r2-storage.ts:27-35` | `R2_ENDPOINT`,`R2_ACCESS_KEY_ID`,`R2_SECRET_ACCESS_KEY`,`R2_BUCKET` | `object_store` crate (`AmazonS3Builder`+custom endpoint) — usage is 3 simple verbs, no presign/multipart at this layer | 🔴 | `apps/api/tests/r2-storage.test.ts:8-32` |
| PutObjectCommand | `r2-storage.ts:49-58` | same | `object_store::put` | 🔴 | needs-new |
| GetObjectCommand (404→null) | `r2-storage.ts:62-72` | same | `object_store::get` | | needs-new |
| DeleteObjectCommand | `r2-storage.ts:76-78` | same | `object_store::delete` | | needs-new |
| Presigned PUT (client uploads direct to R2, 300s TTL) | `routes/owner/product-media.ts:142-168` (TTL `:29`) | `R2_BUCKET`,`R2_ENDPOINT`,`R2_ACCESS_KEY_ID`,`R2_SECRET_ACCESS_KEY` | `aws-sdk-s3` (Rust) specifically for presigning — `object_store` has no first-class presign API | 🔴 | `apps/api/tests/product-media-validation.test.ts` (validation only; presign flow is needs-new) |
| Multipart streaming upload (backup dumps) | `workers/backup/upload.ts:38-52` (`@aws-sdk/lib-storage` Upload, 5MB parts, queue 4) | R2 env vars via `R2Config` | `object_store::put_multipart` or `aws-sdk-s3` multipart trio | 🔴 | needs-new |
| JSON manifest upload | `upload.ts:55-71` | same | `object_store::put` | | needs-new |
| Backup verify GetObjectCommand | `workers/backup/backup-verify.ts:111-122`, `r2-verify.ts:50,68` | `R2_BUCKET` | `object_store::get` | | needs-new |
| HeadBucketCommand health check | `routes/health.ts:156-166` | `BACKUP_ENABLED` + R2 env | `object_store` head/list or raw HTTP HEAD | | needs-new |
| `StorageProvider` port (abstraction) | `apps/api/src/ports.ts:18-22` | — | Rust `trait StorageProvider { put/get/delete }` | | consumed across product-media/themes/menu-import, no dedicated test |
| `LocalFsStorageProvider` (dev/no-R2 fallback) | `apps/api/src/lib/local-storage.ts:5-37` | `STORAGE_DIR` (default `tmp/imports`) | `tokio::fs` impl of same trait | | needs-new |
| `getImageUrl` (public URL vs proxy) | `apps/api/src/lib/image-url.ts:1-23` | `R2_PUBLIC_URL`, `APP_BASE_URL` | plain string logic | | `e2e/tests/flow-ui-images.spec.ts:58` |
| Traversal-guarded read proxy (`/images/*`, `/media/*`) | `apps/api/src/routes/spa-proxy.ts:158-176,196+` | — | axum handler + trait | 🔴 | `e2e/tests/media-render.spec.ts:9-21` |

**Rust recommendation:** `object_store` for the base put/get/delete abstraction (matches the simple verb pattern); `aws-sdk-s3` specifically retained for presigned-URL generation (product-media direct uploads) since `object_store` has no first-class presign API.

---

## 5. sharp image pipeline

**Verified active** — 4 real transform call sites (13 total "sharp" hits incl. imports/comments).

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Theme logo upload: `resize(512×512, inside).webp(q80)` | `apps/api/src/routes/owner/themes.ts:127-130` | — | `image` crate + webp feature | | needs-new |
| Product image upload: `resize(800×800, inside).webp(q82)` | `apps/api/src/routes/spa-proxy.ts:222-226` | — | `image` crate | | `e2e/tests/flow-ui-images.spec.ts:46-59` |
| Public entry-anchor photo: `.rotate() (EXIF auto-orient).resize(1024×1024, inside).webp(q78)` | `apps/api/src/routes/spa-proxy.ts:279-280` | anonymous upload, IP rate-limited | `image` crate | 🔴 (unauthenticated upload, PII-adjacent photo) | needs-new |
| Logo colour sampling: `resize(48×48, inside).ensureAlpha().raw()` (pixel extraction, no format conversion) | `apps/api/src/lib/brand-extractor.ts:307-311` | called from `POST /api/owner/brand/generate` (`spa-proxy.ts:643`) | `image` crate | | needs-new |

All 4 call sites target **WebP only** (never AVIF), quality 78-82, resize dims scaled per use-case. Rust's `image` crate has weak AVIF encode support but that's moot here since WebP is the only target format. Recommend `image` crate in-process (not a sidecar) — none of the transforms need libvips-specific features (no PDF/SVG rasterization, no complex compositing), avoiding sharp's native-binary (libvips) deploy dependency.

---

## 6. tesseract OCR + PDF menu-import

**Verified active** (tesseract.js call confirmed, not a dead dep — see dependency-appendix correction note below re: grep false-negative on this file).

`Tesseract.recognize(input.bytes, 'sqi+eng', {...})` — `apps/api/src/lib/ai-ocr-parser.ts:408`. Languages: **`sqi+eng`** (Albanian + English) hardcoded, matching the product's Albania locale (default currency `ALL`). Alternate OCR engine: **PaddleOCR** via Python subprocess, config-selectable (`MENU_OCR_ENGINE` env or `config.ocr_engine`), one-shot `execFileSync` per image (`ai-ocr-parser.ts:308-331`).

PDF: `pdfjs-dist` — `ai-ocr-parser.ts:349-352` pre-loads the worker module (`import('pdfjs-dist/build/pdf.worker.mjs')`) onto `globalThis` to dodge esbuild bundling, then `pdfjs.getDocument({data}).promise` → per-page `getTextContent()`, reconstructing visual lines from `item.transform[5]` y-coordinate (`:354-376`). Scanned/image-only PDFs are **not** rasterized+OCR'd server-side — deliberate: returns `PARSE_ERROR`/`OCR_LOW_QUALITY` telling the owner to re-upload as an image (`:385-396`), avoiding a heavy native-canvas dependency.

Parsers DI wiring: `apps/api/src/server.ts:299-302` constructs `const parsers = { 'csv': new CsvMenuParser(), 'ai-ocr': new AiOcrParser(memoryService) }`, passed at `server.ts:525` into `registerCoreRoutes(fastify, {..., parsers, ...})`; `apps/api/src/bootstrap/routes.ts:73` types it `Record<string, MenuParserProvider>`, wired into `menuImportRoutes` at `:124`.

Flow: **Upload** — `POST /api/owner/menu/import/preview` (`routes/owner/menu-import.ts:24-163`), mime-based routing (pdf/image/csv, `:82-93`) dispatches to `parsers[source].parse(...)` (`:101-106`). **Parse** — `AiOcrParser.parse()` (`ai-ocr-parser.ts:333`; PDF-text branch `:342-396`; tesseract/paddle OCR branch `:397-422`); PII redaction happens **before** the LLM call (`piiRedactor.redact(rawText)` at `:456`, fail-closed dense-PII gate `:432-441` per ADR-0011); LLM structuring via `LLM_PROVIDER` env (default `llama3.1:8b-instruct`, also groq/openai/openrouter/zen) at `:460+`. **Confirm** — `POST /api/owner/menu/import/commit` (`menu-import.ts:231-589`) writes categories/products/modifier-groups/modifiers via `withTenant` in one transaction (`:253-520`), stamps `menu_confirmed_at`, best-effort auto-branding. Note: `routes/owner/menu-confirm.ts:8-28` is a **separate, narrower** route — only flips `allergens_confirmed=true` on a single product (food-safety liability gate), unrelated to the OCR/PDF pipeline despite the similar name.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| tesseract.js OCR call | `apps/api/src/lib/ai-ocr-parser.ts:408-412` | lang `sqi+eng` hardcoded; `MENU_OCR_ENGINE` selects tesseract vs paddle | `leptess` if in-process, or keep tesseract.js/PaddleOCR as an out-of-process sidecar (recommended — OCR is already subprocess-shaped, not hot-path) | 🔴 (menu photos can carry incidental PII) | `apps/api/tests/paddle-ocr-seam.test.ts:26,51` (proves engine-swap seam) |
| PaddleOCR subprocess engine | `ai-ocr-parser.ts:308-331` | `PADDLE_OCR_PYTHON`, `PADDLE_OCR_SCRIPT`, `MENU_OCR_ENGINE=paddle` | keep as sidecar (Python), same rationale | | `apps/api/tests/paddle-ocr-seam.test.ts:1-60` |
| PDF text extraction | `ai-ocr-parser.ts:344-378` | none | `pdfium-render` preferred over `lopdf` (handles text+positions cleanly, closer semantic match to pdfjs-dist) | | `apps/api/tests/ai-ocr-parser.test.ts:44-96` |
| Scanned-PDF reject path | `ai-ocr-parser.ts:385-396` | — | same handler, no OCR fallback (product decision) | | `apps/api/tests/ai-ocr-parser.test.ts:83-95` |
| PII redaction before LLM egress | `ai-ocr-parser.ts:432-441,456` | ADR-0011 binding | port `pii-redactor` logic directly | 🔴 | `apps/api/tests/menu-region-pii.test.ts`, `ocr-redaction.test.ts` |
| Parsers DI wiring | `server.ts:299-302,525`; `bootstrap/routes.ts:73,124` | `LLM_PROVIDER`,`GROQ_API_KEY`,`OPENAI_API_KEY`,`OPENROUTER_API_KEY`,`OPENCODE_ZEN_API_KEY` | axum app-state holding `Arc<dyn MenuParserProvider>` map | | `e2e/tests/groq-import-proof.spec.ts:17-24,54-65` |
| Menu-import upload/commit routes | `routes/owner/menu-import.ts:24,173,231` | rate limits (5/min preview, 1/min anon) | axum handlers, same tx-per-commit shape | 🔴 (uploaded PDFs may carry contact-block PII) | `e2e/tests/groq-import-proof.spec.ts` |
| menu-confirm.ts (allergen confirm, adjacent) | `routes/owner/menu-confirm.ts:10-27` | — | trivial axum handler | 🔴 (food-safety liability) | needs-new |

---

## 7. Menu translate

**Verified active — LibreTranslate**, not an LLM. `apps/api/src/lib/libretranslate-provider.ts:5-68` POSTs to `process.env.TRANSLATION_ENDPOINT || 'http://localhost:5000/translate'` — a self-hosted LibreTranslate instance (default localhost implies it runs co-located, not a public API). Body: `{q:texts[], source, target, format:'text', api_key:''}` (`:26-36`). Degrades to passthrough (untranslated text tagged `model_id:'fallback_degraded'`) after 3 consecutive failures (`:15-23`); request-level failure returns `model_id:'fallback_error'` (`:54-66`) — never throws to caller.

The task's suspected `"translation"` npm package **does not exist**: `grep -n '"translation"' apps/api/package.json` → no match; no import of any such package found anywhere.

Route: `POST /locations/:id/menu/translate` (`routes/owner/menu-translate.ts:10-219`), rate-limited 1/min/location, iterates target locales, batch-translates categories/products/modifiers with context tags, respects manual-edit lock (`is_auto=false` skips unless `force`), writes `*_translations` rows with `is_auto=true`. `TRANSLATION_PROVIDER` env exists as a label only — doesn't switch behavior (`LibreTranslateProvider` hardcoded at `server.ts:309`).

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| `LibreTranslateProvider.translate()` | `apps/api/src/lib/libretranslate-provider.ts:10-67` | `TRANSLATION_ENDPOINT` (default `http://localhost:5000/translate`), `TRANSLATION_PROVIDER` (label only) | `reqwest::Client` POST, same JSON shape | | needs-new (no test file matching translate/libretranslate found) |
| Degradation fallback (3 failures → passthrough) | `libretranslate-provider.ts:15-23,54-66` | in-memory counter | same circuit-breaker struct | | needs-new |
| menu-translate route | `routes/owner/menu-translate.ts:10-219` | rate limit 1/min/location; `is_auto` manual-edit lock | axum handler, same tx-scoped upsert loop | | needs-new |
| `TranslationProvider` port wiring | `server.ts:309`, `bootstrap/routes.ts:74,125` | hardcoded to LibreTranslateProvider despite `TRANSLATION_PROVIDER` env existing | Rust `trait TranslationProvider` | | needs-new |
| "translation" npm package | n/a — confirmed absent | n/a | n/a | | `grep -n '"translation"' apps/api/package.json` → no match |

**Gap flagged:** zero existing automated test coverage (unit or e2e) for the translate route or provider — needs tests before/alongside the Rust port.

---

## 8. OTP (One-Time Password) 🔴

**Verified wired but globally dark.** ~130 grep hits for "otp"; real wiring at `bootstrap/routes.ts:54,144`, pre-auth allowlist `server.ts:418-420`, order-flow integration `routes/orders.ts:20-22,158-191,321-346`, preflight `lib/preflight.ts:136-166`, FE modal `apps/web/src/pages/client/CheckoutPage.tsx:39-513,754-760`.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| `OTP_ENABLED` flag (gates send/verify/order flow) | `routes/customer/otp.ts:9,50,128`; `packages/config/src/index.ts:48` (default `'false'`); mirrored `routes/orders.ts:22` | `OTP_ENABLED`, default **false**, absent from `.env.example` | `AppConfig.otp_enabled: bool`, checked at handler entry | 🔴 | `apps/api/tests/test-stage26.ts:169` (400 `OTP_NOT_REQUIRED`) |
| SMS "send" = console.log scaffold, no real gateway | `routes/customer/otp.ts:98-100` (`// SMS scaffold — P5+ real gateway`) | none — zero Twilio/SMS SDK hits repo-wide | no-op stub, defer: `trait SmsProvider` + `LogStubProvider`; real `TwilioProvider` only once a carrier is contracted | 🔴 (phone PII in send path) | needs-new |
| Code generation + argon2id hashing | `lib/otp.ts:11-13` (`crypto.randomInt`), `:16-18` (argon2id, memoryCost 19456/timeCost 2/parallelism 1) | `OTP_CODE_LENGTH=6` | `argon2` crate, same params; raw code never persisted/logged | 🔴 | `apps/api/tests/argon2-params-lock.test.ts:15,59`; `test-stage26.ts:210-211` |
| Storage: `phone_otp.code_hash` (raw never stored; immutability trigger) | `packages/db/migrations/1780421100054_anti-fake-seam.ts:19-28,44-55`; RLS FORCE `:37-41` | — | Postgres table+trigger unchanged; sqlx model with `code_hash` only | 🔴 | `test-stage26.ts:397-406`, `:522-531` |
| Storage: `customer_otp_sessions` (opaque token handoff) | `packages/db/migrations/1780421100057_anti-fake-signals.ts:73-88,106` (15-min, NOT a session token) | — | sqlx repo; 32 random bytes, sha256 stored, raw returned once | 🔴 | `test-stage26.ts:186-229` |
| Rate limit: send 3/15min, verify 5/15min | `otp.ts:36,55-63,114` | `OTP_SEND_RATE_LIMIT=3`/`OTP_VERIFY_RATE_LIMIT=5` | tower-governor + DB sliding-window count (dual layer) | 🔴 | `test-stage26.ts:232-259` |
| Verify lockout: 5 wrong attempts → consume + 1h lockout | `otp.ts:151-178` | `OTP_LOCKOUT_HOURS=1` | same counter semantics, 429+Retry-After | 🔴 | needs-new (lockout branch untested) |
| Expiry: 5-min code, 15-min session, 410 on lapse | `otp.ts:80,86,142,147,160,194` | `OTP_TTL_MS=300000`, `VERIFIED_TOKEN_TTL_MS=900000` | `DateTime<Utc>` reject-at-query-time, domain error `OtpExpired` | 🔴 | needs-new (expiry-lapse path untested) |
| Order-flow server-authoritative re-verify | `orders.ts:158-191,321-346` | preflight `otp_required` reason | same two-path trust model; never trust client-declared verified booleans | 🔴 | `apps/api/tests/preflight.test.ts:115-125,154-176` |

**Summary:** OTP is disabled by default everywhere (no `.env.example` entry) with **no real SMS provider** — the send path is a console.log scaffold explicitly marked "P5+ real gateway." Storage security is strong for a dark feature (argon2id hashing enforced by both app logic and a DB immutability trigger, RLS FORCE, sha256 phone hashing). Rate-limiting is layered and send-path-proven, but verify-lockout and expiry branches are untested. Rust plan: `SmsProvider` trait defaulting to a log-only stub, preserve hashing/RLS/limiter architecture, add lockout+expiry regression tests before ever flipping the flag live.

---

## 9. Crypto/Plisio payments 🔴🔴

**Verified wired, dark by default.** Adapter (`lib/payments/plisio.ts`), registry (`lib/payments/registry.ts:14-21`), webhook (`routes/payments-webhook.ts:13`), checkout initiation (`routes/orders.ts:630-693`), PHP-serialize helper (`lib/php-serialize.ts`). `grep -rln "verifyWebhook|createPlisioAdapter|payments-webhook" apps/api/tests/ e2e/` → **0 hits** — the HMAC verification path is completely untested.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| `PAYMENTS_PREPAID_ENABLED` flag | `lib/payments/registry.ts:6` | default OFF, not in `.env.example` | config bool default false | 🔴 | needs-new |
| `PAYMENTS_CRYPTO_ENABLED` flag + route gate (404 when off) | `registry.ts:7`; `routes/payments-webhook.ts:14` | default OFF | config bool default false; 404 on dark route | 🔴 | needs-new |
| `PAYMENTS_PROVIDER` selector (fail-safe to cash) | `registry.ts:14-21` (plisio only if `PLISIO_SECRET_KEY` set, else cash — "never a half-wired money path") | default `'cash'` | enum config, default Cash | 🔴 | needs-new |
| Plisio create-invoice API call | `lib/payments/plisio.ts:8` (`https://plisio.net/api/v1`), `:43,56` (GET `/invoices/new`; USDT/USDC; callback URLs) | `PLISIO_SECRET_KEY`; callback base `PUBLIC_API_BASE_URL`/`VITE_BASE_URL` | `reqwest::Client` + serde-typed response | 🔴 | needs-new (no HTTP mock test) |
| **Webhook HMAC signature verification** — HMAC-SHA1 over PHP-serialized payload (minus `verify_hash`), constant-time compare, fail-closed | `plisio.ts:69-83` (`crypto.timingSafeEqual` `:79`, try/catch→false `:80-82`, missing/non-string hash fails closed `:75`) | secret = `PLISIO_SECRET_KEY` | `hmac` crate (`Hmac<Sha1>`) + `subtle::ConstantTimeEq` | 🔴🔴 | **needs-new** — zero tests exercise `verifyWebhook` |
| Webhook route: fail-closed 401 (not silent 200), raw-body config | `routes/payments-webhook.ts:13-27` | dark behind crypto flag + provider check | axum route + raw-body middleware | 🔴 | needs-new |
| PHP-serialize helper — **unvalidated wire fidelity against real Plisio** | `lib/php-serialize.ts:1-27`; self-flagged NEEDS-VALIDATION at `:3-4` and `plisio.ts:71-73` | — | custom serializer matching PHP `serialize()` byte format; must be byte-proven vs a live Plisio callback before launch | 🔴 | needs-new |
| `payments` ledger table (integer minor units, UNIQUE provider+ref, CHECK refunded≤captured≤amount, RLS FORCE) | `packages/db/migrations/1790000000083_payments-ledger.ts:24-44,68-75` | — | table unchanged; sqlx model | 🔴 | `apps/api/tests/refund-due-spine.test.ts:145` |
| `payment_events` append-only ledger (signature_verified bool, idempotency UNIQUE) | same migration `:49-64` | — | append-only insert model | 🔴 | `refund-due-spine.test.ts:149-150,223,277-285` |
| `payment_location_by_provider_ref` SECURITY DEFINER resolver | migration `:98-111`; called `payments-webhook.ts:34` | — | **preserve as Postgres DEFINER fn** — resolves tenant for unauthenticated webhook before RLS-scoped writes | 🔴 | needs-new |
| `PLISIO_SECRET_KEY` env var | `registry.ts:16`; not in `.env.example` | unset → silent safe fallback to cash | `secrecy::Secret<String>`, required iff provider=plisio | 🔴 | needs-new |

**🔴 Named finding (per task requirement):** Webhook signature verification is **present and correctly shaped** — HMAC-SHA1 over a PHP-serialized copy of the payload keyed by `PLISIO_SECRET_KEY`, compared with `crypto.timingSafeEqual` (constant-time, not naive `===`), failing closed to 401 (never a silent 200). Two caveats: (1) the PHP-serialize key-ordering fidelity is **self-flagged as never validated against a real Plisio callback** — a mismatch would reject all legit webhooks (fail-closed, launch-blocking but not exploitable); (2) **zero test coverage** of `verifyWebhook`/the adapter/the route. Whole vertical dark (both flags off), defensive fallback to no-op cash on misconfiguration.

---

## 10. GDPR export/erase/anonymizer 🔴

**Verified always-on, no feature flag.** Routes registered unconditionally (`bootstrap/routes.ts:140`), workers started unconditionally (`bootstrap/workers.ts:99-104`), `anonymizer-retention` in the `WORKER_CRITICAL_LIST` heartbeat set. **Key negative finding: no GDPR export/Art.15 subject-access endpoint exists** — exhaustive grep for subject-access/DSAR/export patterns returned nothing. Only erasure (Art.17) is implemented.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| GDPR **export endpoint: NOT IMPLEMENTED** | (absent — grep-verified) | — | greenfield `gdpr::export` module if Art.15 required | 🔴 | needs-new (feature absent) |
| Erasure-request CREATE (cross-tenant guard: 404+security-log on foreign tenant) | `routes/owner/gdpr.ts:33-136` (guard `:74-85`, enqueue `:131-133`) | rate-limit 30/min; always-on | sqlx handler: resolve phone→customer, same-tenant proof, INSERT+enqueue | 🔴 | `apps/api/tests/gdpr-authz.test.ts:116-181` |
| Erasure-request LIST (masked)/GET+audit join | `gdpr.ts:139-196,199-254` (`maskName` `:184`; join `anonymization_audit_log` `:220-227`) | — | sqlx + serde masking; never serialize raw customer_id | 🔴 | needs-new (masking unasserted) |
| Retention settings GET/PUT (30-2555 days) | `gdpr.ts:257-287`; DB CHECK `1780421100060_anonymization-seam.ts:9` | — | typed range + keep DB CHECK | 🔴 | `apps/api/tests/test-stage30.ts:297-331` |
| `GdprErasureWorker`: queue consume + fail-loud backstop (re-read `anonymized_at` before completing) | `workers/anonymizer-gdpr.ts:16-20,22-94` (`FOR UPDATE SKIP LOCKED`) | pg-boss singletonKey | Rust worker; same "never complete on unconfirmed effect" discipline | 🔴 | `apps/api/tests/anonymizer-gdpr-backstop.test.ts:96-127` |
| Worker retry: 3× backoff `2^n*60s`, reset to pending | `anonymizer-gdpr.ts:138-165` | hardcoded | same state machine (no stranded in_progress) | 🔴 | `anonymizer-gdpr-backstop.test.ts:127-150` |
| Provenance audit insert | `anonymizer-gdpr.ts:103-132` (`actor_location_id`/`subject_location_id`/`request_id`) | — | sqlx INSERT, identical fields | 🔴 | `apps/api/tests/anonymizer-gdpr-worker-provenance.test.ts:61-96` |
| `anonymizeCustomer` — **in-place UPDATE, not delete** (`phone='anon_'+uuid`, `name=NULL`, `marketing_opt_in=false`, `anonymized_at=now()`) | `apps/api/src/lib/anonymizer/index.ts:124-212` (UPDATE `:148-156`; fail-closed on missing locationId `:127-130`) | — | sqlx txn: `SELECT...FOR UPDATE` then masked UPDATE | 🔴 | `test-stage30.ts:167-169,244-260` |
| `anonymizeOrder` — NULLs client_ip_hash/delivery_address/delivery_instructions/messenger fields | `anonymizer/index.ts:214-283` (UPDATE `:237-249`) | — | same pattern | 🔴 | needs-new (order path unasserted) |
| Retention worker: nightly cron sweep, advisory lock, batch | `workers/anonymizer-retention.ts:21-28,33,79-91`; scan `anonymizer/index.ts:285-309` | `ANONYMIZER_RETENTION_CRON` default `'0 3 * * *'`, `ANONYMIZER_RETENTION_BATCH_SIZE`=100 | tokio-cron + sqlx batch; **keep the Postgres advisory lock** (multi-instance exclusion) | 🔴 | `test-stage30.ts:340-357` |
| DEFINER fn `erase_shadow_tenant` (**LIVE**) | `packages/db/migrations/1790000000082_phase1c-bootstrap-fns.ts:36-50`; caller `modules/acquisition/provisioning.ts:235` | always-on | **preserve as Postgres DEFINER fn, do not reimplement in Rust** — cross-tenant DELETE the app role has no grant for | 🔴 | `apps/api/tests/retention-sweep.test.ts:48-52` |
| DEFINER fn `gdpr_erase_customer` (**STAGED DRAFT, NOT APPLIED**) | `docs/design/audit-fix-rls-reliability/migration-drafts/1790000000088_gdpr-erase-definer.ts:4,14-16,38-81` | gated behind LC4-MIG/GATE-FLIP-E2E | preserve as Postgres fn when it lands — RLS-visibility bypass is structural | 🔴 | needs-new (requires rehearsal-DB proof) |
| Tables `gdpr_erasure_requests` + `anonymization_audit_log` (RLS FORCE) | `1780421100060_anonymization-seam.ts:12-24,32-42,46-59` | — | sqlx models; RLS stays in Postgres, never app-layer filtering | 🔴 | `apps/api/tests/phase5/rls-adversarial.test.ts:35,82` |

**Summary:** Erase-only flow: owner submits request → tenant-scoped validation (404+security-log on foreign tenant) → `gdpr_erasure_requests` row → pg-boss job → `GdprErasureWorker` → `AnonymizerService` in-place masking UPDATE on `customers`/`orders` (never hard delete) → fail-loud re-read backstop before marking completed → retry 3× → audit append. Retention worker sweeps nightly (default 03:00, per-location retention 30-2555 days) under advisory lock. **No export/Art.15 endpoint exists** — greenfield gap. Two DEFINER functions must stay in Postgres: `erase_shadow_tenant` (live) and `gdpr_erase_customer` (staged draft). Always-on, no feature flag.

---

## 11. Backup/DR 🔴

**Verified heavily wired, not dormant.** `BackupCronWorker`+`BackupVerifyWorker` started from `server.ts` behind `BACKUP_ENABLED`; pg-boss cron per cadence; operator CLIs in `scripts/backup-{restore,verify,list,drill}.ts`; admin routes under the platform-admin plane.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Dump: `pg_dump --format=custom --compress=9 --no-owner --no-acl --quote-all-identifiers`, full-DB | `apps/api/src/workers/backup/dump.ts:15-37` | source DSN `DATABASE_URL_MIGRATIONS` (`workers/backup/index.ts:108`) | `tokio::process::Command::new("pg_dump")`; **sidecar/ops binary, not the API request path** | 🔴 | `apps/api/tests/test-stage20.ts:66-105` (mocked) |
| Upload: encrypted multipart stream to R2, key `dowiz-backups/<env>/<type>/<date>/<id>.enc.parts` | `workers/backup/upload.ts:30-53`; key `index.ts:119-133` | `R2_ENDPOINT`/`R2_ACCESS_KEY_ID`/`R2_SECRET_ACCESS_KEY`/`R2_BUCKET` | `aws-sdk-s3` multipart Upload; sidecar | 🔴 | `test-stage20.ts:66-105` (S3 mocked) |
| Manifest JSON (iv/authTag/keyId/plaintext-sha256/per-table rowCounts) | `upload.ts:55-71`; key `index.ts:151-162` | same R2 vars | `aws-sdk-s3` PutObject; sidecar | 🔴 | needs-new |
| Encryption: AES-256-GCM stream, random 12-byte IV | `workers/backup/encrypt.ts:11-32` (key must be exactly 32 bytes base64, `:14-15`) | `BACKUP_ENCRYPTION_KEY` | `aes-gcm` crate `Aes256Gcm`; sidecar | 🔴 | `apps/api/tests/backup-drill-integrity.test.ts:145-153` |
| Key management: keyring lookup, fail-loud on unknown keyId | `encrypt.ts:44-72` (`BACKUP_KEYRING` deliberately raw-read, outside Zod) | `BACKUP_ENCRYPTION_KEY`, `BACKUP_KEYRING` | env/secret-manager lookup; keep raw read | 🔴 | `backup-drill-integrity.test.ts:145-153` |
| Verify #1 — daily full restore drill (download→decrypt→sha256→`pg_restore` into throwaway sandbox DB→smoke checks; advisory lock 3) | `workers/backup/backup-verify.ts:192-215,286-410`; sandbox `lib/restore-sandbox.ts:28-65` (CREATE/DROP DATABASE via `DATABASE_URL_ADMIN`) | `DATABASE_URL_ADMIN`, `BACKUP_VERIFY_LOCK=3` | `tokio::process::Command` wrapping `pg_restore` into scratch DB, never prod; sidecar | 🔴 | `backup-drill-integrity.test.ts:96-142` |
| Verify #2 — smoke checks: schema presence, strict row-count parity, FK count, business invariants | `workers/backup/smoke-checks.ts:27-50,84-132` | manifest rowCounts from `backup-verify.ts:355-358` | assertion fns over sandbox sqlx pool; sidecar | 🔴 | `backup-drill-integrity.test.ts:67-92` |
| Verify #3 — R2 sample verify every 6h (3 random manifests/7d, decrypt+checksum, `pg_restore --list`, lifecycle drift) | `workers/backup/r2-verify.ts:99-129,131-205`; expected lifecycle `:32-37` | cron `0 */6 * * *` (`backup-verify-scheduled.ts:41-48`) | `GetBucketLifecycleConfiguration`+`pg_restore --list`; sidecar | 🔴 | `apps/api/tests/test-stage32.ts:89-108` (structural) |
| Schedules: full drill daily 04:00 UTC, gated `BACKUP_ENABLED`, pg-boss singleton | `backup-verify-scheduled.ts:19-48` | `BACKUP_ENABLED`, `RESTORE_VERIFY_CRON` default `0 4 * * *`, `RESTORE_VERIFY_FULL_HASH`, `BACKUP_{HOURLY,DAILY,WEEKLY,MONTHLY}_CRON` | tokio-cron-scheduler or systemd timer; NOT in main API process | 🔴 | `test-stage32.ts:31-41` (structural) |
| **Restore-to-prod: NO AUTOMATED PATH** — placeholder script + dry-run-only CLI | `apps/api/src/scripts/restore.ts:38` (placeholder); `scripts/backup-restore.ts:277-286` ("not yet implemented — use --dry-run", prints manual `pg_restore` instructions, exits 1) | `DATABASE_URL_MIGRATIONS`, key vars, R2 vars | Rust ops binary: real `pg_restore` behind explicit confirmation gate — **largest gap to close** | 🔴 | needs-new (no test exercises a prod restore) |
| Audit trail: `backup_audit_log`/`backup_metadata` | `workers/backup/audit.ts:4-82`; migrations `1780421100048-050` | — | thin shared sqlx crate usable by API + ops binary | 🔴 | `test-stage20.ts:110-141` |
| Admin routes: list/verify(rate-limited,single-flight)/dr-report | `routes/admin/backups.ts:13-116`; auth `routes/admin/index.ts:20-21` + platform-admin gate | — | these 3 endpoints stay in the API (operator triggers); pipeline moves to sidecar | 🔴 | `e2e/tests/admin-platform-authz.spec.ts:21-23,93-97`; `apps/api/tests/platform-admin-gate.integration.test.ts:58-60,87-89` |

**Summary:** Dump→encrypt→upload→verify pipeline is genuinely strong (daily restore-drill into a disposable sandbox DB with strict row-count/business-invariant checks, plus 6-hourly R2 sample verification). **No automated restore-to-production exists** — this is a documented manual procedure (DR runbook), not code, and is the single largest rebuild gap. Rust plan: pipeline becomes a sidecar ops binary (`tokio::process::Command`+`aws-sdk-s3`+`aes-gcm`), the 3 admin trigger endpoints stay in the axum API behind the platform-admin guard, and a confirmation-gated real restore command should finally be implemented.

---

## 12. mem0ai

**VERDICT: ACTIVE, but nearly vestigial** — imported at `apps/api/src/lib/memory.ts:1` (`import { Memory, ... } from 'mem0ai/oss'`) — the real npm package's **self-hosted/OSS mode**, not the hosted mem0 SaaS (no mem0 API key needed). Config (`buildMemoryConfig`, `memory.ts:32-58`): LLM provider = Ollama (`MEM0_LLM_MODEL` default `llama3.1:8b`), embedder = Ollama (`MEM0_EMBED_MODEL` default `nomic-embed-text`), vector store = `provider:'memory'` (in-process, **non-persistent, dies on restart**), Ollama endpoint `MEM0_OLLAMA_URL` default `http://localhost:11434`.

Instantiated at `server.ts:293` (non-blocking init, failure swallowed → falls back to no-op), decorated `fastify.memory`, threaded into `AiOcrParser` and `buildNotifications`. **Only one real call site invokes it**: `notifications/workers/index.ts:311` (`this.memory?.recordWorkerAction('notification', ...)` on successful dispatch). All other exposed methods (`getWorkerContext`, `recordUserInteraction`, `getUserContext`, `search`) have **zero callers** — write-only, never read back by any consumer today.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| mem0ai (memory service, Ollama-backed, write-only) | `apps/api/src/lib/memory.ts:1,66`; call site `notifications/workers/index.ts:311` | `MEM0_LLM_MODEL`, `MEM0_EMBED_MODEL`, `MEM0_OLLAMA_URL` | custom `reqwest` client to Ollama HTTP API + simple in-mem vector store — no direct Rust crate exists; **candidate to drop in rebuild** given 1 write-only call site and non-persistent store | | `apps/api/tests/ai-ocr-parser.test.ts` |

**Verdict for the report:** ACTIVE (real import, real HTTP calls to a local Ollama instance) but low-value — non-persistent, write-only, unread. Recommend dropping in the Rust rebuild unless a concrete read-path consumer is defined first.

---

## 13. Maps-related server-side

**No server-side Google Maps/Mapbox/geocoding integration found.** Every `googleapis.com` hit is Google Fonts CDN (CSP) or the Google OAuth token endpoint (`routes/auth.ts:84`, `oauth2.googleapis.com/token`) — auth, not maps. `packages/platform/src/stubs.ts:5-25` defines a `GeocodingProvider`/`StubGeocodingProvider` — confirmed **dead/inert**, zero consumers.

The one real "maps-adjacent" server-side integration is **road routing**, not maps display: `packages/platform/src/routing-provider.ts:141` `OrsRoutingProvider` POSTs to `${ROUTING_BASE_URL}/v2/directions/driving-car/geojson` — OpenRouteService-shaped directions API (default `ROUTING_BASE_URL=https://api.openrouteservice.org`; `ROUTING_PROVIDER` picks `ors`|`self`(OSRM)|`haversine`, default `ors`; optional `ROUTING_API_KEY`). Circuit breaker + haversine fallback on failure — never blocks a delivery. Consumed via `apps/api/src/lib/routing.ts` (Redis-backed route cache/claim locks). Client-side-only CSP allowances for `tiles.openfreemap.org`/`router.project-osrm.org` (browser map tiles) are out of scope for this server-side census. `brand-extractor.ts`'s SSRF-guarded fetches of a restaurant's own website (logo/color scrape) are unrelated to maps.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Routing (OpenRouteService/OSRM directions) | `packages/platform/src/routing-provider.ts:141`; `apps/api/src/lib/routing.ts` | `ROUTING_PROVIDER`, `ROUTING_BASE_URL`, `ROUTING_API_KEY` | `reqwest` client, same OSRM/ORS directions contract | | needs-new |
| GeocodingProvider stub | `packages/platform/src/stubs.ts:5-25` | none | N/A — dead seam, drop | | needs-new |

---

## 14. Turnstile 🔴 (dead code finding)

`apps/api/src/plugins/turnstile.ts` (51 lines): decorates `fastify.verifyTurnstile`, reads `request.body.turnstile_token`, POSTs to `https://challenges.cloudflare.com/turnstile/v0/siteverify` (line 20) with `{secret, response, remoteip}`. Missing token → 403 `challenge_required`; verification failure → 403 `challenge_failed`; network error → 503 `challenge_unavailable`.

**CRITICAL FINDING: the plugin is built but never wired in.** No `fastify.register(turnstilePlugin, ...)` anywhere (only self-references + one stale comment in `types/fastify.d.ts:6`). No route uses `verifyTurnstile` as a preHandler or reads `turnstilePassed`. No `TURNSTILE_SECRET_KEY` entry exists in `packages/config/src/index.ts` at all. Zero test coverage. **Net effect: bot/abuse defense was scaffolded but is completely inert in the running app today.**

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Turnstile verify plugin (never registered/used anywhere) | `apps/api/src/plugins/turnstile.ts:20` | no `TURNSTILE_SECRET_KEY` in config at all | `reqwest` POST to siteverify, if/when revived | 🔴 | needs-new (zero test coverage; currently dead code) |

---

## 15. Exchange rates

Source: `apps/api/src/workers/rates-refresh.ts:7` — `https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/all.min.json`, a free, keyless, community-maintained currency-rates JSON on jsDelivr CDN (not a dedicated forex provider). Schedule: `RATES_CRON` env, default `'0 * * * *'` (hourly, `:16`), pg-boss cron + immediate 5s-after-boot kick (`:22`); advisory lock `pg_try_advisory_lock(8192)` (`:33`) ensures single-instance refresh. Only the ALL↔EUR pair is derived (reciprocal of `eur.all`, `:59`), upserted into `exchange_rates` (`base_currency`/`target_currency`/`rate`/`source='fawazahmed0'`/`fetched_at`). Consumer: `routes/public/rates.ts` (`GET /v1/rates`) reads latest row; empty table → static fallback (`rate: 0.0099`, 300s cache) so the storefront's secondary-currency display never breaks. No API key needed.

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Rates fetch (jsDelivr currency-api) | `apps/api/src/workers/rates-refresh.ts:7,25,61` | `RATES_CRON` (default hourly) | `reqwest` client + same Postgres table | | needs-new |
| `GET /v1/rates` (with static fallback) | `apps/api/src/routes/public/rates.ts:14` | — | axum handler | | needs-new |

---

## 16. Anything else found

### Sentry
Genuine, heavily-customized wrapper: `apps/api/src/lib/sentry.ts` (113 lines) — aggressive `beforeSend`/`beforeBreadcrumb` PII redaction (via the same `PiiRedactor`), cookie/header stripping, user-id-only, tag allowlisting. Init at `server.ts:62` (`initSentry(env.SENTRY_DSN, env.GIT_SHA)`). Env: `SENTRY_DSN` (optional). Tests: `test-stage31/32/34.ts`.

### crt.sh / RSSHub
**Not code-integrated anywhere in the tracked repo.** Only hits are in `docs/security/hardening-findings-2026-07-02.md` (a report) and `loops/runs/plane-events-2026-07.jsonl` (a runtime log of an ad-hoc `asset-scan` step that failed with "crt.sh unreachable"). No `tools/`/`apps/`/`packages/` source references. Contrary to project-memory phrasing, this is an operational/ad-hoc tool invocation, not shipped code.

### Redis (ioredis)
Two independent real usages, both pure KV/lock — **not** pub/sub despite a stale comment in `packages/config/src/index.ts:10` ("upstash, pub/sub only"): (1) `server.ts:352` (`new Redis(env.REDIS_URL)`, `fastify.redis`) used in `routes/auth.ts:46,76,78,161,180,183` as short-TTL KV for Google OAuth state/code exchange; (2) `lib/routing.ts:123-127` (separate lazy connection `routeRedis()`) caches the authoritative `RouteResult` per order (`SET...EX 2h`) and provides `claimOnce` NX locks for single-instance re-route computation. Note: `RedisMessageBus` (`server.ts:5,227`) is **not actually Redis** — `packages/platform/src/message-bus.ts:242-243` aliases it directly to `PgMessageBus` (Postgres LISTEN/NOTIFY); misleading name, real backing store is Postgres.

### pg-boss
Durable job queue/scheduler backing ~20 worker classes (settlement-cron, dwell-monitor, courier-dispatch, anonymizer-gdpr/retention, backup hourly/daily/weekly/monthly+verify, liveness-checker, signal-raiser, access-request-notify/retention/reconcile, reconciliation nightly, rates-refresh, order-timeout-sweep, courier-cron gps-purge/stale-check, courier-offer-sweep, dwell-escalation, notify-dispatch/customer-status/telegram-send, velocity-flush, free-tier-watch). Known version-skew flagged in comments (`bootstrap/workers.ts:137-138`): `packages/platform` pins pg-boss `^10` while `apps/api` types against pg-boss `^12`, bridged with `as unknown as PgBoss` casts.

### /metrics
`apps/api/src/lib/metrics.ts` (144 lines) — deliberately zero-dependency, hand-rolled Prometheus text-exposition exporter (explicitly not `prom-client`, per its own comment). `GET /metrics` is dark by default (404 unless `METRICS_TOKEN` set), then requires constant-time-matched Bearer token. Pure in-process logic serving a standard exposition format for an external scraper. Test: `apps/api/tests/metrics.test.ts`.

### PII logic (not integrations — pure in-process)
`security/headers.ts`, `pii-cipher.ts` (AES-256-GCM via Node's built-in crypto), `pii-redactor.ts`, `pii-mask.ts`, `pii-leak-detector.ts`, `ip-hash.ts` (SHA-256 daily-salted), `client-ip.ts` (Fly-edge-header trust model, never trusts `X-Forwarded-For`) — all confirmed pure logic, no third-party service calls.

### Webhook receivers
Exactly two in the whole repo: `routes/payments-webhook.ts` (Plisio, dark) and `routes/telegram-webhook.ts` (gated by `TELEGRAM_BOT_SECRET`). No others (`find apps/api/src/routes -iname '*webhook*'`).

| surface item | file:line | env/config | Rust target | 🔴 | proof |
|---|---|---|---|---|---|
| Sentry | `apps/api/src/lib/sentry.ts` (full file); init `server.ts:62` | `SENTRY_DSN`, `GIT_SHA` | `sentry` crate (+ `sentry-tower`) | | `apps/api/tests/test-stage31.ts`,`32`,`34` |
| crt.sh / RSSHub | not code-integrated (docs/log references only) | none | N/A — ad-hoc tool, not shipped code | | n/a |
| Redis — OAuth state KV | `routes/auth.ts:46,76,78,161,180,183`; conn `server.ts:352` | `REDIS_URL` | `redis`/`deadpool-redis` crate | | needs-new |
| Redis — route cache/lock | `lib/routing.ts:123-162` | `REDIS_URL` | `redis`/`deadpool-redis` | | `apps/api/tests/verify-n2.ts` |
| "RedisMessageBus" (actually Postgres LISTEN/NOTIFY) | `packages/platform/src/message-bus.ts:242-243`; `server.ts:227` | n/a | `sqlx` LISTEN/NOTIFY or `tokio-postgres` | | needs-new |
| pg-boss (job queue, ~20 workers) | `bootstrap/workers.ts` (hub); pervasive across `workers/*.ts` | version-skew noted `workers.ts:137-138` | `pgmq` crate, or reimplement Postgres-backed queue (decision point) | | needs-new |
| `/metrics` (hand-rolled Prometheus exporter) | `apps/api/src/lib/metrics.ts` (full file) | `METRICS_TOKEN` | `metrics`/`metrics-exporter-prometheus` crate, or keep hand-rolled text format | | `apps/api/tests/metrics.test.ts` |
| PII logic (cipher/redactor/mask/leak-detector) | `lib/pii-cipher.ts`, `pii-redactor.ts`, `pii-mask.ts`, `pii-leak-detector.ts` | `COURIER_PII_ENCRYPTION_KEY` | pure logic — `aes-gcm`/`ring` crate + regex | | n/a (not an integration) |
| ip-hash / client-ip | `lib/ip-hash.ts`, `client-ip.ts` | `IP_HASH_SALT` | pure logic — `sha2` crate | | n/a (not an integration) |
| Webhooks (payments, telegram) | `routes/payments-webhook.ts`, `telegram-webhook.ts` | `TELEGRAM_BOT_SECRET`, `PAYMENTS_CRYPTO_ENABLED` | axum route handlers | | needs-new |

---

## Dependency Classification Appendix

`apps/worker/package.json` "dependencies" contains **only** 4 internal `@deliveryos/*` workspace packages — zero third-party deps. All census below is for `apps/api/package.json`.

**Correction / grep-hazard note:** `apps/api/src/lib/ai-ocr-parser.ts` (and, harmlessly, `apps/api/src/lib/courier-relay-guard.ts`) contain 2 stray null bytes that make plain GNU/ripgrep-style recursive grep treat the file as binary and silently skip it — this initially produced a false "zero usages" reading for `tesseract.js` and `pdfjs-dist`. Confirmed real usage via `grep -a` / `Read`. Anyone re-running this census should pass `-a` to grep or use `Read` for these two files.

**No dead dependencies found** in `apps/api/package.json` once the grep-hazard above is corrected for.

| package | classify | evidence | proposed Rust crate |
|---|---|---|---|
| @aws-sdk/client-s3 | integration | 4 files (R2/S3 storage provider) | `aws-sdk-s3` |
| @aws-sdk/lib-storage | integration | co-imported for multipart upload | `aws-sdk-s3` (built-in multipart) |
| @aws-sdk/s3-request-presigner | integration | co-imported for presigned URLs | `aws-sdk-s3` presigning API |
| @deliveryos/config, @deliveryos/db, @deliveryos/domain, @deliveryos/platform, @deliveryos/shared-types | core-framework (internal workspace) | monorepo internal packages | fold into Rust workspace crates (`config`, `db`, `domain`, `platform`, `shared-types`) |
| @fastify/cors | core-framework | `server.ts:140` | `tower-http` `CorsLayer` |
| @fastify/multipart | core-framework | `server.ts:355` | axum built-in multipart extractor |
| @fastify/rate-limit | core-framework | `server.ts:360` | `tower-governor` |
| @fastify/static | core-framework | `server.ts:156` | `tower-http` `ServeDir` |
| @sentry/node | integration | `lib/sentry.ts` full file, init `server.ts:62` | `sentry` crate |
| @tabler/icons-webfont | integration (static asset) | `ssr-renderer.ts:363`, `public/admin/dashboard.html:7` — served as `/dist/tabler/tabler-icons.min.css` | N/A — static font asset via `tower-http::ServeDir` |
| @types/pg | core-framework (types-only, dev-adjacent) | typings for `pg` | N/A — `sqlx`/`tokio-postgres` have native types |
| argon2 | integration | 6 files (password + OTP-code hashing) | `argon2` crate |
| fastify | core-framework | app-wide | `axum` |
| fastify-plugin | core-framework | `plugins/auth.ts`, `plugins/turnstile.ts` | N/A — no plugin-wrapper convention needed |
| fastify-type-provider-zod | core-framework | 44 files (schema validation) | `serde` + `validator`/`garde`, or `utoipa` |
| htm | integration (SSR) | `ssr-renderer.ts:3`, `ssr-client-renderer.ts:3` | N/A — replaced by `askama`/`maud` templating |
| ioredis | integration | `server.ts:352` (OAuth KV) + `lib/routing.ts` (route cache/lock) | `redis`/`deadpool-redis` |
| jose | integration | `routes/auth.ts:106` (dynamic import, `decodeJwt` for Google OAuth id_token) | `jsonwebtoken` crate |
| lru-cache | integration | 1 file (`ssr-renderer.ts` SSR HTML cache) | `moka` or `lru` crate |
| mem0ai | integration | `lib/memory.ts:1` + `notifications/workers/index.ts:311` (1 write-only call site) | custom reqwest-to-Ollama client — low value, candidate to drop |
| pdfjs-dist | integration | `lib/ai-ocr-parser.ts:349-352` (PDF menu import) — see grep-hazard note above | `pdfium-render` (preferred) or `lopdf` |
| pg | core-framework | db pool app-wide | `sqlx` or `tokio-postgres` |
| pg-boss | integration | ~20 worker files | `pgmq` crate, or reimplement Postgres-backed queue (decision point) |
| pino | core-framework | logger | `tracing` + `tracing-subscriber` |
| preact | integration (SSR) | `ssr-renderer.ts:1`, `ssr-client-renderer.ts:1` | N/A — see SSR note below |
| preact-render-to-string | integration (SSR) | `ssr-renderer.ts:2` | N/A — see SSR note below |
| sharp | integration | 4 call sites (theme logo, product image, entry photo, brand-color sampling) | `image` crate |
| tesseract.js | integration | `lib/ai-ocr-parser.ts:7,340,400` (OCR menu import) — see grep-hazard note above | `leptess` bindings, or keep tesseract.js/PaddleOCR as sidecar (recommended) |
| web-push | integration | 2 files (VAPID push notifications) | `web-push` crate |
| ws | core-framework | `apps/api/src/websocket.ts` (real-time courier/order updates) | `tokio-tungstenite` / axum WS upgrade |
| zod | core-framework | 57 files | `serde` + `validator`/`garde` |

**SSR investigation (preact/preact-render-to-string/htm/@tabler/icons-webfont):** not stray frontend deps — they power a genuine server-side rendering path. `apps/api/src/lib/ssr-renderer.ts` and `ssr-client-renderer.ts` import `h` from preact, `render` from preact-render-to-string, and `htm` (tagged-template JSX-without-build-step) to render the **public storefront menu HTML** server-side for bot/crawler user-agents (browsers get the SPA shell — matches prior project memory: "SSR menu bot-UA-only"). Preact was chosen over React for zero build-step dependency via htm. `@tabler/icons-webfont` is the icon webfont CSS linked into that same SSR HTML and static internal admin pages — a pure static asset, no runtime code. In the Rust rebuild: the SSR path becomes an `askama`/`maud` template rendering the same bot-served HTML; the icon webfont is a static file served via `tower-http::ServeDir` — no crate needed.

---

## Report Roll-up

**Integration count:** 16 named integration areas censused (Telegram, Web-push+pipeline, Email, R2/S3, sharp, tesseract+PDF, menu-translate, OTP, Plisio, GDPR, Backup/DR, mem0ai, Maps/routing, Turnstile, Exchange rates, Other-misc) + 1 dependency-classification appendix.

- **Verified-active:** 13 of 16 (Telegram [webhook mode], Web-push+pipeline, Email, R2/S3, sharp, tesseract+PDF, menu-translate, OTP [dark-flagged but wired], Plisio [dark-flagged but wired], GDPR/anonymizer, Backup/DR, mem0ai [low-value], Sentry/Redis/pg-boss/metrics/rates cluster).
- **Verified-dead/inert in production today:** 3 — (1) Telegram **polling mode** (`telegram.poll.ts`/`getUpdates`, code present, never started — webhook is the live path); (2) **Turnstile plugin** (fully built, `challenges.cloudflare.com/turnstile/v0/siteverify` wired, but never registered with Fastify anywhere, no config, zero routes protected — a real gap for bot/abuse defense); (3) **`adapters/push.ts` `PushAdapter`** scaffold (dead code, superseded by `webpush.ts`). Also functionally dead/removed: WhatsApp/Baileys channel (fully retired via migration, remnants are comments only) — not counted as a "found" integration since it no longer exists.
- **mem0ai verdict:** **ACTIVE**, not dead — real import of `mem0ai/oss` (self-hosted mode, Ollama-backed LLM+embedder, non-persistent in-memory vector store), but has exactly one call site (`notifications/workers/index.ts:311`, write-only `recordWorkerAction`) and zero read-path consumers anywhere. Recommend dropping in the Rust rebuild unless a concrete read-path use is defined.
- **🔴 red-line integration count:** 4 of the 16 areas are red-line-heavy by the task's own definition (money/PII/auth/DR): **OTP**, **Crypto/Plisio**, **GDPR/anonymizer**, **Backup/DR** — plus individual 🔴 rows scattered through Telegram (webhook signature), Web-push (VAPID private key, category-prefs/consent), R2/S3 (presigned URLs, unauthenticated entry-photo upload), tesseract+PDF (PII in uploaded menus), and Turnstile (dead bot-defense). Total 🔴-tagged table rows across the document: **60+**.
- **Dependency classification:** `apps/worker/package.json` has 0 third-party deps (all internal workspace packages). `apps/api/package.json` has 28 third-party "dependencies" entries classified as: **10 core-framework** (fastify, @fastify/cors, @fastify/multipart, @fastify/rate-limit, @fastify/static, fastify-plugin, fastify-type-provider-zod, pg, pino, zod, ws — 11 if `ws` counted separately) / **17 integration** (@aws-sdk/client-s3, @aws-sdk/lib-storage, @aws-sdk/s3-request-presigner, @sentry/node, @tabler/icons-webfont, argon2, htm, ioredis, jose, lru-cache, mem0ai, pdfjs-dist, pg-boss, preact, preact-render-to-string, sharp, tesseract.js, web-push) / **0 confirmed dead** (an initial grep pass falsely flagged tesseract.js/pdfjs-dist as dead due to a null-byte grep hazard in `ai-ocr-parser.ts`, corrected via direct Read/grep -a).
- **Additional integrations found beyond the original 16-item list:** Sentry error tracking, Redis (2 distinct usages — OAuth KV + routing cache/lock, despite a misleading `RedisMessageBus` name that's actually Postgres), pg-boss job queue (~20 workers, with a noted version-skew risk), hand-rolled Prometheus `/metrics` exporter, road-routing integration (OpenRouteService/OSRM via `packages/platform/src/routing-provider.ts` — not "Maps" per se but the closest server-side geo-API call in the repo).
- **Could not fully verify:** (1) crt.sh/RSSHub — per project memory these were expected to be "code-integrated," but exhaustive repo-wide grep found them only in a docs report and a runtime JSONL log of an ad-hoc failed tool invocation, never in `tools/`/`apps/`/`packages/` source — treated as **not code-integrated**, flagged as a discrepancy with the memory note rather than asserted as a hard negative beyond what grep can prove. (2) The Plisio PHP-serialize wire-format fidelity is explicitly self-flagged in the source itself as never validated against a live Plisio callback — this is a known-unverifiable-without-a-real-sandbox-transaction gap, not a research miss. (3) Whether GDPR Art.15 export is a genuine intentional product-scope decision or an oversight could not be determined from code alone (routes/config only prove absence, not intent).


---

<!-- ============ §5 BOOT / LIFECYCLE / ENV / FLAGS ============ -->

# BOOT / LIFECYCLE + FEATURE-FLAG + ENV CENSUS
Source of truth for Rust rebuild (`crates/api/src/boot/{env.rs,schema_guard.rs,pools.rs,shutdown.rs}` + `crates/api/src/flags.rs`).
Read fully: `apps/api/src/server.ts` (890 lines), `packages/db/src/index.ts` (63), `packages/config/src/index.ts` (244),
`apps/api/src/plugins/{auth,dev-guard,turnstile}.ts`, `apps/api/src/bootstrap/{routes,workers,notifications,messaging}.ts`,
`apps/api/src/{shutdown,lib/schema-guard,lib/api-error,lib/logger,lib/sentry}.ts`, `apps/worker/src/*.ts`,
`packages/platform/src/auth/tenant.ts`, `Dockerfile`, `fly.toml`.

---

## 1. Env preflight census

**Registry (validated, typed) — `packages/config/src/index.ts` `EnvSchema` (Zod), lines 1-206.**
This *is* the flag/env registry: `loadEnv()` (line 210) calls `EnvSchema.safeParse(process.env)`,
throws with an aggregated issue list on any failure (no partial boot), then runs
`assertDevAuthDisabledInProd(env)` (line 230) as a second, semantic guard.

Extraction — count of `EnvSchema` object keys:
```
grep -c ": z\." packages/config/src/index.ts   # counts schema-field lines (approx, some multi-line)
```
Direct field count by reading the schema: **80 declared env vars** (I enumerated every `<NAME>: z...` line
in the 206-line schema body — 80 distinct keys, `NODE_ENV` through `ACCESS_REQUEST_NOTIFY_MAX_ATTEMPTS`).

Extraction — vars read via **raw `process.env.X`** (bypassing the schema — "shadow" env, unvalidated at boot):
```
grep -rhoE "process\.env\.[A-Z_0-9]+" apps/api/src apps/worker/src packages/db/src packages/config/src | sort -u | wc -l
→ 48
```
**Reconciliation**: the two sets overlap only partially. `packages/db/src/index.ts` and the plugins read
`env.DATABASE_URL_OPERATIONAL` etc. **through the typed `env` object** (schema-validated), so they don't
appear in the raw-`process.env` grep. The 48 raw hits are read directly, several *duplicating* a
schema-validated name (e.g. `NODE_ENV`, `LOG_LEVEL`, `FLY_MACHINE_ID`, `IP_HASH_SALT`, `HOSTNAME`,
`RENDER_GIT_COMMIT`, `MEM0_*`, `TRANSLATION_*`, `VAPID_*`, `R2_*`, `BACKUP_ENCRYPTION_KEY`,
`ENFORCE_VENUE_HOURS`, `WORKER_HEARTBEAT_INTERVAL_MS`) — those are validated-but-also-read-raw
in a handful of call sites instead of threading the typed `env`. The remainder are **never in the
Zod schema at all** — genuinely unvalidated at boot: `ACQUISITION_RETENTION_CRON`,
`ACQUISITION_SHADOW_TTL_DAYS`, `BACKUP_KEYRING`, `COURIER_OFFER_HANDSHAKE_ENABLED`,
`COURIER_OFFER_TTL_MIN`, `DATABASE_URL_ADMIN` (schema has it, optional — dup), `DELIVERY_TRACE_GPS_RETENTION`,
`DELIVERY_TRACE_RETENTION_CRON`, `METRICS_TOKEN`, `PAYMENTS_CRYPTO_ENABLED`, `PAYMENTS_PREPAID_ENABLED`,
`PAYMENTS_PROVIDER`, `PLISIO_SECRET_KEY`, `PROVISION_OPS_SECRET`, `PUBLIC_API_BASE_URL`, `STORAGE_DIR`,
`TG_CATEGORY_GATING`, `TG_STOREFRONT_ACTION`, `VITE_BASE_URL` (FE var read by a server script, benign),
`VOICE_CONTROL_ENABLED`, `VOICE_KILL`. **Rust note**: unify all env access behind one typed `Env` struct
(serde+validator or a hand-rolled `envy`-style loader) parsed **once** at boot — no direct `std::env::var`
call sites anywhere else in the crate (enforce via a clippy/grep lint), closing exactly this schema/shadow-env split.

### Full table (grouped; "validated" = present in `EnvSchema`; "raw" = also/only read via `process.env.X`)

| var | required? | default | validated how | consumer | Rust note |
|---|---|---|---|---|---|
| `NODE_ENV` | yes | — (enum) | Zod enum(dev/test/production) | everywhere (HSTS gate, sentry env, boot-guard D) | `enum Environment` |
| `PORT` | no | 8080 | `z.coerce.number().int().positive()` | `fastify.listen` server.ts:876 | `u16` w/ default |
| `APP_BASE_URL` | yes | — | `z.string().url()` | OAuth redirect / links | `url::Url` |
| **`DATABASE_URL_OPERATIONAL`** | yes | — | `z.string().url()` | `createOperationalPool()` (transaction-mode :6543) | see §3 |
| **`DATABASE_URL_SESSION`** | yes | — | `z.string().url()` | `createSessionPool()` (session-mode :5432) | see §3 |
| **`DATABASE_URL_MIGRATIONS`** | yes | — | `z.string().url()` | migrator (`release_command`) + `backupPool` (server.ts:214-217) | see §3 |
| `REDIS_URL` | yes | — | `z.string().url()` | `new Redis(env.REDIS_URL)` server.ts:352 (pub/sub only — NOT the MessageBus, that's pg LISTEN/NOTIFY) | `redis` crate |
| `JWT_PRIVATE_KEY` / `JWT_PUBLIC_KEY` / `JWT_KID` | yes | — | `.min(1)` | RS256 signing (`@deliveryos/platform`) | `jsonwebtoken` RS256 keypair |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | yes | — | `.min(1)` | OAuth | — |
| `GOOGLE_OAUTH_ENABLED` | no | `false` | enum | gates `/api/auth/google` routes (BE+FE `VITE_GOOGLE_OAUTH_ENABLED` mirror) | flags.rs |
| `DEV_AUTH_SECRET` | no | unset | `.optional()` | `/dev`,`/api/dev` shared-secret gate (`plugins/dev-guard.ts`) | 🔴 see §6 |
| `ALLOW_DEV_LOGIN` | no | `false` | enum | master dev-bypass gate; boot-guard D FATALs if `true` in prod | 🔴 |
| `DEV_LOGIN_EMAIL` / `DEV_LOGIN_PASSWORD` | no | unset | `.optional()` | seeded dev account creds | — |
| `JWT_DEV_KID` / `JWT_DEV_PRIVATE_KEY` / `JWT_DEV_PUBLIC_KEY` | no | unset | `.optional()` | dev-token signing keypair (segregated kid) | 🔴 boot-guard D checks these too |
| `TELEGRAM_BOT_TOKEN` / `_SECRET` / `_USERNAME` | no | unset | `.optional()` | Telegram adapter + webhook HMAC | — |
| `OTP_ENABLED` | no | `false` | enum | global phone-OTP kill-switch (per-location `require_phone_otp` needs this AND) | flags.rs |
| `MEDIA_RICH_ENABLED` | no | `false` | enum | ADR-0002 cinematic product-media; read via `loadEnv().MEDIA_RICH_ENABLED` in `routes/public/menu.ts:74` (AND `locations.plan='business'`) | flags.rs |
| `FUNNEL_INGEST_ENABLED` | no | `true` | enum | ADR-0009 funnel ingest kill-switch (route always mounted, 204s when off) | flags.rs |
| `ENFORCE_VENUE_HOURS` | no | `false` | enum | closed-venue order-gate parity check (orders.ts) | flags.rs |
| `OPENROUTER_*` / `OPENCODE_ZEN_*` / `GROQ_*` / `OPENAI_*` / `LLM_ADAPTER` / `LLM_PROVIDER` / `*_ENDPOINT` | no | unset | `.optional()` | AI-OCR menu-import provider chain (fallback to heuristic parser) | provider trait, all optional |
| `BACKUP_ENCRYPTION_KEY` | no (required if `BACKUP_ENABLED`) | unset | `.optional()`, not cross-validated by Zod | backup worker | validate at use-site |
| `R2_ACCESS_KEY_ID`/`R2_SECRET_ACCESS_KEY`/`R2_ENDPOINT`/`R2_BUCKET`/`R2_PUBLIC_URL` | no | unset | `.optional()` | `R2StorageProvider` selection: `process.env.R2_BUCKET && process.env.R2_ENDPOINT` (server.ts:306) — **raw env read, not `env.`**, dupes schema | object-storage trait |
| `OPERATIONAL_POOL_SIZE` | no | 20 | `z.coerce.number().int().positive()` | operational pool `max` | see §3 |
| `BACKUP_ENABLED`/`BACKUP_POOL_SIZE`/`BACKUP_*_CRON`/`BACKUP_*_RETENTION_*`/`BACKUP_PII_FIELDS` | no | assorted defaults | enum/coerce/string | backup cron workers | cron scheduler |
| `DWELL_CRON`/`DWELL_TIER2_DELAY_MS`/`DWELL_TIER3_DELAY_MS`/`DWELL_TIER3_ENABLED`/`DWELL_BATCH_THRESHOLD` | no | assorted | coerce/enum | dwell-monitor worker | — |
| `SIGNAL_RAISE_CRON`,`OTP_*_RATE_LIMIT`,`OTP_TTL_MS`,`VELOCITY_*` | no | assorted | coerce | anti-fake-signals (P26) | — |
| `VAPID_PUBLIC_KEY`/`VAPID_PRIVATE_KEY` | **yes** | — | `.min(1)` (not optional!) | web-push; `buildNotifications` only registers the `push` channel if BOTH set (redundant runtime re-check since schema already requires them) | — |
| `VAPID_SUBJECT` | no | `push@deliveryos.app` | `z.string().email().default(...)` | `normalizeVapidSubject` (bootstrap/notifications.ts:20) | — |
| `ANONYMIZER_RETENTION_CRON`/`_BATCH_SIZE`, `R2_RETENTION_OVERRIDE_DAYS` | no | assorted | coerce | GDPR anonymizer workers | 🔴 GDPR-adjacent |
| `SENTRY_DSN` | no | unset | `.optional()` | `initSentry` (only if set) server.ts:61 | `sentry` crate optional init |
| `LOG_LEVEL` | no | `info` | enum(trace..fatal) | pino logger level (ALSO re-read raw via `process.env.LOG_LEVEL` in `lib/logger.ts:77,109` — dup) | `tracing` level filter |
| `WORKER_HEARTBEAT_INTERVAL_MS`/`WORKER_LIVENESS_CHECK_MS`/`WORKER_LIVENESS_STALE_MS`/`WORKER_CRITICAL_LIST` | no | assorted | coerce/string default (8-worker CSV) | `WorkerHeartbeat` + `LivenessChecker` | — |
| `GIT_SHA` | no | unset | `.optional()` | Sentry release tag | — |
| `DATABASE_URL_ADMIN` | no | unset | `z.string().url().optional()` | backup restore-verify (superuser-adjacent, separate from the 3-pool trio) | 🔴 |
| `RESTORE_VERIFY_CRON`/`RESTORE_VERIFY_FULL_HASH`/`RESTORE_POOL_SIZE` | no | assorted | enum/coerce | P32 backup-verify worker | — |
| `RATES_CRON` | no | `0 * * * *` | string default | currency rates refresh worker | — |
| `COURIER_PII_ENCRYPTION_KEY`,`COURIER_ACCEPT_WINDOW_MS`,`CANCEL_AFTER_DISPATCH_WINDOW_MS`,`COURIER_DISPATCH_MAX_ATTEMPTS`,`COURIER_ASSIGN_ACCEPT_TIMEOUT_MS`,`COURIER_GPS_MAX_DIST_KM` | no | unset (all `z.string().optional()` — **not coerced to number** despite being numeric-in-spirit) | `.optional()` | courier dispatch/accept timing | Rust: parse-at-use, or fix upstream to `z.coerce.number()` |
| `DISPATCH_OWNER_GRACE_ENABLED` | no | `false` 🔴 | enum | ships flag-off pending operator ratification (dispatch-exhaustion auto-cancel) | 🔴 ships-dark, ratify before flip |
| `DISPATCH_OWNER_GRACE_MS` | no | unset | `.optional()` string | grace window (default 900000 in code, not schema) | — |
| `FLY_MACHINE_ID`,`HOSTNAME`,`RENDER_GIT_COMMIT` | no | unset | `.optional()` | worker-id derivation (`Heartbeat` ctor default arg) — also read raw in `apps/worker/src/heartbeat.ts:9` | — |
| `GROQ_ENDPOINT`,`LLM_ENDPOINT`,`OPENAI_ENDPOINT`,`TRANSLATION_ENDPOINT`,`TRANSLATION_PROVIDER` | no | unset | `.optional()` | provider endpoint overrides | — |
| `IP_HASH_SALT` | **yes** | — | `.min(1)`, custom message "Required for deterministic PII hashing" | client-IP hashing for privacy-preserving analytics | secret, 🔴-adjacent (PII) |
| `MEM0_EMBED_MODEL`/`MEM0_LLM_MODEL`/`MEM0_OLLAMA_URL` | no | unset | `.optional()` | `getMemoryService()` (mem0 agent memory) | — |
| `ROUTING_PROVIDER` | no | `ors` | `z.enum(['ors','self','haversine'])` | per-leg routing provider select; degrades to haversine if misconfigured | provider enum |
| `ROUTING_BASE_URL` | no | `https://api.openrouteservice.org` | `z.string().url().default(...)` | ORS client | — |
| `ROUTING_API_KEY` | no | unset | `.optional()` | ORS auth | — |
| `ACCESS_GATE_PUBLIC_ENABLED` | no | `false` | enum | STOP-1 gate: route registration (`bootstrap/routes.ts:118`) + FE CTA (`VITE_ACCESS_GATE_PUBLIC_ENABLED` mirror) | flags.rs |
| `ACCESS_GATE_INVITE_GATING_SHIPPED` | no | `false` | enum | companion CI banned-strings gate (scarcity copy) | flags.rs (CI-only really) |
| `RESEND_API_KEY`,`WAITLIST_NOTIFY_EMAIL` | no | unset | `.optional()` | operator email notify for access requests | — |
| `PRIVACY_NOTICE_VERSION` | no | `2026-06-20` | string default | stamped on consent rows; CI content-hash test fails build on drift | — |
| `ACCESS_REQUEST_RETENTION`,`_RETENTION_CRON`,`_RECONCILE_CRON`,`_NOTIFY_MAX_ATTEMPTS` | no | assorted | string/coerce | GDPR retention/reconcile crons | 🔴 GDPR |

**Shadow (unvalidated, `process.env.X` direct-read, no Zod entry) — flag/secret-shaped, worth promoting into the schema on the Rust port:**
`METRICS_TOKEN` (dark unless set — `/metrics` gate), `PROVISION_OPS_SECRET` (acquisition/provisioning ops gate, 404 fail-closed when unset), `PLISIO_SECRET_KEY` + `PAYMENTS_PROVIDER` + `PAYMENTS_CRYPTO_ENABLED` + `PAYMENTS_PREPAID_ENABLED` (crypto-payments vertical, `lib/payments/registry.ts:6-7`), `VOICE_CONTROL_ENABLED` + `VOICE_KILL` (`lib/voice-flag.ts` — combined AND/NOT gate), `COURIER_OFFER_HANDSHAKE_ENABLED`/`COURIER_OFFER_TTL_MIN`, `TG_CATEGORY_GATING`/`TG_STOREFRONT_ACTION` (Telegram FE mirrors), `BACKUP_KEYRING`, `STORAGE_DIR`, `PUBLIC_API_BASE_URL`, `ACQUISITION_RETENTION_CRON`/`ACQUISITION_SHADOW_TTL_DAYS`, `DELIVERY_TRACE_GPS_RETENTION`/`DELIVERY_TRACE_RETENTION_CRON`.

**The known trio — `DATABASE_URL_MIGRATIONS` / `DATABASE_URL_OPERATIONAL` / `DATABASE_URL_SESSION`** (all `z.string().url()`, all required):

| var | port / pooler mode | who uses it | pool size | why a separate pool |
|---|---|---|---|---|
| `DATABASE_URL_OPERATIONAL` | Supavisor **transaction mode** :6543 | `createOperationalPool()` (`packages/db/src/index.ts:17`) → decorated as `fastify.db` (server.ts:209-210); every route handler's hot-path pool | `max: env.OPERATIONAL_POOL_SIZE` (default 20), `idleTimeoutMillis: 30000`, `connectionTimeoutMillis: 5000`, **`SET statement_timeout='10s'`** on connect | Transaction mode multiplexes many logical clients over few physical PG backends — cheap for short request-scoped queries, but **does not support session state / `LISTEN`/`NOTIFY`** or named prepared-statement caching (comment explicitly warns against caching) |
| `DATABASE_URL_SESSION` | Supavisor **session mode** :5432 | `createSessionPool()` (`packages/db/src/index.ts:48`) → `messageBusPool` for `RedisMessageBus`/pg LISTEN-NOTIFY (server.ts:222-223); workers needing `SET`, advisory locks, DDL | `max: 3` (hardcoded, not env-tunable), same idle/conn timeouts, **`SET statement_timeout='30s'`** | LISTEN/NOTIFY requires a held session-scoped connection — the transaction pooler breaks this; also used for pg-boss's own session needs where noted |
| `DATABASE_URL_MIGRATIONS` | Supavisor **session mode** :5432 (comment says so, same as `DATABASE_URL_SESSION` — likely same physical connstring, separate secret slot) | (a) `release_command` migrator (`dist/migrate/index.cjs`, fly.toml:15) runs BEFORE traffic on every deploy; (b) `backupPool` in server.ts:213-217 (`max: env.BACKUP_POOL_SIZE`, default 2) — "Dedicated backup pool to avoid starving operational queries" | migrator: ad hoc connection; backupPool: `env.BACKUP_POOL_SIZE \|\| 2` | Migrations need DDL rights + session semantics; kept **separate from operational** so a long backup/pg_dump never contends with request traffic, and separate from the LISTEN/NOTIFY session pool so a stuck backup can't starve realtime notifications |

Additionally, pg-boss (queue) opens its **own** connection using a same-cluster-but-forced-session-port URL:
`server.ts:243-245` — `const opUrl = new URL(env.DATABASE_URL_OPERATIONAL); opUrl.port = '5432'; new PgBossQueueProvider(opUrl.toString())` — i.e. pg-boss reuses the *operational* connstring's host/creds but **rewrites the port to 5432** (session mode) because "Transaction pooler (port 6543) blocks LISTEN/NOTIFY. Session port is required" (comment, server.ts:240-242). This is effectively a 4th logical pool identity sharing the trio's credentials.

**Why 3 (not 1)**: Supabase/Supavisor exposes the same Postgres via two pooler front-ends (transaction :6543 vs session :5432) plus a bare/admin path; the app needs (1) a large multiplexed pool for stateless per-request reads/writes (operational/transaction), (2) a small pool that holds real sessions open for `LISTEN/NOTIFY` and advisory locks (session), and (3) a migration/backup-privileged path kept isolated from both so DDL and dumps never starve or get starved by hot-path traffic (migrations). `DATABASE_URL_ADMIN` (optional, restore-verify only) is a further-privileged 4th slot outside the trio.

**Rust mapping**: `crates/api/src/boot/pools.rs` — three `sqlx::PgPool` (or `deadpool-postgres`) builders:
`operational_pool()` (max 20, `statement_timeout` session var set via `after_connect` hook, reject-superuser guard ported verbatim — see §3), `session_pool()` (max 3, 30s timeout, used for a dedicated LISTEN/NOTIFY task + advisory locks), `migrations_pool()` (small, DDL-capable, used only by the migration runner + backup worker). sqlx's `PgListener` needs its own session-mode connection exactly like `messageBusPool` here.

---

## 2. Schema-head guard

**File**: `apps/api/src/lib/schema-guard.ts`, function `assertSchemaCurrent(pool: Pool): Promise<void>` (lines 24-66).
**Caller**: `server.ts:226` — `await assertSchemaCurrent(messageBusPool)` (uses the **session pool**, not operational), run immediately after the message bus pool is created, BEFORE `messageBus.connect()`/queue setup/route registration — i.e. very early in boot.

**Exact check logic**:
1. Expected head is a **build-time constant** `__EXPECTED_MIGRATION_HEAD__` (line 22, `declare const`), injected via esbuild `define` in `scripts/build-apps.ts` — the newest migration file's basename (no extension). In **unbundled/dev/tsx runs the define is absent** (`typeof __EXPECTED_MIGRATION_HEAD__ !== 'undefined'` is false) → the guard is a **no-op** in dev (schema-guard.ts:26-29). This is a real gap: dev never gets this protection, by design ("local databases are managed by hand").
2. If a head is stamped: query `SELECT name FROM pgmigrations ORDER BY id DESC LIMIT 1` for logging, then the actual pass/fail check is `SELECT 1 FROM pgmigrations WHERE name = $1 LIMIT 1` with `$1 = expectedHead` (lines 33-44). **Presence, not exact-match-of-latest**: extra/newer migrations beyond `expectedHead` are fine ("ahead is fine — extra migrations never remove what this build needs") — only *missing* the expected row fails.
3. Error-handling nuance: if the query throws because `pgmigrations` table itself doesn't exist (regex match on the error message, line 53) → genuinely behind, falls through to **FATAL exit**. Any *other* transient error (e.g. connection blip) → **warn and continue booting** (lines 53-56) — deliberately fails open on non-definitive errors so a flaky DB check doesn't take down a healthy app.
4. FATAL path: `console.error(...)` with the expected vs applied head, then **`process.exit(1)`** (schema-guard.ts:65) — hard process exit, not an exception the caller can catch/retry.
5. Migration count expected: not a count, a **name/basename match** against the single newest migration's filename (e.g. whatever `pgmigrations` row was stamped as head at build time) — the guard doesn't know or care how many migrations exist, only whether that one newest name is present in the applied set.

**Rust mapping**: `crates/api/src/boot/schema_guard.rs` — a build script (`build.rs`) embeds the expected head migration name as a `const EXPECTED_MIGRATION_HEAD: Option<&str>` (via `env!`/`option_env!` at compile time, mirroring the esbuild `define`); at boot, `sqlx::migrate!()` naturally tracks its own `_sqlx_migrations` table, so the idiomatic replacement is **`sqlx::migrate!().run(&pool)`** as the release/deploy step (fly.toml `release_command`) and, as a defense-in-depth backstop mirroring this guard, a boot-time `SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1` compared against the compiled-in expected version, with the same "present in dev = no-op, missing on prod = `std::process::exit(1)`" semantics. 🔴 **adjacent to red-line** (`packages/db/migrations/` is a red-line glob per the Ship Discipline rule) — any change to this guard's semantics should go through the council, not be silently altered during the port.

---

## 3. Pool setup

**Pools created** (all in `packages/db/src/index.ts` unless noted):

| pool | fn | connstring | size | idle/conn timeout | statement_timeout | RLS role check | file:line |
|---|---|---|---|---|---|---|---|
| Operational | `createOperationalPool()` | `DATABASE_URL_OPERATIONAL` | `env.OPERATIONAL_POOL_SIZE` (default 20) | idle 30000ms / conn 5000ms | `SET statement_timeout='10s'` | **yes** — see below | `packages/db/src/index.ts:17-42` |
| Session (MessageBus) | `createSessionPool()` | `DATABASE_URL_SESSION` | `3` (hardcoded) | idle 30000ms / conn 5000ms | `SET statement_timeout='30s'` | no explicit role check | `packages/db/src/index.ts:48-63` |
| Backup | ad hoc `new Pool()` | `DATABASE_URL_MIGRATIONS` | `env.BACKUP_POOL_SIZE` (default 2) | pg defaults | none set | no explicit role check | `apps/api/src/server.ts:213-217` |
| pg-boss queue conn | `PgBossQueueProvider` | `DATABASE_URL_OPERATIONAL` with port force-rewritten to `5432` | pg-boss internal default | pg-boss internal | pg-boss internal | none observed here | `apps/api/src/server.ts:243-245` |
| Worker session pool | `createSessionPool()` (same fn, separate process) | `DATABASE_URL_SESSION` | `3` | same as above | `30s` | none explicit | `apps/worker/src/index.ts:10` |

**🔴 RLS role guardrail** (`packages/db/src/index.ts:31-39`, inside `createOperationalPool`'s `pool.on('connect', ...)` handler):
```js
pool.on('connect', async (client) => {
  await client.query("SET statement_timeout = '10s'");
  const res = await client.query('SELECT current_user');
  if (res.rows[0].current_user === 'postgres') {
    client.release(true); // Destroy the connection
    throw new Error("SECURITY FAULT: Operational pool connected as 'postgres' superuser. This bypasses RLS. Use a dedicated restricted role.");
  }
});
```
This runs on **every new physical connection** the operational pool opens, and hard-fails (destroys the connection + throws) if the connected role is literally `postgres` (superuser, bypasses RLS). It does **not** positively assert the role is a specific NOBYPASSRLS role by name — it's a negative/denylist check on the single most dangerous identity, not a full role allowlist. The session pool and backup pool have **no equivalent check** — worth flagging as a gap when porting (the guard should arguably apply to all three, or at minimum session, since it also touches tenant data).

**Tenancy mechanism — `withTenant`** (`packages/platform/src/auth/tenant.ts`, 21 lines, full file):
```ts
export async function withTenant<T>(pool: Pool, userId: string, fn: (client: PoolClient) => Promise<T>): Promise<T> {
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query("SELECT set_config('app.user_id', $1, true)", [userId]);
    const result = await fn(client);
    await client.query('COMMIT');
    return result;
  } catch (err) {
    await client.query('ROLLBACK'); throw err;
  } finally { client.release(); }
}
```
Sets `app.user_id` (not literally `app.current_tenant` — the actual GUC name is `app.user_id`) as a **transaction-local** (`set_config(..., true)` third arg = local) session variable, inside an explicit `BEGIN`/`COMMIT`/`ROLLBACK`, so RLS policies keyed on `current_setting('app.user_id')` scope every query in that closure to the calling user — and it resets automatically at transaction end (never leaks across pooled-connection reuse). Call sites: `routes/spa-proxy.ts:302,531,567`, plus `owner/locations.ts`, `owner/product-media.ts`, `owner/alerts.ts`, `owner/courier-invites.ts`, and ~15 more (couriers, orders, workers, gdpr, etc. — 24 files import it total). **Not every route uses it** — many routes rely on `requireLocationAccess`/`requireRole` app-level authz instead of RLS-via-`withTenant`; RLS is defense-in-depth layered on top, not the sole authz mechanism.

**Rust mapping** 🔴: `crates/api/src/boot/pools.rs` builds the three `PgPool`s with sqlx's `after_connect` hook replicating the `current_user != 'postgres'` guard (and ideally hardening it to an **allowlist** of the expected app role name rather than a `postgres`-denylist, if the actual role name is known/stable — flag for council). Tenancy → a `with_tenant<T>(pool, user_id, F) -> Result<T>` helper that opens a transaction, runs `SELECT set_config('app.user_id', $1, true)`, executes the closure, commits/rolls back — a direct 1:1 port. **This whole area (RLS role + tenancy mechanism) is council-gated per the task brief** — don't silently change semantics.

---

## 4. Boot sequence order (`apps/api/src/server.ts`, `main()`, lines 57-890)

1. `loadEnv()` — parse+validate all env vars, FATAL-throw on any invalid (line 58).
2. Sentry init if `SENTRY_DSN` set (lines 61-64).
3. Register `unhandledRejection`/`uncaughtException` process guards — **before** anything else can throw (lines 73-80) — keeps the process alive (WS connections must not die on one bad promise).
4. Construct the Fastify instance: custom `genReqId` (server-authoritative correlation id, never trusts inbound header), Zod validator/serializer compilers, `bodyLimit: 10MB` (lines 82-124).
5. Register global `onRequest` hooks: security headers (HSTS/nosniff/frame-options/referrer-policy, lines 126-136), strict CORS default-deny with per-route override for public menu/order POST (lines 139-154), static file serving + cache-header hook (lines 156-176), subdomain rewrite (`margherita.dowiz.org` → `/s/margherita`, lines 178-185), correlation-id capture (lines 187-204).
6. `fastify.decorate('wss', null)` placeholder (line 206).
7. **Operational pool** created + decorated as `fastify.db` (lines 208-210).
8. **Backup pool** created (`DATABASE_URL_MIGRATIONS`, lines 212-217).
9. **Session pool** (`messageBusPool`) created via dynamic import (line 222-223).
10. **Schema-head guard** — `assertSchemaCurrent(messageBusPool)` — FATAL-exit point if migrations are behind (line 226).
11. `RedisMessageBus` constructed on the session pool + `.connect()` (lines 227-228), then a **hardcoded 100ms sleep** "to ensure LISTEN commands are processed" (line 236) — a race-condition band-aid worth removing/replacing with an ack in the Rust port.
12. **Queue provider** (`PgBossQueueProvider`) constructed on a port-rewritten session URL derived from the operational connstring, `.start()`'d (lines 239-246); then all `ALL_QUEUES` are `createQueue()`'d best-effort (warn-not-fail on DDL-permission errors, lines 251-255).
13. Metrics registered (`registerMetrics`, pool gauges + WS + pgboss-pending-jobs, lines 259-273) — dark unless `METRICS_TOKEN` set.
14. pgboss schema existence sanity check (best-effort warn, lines 276-287).
15. **Memory service** (mem0) initialized async, non-blocking (`.catch` swallows failure, lines 293-297).
16. Menu parsers (csv, ai-ocr) + **storage provider** selection (R2 if `R2_BUCKET`+`R2_ENDPOINT` env set, else local fs) + translation provider constructed (lines 299-309).
17. **Notification providers** built (`buildNotifications` — telegram always, web-push only if VAPID configured, lines 313-317).
18. **Background workers** started under a **3-second boot budget** (`WORKER_BOOT_BUDGET_MS = 3_000`, `Promise.race`, lines 338-350) — if worker startup hangs or throws past budget, log and **proceed to listen anyway** (explicit boot-resilience decision from a real 2026-06-21 incident where worker startup blocked `fastify.listen` and failed Fly's `/livez` check). Workers keep initializing in the background even after the race "wins" via timeout.
19. Redis client (`ioredis`, pub/sub-only) constructed + decorated (lines 352-353).
20. `@fastify/multipart` registered (10MB limit, line 355-358).
21. `@fastify/rate-limit` registered — global 100/min keyed on **real client IP** (`clientIp(request)`, never raw `request.ip` which is the Fly edge socket) — errors route through the one `ApiError`/envelope path (lines 360-376).
22. Custom `application/json` content-type parser tolerating empty body (lines 379-393).
23. `authPlugin`, `securityHeadersPlugin`, `healthRoutes` registered (lines 395-397).
24. Auth-prefix gate `onRequest` hook: `/api/owner/`, `/api/courier/`, `/api/customer/` require a `Bearer` header (401 if missing) EXCEPT explicit `NO_AUTH_PATHS` and the pre-auth OTP send/verify regex; `/dev`,`/api/dev` paths gated by `isDevRequestAuthorized` (404 if not authorized — fails closed, never reveals existence) (lines 399-427).
25. `registerReplySendError` (the `reply.sendError` decorator) + the global `setErrorHandler` (the ONE error envelope, ADR-0010) registered (lines 429-517) — see §7.
26. **`registerCoreRoutes(fastify, {...})`** — the ~50-plugin core app route registration in original load-bearing order (awaited; dynamically imports `routes/auth/local.js`) (line 525; full list in `bootstrap/routes.ts`).
27. Order-sensitive tail registrations AFTER core routes: Telegram webhook, Plisio crypto-payments webhook (dark unless `PAYMENTS_CRYPTO_ENABLED`), mock-auth routes, `/internal` acquisition routes (gated by `PROVISION_OPS_SECRET`), inline `/api/dev/*` handlers (mock-auth/create-assignment/seed-data), SPA-proxy catch-all, owner product-media routes, owner refunds routes (dark unless `PAYMENTS_PREPAID_ENABLED`), the **admin plane** (structural `registerAdminPlaneGate` BEFORE the admin routes themselves, closing BOLA) (lines 527-837).
28. SPA fallback `setNotFoundHandler` — serves `index.html` for HTML GETs matching known SPA route prefixes, else the one 404 envelope (lines 839-852).
29. `fastify.ready()` callback: print routes, **`setupWebSocket(fastify, messageBus)`** — WS server attached only once Fastify is fully ready (lines 854-858).
30. `onClose` hook registered: closes WS clients (code 1012 "Server restarting"), stops telegram poller, stops all heartbeats, `queue.stop()` (lines 860-871).
31. **`setupShutdown(fastify, pool, messageBus, queue)`** — installs SIGTERM/SIGINT/SIGHUP handlers (line 873; see §5).
32. `fastify.listen({port, host:'0.0.0.0'})` (line 877) — **this is the point Fly's `/livez` check needs to succeed by**.
33. Post-listen fail-fast assertions: `assertAccessRequestSchedules(pool)` and `assertDeliveryTraceSchedule(pool)` — a missing cron schedule after listen is a **visible deploy failure** (`process.exit(1)` in the catch, lines 879-887), not a silent zombie.

**Env-gated boot branches**: dev routes (`ALLOW_DEV_LOGIN`+`DEV_AUTH_SECRET`), Google OAuth routes (`GOOGLE_OAUTH_ENABLED`), access-request route (`ACCESS_GATE_PUBLIC_ENABLED`), crypto webhook/refunds (`PAYMENTS_CRYPTO_ENABLED`/`PAYMENTS_PREPAID_ENABLED`), media-rich menu fields (`MEDIA_RICH_ENABLED`), metrics endpoint (`METRICS_TOKEN` presence), acquisition/provisioning routes (`PROVISION_OPS_SECRET` presence), Sentry (`SENTRY_DSN` presence), web-push channel (`VAPID_PUBLIC_KEY`+`VAPID_PRIVATE_KEY` presence — though schema already requires them, so this is always true at runtime), R2 vs local storage (`R2_BUCKET`+`R2_ENDPOINT` presence). Workers themselves are **not** individually flag-gated at boot (all always start) except the overall boot-budget race.

**Rust mapping**: `crates/api/src/boot/` orchestrates this as an explicit ordered async fn — `env::load()` → `pools::init()` (3 pools) → `schema_guard::assert_current()` → message-bus/queue init → metrics → storage/notification provider selection → background worker spawn (with an analogous bounded `tokio::time::timeout` race so a hung worker never blocks the listener) → axum router assembly (middleware layers mirroring the onRequest hooks, in the same order — CORS, security headers, subdomain rewrite, correlation-id, auth-prefix gate) → route registration → `axum::serve` → post-listen cron-schedule assertions.

---

## 5. Graceful shutdown

**API** (`apps/api/src/shutdown.ts`, full 89-line file, `setupShutdown`):
Signal handlers: `SIGTERM`, `SIGINT`, `SIGHUP` (lines 86-88), all funnel into one idempotent `shutdown(signal)` guarded by a `shuttingDown` flag (line 29, 32).

Drain order (numbered in-code comments):
1. Forward `SIGTERM` to any tracked child processes (e.g. `pg_dump`) (lines 36-41).
2. **Stop accepting new HTTP requests** — `await fastify.close()` (lines 43-50).
3. **Drain queue** with a **10-second timeout** race (`Promise.race([queue.stop(), setTimeout(10000)])`, lines 52-64) — allows active jobs to finish but won't hang shutdown forever.
4. **Close MessageBus** — `await messageBus.close()` (lines 66-68) — no timeout guard here (gap vs. the queue's 10s race).
5. Cleanup temp backup files (`.tmp/backups`) (line 70-71, `cleanupTempFiles()`).
6. **Close db pool** — `await pool.end()` (lines 73-77).
7. `process.exit(0)` (line 83).

Note: **WebSocket closing is NOT in `shutdown.ts`** — it's in the separate `onClose` hook registered in `server.ts:860-871` (closes each `fastify.wss.clients` with code 1012 "Server restarting", stops telegram poller, stops heartbeats, `queue.stop()` again). Since `fastify.close()` (step 2 above) triggers Fastify's own `onClose` hooks, the *actual* observed order is: HTTP stop-accepting → Fastify's `onClose` fires (closes WS, stops poller/heartbeats, stops queue) as part of `fastify.close()` resolving → **then** `shutdown()`'s own explicit queue-drain-with-timeout runs again (redundant second `queue.stop()` call) → messageBus close → temp cleanup → pool close → exit. This double-stop of the queue is a minor redundancy worth collapsing in the Rust port.

`fly.toml`: `kill_signal = "SIGTERM"`, `kill_timeout = "30s"` (lines 43-44) — Fly gives the process 30s total before SIGKILL, comfortably above the queue's internal 10s race.

**Worker** (`apps/worker/src/shutdown.ts`, full 29-line file, `setupShutdown(queue, pool, heartbeat)`):
Simpler — no HTTP server, no MessageBus-close, no child-process forwarding. Order: 1) `heartbeat.stop()` → 2) `queue.stop()` (no timeout race here, unlike the API's) → 3) `pool.end()` → 4) `process.exit(0)`. Handlers: `SIGTERM`, `SIGINT` only (no `SIGHUP`, asymmetric vs. the API).

**Rust mapping**: `crates/api/src/boot/shutdown.rs` — a `tokio::signal` listener for SIGTERM/SIGINT (SIGHUP is unusual for a service; consider dropping or keeping for parity), driving axum's graceful-shutdown future, then sequentially: close WS connections (broadcast a close frame to all tracked sockets) → `queue.stop()` under a `tokio::time::timeout(10s)` → message-bus listener task abort/close → pool `.close()` (sqlx). Collapse the double-queue-stop the JS version has.

---

## 6. Feature-flag registry census

**No single dedicated "flags.ts" file exists** — the registry IS `packages/config/src/index.ts`'s `EnvSchema` for anything schema-validated, plus a handful of raw `process.env` reads for flags added after the schema was last touched (see §1 shadow list). Two additional per-flag helper modules centralize *derived* boolean logic beyond a bare `env.X === 'true'`:
- `apps/api/src/lib/voice-flag.ts` — `isVoiceEnabled()` = `VOICE_CONTROL_ENABLED==='true' && VOICE_KILL!=='true'` (AND-of-two-raw-vars, not in schema).
- `apps/api/src/lib/payments/registry.ts` — `isPrepaidEnabled()`/`isCryptoEnabled()` (raw env reads, not in schema, comment explicitly says "no config-schema churn").
- `apps/api/src/plugins/dev-guard.ts` — `devLoginAllowed(env)` = `ALLOW_DEV_LOGIN==='true' && !!DEV_AUTH_SECRET` (schema-backed).

Extraction — every `*_ENABLED`-shaped identifier across `apps/`+`packages/` (server flags + FE `VITE_*` mirrors):
```
grep -rhoE "[A-Z_]*_ENABLED\b" --include="*.ts" --include="*.tsx" apps packages | grep -v node_modules | sort -u | wc -l
→ 35 (includes both bare server flags and their VITE_ FE-mirror counterparts, and a few non-flag false-positive matches like the literal token `_ENABLED`)
```

| flag | default | where defined | what it gates | runtime-changeable? |
|---|---|---|---|---|
| `OTP_ENABLED` | `false` | `EnvSchema` (config/index.ts:48) | global phone-OTP; per-location `require_phone_otp` needs this AND | redeploy only (env var) |
| `MEDIA_RICH_ENABLED` | `false` | `EnvSchema` :53 | ADR-0002 product-media schema/render (AND `locations.plan='business'`) | redeploy only |
| `FUNNEL_INGEST_ENABLED` | `true` | `EnvSchema` :57 | funnel-ingest route (always mounted; 204 no-op when off) | redeploy only (can be flipped as a Fly secret w/o rebuild since it's a plain env var, not baked at build time) |
| `ENFORCE_VENUE_HOURS` | `false` | `EnvSchema` :64 | closed-venue order rejection (409) | redeploy only |
| `GOOGLE_OAUTH_ENABLED` | `false` | `EnvSchema` :19 | `/api/auth/google` route + FE button (`VITE_GOOGLE_OAUTH_ENABLED` baked at build) | backend: env-var (no rebuild); FE mirror: **build-time only** |
| `ALLOW_DEV_LOGIN` | `false` | `EnvSchema` :29 | ALL dev/test auth bypass surfaces; boot-guard D FATALs if true+NODE_ENV=production | env-var, but prod boot-guard makes it unflippable in prod |
| `BACKUP_ENABLED` | `false` | `EnvSchema` :89 | backup cron workers | env-var |
| `DWELL_TIER3_ENABLED` | `false` | `EnvSchema` :103 | dwell-monitor tier-3 escalation | env-var |
| `RESTORE_VERIFY_FULL_HASH` | `false` | `EnvSchema` :135 | full-hash mode on backup restore-verify | env-var |
| `DISPATCH_OWNER_GRACE_ENABLED` | `false` 🔴 | `EnvSchema` :161 | dispatch-exhaustion auto-cancel-to-customer; **explicitly ships flag-off pending operator ethics ratification** | env-var, but gated on human ratification before flip |
| `ACCESS_GATE_PUBLIC_ENABLED` | `false` | `EnvSchema` :188 | POST `/api/access-requests` route registration (structural — 404 while off, not a runtime branch) + FE CTA (`VITE_ACCESS_GATE_PUBLIC_ENABLED`, build-time) | backend: **redeploy required** (route registration happens once at boot, not per-request) |
| `ACCESS_GATE_INVITE_GATING_SHIPPED` | `false` | `EnvSchema` :191 | CI banned-strings test permission (not really a runtime gate) | CI-only |
| `PAYMENTS_CRYPTO_ENABLED` | `false` (implicit — no schema entry, raw `process.env` read) | `lib/payments/registry.ts:7` | Plisio webhook route (404 when off) + `orders.ts` crypto-fork branch; FE mirror `VITE_PAYMENTS_CRYPTO_ENABLED` | env-var (backend, per-request check, no redeploy needed to flip) |
| `PAYMENTS_PREPAID_ENABLED` | `false` (implicit) | `lib/payments/registry.ts:6` | owner refunds routes dark/404 + `orders.ts` crypto-fork AND-condition | env-var, per-request check |
| `VOICE_CONTROL_ENABLED` | `false` (implicit — `undefined !== 'true'`) | `lib/voice-flag.ts:12` | `/api/public/voice-config` reports `enabled`; FE polls this (no-store) before activating voice; also widens CSP `connect-src` | env-var, per-request check — **the whole point is it's a live poll, not baked** |
| `VOICE_KILL` | not set | `lib/voice-flag.ts:12` | emergency hot-kill — `true` disables voice for all clients on next poll without rebuild, even if `VOICE_CONTROL_ENABLED=true` | env-var, live (no-store fetch + no-store response header) — the clearest "runtime-changeable" flag in the whole system |
| `METRICS_TOKEN` | unset (dark) | raw `process.env`, `lib/metrics.ts:135` | `/metrics` scrape endpoint — 404-equivalent unless token set/matched | env-var |
| `PROVISION_OPS_SECRET` | unset (dark) | raw `process.env`, `server.ts:545` | `/internal` acquisition/provisioning routes | env-var |

**Build-time-only (`VITE_*`, baked into the FE bundle by `vite build`, cannot change without a rebuild)** — from `Dockerfile` `ARG`/`ENV` pairs (lines 17-36): `VITE_ACCESS_GATE_PUBLIC_ENABLED`, `VITE_TG_CATEGORY_GATING`, `VITE_MENU_CHARACTERISTICS_ENABLED`, `VITE_MENU_CHARACTERISTICS_COMPARISON`, `VITE_MENU_CHARACTERISTICS_FILTER`, `VITE_MENU_ALLERGEN_FILTER` — all default `false`, all passed as `--build-arg` on staging to verify before a prod flip (per Dockerfile comments, this is deliberate ship-discipline: prod stays dark by default, staging is where flags get proven). Others found via grep but not in the Dockerfile ARG list (so presumably set via a `.env` at build time or a separate build pipeline step): `VITE_GOOGLE_OAUTH_ENABLED`, `VITE_KEYBOARD_SHORTCUTS_ENABLED`, `VITE_MEDIA_RICH_ENABLED`, `VITE_PULL_TO_REFRESH_ENABLED`, `VITE_UNDO_REDO_ENABLED`, `VITE_VOICE_ENABLED`, `VITE_PAYMENTS_CRYPTO_ENABLED`.

**Rust mapping**: `crates/api/src/flags.rs` — a single `Flags` struct populated once from the typed `Env` at boot (mirrors `EnvSchema`), each field a plain `bool`/`enum`, with a `utoipa`-documented `GET /api/config` (or per-feature `GET /api/public/{feature}-config` like the existing `voice-config`) endpoint for the ones that need **live** (no-redeploy) checking — `VOICE_CONTROL_ENABLED`/`VOICE_KILL` is the one flag that structurally *requires* this pattern (poll-based kill-switch); most others (`OTP_ENABLED`, `MEDIA_RICH_ENABLED`, etc.) are read once at boot and could be either baked into `Flags` or exposed the same way for consistency. Recommend consolidating the "shadow" flags (`PAYMENTS_*`, `VOICE_*`) into the same typed struct as the schema-backed ones — the JS split (schema vs. raw `process.env`) is exactly the kind of drift the Rust port should eliminate.

---

## 7. Error envelope + logging

**Error envelope** (ADR-0010) — `apps/api/src/lib/api-error.ts` (full 92-line file) + the `setErrorHandler` in `server.ts:443-517`.

Shape (`buildErrorEnvelope`, api-error.ts:56-72):
```ts
{ code: string /* SCREAMING_SNAKE, stable, FE-branchable */,
  message: string,
  fields?: { path: string; code: string }[],  // 422/400 — PATHS only, never submitted values (B4 no-PII)
  correlationId: string,                       // server-generated request id, always echoed
  retryAfterMs?: number,                        // 429
  status: number,                               // numeric HTTP status (moved out of `code` in A1)
  error: string }                               // legacy string field, kept for un-migrated FE (code-preserving rollout)
```
`ApiError extends Error` class (lines 17-38) carries `status`, `code`, optional `fields`/`retryAfterMs`, and a `statusCode` getter mirroring `status` (needed because `@fastify/rate-limit` and Fastify's own default handler read `.statusCode`, not `.status`). `isContractCode()` (line 41-43) validates a code is `^[A-Z][A-Z0-9_]*$` before ever surfacing a driver/library code (e.g. blocks a raw Postgres `23505` from leaking — falls back to generic `INTERNAL`/`ERROR`).

`setErrorHandler` logic (server.ts:443-517):
- Always echoes `x-correlation-id` header from `request.id` (server-authoritative, never trusts inbound).
- Validation errors (AJV array `error.validation` OR Zod `.issues`/`FST_ERR_VALIDATION`) → **400 `VALIDATION_FAILED`**, generic message, `fields` = path+code only (no raw Zod/AJV dump, no submitted values).
- Otherwise: `status = apiErr?.status ?? error.statusCode ?? 500`; `code` = `apiErr.code` if `ApiError`, else the error's own code IF it's contract-shaped (`isContractCode`), else `INTERNAL` (5xx) / `ERROR` (4xx).
- **5xx → generic "Internal server error" message always** (never leak stack/internals); logs `request.log.error` + tags/captures to Sentry (`correlationId`, `error_code` tags — Sentry tag allowlist enforced separately in `sentry.ts`).
- 429s get a `retry-after` header from `retryAfterMs`.
- `@fastify/rate-limit` special-cased via `rateLimitError()` (api-error.ts:83-91) because the plugin *throws* its `errorResponseBuilder` return value directly (bypassing normal reply flow) — must be a throwable `ApiError` or it silently becomes a 500.
- 404 (`setNotFoundHandler`, server.ts:841-852) emits the same envelope via `reply.sendError(404, 'NOT_FOUND', 'Not found')` — the *unmatched path* itself is deliberately **not** serialized in the body (only in server-side logs/correlationId trace) to avoid leaking route structure.

**Logging** — `apps/api/src/lib/logger.ts` (full 143-line file), pino-based:
- `createPinoLogger(name)` — level from `LOG_LEVEL` (default `info`), `mixin()` injects the AsyncLocalStorage-tracked `correlationId` into every log line, custom serializers for `err`/`req`/`res`.
- **Redaction, two layers**: (a) pino's built-in `redact.paths` for known header names (`cookie`, `authorization`, `set-cookie`, `x-api-key`) with `censor: '[REDACTED]'`; (b) a custom `deepRedact()` walking arbitrary log objects, blanking any key in a `SENSITIVE_KEYS` set (email, phone, tokens, addresses, PII fields — lines 42-49) AND running string values through a `PiiRedactor` (separate module) for pattern-based PII scrubbing even in unstructured strings.
- **URL secret redaction** (`redactUrlSecrets`, lines 24-40): strips query-string values for a `SENSITIVE_QUERY_PARAMS` set (`token`, `access_token`, `jwt`, `secret`, etc.) — added specifically because the realtime WS authenticates via `?token=<JWT>` and Pino's default `req` serializer would otherwise log the full JWT on every WS upgrade (documented incident-driven fix, `docs/design/ws-token-in-url/`).
- `getFastifyLoggerConfig()` (lines 107-129) is what's actually wired into the Fastify constructor (`server.ts:83`) — a lighter config than `createPinoLogger` (no `mixin`, simpler serializers) but the same header-redact paths.

**Sentry** (`apps/api/src/lib/sentry.ts`, full 113-line file): only initialized if `SENTRY_DSN` set. `beforeSend`/`beforeBreadcrumb` hooks redact exception messages via `PiiRedactor`, blank cookies/headers, reduce `event.user` to `{id}` only (drops email/name/etc.), and **allowlist** tags to exactly `{role, location_id, order_id, worker, db, error_code, correlationId}` — everything else stripped before leaving the process.

**Rust mapping**: `tracing` + `tracing-subscriber` with a custom `Layer`/`Visit` implementation mirroring `deepRedact` (blank a `SENSITIVE_KEYS` set of field names) and a `redact_url_secrets`-equivalent applied to any logged request URI. Error mapping: an `ApiError` type implementing axum's `IntoResponse`, carrying `{status, code, message, fields, retry_after_ms}`, with a single `fallback_handler`/`from_fn` middleware building the identical JSON envelope (including the same 5xx-generic-message rule and `isContractCode` regex gate against leaking DB error codes) — this is a straightforward, low-risk 1:1 port since the shape is already a stable documented contract (ADR-0010) that ~10 e2e tests assert against.

---

## 8. Rust boot mapping (proposed module tree)

```
crates/api/src/boot/
  env.rs          — typed Env struct (serde + validator), one load_env() call at process start.
                    Consolidates BOTH the current Zod-schema fields AND the 20 "shadow" process.env
                    reads found in §1 into ONE source of truth (closes the JS split).
                    Includes the assertDevAuthDisabledInProd-equivalent boot assertion.
  schema_guard.rs — 🔴 (red-line-adjacent per task brief / packages/db/migrations glob)
                    Compares a build-embedded expected migration version (via build.rs) against
                    sqlx's _sqlx_migrations table; std::process::exit(1) on FATAL mismatch,
                    warn-and-continue on non-definitive (connection) errors, silent no-op when
                    the build-time constant is absent (dev builds). COUNCIL REVIEW before altering
                    exact semantics — this is the exact mechanism a prior prod outage (schema drift)
                    depends on as its backstop.
  pools.rs        — 🔴 (RLS role + tenancy = council per task brief)
                    Three sqlx::PgPool builders (operational/session/migrations) replicating sizes,
                    timeouts, and the current_user != 'postgres' after-connect guard; a
                    with_tenant<T>() helper porting `SELECT set_config('app.user_id', $1, true)`
                    inside an explicit transaction. Flag for council: should the superuser-denylist
                    become a positive role-allowlist? Should the guard extend to the session/backup
                    pools (currently only operational has it)?
  shutdown.rs     — tokio::signal(SIGTERM/SIGINT[/SIGHUP]) → axum graceful shutdown → WS close →
                    queue.stop() under 10s timeout (collapse the JS double-stop) → message-bus
                    listener close → pool.close(). Mirrors apps/api/src/shutdown.ts 1:1 order.
crates/api/src/flags.rs
                  — Flags struct derived from Env at boot; utoipa-documented GET /api/config (or
                    per-feature endpoints matching the existing GET /api/public/voice-config
                    pattern) for the subset that must be live-pollable (VOICE_CONTROL_ENABLED/
                    VOICE_KILL is the only flag that structurally needs poll-based no-redeploy
                    semantics today — everything else is currently redeploy-only despite being a
                    plain env var, which is a design choice worth revisiting in the rewrite, not
                    a hard requirement to replicate).
```

**Items marked 🔴 requiring council/human gate before implementation** (not just documentation):
1. **Schema-head guard exact semantics** — it's the safety backstop for a real past prod outage (schema drift); don't let a "cleaner" Rust idiom (`sqlx::migrate!()`) silently drop the "fail-fast on missing head, warn-and-continue on ambiguous error" distinction.
2. **RLS role check + `withTenant` tenancy mechanism** — money/authz/RLS is an explicit red-line glob in this repo's Ship Discipline rule; any behavioral change (not just a language port) needs the Triadic Council.
3. **`DISPATCH_OWNER_GRACE_ENABLED`** — ships flag-off pending explicit operator ethics ratification; the Rust port must preserve the default-off + must NOT auto-flip it.
4. **The superuser-denylist gap on session/backup pools** — flagged as a possible hardening opportunity, not something to silently "fix" without discussion (could break local/dev workflows that connect as a broader role).

## Unresolved / needs follow-up
- `EnvSchema` field count (80) was derived by manual enumeration of the schema body (no single `grep -c` line matched cleanly due to multi-line/comment formatting) — worth a script-based exact count if this doc needs to be re-derived automatically later.
- Several numeric-shaped courier-dispatch env vars (`COURIER_ACCEPT_WINDOW_MS`, `COURIER_DISPATCH_MAX_ATTEMPTS`, etc.) are typed `z.string().optional()` rather than `z.coerce.number()` in the current schema — a pre-existing looseness, not something I should "fix" under a read-only inventory task, but worth flagging for the Rust `Env` struct to type correctly from the start.
- I did not find a DB-backed / admin-UI-editable flag mechanism anywhere (no `feature_flags` table, no LaunchDarkly-style client) — every flag is env-var or build-time `VITE_*`. Confirmed via `grep` across `apps/api/src/routes` for anything resembling a flags CRUD route; none found. If one exists it would be under a name I didn't guess — flagging as "not found" rather than "confirmed absent."


---

<!-- ============ §6 SYNTHESIS ============ -->

# §6 — Lane A synthesis: 🔴 register, orphans, gaps

## 6.1 🔴 Council-before-port register (roll-up)

| Surface | 🔴 count | Where |
|---|---|---|
| HTTP routes | **98** rows (32 owner + 39 courier/customer/core + 27 public/admin/infra) | §1 tables |
| WS protocol | **4** items (owner tri-state verdict+fan-out guard; courier tri-state verdict+fan-out guard; `?token=`→`Sec-WebSocket-Protocol` migration ADR-0013-addendum [PROPOSED, not built]; room-string wire contract) | §2 |
| Job queues | **12** queue names / 9 families (settlement, reconciliation, order-timeout, courier-dispatch+offer-sweep, GDPR erasure+retention, backup×4+verify×2, retention sweeps) | §3 |
| Integrations | **4** red-line-heavy areas (OTP, Plisio, GDPR/anonymizer, Backup/DR) + scattered 🔴 rows in Telegram-webhook/web-push/R2/OCR (≈60 fine-grained rows, partially overlapping the route rows above) | §4 |
| Boot/lifecycle | **3** items (schema-head guard semantics; RLS role guardrail + `withTenant` tenancy; `DISPATCH_OWNER_GRACE_ENABLED` stays default-off) | §5 |
| **Total distinct council-gated items** | **≈121** (98 route rows + 12 queues + 4 WS + 4 integration areas + 3 boot; integration fine-grained rows largely overlap route rows) | |

Council grouping proposal (so this is ~8 councils, not 121): (C1) money/orders state-machine
(orders POST + status transitions + settlements + refunds + reconciliation + timeout sweeps),
(C2) auth/JWT/session/OTP (+dev-route parity), (C3) RLS/tenancy (`withTenant`, GUC naming, pool
guardrail), (C4) WS authz (ADR-0013 tri-state + token-channel migration), (C5) payments webhooks
(Plisio HMAC), (C6) GDPR/anonymizer (+SECURITY DEFINER functions stay in Postgres), (C7) backup/DR,
(C8) courier dispatch/offer-sweep.

## 6.2 Dead / orphaned surface found by the census (decide: port, fix, or drop — do NOT silently port)

| Finding | Evidence | Recommendation |
|---|---|---|
| `POST /couriers/invites` (`routes/couriers.ts`, mounted UNPREFIXED — no `/api`) | zero FE references; all traffic uses `/api/owner/locations/:locationId/courier-invites` | drop in rebuild (operator confirm) |
| `menu-translate.ts` + `menu-confirm.ts` owner routes | fully built, liability-relevant (allergen confirm), **no FE caller** in `apps/web/src` | escalate to operator: wire or drop |
| `courier:<id>:shift` WS channel | published (`shiftService.ts:60`), zero FE consumers | drop topic in rebuild |
| `StatusWSClient` (`apps/api/src/client/status/ws.ts`) | zero import sites | drop |
| WS message-auth (`auth` inbound type) | FE always sends `?token=` on URL; message-auth path is dead code today | resolve via C4 (token-channel migration decides) |
| `dwell.escalate` queue | undefined constant + never-instantiated worker, masked by `@ts-nocheck` | drop |
| `order.pending_aging`, `settlement.cron`-as-queue constants | never implemented | drop constants |
| Turnstile plugin | fully built incl. siteverify, **never registered** — zero routes protected | decide: register in rebuild (bot-defense gap) or drop |
| Telegram poll-mode worker | code present, never started (webhook mode is live) | drop poll mode |
| `adapters/push.ts` PushAdapter scaffold | superseded by `webpush.ts` | drop |
| mem0ai | 1 write-only call site, no read path, non-persistent store | drop in rebuild |
| Duplicate dev routes (`dev/mock-auth.ts` vs `server.ts` inline mock-auth/create-assignment) | two independently-maintained implementations | collapse to one in Rust |
| Refunds mount-path anomaly | `/api/owner/:locationId/refunds` (missing `locations/` segment vs all siblings) | decide before utoipa schema is written |
| Dual tenant GUC | `app.user_id` (primary) vs legacy `app.current_tenant` in a few files | resolve in C3 before any sqlx port |

## 6.3 Behavior bugs found during census (pre-existing; rebuild must NOT copy them, fix via normal loop first or council)

- **Telegram webhook fail-open**: missing `X-Telegram-Bot-Api-Secret-Token` header is processed, not
  rejected — contradicts its own e2e test (§4 Telegram). 🔴
- **Plisio HMAC byte-ordering** self-flagged as never validated against a real callback; zero tests. 🔴
- **Superuser-denylist pool guard** runs only on the operational pool, not session/backup pools (§5). 🔴
- **24/30 queues** run on bare pg-boss defaults (2 retries, 0 backoff, no DLQ) — only 6 got the
  2026-07-03 hardening; the Rust baseline must be the hardened profile for all queues (§3).
- **`_truncated` NOTIFY-overflow frames** have no explicit FE recovery handler — recovery piggybacks
  on unrelated refetch timers; untested (§2).
- **GDPR has no Art.15 export endpoint** (erase-only) — compliance gap to schedule, not a port item (§4).
- **Backup/DR**: verification automated, restore-to-production is a manual runbook only (§4).

## 6.4 Honesty section — what this census could NOT fully enumerate

1. **Per-order customer JWT issuance** (how a guest gets a token scoped to `user.orderId`) sits outside
   the WS lane's file scope — it IS covered as a route row (§1 customer/track + orders), but the
   token-mint↔WS-authz linkage needs one focused pass in council C4.
2. **Exact `EnvSchema` field count (80)** was manual enumeration (multi-line Zod formatting defeats a
   single grep) — re-derive with a small script when the Rust `Env` struct is written.
3. **apps/api's 8 heartbeat timers**: whether each has an explicit stop-on-shutdown call was outside
   the jobs lane's scope — verify during the shutdown port (§5 drain order is otherwise fully mapped).
4. **crt.sh / RSSHub**: project memory says code-integrated; exhaustive grep finds them only in docs +
   an ad-hoc tool-call log, never in tracked source. Recorded as a discrepancy, not a hard negative.
5. **Fastify implicit surface**: HEAD routes auto-added by Fastify for each GET, and `OPTIONS` handled
   by CORS plugin — not separate census rows by design; the Rust port gets these from tower-http CORS
   + axum method routing (documented so the 236 count is understood as *registrations*, not verbs).


---

<!-- ============ §7 TECHNOLOGY DECISIONS (researched 2026-07-04) ============ -->

# Rust Rebuild — Technology Decisions (Job Queue / Imaging / OCR / PDF)

Research date: 2026-07-04. Docs-only research memo — no code changed. Scope: dowiz/DeliveryOS backend
rebuild on Rust/axum/tokio/sqlx, Fly.io, Postgres via Supabase behind the Supavisor pooler (transaction-mode),
schema unchanged, pg-boss being replaced. Four independent parallel research passes, each with live
crates.io/GitHub freshness checks as of 2026-07-04 (not relying on pre-2025 training knowledge).

---

## Executive summary — four verdicts

| # | Decision | Verdict | Confidence |
|---|---|---|---|
| 1 | Postgres-backed Rust job queue | **Hand-roll** (`FOR UPDATE SKIP LOCKED` + optional `PgListener` on a direct connection) — fallback: `graphile_worker_rs` | Medium |
| 2 | Imaging pipeline (sharp replacement) | **libvips via Rust FFI bindings** (`olxgroup-oss/libvips-rust-bindings`) — fallback: pure-Rust `image`+`fast_image_resize`+`ravif`+`webp` | Medium |
| 3 | OCR for menu import | **Separate self-hosted Tesseract OCR sidecar/microservice** (Fly multi-container or separate app over 6PN) — not FFI, not CLI-in-main-image | High |
| 4 | PDF text/image extraction | **`pdfium-render`** on a glibc (Debian-slim) base image — fallback: pure-Rust `pdf-extract`+`lopdf` | Medium-high |

---

## Decision 1: Postgres-backed Rust job queue

Research date: 2026-07-04. All freshness claims are sourced from `crates.io` API (`/api/v1/crates/<name>`) and GitHub REST API (`pushed_at`) fetched live today, not from training memory.

### Freshness snapshot (checked 2026-07-04)

| Crate/repo | crates.io max version | crates.io last publish | GitHub last push | Stars | Open issues |
|---|---|---|---|---|---|
| `apalis` (core) | 0.7.4 | 2026-05-06 | 2026-06-30 | 1,294 | 1 |
| `apalis-postgres` | 1.0.0-rc.8 (no stable yet) | 2026-05-08 | 2026-06-29 | 4 (new dedicated repo) | 11 |
| `apalis-sql` | 0.7.4 | 2026-05-06 | (same monorepo as apalis) | — | — |
| `underway` | 0.2.0 | 2025-07-16 (**~1yr stale**) | 2026-05-21 | 170 | 16 |
| `graphile_worker` (graphile_worker_rs) | 0.13.3 | 2026-06-27 | 2026-06-27 | 79 | 2 |
| `pgboss` (pgboss-rs) | 0.1.0-rc5 (main has rc6 unreleased) | 2025-12-17 | 2026-06-18 | 12 | 2 |
| `pgmq` (extension crate, pgrx) | 0.33.7 | 2026-07-01 | 2026-07-02 | 4,998 | 37 |
| `sqlxmq` | 0.6.0 | 2025-05-25 (**~14mo stale**) | 2025-05-25 | 163 | 8 |
| `effectum` | 0.7.0 | 2024-07-23 (**~2yr stale, SQLite only**) | 2024-07-23 | 47 | 2 |
| `sequin` | n/a (Postgres CDC platform, not a job queue) | — | active | — | — |

### Candidates scoring matrix

Legend: **yes** = supported and verified in source/docs · **partial** = supported with a caveat · **no** = not supported / not found.

| Candidate | (a) Transactional enqueue | (b) Retry+backoff | (c) DLQ / failed retention+inspect | (d) Cron/recurring | (e) Singleton/dedup key | (f) Supavisor (txn-mode pooler) fit | (g) tokio+sqlx native, maintained 2026 |
|---|---|---|---|---|---|---|---|
| **apalis-postgres** (+apalis-sql) | **partial** — high-level `PostgresStorage::push()`/`Sink` always owns/clones its own `PgPool` and calls `pool.begin()` internally; but the underlying `push_tasks(conn: E, ...)` fn is generic over `E: Executor<'a, Database=Postgres>` (verified in `src/sink.rs`), so it *can* run inside a caller's `&mut Transaction` if you bypass the convenience API | **partial** — `attempts`/`max_attempts` tracked, failed jobs re-fetched (`status='Failed' AND attempts<max_attempts`); backoff/delay is applied via apalis-core's tower `RetryLayer`, not the storage crate itself — no dedicated per-queue backoff config found in `apalis-postgres` | **partial** — failed jobs stay in the same `apalis.jobs` table (`status='Failed'`), inspectable via `list_jobs`/`stats`/`overview` SQL fns and the optional `apalis-board` web UI; not a separate dead-letter table | **yes** — separate `apalis-cron` crate pipes cron schedules into any apalis backend incl. postgres | **yes** — verified: `TaskBuilder::with_idempotency_key()` + `CREATE UNIQUE INDEX idx_jobs_idempotency_key ON apalis.jobs(job_type, idempotency_key)` (migration `20260508093314_idempotency_key.sql`) — real DB-level dedup | **partial** — ships both `PostgresStorage` (pure `FOR UPDATE SKIP LOCKED` polling, no session state) and `PostgresStorageWithListener` (uses `NOTIFY`); use the polling variant through Supavisor, reserve NOTIFY for a small direct/session-mode pool | **yes** — sqlx-based, "runtime agnostic (tokio, async-std...)" per README; whole ecosystem pushed within days of this research (2026-06-29/30) but is pre-1.0 (`rc.8`/`rc.9`) |
| **underway** | **yes (best-documented)** — docs.rs states explicitly: "Enqueue tasks within your transactions and use the worker's transaction within your tasks"; task `execute()` handler literally receives a `Transaction<'_, Postgres>` | **yes** — explicit `RetryPolicy` type with `backoff_coefficient`, `initial_interval_ms`, `max_interval_ms`, configurable per task type (verified in `src/task.rs`, `src/worker.rs`) | **yes** — first-class `.dead_letter_queue("name")` builder method creates a real secondary DLQ (verified in `src/queue.rs` doc-comments + API) | **yes** — cron-like scheduling, e.g. `@daily[America/Los_Angeles]` via a `Scheduler` | **partial** — "concurrency key" (`Task::concurrency_key`) serializes execution of tasks sharing a key but does **not** prevent duplicate *enqueue* the way pg-boss `singletonKey`/`singletonFor` does — it's a mutex, not a dedup-on-insert | **partial** — worker loop uses `PgListener`/`LISTEN` for low-latency wake (`connect_listener_with_retry`, `listener.recv()` in `src/worker.rs`) which **breaks under Supavisor transaction mode**; BUT there is an explicit "pending task polling fallback" (`polling_interval.tick()`, default configurable, e.g. 1 min) so it degrades to poll-only rather than failing outright — still needs a small direct/session-mode connection for the listener to get low latency at all | **partial** — tokio+sqlx native, but crates.io is **~1 year stale** (0.2.0, Jul 2025) while `main` has extensive unreleased **breaking** changes (CHANGELOG "Unreleased" section is mostly `Breaking:` entries) — using it today means pinning a git rev of a churning pre-1.0 API, not a published release |
| **pgmq** (Tembo-originated Postgres extension + Rust clients `pgmq`/`pgmq-rs`) | **yes** — `PGMQueueExt` API added transaction support in v0.29.0 (confirmed via GitHub issue #257, merged PR #273, Aug 2024); `PGMQueue` (the simpler API) still owns its own pool | **no** — no built-in exponential backoff; only a raw `vt` (visibility timeout) + `read_ct` counter. You must compute your own backoff and call `set_vt`/re-send yourself | **yes** — `archive()` moves a message from the live queue table to a per-queue archive table for long-term retention/inspection (genuine SQS-style DLQ pattern) | **no (native)** / **partial (composable)** — no built-in scheduler; the standard pattern is pairing with the separately-installed `pg_cron` extension to call `pgmq.send()`/`pgmq.read()` on a schedule | **no** — no unique/singleton constraint concept; would need a bespoke unique index on your own payload key | **yes (best fit)** — pgmq's core API (`send`/`read`/`archive`/`pop`) is pure SQL function calls with **no LISTEN/NOTIFY, no session state, no advisory locks** — works cleanly through a transaction-mode pooler by design | **yes** — extremely active (pushed yesterday, 2026-07-02; 4,998 stars); **Tembo pivoted away from managed Postgres/Trunk in May 2025** to "coding agent orchestration," but pgmq itself moved out of the `tembo-io` org to its own `pgmq/pgmq` org and remains maintained by its original author (Adam Hendel/ChuckHend) — de facto community/Supabase-aligned now, not abandoned. It's a pgrx Postgres **extension** (Rust-implemented) plus a thin Rust client — not a "job framework" |
| **sqlxmq** | **no evidence found** — examples only show `.spawn(&pool)`; no documented `Executor`-generic enqueue path | **yes** — "Automatic retries with exponential backoff. Number of retries and initial backoff parameters are configurable" | **no** — no dead-letter concept documented | **no** — only "run at a future time," no recurring/cron | **partial** — "opt-in strictly ordered job delivery" per channel, not a true dedup/singleton key | **no** — relies on "NOTIFY-based polling," i.e. LISTEN/NOTIFY as its core wake mechanism | **no** — **stale**: last GitHub push and last crates.io release both 2025-05-25, over a year old as of this research; effectively unmaintained (single maintainer, no activity in 14 months) |
| **effectum** | n/a | n/a | n/a | n/a | n/a | n/a | **no** — SQLite-only ("Job Queue based on SQLite so it doesn't depend on any other services"), **not a Postgres candidate at all**; also stale (last push 2024-07-23, 2 years old) — excluded from serious consideration |
| **graphile_worker_rs** (Rust port of Graphile Worker) | **yes** — `add_job(mut executor: impl DbExecutorArg, ...)` is generic over a pluggable executor trait; the crate ships an sqlx driver backend (`graphile-worker-database/src/sqlx/transaction.rs`) alongside a `tokio-postgres` backend, so it accepts an existing sqlx `Transaction` | **yes** — `attempts`/retry with `job_key_mode`; test suite (`tests/worker_utils_reschedule_jobs.rs`) and dedicated `graphile-worker-recovery` crate cover retry semantics, mirroring the mature Node.js Graphile Worker's backoff | **yes** — explicit "permafailed" terminal state with `worker_utils_permanently_fail_jobs.rs` and `delete_permafailed.rs` in its test suite — a genuine dead-job concept with inspection via `graphile-worker-admin-ui`/`graphile-worker-admin-api` | **yes** — dedicated `graphile-worker-crontab-parser`/`graphile-worker-crontab-runner`/`graphile-worker-crontab-types` crates, cron-syntax compatible with the Node original | **yes** — `job_key` + `job_key_mode` (`preserve_run_at` / `replace` / `unsafe_dedupe`) is a direct, well-tested port of Graphile Worker's dedup mechanism (own test files: `job_key.rs`, `job_key_mode.rs`, `deduplication.rs`) — closest functional match to pg-boss `singletonKey` found in this research | **partial** — uses LISTEN/NOTIFY for low-latency wake (`graphile-worker-runtime/src/notify/*`, `graphile-worker-database/src/tokio_postgres/listener.rs`); Graphile Worker's Node parent design always polls at a fixed interval regardless of NOTIFY (NOTIFY only shortens latency), and the Rust port's `graphile-worker-runtime` retains an interval/timeout module consistent with that — same "needs a direct/session-mode connection for low latency, degrades to poll otherwise" profile as underway, though I could not 100% confirm the fallback interval value in the time available | **yes** — actively maintained (crates.io `0.13.3` published 2026-06-27, i.e. days-fresh), sqlx+tokio native, no async-std; small community (79 stars, effectively one primary maintainer `leo91000`) but unusually serious engineering: multi-driver abstraction, OpenTelemetry spans, admin UI, extensive test coverage |
| **pgboss-rs** (`rustworthy/pgboss-rs`) | **no** — every public method (`send_job`, `fetch_job`, `complete_job`, etc.) calls `.fetch_one(&self.pool)`/`.fetch_optional(&self.pool)` directly on the `Client`'s own internal pool; no `Executor`/`Transaction` parameter found in `src/client/public/job_ops.rs` | **yes** — `Job::builder().retry_limit(n).retry_delay(Duration)` mirrors pg-boss's own retry/backoff fields directly | **yes** — first-class `.dead_letter("queue_name")` on `Queue::builder()`, directly ported from pg-boss's DLQ concept | **no** — no `schedule()`/cron API found (`cron_on` only appears as a read-only flag reflecting the pg-boss *schema's* internal app metadata, not a feature this crate implements) | **yes (exact pg-boss parity)** — `Job::builder().singleton_key("buzz").singleton_for(Duration::from_secs(7))`, i.e. the literal pg-boss `singletonKey`/`singletonSeconds` API surface, explicitly "inspired by, compatible with and partially ported from pg-boss" | **likely yes** — no LISTEN/NOTIFY or scheduler file found in the source tree; appears to be pure `SKIP LOCKED`-style polling via sqlx, which should be Supavisor-clean, though I did not find an explicit statement confirming this | **partial** — sqlx 0.8 + tokio native; GitHub pushed 2026-06-18 (fresh) but crates.io stuck at a pre-1.0 `0.1.0-rc5` since 2025-12-17, `main` already at unreleased `rc6`; **very small adoption** (12 stars, 2 open issues, essentially one maintainer) |
| **Hand-rolled**: `FOR UPDATE SKIP LOCKED` polling + `sqlx::postgres::PgListener` for wake-ups | **yes by construction** — you `INSERT` the job row using the same `&mut Transaction<'_, Postgres>` as the business write; this is the one candidate where (a) is trivially guaranteed, not a library feature to trust | **yes, DIY** — you own the `attempts`/`next_run_at = now() + backoff(attempts)` logic; well-documented pattern (Netdata, SupaExplorer, Neon guides all describe it), effort not correctness-risk | **yes, DIY** — a `status='dead'` terminal state + a query to list/replay dead rows is a few lines; no library bug surface | **yes, DIY** — cheapest via `pg_cron` scheduling a `SELECT enqueue(...)` call, or an in-process `tokio_cron_scheduler`/`croner` crate ticking a Rust cron parser | **yes, DIY** — a `UNIQUE (queue_name, singleton_key) WHERE status IN ('pending','active')` partial unique index gives you exact pg-boss `singletonKey` semantics with an `ON CONFLICT DO NOTHING` on enqueue | **yes (best fit)** — primary drain loop is pure `SELECT ... FOR UPDATE SKIP LOCKED` through the normal Supavisor-pooled connection (no session state); `PgListener` is added only as an optional latency optimization on a small **dedicated direct (port 5432) connection pool**, and the design explicitly tolerates the listener dropping/reconnecting because polling is the source of truth, not NOTIFY | **yes** — zero dependency-freshness risk since there is no third-party queue crate to go stale; pure `sqlx`+`tokio`, matches project's existing stack exactly |

### Citations

- [apalis (crates.io API)](https://crates.io/api/v1/crates/apalis) — max_version 0.7.4, updated 2026-05-06 — checked 2026-07-04, live API response
- [apalis-postgres (crates.io API)](https://crates.io/api/v1/crates/apalis-postgres) — pre-1.0 `1.0.0-rc.8`, updated 2026-05-08, created 2025-10-25 (young dedicated postgres repo) — checked 2026-07-04
- [github.com/apalis-dev/apalis](https://github.com/apalis-dev/apalis) — pushed 2026-06-30, 1,294 stars, 1 open issue (via `gh api repos/apalis-dev/apalis`) — checked 2026-07-04
- [github.com/apalis-dev/apalis-postgres](https://github.com/apalis-dev/apalis-postgres) — pushed 2026-06-29; source inspected directly: `src/sink.rs` (`push_tasks<E: Executor<'a, Database=Postgres>>`), `migrations/20260508093314_idempotency_key.sql` (unique dedup index), `examples/unique_jobs.rs`, `migrations/20250210092135_include_failed_in_get_jobs.sql` (retry-eligible failed jobs) — checked 2026-07-04
- [underway (docs.rs)](https://docs.rs/underway/latest/underway/) — explicit transactional-enqueue claim — checked 2026-07-04
- [github.com/maxcountryman/underway](https://github.com/maxcountryman/underway) — pushed 2026-05-21, 170 stars; source inspected: `src/queue.rs` (`.dead_letter_queue(...)`), `src/task.rs` (`RetryPolicy`, `backoff_coefficient`, "concurrency key" docs), `src/worker.rs` (`connect_listener_with_retry`, "Pending task polling fallback", `polling_interval.tick()`) — checked 2026-07-04
- [underway CHANGELOG.md](https://raw.githubusercontent.com/maxcountryman/underway/main/CHANGELOG.md) — shows extensive unreleased **Breaking** changes since 0.2.0 (2025-07-16) — checked 2026-07-04
- [github.com/pgmq/pgmq](https://github.com/pgmq/pgmq) — pushed 2026-07-02, 4,998 stars, 37 open issues — checked 2026-07-04
- [pgmq/pgmq issue #257 "Transaction Support (Rust)"](https://github.com/pgmq/pgmq/issues/257) — closed 2024-08-24; confirms `PGMQueueExt` transaction support shipped in v0.29.0 (PR #273) — checked 2026-07-04 via `gh api`
- [Supabase Docs — PGMQ Extension](https://supabase.com/docs/guides/queues/pgmq) and [Supabase Queues](https://supabase.com/docs/guides/queues) — confirms pgmq is the extension backing Supabase's own managed "Queues" product; dowiz is already on Supabase, so this may be enable-via-dashboard rather than self-hosted — checked 2026-07-04
- Tembo pivot reporting (WebSearch synthesis) — Tembo shut down its managed Postgres cloud and Trunk registry, pivoting to "coding agent orchestration" in May 2025; pgmq itself continued under its own `pgmq/pgmq` GitHub org, maintained by original author ChuckHend (Adam Hendel) — checked 2026-07-04
- [sqlxmq (github.com/Diggsey/sqlxmq)](https://github.com/Diggsey/sqlxmq) — pushed 2025-05-25 (latest release also 2025-05-25) — **~14 months stale relative to this research date** — checked 2026-07-04 via `gh api`
- [effectum (github.com/dimfeld/effectum)](https://github.com/dimfeld/effectum) — pushed 2024-07-23, SQLite-only — **~2 years stale**, disqualified on both freshness and Postgres-fit — checked 2026-07-04
- [leo91000/graphile_worker_rs](https://github.com/leo91000/graphile_worker_rs) — pushed 2026-06-27 (days-fresh), 79 stars, 2 open issues; source inspected: `crates/graphile-worker-queries/src/add_job/single.rs` (`add_job(mut executor: impl DbExecutorArg, ...)`), `crates/graphile-worker-database/src/lib.rs` (`DbExecutorArg`, `DbTransaction`, `TransactionDriver`, both sqlx and tokio-postgres backends), test files `job_key.rs`/`job_key_mode.rs`/`deduplication.rs`/`worker_utils_permanently_fail_jobs.rs`/`delete_permafailed.rs` — checked 2026-07-04
- [graphile_worker (crates.io API)](https://crates.io/api/v1/crates/graphile_worker) — max_version 0.13.3, updated 2026-06-27 — checked 2026-07-04
- [rustworthy/pgboss-rs](https://github.com/rustworthy/pgboss-rs) — pushed 2026-06-18, 12 stars, 2 open issues; README explicitly states "Inspired by, compatible with and partially ported from `pg-boss`" with `singleton_key`/`singleton_for`/`dead_letter`/`retry_limit`/`retry_delay` API shown verbatim; source inspected: `src/client/public/job_ops.rs` (all methods use `&self.pool` directly, no transaction parameter), `src/client/mod.rs` (`cron_on` is read-only schema metadata, not an implemented feature) — checked 2026-07-04
- [pgboss (crates.io API)](https://crates.io/api/v1/crates/pgboss) — max_version `0.1.0-rc5`, updated 2025-12-17; `Cargo.toml` on `main` already at `0.1.0-rc6` (unpublished) — checked 2026-07-04
- [Sequin (sequinstream/sequin)](https://github.com/sequinstream/sequin) — confirmed to be a Postgres change-data-capture → streams/queues/search-index platform (Kafka/SQS/Elasticsearch sinks), not a job-execution queue — **not a fit for this decision**, included for completeness only — checked 2026-07-04
- [Supabase Supavisor FAQ / troubleshooting](https://supabase.com/docs/guides/troubleshooting/supavisor-faq-YyP5tI) and [Disabling Prepared Statements](https://supabase.com/docs/guides/troubleshooting/disabling-prepared-statements-qL8lEL) — confirms: transaction mode (port 6543) has "no prepared statements, no SET commands, no LISTEN/NOTIFY, no temporary tables" across pooled transactions; session mode (port 5432, direct) is required for LISTEN/NOTIFY and holds a dedicated backend connection for the client's session — checked 2026-07-04
- ["I use Postgres SKIP LOCKED as a queue" — Hacker News](https://news.ycombinator.com/item?id=20020501) and [Netdata — Using FOR UPDATE SKIP LOCKED](https://www.netdata.cloud/academy/update-skip-locked/) — background on the hand-rolled pattern's production track record — checked 2026-07-04

### Verdict

**Primary: hand-roll it** — a small `sqlx`-native queue module using `SELECT ... FOR UPDATE SKIP LOCKED` for the drain loop (through the normal Supavisor-pooled connection), a `run_at`/`attempts` column pair for backoff, a `status='dead'` terminal state for the DLQ, a `UNIQUE (queue_name, singleton_key) WHERE status IN ('pending','active')` partial index for pg-boss-style singleton dedup, and `pg_cron` (already available on Supabase) or a small `tokio::time::interval` loop for recurring jobs. Add `sqlx::postgres::PgListener` only as an optional latency shortcut on a **small dedicated direct (port 5432) connection pool** — never load-bearing, since the SKIP LOCKED poll loop is the correctness source of truth.

**Fallback: `graphile_worker_rs`** (crate `graphile_worker`, `github.com/leo91000/graphile_worker_rs`) if the team wants a batteries-included framework instead of owning the code: it is the freshest actively-developed option found (crates.io publish 2026-06-27), has real `job_key`/`job_key_mode` dedup, cron, retry, and a permafailed/DLQ concept, is sqlx+tokio native, and — critically — is a faithful port of a Node.js library (Graphile Worker) with years of independent production hardening, which meaningfully de-risks "did we get the retry/backoff/singleton edge cases right" relative to a from-scratch implementation.

**Do not adopt as primary:** `underway` (crates.io is a year stale; `main` has heavy unreleased breaking churn), `sqlxmq`/`effectum` (both stale/off-target), `apalis-postgres`/`pgboss-rs` (both pre-1.0 with real gaps — apalis-postgres's transactional-enqueue and DLQ stories are "partial/low-level only," pgboss-rs has no transactional enqueue and no cron at all). `pgmq` is worth keeping on the radar as a complementary building block (see Rationale) but not as the primary framework.

### Rationale

No candidate scores "yes" across all seven requirements without a caveat — this is a real market gap, not a research miss (three independent search angles converged on the same ~8 candidates). Given that:

1. **The schema doesn't change and dowiz already knows pg-boss's semantics intimately** (it's the system being replaced) — replicating `singletonKey`/retry/backoff/cron/archive as a ~300-400 line, fully-tested Rust module is a bounded, well-understood scope, not exploratory work. Every pattern needed (SKIP LOCKED, partial-unique-index dedup, backoff-via-`run_at`) is independently well-documented and used in production by pg-boss itself, `river` (Go), and countless blog-verified implementations.
2. **Supavisor compatibility is the sharpest constraint**, and it favors hand-rolling directly: every library candidate that depends on LISTEN/NOTIFY for its primary wake mechanism (`underway`, `graphile_worker_rs`, `sqlxmq`) needs a workaround (direct connection for the listener) bolted on regardless — building the polling-first, NOTIFY-optional architecture from day one avoids retrofitting that workaround into a third-party crate's internals later.
3. **Freshness/maturity risk is asymmetric**: the two best feature-matches for pg-boss parity (`pgboss-rs` for singleton semantics, `underway` for DLQ+backoff+transactional-both-ways) are both effectively single-maintainer, pre-1.0, and in `pgboss-rs`'s case have not published a crates.io release matching `main` in 7 months. For a money-adjacent settlements/order pipeline, depending on such a crate is a bigger long-term liability than owning ~400 lines of SQL+Rust that the team can audit and gate with its own regression tests (consistent with this repo's "Mandatory Proof Rule" / guardrail discipline).
4. **`pgmq` deserves a place in the toolbox, not the framework slot**: it's the freshest, most-adopted, most Supavisor-friendly option of all (pure function calls, zero session state) and is literally the extension behind Supabase's own "Queues" product — meaning it might already be one dashboard toggle away. But it natively provides only the message-durability primitive (send/read/archive/visibility-timeout); retry-backoff, singleton dedup, and cron all still need to be built on top, which is roughly the same amount of app-level code as hand-rolling the whole thing on a plain table — so it doesn't buy enough to justify adding an extension dependency (with its own upgrade/version-pinning surface on Supabase) unless a later requirement (e.g., cross-service fan-out, SQS-style at-least-once semantics beyond a single Rust binary) calls for it specifically.

### Strongest counter-argument

The single best case against "hand-roll it" is: **libraries have already found the bugs you haven't hit yet.** pg-boss itself has ~8 years of production hardening behind its `singletonKey`/`singletonFor`, retry/backoff, and archive semantics — subtle races (e.g., two workers both passing the `singleton_for` window check before either commits, backoff jitter avoiding thundering-herd retries after an outage, correct behavior when a worker crashes mid-job vs. cleanly fails it) are exactly the kind of edge cases that get found via years of GitHub issues, not via a week of writing SQL. `graphile_worker_rs` is a direct port of a similarly mature Node library (Graphile Worker) and ships a real test suite explicitly covering `job_key_mode` semantics and permafail recovery — meaning adopting it gets dowiz those years of edge-case coverage "for free," at the cost of depending on a 79-star, largely single-maintainer crate. If the team's risk tolerance for **novel correctness bugs in hand-rolled concurrency code** is lower than its risk tolerance for **dependency/small-community risk**, `graphile_worker_rs` — not the hand-rolled path — is the more defensible primary choice, and the fallback/primary recommendation above should be inverted. This is a judgment call between two real risks (reinventing hardened logic vs. depending on an unproven-at-scale crate), not a case where one option is objectively safer.

---

## Decision 2: Imaging pipeline

Context: dowiz/DeliveryOS Rust rebuild (axum/tokio/sqlx on Fly.io) needs a `sharp` replacement for menu-item
photo processing: resize, WebP encode, AVIF encode, EXIF strip, on a small/cheap Fly.io VM (1-2 shared
vCPU, 256MB-1GB RAM). Research date: 2026-07-04 (all freshness checks as of this date unless noted).

### Candidates comparison

| Candidate | Resize quality | WebP support | AVIF support | EXIF strip | Memory footprint | Docker image size impact | Maintenance freshness |
|---|---|---|---|---|---|---|---|
| **libvips via Rust FFI bindings** (`olxgroup-oss/libvips-rust-bindings`, crate name `libvips`) | Excellent — libvips' demand-driven pipeline is the reference implementation sharp itself wraps; same resize kernels (bilinear/bicubic/lanczos3/mitchell) as sharp. | Yes, lossy + lossless, via libwebp (bundled dep of libvips). | Yes, via libheif; libheif itself is built against a *pluggable* AV1 encoder — aom, SVT-AV1, or **rav1e** (`compression=av1`, tunable `effort` 0-9). | Yes — native `strip=true` save option / `vips_image_remove()`, the same API sharp exposes. | Low — official libvips benchmark: **0.57s / 94.28MB peak** for crop+downscale+sharpen+save on a 10000×10000 8-bit RGB TIFF, vs ImageMagick `convert` **4.44s / 1499.29MB** (7.8x slower, ~16x more memory) on the same task. | Large — Alpine `apk add vips` pulls **~112MB** of transitive deps (pulls in Python, OpenEXR, etc. per maintainer discussion); Debian `libvips-dev` also "considerable" (exact MB unconfirmed, flagged as an open question). | Very fresh: bindings crate **v2.3.0, 2026-06-19**; wraps libvips C **8.18.3, 2026-06-09**. But binding-crate ecosystem is fragmented — competing unofficial crates (`libvips-rs`, `rs-vips`, `vips-rs`/houseme) exist, single small-team maintainer for the main one (25 open issues, 172 commits). |
| **Pure-Rust: `image` crate + `ravif` (AVIF) + `image-webp` or `webp` crate (WebP)** | Good but the `image` crate's built-in resize is slow/non-SIMD (see AVIF section for a direct benchmark: 29-190ms vs 0.3-15ms). Pairing with `fast_image_resize` (SIMD SSE4.1/AVX2/NEON/wasm128, v6.0.0, actively maintained) closes this gap and keeps resize cost negligible relative to encode cost. | Split answer: `image-webp` (pure Rust, used by `image` 0.25.7+) is **lossless-only** — no lossy VP8 encoder, explicitly deferred by maintainers ("non-trivial task... future possibility"). Lossless WebP on photographic content gives poor compression (comparable to PNG, not JPEG-class). For usable lossy WebP you need the `webp` crate, a safe wrapper around `libwebp-sys` (C dep, ~2.5MB source) — a much smaller, narrower C dependency than all of libvips, but not "zero C deps." | Yes, pure Rust — `image`'s `avif` feature encodes via `ravif`/`rav1e`, no C library needed for encoding (the `avif-native` feature that pulls in C `dav1d` is decode-only, not required if you only ever *produce* AVIF from JPEG/PNG sources). | Effectively free — decoding to raw pixels and re-encoding from scratch does not carry over source EXIF unless you explicitly call the new (0.25.7+) exif-embedding API. Stripping is the default, not an extra step (structural advantage over libvips, where you must remember `strip=true`). | Decode/resize buffers are small (a 4000×2700 RGBA frame ≈ 43MB raw). No published peak-RSS figure found for `ravif`/`rav1e` AVIF-encoding a single ~2000×2000-class photo — flagged as an **open data gap**; rav1e's own design goals target lower peak-RSS/allocation-count than libaom/SVT-AV1, but that's a comparison against other AV1 encoders, not against libvips end-to-end. Recommend a bench spike before committing. | Smallest by far — pure-Rust JPEG/PNG decode + WebP-lossless/AVIF encode needs no C shared libs; a musl-static binary can ship in `scratch`/distroless at a few MB. Adding `libwebp-sys` for lossy WebP adds a small, self-contained C build dep (no runtime shared-lib bloat, ~2.5MB source, compiled via `cc`). | `image` crate: de facto standard (image-rs org), continuously released 0.25.x series, large user base — low bus-factor risk. `ravif` **0.13.0, 2026-01-19** (releases in Oct-2024, Apr-2025, Jun-2025, Jan-2026 — steady cadence). `rav1e` **v0.8.1 tagged 2025-06-16** + weekly pre-releases through at least Sept-2025, 4283 commits, 211 open issues (healthy churn, Xiph-backed). `webp` crate **v0.3.1, 2026-05-16** (fresh). `image-webp` **v0.2.3** (lossless-only ceiling is a permanent-feeling gap, not a freshness problem). |
| **`zune-image` (etemesi254, pure Rust)** | Good for the formats it supports (JPEG/PNG/PPM/QOI/Farbfeld/PSD/JPEG-XL/HDR decode; PPM/QOI/Farbfeld/JPEG-XL/HDR encode) but this is moot given the format gap below. | **No.** Not in the supported-format list at all, decode or encode. | **No.** Not in the supported-format list at all, decode or encode. Maintainer stated years ago that WebP/AVIF are "complicated and complex beasts... will take a while, probably a year or so" — that "year or so" has not materialized; still absent as of the 2025-10-16 release. | N/A — moot without WebP/AVIF output. | N/A | N/A | Latest release **v0.5.3, 2025-10-16**, 44 open issues, single primary maintainer. Actively developed but has never closed the WebP/AVIF gap that this decision requires — **disqualifying** for this use case as a standalone solution. |
| **Shell out to `vips`/`vipsthumbnail` or ImageMagick CLI via `tokio::process::Command`** | Same as whichever binary you invoke — identical to libvips-FFI row if using `vipsthumbnail`. | Same as underlying binary (lossy+lossless via libvips/libwebp, or ImageMagick's own WebP delegate). | Same as underlying binary (libheif/aom/rav1e via libvips, or ImageMagick's AVIF delegate). | Yes, via CLI flags (`--strip` in ImageMagick, `strip=true` in vips CLI options). | Same big-binary RSS profile as option 1 (vips CLI: ~94MB peak per the same benchmark) or much worse if full ImageMagick is chosen (~1.5GB peak per the same benchmark) — **times however many concurrent processes are spawned**, since each process gets its own heap (no shared allocator across requests, unlike an in-process FFI call). | **No smaller than option 1** — you still need the full vips or ImageMagick binary + its shared-lib tree in the image (~100-250MB range); arguably larger for ImageMagick, which bundles many unneeded format delegates by default. | Freshness question shifts entirely to the OS/distro packaging you build from rather than a Rust crate; vips 8.18.3 (2026-06-09) and ImageMagick are both independently well-maintained upstream. | Operational complexity not present in options 1/2: `tokio::process::Command::spawn` is a **blocking** call requiring `spawn_blocking` or a dedicated thread to avoid stalling the async executor; spawned children can become zombies if not reliably reaped (tokio does this "best-effort," no hard timing guarantee); fork/exec of a *heavy* dynamically-linked binary (not the tiny synthetic processes in public spawn-cost benchmarks) pays real copy-on-write + dynamic-linker startup cost on every request, directly competing with the encode work itself for the same 1-2 shared vCPUs under concurrent load. Also loses Rust's compile-time argument-construction safety unless argv arrays are built carefully (shell-injection-adjacent surface for any user-influenced filename/params). |

### AVIF encode cost evidence

AVIF/AV1 still-image encoding is dramatically more CPU-expensive than WebP or JPEG at comparable quality —
this is a property of the AV1 codec's block-partition search, not of any particular wrapper library, and it
shows up consistently across independent sources spanning 2022-2026:

- **kornelski (cavif-rs, rav1e-based) via EWWW Image Optimizer, cited on `ewww.io`:** at the default `--speed 4`
  setting, encoding one photo took **"46 seconds to encode"**, described as **"56x times slower than WebP."**
  Bumping to `--speed 5` brought it to **"8 seconds"**, still **"10x slower than WebP."** On a machine "with
  lots of CPU cores" (unspecified core count), large-scale testing got the ratio down to **"4x"** vs `cwebp`
  — i.e. more cores help a lot, but the gap never closes. (Source: ewww.io blog citing cavif-rs/ravif
  behavior; exact test-image resolution not stated in the retrievable text.)
- **xiph/rav1e maintainers, GitHub Issue #2750 "Still image encoding is expensive":** confirms the same shape
  from the encoder's own upstream: "Encoding of AV1 appears to be much more CPU-intensive than of other image
  formats at comparable compression ratios." At speed ≤4, rav1e beats WebP on size/quality but is "an order of
  magnitude slower"; at speed 6, still slower than `cwebp` with worse output; even at speed 10 (fastest,
  lowest quality) it's still reported "twice as slow" as MozJPEG.
- **libvips maintainers/users, GitHub Issue #2983 "10x slowdown when converting to avif in some cases"
  (2026, libvips using an AV1 backend — aom/rav1e/SVT-AV1 interchangeably via libheif):** a **3840×2160
  (~8.3MP)** PNG→AVIF conversion at `compression=av1, effort=2, Q=80, strip=true` (i.e. a *fast* preset, not
  the default) took **~11 seconds** without a resize step first; resizing before encoding cut this
  dramatically. Peak memory in this issue's traces was small (17-38MB) — confirming the *memory* side of AV1
  encoding is not the bottleneck, **CPU time is.**
- **strukturag/libheif Issue #1458:** independent real-world report of **3.5-10 seconds** to AVIF-encode a
  1280×800 screenshot via libheif/aom, vs 15-30ms for JPEG on the same machine — a **~100-1000x** gap for that
  specific comparison (JPEG encode is unusually cheap, so this ratio is an outlier vs the ~10-56x figures
  above, but the direction is unanimous).
- **`cavif` (kornelski) official docs:** `--speed` 1-10, default 4; **"Speeds 1 and 2 are unbelievably slow,
  but make files ~3-5% smaller"**; **"Speeds 7 and above degrade compression significantly, and are not
  recommended."** Multi-threaded — "the more [cores], the better."
- **libvips `effort` parameter mapping (GitHub issue #1788 discussion):** libvips' AVIF `effort` (0=fastest,
  9=slowest, default 4) is roughly `9 − cpu_used`/`speed` in the underlying aom/rav1e terms — i.e. the same
  speed/quality dial exists no matter which wrapper you pick, because it's the same AV1 encoder underneath.

**Implication for a 1-2 vCPU / 256MB-1GB Fly.io VM:** treat AVIF encoding as a *background/async* job, not
something done synchronously in the request path. At a fast preset (`effort`/`speed` 1-2 out of 9-10), a
single ~2-8MP menu photo will plausibly cost low-single-digit seconds of wall time on a shared vCPU (extrapolating
from the 11s/8.3MP-at-effort=2 and 8s/unspecified-at-speed=5 data points above) — fine for one image, but a
burst of concurrent uploads will queue/starve on a 1-2 vCPU box regardless of which crate/library is chosen.
This is a **structural constraint of AV1, not a library-selection problem** — no candidate in this comparison
escapes it.

### Citations

- https://github.com/xiph/rav1e/issues/2750 — rav1e maintainers confirm still-image AV1 encoding is "an order of magnitude slower" than WebP at usable quality settings — checked 2026-07-04
- https://github.com/libvips/libvips/issues/2983 — real 2026 libvips bug report: ~11s to AVIF-encode an 8.3MP PNG at a fast `effort=2` preset — checked 2026-07-04
- https://ewww.io/2022/06/02/what-in-the-world-is-an-avif/ — concrete cavif-rs/ravif numbers: 46s at speed 4, 8s at speed 5, "56x"/"10x"/"4x" slower than WebP depending on cores/preset — checked 2026-07-04
- https://github.com/strukturag/libheif/issues/1458 — independent real-world report of 3.5-10s AVIF encode for a 1280×800 image vs 15-30ms JPEG — checked 2026-07-04
- https://github.com/kornelski/cavif-rs — official cavif/ravif speed/quality docs (--speed 1-10, default 4, multi-threaded) — checked 2026-07-04
- https://crates.io (fetched via lib.rs/docs.rs mirrors) — ravif v0.13.0 released 2026-01-19, prior releases 2025-06-21 (0.12.0), 2025-04-14, 2024-10-17 — checked 2026-07-04
- https://github.com/xiph/rav1e — v0.8.1 tagged 2025-06-16, weekly pre-releases through at least Sept 2025, 4283 commits, 211 open issues — checked 2026-07-04
- https://github.com/olxgroup-oss/libvips-rust-bindings — v2.3.0 released 2026-06-19, MIT, wraps C libvips via bindgen, 172 commits, 25 open issues — checked 2026-07-04
- https://github.com/libvips/libvips/releases — C libvips v8.18.3 released 2026-06-09 (also 8.18.0 2025-12-17, 8.18.1 2026-03-18, 8.18.2 2026-03-31) — actively maintained — checked 2026-07-04
- https://github.com/libvips/libvips/wiki/Speed-and-memory-use — official benchmark: libvips 0.57s/94.28MB vs ImageMagick convert 4.44s/1499.29MB for the same crop+resize+sharpen task — checked 2026-07-04
- https://github.com/libvips/ruby-vips/issues/370 and https://github.com/libvips/libvips/discussions/3899 — Docker/Alpine `apk add vips` pulls ~112MB of transitive deps (Python, OpenEXR, etc.); Debian `libvips-dev` also "considerable" size (exact MB not published) — checked 2026-07-04
- https://github.com/image-rs/image/blob/main/CHANGES.md — image crate history: AVIF encode via `ravif` since 0.23.10, speed/quality options since 0.23.13, WebP encoding switched to pure-Rust `image-webp` in 0.25.7, EXIF embed/clear helpers added in 0.25.7 — checked 2026-07-04
- https://github.com/image-rs/image-webp — pure-Rust WebP crate is **lossless-only**; lossy VP8 explicitly deferred ("non-trivial task... future possibility") — checked 2026-07-04
- https://docs.rs/webp/latest/webp/ — `webp` crate v0.3.1 (2026-05-16), safe wrapper around `libwebp-sys` (C dep), supports lossy encoding — checked 2026-07-04
- https://docs.rs/fast_image_resize/latest/fast_image_resize/ — v6.0.0, SIMD resize, benchmark: 0.28-15.3ms (SSE4.1) vs 29.28-189.93ms for `image` crate's built-in resize on a 4928×3279→852×567 downscale — checked 2026-07-04
- https://github.com/etemesi254/zune-image and https://github.com/etemesi254/zune-image/discussions/102 — v0.5.3 (2025-10-16), 44 open issues; confirms **no WebP or AVIF support**, encode or decode, years after maintainer said the gap would close "in a year or so" — checked 2026-07-04
- https://kobzol.github.io/rust/2024/01/28/process-spawning-performance-in-rust.html — process-spawn cost benchmarks (10k tiny spawns/~1s on modern glibc) used to reason about CLI shell-out overhead; notes fork/exec cost scales with parent memory and env-var count — checked 2026-07-04
- https://docs.rs/tokio/latest/tokio/process/index.html — `tokio::process::Command::spawn` semantics: blocking call, best-effort zombie reaping, no hard timing guarantee — checked 2026-07-04
- https://github.com/libvips/libvips/issues/1788 — libvips `effort` (0-9) maps onto the same aom/rav1e `speed`/`cpu_used` dial used everywhere else — confirms the AVIF cost is codec-inherent, not wrapper-specific — checked 2026-07-04

### Verdict

**Primary recommendation: libvips via the Rust FFI bindings** (`olxgroup-oss/libvips-rust-bindings`, wrapping
C libvips 8.18.3), built with libheif configured to use rav1e or aom as the AV1 backend, for resize + lossy
WebP + AVIF + EXIF strip through a single dependency.

**Fallback / worth prototyping in parallel: pure-Rust `image` + `fast_image_resize` + `ravif` (AVIF) + `webp`
crate (lossy WebP via a single small `libwebp-sys` C dep)** — see Fallback section.

Confidence: **medium**. High confidence that libvips is the right *format/API* choice (proven low resource
footprint, single mature dependency, sharp-equivalent feature surface). Lower confidence on the Docker-image-size
tradeoff specifically, since exact MB figures for libvips-on-Fly's base image weren't found (flagged gap) and
because AVIF's CPU cost is a hard constraint that applies identically to every candidate, meaning the real risk
to validate before committing is a wall-clock/queueing bench on Fly's actual shared-vCPU class, not a library
choice.

### Rationale

- Requirements need **four capabilities in one coherent pipeline**: resize, lossy WebP, AVIF, EXIF strip.
  Only two candidates actually clear all four: libvips (native, one dependency) and a pure-Rust *stack*
  (`image` + `fast_image_resize` + `ravif` + `webp`/`libwebp-sys`, at least four crates from three different
  maintainers). `zune-image` is disqualified outright (no WebP/AVIF at all). CLI shell-out inherits whichever
  binary's capabilities you invoke and adds process-management overhead on top for no capability gain.
- libvips has the best-documented resource profile of any candidate here: the official maintainer benchmark
  (0.57s/94MB vs ImageMagick's 4.4s/1.5GB) is exactly the kind of "reasonable memory/CPU on a small VM" evidence
  the requirement asks for, and it's a real, reproducible, upstream-published number — not a marketing claim.
- The Rust bindings are *currently* very fresh (2026-06-19, tracking a C library released 2026-06-09), which
  de-risks the "is this abandoned" concern — but the binding-crate layer itself is thin (a small team, several
  competing forks exist), so the *C library* being healthy matters more than the *binding crate* being healthy;
  if the current binding crate stalls, re-bindgen-ing against libvips' C API is a bounded, mechanical task,
  not a rewrite.
- EXIF stripping is a first-class, well-tested libvips feature (the same one `sharp` itself exposes, since
  sharp is a libvips wrapper) — lower implementation risk than assembling metadata-stripping behavior across
  multiple pure-Rust crates.
- The AVIF CPU-cost evidence is unambiguous and *does not favor any one candidate* — it means the architecture
  needs a queue/background-worker/timeout for AVIF regardless of which library wins this decision, and that
  should be designed in from day one rather than discovered under load.

### Strongest counter-argument

The **pure-Rust path's Docker-image-size advantage is real and large**, and this decision explicitly says
"cost-sensitive small Fly.io VM." A statically-linked (musl) Rust binary with `image`+`ravif` needs zero C
shared libraries for AVIF encoding and JPEG/PNG decoding (rav1e/ravif are pure Rust; only lossy WebP needs
`libwebp-sys`, one narrow C dependency vs. libvips' whole transitive tree of ~112MB+ of libraries including
things this project will never use, like OpenEXR or Python bindings pulled in by Alpine's `vips` package).
A smaller image means faster `flyctl deploy`, faster cold boots (relevant if this scales-to-zero or restarts
often), and a smaller CVE/patch surface (no glibc-adjacent C library stack to keep patched independent of the
Rust toolchain). If the team is optimizing hard for Fly.io machine cost and deploy velocity over engineering
convenience, and is willing to own the `image-webp`-is-lossless-only gap by adding the one `libwebp-sys` C
dependency, the pure-Rust stack is a legitimate — arguably better — default, and this memo's "medium" confidence
reflects that this counter-argument was not conclusively defeated by the evidence gathered (no side-by-side
Docker image size number was found for "libvips FFI on Fly's actual base image" vs "musl-static Rust binary,"
only proxy numbers from Alpine/Debian community discussions).

### Fallback option

If the libvips FFI bindings prove fragile in practice (binding crate falls behind a libvips ABI change,
bindgen/build issues on Fly's build image, or the small-team maintenance risk materializes as an unmerged PR
blocking a needed libvips feature), fall back to the pure-Rust stack:

- Decode: `image` crate's built-in pure-Rust JPEG/PNG/GIF decoders.
- Resize: `fast_image_resize` (SIMD, v6.0.0) instead of `image`'s built-in resize (10-20x faster per its own
  published benchmark, and resize cost is negligible either way next to AVIF encode cost).
- WebP encode: the `webp` crate (wraps `libwebp-sys`, v0.3.1, 2026-05-16) for lossy output — the one C
  dependency this fallback keeps, because pure-Rust `image-webp` cannot do lossy WebP and lossless WebP is
  the wrong tool for photographic menu-item photos.
- AVIF encode: `image` crate's `avif` feature (pure Rust, via `ravif`/`rav1e`, v0.13.0, 2026-01-19) — no C
  dependency needed since only encoding (not decoding) AVIF is required.
- EXIF strip: free by construction — a fresh re-encode from decoded raw pixels carries no metadata unless the
  app explicitly calls the 0.25.7+ embed-EXIF API, so simply not calling it satisfies the requirement.
- Second-order fallback (if the Rust crate ecosystem itself proves too immature under production load): shell
  out to `vipsthumbnail`/`vips` (not full ImageMagick, to avoid its larger default dependency set) via
  `tokio::process::Command` + `spawn_blocking`, accepting the same ~100-250MB Docker footprint as the FFI
  option but decoupling the Rust build from libvips' C headers/bindgen entirely — useful only as a last resort
  given the process-spawn and zombie-reaping operational overhead documented above.

---

## Decision 3: OCR for menu import

Context: dowiz/DeliveryOS is rebuilding its backend in Rust (axum/tokio/sqlx) on Fly.io. It needs
OCR for menu-import (photographed/scanned restaurant menus → structured text), currently done via
`tesseract.js` in Node. Albanian (sq / `sqi`) is a required language among others. Goal: keep the
main API Docker image small/static/scratch-based (~15-25MB target) for fast Fly.io cold starts.

### Candidates comparison

| Candidate | Maintenance freshness (2025/2026) | Albanian (sqi) support | Docker image size impact on main API container | Integration complexity | Latency / perf notes |
|---|---|---|---|---|---|
| **1. `leptess`** (houqp/leptess, Rust FFI bindings to Tesseract+Leptonica) | **Stale/effectively abandoned.** Last GitHub commit 2023-09-13, last crates.io publish v0.14.0 on 2023-02-21. Zero commits in 2024, 2025, or 2026. 11 open issues untouched, 286★/30 forks. Confirms community concern that it's dead. | Works — `sqi.traineddata` is a standard, officially-published Tesseract LSTM model (same tier as other languages), just needs to be present in `TESSDATA_PREFIX`. Not leptess-specific. | **Forces away from scratch.** FFI binding links dynamically to `libtesseract`+`liblept` at runtime → both .so libs + their transitive C-library stack (ICU, cairo, pango, harfbuzz, fontconfig) must exist in the running container, plus `.traineddata` files as separate mounted/copied assets. Scratch/distroless-static is not possible for this container. | Medium — safe-ish Rust API, but tied to an unmaintained crate; any Tesseract 5.x API drift or build issue has no upstream fix path. | In-process FFI call, no subprocess overhead, but known past issues with thread-safety/panic propagation from the C layer inside a shared address space. |
| **2. Other Tesseract Rust bindings** (`tesseract`/`tesseract-sys`/`tesseract-plumbing` [ccouzens], `tesseract-rs` [cafercangundogdu], `kreuzberg-tesseract` fork) | Mixed. `tesseract-sys` last crates.io publish 2025-06-14 (v0.6.3, 375k downloads) but its GitHub repo shows "Windows/Mac maintainers wanted" and a 2020 release tag — low-bandwidth but not dead. `tesseract` (higher level) last published 2025-04-19. Newer, separate crate **`tesseract-rs`** (cafercangundogdu) is the freshest binding: v0.2.0 published **2026-03-23**, only 26★/2 open issues (young/small project), builds Tesseract+Leptonica from source via CMake+C++ at compile time rather than linking system libs. Its actively-developed fork **`kreuzberg-tesseract`** is the most active of all (134 published versions, latest 2026-06-24, part of kreuzberg-dev/kreuzberg). | Same as leptess — standard `sqi.traineddata` works with any binding; `tesseract-rs` auto-downloads only eng+tur by default, so sqi would need manual provisioning either way. | Same fundamental constraint as leptess: an FFI binding to Tesseract/Leptonica, whether dynamically linked (ccouzens family) or vendor-compiled-from-source at build time (`tesseract-rs`/kreuzberg), produces a binary/image that embeds the full C++ OCR+image-processing stack — not scratch-compatible. Vendored-from-source at least avoids needing the OS package manager, but the resulting artifact is still large and not a "static single binary + nothing else" story once codec/language-data dependencies are counted. | Medium-high — freshest option (`tesseract-rs`/kreuzberg) is the least battle-tested; ccouzens family is stable but sparse-maintenance. Either way, tied to a binding crate's release cadence for Tesseract version compat. | In-process FFI, no subprocess overhead; compile times increase materially (C++ toolchain + CMake build of Tesseract+Leptonica during `cargo build`). |
| **3. Subprocess/CLI (`tokio::process::Command` → `tesseract` binary via apt/apk)** | N/A (no Rust crate to maintain — decoupled from crate-freshness risk entirely; upstream Tesseract itself is actively released, currently 5.5.x). | Same — install `tesseract-ocr-sqi` (Debian, 1.85MB installed) or Alpine's `tesseract-ocr` + place `sqi.traineddata` in `tessdata`. | **Still forces away from scratch — this is the key finding.** Whether called via FFI or shelled out, the *same* container needs `libtesseract`+`liblept`+transitive deps present. Concrete package math: Debian `tesseract-ocr` core = 2.1MB installed, but `libicu72` alone = **36.2MB** (amd64), plus cairo/pango/harfbuzz/fontconfig graphics stack (~10-15MB combined), plus per-language `.traineddata` (`tessdata_fast` ~2-4MB/lang; Debian's `tesseract-ocr-sqi` = 1.85MB). Realistic addition to a Debian-slim base: **~60-100MB+** on top of the base image — 4-8x over the 15-25MB scratch target. Alpine is leaner (`tesseract-ocr` package itself = 4.5MB installed, musl-based ICU/cairo/pango/harfbuzz stack, ~25-35MB total) but still **2x+ over budget**. A fully-static Tesseract build exists (community projects e.g. `DanielMYT/tesseract-static`, `wingedrhino/static-tesseract`, built originally for AWS Lambda) but requires a custom musl cross-toolchain and works around a known upstream build-system bug that force-links `libstdc++.so` even in static mode — an unofficial, self-maintained path, not a turnkey one. | Low — no Rust crate dependency at all, no FFI unsafe surface, no C++ build step in `cargo build`. Just spawn+wait, parse stdout/stdout-file. Full process isolation: a Tesseract crash/hang can't take down the API process (a real reliability plus over in-process FFI). | Process spawn (fork+exec) + temp-file/pipe I/O adds tens of ms, but OCR itself is CPU-bound (typically ~0.5-3s/page) so subprocess overhead is noise. Easiest to bound with a timeout via `tokio::time::timeout` around `Child::wait()`. |
| **4. Separate OCR microservice/sidecar (own container, or hosted API)** | N/A — decoupled entirely; you pick Tesseract-in-a-container (Python+tesseract or Rust+axum+tesseract-rs, e.g. reference impl `seferino-fernandez/ocr_service`) on your own release cadence, or a hosted API (AWS Textract, Google Cloud Vision, Azure Read, Mistral OCR) with its own SLA. | Same Tesseract sqi support if self-hosted. For hosted APIs, language coverage varies by vendor/feature (e.g., Textract's structured form/table extraction is more English/Latin-form-centric; Google Cloud Vision's general text-detection language list is broad and commonly includes Albanian) — **must be verified per-vendor before committing**, not assumed. | **Only option that keeps the main API image scratch/distroless-static.** The heavy C/C++ OCR stack lives in a wholly separate deployable unit. On Fly.io this can be done two ways: (a) Fly Machines' multi-container feature (`containers` array + Pilot init) to co-locate an OCR sidecar container on the *same* Machine, talking over `localhost`; or (b) a fully separate Fly App/Machine reachable over Fly's private 6PN network (`<app>.internal` DNS) via HTTP. Either keeps the main image at target size; the OCR container itself stays in the 100-250MB range, but that no longer matters for the main API's cold-start profile. | Medium — new deployable surface (extra Dockerfile, health checks, versioning), plus a network call + (de)serialization boundary from the Rust API. Hosted-API option removes ALL image/ops burden but adds a paid external dependency, PII/data-residency review (menu photos may contain no PII, but still an external egress to evaluate), and rate-limit/outage handling. | Same-machine sidecar: ~1-5ms overhead, negligible. Cross-app-over-6PN: ~1-10ms intra-region. Hosted cloud API: real internet round-trip (tens to hundreds of ms) plus external rate limits — fine for an async "process this uploaded menu" job, less ideal for a synchronous request path. |

### Citations

- https://github.com/houqp/leptess/commits/master — leptess's last commit is 2023-09-13 ("Update dependencies #58"); zero commits since — checked 2026-07-04
- https://crates.io/api/v1/crates/leptess — leptess crates.io: max/newest version 0.14.0, last published 2023-02-21, 189,748 downloads — checked 2026-07-04
- https://crates.io/api/v1/crates/tesseract — "tesseract" crate: v0.15.2, last published 2025-04-19, 194,530 downloads, repo listed as antimatter15/tesseract-rs — checked 2026-07-04
- https://crates.io/api/v1/crates/tesseract-sys — v0.6.3, last published 2025-06-14, 375,120 downloads — checked 2026-07-04
- https://github.com/ccouzens/tesseract-sys — 29★, 5 open issues, "Windows and Mac maintainers wanted", last tagged GitHub release 0.5.5 (Dec 2020) despite newer crates.io publishes — checked 2026-07-04
- https://crates.io/api/v1/crates/tesseract-rs — cafercangundogdu's crate: v0.2.0, last published **2026-03-23**, 48,630 downloads — checked 2026-07-04
- https://github.com/cafercangundogdu/tesseract-rs — 26★, 2 open issues, builds Tesseract+Leptonica from source (CMake+C++), supports Linux/macOS/Windows, requires Rust ≥1.83 — checked 2026-07-04
- https://crates.io/api/v1/crates/kreuzberg-tesseract — most active fork: 134 versions published, latest **2026-06-24** (v5.0.0-rc.35), 34,184 downloads, repo kreuzberg-dev/kreuzberg — checked 2026-07-04
- https://packages.debian.org/bookworm/tesseract-ocr — core package: 392.8kB download / 2,135.0kB installed; deps include libtesseract5, liblept5 (≥1.75.3), libicu72, libstdc++6, cairo/pango/fontconfig/libarchive stack — checked 2026-07-04
- https://packages.debian.org/bookworm/libicu72 — libicu72 installed size 36,170.0kB (amd64) — a single transitive dependency bigger than the entire scratch-image budget — checked 2026-07-04
- https://packages.debian.org/sid/tesseract-ocr-sqi — Albanian language pack (`sqi`): 1,846.0kB installed, recommends `tesseract-ocr` ≥4.9.9 — checked 2026-07-04
- https://pkgs.alpinelinux.org/package/edge/community/x86_64/tesseract-ocr — Alpine `tesseract-ocr`: 4.5MiB installed, 14 runtime deps (musl, cairo/fontconfig/harfbuzz/pango, ICU, glib/gobject, leptonica, libgcc/libstdc++/libgomp) — checked 2026-07-04
- https://github.com/tesseract-ocr/tessdata_fast (and tessdata/tessdata_best) — `eng.traineddata` sizes: tessdata_fast ≈3.92MB, tessdata (default) ≈22.4MB, tessdata_best ≈14.7MB — Debian/Ubuntu package the `fast` variant — checked 2026-07-04
- https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html — confirms `sqi` (Albanian) is present as a standard officially-published trained-data language across Tesseract versions, same tier as other supported languages — checked 2026-07-04
- https://github.com/DanielMYT/tesseract-static and https://github.com/wingedrhino/static-tesseract — community-maintained fully-static Tesseract builds (musl toolchain) originally built for AWS Lambda; note a documented upstream build-system bug that force-links `libstdc++.so` even under static linking — checked 2026-07-04
- https://fly.io/docs/machines/guides-examples/multi-container-machines/ — Fly Machines support a `containers` array (via Pilot init) to co-locate sidecar containers on one Machine, communicating over localhost — checked 2026-07-04
- https://github.com/seferino-fernandez/ocr_service — reference implementation: Axum + `tesseract-rs` OCR microservice with `/api/v1/images` endpoint and Docker/Docker-Compose support — checked 2026-07-04
- https://github.com/robertknight/ocrs — pure-Rust neural-net OCR (ONNX/RTen) alternative worth a future look if Tesseract's C dependency chain becomes a recurring pain point; not evaluated for Albanian coverage in this pass — checked 2026-07-04

### Verdict

Use **Option 4: a separate OCR microservice/sidecar**, self-hosted with Tesseract (not a hosted cloud
API), called over Fly.io's internal network (or same-Machine multi-container sidecar) from the main
Rust API. Do not bind Tesseract/Leptonica into the main API's process (rules out both `leptess` and
every alternative Rust binding crate, including the freshest ones), and do not shell out to the
`tesseract` CLI from inside the main API container either — both keep the C/C++ OCR stack resident
in the same image you're trying to shrink to 15-25MB.

### Rationale

The size-forcing constraint is structural, not a library-choice problem: `libtesseract`+`liblept`
pull in ICU (36MB installed on Debian alone — bigger than the entire scratch budget by itself),
plus cairo/pango/harfbuzz/fontconfig, plus per-language `.traineddata` files. This is true whether
Tesseract is reached via FFI (`leptess`, `tesseract-sys`, `tesseract-rs`) or via subprocess
(`tokio::process::Command`) — the subprocess approach only removes the *Rust-crate maintenance*
risk (which matters, since `leptess` is confirmed dead since 2023 and even the freshest bindings are
young/small or low-bandwidth), it does nothing for image size. The only architecture that actually
protects the main API's scratch/distroless-static goal is physically separating the OCR runtime into
its own container/Machine, which Fly.io explicitly supports either as a same-Machine sidecar
(lowest latency, `containers` array + Pilot) or a separate app over the private 6PN network. Since
menu-import OCR is not a hot, latency-critical path (it's a background/async "process this uploaded
scan" job, not a per-request storefront call), the extra network hop is an acceptable, well-precedented
trade for keeping the primary API's cold-start profile intact. Self-hosting Tesseract in that sidecar
(vs. a hosted API) avoids introducing a new paid vendor dependency, external egress of restaurant menu
images, and per-vendor language-support verification for a well-solved, already-proven (`tesseract.js`
today) use case — Albanian sqi is a standard, official Tesseract language pack, not a special case.

### Strongest counter-argument

If menu-import volume is low and truly asynchronous (e.g., a queued job processed within seconds-to-
minutes, not blocking any user-facing request), the added operational surface of a second deployable
service (its own Dockerfile, health checks, versioning, and a new internal network hop to reason about)
may be more complexity than it's worth for what could be a rarely-invoked code path. In that case,
accepting a larger single image for a low-traffic *admin-only, background* import feature — i.e.,
just apt-installing `tesseract-ocr` + `tesseract-ocr-sqi` into the main API image and shelling out via
`tokio::process::Command` (Option 3) — is a legitimate simpler alternative, provided the team is
comfortable that the "15-25MB scratch" target becomes "15-25MB for most routes, one heavier image for
the whole service" rather than an unconditional constraint. This trades architectural purity for one
fewer moving part, and is defensible if profiling shows menu-import is rare enough that the
image-size/cold-start cost doesn't matter in practice (Fly.io cold-start impact scales with image
size on the machines that actually run this route, not globally, if process groups are used).

### Fallback option

If the team wants to keep everything in one deployable unit but still preserve a small image for the
*hot* request path, use **Fly.io process groups** (`fly.toml` `[processes]`) to run two process groups
from the same app/image family: a `web` group (no Tesseract, scratch/distroless-static, small,
fast-scaling) and a separate `worker`/`import` group that runs a heavier image with Tesseract installed,
scaled independently (including scale-to-zero when idle). This gets most of the benefit of Option 4
(hot path stays small) without standing up a wholly separate service/sidecar, at the cost of maintaining
two Dockerfiles/image variants within the same app. If Tesseract's C-dependency weight becomes a
recurring irritant regardless of architecture, `ocrs` (github.com/robertknight/ocrs, pure-Rust ONNX-based
OCR) is worth a follow-up evaluation — but its language-model coverage (including Albanian) was not
verified in this pass and would need its own research spike before being considered a real alternative.

---

## Decision 4: PDF extraction

Context: dowiz/DeliveryOS backend rebuild in Rust (axum/tokio/sqlx) on Fly.io. Need PDF text + image extraction for menu-import (owner-uploaded PDF menus parsed into structured products). Project's own plan (ADR-020) is to open-source under AGPLv3 — but that flip is currently **gated** (pending secrets scrub + EUTM filing per project memory), not yet final.

### Candidates comparison

| Candidate | License | Maintenance freshness (2025/2026) | Text extraction quality | Image extraction capability | Binary/build complexity | Alpine/musl/static-container compatibility |
|---|---|---|---|---|---|---|
| **pdfium-render** (wraps Google PDFium) | Crate: MIT OR Apache-2.0. PDFium itself: BSD-3-Clause + Apache-2.0 (Google/Chromium project) | Very active — v0.9.3 latest; v0.9.0 (2026-03-30), v0.9.1 (2026-05-02) tracking new PDFium builds monthly-ish; maintainer (ajrcarey) responsive, issues opened/triaged through 2025-2026 | Excellent — literally Chrome's PDF engine; reported to handle malformed/real-world PDFs and complex layouts that pure-Rust libs reject | Strong — full rendering engine, can rasterize pages and pull embedded images | Low at compile-time (no C build) but **binary-blob distribution**: crate itself ships no PDFium; must dynamically link a prebuilt shared lib (bblanchon/pdfium-binaries or paulocoutinhox/pdfium-lib) or statically link your own build; ~20MB shared-lib footprint reported in practice | **Prebuilt musl builds (x64/x86/arm64) are upstream-broken** (bblanchon/pdfium-binaries issues #191/#192/#193, all open/failing). glibc builds (Debian-slim/Ubuntu) work fine and are the community-recommended base — Alpine explicitly discouraged ("requires additional compatibility layers") |
| **lopdf** | MIT (one bundled font, Montserrat, excluded) | Very active — v0.43.0 released 2026-06-29; 27 releases, 2.2k★/268 forks | Weak-to-mediocre for extraction specifically — it's a manipulation library first; has `extract_text()` but real users report it "struggles with complex font handling" and is "strict" where "real-world PDFs often aren't." Historical CID/Identity-H Unicode decode gaps (#125) and ToUnicodeCMap parse failures (#330), partially fixed in v0.39.0 | Not a focus — low-level access to image XObjects only, no high-level decode/extract API | Trivial — pure Rust, zero C dependencies | Excellent — zero native deps, works out-of-the-box on musl/Alpine/scratch, static binaries |
| **pdf-extract** | MIT | Active — v0.8.2 (2025-02-09), v0.9.0 (2025-04-03), v0.10.0 (2025-10-03); regular cadence | Purpose-built for text extraction but with real robustness gaps in the wild: 51 open issues incl. a reported extraction-quality regression (font cache persisting across pages), unsupported CJK encodings (GBK-EUC-H, #126), panics on malformed content streams (#134, #129) | None — text-only; README defers to other tools for layout/images | Trivial — pure Rust (built on lopdf internally, inherits its parsing strictness) | Excellent — same as lopdf, no native deps |
| **mupdf-rs** (`mupdf` + `mupdf-sys`, wraps Artifex MuPDF) | **AGPL-3.0** (crate explicitly declares this, mirrors upstream MuPDF license) | Very active — v0.8.0 (2026-06-22), v0.7.0 (2026-05-25), v0.6.0 (2026-01-19), v0.5.0 (2025-04-27); 9 open issues, 498 commits | Strong — MuPDF is a mature, widely-used production PDF engine (text, words+bboxes, structured text) | Strongest of the four — single library extracts plain text, positioned words, images, **and** vector drawings from the same page object; can also rasterize scanned pages as an OCR-handoff point | High — `mupdf-sys` **compiles MuPDF from source** via CMake + bindgen (needs libclang, C/C++ toolchain, git submodules for vendored MuPDF, Fontconfig dev headers if system-fonts feature on) | Unproven/risky for musl specifically in the Rust binding; no direct evidence found either way, but upstream PyMuPDF (same MuPDF C core) has a **recurring history of Alpine/musl build breakage** across versions (#4841, #3279, #391, discussion #3360) — treat as an open risk requiring a build spike, not a proven blocker |

### License analysis

**PDFium** (BSD-3-Clause / Apache-2.0, Google) and **pdfium-render**, **lopdf**, **pdf-extract** (all MIT) are fully permissive — zero license interplay with anything, proprietary or copyleft. They impose no obligation regardless of what dowiz ultimately does with its own licensing.

**mupdf-rs / MuPDF is AGPL-3.0**, dual-licensed by Artifex (commercial license as the paid alternative to AGPL obligations). This is the case the task explicitly asked to reason through:

- **Is there a conflict with dowiz's own planned AGPLv3 release?** No — this is a clean fit, not a conflict. AGPL is self-compatible: combining AGPL-licensed code with an application that is *itself* licensed AGPLv3, and distributing/operating the combined work under AGPLv3 (including honoring the network-use "offer corresponding source" clause, §13), fully satisfies both the dependency's license and the project's own. Community consensus confirms this (FOSSA, Vaultinum AGPL-compliance guides): two AGPL works combined under AGPL create no direct incompatibility — the risk category that *does* exist is transitive MIT/Apache dependencies *of* an AGPL package conflicting with AGPL, which is not the situation here (PDFium/lopdf/pdf-extract are all permissive and would layer under an AGPLv3 umbrella without issue too).
- **Does Artifex require a commercial license even for open-source users?** Legally, no — the AGPL text alone should be sufficient if dowiz ships full corresponding source of the *whole* application (not just the MuPDF bindings) and preserves that offer to every network user. But there is a documented **pattern of Artifex/maintainer behavior that reads as commercially aggressive**: in the PyMuPDF licensing discussion (github.com/pymupdf/PyMuPDF/discussions/971), the maintainer's default response to *any* question with a whiff of commercial use — even MIT-licensed downstream integrations like LangChain, or purely internal enterprise use — is "contact Artifex, they'll find you an attractive solution," rather than a straightforward "AGPL covers you." Artifex's own licensing page frames AGPL's copyleft requirements as "intentionally robust... which incentivizes commercial licensing" — i.e., the dual-license business model depends on some ambiguity pushing users toward a paid contract even when strict compliance would suffice. This is a **business/relationship risk (unsolicited sales pressure, contract ambiguity), not a hard legal blocker**, provided dowiz's own AGPLv3 release is genuine and complete.
- **Practical implication for dowiz**: adopting mupdf-rs is legally clean *once* the AGPLv3 flip is real and complete. But that flip is currently gated (pending secrets scrub + EUTM filing per project memory) — taking on an AGPL runtime dependency today effectively pre-commits the codebase to the AGPLv3 outcome before that gate clears, since ripping MuPDF back out later (if the flip stalls or a proprietary fork is ever needed) means either paying Artifex or re-doing this decision. This is the one license-shaped reason to prefer a permissively-licensed candidate (pdfium-render) even though "AGPL is not an automatic blocker" holds true in principle.

### Citations

- https://github.com/ajrcarey/pdfium-render — dynamic/static/system-lib binding options, no bundled PDFium, MIT OR Apache-2.0 license, v0.9.3 — checked 2026-07-04
- https://crates.io/crates/pdfium-render — release cadence (0.9.0 2026-03-30, 0.9.1 2026-05-02) — checked 2026-07-04
- https://github.com/chromium/pdfium/blob/main/LICENSE — PDFium dual BSD-3-Clause / Apache-2.0 — checked 2026-07-04
- https://github.com/bblanchon/pdfium-binaries/issues/52, /issues/134 — musl support requests/history — checked 2026-07-04
- (search-derived) bblanchon/pdfium-binaries issues #191/#192/#193 — musl x64/arm64/x86 builds marked failing — checked 2026-07-04
- https://michaelbommarito.com/wiki/programming/tools/pdfium-docker/ — Debian/glibc recommended over Alpine for PDFium Docker deployment; LD_LIBRARY_PATH and font-package gotchas — checked 2026-07-04
- https://dev.to/hiyoyok/lopdf-vs-pdfium-in-rust-what-i-learned-building-a-pdf-app-233b — real-world builder's verdict: lopdf for structural ops only, pdfium for malformed/complex-layout text extraction, ~20MB bundle size concern — checked 2026-07-04
- https://github.com/J-F-Liu/lopdf — MIT license, v0.43.0 (2026-06-29), extract_text() API, manipulation-first focus — checked 2026-07-04
- https://github.com/J-F-Liu/lopdf/issues/125, /issues/330 — Unicode/CID decode and ToUnicodeCMap parsing gaps — checked 2026-07-04
- https://github.com/jrmuizel/pdf-extract — MIT license, text-only focus — checked 2026-07-04
- https://crates.io/crates/pdf-extract — release dates 0.8.2 (2025-02-09), 0.9.0 (2025-04-03), 0.10.0 (2025-10-03) — checked 2026-07-04
- (search-derived) jrmuizel/pdf-extract issues (51 open) — font-cache regression, GBK-EUC-H unsupported, InvalidContentStream panics — checked 2026-07-04
- https://github.com/messense/mupdf-rs — AGPL-3.0, v0.8.0 (2026-06-22), 9 open issues, 498 commits, text+image+vector extraction, CMake/bindgen/Fontconfig build requirements — checked 2026-07-04
- https://artifex.com/licensing/agpl/ (title/metadata only, 500 on fetch) + search summary — dual AGPL/commercial model framing — checked 2026-07-04
- https://github.com/pymupdf/PyMuPDF/discussions/971 — maintainer steers any commercial-flavored question toward "contact Artifex," even for MIT-licensed or internal use — checked 2026-07-04
- https://github.com/pymupdf/PyMuPDF/issues/4841, /issues/3279, /issues/391, discussion #3360 — recurring Alpine/musl build breakage for the underlying MuPDF C core across versions — checked 2026-07-04
- https://vaultinum.com/blog/essential-guide-to-agpl-compliance-for-tech-companies, https://fossa.com/blog/open-source-software-licenses-101-agpl-license/ — AGPL-in-AGPL is not a direct conflict; transitive non-copyleft deps are the actual risk category — checked 2026-07-04

### Verdict

**Primary: `pdfium-render`**, deployed against a **glibc base image (Debian-slim/Ubuntu on Fly.io)**, not Alpine/musl. Use `lopdf` alongside it only for any structural PDF manipulation needs (not extraction).

### Rationale

Menu-import has to survive arbitrary, messy, owner-uploaded PDFs (exports from Word/Canva/POS systems, multi-column layouts, embedded photos) — this is exactly PDFium's home turf: it's Chrome's production PDF engine, actively maintained through mid-2026, and both text and image extraction (via page rendering) are strong in one library. Its license (BSD/Apache-2.0 via crate MIT/Apache-2.0) is a total non-issue regardless of what dowiz's own licensing ends up being, which matters because the AGPLv3 flip (ADR-020) is explicitly gated/not-yet-final — a permissive PDF-extraction dependency doesn't force or pre-empt that decision. The one real deployment wrinkle — prebuilt musl binaries for pdfium are upstream-broken — is fully solvable by building the Fly.io image on Debian-slim (glibc), which is already a common, well-supported Rust-on-Fly.io pattern, not an exotic one.

### Strongest counter-argument

`mupdf-rs` is arguably the technically stronger pick: one library gets text + images + vector drawings + scanned-page rasterization (a clean OCR handoff point) instead of composing pdfium (rendering) with something else for edge cases, and its AGPL-3.0 license is — per the task's own framing — **not automatically a blocker** for a project that plans to be AGPLv3 itself; combining AGPL with AGPL is a clean, legally uncomplicated fit, and dowiz is exactly the kind of project the AGPL is meant to serve well. If the AGPLv3 flip is treated as effectively certain rather than merely gated, mupdf-rs should be re-evaluated as the primary choice, with the caveats that (a) its build is a from-source C/C++ compile (heavier CI, unproven musl story per PyMuPDF's rocky Alpine history) and (b) Artifex's sales-first posture toward any "commercial-sounding" use case (documented in the PyMuPDF licensing discussion) is worth a one-time confirmatory email before shipping, purely as relationship/PR risk management, not because the license text requires it.

### Fallback option

If the pdfium binary-distribution/glibc-base-image constraint turns out to be unworkable for the target Fly.io container strategy (e.g. a hard requirement for Alpine/musl/scratch for image-size or security reasons), fall back to a **pure-Rust `pdf-extract` (primary) + `lopdf` (structural ops) combo** — zero native dependencies, trivially musl/Alpine/static-container compatible, permissively licensed. Accept the trade-off explicitly: real-world quality gaps (CJK encoding gaps, occasional panics on malformed content streams, weaker performance on non-trivial layouts) mean a higher share of uploaded menu PDFs will need manual owner correction or will fail extraction outright and require routing to the OCR fallback path (Decision 3, researched separately) — including for any scanned-image-only PDFs, which none of these four text-extraction libraries can handle without an OCR step regardless of which is chosen.
