import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({
    connectionString: env.***REDACTED***,
  });

  await boss.start();
  const res = await boss.getQueueSize('spike-queue');
  console.log('Queue size:', res);
  
  await boss.stop();
  process.exit(0);
}

run();
