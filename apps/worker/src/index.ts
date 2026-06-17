import { createSessionPool } from '@deliveryos/db';
import { PgBossQueueProvider, PgMessageBus } from '@deliveryos/platform';
import { Heartbeat } from './heartbeat.js';
import { registerHandlers } from './handlers.js';
import { setupShutdown } from './shutdown.js';

async function main() {
  console.log('[Worker] Starting...');

  const pool = createSessionPool();
  const queue = new PgBossQueueProvider();

  await queue.start();
  console.log('[Worker] QueueProvider started');

  const messageBus = new PgMessageBus(pool);
  registerHandlers(queue, pool, messageBus);
  console.log('[Worker] Handlers registered');

  const heartbeat = new Heartbeat(pool);
  heartbeat.start();
  console.log('[Worker] Heartbeat started');

  setupShutdown(queue, pool, heartbeat);

  console.log('[Worker] Ready');
}

main().catch(err => {
  console.error('[Worker] Fatal error during startup:', err);
  process.exit(1);
});
