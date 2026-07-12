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
