import type { MigrationBuilder } from 'node-pg-migrate';
import PgBoss from 'pg-boss';

/**
 * P0-PGBOSS · Bootstrap the pg-boss schema during migration (idempotent).
 *
 * Runtime (server.ts -> PgBossQueueProvider) starts pg-boss with `migrate:false`,
 * so it NEVER creates its own tables -- it only reads pgboss.version. On a fresh
 * database that schema does not exist, so `boss.start()` throws
 * "pg-boss is not installed" and the API cannot boot.
 *
 * This migration runs pg-boss's OWN installer once, against the *migration*
 * connection role (dowiz_migrator / superuser on bare PG, postgres on Supabase),
 * which creates the `pgboss` schema + internal tables idempotently. We then
 * re-apply the SAME least-privilege DML-only grants that 1790000000009 applies,
 * so the runtime role can use the freshly-created tables.
 *
 * --- Transaction handling (important) ---
 * node-pg-migrate runs ALL pending migrations inside ONE transaction
 * (`--single-transaction` defaults to true). pg-boss's installer opens its OWN
 * connection; from there the `pgboss` schema that migration 1790000000006 created
 * is still UNCOMMITTED and invisible, so pg-boss tries to CREATE it again and
 * deadlocks on pg_namespace -> "canceling statement due to lock timeout".
 *
 * Fix: mark this migration `pgm.noTransaction()` (so node-pg-migrate breaks the
 * single transaction around it) AND explicitly COMMIT the runner connection
 * before invoking pg-boss, so every prior migration -- including the pgboss
 * schema -- is durably visible to pg-boss's separate connection. We reopen a
 * transaction afterwards so node-pg-migrate's bookkeeping INSERT proceeds
 * normally. The schema-creation work is committed before boss.start() runs.
 *
 * Idempotent: pg-boss's contractor only runs install/migrate steps not already
 * applied, so re-running on an existing DB is a no-op. Prod (Supabase) is
 * unaffected: on an already-bootstrapped DB boss.start() just verifies the
 * version and the re-grant statements are no-ops.
 *
 * Ordering note: 1790000000008 (grant CREATE) and 1790000000009 (revoke CREATE)
 * are consecutive 13-digit timestamps with no integer gap, so a migration cannot
 * be slotted strictly between them. The migration role is a superuser and does
 * not need the transient PUBLIC CREATE grant, so we run AFTER 0009 and re-grant,
 * leaving the DB in the intended end state (schema present, runtime role DML-only).
 */

const MIGRATION_DB_URL =
  process.env.DATABASE_URL_MIGRATIONS || process.env.DATABASE_URL_SESSION;

/**
 * Source of truth: packages/shared-types/src/queue-names.ts (QUEUE_NAMES).
 * pg-boss 10 backs each queue with its OWN partition table created via
 * `CREATE TABLE pgboss.<hash> (LIKE pgboss.job)` -- which needs CREATE on the
 * pgboss schema. The runtime app role had CREATE revoked (migration 0009), so it
 * CANNOT create these at boot. We therefore pre-create every application queue
 * here under the migration (superuser) role; the runtime's idempotent
 * `createQueue()` calls then become no-ops. Keep this list in sync with
 * queue-names.ts (covered by the fresh-provision CI smoke test).
 */
const APP_QUEUES: readonly string[] = [
  'notify.dispatch',
  'notify.customer_status',
  'notify.telegram.send',
  'order.pending_aging',
  'order.timeout',
  'courier.dispatch',
  'courier.stale_check',
  'settlement.cron',
  'settlement.generate',
  'dwell.monitor',
  'anonymizer.retention',
  'anonymizer.gdpr',
  'velocity.flush',
  'free_tier.watch',
  'signal.raiser',
  'liveness.check',
  'gps.purge',
  'backup.hourly',
  'backup.daily',
  'backup.weekly',
  'backup.monthly',
  'backup.verify.restore',
  'backup.verify.r2',
  'reconciliation.nightly',
  'rates.refresh',
  // Soft access gate (ADR-soft-access-gate) — fresh-provision parity with queue-names.ts.
  // 1790000000042 is the load-bearing pre-create for already-provisioned envs.
  'access-request.notify',
  'access-request.reconcile',
  'access-request.retention-sweep',
];

export async function up(pgm: MigrationBuilder): Promise<void> {
  if (!MIGRATION_DB_URL) {
    throw new Error(
      'pgboss-bootstrap: DATABASE_URL_MIGRATIONS (or DATABASE_URL_SESSION) must be set'
    );
  }

  // Break out of node-pg-migrate's single transaction for this migration.
  pgm.noTransaction();

  // Commit everything applied so far (incl. the pgboss schema from 0006) so it
  // is visible to pg-boss's separate connection. Harmless if no tx is open.
  await pgm.db.query('COMMIT');

  // 1. Let pg-boss install/upgrade its own schema -- the ONLY place pg-boss runs
  //    DDL, under the migration (superuser) role.
  const boss = new PgBoss({
    connectionString: MIGRATION_DB_URL,
    schema: 'pgboss',
    application_name: 'pgboss-migration-bootstrap',
    max: 2,
    migrate: true,
  });
  boss.on('error', (err) => console.error('[pgboss-bootstrap] pg-boss error:', err));
  await boss.start();

  // Pre-create every application queue (and its partition table) under the
  // superuser role, since the runtime role has no CREATE on pgboss. Idempotent.
  for (const q of APP_QUEUES) {
    await boss.createQueue(q);
  }

  await boss.stop({ graceful: true, wait: true });

  // 2. Re-apply least-privilege DML grants on the now-existing pgboss tables,
  //    mirroring migration 1790000000009 so the runtime (app) role can operate
  //    the queue without any DDL privilege. Idempotent. Runs in autocommit.
  await pgm.db.query(`
    DO $bootstrap_grants$
    DECLARE
      t text;
    BEGIN
      GRANT USAGE ON SCHEMA pgboss TO PUBLIC;
      FOR t IN SELECT table_name FROM information_schema.tables
               WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
      LOOP
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON pgboss.%I TO PUBLIC', t);
      END LOOP;
      EXECUTE 'GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA pgboss TO PUBLIC';
    END;
    $bootstrap_grants$;
  `);

  // 3. Default privileges for any tables a future pg-boss upgrade adds.
  await pgm.db.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss
           GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO PUBLIC;`);
  await pgm.db.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss
           GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO PUBLIC;`);

  // Reopen a transaction so node-pg-migrate's "INSERT INTO pgmigrations" (and
  // the COMMIT it appends for noTransaction migrations) operate as expected.
  await pgm.db.query('BEGIN');
}

export async function down(): Promise<void> {
  // No-op: 1790000000006's down() drops the pgboss schema (CASCADE), removing
  // everything this migration created.
}
