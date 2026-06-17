import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS order_ratings (
      id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id    uuid        NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid        NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      courier_id  uuid        REFERENCES couriers(id),
      stars       smallint    NOT NULL CHECK (stars BETWEEN 1 AND 5),
      comment     text,
      created_at  timestamptz NOT NULL DEFAULT now(),
      updated_at  timestamptz NOT NULL DEFAULT now(),
      UNIQUE (order_id)
    );

    CREATE INDEX IF NOT EXISTS order_ratings_location_id_idx ON order_ratings (location_id);
    CREATE INDEX IF NOT EXISTS order_ratings_courier_id_idx  ON order_ratings (courier_id) WHERE courier_id IS NOT NULL;

    -- RLS
    ALTER TABLE order_ratings ENABLE ROW LEVEL SECURITY;

    -- Owner: read all ratings for their location(s) via membership
    CREATE POLICY owner_select_ratings ON order_ratings
      FOR SELECT
      USING (location_id IN (SELECT app_member_location_ids()));

    -- Customer: insert/update/select their own rating (via order ownership)
    CREATE POLICY customer_insert_rating ON order_ratings
      FOR INSERT
      WITH CHECK (
        order_id IN (
          SELECT id FROM orders
          WHERE customer_id = app_current_user()
        )
      );

    CREATE POLICY customer_update_rating ON order_ratings
      FOR UPDATE
      USING (
        order_id IN (
          SELECT id FROM orders
          WHERE customer_id = app_current_user()
        )
      );

    CREATE POLICY customer_select_own_rating ON order_ratings
      FOR SELECT
      USING (
        order_id IN (
          SELECT id FROM orders
          WHERE customer_id = app_current_user()
        )
      );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS order_ratings CASCADE;
  `);
}
