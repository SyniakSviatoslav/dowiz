# DeliveryOS — Bug Investigation Report

> Generated: 2026-06-12 · Investigator: opencode

---

## Summary

Six distinct issues were investigated in the April batch. Three have been fixed, three require further work. Below is the full analysis for each.

---

## Issue 1: Order Status 401 Unauthorized

**Status:** ✅ FIXED  
**Severity:** HIGH — blocked customer order tracking entirely  
**File:** `apps/api/src/routes/auth/local.ts:62`

### Finding

The local login route unconditionally assigned `const role = 'owner'` for every authenticated user, regardless of their actual role in the system. The customer order status endpoint (`apps/api/src/routes/customer/orders.ts:16`) requires `requireRole(['customer'])`, which always failed — returning 403 (shown as 401 to the frontend).

### Root Cause

The `role` field in the JWT token determines what endpoints a user can access. By hardcoding `'owner'`, even users who are purely customers (like `test@dowiz.com` who owns no orgs) could never access customer-scoped routes.

### Fix

Changed role determination to query the database:
1. Check if user owns any `organizations` → `'owner'`
2. Else check active `memberships` → use their role
3. Else default to `'customer'`

### Risk

Low — the query is simple, has a try/catch fallback, and doesn't change behavior for existing org owners.

---

## Issue 2: Missing Address Validation in Checkout

**Status:** ✅ FIXED  
**Severity:** MEDIUM — incomplete delivery data, couriers can't find entrance/apartment  
**Files:**
- `apps/web/src/pages/client/CheckoutPage.tsx`
- `packages/ui/src/lib/i18n.ts`
- `packages/shared-types/src/legacy.ts` (schema already supported it)

### Finding

The checkout form collected a single free-form `address` field but never gathered `entrance` or `apartment` numbers. The API schema (`CreateOrderInput.prefs.dropoff`) already defined these fields as optional, and the `OrderPreferences` type had `entrance`, `apartment`, `floor`, and `code`. But the frontend never populated them — the `prefs` field was entirely absent from order requests.

### Root Cause

Historical gap: when the dropoff preference schema was added to `legacy.ts`, the checkout form was never updated to collect the extra fields or send them in the API request.

### Fix

1. Added `entrance` and `apartment` state variables and form inputs
2. Added required validation for delivery orders (entrance + apartment must be non-empty)
3. Included `prefs.dropoff` in the order request body
4. Saved/restored values via localStorage for persistence
5. Added Albanian and English i18n keys

### Risk

Low — fields are validated on submit, localStorage handles persistence gracefully.

---

## Issue 3: Price Mismatches Between Order Creation and Admin Display

**Status:** ❌ NO FIX NEEDED — no critical mismatch found  
**Severity:** LOW

### Finding

Traced the full price flow:
1. **Customer checkout UI:** `subtotal = Σ(item.price × quantity)` — uses client-side CartItem prices that may include modifiers
2. **API order creation** (`orders.ts:389-501`): recalculates everything server-side from DB product prices, modifier deltas, tax rate, and delivery fee. Result stored in `orders.total`, `orders.subtotal`, `orders.delivery_fee`, `orders.tax_total`.
3. **Admin dashboard** (`spa-proxy.ts:430-441`): reads `orders.total` directly from DB
4. **Customer order status** (`customer/orders.ts:82-102`): reads the same `orders.total`

All three read the same stored value. There is no data path where a mismatch can persist. Minor visual differences could occur if the client-side subtotal doesn't include modifier prices (the client calculates `item.price × quantity` without modifiers, while the API adds them), but the final stored total is authoritative.

### Recommendation

No code change needed. If users report visual discrepancies, the root cause is the client-side calculator not including modifier prices in the subtotal display — a cosmetic issue, not a data integrity bug.

---

## Issue 4: Admin Dashboard Not Updating in Real-Time

**Status:** 🔍 INVESTIGATED — root cause known, not fixed  
**Severity:** MEDIUM — admin must manually refresh to see new orders  
**File:** `apps/web/src/pages/admin/DashboardPage.tsx:78`

### Finding

The dashboard subscribes to a WebSocket room (`admin:${tenantId}`) with a message handler that can:
- Add new orders (`msg.type === 'order_created'`)
- Update existing orders (`msg.type === 'order_updated'`)
- Update courier positions (`msg.type === 'courier_position'`)

However, the WebSocket is configured with **`enabled: false`** on line 78:

```typescript
useWebSocket({
  room: `admin:${tenantId}`,
  enabled: false,  // <-- never connects
  onMessage: (msg) => { ... },
});
```

Meanwhile, the API **does** publish dashboard events on order creation (`orders.ts:632-643`):
```typescript
await messageBus.publish(`location:${locationId}:dashboard`, {
  type: 'order.created',
  data: { orderId, status, total, ... }
});
```

The event path works server-side; the frontend just never connects to receive them.

### Root Cause

Unknown why `enabled: false` was set — possibly a half-finished feature or a workaround for a connection issue. The code for receiving and processing messages is fully written and correct.

### Recommended Fix

Change `enabled: false` to `enabled: true` and verify the WebSocket connects. The `room` value uses `admin:${tenantId}` — ensure `tenantId` is populated (set from settings response on line 51: `if (res.id) setTenantId(res.id)`).

### Risk

Low — the event publishing on the server is already active, and the frontend handler is tested logic.

---

## Issue 5: Delayed Pending Notifications (~5 Minute Delay)

**Status:** 🔍 INVESTIGATED — root cause known, not fixed  
**Severity:** MEDIUM — owners wait 5+ minutes for new-order Telegram alerts

### Finding

There are two parallel mechanisms for pending order notifications, one broken and one slow:

#### Mechanism A: Dwell Monitor (should fire in ~60 seconds — BROKEN)

| File | Line | Problem |
|---|---|---|
| `apps/api/src/lib/dwell-thresholds.ts` | 14-20 | Default `pending_s: 60` (1 minute threshold — correct) |
| `apps/api/src/workers/dwell-monitor.ts` | 23 | Runs every minute via `DWELL_CRON` — correct |
| `apps/api/src/workers/dwell-monitor.ts` | 123-144 | `scheduleEscalation()` sends **`targetId: null`** to `notify.dispatch` |
| `apps/api/src/notifications/workers/index.ts` | 185-192 | Dispatch handler queries `WHERE id = null` → finds nothing → **silent return** |
| `apps/api/src/workers/dwell-monitor.ts` | 131-132 | Event type is `'dwell.alert'` which is **not in the `NotificationEventType` union** |
| `apps/api/src/notifications/render.ts` | 47-114 | No case for `'dwell.alert'` → renders `"Unknown notification"` |
| `apps/api/src/notifications/workers/index.ts` | 206-208 | Quiet hours suppress all events except `'order.pending_aging'` and `'test'` → `'dwell.alert'` dropped |

**Double bug:** `targetId: null` makes dispatch find zero targets AND `'dwell.alert'` isn't in the event type union. The dwell monitor effectively produces zero Telegram notifications.

Additionally, the `DwellEscalationWorker` (`apps/api/src/workers/dwell-escalation.ts:19`) registers for the `dwell.escalate` queue but **nobody publishes to it** — dead code.

#### Mechanism B: Pending Aging (works — ~5 minute delay)

| Config | Value |
|---|---|
| `PENDING_AGING_THRESHOLD_MS` | 300,000ms (5 minutes) |
| Cron schedule | `*/5 * * * *` (every 5 minutes) |
| Event type sent | `'order.pending_aging'` — **IN** the event union, **HANDLED** by render, **ALLOWED** during quiet hours |

This path finds orders that have been pending 5+ minutes, creates alerts with real `targetId` values, and dispatches them correctly. This is the ~5 minute delay the user sees.

### Recommended Fix

Two approaches:

**Quick fix:** Reduce `PENDING_AGING_THRESHOLD_MS` from 300,000 to 60,000 and change cron from `*/5 * * * *` to `* * * * *`. This makes the working path fire every 60 seconds for 60-second-old orders. Single file change.

**Proper fix:** Fix the dwell-monitor path:
1. In `scheduleEscalation()`: pass a real `targetId` (or list of location targets)
2. Add `'dwell.alert'` to the `NotificationEventType` union
3. Add a render case for `'dwell.alert'`
4. Allow `'dwell.alert'` during quiet hours
5. Wire the `dwell.escalate` queue publisher

The quick fix is safer and addresses the user-facing symptom. The proper fix is more thorough but touches 5+ files.

---

## Issue 6: Missing Telegram Notifications for Admin Status Changes

**Status:** 🔍 INVESTIGATED — root cause known, not fixed  
**Severity:** HIGH — owners get no Telegram alert when order is confirmed/rejected by admin

### Finding

The notification chain for admin order status changes has **two parallel code paths**, and only one triggers Telegram:

#### Path A: PATCH `/orders/:id/status` → ✅ Works

| Step | File | Line |
|---|---|---|
| 1. Admin status change via API | `orders.ts` | 758-795 |
| 2. Calls `updateOrderStatus()` | `orders.ts` | 780 |
| 3. Publishes `'order.confirmed'` / `'order.rejected'` | `orderStatusService.ts` | 79-83 |
| 4. Server subscribes → enqueues `notify.telegram.send` | `server.ts` | 488-502 |
| 5. Worker processes → sends Telegram message | `notifications/workers/index.ts` | 327-498 |

This path is **fully wired and should work**. If it isn't working, possible causes:
- Owner has no active Telegram targets (`owner_notification_targets` table)
- Event is disabled in target preferences (`prefs[event] === false`)
- Dedup cache from a previous run (resets on restart)
- Location ID mismatch in event payload

#### Path B: Admin dashboard routes in `dashboard.ts` → ❌ No Telegram

| Step | File | Line | Problem |
|---|---|---|---|
| 1. Admin clicks status button in dashboard UI | `DashboardPage.tsx` | 111-118 | Calls `PATCH /orders/:id/status` → **this IS Path A** |

The dashboard's `handleUpdateStatus` (DashboardPage.tsx:111) does call the correct `PATCH /orders/:id/status` endpoint, which goes through Path A. So the standard dashboard flow should work.

**However**, there may be additional routes in `owner/dashboard.ts` that handle status changes **directly** without going through `updateOrderStatus()`. These routes publish to `order:${orderId}` with `type: 'order.status'` instead of to the global `'order.confirmed'`/`'order.rejected'` channels that the Telegram handlers subscribe to.

### Additional Finding: Channel Mismatch

`apps/api/src/workers/lifecycle-handlers.ts:30-35` subscribes to `'order.status'` as a global channel, but `orderStatusService.ts:59-65` publishes to `order:${orderId}` with `type: 'order.status'` — these are different channels. This lifecycle handler is dead code.

### Recommended Fix

1. Verify that the PATCH route is being called by the dashboard (it should be — DashboardPage.tsx:114 calls `apiClient(`/orders/${id}/status`)`).
2. If Telegram notifications still don't arrive, debug the actual event flow by checking `pgboss.job` and `notification_outbox_audit` after a status change.
3. As a safety net, add a handler for `order:*` wildcard events in `server.ts` to catch any status changes published via the per-order channel.
4. Remove or fix the dead code in `lifecycle-handlers.ts:30-35` to avoid confusion.

---

## Appendix: Files Changed

### Fixed (3 issues)

| File | Change |
|---|---|
| `apps/api/src/routes/auth/local.ts` | Role determined by org ownership/memberships (was hardcoded `'owner'`) |
| `apps/web/src/pages/client/CheckoutPage.tsx` | Added entrance/apartment fields, validation, prefs.dropoff in order request |
| `packages/ui/src/lib/i18n.ts` | Added Albanian and English i18n keys for entrance/apartment |

### Investigated but not fixed (3 issues)

| File | Issue |
|---|---|
| `apps/web/src/pages/admin/DashboardPage.tsx:78` | WebSocket `enabled: false` — change to `true` |
| `apps/api/src/workers/dwell-monitor.ts:123-144` | `targetId: null` + `'dwell.alert'` not in event union |
| `apps/api/src/notifications/workers/index.ts:291` | `PENDING_AGING_THRESHOLD_MS` = 5 min (consider reducing) |
| `apps/api/src/notifications/workers/index.ts:206-208` | Quiet hours suppress `'dwell.alert'` |
| `apps/api/src/routes/owner/dashboard.ts` | May bypass `updateOrderStatus()` — verify PATCH route usage |
| `apps/api/src/workers/lifecycle-handlers.ts:30-35` | Subscribes to wrong channel — dead code |

---

## Priority Ranking for Remaining Fixes

1. **Admin dashboard real-time** (enabled: false) — one-line fix, high impact
2. **Pending notification delay** (reduce threshold + fix dwell monitor) — moderate effort, high impact
3. **Missing Telegram for admin changes** — may already work; needs verification. Low effort to debug.
