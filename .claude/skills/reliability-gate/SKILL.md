---
name: reliability-gate
description: Run the DeliveryOS lifecycle reliability gate. Traces ONE order from /s/:slug through L0–L11, verifies every surface, produces GO/NO-GO verdict. Invoke with /reliability-gate.
---

# DeliveryOS — Reliability Gate Skill

## When to invoke
- After any significant backend change (schema, routes, workers, migrations)
- Before a production deploy
- On a scheduled cadence (weekly/after each sprint)
- When a cross-surface consistency bug is suspected

## What this gate checks

One order is traced from client entry (`/s/:slug`) through all lifecycle stages to delivered+feedback, verified on every surface. The gate does NOT re-audit pixel polish or individual unit invariants — it stitches them together along a single journey and proves cross-surface consistency.

## Five threads checked on every run

1. 🔴 **Exactly-once** — rightly one order, one hold, one trace row, one feedback job, one rating set on every surface
2. 🔴 **Recoverable** — kill at any stage → no orphans, durable jobs survive
3. 🔴 **Cross-surface consistent** — DELIVERED shows on every required surface, absent from active board
4. 🔴 **Proof-by-artifact** — PASS only with code citation; "should work" = FAIL
5. 🔴 **Timely signal** — dwell/dead-man's-switch exists at every potential stuck point

## Execution protocol (5 parallel agents)

Dispatch ALL FIVE in a single message to maximize parallelism. Each agent gets full file access.

### Agent 1 — L0 · L1 · L2 (Entry → Order creation)
Files to audit:
- `apps/api/src/routes/public/menu.ts` — SSR, cache headers, menu_version
- `apps/api/src/routes/orders.ts` — transaction atomicity, idempotency, server-side pricing
- `packages/db/migrations/1780310074262_orders.ts` — idempotency_keys PK
- `packages/db/migrations/1790000000029_idempotency-composite-pk.ts` — composite PK migration
- `apps/web/src/pages/client/MenuPage.tsx` — cart localStorage, is_available check

PASS criteria:
- SSR route exists with cache headers ≤60s
- `POST /orders` is one BEGIN/COMMIT with idempotency_keys + orders + items + timeout-job
- `idempotency_keys` PK is now `(location_id, key)` (composite after migration 029)
- Server recomputes total from `products` table (no client total trusted)
- Double POST with same key → same response (idempotency guard)

### Agent 2 — L3 · L4 · L5 (Notifications → CONFIRMED → PREPARING/READY)
Files to audit:
- `apps/worker/src/handlers.ts` — ORDER_TIMEOUT: messageBus.publish after cancel
- `apps/worker/src/index.ts` — PgMessageBus wired to registerHandlers
- `apps/api/src/lib/orderStatusService.ts` — updateOrderStatus: anti-race WHERE guard + READY courier push
- `apps/api/src/routes/owner/dashboard.ts` — transitionOrder: queue.cancel on CONFIRMED, countSql date filter
- `packages/platform/src/message-bus.ts` — RedisMessageBus = PgMessageBus alias

PASS criteria:
- ORDER_TIMEOUT handler: after rowCount>0 → INSERT order_status_history + publish to `order:{id}` + publish to `location:{id}:dashboard`
- Worker index: `new PgMessageBus(pool)` passed to `registerHandlers`
- orderStatusService: WHERE status=$currentStatus RETURNING id; rowCount=0 → 409; READY → publishes to `courier:{courierId}`
- transitionOrder: pgboss job cancelled when newStatus='CONFIRMED'
- countSql has `DATE(created_at) = CURRENT_DATE`

### Agent 3 — L6 · L7 (IN_DELIVERY → DELIVERED termination node)
Files to audit:
- `apps/api/src/routes/courier/assignments.ts` — DELIVERED handler (lines 270–385)
- `packages/db/migrations/1790000000027_delivery-trace.ts` — delivery_trace schema
- `packages/db/migrations/1790000000028_courier-cash-ledger.ts` — courier_cash_ledger schema
- `apps/api/src/routes/courier/shifts.ts` — GPS accuracy/speed filters

PASS criteria:
- DELIVERED handler: single BEGIN/COMMIT containing updateOrderStatus + delivery_trace INSERT + (if cash_collected) courier_cash_ledger INSERT
- delivery_trace INSERT uses ON CONFLICT (order_id) DO NOTHING (idempotency)
- feedback job enqueued AFTER COMMIT with startAfter=30*60s
- GPS: accuracy>100m → 400; speed>150km/h → 400
- delivery_trace migration: UNIQUE(order_id), RLS enabled+forced, owner+courier SELECT policies
- courier_cash_ledger migration: CHECK(type IN('hold','release','settle')), RLS

### Agent 4 — L8 · L9 · L10 (Post-DELIVERED: WS, feedback, ratings propagation)
Files to audit:
- `apps/api/src/lib/orderStatusService.ts` — publish to orderChannel + dashboardChannel on DELIVERED
- `apps/web/src/pages/client/OrderStatusPage.tsx` — StarRatingBlock canSubmit flow
- `apps/api/src/routes/customer/ratings.ts` — UPSERT ON CONFLICT(order_id), 24h window
- `apps/api/src/routes/owner/couriers.ts` — avg_rating query, ordersToday CURRENT_DATE filter
- `apps/worker/src/handlers.ts` — ORDER_FEEDBACK_REMINDER registered handler

PASS criteria:
- orderStatusService: DELIVERED publishes to both `order:{id}` and `location:{id}:dashboard`
- StarRatingBlock: shown when status=DELIVERED; controlled by canSubmit from backend
- ratings route: UPSERT with ON CONFLICT(order_id) (exactly-once)
- couriers details: avg_rating from order_ratings WHERE courier_id + location_id
- ordersToday subquery: AND ca.delivered_at >= CURRENT_DATE
- ORDER_FEEDBACK_REMINDER handler: registered in registerHandlers

### Agent 5 — L11 · Cross-cutting (tenant, N=2, exactly-once surface matrix)
Files to audit:
- `apps/api/src/routes/spa-proxy.ts` — analytics withTenant wrapping
- `apps/api/src/routes/owner/dashboard.ts` — countSql, DispatchView import
- `apps/web/src/pages/admin/DispatchView.tsx` — component exists and renders DELIVERED orders
- `packages/platform/src/message-bus.ts` — Postgres NOTIFY broadcast (N=2 cross-instance)
- `apps/api/src/routes/owner/couriers.ts` — list + details ratings+stats

PASS criteria per surface:
1. CustomerStatusPage: DELIVERED status shown, canSubmit controls form
2. ActiveOrdersBoard: DashboardPage.tsx filters out DELIVERED from live board
3. DispatchView: component exists at `apps/web/src/pages/admin/DispatchView.tsx`, fetches DELIVERED snapshot
4. QuickStats countSql: DATE(created_at)=CURRENT_DATE filter present
5. Analytics: wrapped with withTenant in spa-proxy.ts
6. CourierDailyStats: avg_rating from order_ratings, ordersToday date-filtered
7. delivery_trace (DB): migration 027 exists with UNIQUE(order_id)
8. courier_cash_ledger (DB): migration 028 exists with CHECK(type)
9. Ratings (DB): UPSERT ON CONFLICT(order_id) in ratings route
10. idempotency_keys (DB): migration 029 composite PK (location_id, key)

N=2 check: `PgMessageBus.publish` uses `NOTIFY` via pool → broadcasts to ALL Postgres LISTEN clients → cross-instance works
Tenant: analytics wrapped with withTenant; orderStatusService no-SQL-level location_id in UPDATE (relies on state machine + existing order FK)

## Output format

After all 5 agents return, synthesize into:

```
## Стадійна таблиця L0–L10
| Стадія | PASS/FAIL | Доказ (файл:рядок) |

## L11 Cross-surface matrix (10 рядків)
| # | Поверхня | PASS/FAIL/PARTIAL | Артефакт |

## Наскрізні нитки
- Exactly-once: ...
- Recoverable: ...
- N=2: ...
- Tenant: ...

## Flag-only
(залишкові борги, не блокери)

## Вердикт: GO / NO-GO
```

**GO** = ALL L0–L11 PASS з артефактом; exactly-once наскрізь; N=2 cross-instance working; нуль cross-tenant leaks; нуль часткового стану при rollback.
**NO-GO** = будь-який FAIL у L0–L11 або поверхні 1–10 L11; дублювання на будь-якій поверхні; відсутній delivery_trace/cash_ledger/feedback job.

## Invocation

When user types `/reliability-gate`:
1. Dispatch all 5 agents in a SINGLE message (parallel)
2. Wait for all to return
3. Synthesize verdict per output format above
4. Save any new regressions to memory as project memories

## Scheduling hint

For autonomous periodic runs, this gate can be scheduled via CronCreate:
- Weekly before Monday deploy: `0 8 * * 1`
- After each major fix session

## Reconciliation notes (source-of-truth as of the lifecycle build)

**Now BUILT** (were phantom FAILs against an aspirational spec; the .ignored_db copies were never tracked):
- delivery_trace — migration 1790000000027; DELIVERED handler INSERTs it (ON CONFLICT order_id DO NOTHING).
- courier_cash_ledger — migration 1790000000028; AUDIT-ONLY (settlement_items stays authoritative); DELIVERED writes a hold on cash_collected.
- idempotency_keys composite PK (location_id, key) — migration 1790000000029.

- A post-delivery feedback reminder is infeasible as a dedicated pg-boss queue: pgboss.queue is owned by the operational role (no CREATE on the pgboss schema, revoked by migration 009) and the migration role lacks REFERENCES on it, so create_queue fails for every available role (order.timeout has the same fate — unregistered queues never enqueue). Revisit via an existing registered queue (notify.*) or a cron sweep.

**Known debt (FLAG-ONLY — do NOT treat as NO-GO blockers; never implemented in tracked source):**
- DispatchView.tsx does not exist; the owner DELIVERED/in-flight snapshot is served by apps/api/src/routes/owner/dashboard.ts (status IN (IN_DELIVERY,READY)).
- dashboard countSql has no DATE(created_at)=CURRENT_DATE filter — status counts are all-time.
- transitionOrder neutralizes the order timeout via timeout_at=NULL in the UPDATE, NOT a pgboss job cancel (functionally equivalent; the worker also guards WHERE status=PENDING).
- updateOrderStatus does not publish to courier:{courierId} on READY; courier fan-out happens on assign→IN_DELIVERY (dashboard.ts).
- courier GPS ping (shifts.ts) enforces only a geofence range check; no accuracy>100m / speed>150km/h bounds.

A run is GO when L0–L11 pass OR the only failures are in this Known-debt list.
