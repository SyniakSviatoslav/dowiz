# Fix Plan — Menu Categories + Notifications + Tests

> Generated: 2026-06-15 · Skills: RLS-tenant-isolation, improve-codebase-architecture, request-refactor-plan

---

## Re-analysis with Skills Context

### RLS Contract Violations (deliveryos-rls-tenant-isolation)

| # | Violation | Location | Severity |
|---|---|---|---|
| V1 | `spa-proxy.ts` queries `categories`/`products` without `SET LOCAL app.tenant_id` — relies on raw `WHERE location_id = $1` filter instead of RLS | `spa-proxy.ts:108-118` | 🟠 RLS bypass |
| V2 | Missing `withTenant()` wrapper — `app.user_id` not set, `app_member_location_ids()` returns empty | `spa-proxy.ts:107-119` | 🟠 Works only because pool role may have `BYPASSRLS` |
| V3 | Dynamic SQL via string interpolation on user-controlled keys — violates parameterized-only rule | `owner/categories.ts:136-144` | 🔴 SQL injection risk |

### Architecture Friction (improve-codebase-architecture)

| # | Issue | Location | Depth |
|---|---|---|---|
| A1 | Response shape contract lives in 3 places (Zod schema, manual interface, SQL columns) — all disagree | `shared-types` + `spa-proxy.ts` + `MenuManagerPage.tsx` | Shallow: 3 representations for 1 concept |
| A2 | spa-proxy.ts handles BOTH route dispatch AND DB queries — no separation between HTTP concerns and data access | `spa-proxy.ts` (677 lines) | Shallow: monolith with no seams |
| A3 | `apiClient` schema.parse() silently swallows errors — Zod failures become empty states without feedback | `apiClient.ts` + `MenuManagerPage.tsx:128` | Shallow: error handling is pass-through |

---

## Fix Plan (7 steps, each independently verifiable)

### Step 1: Fix CategoryResponse Zod schema — align snake_case API shape

**Problem:** `CategoryResponse` schema (`shared-types/src/contracts/owner/categories.ts:16-23`) expects camelCase + `.strict()` — rejects the snake_case the API actually returns. `apiClient.schema.parse()` always throws → UI shows "Failed to load menu".

**Changes:**
- `packages/shared-types/src/contracts/owner/categories.ts` — change `CategoryResponse` to match snake_case API output:
  - `sortOrder` → `sort_order`
  - `productCount` → `product_count`
  - Remove `imageKey`, `createdAt` (not returned by API)
  - Remove `.strict()` (allow future additions)
- `apps/web/src/pages/admin/MenuManagerPage.tsx` — remove manual `Category` interface (now inferred from Zod schema)
- Verify: `pnpm typecheck` passes, `apiClient` no longer throws on category fetch

### Step 2: Add UNIQUE constraint to categories(location_id, name)

**Problem:** No unique constraint allows duplicate category names. Seed-data endpoint checks existence but race conditions can create duplicates.

**Changes:**
- New migration `1790000000019_add_categories_unique.ts`:
  - Clean duplicate categories before adding constraint (keep the one with most products)
  - `CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_loc_name ON categories(location_id, name)`
- `packages/db/scripts/seed.ts` + `apps/api/src/server.ts` (seed-data) — add `ON CONFLICT (location_id, name) DO NOTHING` to category inserts
- Verify: `pnpm migrate:up` succeeds, duplicate names prevented

### Step 3: Add RLS tenant context to spa-proxy queries

**Problem:** spa-proxy queries bypass RLS — no `SET LOCAL app.tenant_id` before queries, no `withTenant()` wrapper.

**Changes:**
- `apps/api/src/routes/spa-proxy.ts` — wrap ALL db.query() calls that touch tenant-scoped tables (categories, products, orders, settings, etc.) with tenant context:
  - Before each query: `SELECT set_config('app.current_tenant', $1, true)` with the resolved `locId`
  - Or use the existing `withTenant()` helper if available
- Alternative: add an `onRequest` hook to spa-proxy that sets tenant context once for the request lifecycle
- Verify: RLS check script passes, queries still return correct data

### Step 4: Fix dynamic SQL in categories PATCH

**Problem:** `owner/categories.ts:136-144` builds SQL by interpolating user-controlled object keys.

**Changes:**
- `apps/api/src/routes/owner/categories.ts` — replace dynamic `SET clauses = Object.entries(updates).map(...)` with explicit named columns:
  - Only allow `name` and `sort_order` as settable fields
  - Static SQL: `UPDATE categories SET name = COALESCE($1, name), sort_order = COALESCE($2, sort_order) WHERE id = $3 AND location_id = $4`
- Verify: `pnpm typecheck` passes, PATCH still updates name + sort_order correctly

### Step 5: Investigate + fix notification delivery

**Problem:** Telegram notifications not received for new orders. Need systematic debugging.

**Changes:**
- Trace notification chain end-to-end:
  1. `POST /api/orders` → does it publish `BUS_CHANNELS.ORDER_CREATED`?
  2. `server.ts` → is there a subscriber that creates `notify.dispatch` jobs?
  3. Notification worker → does `notify.dispatch` queue handler build correct Telegram data?
  4. Telegram API → does `notify.telegram.send` worker call the Bot API?
- Check `notification_outbox_audit` table for audit trail of dropped/sent messages
- Verify: place a test order, confirm audit entry with status `delivered`

### Step 6: Fix 4 skipped lifecycle tests

**Problem:** Flows 3, 4, 6, 17 skipped because order is rejected early — tests hardcode sequential state machine transitions that don't allow branch points.

**Changes:**
- `e2e/tests/flow-core-lifecycles.spec.ts`:
  - After Flow 2 (reject), the order status is 'REJECTED' — flows 3, 4, 5, 6 check for PENDING/CONFIRMED and skip
  - **Option A:** Create a SECOND order in between that stays PENDING for the assign/no-show/cancel flows
  - **Option B:** Remove the reject step (Flow 2) and use confirm instead, then branch later
  - **Option C:** Move reject to AFTER the assign/no-show/cancel flows complete
- Preferred: **Option C** — reorder flows so assign → no-show → customer-cancel happen BEFORE reject. This lets all flows run on the same order
- Verify: all 32 tests run without skips, 0 failures

### Step 7: Seed-data cleanup — idempotent category seeding

**Problem:** Running `POST /api/dev/seed-data` multiple times leaves stale categories with 0 products.

**Changes:**
- `apps/api/src/server.ts` (seed-data endpoint) — after creating/updating categories, CLEAN UP categories with 0 products that aren't in the seeded list:
  ```sql
  DELETE FROM categories
  WHERE location_id = $1
    AND id NOT IN ($2, $3, ...)  -- only seeded category IDs
    AND (SELECT count(*) FROM products WHERE category_id = categories.id) = 0
  ```
- Verify: seed-data endpoint run twice produces identical results, no category bloat

---

## Verification Gate

After each step, run:
```bash
pnpm typecheck
pnpm verify:rls
pnpm verify:no-raw-status-update
```

After all fixes:
```bash
VITE_BASE_URL=https://dowiz.fly.dev npx playwright test e2e/tests/flow-core-lifecycles.spec.ts --project=mobile --reporter=list
# Expect: 32/32 passed, 0 skipped, 0 failed
```

After notification fix:
```bash
# Place test order via API, check notification_outbox_audit
curl -X POST https://dowiz.fly.dev/api/orders -H 'Content-Type: application/json' -d '{...}'
# Verify audit entry status = 'delivered'
```
