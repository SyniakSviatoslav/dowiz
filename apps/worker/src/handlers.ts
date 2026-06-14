import type { QueueProvider } from '@deliveryos/platform';
import type { Pool } from 'pg';
import { QUEUE_NAMES } from '@deliveryos/shared-types';

export function registerHandlers(queue: QueueProvider, pool: Pool) {

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
        console.log(`[Worker] Order ${orderId} auto-cancelled (timeout)`);
      } else {
        console.log(`[Worker] Order ${orderId} already transitioned, timeout no-op`);
      }
    } catch (err) {
      console.error(`[Worker] Error processing order.timeout for ${orderId}:`, err);
    }
  });
}
