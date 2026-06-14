import pg from 'pg';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const pool = new pg.Pool({ connectionString: env.DATABASE_URL_SESSION });
  const res = await pool.query("SELECT table_name FROM information_schema.tables WHERE table_schema='pgboss'");
  console.log('Tables in pgboss schema:', res.rows);
  await pool.end();
}

run();
