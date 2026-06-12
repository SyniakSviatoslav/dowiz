import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function notificationAuditRoutes(fastify, opts) {
  const { db } = opts as any;

  // GET /admin/notification-audit — lightweight audit query for release gate
  // Returns count of delivered entries matching event+location within time window
  // PII-free: only exposes event, status, channel, count — no targets or addresses
  fastify.get('/admin/notification-audit', {
    schema: {
      querystring: z.object({
        event: z.string().min(1).max(50),
        locationId: z.string().uuid().optional(),
        status: z.string().optional(),
        sinceMinutes: z.coerce.number().int().positive().default(30),
      }).strict(),
    },
  }, async (request, reply) => {
    const { event, locationId, status, sinceMinutes } = request.query as any;

    let query = `SELECT event, status, channel, count(*)::int AS cnt
                 FROM notification_outbox_audit
                 WHERE event = $1 AND created_at > now() - ($2 || ' minutes')::interval`;
    const params: any[] = [event, String(sinceMinutes)];

    if (locationId) {
      query += ` AND location_id = $3`;
      params.push(locationId);
    }
    if (status) {
      query += ` AND status = $${params.length + 1}`;
      params.push(status);
    }

    query += ` GROUP BY event, status, channel ORDER BY cnt DESC LIMIT 20`;

    try {
      const res = await db.query(query, params);
      return { audit: res.rows };
    } catch (err: any) {
      return reply.status(500).send({ error: 'Audit query failed', detail: err.message });
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
