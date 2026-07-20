import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { DEFAULT_DWELL_THRESHOLDS } from '../../lib/dwell-thresholds.js';
import { withTenant } from '@deliveryos/platform';

const inputSchema = z.object({
  pending_s: z.number().int().min(10).max(3600),
  confirmed_s: z.number().int().min(10).max(3600),
  preparing_s: z.number().int().min(10).max(7200),
  en_route_s: z.number().int().min(10).max(7200),
});

export default (async function ownerDwellSettingsRoutes(fastify: any, opts: any) {
  const { db } = opts;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── GET ─────────────────────────────────────────────────────────
  fastify.get('/:locationId/settings/dwell', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const userId = (request.user as any).userId;
    const res = await withTenant(db, userId, (client) =>
      client.query(
        `SELECT dwell_thresholds FROM locations WHERE id = $1`,
        [locationId],
      ),
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    const thresholds = res.rows[0]?.dwell_thresholds || DEFAULT_DWELL_THRESHOLDS;
    return reply.send({ dwellThresholds: thresholds });
  });

  // ─── PUT ──────────────────────────────────────────────────────────
  fastify.put('/:locationId/settings/dwell', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({ dwellThresholds: inputSchema }),
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const { dwellThresholds } = request.body;
    const stored = { v: 1, ...dwellThresholds };
    const userId = (request.user as any).userId;

    const res = await withTenant(db, userId, (client) =>
      client.query(
        `UPDATE locations SET dwell_thresholds = $1 WHERE id = $2 RETURNING id`,
        [JSON.stringify(stored), locationId],
      ),
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    return reply.send({ dwellThresholds: stored });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
