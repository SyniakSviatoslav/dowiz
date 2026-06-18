import type { QueueProvider, MessageBus } from '@deliveryos/platform';
import type { Pool } from 'pg';
import { QUEUE_NAMES } from '@deliveryos/shared-types';

// Channel names mirror apps/api/src/lib/registry.ts (orderChannel / dashboardChannel).
const orderChannel = (id: string) => `order:${id}`;
const dashboardChannel = (id: string) => `location:${id}:dashboard`;

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
        console.log(`[Worker] Order ${orderId} auto-cancelled (timeout)`);
        const locationId = res.rows[0].location_id;
        const ts = new Date().toISOString();

        // Audit trail (best-effort — never block the broadcast on it).
        try {
          await pool.query(
            `INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor)
             VALUES ($1, $2, 'PENDING', 'CANCELLED', 'system:timeout')`,
            [orderId, locationId]
          );
        } catch (e) {
          console.error(`[Worker] order.timeout history insert failed for ${orderId}:`, e);
        }

        // Cross-surface live update: the customer status page + owner dashboard
        // must see the auto-cancel without a refresh (previously this handler was
        // silent, so a timed-out order only flipped on the next page load).
        await messageBus.publish(orderChannel(orderId), {
          type: 'order.status', orderId, status: 'CANCELLED', locationId, timestamp: ts,
        });
        await messageBus.publish(dashboardChannel(locationId), {
          type: 'order.status', data: { orderId, status: 'CANCELLED', statusUpdatedAt: ts },
        });
      } else {
        console.log(`[Worker] Order ${orderId} already transitioned, timeout no-op`);
      }
    } catch (err) {
      console.error(`[Worker] Error processing order.timeout for ${orderId}:`, err);
    }
  });

  // Post-delivery feedback nudge (~30 min after delivery, within the 24h window).
  // If the order is still unrated, push a reminder to the order channel so a client
  // that's still on the status page can prompt for a rating.
  // ponytail: WS nudge only — a real push would route via the notification pipeline.
  queue.work(QUEUE_NAMES.ORDER_FEEDBACK_REMINDER, async (payload: Record<string, unknown>) => {
    const { orderId } = payload;
    if (!orderId || typeof orderId !== 'string') return;
    try {
      const rated = await pool.query(`SELECT 1 FROM order_ratings WHERE order_id = $1`, [orderId]);
      if (rated.rowCount && rated.rowCount > 0) return; // already rated → no nudge
      await messageBus.publish(orderChannel(orderId), {
        type: 'order.feedback_reminder',
        payload: { orderId },
      });
    } catch (err) {
      console.error(`[Worker] feedback reminder failed for ${orderId}:`, err);
    }
  });
}
