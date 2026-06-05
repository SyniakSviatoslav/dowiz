import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({ connectionString: env.***REDACTED*** });
  await boss.start();
  
  await boss.createQueue('spike-singleton');
  
  const id1 = await boss.send('spike-singleton', { test: 1 }, { singletonKey: 'unique-task', singletonSeconds: 60 });
  const id2 = await boss.send('spike-singleton', { test: 2 }, { singletonKey: 'unique-task', singletonSeconds: 60 });
  const id3 = await boss.send('spike-singleton', { test: 3 }, { singletonKey: 'unique-task', singletonSeconds: 60 });
  
  console.log('ID 1:', id1);
  console.log('ID 2:', id2);
  console.log('ID 3:', id3);
  
  await boss.stop();
  process.exit(0);
}

run();
