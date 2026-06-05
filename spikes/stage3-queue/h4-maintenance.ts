import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';

async function run() {
  const env = loadEnv();
  const pool = createSessionPool();

  try {
    const res = await pool.query('SELECT version()');
    console.log('✅ PG Version:', res.rows[0].version);
  } catch (err) {
    console.error('Failed to get PG version', err);
  } finally {
    await pool.end();
  }

  console.log('Connecting to pg-boss...');
  const boss = new PgBoss({
    connectionString: env.***REDACTED***,
    max: 3,
  });

  try {
    await boss.start();
    console.log('Running maintenance manually...');
    await boss.maintain();
    console.log('✅ H4 SUCCESS: Maintenance completed without advisory lock errors.');
  } catch (err: unknown) {
    console.error('❌ H4 FAILED: Maintenance error.');
    console.error(err);
  } finally {
    await boss.stop().catch(() => {});
    process.exit(0);
  }
}

run();
