import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Alter location_alerts to add actual columns and drop placeholder_data
  pgm.sql(`
    ALTER TABLE location_alerts
      DROP COLUMN IF EXISTS placeholder_data,
      ADD COLUMN IF NOT EXISTS order_id uuid REFERENCES orders(id) ON DELETE CASCADE,
      ADD COLUMN IF NOT EXISTS kind text NOT NULL,
      ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'pending',
      ADD COLUMN IF NOT EXISTS attempts jsonb NOT NULL DEFAULT '[]'::jsonb,
      ADD COLUMN IF NOT EXISTS last_error text,
      ADD COLUMN IF NOT EXISTS resolved_at timestamptz,
      ADD COLUMN IF NOT EXISTS escalation_level int NOT NULL DEFAULT 0;
  `);

  pgm.sql(`CREATE INDEX IF NOT EXISTS location_alerts_order_idx ON location_alerts(order_id);`);
  pgm.sql(`CREATE INDEX IF NOT EXISTS location_alerts_kind_idx ON location_alerts(kind);`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_alerts
      DROP COLUMN IF EXISTS order_id,
      DROP COLUMN IF EXISTS kind,
      DROP COLUMN IF EXISTS status,
      DROP COLUMN IF EXISTS attempts,
      DROP COLUMN IF EXISTS last_error,
      DROP COLUMN IF EXISTS resolved_at,
      DROP COLUMN IF EXISTS escalation_level,
      ADD COLUMN IF NOT EXISTS placeholder_data text;
  `);
}
