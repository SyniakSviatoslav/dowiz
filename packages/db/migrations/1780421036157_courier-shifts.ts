import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_shifts (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      courier_id uuid NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      status text NOT NULL DEFAULT 'offline' CHECK (status IN ('offline', 'available', 'on_delivery')),
      started_at timestamptz NOT NULL DEFAULT now(),
      ended_at timestamptz,
      last_heartbeat_at timestamptz
    );
    CREATE INDEX courier_shifts_dispatch_idx ON courier_shifts(location_id, status) WHERE status = 'available';
    
    ALTER TABLE courier_shifts ENABLE ROW LEVEL SECURITY;
    CREATE POLICY isolate_courier_shifts ON courier_shifts
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_shifts;
  `);
}
