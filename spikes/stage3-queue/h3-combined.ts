import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const queueName = 'spike-queue-3';

  const bossProducer = new PgBoss({ connectionString: env.***REDACTED***, max: 2 });
  const bossWorker1 = new PgBoss({ connectionString: env.***REDACTED***, max: 2 });
  const bossWorker2 = new PgBoss({ connectionString: env.***REDACTED***, max: 2 });

  await bossProducer.start();
  await bossWorker1.start();
  await bossWorker2.start();
  
  await bossProducer.createQueue(queueName);

  console.log('--- Starting Workers ---');
  let processedCount = 0;

  const handler = (workerId: string) => async (jobs: unknown[]) => {
    for (const job of jobs as Record<string, unknown>[]) {
      console.log(`[${workerId}] Received job: ${job.id} | Data:`, job.data);
      processedCount++;
      await new Promise(resolve => setTimeout(resolve, 500));
      console.log(`[${workerId}] Completed job: ${job.id}`);
    }
  };

  await bossWorker1.work(queueName, handler('worker-1'));
  await bossWorker2.work(queueName, handler('worker-2'));

  console.log('--- Sending Jobs ---');
  for (let i = 0; i < 10; i++) {
    const id = await bossProducer.send(queueName, { id: i, type: 'standard' });
    console.log(`Sent standard job, got id: ${id}`);
  }

  await bossProducer.send(queueName, { type: 'delayed' }, { startAfter: 5 }); // 5 seconds

  await bossProducer.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task', singletonSeconds: 60 });
  await bossProducer.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task', singletonSeconds: 60 });
  await bossProducer.send(queueName, { type: 'singleton' }, { singletonKey: 'unique-task', singletonSeconds: 60 });

  console.log('Jobs enqueued. Waiting 12 seconds for delayed job to fire...');
  
  await new Promise(resolve => setTimeout(resolve, 12000));
  
  console.log(`Total jobs processed: ${processedCount} (Expected: 10 standard + 1 delayed + 1 singleton = 12)`);
  
  await bossProducer.stop();
  await bossWorker1.stop();
  await bossWorker2.stop();
  process.exit(0);
}

run();
