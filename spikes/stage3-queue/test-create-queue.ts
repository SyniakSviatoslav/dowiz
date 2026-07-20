import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({ connectionString: env.DATABASE_URL_SESSION });
  boss.on('error', console.error);
  await boss.start();
  try {
    await boss.createQueue('spike-queue');
    console.log('Queue created successfully.');
  } catch (err) {
    console.error('Error creating queue:', err);
  }
  const id = await boss.send('spike-queue', { test: true });
  console.log('Job ID:', id);
  await boss.stop();
  process.exit(0);
}

run();
