// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

const STALE_MS = parseInt(env.WORKER_LIVENESS_STALE_MS || '60000', 10);
// Fallback mirrors the packages/config WORKER_CRITICAL_LIST default (ADR-dispatch-recovery R3′:
// backup-hourly added — the data-recovery red-line gets a live 60s death-detection path).
const CRITICAL_WORKERS = (env.WORKER_CRITICAL_LIST || 'dispatcher,settlement-cron,dwell-monitor,anonymizer-retention,backup-hourly')
  .split(',').map(w => w.trim());

interface StaleWorker {
  worker_id: string;
  instance_id: string | null;
  job_name: string | null;
  last_seen_at: Date;
  stale_seconds: number;
}

export class LivenessChecker {
  private previouslyStale = new Set<string>();

  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work(QUEUE_NAMES.LIVENESS_CHECK, { singletonKey: QUEUE_NAMES.LIVENESS_CHECK }, async () => {
      await this.run();
    });
    const cronMs = parseInt(env.WORKER_LIVENESS_CHECK_MS || '60000', 10);
    const cronSec = Math.max(Math.floor(cronMs / 1000), 30);
    await this.boss.createQueue(QUEUE_NAMES.LIVENESS_CHECK);
    await this.boss.schedule(QUEUE_NAMES.LIVENESS_CHECK, `*/${cronSec} * * * * *`, null, {
      singletonKey: QUEUE_NAMES.LIVENESS_CHECK,
    });
  }

  private async run() {
    const intervalStr = `${STALE_MS / 1000} seconds`;
    try {
      const staleRes = await this.pool.query<StaleWorker>(
        `SELECT worker_id, instance_id, job_name, last_seen_at,
                EXTRACT(EPOCH FROM now() - last_seen_at)::int AS stale_seconds
         FROM ops_worker_heartbeat
         WHERE status != 'healthy'
            OR last_seen_at < now() - $1::interval
         ORDER BY stale_seconds DESC`,
        [intervalStr],
      );

      const staleWorkers = staleRes.rows;
      const criticalStale = staleWorkers.filter(w => CRITICAL_WORKERS.includes(w.worker_id));
      const currentStaleIds = new Set(staleWorkers.map(w => w.worker_id));

      // — Alert for newly stale critical workers —
      const newlyStale = criticalStale.filter(w => !this.previouslyStale.has(w.worker_id));
      if (newlyStale.length > 0) {
        const lines = newlyStale.map(w =>
          `• ${w.worker_id} (${w.instance_id || 'unknown'}) — ${w.stale_seconds}s stale`
        );
        const message = `⚠️ Worker liveness alert:\n${lines.join('\n')}`;

        if (newlyStale.length > 3) {
          await this.messageBus.publish(BUS_CHANNELS.WORKER_BATCH_STALE, {
            count: newlyStale.length,
            workers: newlyStale.map(w => w.worker_id),
            timestamp: new Date().toISOString(),
          });
        } else {
          for (const w of newlyStale) {
            await this.messageBus.publish(BUS_CHANNELS.WORKER_STALE, {
              workerId: w.worker_id,
              instanceId: w.instance_id,
              staleSeconds: w.stale_seconds,
              timestamp: new Date().toISOString(),
            });
          }
        }

        await this.messageBus.publish(BUS_CHANNELS.ALERT_WORKER_LIVENESS, {
          message,
          criticalCount: newlyStale.length,
          timestamp: new Date().toISOString(),
        });
      }

      // — Auto-resolve for recovered workers —
      const recovered = [...this.previouslyStale].filter(id => !currentStaleIds.has(id));
      for (const workerId of recovered) {
        await this.messageBus.publish(BUS_CHANNELS.WORKER_RECOVERED, {
          workerId,
          timestamp: new Date().toISOString(),
        });
      }

      this.previouslyStale = currentStaleIds;
    } catch (err) {
      console.error('[LivenessChecker] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.LIVENESS_CHECK_FAILED, {
        error: String(err),
        time: new Date().toISOString(),
      });
    }
  }
}
