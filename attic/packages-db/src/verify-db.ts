import { createOperationalPool, createSessionPool } from './index.js';
import { loadEnv } from '@deliveryos/config';

async function verify() {
  let env;
  try {
    env = loadEnv();
  } catch (err: unknown) {
    console.error('Environment validation failed:', (err as Error).message);
    process.exit(1);
  }

  if (!env.***REDACTED*** || !env.***REDACTED***) {
    console.error('Error: Please fill DATABASE_URL_* in .env');
    process.exit(1);
  }

  console.log('Verifying connection to Operational pool (6543)...');
  const opPool = createOperationalPool();
  try {
    const res = await opPool.query('SELECT version()');
    console.log('✅ Operational pool connected:', res.rows[0].version);
  } catch (err: unknown) {
    console.error('❌ Operational pool failed. Перевір, що Supabase-проєкт не на паузі (Free) і рядок правильний.');
    console.error((err as Error).message);
    process.exit(1);
  } finally {
    await opPool.end();
  }

  console.log('Verifying connection to Session pool (5432)...');
  const sessionPool = createSessionPool();
  try {
    const res = await sessionPool.query('SELECT version()');
    console.log('✅ Session pool connected:', res.rows[0].version);
  } catch (err: unknown) {
    console.error('❌ Session pool failed. Перевір, що Supabase-проєкт не на паузі (Free) і рядок правильний.');
    console.error((err as Error).message);
    process.exit(1);
  } finally {
    await sessionPool.end();
  }

  console.log('All DB connections verified successfully.');
  process.exit(0);
}

verify();
