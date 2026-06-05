import PgBoss from 'pg-boss';
import { loadEnv } from '@deliveryos/config';

async function run() {
  const env = loadEnv();
  const boss = new PgBoss({
    connectionString: env.DATABASE_URL_SESSION,
    max: 2,
  });

  await boss.start();

  const queueName = 'spike-queue';
  const workerId = process.argv[2] || 'worker-unknown';

  console.log(`[${workerId}] Worker started. Waiting for jobs...`);

  await boss.work(queueName, async (jobs) => {
    // In pg-boss >= 9, jobs is an array
    for (const job of jobs) {
      console.log(`[${workerId}] Received job: ${job.id} | Data:`, job.data);
      // simulate work
      await new Promise(resolve => setTimeout(resolve, 500));
      console.log(`[${workerId}] Completed job: ${job.id}`);
    }
  });

  // Keep alive for 25 seconds to catch the delayed job
  setTimeout(async () => {
    console.log(`[${workerId}] Shutting down...`);
    await boss.stop();
    process.exit(0);
  }, 25000);
}

run();
