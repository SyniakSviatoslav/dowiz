// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function courierSettlementRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['courier']));

  // GET /api/courier/me/payouts
  fastify.get('/me/payouts', {
    schema: {
      querystring: z.object({
        status: z.enum(['pending', 'approved', 'paid', 'disputed']).optional()
      })
    }
  }, async (request, reply) => {
    const courierId = (request.user as any).sub;
    const { status } = request.query;

    // Set RLS context
    const activeLocationId = (request.user as any).activeLocationId;
    if (activeLocationId) {
      await db.query(`SELECT set_config('app.current_tenant', $1, true)`, [activeLocationId]);
    }

    let sql = `
      SELECT p.id, p.location_id, l.name as location_name, p.deliveries_count, p.total_earned,
             p.period_start, p.period_end, p.status, p.created_at, p.approved_at, p.paid_at,
             l.currency_code as currency
      FROM courier_payouts p
      JOIN locations l ON l.id = p.location_id
      WHERE p.courier_id = $1
    `;
    const params: any[] = [courierId];
    let idx = 2;

    if (status) {
      sql += ` AND p.status = $${idx++}`;
      params.push(status);
    }

    sql += ` ORDER BY p.created_at DESC`;

    const res = await db.query(sql, params);
    return { payouts: res.rows };
  });

  // GET /api/courier/me/payouts/:id
  fastify.get('/me/payouts/:id', {
    schema: { params: z.object({ id: z.string().uuid() }) }
  }, async (request, reply) => {
    const courierId = (request.user as any).sub;
    const { id } = request.params;

    const activeLocationId = (request.user as any).activeLocationId;
    if (activeLocationId) {
      await db.query(`SELECT set_config('app.current_tenant', $1, true)`, [activeLocationId]);
    }

    const payoutRes = await db.query(`
      SELECT p.id, p.location_id, l.name as location_name, p.deliveries_count, p.total_earned,
             p.period_start, p.period_end, p.status, p.created_at, p.approved_at, p.paid_at,
             l.currency_code as currency
      FROM courier_payouts p
      JOIN locations l ON l.id = p.location_id
      WHERE p.id = $1 AND p.courier_id = $2
    `, [id, courierId]);

    if (payoutRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    // Set RLS context for items query too
    if (activeLocationId) {
      await db.query(`SELECT set_config('app.current_tenant', $1, true)`, [activeLocationId]);
    }

    // Fetch masked items
    const itemsRes = await db.query(`
      SELECT ca.delivered_at, si.amount, si.currency_code as currency
      FROM settlement_items si
      JOIN courier_assignments ca ON ca.id = si.assignment_id
      WHERE si.payout_id = $1
      ORDER BY ca.delivered_at DESC
    `, [id]);

    return {
      payout: payoutRes.rows[0],
      items: itemsRes.rows // strictly no orderId, no assignmentId, no customer phone
    };
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
