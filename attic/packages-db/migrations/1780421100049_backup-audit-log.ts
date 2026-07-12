import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE backup_audit_log (
      id bigserial PRIMARY KEY,
      backup_id uuid REFERENCES backup_metadata(id) ON DELETE CASCADE,
      action text NOT NULL CHECK (action IN ('started', 'completed', 'failed', 'restore_drill_started', 'restore_drill_completed', 'key_rotated', 'retention_violated')),
      actor_kind text NOT NULL CHECK (actor_kind IN ('system', 'owner', 'admin')),
      actor_id uuid,
      metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    ALTER TABLE backup_audit_log ENABLE ROW LEVEL SECURITY;
    CREATE POLICY backup_audit_log_owner_read ON backup_audit_log
      FOR SELECT TO authenticated USING (true);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS backup_audit_log CASCADE;
  `);
}
