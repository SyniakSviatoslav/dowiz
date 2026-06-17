import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS delivery_trace (
      id             uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id       uuid        NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      courier_id     uuid        NOT NULL REFERENCES couriers(id),
      location_id    uuid        NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      trace_points   jsonb       NOT NULL DEFAULT '[]',
      started_at     timestamptz NOT NULL DEFAULT now(),
      ended_at       timestamptz,
      distance_meters numeric,
      UNIQUE (order_id)
    );

    CREATE INDEX IF NOT EXISTS delivery_trace_location_id_idx ON delivery_trace (location_id);
    CREATE INDEX IF NOT EXISTS delivery_trace_courier_id_idx  ON delivery_trace (courier_id);

    ALTER TABLE delivery_trace ENABLE ROW LEVEL SECURITY;
    ALTER TABLE delivery_trace FORCE ROW LEVEL SECURITY;

    CREATE POLICY owner_select_delivery_trace ON delivery_trace
      FOR SELECT
      USING (location_id IN (SELECT app_member_location_ids()));

    CREATE POLICY courier_select_own_trace ON delivery_trace
      FOR SELECT
      USING (courier_id = app_current_user());
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS delivery_trace CASCADE;`);
}
