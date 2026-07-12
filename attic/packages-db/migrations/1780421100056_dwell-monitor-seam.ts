import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Add acknowledge and resolution columns
    ALTER TABLE location_alerts
      ADD COLUMN IF NOT EXISTS acknowledged_at timestamptz,
      ADD COLUMN IF NOT EXISTS acknowledged_by_owner_id uuid REFERENCES users(id),
      ADD COLUMN IF NOT EXISTS resolution_reason text;

    -- De-dup partial unique index: one active alert per (order_id, kind)
    CREATE UNIQUE INDEX IF NOT EXISTS location_alerts_dedup_idx
      ON location_alerts(order_id, kind)
      WHERE resolved_at IS NULL;

    -- Composite index for dwell-monitor queries
    CREATE INDEX IF NOT EXISTS location_alerts_active_idx
      ON location_alerts(location_id, kind)
      WHERE resolved_at IS NULL AND kind LIKE 'dwell_%';

    COMMENT ON COLUMN location_alerts.acknowledged_at IS 'Set when owner acknowledges the alert. Cancels pending escalation jobs.';
    COMMENT ON COLUMN location_alerts.acknowledged_by_owner_id IS 'Owner who acknowledged.';
    COMMENT ON COLUMN location_alerts.resolution_reason IS 'e.g. lifecycle_confirmed, owner_acknowledge, auto_resolve';
    COMMENT ON INDEX location_alerts_dedup_idx IS 'Prevents duplicate active alerts per (order_id, kind)';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP INDEX IF EXISTS location_alerts_dedup_idx;
    DROP INDEX IF EXISTS location_alerts_active_idx;
    ALTER TABLE location_alerts
      DROP COLUMN IF EXISTS acknowledged_at,
      DROP COLUMN IF EXISTS acknowledged_by_owner_id,
      DROP COLUMN IF EXISTS resolution_reason;
  `);
}
