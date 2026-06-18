# DeliveryOS (dowiz) — Deep Project Check

**Date:** 2026-06-18
**Branch:** chore/agentic-tooling-integration
**Method:** Repowise code-intelligence (health/overview/dead-code/risk) + 4 parallel deep-analysis agents (architecture, tests/CI, deps/build/deploy, data-layer/SQL/RLS) + prior security review + live dogfood of dowiz.fly.dev.
**Verification note:** Findings from sub-agents cite `file:line` and were spot-checked. Items I personally verified are marked ✅. Where a claim needs live-DB confirmation it is marked ⚠️.

---

## 1. Executive summary

DeliveryOS is a large, feature-complete multi-tenant delivery platform (Fastify API + React SPA + Postgres + pg-boss workers, pnpm monorepo). It is **broad and functional but carries heavy structural and process debt**, concentrated in a handful of god-files and in the gap between "works in prod" and "reproducible/guarded."

**Code-health KPIs (Repowise, index fresh):**

| Metric | Value |
|--------|-------|
| Files / NLOC | 1,263 / 134,136 |
| Average health | **7.89 / 10 — "warning" band** |
| Health distribution | 59.2% healthy · 31.6% warning · **9.2% (32 files) "alert"** |
| Hotspot health | 7.27 |
| Open findings | **2,616** |
| Bus factor | **1 — a single author owns 100% of indexed files** |
| Churn trend | **increasing** |
| Worst files | `courier/shifts.ts` (1.0), `orders.ts` (CCN **140**), `server.ts` (CCN **111**), `spa-proxy.ts` (CCN **121**), `MenuManagerPage.tsx` (905 NLOC), `CheckoutPage.tsx` (1.05) |

**Top 6 risks (cross-domain):**
1. **Security:** four unauthenticated dev endpoints minting JWTs + hardcoded dev creds (FIXED in working tree, not yet deployed).
2. **Data layer:** RLS "regime mismatch" — courier-family tables (keyed on `app.current_tenant`) written through `withTenant` (which only sets `app.user_id`); plus open-transaction early-returns leaking locks.
3. **Process:** CI runs **no unit or e2e tests on PRs**; e2e runs only *post-deploy against production*. The "Mandatory Proof Rule" is cultural, not enforced.
4. **Architecture:** `server.ts` re-defines auth/dev routes inline, *duplicating and diverging from* the extracted modules; `i18n.ts` is 2,700 lines (98% data).
5. **Build/deploy:** Dockerfile runs as root, silently clobbers `sw.js`/`manifest.json`, and `npm install`s unpinned native deps; pg-boss v10 vs v12 split across workspaces.
6. **Org:** bus factor 1 — total knowledge concentration in one owner.

---

## 2. Security (prior review — fixes in working tree)

Full detail in the earlier security review. Status of the items fixed this session (all ✅ verified, unit-tested, typecheck-clean — **not yet deployed**):

| ID | Issue | Status |
|----|-------|--------|
| C1 | `/dev/*` + `/api/dev/*` mint JWTs with no auth | Fixed — `DEV_AUTH_SECRET` fail-closed guard |
| H1 | Hardcoded dev-login creds (`test@/empty@dowiz.com`) | Fixed — gated by `DEV_AUTH_SECRET` |
| H2+H4 | Courier access tokens ignore session revocation + stale membership | Fixed — live `courier_sessions` + `courier_locations` check |
| H3 | Courier `/me/history` leaked plaintext customer names | Fixed — masked |
| H5 | `/auth/refresh` inferred role from nullable `google_sub` | Fixed — explicit owner role |
| M1 | Cross-tenant order-transition IDOR | Fixed — `withTenant` + location-scoped |

Crypto (AES-256-GCM PII cipher), JWT verification (RS256, alg/kid pinned), refresh-token rotation/reuse detection, and reveal-contact authz were reviewed and found **sound**. `plugins/auth.ts` carries a **prior-defect biomarker (5 bug-fixes/180d)** — the single highest-churn security file; treat all edits there as high-risk.

---

## 3. Data layer — SQL / RLS / transactions (HIGH)

The most dangerous *latent* defects live here, at the seam between two incompatible tenancy regimes.

**Two RLS regimes (essential context):**
- **Regime A — `app.user_id`** (membership policies): `orders`, `order_items`, `customers`, `locations`, … Set by `withTenant()`.
- **Regime B — `app.current_tenant`** (FORCE RLS): all `courier_*`, `settlement_*`. Set by manual `set_config`.
- `withTenant()` sets **only** `app.user_id` — wrapping a Regime-B table in it gives **no valid tenant context**.

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| D1 | ⚠️ Med | `courier_payouts.paid_at` referenced (SELECT + `UPDATE…SET paid_at=now()`) but **no committed migration defines it**. Per commit `f446539` the column was added to the **live DB out-of-band**; the migration is governance-blocked. → prod works, but a **fresh DB rebuild breaks owner settlements**. | `owner/settlements.ts:30,64,179`; no migration |
| D2 | High | **Regime mismatch:** courier-family writes wrapped in `withTenant` (sets `app.user_id`, not `app.current_tenant`) → policy `::uuid` cast on unset GUC denies/throws. | `owner/courier-invites.ts:42-57`; `orders.ts` dispatch ~L831; `dashboard.ts` |
| D3 | High | **Open transaction returned to pool on early `return`** — 3 handlers `BEGIN` then bare-`return` after `SELECT … FOR UPDATE` with no `ROLLBACK`; lock-holding client handed to next borrower. | `dashboard.ts` assign-courier/pickup/deliver (L218/340/409) |
| D4 | High | **Nested `BEGIN/COMMIT` inside a `withTenant` callback** — inner COMMIT closes the outer txn early; outer COMMIT then runs with no open txn. | `owner/courier-invites.ts:43-64,104-122`; `owner/menu-import.ts:177-410` |
| D5 | High | **Operational role grant gap:** `deliveryos_operational_user` is created `NOBYPASSRLS` with **SELECT-only** grants — zero INSERT/UPDATE/DELETE anywhere in migrations — yet `createOperationalPool()` forbids `postgres` and the hot path writes heavily. The real write-capable role is unmanaged/out-of-band. | `1790000000015_operational-pool-role.ts`; `packages/db/src/index.ts:28-35` |
| D6 | Med | `set_config(…, is_local=true)` **without an enclosing `BEGIN`** → GUC discarded in autocommit, query runs with no tenant context. | `owner/couriers.ts:143`; `courier/assignments.ts:98`; `courier/settlements.ts:28` |
| D7 | Med | `verify-rls.ts` only exercises Regime A (sets `app.user_id`); Regime-B courier tables "pass" by denying-on-unset-GUC — a blind spot exactly where the bugs are. | `packages/db/scripts/verify-rls.ts:39-48,71` |
| D8 | Med | Advisory lock released with wrong key arg → session lock never released; future backups of that type silently skipped. | `workers/backup/index.ts:209` |
| D9 | Low | 81 non-idempotent `ADD COLUMN` (no `IF NOT EXISTS`) — partial-apply hazard. | verify-migrations output |

**Cleared (no drift):** `courier_invites.created_by` (renamed correctly to `created_by_owner_id`), `delivery_pin_lat/lng` (never existed; `delivery_lat/lng` are correct). Migration ordering is append-only and passes `verify-migrations`. Hot-path indexes (orders listing, courier dispatch) are present and adequate.

---

## 4. Architecture & tech debt (HIGH)

| # | Sev | Finding | Evidence |
|---|-----|---------|----------|
| A1 | High | **Triplicated/divergent dev+auth handlers** — `/api/dev/mock-auth`, `/dev/create-assignment`, `/auth/local/login` exist BOTH as extracted modules AND re-defined inline in `server.ts`, with drifted logic; Fastify last-registration-wins is silent. `localAuthRoutes` is imported but never registered (dead). | `server.ts:592-821`; `routes/dev/mock-auth.ts`; `routes/auth/local.ts` |
| A2 | High | **`server.ts` = 878-line god-bootstrap** (CCN 111) mixing 8+ responsibilities: hooks, pool/bus/queue init, ~15 worker boots, error handler, ~60 registrations, 4 inline handlers, SPA fallback, shutdown. | `apps/api/src/server.ts` |
| A3 | High | **`orders.ts` POST = 660-line handler** (CCN **140**, untested hotspot) — pricing/tax/tier/modifier-validation/OTP/throttle/idempotency all inline with ~15 ROLLBACK exits; pure logic not extracted/injectable. | `routes/orders.ts:56-717` |
| A4 | High | **`i18n.ts` = 2,694 lines, ~98% a single message literal** (3 locale blocks × ~880 keys). #1 churn hotspot precisely because every copy edit touches a 2,700-line file. Extract to JSON. | `packages/ui/src/lib/i18n.ts` |
| A5 | High | **`spa-proxy.ts` (CCN 121)** is misnamed — it's ~15 full owner endpoints that **re-implement JWT verification locally** instead of using the shared auth plugin. Fold into `routes/owner/*`. | `routes/spa-proxy.ts:15-113` |
| A6 | Med | **Tenant/transaction boilerplate duplicated** — manual `set_config('app.current_tenant')` in 9 files; manual `BEGIN`/catch-ROLLBACK/finally-release 40+ times. A `withTransaction()` wrapper collapses all. | repo-wide |
| A7 | Med | `MenuManagerPage.tsx` (984 lines, 32 `useState`) and `MenuPage.tsx`/`CheckoutPage.tsx` mix concerns; 3 client pages bypass the typed `apiClient` with raw `fetch()` (no schema validation). | `apps/web/src/pages/**` |
| A8 | Low | Dead/leftover: `/api/debug/test-notification` (no auth) in bootstrap; `workers/reconciliation.ts` (337 lines, never wired); 19 high-confidence unused exports (~222 lines) incl. `plugins/auth.ts:requireRole,softVerifyAuth`. | see dead-code ledger |

---

## 5. Tests, CI & reliability (HIGH)

| # | Sev | Finding |
|---|-----|---------|
| T1 | High | **CI runs NO unit tests and NO e2e on PRs.** `ci.yml` validate = build/typecheck/lint/verify only. The 46+ `node:test` files are never run by CI and can rot. |
| T2 | High | **E2E runs only POST-deploy, against live production** (`dowiz.fly.dev`). The gate fires *after* bad code is live — smoke alarm, not guardrail. A second workflow (`fly-deploy.yml`) deploys with **no validation**. |
| T3 | High | **Critical paths unit-untested:** `PATCH /orders/:id/status` state-machine + synchronous courier auto-assignment, settlements (money owed — `owner/settlements.ts`, `courier/settlements.ts`), `owner/gdpr.ts`, `customer/otp.ts`, `customer/orders.ts`. Coverage exists only via flaky live e2e. |
| T4 | High | **E2E fragility:** `retries:0`, `workers:1`, serial; 60 specs depend on `/dev/mock-auth` + `DEV_AUTH_SECRET`; broken `e2e/helpers/auth.ts:getDevToken` hits a non-existent route and silently falls back to `'dev_test_token'` (false greens). |
| T5 | Med | **No aggregate test runner** — no root `pnpm test`, no `node --test` glob; ~half the `.test.ts` files are unwired to any script and DB-coupled, so the "Mandatory Proof Rule" is **manual-only**. No coverage tooling. |
| T6 | Med | Suite sprawl: ~50 overlapping `flow-*` specs + checked-in scratch specs (`debug-order`, `simple-test`, `quick-order-test`). |

110 test files total (~24 node:test units + ~84 Playwright e2e). Frontend has just **2** unit specs.

---

## 6. Dependencies, build & deploy (HIGH)

| # | Sev | Finding |
|---|-----|---------|
| B1 | High | **pg-boss major split:** `apps/api` pins `^12.18.2`, `packages/platform` (the actual queue code) pins `^10.1.5`; lockfile resolves **both**. Two majors of the job queue in one deploy. |
| B2 | High | **Dockerfile, 3 issues in ~8 lines:** runs as **root** (no `USER`); `COPY apps/api/public` + `apps/web/dist` into the same dir **silently clobbers `sw.js`/`manifest.json`**; `RUN npm install argon2 sharp @aws-sdk/*` **unpinned, no lockfile** → non-reproducible. |
| B3 | High | **51 of 329 source files (15.5%) carry `@ts-nocheck`** — incl. `server.ts`, `shutdown.ts`, all `client/*`, all `workers/*`, `security/headers.ts`. The `tsc` typecheck gate is partly hollow. |
| B4 | High | **`mem0ai` supply-chain bloat** — one helper file (`lib/memory.ts`) drags in Anthropic/Azure/Google/LangChain/Mistral/Qdrant/Supabase/better-sqlite3/openai. Largest contributor to the 948 MB store. Isolate or remove. |
| B5 | Med | **`.env.example` missing required keys** (`VAPID_PUBLIC_KEY/PRIVATE_KEY`, `APP_BASE_URL`) → a fresh clone fails `loadEnv()` at boot. |
| B6 | Low | `corepack prepare pnpm@latest` contradicts pinned `pnpm@9.4.0`; `maplibre-gl` stray in root deps; Baileys `7.0.0-rc13` (pre-release WhatsApp lib) in prod. |

**Good:** no committed live secrets (scans were false positives in detector scripts); `.gitignore` covers `.env`/`dist`; lockfile committed; multi-stage Docker; real `/health` check wired to fly.toml; env schema itself is thorough with prod-safe defaults.

---

## 7. Runtime / UX (dogfood of dowiz.fly.dev)

| # | Sev | Finding |
|---|-----|---------|
| U1 | Med | **`/admin` renders the full owner dashboard shell to unauthenticated users** (no route guard). APIs correctly 401 (no data leak), but the protected UI is exposed and shows a broken empty state instead of redirecting to `/login`. |
| U2 | Med | **Double HTML-entity encoding** of the venue name in SSR head — "Dubin & Sushi" → `Dubin &amp;amp; Sushi` across `<title>`/OG/Twitter/description (10×). Visible in browser tab + social cards; SEO/AEO hit. |
| U3 | Low | Login error "Login failed." is hardcoded English on the Albanian UI. |
| U4 | Low | City "Durrës" mis-cased to "DurrëS" in SSR meta. |
| U5 | Info | No JSON-LD on the public menu page despite a `jsonld-builder` + SEO/AEO focus. |

Console clean on login/menu pages; no PII leaks observed (APIs enforce 401).

---

## 8. Cross-cutting themes

1. **"Works in prod" ≠ "reproducible/guarded."** The strongest defects (out-of-band `paid_at`, operational-role grants, no CI tests, post-deploy-only e2e, missing `.env.example` keys) all share this shape: the running system is patched, but a clean rebuild or a regression would not be caught.
2. **God-files concentrate churn, complexity, and untested risk.** `orders.ts`, `server.ts`, `spa-proxy.ts`, `i18n.ts`, `MenuManagerPage.tsx` are simultaneously the biggest, most-changed, highest-CCN, and least-tested files. Decomposing these 5 addresses architecture, testability, and churn at once.
3. **Two RLS regimes with one helper.** `withTenant` only fits Regime A; every courier/settlement path that uses it is silently unprotected. One `withTenant`/`withTransaction` that takes the regime explicitly would remove an entire bug class.
4. **Bus factor 1.** 100% single-owner across 870 indexed files is the top organizational risk — no review redundancy, no knowledge backup.

---

## 9. Prioritized remediation roadmap

**P0 — correctness/security (do first):**
- Deploy the security fixes (C1/H1–H5/M1) + set `DEV_AUTH_SECRET` on fly.dev & CI.
- Commit a migration for `courier_payouts.paid_at` (D1) so rebuilds don't break settlements.
- Add `ROLLBACK` to the 3 `dashboard.ts` early-return handlers (D3); remove nested `BEGIN/COMMIT` in `courier-invites.ts`/`menu-import.ts` (D4).
- Fix the RLS regime mismatch on courier-family writes (D2); reconcile the operational-role write grants (D5).

**P1 — process (stop the bleeding):**
- Add a CI job that runs unit tests on PRs (provision a Postgres service); wire an aggregate `pnpm test` (T1, T5).
- Gate deploy on e2e instead of running it only post-deploy; remove the unvalidated `fly-deploy.yml` path (T2).
- Fix `getDevToken` false-green fallback (T4).

**P2 — architecture (compounding payoff):**
- Delete the inline `server.ts` handlers; register the extracted modules only (A1).
- Extract `i18n.ts` locales to JSON (A4) — kills the #1 churn hotspot.
- Fold `spa-proxy.ts` into `routes/owner/*` behind the shared auth plugin (A5).
- Extract `orders.ts` pricing/OTP/preflight into injectable, unit-tested functions (A3, T3).

**P3 — hygiene:**
- Dockerfile: add non-root `USER`, resolve the asset COPY collision, pin runtime native deps (B2).
- Burn down `@ts-nocheck` on server/workers (B3); isolate/remove `mem0ai` (B4); align pg-boss (B1).
- Regenerate `.env.example` from the Zod schema (B5).
- Add the `/admin` route guard (U1); fix SSR double-encoding (U2).

---

## 10. Methodology / tools used

- **Repowise MCP:** `get_health` (KPIs + worst-file biomarkers), `get_overview` (topology, git/knowledge map), `get_dead_code` (369 findings), `get_risk`/`get_context` for orientation.
- **Parallel analysis agents:** architecture/tech-debt, tests/CI/reliability, deps/build/deploy, data-layer/SQL/RLS — each grounded in real source with `file:line`.
- **Skills:** OWASP-security lens (prior review), agent-browser `dogfood` (live UX).
- **Honesty:** ⚠️ items (e.g. D1 `paid_at`, D5 operational role) need live-DB confirmation; the dogfood tested the *deployed* build, so the working-tree security fixes were not exercised there.
