# DeliveryOS — API, DB, WS, Worker Inventory (Phase A)

> **Generated:** 2026-06-04 · **Updated:** 2026-06-14 (structural sweep) · **Source:** Code — not docs · **Scope:** read-only audit  
> **Evidence key:** every entry traceable to `file:line` in the repository

---

## A1. API Route Inventory (121 registered + 8 unregistered)

### Registered Routes

| # | Method | Path | Handler | Auth | Schema | Rate Limit | Idempotent |
|---|--------|------|---------|------|--------|------------|------------|
| 1 | GET | `/health` | `routes/health.ts:45` | None | — | 100/min | No |
| 2 | GET | `/auth/google` | `routes/auth.ts:15` | None | — | 10/min | No |
| 3 | GET | `/auth/google/callback` | `routes/auth.ts:41` | None | query strict | 10/min | No |
| 4 | POST | `/auth/exchange` | `routes/auth.ts:150` | None | code uuid strict | 10/min | No |
| 5 | POST | `/auth/refresh` | `routes/auth.ts:164` | None | refresh_token strict | 5/min | Reuse-revoke |
| 6 | POST | `/auth/courier/activate` | `routes/auth.ts:225` | None | code+phone+name strict | 5/min | No |
| 7 | POST | `/orders` | `routes/orders.ts:48` | Optional | CreateOrderInput | 10/min | **Yes** |
| 8 | GET | `/orders/:id` | `routes/orders.ts:591` | Optional | — | 100/min | No |
| 9 | PATCH | `/orders/:id/status` | `routes/orders.ts:667` | Owner JWT | StatusUpdateInput | 100/min | No |
| 10 | GET | `/public/locations/:id/menu` | `routes/public/menu.ts:8` | None | — | 100/min | No |
| 11 | GET | `/s/:slug` (SSR) | `routes/public/ssr.ts:19` | None | — | 100/min | LRU 60s |
| 12-21 | — | *(public SEO, PWA, telemetry, theme, fallback)* | — | None | — | 100/min | No |
| 22-116 | — | *(owner CRUD, courier, signals, alerts, GDPR, settlements, etc.)* | — | Various | Zod strict | Various | Partial |
| 117-121 | — | *(admin backups, fallback)* | — | **No auth** | — | 100/min | No |

### Unregistered Routes (Dead Code)

| Method | Path | File | Status |
|--------|------|------|--------|
| GET | `/api/courier/me/assignments` | `routes/courier/assignments.ts:14` | **NOT imported** in server.ts |
| POST | `/api/courier/assignments/:id/accept` | `routes/courier/assignments.ts:37` | **NOT imported** |
| POST | `/api/courier/assignments/:id/reject` | `routes/courier/assignments.ts:94` | **NOT imported** |
| POST | `/api/courier/assignments/:id/picked-up` | `routes/courier/assignments.ts:155` | **NOT imported** |
| POST | `/api/courier/assignments/:id/delivered` | `routes/courier/assignments.ts:203` | **NOT imported** |
| POST | `/api/courier/assignments/:id/cancel` | `routes/courier/assignments.ts:272` | **NOT imported** |
| POST | `/api/courier/shifts/transition` | `routes/courier/shifts.ts:12` | **NOT imported** |
| POST | `/api/courier/shifts/ping` | `routes/courier/shifts.ts:145` | **NOT imported** |

### Auth Gaps
- **Owner theme routes** (#45-47): no auth — `routes/owner/themes.ts` has no preValidation hook
- **Admin backup routes** (#117-121): no auth at all
- **Notification routes** (#48-51): no explicit auth

---

## A2. WebSocket Architecture

| Component | Detail |
|-----------|--------|
| Server | `ws` library on Fastify HTTP server, path `/ws` |
| Auth | Message-based: client sends `{type:'auth', token}` as first message; 5s timeout |
| Room RBAC | Customer: only `order:{ownOrderId}`. Owner/Courier: any `location:` rooms |
| Client hook | `apps/web/src/lib/useWebSocket.ts` — React hook; token from localStorage |

### WS Rooms
| Room | Consumers | Publishers |
|------|-----------|------------|
| `order:{id}` | Customer status, Courier delivery | `orders.ts`, `courier-events.ts`, `dashboard.ts`, `signals.ts` |
| `location:{id}:dashboard` | Owner dashboard | `orders.ts`, `dashboard.ts`, `dwell-monitor.ts`, `signal-raiser.ts`, etc. |
| `location:{id}:couriers` | Owner courier map | `courier-events.ts` (GPS + assignment) |

### MessageBus Channels (20+ with no subscriber)
Many channels publish with no subscribe handler — fire-and-forget or log-only. No N-safety concern at N=1.

---

## A3. Worker/pg-boss Inventory (22 jobs)

| # | Job Name | Claim-Check | Transactional | Schedule |
|---|----------|-------------|---------------|----------|
| 1 | `courier.dispatch` | Partial (FOR UPDATE) | In-tx (dispatch_queue) | On-demand |
| 2 | `order.timeout` | **Yes** (orderId only) | **Yes** (outbox) | On-demand |
| 3-14 | *(cron: settlement, dwell, backup, etc.)* | N/A | N/A | Various cron |
| 15-18 | *(signal, liveness, free-tier, velocity)* | Mixed | Varies | Various |
| 19-22 | *(notify.dispatch, notify.customer_status, etc.)* | **Yes** | Separate | Triggered |

**Only `order.timeout` uses true outbox pattern** (enqueue inside same tx as order INSERT via `db:` option).

**`velocity.flush` carries full event data** — only job that doesn't use claim-check.

---

## A4. Database — 67 Migrations, ~50 Tables

### RLS Coverage
- **RLS + FORCE**: 35 tables (all core tenant data)
- **RLS enabled, no FORCE**: 9 tables
- **No RLS**: 6 tables (`users`, `couriers`, `courier_sessions`, `customer_contact_reveals`, `upload_audit`, `free_tier_snapshots`)

### Critical Finding
**Operational pool connects as `postgres` superuser** (port 6543), which **bypasses RLS by default**. RLS only activates when `SET LOCAL app.user_id` or `SET LOCAL app.current_tenant` is explicitly issued per-request. App-level WHERE clauses in some routes are the actual guard.

### Tenant Isolation Patterns
- **Pattern A**: `app.user_id` → `app_member_location_ids()` (core owner tables)
- **Pattern B**: `SET LOCAL app.current_tenant` (courier tables)
- **Pattern C**: JWT claims directly (notification tables)

---

## A5. Platform Shims/Shves

| Interface | Implementation | Status |
|-----------|---------------|--------|
| `QueueProvider` | pg-boss | **Real** |
| `MessageBus` | PostgreSQL NOTIFY/LISTEN | **Real** |
| `NotificationProvider` | Telegram + WebPush adapters | **Real** |
| `MenuParserProvider` | CSV + AI-OCR | **Real** |
| `AnonymizerService` | Retention + GDPR erase | **Real** |
| `BackupProvider` | pg_dump + R2 upload | **Real** |
| Stripe/Billing | — | **Stub** (post-MVP) |
| AI (LibreTranslate) | External API | **STUB** (no real call) |

---

## A6. Frontend Screens

| Screen | Route | API Calls | Map | Status |
|--------|-------|-----------|-----|--------|
| Menu | `/s/:slug` | `GET /public/menu/:slug` | Hero section | ✅ |
| Cart | *(drawer)* | localStorage (CartProvider) | — | ✅ |
| Checkout | `/s/:slug/checkout` | `POST /customer/otp/send`, `POST /customer/orders` | MapWithPin | ✅ |
| Order Status | `/s/:slug/order/:id` | `GET /customer/orders/:id/status` | CourierLiveMap | ✅ |
| Dashboard | `/admin` | `GET /owner/orders` | CourierLiveMap | ✅ |
| Menu Manager | `/admin/menu` | `GET/POST /owner/menu/categories`, `GET/POST /owner/menu/products` | — | ✅ |
| Branding | `/admin/branding` | `GET/PUT /owner/brand` | — | ✅ |
| Couriers | `/admin/couriers` | `GET /owner/couriers` | — | ✅ |
| Analytics | `/admin/analytics` | `GET /owner/analytics` | — | ✅ |
| CRM | `/admin/crm` | `GET /owner/customers`, `POST /owner/customers/:id/reveal-contact` | — | ✅ |
| Settings | `/admin/settings` | `GET/PATCH /owner/settings` | — | ✅ |
| Onboarding | `/admin/onboarding` | `POST /owner/onboarding` | MapWithRadius | ✅ |
| Tasks | `/courier` | `GET /courier/me/assignments` | — | ✅ |
| Delivery | `/courier/delivery/:id` | `GET/PATCH /courier/orders/:id`, WS location | CourierLiveMap | ✅ |
| Login | `/courier/login` | `POST /courier/auth/login` | — | ✅ |
| Earnings | `/courier/earnings` | `GET /courier/me/payouts` | — | ✅ |
| History | `/courier/history` | `GET /courier/me/history` | — | ✅ |
| Shift | `/courier/shift` | `GET/POST /courier/me/shifts` | — | ✅ |

---

## A7. Discovered Flows

1. Owner OAuth Login (Google → exchange → HS256 JWT)
2. Onboarding (8-step wizard → auto-open)
3. Menu Management (CRUD + import + stop-list)
4. Customer Order (menu → cart → checkout → POST /orders → status)
5. Order Lifecycle (10 states, status-guarded transitions)
6. Durable Timeout (outbox → cancel PENDING)
7. Courier Invite → Activation → Trusted Device
8. Courier Delivery (assign → pickup → deliver + GPS stream)
9. Cash Cycle (payment_outcome + settlement reconciliation)
10. Branding/Theme (config → CSS render)
11. Embed/iframe (CORS wildcard + postMessage)
12. WS Live (order + location rooms, reconcile-on-reconnect)
13. SSR Menu (`/s/:slug` + Cloudflare cache)
14. Notifications (Telegram + Push)
15. Fallback/Degradation (backend-down → phone fallback)
16. Lifecycle Ops (connection budget, graceful shutdown, restarts)
17. Anonymizer (retention cron + GDPR erasure)
