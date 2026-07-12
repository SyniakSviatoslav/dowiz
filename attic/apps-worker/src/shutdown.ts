import type { QueueProvider } from '@deliveryos/platform';
import { Heartbeat } from './heartbeat.js';
import type { Pool } from 'pg';

export function setupShutdown(queue: QueueProvider, pool: Pool, heartbeat: Heartbeat) {
  const shutdown = async (signal: string) => {
    console.log(`\n[Worker] Received ${signal}. Starting graceful shutdown...`);
    
    heartbeat.stop();
    console.log('[Worker] Stopped heartbeat');
    
    console.log('[Worker] Draining queues...');
    try {
      await queue.stop();
      console.log('[Worker] Queues drained');
    } catch (err) {
      console.error('[Worker] Error stopping queues', err);
    }
    
    console.log('[Worker] Closing db pool...');
    await pool.end();
    
    console.log('[Worker] Shutdown complete');
    process.exit(0);
  };

  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));
}
