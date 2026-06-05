// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { maskName, maskPhone } from '../../lib/pii-mask.js';
import crypto from 'node:crypto';

const VALID_STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED'] as const;

export default (async function ownerDashboardRoutes(fastify, opts) {
  const { db, messageBus, queue } = opts as any;

  fastify.addHook('onRequest', async (request, reply) => {
    try {
      await request.jwtVerify();
      const user = request.user as any;
      if (user.role !== 'owner') return reply.status(403).send({ error: 'Owner only' });
      const { locationId } = request.params as any;
      if (!locationId || !user.activeLocationId || user.activeLocationId !== locationId) return reply.status(404).send({ error: 'Not found' });
    } catch {
      return reply.status(401).send({ error: 'Unauthorized' });
    }
  });

  // ─── Snapshot ──────────────────────────────────────────────────────
  fastify.get('/:locationId/dashboard/snapshot', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        status: z.string().optional(),
        limit: z.coerce.number().int().min(1).max(100).default(100),
        cursor: z.string().optional(),
      }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { status, limit, cursor } = request.query;

    const params: any[] = [locationId];
    let statusFilter = '';
    if (status) {
      const statuses = status.split(',').map(s => s.trim().toUpperCase()).filter(s => VALID_STATUSES.includes(s as any));
      if (statuses.length > 0) {
        statusFilter = ` AND o.status IN (${statuses.map((_, i) => `$${i + 2}`).join(',')})`;
        params.push(...statuses);
      }
    }

    let cursorClause = '';
    if (cursor) {
      try {
        const decoded = JSON.parse(Buffer.from(cursor, 'base64url').toString());
        if (decoded.createdAt) {
          cursorClause = ` AND o.created_at < $${params.length + 1}`;
          params.push(decoded.createdAt);
        }
      } catch { /* invalid cursor */ }
    }

    const limitIdx = params.length + 1;
    params.push(limit + 1);

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

    const [ordersRes, countsRes, deliveryRes] = await Promise.all([
      db.query(orderSql, params),
      db.query(countSql, [locationId]),
      db.query(`
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
    ]);

    const hasMore = ordersRes.rows.length > limit;
    const orders = (hasMore ? ordersRes.rows.slice(0, limit) : ordersRes.rows).map((row: any) => {
      let preflight = null;
      try { preflight = typeof row.preflight === 'string' ? JSON.parse(row.preflight) : row.preflight; } catch {}
      let metadata = null;
      try { metadata = typeof row.metadata === 'string' ? JSON.parse(row.metadata) : row.metadata; } catch {}
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

    const { decryptPII } = await import('../../lib/pii-cipher.js');

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
      ? Buffer.from(JSON.stringify({ createdAt: orders[orders.length - 1].createdAt })).toString('base64url')
      : null;

    // Count active dwell alerts + signals for this location
    const alertCountRes = await db.query(
      `SELECT COUNT(*)::int AS cnt FROM location_alerts
       WHERE location_id = $1 AND status = 'active' AND resolved_at IS NULL`,
      [locationId],
    );
    const activeAlertCount = alertCountRes.rows[0]?.cnt || 0;

    const signalCountRes = await db.query(
      `SELECT COUNT(*)::int AS cnt FROM customer_signals
       WHERE location_id = $1 AND acknowledged_at IS NULL AND dismissed_at IS NULL`,
      [locationId],
    );
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
  }, async (request, reply) => {
    const { orderId } = request.params as any;
    const user = request.user as any;
    const result = await transitionOrder(db, messageBus, orderId, user, 'CONFIRMED');
    return reply.status(200).send(result);
  });

  // ─── Reject ────────────────────────────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/reject', {
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } },
    schema: {
      body: z.object({ reason: z.string().max(500).optional() }),
    },
  }, async (request, reply) => {
    const { orderId } = request.params as any;
    const { reason } = request.body;
    const user = request.user as any;
    const result = await transitionOrder(db, messageBus, orderId, user, 'REJECTED', reason);
    return reply.status(200).send(result);
  });

  // ─── Assign Courier ────────────────────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/assign-courier', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
    schema: {
      body: z.object({ courierId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId, orderId } = request.params as any;
    const { courierId } = request.body;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const orderCheck = await client.query(
        `SELECT id, status FROM orders WHERE id = $1 AND location_id = $2 FOR UPDATE`,
        [orderId, locationId],
      );
      if (orderCheck.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

      const order = orderCheck.rows[0];
      if (order.status !== 'CONFIRMED' && order.status !== 'PREPARING') {
        return reply.status(409).send({ error: 'Order must be CONFIRMED or PREPARING to assign courier' });
      }

      const courierCheck = await client.query(
        `SELECT c.id FROM couriers c
         JOIN courier_locations cl ON cl.courier_id = c.id
         WHERE c.id = $1 AND cl.location_id = $2 AND c.status = 'active'`,
        [courierId, locationId],
      );
      if (courierCheck.rowCount === 0) return reply.status(404).send({ error: 'Courier not found in this location' });

      const busyCheck = await client.query(
        `SELECT id FROM courier_assignments
         WHERE courier_id = $1 AND status IN ('accepted', 'picked_up') LIMIT 1`,
        [courierId],
      );
      if (busyCheck.rowCount > 0) return reply.status(409).send({ error: 'Courier is already on a delivery' });

      const shiftCheck = await client.query(
        `SELECT id FROM courier_shifts
         WHERE courier_id = $1 AND status = 'available' AND location_id = $2
         ORDER BY started_at DESC LIMIT 1`,
        [courierId, locationId],
      );
      if (shiftCheck.rowCount === 0) return reply.status(409).send({ error: 'Courier has no active shift' });

      const shiftId = shiftCheck.rows[0].id;
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

      await client.query(
        `UPDATE orders SET status = 'IN_DELIVERY', courier_id = $1 WHERE id = $2`,
        [courierId, orderId],
      );

      await client.query('COMMIT');

      await messageBus.publish(`order:${orderId}`, { type: 'order.status', orderId, status: 'IN_DELIVERY', locationId, timestamp: new Date().toISOString() });
      await messageBus.publish(`location:${locationId}:dashboard`, {
        type: 'courier.assignment_created',
        data: { orderId, courierId },
      });

      return reply.status(200).send({ id: assignId, orderId, courierId, status: 'assigned' });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;

async function transitionOrder(db: any, messageBus: any, orderId: string, user: any, newStatus: string, reason?: string) {
  const client = await db.connect();
  try {
    const cur = await client.query(
      `SELECT id, status, location_id FROM orders WHERE id = $1`,
      [orderId],
    );
    if (!cur.rowCount) throw { statusCode: 404, error: 'Order not found' };

    const currentStatus = cur.rows[0].status;
    const locationId = cur.rows[0].location_id;

    if (!user.activeLocationId || user.activeLocationId !== locationId) {
      throw { statusCode: 404, error: 'Not found' };
    }

    const acceptedFrom = newStatus === 'CONFIRMED' ? ['PENDING'] : ['PENDING', 'CONFIRMED', 'PREPARING', 'SCHEDULED'];
    if (!acceptedFrom.includes(currentStatus)) {
      throw { statusCode: 409, error: `Cannot transition from ${currentStatus} to ${newStatus}` };
    }

    let res;
    if (newStatus === 'CONFIRMED') {
      res = await client.query(
        `UPDATE orders SET status = $1, confirmed_at = now(), timeout_at = NULL
         WHERE id = $2 AND status = $3 RETURNING id`,
        [newStatus, orderId, currentStatus],
      );
    } else {
      const reasonSet = reason ? `, rejection_reason = $4` : '';
      const rParams = reason ? [newStatus, orderId, currentStatus, reason] : [newStatus, orderId, currentStatus];
      res = await client.query(
        `UPDATE orders SET status = $1${reasonSet} WHERE id = $2 AND status = $3 RETURNING id`,
        rParams,
      );
    }

    if (!res.rowCount) throw { statusCode: 409, error: 'Order status already changed', code: 'CONFLICT' };

    await messageBus.publish(`order:${orderId}`, { type: 'order.status', orderId, status: newStatus, locationId, timestamp: new Date().toISOString() });
    await messageBus.publish(`location:${locationId}:dashboard`, {
      type: `order.${newStatus.toLowerCase()}`,
      data: { orderId, status: newStatus, statusUpdatedAt: new Date().toISOString() },
    });

    return { id: orderId, status: newStatus, statusUpdatedAt: new Date().toISOString() };
  } finally {
    client.release();
  }
}

function haversineKm(a: { lat: number; lng: number }, b: { lat: number; lng: number }): number {
  const R = 6371;
  const dLat = (b.lat - a.lat) * (Math.PI / 180);
  const dLng = (b.lng - a.lng) * (Math.PI / 180);
  const ha = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * (Math.PI / 180)) * Math.cos(b.lat * (Math.PI / 180)) * Math.sin(dLng / 2) ** 2;
  return R * 2 * Math.atan2(Math.sqrt(ha), Math.sqrt(1 - ha));
}
