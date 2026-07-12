import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_positions (
      id bigserial PRIMARY KEY,
      courier_id uuid NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      shift_id uuid REFERENCES courier_shifts(id),
      lat numeric(9,6) NOT NULL,
      lng numeric(9,6) NOT NULL,
      accuracy_meters int,
      source text NOT NULL DEFAULT 'gps' CHECK (source IN ('gps', 'manual', 'network')),
      recorded_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX courier_positions_courier_recent_idx ON courier_positions(courier_id, recorded_at DESC);
    CREATE INDEX courier_positions_location_recent_idx ON courier_positions(location_id, recorded_at DESC);

    ALTER TABLE courier_positions ENABLE ROW LEVEL SECURITY;
    
    CREATE POLICY isolate_courier_positions ON courier_positions
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_positions;
  `);
}
