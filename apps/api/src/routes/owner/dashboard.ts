import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { maskName, maskPhone } from '../../lib/pii-mask.js';
import { decryptPII } from '../../lib/pii-cipher.js';
import crypto from 'node:crypto';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { updateOrderStatus } from '../../lib/orderStatusService.js';
import { withTenant } from '@deliveryos/platform';

const VALID_STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED'] as const;

export default (async function ownerDashboardRoutes(fastify: any, opts: any) {
  const { db, messageBus, queue } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── Snapshot ──────────────────────────────────────────────────────
  fastify.get('/:locationId/dashboard/snapshot', {}, async (request: any, reply: any) => {
    const p = request.params as any;
    const q = request.query as any;
    const locationId = p.locationId;
    const status = q?.status as string | undefined;
    const limit = Math.min(Math.max(parseInt(q?.limit) || 100, 1), 100);
    const cursor = q?.cursor as string | undefined;

    const queryParams: any[] = [locationId];
    let statusFilter = '';
    if (status) {
      const statuses = status.split(',').map(s => s.trim().toUpperCase()).filter(s => VALID_STATUSES.includes(s as any));
      if (statuses.length > 0) {
        statusFilter = ` AND o.status IN (${statuses.map((_, i) => `$${i + 2}`).join(',')})`;
        queryParams.push(...statuses);
      }
    }

    let cursorClause = '';
    if (cursor) {
      try {
        const decoded = JSON.parse(Buffer.from(cursor, 'base64url').toString());
        if (decoded.createdAt) {
          cursorClause = ` AND o.created_at < $${queryParams.length + 1}`;
          queryParams.push(decoded.createdAt);
        }
      } catch (err: any) {
        console.warn('[dashboard] invalid cursor, ignoring:', err?.message);
      }
    }

    const limitIdx = queryParams.length + 1;
    queryParams.push(limit + 1);
    const userId = (request.user as any).userId;

    const orderSql = `
      SELECT o.id, o.status, o.total, o.currency_code, o.created_at, o.confirmed_at,
             o.subtotal, o.delivery_fee, o.payment_method, o.payment_outcome,
             o.metadata, o.preflight,
             c.name AS customer_name, c.phone AS customer_phone,
             (SELECT COUNT(*) FROM order_items oi WHERE oi.order_id = o.id) AS item_count
      FROM orders o
      LEFT JOIN customers c ON c.id = o.customer_id
      WHERE o.location_id = $1${statusFilter}${cursorClause}
      ORDER BY o.created_at DESC
      LIMIT $${limitIdx}
    `;

    const countSql = `
      SELECT o.status, COUNT(*)::int AS cnt
      FROM orders o
      WHERE o.location_id = $1
      GROUP BY o.status
    `;

    const [ordersRes, countsRes, deliveryRes] = await withTenant(db, userId, async (client) => Promise.all([
      client.query(orderSql, queryParams),
      client.query(countSql, [locationId]),
      client.query(`
        SELECT o.id AS order_id, o.status,
               a.courier_id, a.status AS assignment_status,
               c.full_name_encrypted, c.phone_encrypted,
               cp.lat, cp.lng, o.delivery_lat, o.delivery_lng,
               o.created_at
        FROM orders o
        JOIN courier_assignments a ON a.order_id = o.id AND a.status IN ('accepted', 'picked_up')
        JOIN couriers c ON c.id = a.courier_id
        LEFT JOIN LATERAL (
          SELECT lat, lng FROM courier_positions
          WHERE courier_id = a.courier_id
          ORDER BY recorded_at DESC LIMIT 1
        ) cp ON true
        WHERE o.location_id = $1 AND o.status IN ('IN_DELIVERY', 'READY')
        ORDER BY o.created_at DESC
      `, [locationId]),
    ]));

    const hasMore = ordersRes.rows.length > limit;
    const orders = (hasMore ? ordersRes.rows.slice(0, limit) : ordersRes.rows).map((row: any) => {
      let preflight = null;
      try { preflight = typeof row.preflight === 'string' ? JSON.parse(row.preflight) : row.preflight; } catch (err: any) {
        console.debug('[dashboard] failed to parse preflight for order', row.id, err?.message);
      }
      let metadata = null;
      try { metadata = typeof row.metadata === 'string' ? JSON.parse(row.metadata) : row.metadata; } catch (err: any) {
        console.debug('[dashboard] failed to parse metadata for order', row.id, err?.message);
      }
      return {
        orderId: row.id,
        status: row.status,
        total: row.total,
        currency: row.currency_code || 'ALL',
        createdAt: row.created_at,
        statusUpdatedAt: row.confirmed_at || row.created_at,
        customerNameMasked: maskName(row.customer_name),
        customerPhoneMasked: maskPhone(row.customer_phone),
        itemCount: row.item_count,
        paymentMethod: row.payment_method,
        paymentOutcome: row.payment_outcome,
        dwellSeconds: Math.floor((Date.now() - new Date(row.created_at).getTime()) / 1000),
        preflight,
        metadata,
      };
    });

    const counts: Record<string, number> = {};
    for (const s of VALID_STATUSES) counts[s] = 0;
    for (const row of countsRes.rows) counts[row.status] = row.cnt;

    const activeDeliveries = deliveryRes.rows.map((row: any) => {
      const name = row.full_name_encrypted ? decryptPII(row.full_name_encrypted) : null;
      const phone = row.phone_encrypted ? decryptPII(row.phone_encrypted) : null;
      let distanceKm: number | null = null;
      let etaSec: number | null = null;
      if (row.lat != null && row.lng != null && row.delivery_lat != null && row.delivery_lng != null) {
        distanceKm = haversineKm(
          { lat: Number(row.lat), lng: Number(row.lng) },
          { lat: Number(row.delivery_lat), lng: Number(row.delivery_lng) },
        );
        etaSec = distanceKm < 0.1 ? 60 : Math.round(distanceKm * 180);
      }
      return {
        orderId: row.order_id,
        status: row.status,
        courierName: maskName(name),
        courierPhone: maskPhone(phone),
        distanceToDestinationKm: distanceKm,
        etaSeconds: etaSec,
        pickedUpAt: row.created_at,
      };
    });

    const nextCursor = hasMore && orders.length > 0
      ? Buffer.from(JSON.stringify({ createdAt: orders.at(-1)!.createdAt })).toString('base64url')
      : null;

    // Count active dwell alerts + signals for this location
    const [alertCountRes, signalCountRes] = await withTenant(db, userId, async (client) => Promise.all([
      client.query(
        `SELECT COUNT(*)::int AS cnt FROM location_alerts
         WHERE location_id = $1 AND status = 'active' AND resolved_at IS NULL`,
        [locationId],
      ),
      client.query(
        `SELECT COUNT(*)::int AS cnt FROM customer_signals
         WHERE location_id = $1 AND acknowledged_at IS NULL AND dismissed_at IS NULL`,
        [locationId],
      ),
    ]));
    const activeAlertCount = alertCountRes.rows[0]?.cnt || 0;
    const activeSignalCount = signalCountRes.rows[0]?.cnt || 0;

    return reply.send({
      serverTime: new Date().toISOString(),
      counts,
      orders,
      activeDeliveries,
      nextCursor,
      activeAlertCount,
      activeSignalCount,
    });
  });

  // ─── Confirm ──────────────────────────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/confirm', {
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const user = request.user as any;
    const result = await transitionOrder(db, messageBus, orderId, locationId, user, 'CONFIRMED');
    return reply.status(200).send(result);
  });

  // ─── Reject ────────────────────────────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/reject', {
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const body = request.body as any;
    const reason = body?.reason as string | undefined;
    const user = request.user as any;
    const result = await transitionOrder(db, messageBus, orderId, locationId, user, 'REJECTED', reason);
    return reply.status(200).send(result);
  });

  // ─── Assign Courier ────────────────────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/assign-courier', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const body = request.body as any;
    const courierId = body?.courierId;
    if (!courierId || typeof courierId !== 'string') {
      return reply.status(400).send({ error: 'courierId is required' });
    }

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const orderCheck = await client.query(
        `SELECT id, status FROM orders WHERE id = $1 AND location_id = $2 FOR UPDATE`,
        [orderId, locationId],
      );
      if (orderCheck.rowCount === 0) { await client.query('ROLLBACK'); return reply.status(404).send({ error: 'Not found' }); }

      const order = orderCheck.rows[0];
      if (order.status !== 'CONFIRMED' && order.status !== 'PREPARING' && order.status !== 'READY') {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Order must be CONFIRMED, PREPARING, or READY to assign courier' });
      }

      const courierCheck = await client.query(
        `SELECT c.id FROM couriers c
         JOIN courier_locations cl ON cl.courier_id = c.id
         WHERE c.id = $1 AND cl.location_id = $2 AND c.status = 'active'`,
        [courierId, locationId],
      );
      if (courierCheck.rowCount === 0) { await client.query('ROLLBACK'); return reply.status(404).send({ error: 'Courier not found in this location' }); }

      const busyCheck = await client.query(
        `SELECT ca.id, ca.shift_id, ca.order_id, o.status AS order_status
         FROM courier_assignments ca
         JOIN orders o ON o.id = ca.order_id
         WHERE ca.courier_id = $1 AND ca.status IN ('accepted', 'picked_up') LIMIT 1`,
        [courierId],
      );
      if (busyCheck.rowCount > 0) {
        const old = busyCheck.rows[0];
        await client.query(
          `UPDATE courier_assignments SET status = 'cancelled', cancelled_at = now(), cancellation_reason = 'owner_reassigned'
           WHERE id = $1`,
          [old.id],
        );
        await client.query(
          `UPDATE courier_shifts SET status = 'available' WHERE id = $1`,
          [old.shift_id],
        );
        if (old.order_status === 'IN_DELIVERY') {
          await client.query(
          `UPDATE orders SET status = 'READY', courier_id = NULL WHERE id = $1`,
            [old.order_id],
          );
        }
        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
           VALUES ($1, $2, 'assignment.owner_reassigned', 'owner', $3)`,
          [courierId, locationId, (request as any).user.sub],
        );
      }

      let shiftCheck = await client.query(
        `SELECT id FROM courier_shifts
         WHERE courier_id = $1 AND status IN ('available', 'on_delivery') AND location_id = $2
         ORDER BY started_at DESC LIMIT 1`,
        [courierId, locationId],
      );
      let shiftId: string;
      if (shiftCheck.rowCount === 0) {
        const newShift = await client.query(
          `INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
           VALUES ($1, $2, 'on_delivery', now(), now())
           RETURNING id`,
          [courierId, locationId],
        );
        shiftId = newShift.rows[0].id;
        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
           VALUES ($1, $2, 'shift.auto_started', 'owner', $3)`,
          [courierId, locationId, (request as any).user.sub],
        );
      } else {
        shiftId = shiftCheck.rows[0].id;
      }
      const assignId = crypto.randomUUID();

      await client.query(
        `INSERT INTO courier_assignments (id, order_id, courier_id, shift_id, status, location_id)
         VALUES ($1, $2, $3, $4, 'accepted', $5)`,
        [assignId, orderId, courierId, shiftId, locationId],
      );

      await client.query(
        `UPDATE courier_shifts SET status = 'on_delivery' WHERE id = $1`,
        [shiftId],
      );

      // Canonical path: updateOrderStatus handles state machine + event publishing
      const { updateOrderStatus } = await import('../../lib/orderStatusService.js');
      await updateOrderStatus(client, orderId, locationId, 'IN_DELIVERY', { messageBus });
      // Set courier_id separately (not part of status transition)
      await client.query(
        `UPDATE orders SET courier_id = $1 WHERE id = $2`,
        [courierId, orderId],
      );

      await client.query('COMMIT');

      await messageBus.publish(orderChannel(orderId), { type: BUS_CHANNELS.ORDER_STATUS, orderId, status: 'IN_DELIVERY', locationId, timestamp: new Date().toISOString() });
      await messageBus.publish(`courier:${courierId}`, {
        type: 'task_assigned',
        payload: { id: orderId, orderId, status: 'assigned', courierId }
      });

      return reply.status(200).send({ id: assignId, orderId, courierId, status: 'assigned' });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // ─── Pickup (owner proxy for courier) ──────────────────────────────
  fastify.post('/:locationId/orders/:orderId/pickup', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const orderCheck = await client.query(
        `SELECT id, status FROM orders WHERE id = $1 AND location_id = $2 FOR UPDATE`,
        [orderId, locationId],
      );
      if (orderCheck.rowCount === 0) { await client.query('ROLLBACK'); return reply.status(404).send({ error: 'Not found' }); }
      if (orderCheck.rows[0].status !== 'IN_DELIVERY') {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Order must be IN_DELIVERY to pick up' });
      }

      const assignmentRes = await client.query(
        `SELECT id, courier_id, shift_id, status FROM courier_assignments
         WHERE order_id = $1 AND status IN ('accepted') FOR UPDATE`,
        [orderId],
      );
      if (assignmentRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'No accepted assignment found for this order' });
      }

      const { id: assignmentId, courierId, shiftId } = assignmentRes.rows[0];

      await client.query(
        `UPDATE courier_assignments SET status = 'picked_up', picked_up_at = now() WHERE id = $1`,
        [assignmentId],
      );

      await client.query(
        `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
         VALUES ($1, $2, 'order.picked_up', 'owner', $3)`,
        [courierId, locationId, (request as any).user.sub],
      );

      await client.query('COMMIT');

      await messageBus.publish(orderChannel(orderId), {
        type: BUS_CHANNELS.ORDER_PICKED_UP, orderId, locationId, timestamp: new Date().toISOString(),
      });
      await messageBus.publish(dashboardChannel(locationId), {
        type: 'order.status',
        data: { orderId, status: 'PICKED_UP', statusUpdatedAt: new Date().toISOString() },
      });

      return reply.send({
        success: true,
        assignmentId,
        orderId,
        courierId,
        status: 'picked_up',
      });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // ─── Deliver (owner proxy for courier) ─────────────────────────────
  fastify.post('/:locationId/orders/:orderId/deliver', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const body = request.body as any;
    const cashCollected = body?.cash_collected ?? true;
    const cashAmount = body?.cash_amount;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const orderCheck = await client.query(
        `SELECT id, status, total FROM orders WHERE id = $1 AND location_id = $2 FOR UPDATE`,
        [orderId, locationId],
      );
      if (orderCheck.rowCount === 0) { await client.query('ROLLBACK'); return reply.status(404).send({ error: 'Not found' }); }
      if (orderCheck.rows[0].status !== 'IN_DELIVERY') {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Order must be IN_DELIVERY to deliver' });
      }

      const finalCashAmount = cashAmount ?? orderCheck.rows[0].total;

      const assignmentRes = await client.query(
        `SELECT id, courier_id, shift_id, status FROM courier_assignments
         WHERE order_id = $1 AND status IN ('accepted', 'picked_up') FOR UPDATE`,
        [orderId],
      );
      if (assignmentRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'No active assignment found for this order' });
      }

      const { id: assignmentId, courierId, shiftId } = assignmentRes.rows[0];

      await client.query(
        `UPDATE courier_assignments
         SET status = 'delivered', delivered_at = now(), cash_collected = $1, cash_amount = $2
         WHERE id = $3`,
        [cashCollected, cashCollected ? finalCashAmount : null, assignmentId],
      );

      await client.query(
        `UPDATE courier_shifts SET status = 'available' WHERE id = $1`,
        [shiftId],
      );

      await updateOrderStatus(client, orderId, locationId, 'DELIVERED' as any, { messageBus });

      await client.query(
        `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
         VALUES ($1, $2, 'order.delivered', 'owner', $3)`,
        [courierId, locationId, (request as any).user.sub],
      );

      await client.query('COMMIT');

      return reply.send({
        success: true,
        assignmentId,
        orderId,
        courierId,
        status: 'delivered',
        cashCollected,
        cashAmount: finalCashAmount,
      });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // ─── Verify order state ────────────────────────────────────────────
  fastify.get('/:locationId/orders/:orderId/verify', {
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const userId = (request.user as any).userId;

    const [orderRes, itemsRes, assignmentsRes, auditRes] = await withTenant(db, userId, async (client) => Promise.all([
      client.query(`
        SELECT o.id, o.status, o.created_at, o.confirmed_at, o.ready_at, o.delivered_at,
               o.total, o.subtotal, o.delivery_fee, o.currency_code,
               o.payment_method, o.payment_outcome,
               o.courier_id, o.location_id,
               c.name AS customer_name, c.phone AS customer_phone
        FROM orders o
        LEFT JOIN customers c ON c.id = o.customer_id
        WHERE o.id = $1 AND o.location_id = $2
      `, [orderId, locationId]),
      client.query(`
        SELECT oi.id, oi.product_id, oi.quantity, oi.price_snapshot AS unit_price, (oi.price_snapshot * oi.quantity) AS subtotal,
               p.name AS product_name, p.price AS product_current_price
        FROM order_items oi
        LEFT JOIN products p ON p.id = oi.product_id
        WHERE oi.order_id = $1
      `, [orderId]),
      client.query(`
        SELECT a.id, a.order_id, a.courier_id, a.shift_id, a.status,
               a.assigned_at, a.accepted_at, a.picked_up_at, a.delivered_at,
               a.cash_collected, a.cash_amount,
               c.full_name_encrypted, c.phone_encrypted, c.status AS courier_status,
               s.status AS shift_status, s.started_at AS shift_started_at
        FROM courier_assignments a
        JOIN couriers c ON c.id = a.courier_id
        LEFT JOIN courier_shifts s ON s.id = a.shift_id
        WHERE a.order_id = $1
        ORDER BY a.created_at DESC
      `, [orderId]),
      client.query(`
        SELECT action, actor_kind, actor_id, created_at
        FROM courier_audit_log
        WHERE courier_id IN (
          SELECT courier_id FROM courier_assignments WHERE order_id = $1
        )
        ORDER BY created_at ASC
      `, [orderId]),
    ]));

    if (orderRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const order = orderRes.rows[0];
    const items = itemsRes.rows;
    const assignments = assignmentsRes.rows.map((a: any) => {
      const courierName = a.full_name_encrypted ? decryptPII(a.full_name_encrypted) : null;
      const courierPhone = a.phone_encrypted ? decryptPII(a.phone_encrypted) : null;
      return {
        id: a.id,
        order_id: a.order_id,
        courier_id: a.courier_id,
        shift_id: a.shift_id,
        status: a.status,
        assigned_at: a.assigned_at,
        accepted_at: a.accepted_at,
        picked_up_at: a.picked_up_at,
        delivered_at: a.delivered_at,
        cash_collected: a.cash_collected,
        cash_amount: a.cash_amount,
        courier_name_masked: courierName ? maskName(courierName) : null,
        courier_phone_masked: courierPhone ? maskPhone(courierPhone) : null,
        courier_status: a.courier_status,
        shift_status: a.shift_status,
        shift_started_at: a.shift_started_at,
      };
    });
    const auditLogs = auditRes.rows;

    return reply.send({
      order: {
        ...order,
        customer_name_masked: maskName(order.customer_name),
        customer_phone_masked: maskPhone(order.customer_phone),
      },
      items,
      assignments,
      auditLogs,
    });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;

async function transitionOrder(db: any, messageBus: any, orderId: string, locationId: string, user: any, newStatus: string, reason?: string) {
  // Run under the owner's tenant context so RLS on `orders` (keyed to
  // app.user_id → member locations) acts as a backstop, and scope the order
  // read to the URL location the caller was authorized for. Without both, an
  // owner of location A could transition another tenant's order by id (IDOR).
  return withTenant(db, user.userId, async (client) => {
    const cur = await client.query(
      `SELECT id, status FROM orders WHERE id = $1 AND location_id = $2`,
      [orderId, locationId],
    );
    if (!cur.rowCount) throw { statusCode: 404, error: 'Order not found' };

    // Canonical path: updateOrderStatus handles state machine, anti-race, and events.
    await updateOrderStatus(client, orderId, locationId, newStatus as any, { messageBus });

    // Handle rejection_reason separately (updateOrderStatus doesn't set it)
    if (reason && newStatus === 'REJECTED') {
      await client.query(
        `UPDATE orders SET rejection_reason = $1 WHERE id = $2 AND location_id = $3`,
        [reason, orderId, locationId],
      );
    }

    return { id: orderId, status: newStatus, statusUpdatedAt: new Date().toISOString() };
  });
}

function haversineKm(a: { lat: number; lng: number }, b: { lat: number; lng: number }): number {
  const R = 6371;
  const dLat = (b.lat - a.lat) * (Math.PI / 180);
  const dLng = (b.lng - a.lng) * (Math.PI / 180);
  const ha = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * (Math.PI / 180)) * Math.cos(b.lat * (Math.PI / 180)) * Math.sin(dLng / 2) ** 2;
  return R * 2 * Math.atan2(Math.sqrt(ha), Math.sqrt(1 - ha));
}
