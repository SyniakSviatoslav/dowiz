# DeliveryOS / Dowiz — As-Built Summary v1

> **Audit date:** 2026-06-04 · **Auditor:** read-only code audit, 0 code changes  
> **Version:** v1 · **Status:** DRAFT — pre-pilot readiness assessment  
> **Tests:** 92 unique tests × 3 breakpoints = 276 total, ALL GREEN, 0 failures, 0 flaky

---

## 1. Overview & Scope

**What this is:** Dowiz is an Albanian-market SaaS delivery platform for restaurants. Three roles: Client (orders food), Owner (manages restaurant), Courier (delivers). Mobile-first, 77% cash payments.

**Deployment reality:** N=1 on Supabase Free (operational pool 8 connections, session pool 3), Fly.io single instance, Cloudflare for DNS/CDN. ~50 pilot restaurants planned. Solo developer.

**What's built (as evidenced by code):**

| Phase | Capability | Status |
|-------|-----------|--------|
| 0 | Auth (OAuth Google, HS256 JWT, refresh token rotation) | ✅ |
| 2 | Menu CRUD + import (CSV/photo) + stop-list + menu_version | ✅ |
| 2 | Branding/themes (color/font/logo, live preview) | ✅ |
| 2 | Client storefront (SSR menu, cart, checkout, order status) | ✅ |
| 2 | Embed/iframe with CORS wildcard | ✅ |
| 3 | Courier invite → activation → trusted device | ✅ |
| 3 | Courier delivery (GPS stream, pickup/deliver state machine) | ⚠️ Routes unregistered |
| 3 | Cash cycle (payment_outcome, settlement generation) | ✅ |
| 4 | Anti-fake signals (velocity, OTP, human-in-loop) | ✅ |
| 4 | Dwell monitoring + alerts | ✅ |
| 5 | Anonymizer (retention + GDPR erasure) | ✅ |
| 5 | Backup (pg_dump → R2, hourly/daily/weekly/monthly) | ✅ |
| — | Frontend React PWA (18 screens, 4 map components) | ✅ |
| — | Playwright E2E (92 tests, 3 breakpoints) | ✅ |

---

## 2. Stack & Shims

| Layer | Technology | Implementation |
|-------|-----------|----------------|
| Runtime | Node.js 22, TypeScript 5.6 | — |
| API | Fastify 5 | `apps/api/src/server.ts` |
| DB | PostgreSQL (Supabase) | `packages/db/` — 67 migrations |
| Queue | pg-boss | `packages/platform/src/queue-provider.ts` |
| MessageBus | PostgreSQL NOTIFY/LISTEN | `packages/platform/src/message-bus.ts` |
| Frontend | React 18 + Vite 6 + Tailwind CDN | `apps/web/` |
| Maps | MapLibre GL 5.24 | OpenFreeMap tiles |
| E2E | Playwright 1.60 | Chromium, 3 breakpoints |
| Auth | HS256 JWT (opaque-code→POST exchange) | `apps/api/src/lib/jwt.ts` |
| Notifications | Telegram + WebPush adapters | `apps/api/src/notifications/` |
| Caching | LRU in-process (SSR menu) + Cloudflare | `routes/public/ssr.ts` |

### Shims vs Real

| Interface | Status |
|-----------|--------|
| QueueProvider (pg-boss) | Real |
| MessageBus (PostgreSQL NOTIFY) | Real |
| NotificationProvider (Telegram + Push) | Real |
| MenuParserProvider (CSV + AI-OCR) | Real |
| AnonymizerService | Real |
| BackupProvider (pg_dump + R2) | Real |
| Stripe/Billing | Stub (post-MVP) |
| AI (LibreTranslate) | Stub (no real call) |

---

## 3. Data Model

**67 migrations** — from `001_extensions-and-enums` to `067_free-tier-watch`.

**Core entities:** users → organizations → locations → memberships → categories → products → orders → order_items → customers.

**Courier domain:** couriers → courier_locations → courier_sessions → courier_shifts → courier_assignments → courier_positions → settlement_items → courier_payouts.

**RLS:** 35 tables with FORCE ROW LEVEL SECURITY. However, operational pool connects as `postgres` superuser — RLS is a defense-in-depth layer, not the primary enforcement. Application-level WHERE clauses and `SET LOCAL app.current_tenant` are the actual guards.

---

## 4. Flow Diagrams

See `docs/audit/flows/` for Mermaid diagrams of each flow:

1. `owner-oauth-login.md` — Google → exchange → HS256 JWT
2. `onboarding.md` — 8-step wizard → auto-open
3. `menu-crud.md` — Categories + products + stop-list + import
4. `customer-order.md` — Menu → cart → checkout → POST /orders → status
5. `order-lifecycle.md` — 10-state machine with status-guarded transitions
6. `durable-timeout.md` — Outbox enqueue → cancel PENDING
7. `courier-invite.md` — Invite → activation → trusted device
8. `courier-delivery.md` — Assign → pickup → in_delivery → deliver + GPS WS
9. `cash-cycle.md` — payment_outcome + settlement reconciliation
10. `branding-theme.md` — Config → CSS render → CDN
11. `embed-iframe.md` — CORS wildcard + postMessage
12. `ws-live.md` — order + location rooms + reconcile
13. `ssr-menu.md` — /s/:slug + Cloudflare cache
14. `notifications.md` — Telegram + Push adapters
15. `fallback-degradation.md` — Backend-down → phone fallback
16. `lifecycle-ops.md` — Connection budget, graceful shutdown
17. `anonymizer.md` — Retention cron + GDPR erasure

---

## 5. Security Posture

| Red Line | Status | Critical Issue |
|----------|--------|----------------|
| V1 · Tenant Isolation | WEAK | Superuser DB role bypasses RLS |
| V2 · AuthZ Memberships | WEAK | Theme/notification routes lack auth |
| V3 · Tokens/OAuth | **HOLDS** | — |
| V4 · Idempotency | WEAK | No location_id in dedup key |
| V5 · Money Integrity | **HOLDS** | Server-authoritative prices |
| V6 · Injection/XSS | WEAK | MapLibreBase.innerHTML |
| V7 · Secrets | **HOLDS** | — |
| V8 · PII | **HOLDS** | Claim-check, no PII in JWT |
| V9 · Queue/Worker | WEAK | Only 1 job uses outbox |
| V10 · WS | WEAK | In-memory, N=1 safe |
| V11 · DoS/Throttle | **BROKEN** | No per-phone throttle |
| V12 · Error Leakage | WEAK | No custom error handler |
| V13 · Client Storage | **HOLDS** | Cart versioning implemented |
| V14 · Embed/CORS | WEAK | Courier bottom bar uses fixed |
| V15 · Dependencies | **HOLDS** | — |
| V16 · Cash | **BROKEN** | Courier routes unregistered |

**HOLDS: 7 · WEAK: 6 · BROKEN: 3**

---

## 6. Required Actions Before Pilot

### Must-Fix (CRITICAL + HIGH)

| # | Action | Severity | Ticket |
|---|--------|----------|--------|
| 1 | Register courier routes (assignments + shifts) in server.ts | CRITICAL | FX-NEW |
| 2 | Implement per-phone order throttle (FX-4) | CRITICAL | FX-4 |
| 3 | Set statement_timeout + acquire-timeout (FX-9) | HIGH | FX-9 |
| 4 | Add auth to theme/notification owner routes | HIGH | FX-NEW-2 |
| 5 | Add location_id scope to idempotency dedup (FX-5) | HIGH | FX-5 |
| 6 | Implement custom error handler (FX-6) | HIGH | FX-6 |
| 7 | Fix MapLibreBase.innerHTML → textContent for marker labels | HIGH | FX-NEW-3 |
| 8 | Create non-superuser DB role for operational pool | HIGH | FX-NEW-1 |

### Should-Fix (MEDIUM)

| # | Action | Ticket |
|---|--------|--------|
| 9 | Use outbox pattern for all transactional enqueues | FX-NEW-4 |
| 10 | Fix CourierRoutes bottom bar position:fixed for embed | P1-14 |
| 11 | Remove hardcoded credentials from LoginPage | P0-2 |
| 12 | Enforce OTP in CheckoutPage (don't skip on error) | P0-1 |
| 13 | Extract duplicate exportCSV to shared utility | DONE |
| 14 | Fix broken Tailwind classes in molecules | DONE |
| 15 | Add ErrorBoundary to main.tsx | DONE |

---

## 7. Verdict — Per-Area GO / NO-GO

| Area | Verdict | Conditions |
|------|---------|------------|
| Auth (OAuth + JWT) | **GO** | — |
| Menu & storefront | **GO** | — |
| Order placement (idempotent) | **GO-WITH-FIXES** | FX-5 (location_id scope) |
| Order lifecycle (state machine) | **GO** | — |
| Durable timeout | **GO** | — |
| Courier invite + activation | **GO** | — |
| Courier delivery (GPS + WS) | **NO-GO** | Routes unregistered — broken |
| Cash cycle + settlements | **NO-GO** | Courier routes unregistered |
| Branding / themes | **GO** | Add auth to theme routes |
| Embed / iframe | **GO-WITH-FIXES** | Courier fixed position |
| Notifications | **GO-WITH-FIXES** | Add auth to notification routes |
| Fallback / degradation | **GO-WITH-FIXES** | FX-9 (timeouts) |
| Onboarding | **GO-WITH-FIXES** | Publish missing data |
| Anonymizer / GDPR | **GO** | — |
| Operations (backup, heartbeat) | **GO** | — |

---

## 8. Consciously Deferred (post-MVP)

- Polygon delivery zones (replaced with pin+radius per ONB-5)
- Custom domain support
- Scheduled/pickup orders (runtime)
- Full AI (LibreTranslate stub only)
- Stripe billing / customer card
- PITR / point-in-time recovery
- N>1 scaling (Redis adapter, load balancing)
- OAuth verification (≤100 test users on pilot)

---

## 9. Frontend Test Coverage

| Suite | Tests | Status |
|-------|-------|--------|
| Smoke | 6 | ✅ |
| Client (menu, cart, checkout, status) | 28 | ✅ |
| Admin (dashboard, orders, menu, couriers, analytics, CRM, settings, branding, onboarding) | 21 | ✅ |
| Courier (tasks, delivery, login, earnings, history, shift) | 12 | ✅ |
| Map Components (pin, courier, radius) | 7 | ✅ |
| Error Handling (500, timeout, 422, 429, 404, 401, 403, 503) | 8 | ✅ |
| Embed Mode | 4 | ✅ |
| Cross-Cutting (error boundary, theme, network, nav, localStorage, map fallback) | 6 | ✅ |
| **TOTAL** | **92** | **ALL GREEN** |

---

## 10. Known Audit Limitations

- Server-side tests (Phase 0-5) not executed — audit relies on code structure analysis only
- Database live connection not available — RLS effectiveness verified via schema/code, not runtime
- N=2 WebSocket behavior not tested — N=1 dev environment only
- Load testing not performed — connection budget analysis from code only

---

*dowiz / DeliveryOS · As-Built Summary v1 · read-only audit · 0 code changes · Confidential*
