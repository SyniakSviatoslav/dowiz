// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ownerSettlementRoutes(fastify, opts) {
  const { db, messageBus } = opts as any;

  // Add auth hook
  fastify.addHook('onRequest', async (request, reply) => {
    try {
      await request.jwtVerify();
      const user = request.user as any;
      if (user.role !== 'owner') {
        return reply.status(403).send({ error: 'Owner only' });
      }
      
      const { locationId } = request.params as any;
      if (!locationId || !user.activeLocationId || user.activeLocationId !== locationId) {
        return reply.status(404).send({ error: 'Not found' }); // Tenant isolation
      }
    } catch (err) {
      return reply.status(401).send({ error: 'Unauthorized' });
    }
  });

  // GET /api/owner/locations/:locationId/settlements
  fastify.get('/:locationId/settlements', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        status: z.enum(['pending', 'approved', 'paid', 'disputed']).optional(),
        courier_id: z.string().uuid().optional(),
        period_start: z.string().optional(),
        period_end: z.string().optional()
      })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const q = request.query;
    
    let sql = `
      SELECT p.id, p.courier_id, c.full_name_encrypted, p.deliveries_count, p.total_earned,
             p.period_start, p.period_end, p.status, p.created_at, p.approved_at, p.paid_at,
             (SELECT currency_code FROM locations WHERE id = $1) as currency
      FROM courier_payouts p
      JOIN couriers c ON c.id = p.courier_id
      WHERE p.location_id = $1
    `;
    const params: any[] = [locationId];
    let idx = 2;

    if (q.status) { sql += ` AND p.status = $${idx++}`; params.push(q.status); }
    if (q.courier_id) { sql += ` AND p.courier_id = $${idx++}`; params.push(q.courier_id); }
    if (q.period_start) { sql += ` AND p.period_start >= $${idx++}`; params.push(q.period_start); }
    if (q.period_end) { sql += ` AND p.period_end <= $${idx++}`; params.push(q.period_end); }

    sql += ` ORDER BY p.created_at DESC`;

    const { decryptPII } = await import('../../lib/pii-cipher.js');

    const res = await db.query(sql, params);
    const payouts = res.rows.map(r => {
      const name = decryptPII(r.full_name_encrypted) || '';
      return {
        id: r.id,
        courierId: r.courier_id,
        courierNameMasked: name ? name.charAt(0) + '***' : 'A***',
        deliveriesCount: r.deliveries_count,
        totalEarned: r.total_earned,
        currency: r.currency,
        periodStart: r.period_start,
        periodEnd: r.period_end,
        status: r.status,
        createdAt: r.created_at,
        approvedAt: r.approved_at,
        paidAt: r.paid_at
      };
    });

    return { payouts };
  });

  // GET /api/owner/locations/:locationId/settlements/:id
  fastify.get('/:locationId/settlements/:id', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), id: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId, id } = request.params;
    
    const payoutRes = await db.query(`
      SELECT p.*, c.full_name_encrypted
      FROM courier_payouts p
      JOIN couriers c ON c.id = p.courier_id
      WHERE p.id = $1 AND p.location_id = $2
    `, [id, locationId]);

    if (payoutRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const itemsRes = await db.query(`
      SELECT si.assignment_id, ca.order_id, ca.delivered_at, si.amount, si.currency_code
      FROM settlement_items si
      JOIN courier_assignments ca ON ca.id = si.assignment_id
      WHERE si.payout_id = $1
      ORDER BY ca.delivered_at DESC
    `, [id]);

    const items = itemsRes.rows.map(r => ({
      shortOrderId: r.order_id.substring(0, 8),
      deliveredAt: r.delivered_at,
      amount: r.amount,
      currency: r.currency_code,
    }));

    return { payout: payoutRes.rows[0], items };
  });

  // POST approve
  fastify.post('/:locationId/settlements/:id/approve', {
    schema: { params: z.object({ locationId: z.string().uuid(), id: z.string().uuid() }) },
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } }
  }, async (request, reply) => {
    const { locationId, id } = request.params;
    const userId = (request.user as any).sub;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const pRes = await client.query(`
        UPDATE courier_payouts
        SET status = 'approved', approved_at = now()
        WHERE id = $1 AND location_id = $2 AND status = 'pending'
        RETURNING *
      `, [id, locationId]);

      if (pRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Payout not found or not pending' });
      }

      await client.query(`
        INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, actor_id)
        VALUES ($1, $2, 'approved', 'owner', $3)
      `, [id, locationId, userId]);

      await client.query('COMMIT');

      // Publish event
      const p = pRes.rows[0];
      await messageBus.publish('settlement.approved', {
        payoutId: p.id,
        courierId: p.courier_id,
        locationId: p.location_id,
        totalEarned: p.total_earned,
        currency: 'ALL', // In variant single currency
        periodStart: p.period_start,
        periodEnd: p.period_end
      });

      return reply.status(200).send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // POST pay
  fastify.post('/:locationId/settlements/:id/pay', {
    schema: { 
      params: z.object({ locationId: z.string().uuid(), id: z.string().uuid() }),
      body: z.object({ payment_reference: z.string().optional(), payment_method: z.enum(['cash', 'bank_transfer', 'other']).optional() })
    },
    config: { rateLimit: { max: 30, timeWindow: '1 minute' } }
  }, async (request, reply) => {
    const { locationId, id } = request.params;
    const { payment_reference, payment_method } = request.body;
    const userId = (request.user as any).sub;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const pRes = await client.query(`
        UPDATE courier_payouts
        SET status = 'paid', paid_at = now()
        WHERE id = $1 AND location_id = $2 AND status = 'approved'
        RETURNING *
      `, [id, locationId]);

      if (pRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Payout not found or not approved' });
      }

      await client.query(`
        INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, actor_id, metadata)
        VALUES ($1, $2, 'paid', 'owner', $3, $4)
      `, [id, locationId, userId, JSON.stringify({ payment_reference, payment_method })]);

      await client.query('COMMIT');

      return reply.status(200).send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // POST dispute
  fastify.post('/:locationId/settlements/:id/dispute', {
    schema: { 
      params: z.object({ locationId: z.string().uuid(), id: z.string().uuid() }),
      body: z.object({ reason: z.string(), items: z.array(z.string().uuid()).optional() })
    },
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } }
  }, async (request, reply) => {
    const { locationId, id } = request.params;
    const { reason, items } = request.body;
    const userId = (request.user as any).sub;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const pRes = await client.query(`
        UPDATE courier_payouts
        SET status = 'disputed'
        WHERE id = $1 AND location_id = $2 AND status IN ('pending', 'approved')
        RETURNING *
      `, [id, locationId]);

      if (pRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'ALREADY_DISPUTED or invalid status' });
      }

      await client.query(`
        INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, actor_id, metadata)
        VALUES ($1, $2, 'disputed', 'owner', $3, $4)
      `, [id, locationId, userId, JSON.stringify({ reason, disputed_items: items })]);

      await client.query('COMMIT');

      // Notify courier (Telegram) handled elsewhere by listening to a dispute event, or just emit event
      await messageBus.publish('settlement.disputed', {
        payoutId: id,
        courierId: pRes.rows[0].courier_id,
        locationId
      });

      return reply.status(200).send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // POST reopen
  fastify.post('/:locationId/settlements/:id/reopen', {
    schema: { 
      params: z.object({ locationId: z.string().uuid(), id: z.string().uuid() }),
      body: z.object({ reason: z.string() })
    },
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } }
  }, async (request, reply) => {
    const { locationId, id } = request.params;
    const { reason } = request.body;
    const userId = (request.user as any).sub;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const pRes = await client.query(`
        UPDATE courier_payouts
        SET status = 'pending'
        WHERE id = $1 AND location_id = $2 AND status = 'disputed'
        RETURNING *
      `, [id, locationId]);

      if (pRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'Payout not disputed' });
      }

      await client.query(`
        INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, actor_id, metadata)
        VALUES ($1, $2, 'reopened', 'owner', $3, $4)
      `, [id, locationId, userId, JSON.stringify({ reason })]);

      await client.query('COMMIT');

      return reply.status(200).send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // POST regenerate manually
  fastify.post('/:locationId/settlements/regenerate', {
    schema: { 
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({ referenceDate: z.string() })
    },
    config: { rateLimit: { max: 5, timeWindow: '5 minutes' } }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { referenceDate } = request.body;
    
    // Publish a job specifically for this location or globally
    const { SettlementCronWorker } = await import('../../workers/settlement-cron.js');
    const worker = new SettlementCronWorker(db, null as any);
    await worker.handleGenerate(new Date(referenceDate)); // Technically processes all locations. For scale, we'd limit to locationId. 

    return reply.status(200).send({ success: true });
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
