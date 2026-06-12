// @ts-nocheck
import { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

export class CourierDispatchWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus
  ) {}

  async start() {
    // Register with singletonKey so only one N=2 instance processes at a time
    await this.boss.work(QUEUE_NAMES.COURIER_DISPATCH, { teamSize: 1, teamConcurrency: 1 }, async (job) => {
      const { orderId, locationId } = job.data as any;
      await this.handleDispatch(orderId, locationId);
    });
  }

  async handleDispatch(orderId: string, locationId: string) {
    const maxAttempts = parseInt(env.COURIER_DISPATCH_MAX_ATTEMPTS || '5', 10);
    const retryMs = parseInt(env.COURIER_DISPATCH_RETRY_MS || '30000', 10);

    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');

      // Fetch the queue item
      const queueRes = await client.query(
        `SELECT attempts FROM courier_dispatch_queue WHERE order_id = $1 FOR UPDATE`,
        [orderId]
      );

      if (queueRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return; // Already resolved or cancelled
      }

      const attempts = queueRes.rows[0].attempts + 1;

      // Update attempt count
      await client.query(
        `UPDATE courier_dispatch_queue SET attempts = $1, last_attempt_at = now() WHERE order_id = $2`,
        [attempts, orderId]
      );

      // Attempt to find an available courier
      const shiftRes = await client.query(
        `SELECT cs.courier_id, cs.id AS shift_id 
         FROM courier_shifts cs 
         WHERE cs.location_id = $1 AND cs.status = 'available' 
           AND cs.courier_id NOT IN (
             SELECT courier_id FROM courier_assignments 
             WHERE status IN ('assigned','accepted','picked_up') AND courier_id IS NOT NULL
           ) 
         ORDER BY cs.last_heartbeat_at DESC NULLS LAST, cs.courier_id ASC 
         LIMIT 1 FOR UPDATE SKIP LOCKED`,
        [locationId]
      );

      if (shiftRes.rowCount === 0) {
        // No couriers available
        if (attempts >= maxAttempts) {
          // Escalate failure
          await this.messageBus.publish(BUS_CHANNELS.ORDER_DISPATCH_FAILED, { orderId, locationId, reason: 'No couriers available after max attempts' });
          await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
          await client.query('COMMIT');
          return;
        }

        // Re-enqueue after retryMs
        await client.query('COMMIT');
        await this.boss.send(QUEUE_NAMES.COURIER_DISPATCH, { orderId, locationId }, { startAfter: Math.floor(retryMs / 1000) });
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
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  }
}
