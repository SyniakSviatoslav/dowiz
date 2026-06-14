import pg from 'pg';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const pool = new pg.Pool({ connectionString: env.***REDACTED*** });
  const res = await pool.query("SELECT * FROM pgboss.queue");
  console.log('Queues:', res.rows);
  await pool.end();
}

run();
