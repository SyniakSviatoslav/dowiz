import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';
import { randomUUID } from 'crypto';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({ connectionString: env.DATABASE_URL_SESSION });
  await boss.start();
  const manualId = randomUUID();
  console.log('Trying manual ID:', manualId);
  const id = await boss.send('spike-queue', { test: true }, { id: manualId });
  console.log('Returned Job ID:', id);
  await boss.stop();
  process.exit(0);
}

run();
