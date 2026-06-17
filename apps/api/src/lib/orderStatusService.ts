import type { PoolClient } from 'pg';
import { assertTransition, type OrderStatus } from '@deliveryos/domain';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, orderChannel, dashboardChannel } from './registry.js';

async function fetchOrderDelta(client: PoolClient, orderId: string) {
  const res = await client.query(`
    SELECT o.id, o.status, o.total, o.created_at, loc.currency_code,
      (SELECT count(*) FROM order_items oi WHERE oi.order_id = o.id)::int as item_count,
      (SELECT string_agg(oi.quantity::text || '\u00d7' || oi.name_snapshot, ', ')
       FROM order_items oi WHERE oi.order_id = o.id) as items_summary
    FROM orders o
    LEFT JOIN locations loc ON loc.id = o.location_id
    WHERE o.id = $1
  `, [orderId]);
  const row = res.rows[0];
  if (!row) return null;
  return {
    orderId: row.id,
    status: row.status,
    total: row.total,
    currency: row.currency_code || 'ALL',
    createdAt: row.created_at,
    shortId: '#' + row.id.substring(0, 4).toUpperCase(),
    itemCount: row.item_count || 0,
    itemsSummary: row.items_summary || '',
    courierName: null,
  };
}

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
       WHERE id = $2 AND status = $3 AND location_id = $4 RETURNING id`,
      [newStatus, orderId, currentStatus, locationId]
    );
  } else {
    res = await client.query(
      `UPDATE orders SET status = $1, timeout_at = NULL
       WHERE id = $2 AND status = $3 AND location_id = $4 RETURNING id`,
      [newStatus, orderId, currentStatus, locationId]
    );
  }

  if (!res.rowCount || res.rowCount === 0) {
    throw { statusCode: 409, error: 'Order status already changed', code: 'CONFLICT' };
  }

  // 3b. Write audit trail
  await client.query(
    `INSERT INTO order_status_history (order_id, location_id, from_status, to_status)
     VALUES ($1, $2, $3, $4)`,
    [orderId, locationId, currentStatus, newStatus]
  );

  // 4. Broadcast via MessageBus
  await opts.messageBus.publish(orderChannel(orderId), {
    type: 'order.status',
    orderId,
    status: newStatus,
    locationId: cur.rows[0].location_id,
    timestamp: new Date().toISOString(),
  });

  const dbLocationId = cur.rows[0].location_id;

  // Forward to dashboard room for live owner dashboard
  // Uses 'order.status' type — FE merges by id, no full GET needed
  if (dbLocationId) {
    const delta = await fetchOrderDelta(client, orderId);
    if (delta) {
      await opts.messageBus.publish(dashboardChannel(dbLocationId), {
        type: 'order.status',
        data: { ...delta, statusUpdatedAt: new Date().toISOString() },
      });
    }
  }

  // 5. Publish lifecycle event for notification fan-out
  if (newStatus === 'CONFIRMED' && dbLocationId) {
    await opts.messageBus.publish(BUS_CHANNELS.ORDER_CONFIRMED, { orderId, locationId: dbLocationId });
  } else if (newStatus === 'REJECTED' && dbLocationId) {
    await opts.messageBus.publish(BUS_CHANNELS.ORDER_REJECTED, { orderId, locationId: dbLocationId });
  }

  // 6. Notify assigned courier when order is READY for pickup
  if (newStatus === 'READY') {
    const courierRes = await client.query(
      `SELECT courier_id FROM orders WHERE id = $1`,
      [orderId]
    );
    const courierId = courierRes.rows[0]?.courier_id;
    if (courierId) {
      await opts.messageBus.publish(`courier:${courierId}`, {
        type: 'order.ready',
        orderId,
        locationId: cur.rows[0].location_id,
        timestamp: new Date().toISOString(),
      });
    }
  }
}