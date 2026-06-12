/**
 * NX-2 bootstrap: Pre-create pg-boss tables under admin/migrate role.
 *
 * Runs pg-boss with migrate:true (default) using DATABASE_URL_MIGRATIONS
 * (admin role with DDL privileges). After this script completes,
 * production pg-boss instances can start with migrate:false.
 *
 * Usage: pnpm tsx --env-file=.env scripts/bootstrap-pgboss.ts
 */

import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function bootstrap() {
  const env = loadEnv();
  const adminUrl = env.DATABASE_URL_MIGRATIONS;

  if (!adminUrl) {
    console.error('DATABASE_URL_MIGRATIONS not set — cannot bootstrap pg-boss');
    process.exit(1);
  }

  console.log('[bootstrap-pgboss] Creating pg-boss tables via admin role...');
  console.log(`[bootstrap-pgboss] Schema: pgboss`);

  const boss = new PgBoss({
    connectionString: adminUrl,
    schema: 'pgboss',
    max: 1,
    application_name: 'pgboss-bootstrap',
    // migrate: true is the default — this will create all internal tables
  });

  try {
    await boss.start();
    console.log('[bootstrap-pgboss] ✓ pg-boss started and tables created successfully');

    // Verify tables exist
    const { Pool } = await import('pg');
    const pool = new Pool({ connectionString: adminUrl, max: 1 });
    const res = await pool.query(
      `SELECT table_name FROM information_schema.tables
       WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
       ORDER BY table_name`
    );
    console.log(`[bootstrap-pgboss] Tables in pgboss schema: ${res.rows.map(r => r.table_name).join(', ')}`);
    await pool.end();

    console.log('[bootstrap-pgboss] ✓ Bootstrap complete — pg-boss is ready for migrate:false');
  } catch (err) {
    console.error('[bootstrap-pgboss] Failed to bootstrap pg-boss:', err);
    process.exit(1);
  } finally {
    await boss.stop({ graceful: true, wait: true });
  }
}

bootstrap();
