import type { MessageBus, QueueProvider } from '@deliveryos/platform';
import type { Pool } from 'pg';
import { QUEUE_NAMES } from '@deliveryos/shared-types';

export function registerHandlers(queue: QueueProvider, pool: Pool, messageBus: MessageBus) {

  queue.work('health-job', async (payload: Record<string, unknown>) => {
    console.log(`[Worker] Processed health-job at ${new Date().toISOString()}`, payload);
  });

  queue.work(QUEUE_NAMES.ORDER_TIMEOUT, async (payload: Record<string, unknown>) => {
    const { orderId } = payload;
    if (!orderId || typeof orderId !== 'string') {
      console.error('[Worker] order.timeout missing orderId in payload');
      return;
    }

    console.log(`[Worker] Processing order.timeout for order ${orderId}`);

    try {
      const res = await pool.query(
        `UPDATE orders SET status = 'CANCELLED', timeout_at = NULL
         WHERE id = $1 AND status = 'PENDING'
         RETURNING id, status, location_id`,
        [orderId]
      );

      if (res.rowCount && res.rowCount > 0) {
        const { location_id } = res.rows[0];

        await pool.query(
          `INSERT INTO order_status_history (order_id, location_id, from_status, to_status, created_at)
           VALUES ($1, $2, 'PENDING', 'CANCELLED', now())`,
          [orderId, location_id]
        );

        await messageBus.publish(`order:${orderId}`, {
          type: 'order.status',
          orderId,
          status: 'CANCELLED',
          locationId: location_id,
          timestamp: new Date().toISOString(),
        });

        await messageBus.publish(`location:${location_id}:dashboard`, {
          type: 'order.status',
          data: { orderId, status: 'CANCELLED', statusUpdatedAt: new Date().toISOString() },
        });

        console.log(`[Worker] Order ${orderId} auto-cancelled (timeout)`);
      } else {
        console.log(`[Worker] Order ${orderId} already transitioned, timeout no-op`);
      }
    } catch (err) {
      console.error(`[Worker] Error processing order.timeout for ${orderId}:`, err);
    }
  });

  queue.work(QUEUE_NAMES.ORDER_FEEDBACK_REMINDER, async (payload: Record<string, unknown>) => {
    const { orderId } = payload;
    if (!orderId || typeof orderId !== 'string') return;
    console.log(`[Worker] ORDER_FEEDBACK_REMINDER for order ${orderId}`);
    // TODO: dispatch push notification / email to customer
  });
}
