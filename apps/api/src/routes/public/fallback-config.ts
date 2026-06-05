import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function publicFallbackConfigRoutes(fastify, opts) {
  const { db } = opts as any;

  // ─── GET Fallback Config for a location (public, no auth) ────────
  fastify.get('/api/public/locations/:slug/fallback-config', {
    schema: {
      params: z.object({ slug: z.string() }),
    },
  }, async (request, reply) => {
    const { slug } = request.params;

    const res = await db.query(
      `SELECT fallback_config, phone AS public_phone
       FROM locations WHERE slug = $1`,
      [slug],
    );

    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const row = res.rows[0];
    const config = row.fallback_config || {};

    return reply.send({
      phone: config.phone || row.public_phone || null,
      showPhoneOnError: config.show_phone_on_error !== false,
      showPhoneOnOffline: config.show_phone_on_offline !== false,
    });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
