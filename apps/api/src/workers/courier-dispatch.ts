// @ts-nocheck
import { Pool } from 'pg';
import type { QueueProvider, MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

// ADR-dispatch-recovery (B2): this worker consumes COURIER_DISPATCH jobs pumped from the
// courier_dispatch_queue journal by the CourierOfferSweep drain pass (Option C — durable
// journal + idempotent 1-min sweep-relay). The former in-worker 30s self-retry
// (`this.boss.send`, an undefined-field TypeError) is DELETED, not patched: the 60s pump
// is the single retry cadence, so escalation ≈ COURIER_DISPATCH_MAX_ATTEMPTS × 60s.
// COURIER_DISPATCH_RETRY_MS is retired.
export class CourierDispatchWorker {
  constructor(
    private pool: Pool,
    private queue: QueueProvider,
    private messageBus: MessageBus
  ) {}

  async start() {
    await this.queue.work(QUEUE_NAMES.COURIER_DISPATCH, async (data: any) => {
      const { orderId, locationId } = data;
      await this.handleDispatch(orderId, locationId);
    });
  }

  async handleDispatch(orderId: string, locationId: string) {
    const maxAttempts = parseInt(env.COURIER_DISPATCH_MAX_ATTEMPTS || '5', 10);

    const client = await this.pool.connect();

    // The benign-race journal delete runs AFTER the main tx aborted (23505), so it needs its
    // own tenant-pinned transaction (the txn-local GUC died with the rollback).
    const deleteJournalRow = async () => {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
      await client.query('COMMIT');
    };

    try {
      await client.query('BEGIN');
      // B3: courier dispatch acts on one order → its location. Pin the courier-domain
      // tenant GUC so every courier_dispatch_queue / courier_shifts / courier_assignments
      // query below satisfies the Phase-1 app.current_tenant policies once dowiz_app loses
      // BYPASSRLS. Transaction-local — released at COMMIT/ROLLBACK.
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      // Fetch the queue item
      const queueRes = await client.query(
        `SELECT attempts FROM courier_dispatch_queue WHERE order_id = $1 FOR UPDATE`,
        [orderId]
      );

      if (queueRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return; // Already resolved or cancelled
      }

      // Q6 idempotency pre-check (ADR-dispatch-recovery): if the order is already actively
      // bound elsewhere or is terminal, the journal row is stale — delete it and stop.
      // The orders FOR UPDATE narrows the pre-check/INSERT TOCTOU (belt-and-suspenders;
      // the 23505-by-constraint catch below is the load-bearing fix).
      const ordRes = await client.query(
        `SELECT status FROM orders WHERE id = $1 FOR UPDATE`,
        [orderId]
      );
      const ordStatus = ordRes.rows[0]?.status;
      const activeRes = await client.query(
        `SELECT 1 FROM courier_assignments WHERE order_id = $1
           AND status IN ('offered','assigned','accepted','picked_up') LIMIT 1`,
        [orderId]
      );
      const TERMINAL = ['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP'];
      if (ordRes.rowCount === 0 || activeRes.rowCount > 0 || TERMINAL.includes(ordStatus)) {
        await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
        await client.query('COMMIT');
        return;
      }

      const attempts = queueRes.rows[0].attempts + 1;

      // Update attempt count
      await client.query(
        `UPDATE courier_dispatch_queue SET attempts = $1, last_attempt_at = now() WHERE order_id = $2`,
        [attempts, orderId]
      );

      // Attempt to find an available courier. The exclusion set mirrors the
      // courier_one_active_assignment partial unique (mig 073) INCLUDING 'offered' —
      // an offer-holding courier (shift still 'available' until accept) must never be
      // picked, else flag-ON dispatch 23505-loops forever. Correct in both flag states.
      const shiftRes = await client.query(
        `SELECT cs.courier_id, cs.id AS shift_id
         FROM courier_shifts cs
         WHERE cs.location_id = $1 AND cs.status = 'available'
           AND cs.courier_id NOT IN (
             SELECT courier_id FROM courier_assignments
             WHERE status IN ('offered','assigned','accepted','picked_up') AND courier_id IS NOT NULL
           )
         ORDER BY cs.last_heartbeat_at DESC NULLS LAST, cs.courier_id ASC
         LIMIT 1 FOR UPDATE SKIP LOCKED`,
        [locationId]
      );

      if (shiftRes.rowCount === 0) {
        // No couriers available
        if (attempts >= maxAttempts) {
          // Exhaustion tail — HONEST at both ends (ADR-dispatch-recovery, ETHICAL-STOP-1):
          // 1. persist the durable held / needs-attention marker on the ORDER first,
          // 2. only then delete the journal row (never erase the trace before it is durable),
          // 3. COMMIT, then publish post-commit — the wired bootstrap/messaging consumer
          //    alerts the owner (Telegram-ops) and sends the honest customer push.
          // Requires the orders.dispatch_exhausted_at column
          // (docs/proposals/dispatch-recovery-migration.ts — operator-placed).
          await client.query(
            `UPDATE orders SET dispatch_exhausted_at = now() WHERE id = $1`,
            [orderId]
          );
          await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
          await client.query('COMMIT');
          await this.messageBus.publish(BUS_CHANNELS.ORDER_DISPATCH_FAILED, { orderId, locationId, reason: 'No couriers available after max attempts' });
          return;
        }

        // Not exhausted: the attempts bump is committed and the journal row persists —
        // the 60s CourierOfferSweep pump is the sole retry cadence (self-retry deleted).
        await client.query('COMMIT');
        return;
      }

      const { courier_id, shift_id } = shiftRes.rows[0];

      // Assign!
      await client.query(
        `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status, assigned_at)
         VALUES ($1, $2, $3, $4, 'assigned', now())`,
        [orderId, locationId, courier_id, shift_id]
      );

      await client.query(
        `UPDATE courier_shifts SET status = 'on_delivery' WHERE id = $1`,
        [shift_id]
      );

      await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);

      await client.query('COMMIT');

      // Publish event
      await this.messageBus.publish(BUS_CHANNELS.ORDER_ASSIGNMENT_CREATED, { orderId, locationId, courierId: courier_id, shiftId: shift_id });

    } catch (err) {
      await client.query('ROLLBACK').catch(() => {});
      // 23505 special-cased BY CONSTRAINT (ADR-dispatch-recovery RESOLVE B-CONSIST):
      if (err?.code === '23505' && err?.constraint === 'courier_assignments_order_active_uniq') {
        // Order was bound by another path inside the TOCTOU window → resolved, benign.
        // Delete the journal row and return success: no throw → no pg-boss retry → no
        // re-pump → no false Recon O3 drift.
        await deleteJournalRow();
        return;
      }
      if (err?.code === '23505' && err?.constraint === 'courier_one_active_assignment') {
        // A picked courier raced into another binding — the order still needs a courier.
        // Keep the journal row; the next pump tick re-picks a different courier.
        return;
      }
      throw err;
    } finally {
      client.release();
    }
  }
}
