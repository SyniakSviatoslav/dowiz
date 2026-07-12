import pg from 'pg';
import { loadEnv } from '@deliveryos/config';

const { Pool } = pg;
const env = loadEnv();

/**
 * Operational Pool (Transaction Mode - port 6543)
 * Hot path for API requests.
 * 
 * CRITICAL WARNING: Supavisor's transaction mode does NOT support named/cached prepared statements.
 * Default parameterized queries in node-postgres (`query(text, values)`) use unnamed statements,
 * which is OK. Do NOT enable prepared statement caching or use libraries that cache statements
 * on this pool. If statement caching is needed in the future, use `?pgbouncer=true` or switch 
 * to the session mode pool.
 */
export function createOperationalPool(): pg.Pool {
  const pool = new Pool({
    connectionString: env.DATABASE_URL_OPERATIONAL,
    // Hot-path pool size. Supavisor transaction mode (:6543) multiplexes, so this safely
    // exceeds the old hardcoded 8 — raised to stop public-storefront bursts from starving the
    // pool (the "menu blinks empty under load" fix). Env-tunable; default 20.
    max: env.OPERATIONAL_POOL_SIZE,
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 5000,
    // Honor sslmode=disable (local / tunneled DBs without TLS); default to TLS otherwise.
    ssl: /[?&]sslmode=disable/.test(env.DATABASE_URL_OPERATIONAL) ? false : { rejectUnauthorized: false }
  });

  // FX-9: statement_timeout for operational queries — kill slow queries fast
  // DB Role Guardrail: Prevent operational pool from connecting as superuser (which bypasses RLS)
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '10s'");
    const res = await client.query('SELECT current_user');
    if (res.rows[0].current_user === 'postgres') {
      client.release(true); // Destroy the connection
      throw new Error("SECURITY FAULT: Operational pool connected as 'postgres' superuser. This bypasses RLS. Use a dedicated restricted role.");
    }
  });

  return pool;
}

/**
 * Session Pool (Session Mode - port 5432)
 * For workers, analytics, tasks that require session state (e.g. SET, advisory locks, DDL).
 */
export function createSessionPool(): pg.Pool {
  const pool = new Pool({
    connectionString: env.DATABASE_URL_SESSION,
    max: 3,
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 5000,
    ssl: /[?&]sslmode=disable/.test(env.DATABASE_URL_SESSION) ? false : { rejectUnauthorized: false }
  });

  // FX-9: statement_timeout for session queries — longer, for workers/analytics
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '30s'");
  });

  return pool;
}
