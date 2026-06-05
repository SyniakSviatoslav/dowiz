import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({
    connectionString: env.***REDACTED***,
    max: 2,
  });

  await boss.start();

  const queueName = 'spike-queue';

  console.log('Sending 10 standard jobs...');
  for (let i = 0; i < 10; i++) {
    await boss.send(queueName, { id: i, type: 'standard' });
  }

  console.log('Sending 1 delayed job (startAfter: 10s)...');
  await boss.send(queueName, { type: 'delayed' }, { startAfter: 10 });

  console.log('Sending 3 singleton jobs with same singletonKey...');
  await boss.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task' });
  await boss.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task' });
  await boss.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task' });

  console.log('✅ H3 Producer: All jobs enqueued.');
  await boss.stop();
  process.exit(0);
}

run();
