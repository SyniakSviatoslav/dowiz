# Lane 1 — Current-State Audit (dowiz / DeliveryOS)

> Evidence-grounded baseline for the rebuild-plan research effort. This document establishes WHAT
> EXISTS and WHERE THE PAIN IS. It proposes nothing. All numbers measured 2026-07-04 on branch
> `fix/audit-remediation`; repowise index age 11d (commit-level claims re-verified against live tree).
>
> Total product+test TypeScript: **~145.6k LOC** across `apps/ packages/ tools/ spikes/ e2e/ scripts/`
> (measured: `find … -name '*.ts' -o -name '*.tsx' | xargs wc -l` = 145,559).

---

## 1. Module map

### 1.1 Workspace members (`pnpm-workspace.yaml`: `apps/*, packages/*, tools/*, spikes/*`)

| Package | LOC (TS/TSX) | Files | Purpose | Key deps | Internal deps |
|---|---|---|---|---|---|
| `apps/api` | 59,881 | 410 | Fastify 5 monolith: HTTP API + WS + SSR + widget/PWA client bundles + ~15 in-process background workers | fastify, pg, pg-boss, ioredis, jose, argon2, zod, sharp, tesseract.js, ws, preact+htm, Sentry, OTel | config, db, domain, platform, shared-types |
| `apps/web` | 20,776 | 103 | React 18 + Vite SPA — admin, courier, client storefront | react, react-router-dom 7, framer-motion, maplibre-gl, three, zod | shared-types, ui |
| `apps/worker` | 261 | 6 | Thin pg-boss consumer (separate Fly process: `ORDER_TIMEOUT`, health-job) | pg | config, db, platform, shared-types |
| `packages/config` | 254 | 2 | Zod env schema (~122 `z.` entries; 10 `*_ENABLED` feature flags, default-off) | zod | — |
| `packages/db` | 10,192 | 169 | **157 forward-only migrations** (`packages/db/migrations/`) + pool factories (`createOperationalPool`/`createSessionPool`) + verify-db | pg, node-pg-migrate | config |
| `packages/domain` | 88 | 3 | Order status state machine (`assertTransition`) + errors. **Anemic** — real domain logic lives in `apps/api/src/lib` | — | — |
| `packages/platform` | 1,018 | 11 | Ports: `RedisMessageBus`, `PgBossQueueProvider`, routing-provider, **RS256 JWT** (`auth/jwt.ts`), **`withTenant`** RLS wrapper (`auth/tenant.ts`) | jose, ioredis, pg-boss | config, db, shared-types |
| `packages/shared-types` | 1,590 | 32 | Zod API contracts (`CreateOrderInput` …), `QUEUE_NAMES`, `formatMoney`, allergens | zod | — |
| `packages/ui` | 14,328 | 106 | 44 shared components + theme/tokens + **i18n** (`lib/i18n.ts` 114 LOC + `lib/i18n-catalog.ts` **4,343 LOC**, al/en SSoT) | react | shared-types |
| `packages/voice` | 1,531 | 17 | Voice ASR engine (dark, dep-optional; ADR-0015) | transformers.js (optional) | — |
| `tools/*` | ~5,035 | 69 | `loop-harness` 4,041 · `ccc` 450 · `eslint-plugin-local` 393 · `skillspector` 151 · 2 empty (`acquisition-bulk-provision`, `demo-builder` — 0 TS LOC) | — | — |
| `spikes/stage3-queue` | 369 | 14 | Queue spike — **imports prod packages** (config, db); fenced by `guardrail:spike-boundary` | — | config, db |
| `e2e/` (non-workspace) | 27,696 | 174 specs | Playwright net against deployed staging | playwright | — |
| `scripts/` (root) | 90 files | — | migrate-runner, build-apps (esbuild bundler), guardrails, verify-* | — | — |

### 1.2 Real internal dependency graph

```
web ──► ui ──► shared-types
 │                 ▲
 └──► shared-types │
api ──► platform ──► db ──► config
 │        │  └────► shared-types
 ├──► domain, config, db, shared-types
worker ─► platform, db, config, shared-types
voice ─► (nothing; consumed nowhere in committed tree yet)
spikes/stage3-queue ─► config, db   ← spike touching prod packages (gated)
```

Clean in shape (no cycles, apps never import apps). The violations are **content**, not edges:

- **Frontend inside the API app.** `apps/api/src/client/` (14 files, 1,583 LOC: cart/checkout/menu/status/pwa/embed/widget) is browser code bundled by `apps/api/build-client.js` into `apps/api/public/dist/` and served by fastify-static. Plus `apps/api/src/lib/ssr-renderer.ts` (442 LOC, Preact+htm). The API package owns three UI surfaces.
- **Copy-mirror instead of import.** `apps/worker/src/handlers.ts:5-7` re-declares `orderChannel`/`dashboardChannel` with the comment "Channel names mirror apps/api/src/lib/registry.ts" — because `BUS_CHANNELS` lives in `apps/api/src/lib/registry.ts` while `QUEUE_NAMES` lives in `packages/shared-types/src/queue-names.ts`. The channel registry is split across an app and a package.
- **Two parallel i18n systems.** FE strings in `packages/ui/src/lib/i18n-catalog.ts`; server notification strings in `apps/api/src/notifications/locales.ts` + `bot-strings.ts` (which itself says "Mirrors the server-side i18n pattern in locales.ts", `bot-strings.ts:2`).
- **Anemic domain package.** `packages/domain` = 88 LOC; pricing, dispatch, preflight, idempotency, venue-open, signals all live in `apps/api/src/lib/` (75 files, 8,547 LOC) — reusable-in-name-only monolith internals.

### 1.3 `apps/api` internal breakdown (the real monolith)

| Subdir | Files | LOC | Content |
|---|---|---|---|
| `routes/` | 65 | 15,024 | 50+ route plugins wired in `bootstrap/routes.ts` (owner/, courier/, customer/, public/, dev/) |
| `lib/` | 75 | 8,547 | money, order-pricing, dispatch, preflight, ssr-renderer, spa-shell, pii-cipher, anonymizer, … |
| `workers/` | 30 | 4,368 | in-process background workers (dwell, signals, GDPR, retention, backup, courier cron/dispatch) |
| `notifications/` | 15 | 2,296 | provider registry, render, telegram adapter/poller, own locale dictionary |
| `client/` | 14 | 1,583 | vanilla-TS browser bundles (see §2.5) |
| `modules/` | 12 | 1,280 | acquisition vertical |
| `bootstrap/` | 4 | 555 | routes.ts / workers.ts / notifications.ts extracted from `main()` |
| `plugins/` | 3 | 279 | auth (167 LOC), dev-guard |

---

## 2. Architecture reality

### 2.1 HTTP request lifecycle

`apps/api/src/server.ts` `main()` is the composition root (890 LOC file; `main` flagged
`large_method`/`complex_conditional` critical by repowise):

1. `loadEnv()` (Zod-validated, `packages/config`) → `createOperationalPool` → Redis bus → PgBoss queue → Sentry/OTel.
2. Fastify with **Zod validator + serializer compilers** set globally (`server.ts:~107-124`).
3. Hook chain: security headers (HSTS/nosniff/XFO/Referrer-Policy, `server.ts:~125-135`) → **default-deny CORS** with a public-route override hook (`/public/locations/`, `/s/`, `POST /api/orders` get `ACAO:*`, `server.ts:141-153`) → fastify-static → onSend cache-control shaping (HTML no-store, assets immutable) → subdomain rewrite resolver → rate-limit → auth plugin.
4. `registerCoreRoutes(fastify, {db, messageBus, queue, …})` (`bootstrap/routes.ts:86`) registers ~50 route plugins; **DI is manual via plugin opts** — no container, every provider constructed in `main()` (storage R2/local-fs, CSV/AI-OCR parsers, LibreTranslate, payments…).
5. Error contract: `ApiError` + `buildErrorEnvelope` + `reply.sendError` decorator (`lib/api-error.ts`, ADR-0010) — uniform envelope, contract codes.
6. Boot backstop: `lib/schema-guard.ts` FATAL-exits if the DB is behind the migration head stamped at build time (esbuild `define`); `release_command` runs migrations pre-deploy.

Deploy topology (`fly.toml [processes]`): `web` = `dist/api/server.cjs`, `worker` = `dist/worker/index.cjs`; bundled by `scripts/build-apps.ts` (esbuild, 4 entrypoints: server, worker, all 157 migrations, migrate-runner).

### 2.2 Auth / session model

- **RS256-only JWT** via jose, kid-rotation aware; algorithm pinned both at sign and verify (`packages/platform/src/auth/jwt.ts:55-58, 106-110` — "only RS256 accepted").
- **Owner**: ≤24h access token; **never trusted alone** — a live `memberships … role='owner' AND status='active'` re-check runs per request (ADR-0004 `#6`; `plugins/auth.ts:147`, duplicated inside `routes/spa-proxy.ts` helpers). Revocation is immediate, not TTL-bound.
- **Courier**: DB session rows with `revoked_at`/`expires_at` checked per request (`plugins/auth.ts:13-27,75`).
- **Customer**: short-lived order-scoped token (`issueCustomerToken`, used in `routes/orders.ts`).
- Passwords argon2 (with `argon2-params-lock.test.ts` pinning params). Dev/mock auth fenced by `plugins/dev-guard.ts` (post ADR-0003 backdoor incident).

### 2.3 Data access: raw pg + RLS

Two coexisting disciplines:

1. **`withTenant(pool, userId, fn)`** — `BEGIN; set_config('app.user_id', $1, true); … COMMIT` (`packages/platform/src/auth/tenant.ts:3-21`), used by **24 files** in `apps/api/src`. RLS policies key off `app.user_id`; **30 migrations contain `FORCE ROW LEVEL SECURITY`**.
2. **Direct pool queries** — **174 `db.query(` call sites** in `apps/api/src` hit the operational pool with hand-written `WHERE location_id = $n` tenancy. RLS is a backstop here only if the pool role lacks BYPASSRLS (known open item: B3 "NOBYPASSRLS hard dep" per launch-blocker council).

No ORM, no query builder — SQL strings inline in routes/libs. Pools split by purpose (operational / session / migrations DSNs; staging exposes `DATABASE_URL_{MIGRATIONS,OPERATIONAL,SESSION}`).

### 2.4 WS + bus + jobs

- **WS** (`apps/api/src/websocket.ts`, 531 LOC, `ws`): rooms `order:{id}`, `location:{id}:dashboard`, `location:{id}:couriers`, `courier:{id}:shift` (`lib/registry.ts:47-50`). **Tri-state authz** (`ownerRoomVerdict` `websocket.ts:35`, `courierRoomVerdict`) enforced at **subscribe AND per-frame fan-out** with short-TTL revalidation cache and socket eviction on revocation (ADR-0013; the `#4` owner-streaming-after-revocation fix is documented inline `websocket.ts:83-93`).
- **Bus**: `RedisMessageBus` (243 LOC) — cross-instance pub/sub fan-out to WS.
- **Jobs**: `PgBossQueueProvider` (132 LOC). Enqueued from ~15 files. Consumed in **two deployables**: `apps/api` in-process (`bootstrap/workers.ts:49` — notify dispatch/customer-status/telegram, courier dispatch/cron, dwell monitor, signal raiser, anonymizer/GDPR, 4 retention workers, backup) and `apps/worker` (`ORDER_TIMEOUT` auto-cancel + audit insert, `handlers.ts:15-40`; plus an API-side `order-timeout-sweep.ts` re-enqueuing missed timeouts). One durable-jobs substrate, split consumer topology, and the timeout job's business logic (status mutation + history insert) lives in the thin worker as raw SQL — outside `assertTransition`.

### 2.5 Storefront: the React-vs-Preact-vs-vanilla TRIPLE reality

`GET /s/:slug` (`routes/public/ssr.ts:18-54`):

- **Bots** (UA sniff `isBot`, `lib/spa-shell.ts`) → **Preact+htm SSR** `renderMenuPage` (`lib/ssr-renderer.ts`, 442 LOC) for JSON-LD/OG; hydrated by the dedicated **vanilla-TS bundle** `src/client/menu/app.ts` (comment at `ssr-renderer.ts:8-9`).
- **Humans** → `serveSpaShell` = the **React SPA** (apps/web) with per-location CSP; cart/checkout/status deep links likewise serve the shell (`routes/public/client-flow.ts:15-18`).
- Shadow-tenant preview branch: `read_preview_menu()` → static `renderShadowPreview` for bots / non-orderable SPA preview for humans (`ssr.ts:20-45`).
- The same `src/client` tree also ships `widget.js` (+integrity file), `embed-helper.js`, and the PWA `sw.ts` — all bundled by `apps/api/build-client.js` into `public/dist/`.

Net: **one storefront surface, three renderers** (React 20.8k LOC, Preact 442 LOC, vanilla hydration/widget 1.6k LOC), menu markup semantics duplicated between `ssr-renderer.ts` and `MenuPage.tsx` (1,811 LOC), with 4 of ssr-renderer's exports already dead (§4.1).

### 2.6 FE state management

No store library (no zustand/redux/react-query — verified against both web and ui package.json). Pattern: React context (`lib/CartProvider.tsx`) + hand-rolled `lib/apiClient.ts`/`publicApi.ts` + `lib/hooks.ts` + `lib/useWebSocket.ts`; cart persisted `dos_cart_<slug>` in localStorage with server drift-reconcile (`lib/cartReconcile.ts`). Hot pages carry the state load by hand: **46 `useState` in MenuManagerPage.tsx, 31 in MenuPage.tsx**.

---

## 3. Hotspots & coupling (repowise `get_risk`, verified LOC)

Repo git health: **111 hotspot files, churn trend increasing, bus factor 1 on 100% of 1,168 indexed files** (sole maintainer). Health: avg 7.85/10, but the five files below are where the work actually happens.

| File | LOC | Hotspot | Health | 90d churn | What it is / why it's big |
|---|---|---|---|---|---|
| `apps/api/src/routes/orders.ts` | 980 | 98.5% | **1.0/10** | +1,149/−189 | One default-export plugin holding the whole order vertical: `POST /orders` (phone-keyed rate limit → Zod parse → preflight → venue-open → **pricing** → idempotency hash → `insertOrderWithItems` → dispatch → payments hooks) plus status transitions, courier reads, signals. Biomarkers: untested_hotspot **critical**, nested_complexity, change_entropy critical. `test_gap=true` on a money path. |
| `apps/api/src/server.ts` | 890 | 99.5% | **1.0/10** | +1,470/−490 | Composition root: every provider constructed in `main()`; hooks, CORS, static, WS, workers, shutdown. `main` = large_method + complex_conditional (critical). Fix-heavy pattern: it changes because *anything* changes. |
| `apps/web/…/MenuManagerPage.tsx` | 1,405 | 99.7% | **1.0/10** | +1,901/−915 | Owner menu god-component: products+categories+modifiers+allergens+import+undo/redo+media in one file, 46 `useState`, `handleSaveProduct` complex_method. 16 dependents. |
| `packages/ui/src/lib/i18n.ts` (+catalog) | 114 + 4,343 | 99.9% | 6.35 | +3,829/−546 | Every feature adds keys → 24 dependents, co_change_scatter high, prior_defect critical. Catalog is a single 4.3k-line file (deliberate SSoT with parity gate — the churn is inherent, the single-file shape is the choice). |
| `apps/api/src/routes/spa-proxy.ts` | 885 | 99.6% | 1.45 | +1,475/−659 | Misnamed brain-file (`spaProxyRoutes` = brain_method critical): 4 near-duplicate owner-JWT helpers (`getLocationId`/`isValidOwnerToken`/`getOwnerUserId`/`getOwnerContext` — same membership SQL pasted 3×), `/images/*` + `/media/*` serving with traversal guards, owner settings/branding CRUD, uploads. |

**Coupling web (co-change, repowise):** `orders.ts ↔ server.ts` (12.3), `orders.ts ↔ CheckoutPage/OrderStatusPage/DeliveryPage`, `spa-proxy ↔ MenuPage/CheckoutPage/BrandingPage`, `i18n.ts ↔ every hot page`. API god-files and FE god-pages co-change constantly — the contract layer (`shared-types`, 1,590 LOC) is too thin to absorb change, so edits ripple across the HTTP boundary.

Worst-health file overall: `apps/api/src/routes/courier/shifts.ts` (442 LOC, **1.0/10**, prior-defect biomarker).

---

## 4. Tech-debt inventory

### 4.1 Dead code (repowise `get_dead_code`)

- **369 findings, ~2,370 deletable LOC** total; 89 safe-to-delete.
- 18 **high-confidence** unused exports (190 LOC), notably: `lib/money.ts` `roundHalfUp`+`toMinorUnit` (dead *money* helpers), `plugins/auth.ts` `requireRole`+`softVerifyAuth` (dead *authz* helpers — dangerous to leave: they look load-bearing), 4 dead `ssr-renderer.ts` component exports (`ProductCard`/`OgMetaTags`/`MenuSection`/`HreflangLinks` — fossils of the Preact storefront), `routes/orders.ts` `mapItemRow`.
- **9 zombie root dirs** (~1,630 LOC, no importers): `audit/`, `audit-sentinel/`, `analytics/`, `eval-layer/`, `load/`, `metric-core/`, `temp/`, plus low-cohesion `spikes/`. 173 unreachable files repo-wide.

### 4.2 Duplication & inconsistency

| Duplication | Evidence |
|---|---|
| Channel constants copy-mirrored API↔worker | `apps/worker/src/handlers.ts:5-7` |
| Owner-JWT + membership re-check ×4 in one file | `routes/spa-proxy.ts` helper quartet |
| Two i18n systems (FE catalog vs server notification locales) | `packages/ui/src/lib/i18n-catalog.ts` vs `apps/api/src/notifications/locales.ts` + `bot-strings.ts:2` |
| Menu rendering ×2 (Preact SSR vs React SPA), cart logic ×2 (`src/client/cart/store.ts` vs `apps/web/src/lib/CartProvider.tsx`) | §2.5 |
| Registry split: `QUEUE_NAMES` in shared-types, `BUS_CHANNELS` in api | `apps/api/src/lib/registry.ts:1,52` |
| Order-timeout mutation as raw SQL in worker, outside the domain state machine | `apps/worker/src/handlers.ts:24-30` vs `packages/domain` `assertTransition` |
| **193 `request: any`/`reply: any`** handler signatures in `routes/` despite the global Zod type provider — contracts enforced by manual `.parse()` convention, not the compiler | grep count, e.g. `routes/orders.ts` handlers |

### 4.3 Script & test sprawl

- **70 root `package.json` scripts**: 17 `test:*`, 19 `verify:*`, 6 `guardrail:*`, 4 `backup:*`. Test entries are **stage-numbered history, not features**: `test:phase5-step3 = tsx apps/api/tests/test-stage33.ts`; the combined gate is literally named `test:phase2+phase3+phase4+phase5-step0`. 26 `test-stage*/test-phase*` tsx scripts require a live server + `.env` (see `tests/test-stage30.ts` `serverAvailable()` probe) — they are integration harnesses masquerading as unit scripts, and there is **no single `pnpm test` that runs everything**.
- Dead script: `dev:ui = npx serve src/screens` — `/root/dowiz/src` does not exist.
- `apps/api/tests`: 162 files (good coverage breadth, discoverability poor); `e2e/`: 174 specs / 27.7k LOC.

### 4.4 Harness & repo hygiene footprint

- `.claude/` = **1.4 GB** (worktrees 1.4G, skills 17M, + commands/agents/hooks); `docs/` = 18 MB / 468 md files; `.agents/` 4.1M. The meta-system (loops, councils, ledger, skills) is an order of magnitude larger by file count than any product package — valuable (see §6) but co-located and unfenced.
- Repo root litter: ~15 loose screenshots (`demo-*.png`, `landing-*.png`…), one-off audit JSONs/md reports, non-workspace dirs (`agent/`, `loops/`, `eval/`, `graphify-out/`, `proposed-*`, `secrets/`) at top level.
- **Untracked in-flight code**: `packages/ui/src/voice/` + `apps/web/src/lib/voice/` = 21 files / 2,146 LOC not yet committed (voice FE mount), alongside modified `i18n-catalog.ts`, `confirmation-gate.ts`.
- Counter-signal worth noting: only **7 TODO/FIXME/HACK markers** in all product TS — debt is tracked in the ledger, not smeared inline. That is discipline, not neglect.

---

## 5. Structural pain points — ranked top 10

| # | Pain | Evidence | Why it hurts |
|---|---|---|---|
| 1 | **`server.ts` composition-root god-file** | 890 LOC, `main()` critical biomarkers, 99.5% hotspot, fix-heavy, co-changes with everything (§3) | Every cross-cutting change (new provider, hook, route group) edits the same untested file — highest-frequency merge/regression point in the repo. |
| 2 | **`orders.ts` money-path monolith with `test_gap=true`** | 980 LOC, health 1.0, untested_hotspot **critical**, +1,149 LOC/90d (§3) | The single most business-critical vertical (cash orders) concentrates pricing+dispatch+payments+status in one function with no direct unit net — 🔴 red-line code changed weekly. |
| 3 | **Triple-renderer storefront** | React SPA + Preact SSR (442) + vanilla hydration/widget (1,583); dead Preact exports already fossilizing (§2.5, §4.1) | One surface, three implementations to keep visually/semantically in sync; SSR drift is invisible until a crawler or widget breaks. |
| 4 | **`spa-proxy.ts` brain-file with duplicated auth** | 885 LOC, brain_method critical, 4 copy-paste JWT helpers with membership SQL ×3 (§3) | Authorization logic duplicated inside a misnamed file = the next auth fix will miss one copy; already a prior-defect zone. |
| 5 | **Dual data-access discipline (withTenant vs 174 raw `db.query`)** | tenant.ts:3-21 vs grep count; only 30 FORCE-RLS migrations; B3 NOBYPASSRLS open (§2.3) | Tenancy enforced two different ways means every new route re-decides its isolation model; RLS is a backstop only where the pool role and FORCE both cooperate. |
| 6 | **1,405-LOC / 46-useState MenuManagerPage** | §3; 99.7% hotspot, +1,901 LOC/90d, complex `handleSaveProduct` | The most-edited owner surface has no state architecture — each feature multiplies interacting useStates; fix-heavy churn proves it. |
| 7 | **Split worker topology + domain bypass** | Jobs consumed in both `apps/api` (in-process) and `apps/worker`; timeout cancel is raw SQL outside `assertTransition` (§2.4, §4.2) | Unclear placement rule for new jobs; a state transition that skips the state machine invites an invalid-status bug the machine exists to prevent. |
| 8 | **Test/verify script sprawl, stage-numbered** | 70 root scripts; `test:phase2+phase3+phase4+phase5-step0`; 26 live-env tsx harnesses; no aggregate `pnpm test` (§4.3) | Nobody (human or agent) can answer "did everything pass?" with one command; historical naming hides what a failing script actually protects. |
| 9 | **Typed-route erosion (193 `any` handlers)** | grep; global Zod type provider configured in server.ts yet routes opt out (§4.2) | Contract safety rests on remembering manual `.parse()`; response shapes are unchecked at compile time — the compiler is disarmed exactly at the API boundary. |
| 10 | **Repo hygiene: zombie dirs, dead exports, root litter, 1.4G harness residue** | 9 zombie dirs ~1,630 LOC; 2,370 dead LOC; screenshots/one-off reports at root; `.claude/worktrees` 1.4G (§4.1, §4.4) | For a bus-factor-1 codebase heading to open source, every fossil is onboarding noise and (per ADR-020 goal) a pre-publication liability. |

**Cross-cutting structural risk (not a code fix):** bus factor = 1 on 100% of files, churn trend increasing — the strongest argument for whatever the other lanes propose being *simplifying*, not additive.

---

## 6. What is GOOD and must be preserved

These are load-bearing, proven-in-production decisions. The rebuild must not regress any of them.
Items marked 🔴 are **high-blast-radius — change only with council + proof** (money/pricing, RLS, auth/JWT, `packages/db/migrations/`).

| Keep | Evidence |
|---|---|
| 🔴 **Integer minor-unit money; server-authoritative pricing** | `lib/money.ts:1-9` throws on non-integer subtotal, "RED LINE" comment inline; `lib/order-pricing.ts` `computeOrderPricing` + `resolveDeliveryFee` server-mirror; FE never prices. |
| 🔴 **RLS with FORCE + `withTenant` set_config pattern** | `packages/platform/src/auth/tenant.ts:3-21`; 30 migrations with `FORCE ROW LEVEL SECURITY`; adversarial RLS test (`test:phase5-rls-adversarial`). The *pattern* is right — pain point #5 is about finishing its rollout, not replacing it. |
| 🔴 **RS256-only JWT, kid rotation, live membership re-check per request** | `auth/jwt.ts:106-110` rejects non-RS256; ADR-0004 revocation ≤ next request, not token TTL (`plugins/auth.ts:147`). |
| 🔴 **Forward-only migrations + boot schema-guard + release_command** | 157 migrations; `lib/schema-guard.ts` FATAL on behind-schema; bundled migrate-runner (`scripts/build-apps.ts:69-83`). |
| **Zod parse-at-boundary everywhere** | env (`packages/config`), route inputs (`CreateOrderInput.parse`, orders.ts), global validator/serializer compilers (server.ts), `shared-types` as the contract package. |
| **Uniform error envelope (ADR-0010)** | `lib/api-error.ts` + `reply.sendError` decorator + `verify:error-contract` gate. |
| **Order status state machine in `packages/domain`** | `assertTransition` — small, correct, the seed of a real domain layer. |
| **Order idempotency + canonical request hash** | `lib/order-canonical.ts` `buildRequestHash`, `lib/order-persistence.ts` `insertOrderWithItems`. |
| **WS tri-state authz at subscribe AND fan-out, with eviction** | ADR-0013; `websocket.ts:27-160` single predicate shared by both gates "so they cannot drift". |
| **Ports-and-adapters seam in `packages/platform`** | `QueueProvider`/`MessageBus`/`StorageProvider` interfaces — pg-boss for durable jobs, Redis for fan-out, R2/local-fs storage are all swappable behind small ports. |
| **Feature flags default-off; dark deploys** | 10 `*_ENABLED` flags default `'false'` in `packages/config`; crypto/payments/media shipped dark. |
| **Deterministic guardrail/ledger ratchet** | 75-row `docs/regressions/REGRESSION-LEDGER.md`, 6 `guardrail:*` scripts, `tools/eslint-plugin-local` (393 LOC), husky pre-commit lint→typecheck→build, `compliance:gate`, i18n parity gate. This is the system that keeps bus-factor-1 survivable. |
| **E2E net against real deployed staging** | 174 Playwright specs / 27.7k LOC; Mandatory Proof Rule culture. |
| **Fail-closed privacy primitives** | e.g. `notifications/render.ts:10-19` `coarsenAddress` (fail-closed PII coarsening), pii-cipher/mask, anonymizer + GDPR workers with provenance tests. |
| **Low inline-debt discipline** | 7 TODO/FIXME in 145k LOC — debt lives in the ledger, decisions in ADRs (`docs/`), not in drive-by comments. |

---

*Lane 1 of 5 — feeds lanes 2–5 and the final synthesis. No proposals here by design.*
