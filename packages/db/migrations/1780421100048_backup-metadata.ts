import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE backup_metadata (
      id uuid PRIMARY KEY,
      type text NOT NULL CHECK (type IN ('hourly', 'daily', 'weekly', 'monthly')),
      created_at timestamptz NOT NULL DEFAULT now(),
      completed_at timestamptz,
      status text NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'completed', 'failed')),
      size_bytes bigint CHECK (size_bytes IS NULL OR size_bytes >= 0),
      duration_ms int CHECK (duration_ms IS NULL OR duration_ms >= 0),
      checksum_sha256 text,
      r2_key text,
      error_message text,
      row_counts jsonb NOT NULL DEFAULT '{}'::jsonb,
      attempt int NOT NULL DEFAULT 1,
      triggered_by text NOT NULL DEFAULT 'cron' CHECK (triggered_by IN ('cron', 'manual', 'restore_drill'))
    );

    ALTER TABLE backup_metadata ENABLE ROW LEVEL SECURITY;
    CREATE POLICY backup_metadata_owner_read ON backup_metadata
      FOR SELECT TO authenticated USING (true);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS backup_metadata CASCADE;
  `);
}
