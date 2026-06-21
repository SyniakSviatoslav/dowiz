import type { MigrationBuilder } from 'node-pg-migrate';
import PgBoss from 'pg-boss';

/**
 * B1 FIX (ADR-soft-access-gate) — pre-create the access-request pg-boss queues under
 * the MIGRATION role. Mirrors 1790000000011_pgboss-bootstrap-schema.
 *
 * Why this migration is load-bearing: pg-boss v10 backs each queue with its OWN
 * partition table created via DDL, and 1790000000009 REVOKEd CREATE ON SCHEMA pgboss
 * from the runtime (NOBYPASSRLS) role. So the runtime boot-loop `createQueue(...)` for a
 * genuinely-new queue THROWS and is swallowed (server.ts ~290 `.catch(console.warn)`) →
 * 100% of notifications silently lost. We therefore create the 3 queues here, under the
 * migration (superuser) role that CAN DDL; runtime createQueue then becomes a no-op.
 *
 * Idempotent: createQueue is a no-op if the queue exists. On already-provisioned envs
 * (prod/staging) re-running is harmless. APP_QUEUES in 1790000000011 also lists these for
 * fresh-provision parity, but 011 does not re-run on prod — THIS is the fix for already-
 * provisioned environments.
 */
const MIGRATION_DB_URL =
  process.env.DATABASE_URL_MIGRATIONS || process.env.DATABASE_URL_SESSION;

const NEW_QUEUES: readonly string[] = [
  'access-request.notify',           // per-row operator email
  'access-request.reconcile',        // notify-gap sweep cron (B3 / R2-4)
  'access-request.retention-sweep',  // 12-month auto-erase cron (STOP-2)
];

export async function up(pgm: MigrationBuilder): Promise<void> {
  if (!MIGRATION_DB_URL) {
    throw new Error(
      'access-request-notify-queue: DATABASE_URL_MIGRATIONS (or DATABASE_URL_SESSION) must be set'
    );
  }

  // Break out of node-pg-migrate's single transaction; commit prior work so the
  // pgboss schema is visible to pg-boss's separate connection.
  pgm.noTransaction();
  await pgm.db.query('COMMIT');

  const boss = new PgBoss({
    connectionString: MIGRATION_DB_URL,
    schema: 'pgboss',
    application_name: 'access-request-queue-bootstrap',
    max: 2,
    migrate: true,
  });
  boss.on('error', (err) => console.error('[access-request-queue] pg-boss error:', err));
  await boss.start();
  for (const q of NEW_QUEUES) {
    await boss.createQueue(q);
  }
  await boss.stop({ graceful: true, wait: true });

  // Re-apply least-privilege DML grants on the new partition tables to the runtime
  // role (idempotent; mirrors 1790000000011 / 1790000000009).
  await pgm.db.query(`
    DO $access_req_grants$
    DECLARE t text;
    BEGIN
      FOR t IN SELECT table_name FROM information_schema.tables
               WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
      LOOP
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON pgboss.%I TO PUBLIC', t);
      END LOOP;
    END;
    $access_req_grants$;
  `);

  // Reopen a transaction so node-pg-migrate's bookkeeping INSERT proceeds normally.
  await pgm.db.query('BEGIN');
}

export async function down(): Promise<void> {
  // No-op: 1790000000006's down() drops the pgboss schema (CASCADE).
}
