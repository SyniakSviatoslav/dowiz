// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { maskName, maskPhone } from '../../lib/pii-mask.js';
import { computeSignals } from '../../lib/signals/compute.js';

const KIND_VALUES = ['no_show_recent', 'velocity_rapid', 'velocity_high_volume', 'ip_velocity_rapid', 'ip_velocity_high_volume', 'manual_flag'] as const;

export default (async function ownerSignalRoutes(fastify, opts) {
  const { db, messageBus } = opts as any;

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

  // ─── List Signals ────────────────────────────────────────────────
  fastify.get('/:locationId/signals', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        status: z.enum(['active', 'acknowledged', 'dismissed']).optional(),
        kind: z.enum(KIND_VALUES).optional(),
        limit: z.coerce.number().int().min(1).max(100).default(50),
        cursor: z.string().optional(),
      }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { status, kind, limit, cursor } = request.query;

    const params: any[] = [locationId];
    let clauses = 'WHERE cs.location_id = $1';

    if (status === 'active') {
      clauses += ' AND cs.acknowledged_at IS NULL AND cs.dismissed_at IS NULL';
    } else if (status === 'acknowledged') {
      clauses += ' AND cs.acknowledged_at IS NOT NULL';
    } else if (status === 'dismissed') {
      clauses += ' AND cs.dismissed_at IS NOT NULL';
    }

    if (kind) {
      params.push(kind);
      clauses += ` AND cs.kind = $${params.length}`;
    }

    if (cursor) {
      try {
        const decoded = JSON.parse(Buffer.from(cursor, 'base64url').toString());
        if (decoded.raisedAt) {
          params.push(decoded.raisedAt);
          clauses += ` AND cs.raised_at < $${params.length}`;
        }
      } catch { /* ignore */ }
    }

    const limitIdx = params.length + 1;
    params.push(limit + 1);

    const res = await db.query(`
      SELECT cs.id, cs.customer_id, cs.kind, cs.severity, cs.evidence,
             cs.raised_at, cs.acknowledged_at, cs.dismissed_at,
             c.name AS customer_name, c.phone AS customer_phone
      FROM customer_signals cs
      LEFT JOIN customers c ON c.id = cs.customer_id
      ${clauses}
      ORDER BY cs.raised_at DESC
      LIMIT $${limitIdx}
    `, params);

    const hasMore = res.rows.length > limit;
    const signals = (hasMore ? res.rows.slice(0, limit) : res.rows).map((row: any) => ({
      id: row.id,
      customerId: row.customer_id,
      kind: row.kind,
      severity: row.severity,
      evidence: row.evidence,
      raisedAt: row.raised_at,
      acknowledgedAt: row.acknowledged_at,
      dismissedAt: row.dismissed_at,
      customerNameMasked: maskName(row.customer_name),
      customerPhoneMasked: maskPhone(row.customer_phone),
    }));

    const nextCursor = hasMore && signals.length > 0
      ? Buffer.from(JSON.stringify({ raisedAt: signals[signals.length - 1].raisedAt })).toString('base64url')
      : null;

    return reply.send({ signals, nextCursor });
  });

  // ─── Compute Signals (read-only) ────────────────────────────────
  fastify.get('/:locationId/signals/compute', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        phone_hash: z.string().regex(/^[a-f0-9]{64}$/).optional(),
        ip_hash: z.string().regex(/^[a-f0-9]{64}$/).optional(),
        customer_id: z.string().uuid().optional(),
      }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { phone_hash, ip_hash, customer_id } = request.query;

    const signals = await computeSignals(db, {
      locationId,
      phoneHash: phone_hash,
      clientIpHash: ip_hash,
      customerId: customer_id,
    });

    return reply.send({ signals, computedAt: new Date().toISOString() });
  });

  // ─── Acknowledge Signal ──────────────────────────────────────────
  fastify.post('/:locationId/signals/:signalId/acknowledge', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), signalId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId, signalId } = request.params;
    const user = request.user as any;

    const res = await db.query(
      `UPDATE customer_signals
       SET acknowledged_at = now(), acknowledged_by_owner_id = $1
       WHERE id = $2 AND location_id = $3
         AND acknowledged_at IS NULL AND dismissed_at IS NULL
       RETURNING id, customer_id, kind`,
      [user.userId, signalId, locationId],
    );

    if (res.rowCount === 0) return reply.status(404).send({ error: 'Signal not found or already resolved' });

    // Shift last_no_show_at by -7 days (forgive)
    if (res.rows[0].kind === 'no_show_recent') {
      await db.query(
        `UPDATE customers
         SET last_no_show_at = GREATEST(last_no_show_at - interval '7 days', now() - interval '90 days')
         WHERE id = $1 AND last_no_show_at IS NOT NULL`,
        [res.rows[0].customer_id],
      );
    }

    await messageBus.publish(`location:${locationId}:dashboard`, {
      type: 'preflight.signal_acknowledged',
      data: { signalId, customerId: res.rows[0].customer_id, kind: res.rows[0].kind },
    });

    return reply.send({ id: signalId, acknowledgedAt: new Date().toISOString() });
  });

  // ─── Dismiss Signal ──────────────────────────────────────────────
  fastify.post('/:locationId/signals/:signalId/dismiss', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), signalId: z.string().uuid() }),
      body: z.object({ reason: z.string().max(500).optional() }).strict(),
    },
  }, async (request, reply) => {
    const { locationId, signalId } = request.params;
    const body = request.body || {};
    const user = request.user as any;

    const res = await db.query(
      `UPDATE customer_signals
       SET dismissed_at = now(), dismissed_by_owner_id = $1,
           evidence = evidence || $2::jsonb
       WHERE id = $3 AND location_id = $4
         AND dismissed_at IS NULL
       RETURNING id`,
      [user.userId, JSON.stringify({ dismissReason: body.reason || null, dismissedBy: user.userId }), signalId, locationId],
    );

    if (res.rowCount === 0) return reply.status(404).send({ error: 'Signal not found or already dismissed' });

    await messageBus.publish(`location:${locationId}:dashboard`, {
      type: 'preflight.signal_dismissed',
      data: { signalId, dismissedAt: new Date().toISOString() },
    });

    return reply.send({ id: signalId, dismissedAt: new Date().toISOString() });
  });

  // ─── Mark No-Show (owner manual) ────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/mark-no-show', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), orderId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId, orderId } = request.params;
    const user = request.user as any;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // Find customer and verify order belongs to this location
      const orderRes = await client.query(
        `SELECT customer_id, status FROM orders WHERE id = $1 AND location_id = $2 FOR UPDATE`,
        [orderId, locationId],
      );
      if (orderRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'Order not found' });
      }
      const { customer_id, status } = orderRes.rows[0];

      if (!customer_id) {
        await client.query('ROLLBACK');
        return reply.status(400).send({ error: 'Order has no customer' });
      }

      // Increment no-show counters
      await client.query(
        `UPDATE customers
         SET no_show_count = no_show_count + 1,
             last_no_show_at = now()
         WHERE id = $1`,
        [customer_id],
      );

      // Update order status to CANCELLED with reason
      await client.query(
        `UPDATE orders SET status = 'CANCELLED', status_notes = 'no_show' WHERE id = $1`,
        [orderId],
      );

      await client.query('COMMIT');

      await messageBus.publish(`order.cancelled`, { orderId, locationId, reason: 'no_show' });
      await messageBus.publish(`customer.no_show_incremented`, { customerId: customer_id, orderId, locationId });

      return reply.send({ success: true, customerId: customer_id });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
