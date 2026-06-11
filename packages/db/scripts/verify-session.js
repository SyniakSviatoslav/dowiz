import { createSessionPool } from './src/index.js';
import { loadEnv } from '@deliveryos/config';

async function verifySessionPool() {
  let env;
  try {
    env = loadEnv();
  } catch (err) {
    console.error('Environment validation failed:', err);
    process.exit(1);
  }

  if (!env.DATABASE_URL_SESSION) {
    console.error('Error: Please fill DATABASE_URL_SESSION in .env');
    process.exit(1);
  }

  console.log('Verifying connection to Session pool (5432)...');
  const sessionPool = createSessionPool();
  try {
    const res = await sessionPool.query('SELECT version()');
    console.log('✅ Session pool connected:', res.rows[0].version);
  } catch (err) {
    console.error('❌ Session pool failed.');
    console.error(err.message);
    process.exit(1);
  } finally {
    await sessionPool.end();
  }
}

verifySessionPool();