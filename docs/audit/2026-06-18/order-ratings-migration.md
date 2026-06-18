# order_ratings migration (for manual approval)

`packages/db/migrations/` is governance-protected. Create
`packages/db/migrations/1790000000025_order-ratings.ts` with the content below,
then commit + deploy (migrations run on deploy) to activate the feedback system.
The API/UI/display code that uses this table is already implemented and shipped;
it stays inert (no rating to submit/show) until this table exists.

```ts
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE order_ratings (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL UNIQUE REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL,
      courier_id uuid,
      customer_id uuid,
      rating int NOT NULL CHECK (rating BETWEEN 1 AND 5),
      feedback text CHECK (feedback IS NULL OR char_length(feedback) <= 1000),
      created_at timestamptz NOT NULL DEFAULT now(),
      updated_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX order_ratings_courier_idx ON order_ratings(courier_id) WHERE courier_id IS NOT NULL;
    CREATE INDEX order_ratings_location_idx ON order_ratings(location_id);
  `);

  // RLS mirrors orders/customers: members (owners + couriers) read via
  // tenant_isolation; the customer submit writes through the operational pool
  // with an explicit ownership check in the route.
  pgm.sql(`
    ALTER TABLE order_ratings ENABLE ROW LEVEL SECURITY;
    ALTER TABLE order_ratings FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON order_ratings
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS order_ratings;`);
}
```

## What this unlocks (code already implemented)
- `POST /api/customer/orders/:id/rating` — customer submits a 1–5 star rating + optional feedback (DELIVERED-only, 24h window, UPSERT exactly-once).
- `GET /api/customer/orders/:id/status` now returns `rating`, `feedback`, `canRate`.
- Client `OrderStatusPage` shows a star/feedback block when DELIVERED, or the submitted rating.
- Courier history + admin couriers (avg rating) read real ratings instead of hardcoded null/0.
