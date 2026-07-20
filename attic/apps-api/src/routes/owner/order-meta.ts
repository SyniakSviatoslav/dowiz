import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ownerOrderMetaRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── Patch order metadata (test_order flag, etc.) ─────────────────
  fastify.patch('/:locationId/orders/:orderId/metadata', {
    schema: {
      params: z.object({
        locationId: z.string().uuid(),
        orderId: z.string().uuid(),
      }),
      body: z.object({
        test_order: z.boolean().optional(),
      }).strict(),
    },
  }, async (request: any, reply: any) => {
    const { locationId, orderId } = request.params as any;
    const { test_order } = request.body as any;

    // Only update metadata.test_order — merge into existing metadata
    const res = await db.query(
      `UPDATE orders
       SET metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{test_order}', $1::jsonb, true)
       WHERE id = $2 AND location_id = $3
       RETURNING id`,
      [JSON.stringify(test_order), orderId, locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    return reply.send({ ok: true });
  });
});
