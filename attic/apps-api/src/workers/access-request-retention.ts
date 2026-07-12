// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { BUS_CHANNELS, QUEUE_NAMES } from '../lib/registry.js';

const env = loadEnv();

const RETENTION_LOCK = 5;   // pg_try_advisory_lock id for the retention sweep
const RECONCILE_LOCK = 6;   // pg_try_advisory_lock id for the notify-gap reconcile
const RECONCILE_GRACE = "5 minutes"; // a fresh row is given this long before reconcile re-feeds

/**
 * AccessRequestRetentionWorker (ADR-soft-access-gate, STOP-2 / B3 / R2-9) — owns the two
 * access-request crons:
 *   • access-request.retention-sweep — DELETE rows older than ACCESS_REQUEST_RETENTION
 *     (default 12 months) → 12-month auto-erase, the standing consent-expiry mechanism.
 *   • access-request.reconcile — re-enqueue notified_at IS NULL rows past a grace window
 *     and below the notify_attempts cap → recovers lost fire-and-forget enqueues; emits ONE
 *     aggregated ops alert on a persistent backlog (B7).
 *
 * R3-1 boot-safety: both boss.schedule(...) calls are `.catch`-wrapped (rates-refresh.ts
 * shape, NOT the un-`.catch`'d anonymizer shape) so a schedule throw at boot is logged
 * best-effort and CANNOT abort main() before fastify.listen() (no HTTP-dead zombie). Visible
 * failure is provided instead by assertAccessRequestSchedules() AFTER listen() — see below.
 */
export class AccessRequestRetentionWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.createQueue(QUEUE_NAMES.ACCESS_REQUEST_RETENTION_SWEEP);
    await this.boss.createQueue(QUEUE_NAMES.ACCESS_REQUEST_RECONCILE);

    await this.boss.work(QUEUE_NAMES.ACCESS_REQUEST_RETENTION_SWEEP, async () => this.runRetention());
    await this.boss.work(QUEUE_NAMES.ACCESS_REQUEST_RECONCILE, async () => this.runReconcile());

    const retentionCron = env.ACCESS_REQUEST_RETENTION_CRON || '0 3 * * *';
    const reconcileCron = env.ACCESS_REQUEST_RECONCILE_CRON || '*/15 * * * *';

    // .catch-wrapped (R3-1) — a schedule throw must never poison boot.
    await this.boss
      .schedule(QUEUE_NAMES.ACCESS_REQUEST_RETENTION_SWEEP, retentionCron, null, {
        singletonKey: QUEUE_NAMES.ACCESS_REQUEST_RETENTION_SWEEP,
      })
      .catch((err: any) => console.warn(`[AccessRequestRetention] retention schedule failed: ${err?.message}`));
    await this.boss
      .schedule(QUEUE_NAMES.ACCESS_REQUEST_RECONCILE, reconcileCron, null, {
        singletonKey: QUEUE_NAMES.ACCESS_REQUEST_RECONCILE,
      })
      .catch((err: any) => console.warn(`[AccessRequestRetention] reconcile schedule failed: ${err?.message}`));
  }

  private async runRetention() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query(`SELECT pg_try_advisory_lock(${RETENTION_LOCK}) AS locked`);
      if (!lock.rows[0]?.locked) {
        console.log('[AccessRequestRetention] Skipped — advisory lock held by another instance');
        return;
      }
      try {
        const window = env.ACCESS_REQUEST_RETENTION || '12 months';
        const res = await client.query(
          `DELETE FROM access_requests WHERE created_at < now() - $1::interval`,
          [window],
        );
        console.log(`[AccessRequestRetention] Erased ${res.rowCount} rows older than ${window}`);
      } finally {
        await client.query(`SELECT pg_advisory_unlock(${RETENTION_LOCK})`);
      }
    } catch (err) {
      console.error('[AccessRequestRetention] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }

  private async runReconcile() {
    const cap = env.ACCESS_REQUEST_NOTIFY_MAX_ATTEMPTS ?? 10;
    const client = await this.pool.connect();
    try {
      const lock = await client.query(`SELECT pg_try_advisory_lock(${RECONCILE_LOCK}) AS locked`);
      if (!lock.rows[0]?.locked) {
        console.log('[AccessRequestReconcile] Skipped — advisory lock held by another instance');
        return;
      }
      try {
        // Re-enqueue notify-gap rows still within the attempt cap.
        const due = await client.query(
          `SELECT id FROM access_requests
            WHERE notified_at IS NULL
              AND created_at < now() - $1::interval
              AND notify_attempts < $2`,
          [RECONCILE_GRACE, cap],
        );
        for (const row of due.rows) {
          await this.boss.send(QUEUE_NAMES.ACCESS_REQUEST_NOTIFY, { requestId: row.id }).catch(() => {});
        }
        if (due.rowCount > 0) {
          console.log(`[AccessRequestReconcile] Re-enqueued ${due.rowCount} un-notified rows`);
        }

        // Single aggregated ops alert (B7) for rows that have exhausted the attempt cap
        // and are stuck — never per-row paging.
        const stuck = await client.query(
          `SELECT count(*)::int AS n FROM access_requests
            WHERE notified_at IS NULL AND notify_attempts >= $1`,
          [cap],
        );
        const n = stuck.rows[0]?.n ?? 0;
        if (n > 0) {
          await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, {
            error: `[access-request] ${n} request(s) stuck un-notified past ${cap} attempts (status='new'); inspect access_requests`,
            time: new Date().toISOString(),
          });
        }
      } finally {
        await client.query(`SELECT pg_advisory_unlock(${RECONCILE_LOCK})`);
      }
    } catch (err) {
      console.error('[AccessRequestReconcile] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}

/**
 * R3-1 fail-fast boot-assert. Call AFTER fastify.listen() (so a failed deploy still answers
 * /livez long enough to be observed). Verifies both access-request cron schedules landed in
 * pgboss.schedule; if either is missing it is a VISIBLE deploy failure (process.exit(1) →
 * Fly restarts, deploy shows red) — strictly better than a silent live-but-HTTP-dead zombie.
 * In non-production a miss is logged but does NOT exit (so local/test boot is never bricked).
 */
export async function assertAccessRequestSchedules(pool: Pool): Promise<void> {
  const names = [QUEUE_NAMES.ACCESS_REQUEST_RETENTION_SWEEP, QUEUE_NAMES.ACCESS_REQUEST_RECONCILE];
  let present = new Set<string>();
  try {
    const res = await pool.query(
      `SELECT name FROM pgboss.schedule WHERE name = ANY($1::text[])`,
      [names],
    );
    present = new Set(res.rows.map((r: any) => r.name));
  } catch (err) {
    console.error('[AccessRequestBootAssert] could not read pgboss.schedule:', err);
  }
  const missing = names.filter((n) => !present.has(n));
  if (missing.length === 0) {
    console.log('[AccessRequestBootAssert] cron schedules present:', names.join(', '));
    return;
  }
  console.error(`[AccessRequestBootAssert] MISSING pgboss.schedule rows: ${missing.join(', ')}`);
  if (process.env.NODE_ENV === 'production') {
    console.error('[AccessRequestBootAssert] failing fast (process.exit 1) so the deploy shows red');
    process.exit(1);
  }
}
