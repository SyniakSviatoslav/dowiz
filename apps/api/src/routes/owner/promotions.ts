import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

const PromotionParams = z.object({
  id: z.string().uuid()
});

export default async function ownerPromotionRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  // Resolve owner's location from DB (not from JWT — JWT may not carry activeLocationId)
  const getLocationId = async (request: any, db: any): Promise<string> => {
    const jwtId = (request.user as any).activeLocationId;
    if (jwtId) return jwtId;
    const userId = (request.user as any).userId;
    const res = await db.query(
      `SELECT id FROM locations WHERE owner_id = $1 LIMIT 1`,
      [userId]
    );
    if (res.rowCount === 0) throw new Error('No location found for this user');
    return res.rows[0].id;
  };

  // ─── List Promotions ──────────────────────────────────────────────
  server.get(
    '/api/owner/promotions',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        querystring: z.object({
          is_active: z.coerce.boolean().optional(),
          type: z.enum(['percentage', 'fixed', 'free_delivery']).optional(),
          limit: z.coerce.number().int().min(1).max(100).default(50),
          offset: z.coerce.number().int().min(0).default(0),
        }).strict(),
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const { is_active, type, limit, offset } = request.query;
      const userId = request.user.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        const params: any[] = [locationId];
        let clauses = 'WHERE p.location_id = $1';

        if (is_active !== undefined) {
          params.push(is_active);
          clauses += ` AND p.is_active = $${params.length}`;
        }
        if (type) {
          params.push(type);
          clauses += ` AND p.type = $${params.length}`;
        }

        params.push(limit);
        params.push(offset);

        const data = await client.query(
          `SELECT p.* FROM promotions p ${clauses} ORDER BY p.created_at DESC LIMIT $${params.length - 1} OFFSET $${params.length}`,
          params
        );

        const countRes = await client.query(
          `SELECT COUNT(*) FROM promotions p ${clauses}`,
          params.slice(0, -2)
        );

        return {
          promotions: data.rows,
          total: parseInt(countRes.rows[0].count, 10),
        };
      });

      return reply.send(res);
    }
  );

  // ─── Create Promotion ─────────────────────────────────────────────
  server.post(
    '/api/owner/promotions',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        body: z.object({
          code: z.string().min(1).max(50),
          type: z.enum(['percentage', 'fixed', 'free_delivery']),
          discount_value: z.number().int().positive(),
          min_order_amount: z.number().int().min(0).default(0),
          max_uses: z.number().int().positive().nullable().optional(),
          max_uses_per_customer: z.number().int().positive().default(1),
          valid_from: z.string().datetime().optional(),
          valid_until: z.string().datetime().nullable().optional(),
          is_active: z.boolean().default(true),
          applicable_product_ids: z.array(z.string().uuid()).default([]),
          description: z.string().max(500).nullable().optional(),
        }).strict(),
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const body = request.body;
      const userId = request.user.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `INSERT INTO promotions (location_id, code, type, discount_value, min_order_amount, max_uses, max_uses_per_customer, valid_from, valid_until, is_active, applicable_product_ids, description)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           RETURNING *`,
          [
            locationId,
            body.code,
            body.type,
            body.discount_value,
            body.min_order_amount ?? 0,
            body.max_uses ?? null,
            body.max_uses_per_customer ?? 1,
            body.valid_from ? new Date(body.valid_from) : new Date(),
            body.valid_until ? new Date(body.valid_until) : null,
            body.is_active ?? true,
            body.applicable_product_ids ?? [],
            body.description ?? null,
          ]
        );
      });

      return reply.status(201).send(res.rows[0]);
    }
  );

  // ─── Validate Promotion ───────────────────────────────────────────
  server.post(
    '/api/owner/promotions/validate',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        body: z.object({
          code: z.string().min(1).max(50),
          order_subtotal: z.number().int().min(0),
          product_ids: z.array(z.string().uuid()).optional(),
        }).strict(),
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const { code, order_subtotal, product_ids } = request.body;
      const userId = request.user.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `SELECT * FROM promotions WHERE location_id = $1 AND code = $2`,
          [locationId, code]
        );
      });

      if (res.rowCount === 0) {
        return reply.send({ valid: false, error: 'Promotion code not found' });
      }

      const promo = res.rows[0];

      // Check active
      if (!promo.is_active) {
        return reply.send({ valid: false, error: 'Promotion is not active' });
      }

      // Check valid_from
      if (new Date(promo.valid_from) > new Date()) {
        return reply.send({ valid: false, error: 'Promotion has not started yet' });
      }

      // Check valid_until
      if (promo.valid_until && new Date(promo.valid_until) < new Date()) {
        return reply.send({ valid: false, error: 'Promotion has expired' });
      }

      // Check max_uses
      if (promo.max_uses !== null && promo.current_uses >= promo.max_uses) {
        return reply.send({ valid: false, error: 'Promotion usage limit reached' });
      }

      // Check min_order_amount
      if (order_subtotal < promo.min_order_amount) {
        return reply.send({ valid: false, error: `Minimum order amount of ${promo.min_order_amount} not met` });
      }

      // Check applicable_product_ids
      if (promo.applicable_product_ids && promo.applicable_product_ids.length > 0 && product_ids && product_ids.length > 0) {
        const hasEligibleProduct = product_ids.some((pid: string) => promo.applicable_product_ids.includes(pid));
        if (!hasEligibleProduct) {
          return reply.send({ valid: false, error: 'No eligible products in the order' });
        }
      }

      // Calculate discount amount
      let discount_amount = 0;
      if (promo.type === 'percentage') {
        discount_amount = Math.floor(order_subtotal * promo.discount_value / 100);
      } else if (promo.type === 'fixed') {
        discount_amount = Math.min(promo.discount_value, order_subtotal);
      }
      // free_delivery discount amount is calculated at order level

      return reply.send({
        valid: true,
        promotion: promo,
        discount_amount,
      });
    }
  );

  // ─── Get Single Promotion ─────────────────────────────────────────
  server.get(
    '/api/owner/promotions/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: PromotionParams,
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const { id } = request.params;
      const userId = request.user.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `SELECT * FROM promotions WHERE id = $1 AND location_id = $2`,
          [id, locationId]
        );
      });

      if (res.rowCount === 0) {
        return reply.status(404).send({ error: 'Promotion not found' });
      }

      return reply.send(res.rows[0]);
    }
  );

  // ─── Update Promotion ─────────────────────────────────────────────
  server.patch(
    '/api/owner/promotions/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: PromotionParams,
        body: z.object({
          code: z.string().min(1).max(50).optional(),
          type: z.enum(['percentage', 'fixed', 'free_delivery']).optional(),
          discount_value: z.number().int().positive().optional(),
          min_order_amount: z.number().int().min(0).optional(),
          max_uses: z.number().int().positive().nullable().optional(),
          max_uses_per_customer: z.number().int().positive().optional(),
          valid_from: z.string().datetime().optional(),
          valid_until: z.string().datetime().nullable().optional(),
          is_active: z.boolean().optional(),
          applicable_product_ids: z.array(z.string().uuid()).optional(),
          description: z.string().max(500).nullable().optional(),
        }).strict(),
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const { id } = request.params;
      const body = request.body;
      const userId = request.user.userId;

      const sets: string[] = [];
      const params: any[] = [];
      let idx = 1;

      const fields: Array<{ col: string; val: any }> = [
        { col: 'code', val: body.code },
        { col: 'type', val: body.type },
        { col: 'discount_value', val: body.discount_value },
        { col: 'min_order_amount', val: body.min_order_amount },
        { col: 'max_uses', val: body.max_uses !== undefined ? body.max_uses : null },
        { col: 'max_uses_per_customer', val: body.max_uses_per_customer },
        { col: 'valid_from', val: body.valid_from ? new Date(body.valid_from) : undefined },
        { col: 'valid_until', val: body.valid_until !== undefined ? (body.valid_until ? new Date(body.valid_until) : null) : undefined },
        { col: 'is_active', val: body.is_active },
        { col: 'applicable_product_ids', val: body.applicable_product_ids },
        { col: 'description', val: body.description !== undefined ? body.description : null },
      ];

      for (const f of fields) {
        if (f.val !== undefined) {
          sets.push(`${f.col} = $${idx}`);
          params.push(f.val);
          idx++;
        }
      }

      if (sets.length === 0) {
        return reply.status(400).send({ error: 'No fields to update' });
      }

      params.push(id);
      params.push(locationId);

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `UPDATE promotions SET ${sets.join(', ')} WHERE id = $${idx} AND location_id = $${idx + 1} RETURNING *`,
          params
        );
      });

      if (res.rowCount === 0) {
        return reply.status(404).send({ error: 'Promotion not found' });
      }

      return reply.send(res.rows[0]);
    }
  );

  // ─── Delete Promotion ─────────────────────────────────────────────
  server.delete(
    '/api/owner/promotions/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: PromotionParams,
      }
    },
    async (request: any, reply: any) => {
      const locationId = await getLocationId(request, server.db);
      const { id } = request.params;
      const userId = request.user.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `DELETE FROM promotions WHERE id = $1 AND location_id = $2 RETURNING id`,
          [id, locationId]
        );
      });

      if (res.rowCount === 0) {
        return reply.status(404).send({ error: 'Promotion not found' });
      }

      return reply.send({ success: true });
    }
  );
}
