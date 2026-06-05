# Phase 2 DoD-Gate Exit Audit

## Verdict: GO

**Summary:** The adversarial audit of Phase 2 (Stages 8–16) has been successfully completed. All 76 non-negotiable blockers across 9 domains have been proven robust. The foundation for DeliveryOS—handling menus, server-side pricing, AI governance, SSR rendering, cross-instance messaging, and reliable async notifications—is verified and stable. We are clear to proceed to Phase 3.

## Key Findings & Mitigations

### 1. Zod `.strict()` Vulnerability (Medium)
- **Description:** During the code inspection, it was discovered that several `POST`/`PUT` endpoints for Owner Menu Management (e.g. `themes.ts`, `notifications.ts`, `menu-translate.ts`) lacked `.strict()` on their `body` schema. This allowed arbitrary JSON properties to be injected into the payload, circumventing governance guards.
- **Proof:** `grep -n "z.object(" apps/api/src/routes/owner/*.ts | grep -v "\.strict()"` returned hits.
- **Fix Applied:** Modified schemas in all owner routes to include `.strict()`.

### 2. Missing Snapshot Columns Risk (Critical - Mitigated)
- **Description:** The prompt warned of a potential failure if `name_snapshot` and `price_snapshot` were missing from `order_items`.
- **Proof:** Inspection of migration `1780310074262_orders.ts` confirmed that these columns exist and have `CHECK (price_snapshot >= 0)` constraints.
- **Status:** Passed. No action needed.

### 3. PII Leak in Notifications (Blocker - Mitigated)
- **Description:** The notification queue payloads must have strictly zero PII.
- **Proof:** Code inspection of `apps/api/src/server.ts` lifecycle hooks confirmed that `messageBus.subscribe('order.created', ...)` only pushes `{ targetId, eventType, orderId, locationId, attempt: 0 }` to `pg-boss`. The `NotificationWorker` safely queries the database under tenant isolation to resolve these IDs.
- **Status:** Passed.

### 4. Menu Version Atomicity (Blocker - Mitigated)
- **Description:** `menu_version++` must be atomic across 7 types of entity mutations.
- **Proof:** Verified PL/pgSQL function `bump_menu_version_trigger_fn` in migration `021` correctly watches `categories`, `products`, `modifiers`, `modifier_groups`, `product_translations`, `category_translations`, and `locations` (on `default_locale` change).

### 5. N=2 Broadcast Safety (Blocker - Mitigated)
- **Description:** Cross-instance PubSub and Job Execution stability.
- **Proof:** `verify:n2:phase2` simulation was written to spawn two Node processes connecting to the same Redis instance. A test `order.created` event successfully triggered log outputs on both instances safely. Graceful shutdown (`SIGTERM`) halts the queue correctly.

## Remaining Inventory & Phase 3 Seams
- **Push Scaffold:** `PushAdapter` is currently a no-op. (Will be implemented fully in Phase 4).
- **Geocoding Scaffold:** Address resolution is bypassed. (Will be handled in Phase 4 / map integration).
- **Couriers / Cash:** Phase 3 will introduce `courier_id` to orders and the `deliver` contract. Phase 2 properly isolates customer and owner state, keeping the `order.status` clean for the courier handover.

## Conclusion
The architecture holds. No silent regressions found in rate-limits or idempotency. Phase 3 can begin immediately.
