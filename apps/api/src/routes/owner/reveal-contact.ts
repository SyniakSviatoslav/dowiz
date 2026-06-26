import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { dashboardChannel } from '../../lib/registry.js';
import { withTenant } from '@deliveryos/platform';

export default (async function ownerRevealContactRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── POST Reveal Customer Contact ────────────────────────────────
  fastify.post('/:locationId/orders/:orderId/reveal-customer-contact', {
    config: {
      rateLimit: { max: 10, timeWindow: '1 minute' },
    },
    schema: {
      params: z.object({
        locationId: z.string().uuid(),
        orderId: z.string().uuid(),
      }),
      body: z.object({
        reason: z.string().max(500).optional(),
      }).strict(),
    },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params;
    const { reason } = request.body;
    const user = request.user as any;

    const { orderRes, order } = await withTenant(db, user.userId, async (client) => {
      const orderRes = await client.query(
        `SELECT o.id, o.customer_id, c.name, c.phone
         FROM orders o
         LEFT JOIN customers c ON c.id = o.customer_id
         WHERE o.id = $1 AND o.location_id = $2`,
        [orderId, locationId],
      );

      if (orderRes.rowCount === 0) return { orderRes, order: null };

      const order = orderRes.rows[0];
      if (!order.customer_id) return { orderRes, order };

      // Audit the reveal
      await client.query(
        `INSERT INTO customer_contact_reveals (order_id, customer_id, location_id, revealed_by_owner_id, reason)
         VALUES ($1, $2, $3, $4, $5)`,
        [orderId, order.customer_id, locationId, user.userId, reason || null],
      );

      return { orderRes, order };
    });

    if (orderRes.rowCount === 0) return reply.sendError(404, 'NOT_FOUND', 'Not found');

    if (!order.customer_id) {
      return reply.sendError(404, 'NOT_FOUND', 'Customer not found');
    }

    // Emit event (PII-free)
    await messageBus.publish(`location:${locationId}:dashboard`, {
      type: 'customer.contact_revealed',
      data: { orderId, revealedAt: new Date().toISOString() },
    });

    return reply.send({
      orderId,
      customerId: order.customer_id,
      name: order.name,
      phone: order.phone,
    });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
