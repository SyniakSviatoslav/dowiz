// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { maskName, maskPhone } from '../../lib/pii-mask.js';

export default (async function ownerAlertRoutes(fastify, opts) {
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

  // ─── List Alerts ──────────────────────────────────────────────────
  fastify.get('/:locationId/alerts', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        status: z.enum(['active', 'resolved']).optional(),
        kind: z.string().optional(),
        limit: z.coerce.number().int().min(1).max(100).default(50),
        cursor: z.string().optional(),
      }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { status, kind, limit, cursor } = request.query;

    const params: any[] = [locationId];
    let clauses = 'WHERE la.location_id = $1';

    if (status === 'active') {
      clauses += ' AND la.status = \'active\' AND la.resolved_at IS NULL';
    } else if (status === 'resolved') {
      clauses += ' AND la.resolved_at IS NOT NULL AND la.resolved_at > now() - interval \'24 hours\'';
    }

    if (kind) {
      params.push(kind);
      clauses += ` AND la.kind = $${params.length}`;
    }

    if (cursor) {
      try {
        const decoded = JSON.parse(Buffer.from(cursor, 'base64url').toString());
        if (decoded.createdAt) {
          params.push(decoded.createdAt);
          clauses += ` AND la.created_at < $${params.length}`;
        }
      } catch {
        // invalid cursor — ignore, will use no cursor filter
        console.debug('[alerts] invalid cursor, ignoring');
      }
    }

    const limitIdx = params.length + 1;
    params.push(limit + 1);

    const res = await db.query(`
      SELECT la.id, la.order_id, la.kind, la.status, la.created_at, la.resolved_at,
             la.acknowledged_at, la.escalation_level, la.last_error,
             o.total, o.currency_code,
             c.name AS customer_name, c.phone AS customer_phone,
             COALESCE(o.confirmed_at, o.created_at) AS status_updated_at
      FROM location_alerts la
      JOIN orders o ON o.id = la.order_id
      LEFT JOIN customers c ON c.id = o.customer_id
      ${clauses}
      ORDER BY la.created_at DESC
      LIMIT $${limitIdx}
    `, params);

    const hasMore = res.rows.length > limit;
    const alerts = (hasMore ? res.rows.slice(0, limit) : res.rows).map((row: any) => ({
      id: row.id,
      orderId: row.order_id,
      kind: row.kind,
      status: row.resolved_at ? 'resolved' : 'active',
      createdAt: row.created_at,
      resolvedAt: row.resolved_at,
      acknowledgedAt: row.acknowledged_at,
      escalationLevel: row.escalation_level,
      lastError: row.last_error,
      dwellSeconds: row.status_updated_at ? Math.floor((Date.now() - new Date(row.status_updated_at).getTime()) / 1000) : null,
      total: row.total,
      currency: row.currency_code || 'ALL',
      customerNameMasked: maskName(row.customer_name),
      customerPhoneMasked: maskPhone(row.customer_phone),
    }));

    const nextCursor = hasMore && alerts.length > 0
      ? Buffer.from(JSON.stringify({ createdAt: alerts[alerts.length - 1].createdAt })).toString('base64url')
      : null;

    return reply.send({ alerts, nextCursor });
  });

  // ─── Acknowledge ──────────────────────────────────────────────────
  fastify.post('/:locationId/alerts/:alertId/acknowledge', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), alertId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId, alertId } = request.params;
    const user = request.user as any;

    const res = await db.query(
      `UPDATE location_alerts
       SET status = 'resolved',
           acknowledged_at = now(),
           acknowledged_by_owner_id = $1,
           resolved_at = now(),
           resolution_reason = 'owner_acknowledge'
       WHERE id = $2
         AND location_id = $3
         AND resolved_at IS NULL
       RETURNING id, order_id, kind`,
      [user.userId, alertId, locationId],
    );

    if (res.rowCount === 0) return reply.status(404).send({ error: 'Alert not found or already resolved' });

    const { order_id, kind } = res.rows[0];

    // Cancel pending escalation jobs
    try {
      const jobs = await queue.boss.find({ 'data.alertId': alertId, state: 'created' });
      for (const job of jobs) {
        await queue.boss.cancel(job.id);
      }
    } catch {
      // pg-boss query errors are non-critical — alert already acknowledged
      console.debug('[alerts] pg-boss cancel failed for', alertId);
    }

    await messageBus.publish(`location:${locationId}:dashboard`, {
      type: 'dwell.alert_acknowledged',
      data: { alertId, orderId: order_id, kind, acknowledgedAt: new Date().toISOString() },
    });

    return reply.send({ id: alertId, status: 'resolved', acknowledgedAt: new Date().toISOString() });
  });

  // ─── Bulk Acknowledge ─────────────────────────────────────────────
  fastify.post('/:locationId/alerts/acknowledge-all', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({ kind: z.string().optional() }).optional(),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const body = request.body || {};
    const user = request.user as any;

    let kindClause = '';
    const params: any[] = [user.userId, locationId];
    if (body.kind) {
      kindClause = ' AND kind = $3';
      params.push(body.kind);
    }

    const res = await db.query(`
      UPDATE location_alerts
      SET status = 'resolved',
          acknowledged_at = now(),
          acknowledged_by_owner_id = $1,
          resolved_at = now(),
          resolution_reason = 'owner_acknowledge_bulk'
      WHERE location_id = $2
        AND resolved_at IS NULL
        ${kindClause}
      RETURNING id, order_id, kind
    `, params);

    for (const row of res.rows) {
      await messageBus.publish(`location:${locationId}:dashboard`, {
        type: 'dwell.alert_acknowledged',
        data: { alertId: row.id, orderId: row.order_id, kind: row.kind, acknowledgedAt: new Date().toISOString() },
      });
    }

    return reply.send({ acknowledged: res.rowCount });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
