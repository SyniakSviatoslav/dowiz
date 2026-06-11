import type { PoolClient } from 'pg';
import { assertTransition, type OrderStatus } from '@deliveryos/domain';
import type { MessageBus } from '@deliveryos/platform';

export async function updateOrderStatus(
  client: PoolClient,
  orderId: string,
  locationId: string,
  newStatus: OrderStatus,
  opts: { messageBus: MessageBus }
): Promise<void> {
  // 1. Read current status
  const cur = await client.query(
    `SELECT id, status, location_id FROM orders WHERE id = $1`,
    [orderId]
  );

  if (!cur.rowCount || cur.rowCount === 0) {
    throw { statusCode: 404, error: 'Order not found' };
  }

  const currentStatus: string = cur.rows[0].status;

  // 2. State machine validation (before SQL)
  try {
    assertTransition(currentStatus as OrderStatus, newStatus);
  } catch (e: unknown) {
    const err = e as Error;
    if (err.name === 'IllegalTransitionError' || err.name === 'ScaffoldDisabledError') {
      throw { statusCode: 400, error: err.message, code: err.name };
    }
    if (err.name === 'SameStatusError') {
      throw { statusCode: 400, error: err.message, code: err.name };
    }
    throw e;
  }

  // 3. Status-guarded UPDATE (anti-race)
  let res;
  if (newStatus === 'CONFIRMED') {
    res = await client.query(
      `UPDATE orders SET status = $1, confirmed_at = now(), timeout_at = NULL
       WHERE id = $2 AND status = $3 RETURNING id`,
      [newStatus, orderId, currentStatus]
    );
  } else {
    res = await client.query(
      `UPDATE orders SET status = $1, timeout_at = NULL
       WHERE id = $2 AND status = $3 RETURNING id`,
      [newStatus, orderId, currentStatus]
    );
  }

  if (!res.rowCount || res.rowCount === 0) {
    throw { statusCode: 409, error: 'Order status already changed', code: 'CONFLICT' };
  }

  // 4. Broadcast via MessageBus
  await opts.messageBus.publish(`order:${orderId}`, {
    type: 'order.status',
    orderId,
    status: newStatus,
    locationId: cur.rows[0].location_id,
    timestamp: new Date().toISOString(),
  });

  // Forward to dashboard room for live owner dashboard
  if (cur.rows[0].location_id) {
    await opts.messageBus.publish(`location:${cur.rows[0].location_id}:dashboard`, {
      type: `order.${newStatus.toLowerCase()}`,
      data: { orderId, status: newStatus, statusUpdatedAt: new Date().toISOString() },
    });
  }
}