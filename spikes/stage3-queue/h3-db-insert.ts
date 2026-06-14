import pg from 'pg';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const pool = new pg.Pool({ connectionString: env.***REDACTED*** });
  const res = await pool.query("INSERT INTO pgboss.job (id, name, state, created_on, start_after, expire_in, keep_until) VALUES (gen_random_uuid(), 'spike-queue', 'created', now(), now(), interval '1 day', interval '30 days') RETURNING id");
  console.log('Raw insert returning id:', res.rows);
  await pool.end();
}

run();
