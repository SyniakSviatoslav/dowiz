// @ts-nocheck
import type { FastifyInstance } from 'fastify';
import type { MessageBus, QueueProvider } from '@deliveryos/platform';
import type { Pool } from 'pg';
import fs from 'node:fs/promises';
import path from 'node:path';

// Track child processes for SIGTERM forwarding
const childProcesses = new Set<import('node:child_process').ChildProcess>();

export function registerChildProcess(cp: import('node:child_process').ChildProcess) {
  childProcesses.add(cp);
  cp.on('exit', () => childProcesses.delete(cp));
}

async function cleanupTempFiles() {
  const tempDir = path.join(process.cwd(), '.tmp', 'backups');
  try {
    await fs.access(tempDir);
    const files = await fs.readdir(tempDir);
    await Promise.all(files.map(f => fs.unlink(path.join(tempDir, f)).catch(() => {})));
    console.log(`[Shutdown] Cleaned up ${files.length} temporary backup files`);
  } catch (err: any) {
    console.debug('[shutdown] temp backup dir not found, skipping cleanup:', err?.message);
  }
}

export function setupShutdown(fastify: FastifyInstance, pool: Pool, messageBus: MessageBus, queue?: QueueProvider) {
  let shuttingDown = false;

  const shutdown = async (signal: string) => {
    if (shuttingDown) return;
    shuttingDown = true;
    console.log(`\n[API] Received ${signal}. Starting graceful shutdown...`);

    // 1. Forward SIGTERM to child processes (e.g., pg_dump)
    for (const cp of childProcesses) {
      try { cp.kill('SIGTERM'); } catch (err: any) {
        console.debug('[shutdown] failed to send SIGTERM to child process:', err?.message);
      }
    }

    // 2. Stop accepting new HTTP requests
    console.log('[API] Stopping HTTP server...');
    try {
      await fastify.close();
      console.log('[API] HTTP server closed');
    } catch (err) {
      console.error('[API] Error closing HTTP server', err);
    }

    // 3. Drain queue with timeout (allow active jobs to finish)
    if (queue) {
      console.log('[API] Draining queue...');
      try {
        await Promise.race([
          queue.stop(),
          new Promise(resolve => setTimeout(resolve, 10000)), // 10s timeout
        ]);
        console.log('[API] Queues drained');
      } catch (err) {
        console.error('[API] Error stopping queues', err);
      }
    }

    // 4. Close MessageBus
    console.log('[API] Closing MessageBus...');
    await messageBus.close();

    // 5. Cleanup temp files
    await cleanupTempFiles();

    // 6. Close db pool
    console.log('[API] Closing db pool...');
    try {
      await pool.end();
      console.log('[API] Pool closed');
    } catch (err) {
      console.error('[API] Error closing pool', err);
    }

    console.log('[API] Shutdown complete');
    process.exit(0);
  };

  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));
  process.on('SIGHUP', () => shutdown('SIGHUP'));
}
