// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS } from '../lib/registry.js';
import { runRetentionSweep } from '../modules/acquisition/retention.js';

// AcquisitionRetentionWorker — the recurring caller for the P6 retention sweep (GDPR Art-5(e)):
// reaps expired provision grants + claim invites and hard-deletes never-claimed PUBLIC shadows past the
// TTL. Mirrors AccessRequestRetentionWorker exactly (advisory-lock single-flight; .catch-wrapped schedule
// so a boot throw can never abort main() before fastify.listen). Queue name is a local constant (the
// shared QUEUE_NAMES registry is a protected governance file).
const ACQUISITION_RETENTION_SWEEP = 'acquisition.retention-sweep';
const ACQ_RETENTION_LOCK = 7; // pg_try_advisory_lock id (5/6 = access-request, distinct here)

export class AcquisitionRetentionWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.createQueue(ACQUISITION_RETENTION_SWEEP);
    await this.boss.work(ACQUISITION_RETENTION_SWEEP, async () => this.runSweep());

    // Daily at 03:30 by default (after the access-request 03:00 sweep). Short shadow TTL — a never-claimed
    // public clone is the aggregator-squatting posture; reap it promptly.
    const cron = process.env.ACQUISITION_RETENTION_CRON || '30 3 * * *';
    await this.boss
      .schedule(ACQUISITION_RETENTION_SWEEP, cron, null, { singletonKey: ACQUISITION_RETENTION_SWEEP })
      .catch((err: any) => console.warn(`[AcquisitionRetention] schedule failed: ${err?.message}`));
  }

  private async runSweep() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query(`SELECT pg_try_advisory_lock(${ACQ_RETENTION_LOCK}) AS locked`);
      if (!lock.rows[0]?.locked) {
        console.log('[AcquisitionRetention] Skipped — advisory lock held by another instance');
        return;
      }
      try {
        const ttl = Number(process.env.ACQUISITION_SHADOW_TTL_DAYS) || undefined;
        const res = await runRetentionSweep(this.pool, { abandonedTtlDays: ttl });
        console.log(`[AcquisitionRetention] reaped grants=${res.grants} invites=${res.invites} shadows=${res.shadows}`);
      } finally {
        await client.query(`SELECT pg_advisory_unlock(${ACQ_RETENTION_LOCK})`);
      }
    } catch (err) {
      console.error('[AcquisitionRetention] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}
