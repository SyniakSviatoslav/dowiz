import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  console.log('Connecting to DATABASE_URL_SESSION (5432) for pg-boss install...');

  const boss = new PgBoss({
    connectionString: env.DATABASE_URL_SESSION,
    max: 3, // keep small
  });

  boss.on('error', (error) => console.error('pg-boss error:', error));

  try {
    await boss.start();
    console.log('✅ H1 SUCCESS: pg-boss installed schema successfully.');
  } catch (err: unknown) {
    console.error('❌ H1 FAILED: Could not install pg-boss schema.');
    console.error(err);
  } finally {
    await boss.stop().catch(() => {});
    process.exit(0);
  }
}

run();
