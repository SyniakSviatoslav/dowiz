import { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Ops heartbeat table for worker liveness tracking
  // Intentionally excluded from RLS as it contains purely operational/infrastructure state,
  // not tenant data. Access is strictly constrained to the worker process (upsert)
  // and the API /health check endpoint (read).
  pgm.sql(`
    CREATE TABLE ops_worker_heartbeat (
      worker_id text PRIMARY KEY,
      last_seen_at timestamptz NOT NULL
    );
    ALTER TABLE ops_worker_heartbeat DISABLE ROW LEVEL SECURITY;
  `);
}

export async function down(): Promise<void> {}
