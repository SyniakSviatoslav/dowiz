import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { auditCtx, auditCompleted } from '../../lib/platform-admin.js';

export default (async function notificationAuditRoutes(fastify, opts) {
  const { db } = opts as any;

  // ADR-admin-platform-authz (B4): auth is the platform-admin gate on the parent plane
  // (routes/admin/index.ts) + the root-instance gate in server.ts — NOT a per-file owner check.

  // GET /api/admin/notification-audit — lightweight audit query for release gate.
  // ADR-admin-platform-authz F4: was declared '/admin/notification-audit' under prefix '/api/admin'
  // → the real route was the double-prefixed '/api/admin/admin/notification-audit' and the single
  // path was unregistered (SPA fall-through → false-green). Declared without the redundant segment now.
  // PII-free: only exposes event, status, channel, count — no targets or addresses.
  fastify.get('/notification-audit', {
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
      await auditCompleted(db, auditCtx(request, 'notification_audit.query'), request.log);
      return { audit: res.rows };
    } catch (err: any) {
      // ADR-admin-platform-authz F6: do NOT leak err.message (schema/internal detail) in the envelope.
      request.log?.error?.({ err }, '[admin] notification-audit query failed');
      return reply.status(500).send({ error: 'Audit query failed' });
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
