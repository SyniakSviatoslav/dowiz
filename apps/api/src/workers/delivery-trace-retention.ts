// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS } from '../lib/registry.js';

// deliver v2 (R3-1 / R2-7) — the GPS-crumb retention sweep (anonymize-not-delete). delivery_trace is
// tenant-scoped FORCE, so a context-free operational-pool UPDATE sees 0 rows; the sweep MUST go through the
// SECURITY DEFINER fn `anonymize_stale_delivery_trace(interval)` (reaches all-tenant rows via the function
// OWNER's BYPASSRLS). NULLs gps/name/price crumbs past the window (floored to the 7-day dispute window inside
// the fn); the non-PII facts (timestamp/outcome) survive. Mirrors AccessRequestRetentionWorker (advisory-lock
// single-flight; .catch-wrapped schedule so a boot throw can never abort main() before fastify.listen).
const DELIVERY_TRACE_RETENTION_SWEEP = 'delivery-trace.retention-sweep';
const DT_RETENTION_LOCK = 8; // pg_try_advisory_lock id (distinct from access-request 5/6, acquisition 7)

export class DeliveryTraceRetentionWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.createQueue(DELIVERY_TRACE_RETENTION_SWEEP);
    await this.boss.work(DELIVERY_TRACE_RETENTION_SWEEP, async () => this.runSweep());

    const cron = process.env.DELIVERY_TRACE_RETENTION_CRON || '15 4 * * *'; // daily 04:15
    await this.boss
      .schedule(DELIVERY_TRACE_RETENTION_SWEEP, cron, null, { singletonKey: DELIVERY_TRACE_RETENTION_SWEEP })
      .catch((err: any) => console.warn(`[DeliveryTraceRetention] schedule failed: ${err?.message}`));
  }

  private async runSweep() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query(`SELECT pg_try_advisory_lock(${DT_RETENTION_LOCK}) AS locked`);
      if (!lock.rows[0]?.locked) {
        console.log('[DeliveryTraceRetention] Skipped — advisory lock held by another instance');
        return;
      }
      try {
        // The fn floors p_window to the 7-day dispute window, so a mis-set env can never anonymize evidence
        // inside the dispute window. Default 14 days (7 dispute + 7 settlement buffer).
        const window = process.env.DELIVERY_TRACE_GPS_RETENTION || '14 days';
        const res = await client.query(`SELECT anonymize_stale_delivery_trace($1::interval) AS n`, [window]);
        console.log(`[DeliveryTraceRetention] anonymized ${res.rows[0]?.n ?? 0} stale GPS crumbs older than ${window}`);
      } finally {
        await client.query(`SELECT pg_advisory_unlock(${DT_RETENTION_LOCK})`);
      }
    } catch (err) {
      console.error('[DeliveryTraceRetention] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}

/**
 * R2-7 boot-assert — call AFTER fastify.listen(). A missing GPS-anonymize cron schedule is a VISIBLE prod
 * deploy failure (process.exit(1) in production), not a silent indefinite-retention drift. Mirrors
 * assertAccessRequestSchedules. (Schedule-existence only; the OUTCOME-based efficacy proof lives in the
 * delivery-trace-retention efficacy test, L5.)
 */
export async function assertDeliveryTraceSchedule(pool: Pool): Promise<void> {
  let present = false;
  try {
    const res = await pool.query(`SELECT 1 FROM pgboss.schedule WHERE name = $1`, [DELIVERY_TRACE_RETENTION_SWEEP]);
    present = (res.rowCount ?? 0) > 0;
  } catch (err) {
    console.error('[DeliveryTraceBootAssert] could not read pgboss.schedule:', err);
  }
  if (present) {
    console.log('[DeliveryTraceBootAssert] cron schedule present:', DELIVERY_TRACE_RETENTION_SWEEP);
    return;
  }
  console.error(`[DeliveryTraceBootAssert] MISSING pgboss.schedule row: ${DELIVERY_TRACE_RETENTION_SWEEP}`);
  if (process.env.NODE_ENV === 'production') {
    console.error('[DeliveryTraceBootAssert] failing fast (process.exit 1) so the deploy shows red');
    process.exit(1);
  }
}
