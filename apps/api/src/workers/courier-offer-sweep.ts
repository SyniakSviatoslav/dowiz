// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { dashboardChannel } from '../lib/registry.js';

// deliver v2 §A — the durable offer timer. An 'offered' courier assignment past offered_expires_at flips to
// 'offered_expired' and the order is re-enqueued for another courier. 🔴 The customer order is UNTOUCHED (only
// the binding rolls back) — an unanswered offer can never trap the customer. The deadline is DATA + a 1-min
// idempotent sweep (no live timer to lose), the same machinery shape as OrderTimeoutSweep. Inert unless owners
// actually create 'offered' rows (COURIER_OFFER_HANDSHAKE_ENABLED) — a no-op sweep otherwise.
const SWEEP_QUEUE = 'courier.offer_sweep';
const SWEEP_CRON = '* * * * *';
const SWEEP_LOCK_ID = 9; // distinct: 5 order-timeout, 7 acquisition, 8 delivery-trace

export class CourierOfferSweepWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work(SWEEP_QUEUE, { singletonKey: SWEEP_QUEUE }, async () => this.run());
    await this.boss.createQueue(SWEEP_QUEUE);
    await this.boss
      .schedule(SWEEP_QUEUE, SWEEP_CRON, null, { singletonKey: SWEEP_QUEUE })
      .catch((err: any) => console.warn(`[CourierOfferSweep] schedule failed: ${err?.message}`));
    console.log('[CourierOfferSweep] scheduled (1-min offer-expiry sweep)');
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [SWEEP_LOCK_ID]);
      if (!lock.rows[0]?.locked) return;
      try {
        // Guarded transition: status='offered' AND past deadline → 'offered_expired'. Cross-tenant (one pass).
        // The order row is deliberately not touched.
        const res = await client.query(
          `UPDATE courier_assignments
              SET status='offered_expired', cancelled_at=now(), cancellation_reason='offer_timeout'
            WHERE status='offered' AND offered_expires_at < now()
          RETURNING order_id, location_id`,
        );
        if (!res.rowCount) return;
        console.log(`[CourierOfferSweep] expired ${res.rowCount} unanswered offer(s) → re-offered`);
        for (const row of res.rows) {
          try {
            await client.query(
              `INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now())
               ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`,
              [row.order_id, row.location_id],
            );
            await this.messageBus.publish(dashboardChannel(row.location_id), { type: 'offer_expired', orderId: row.order_id });
          } catch (e) {
            console.error(`[CourierOfferSweep] re-enqueue failed for ${row.order_id}:`, e);
          }
        }
      } finally {
        await client.query('SELECT pg_advisory_unlock($1)', [SWEEP_LOCK_ID]);
      }
    } catch (err) {
      console.error('[CourierOfferSweep] Error:', err);
    } finally {
      client.release();
    }
  }
}
