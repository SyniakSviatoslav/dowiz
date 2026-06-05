import { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE ops_worker_heartbeat
      ADD COLUMN IF NOT EXISTS instance_id text,
      ADD COLUMN IF NOT EXISTS job_name text,
      ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'healthy',
      ADD COLUMN IF NOT EXISTS last_job_at timestamptz;
    CREATE INDEX IF NOT EXISTS idx_worker_heartbeat_status_seen
      ON ops_worker_heartbeat (status, last_seen_at);
  `);
}

export async function down(): Promise<void> {}
