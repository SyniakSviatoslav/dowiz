// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { maskName, maskPhone } from '../../lib/pii-mask.js';

const createRequestSchema = z.object({
  customerId: z.string().uuid().optional(),
  phone: z.string().optional(),
  reason: z.string().max(500).optional(),
}).strict().refine(data => data.customerId || data.phone, {
  message: 'Either customerId or phone is required',
});

const listQuerySchema = z.object({
  status: z.enum(['pending', 'in_progress', 'completed', 'failed']).optional(),
  limit: z.coerce.number().int().min(1).max(100).default(50),
  cursor: z.string().optional(),
});

const retentionBodySchema = z.object({
  retentionDays: z.number().int().min(30).max(2555),
}).strict();

export default (async function ownerGdprRoutes(fastify, opts) {
  const { db, messageBus, queue } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── POST Create GDPR Erasure Request ─────────────────────────────
  fastify.post('/:locationId/gdpr-requests', {
    config: {
      rateLimit: { max: 30, timeWindow: '1 minute' },
    },
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: createRequestSchema,
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { customerId, phone, reason } = request.body;
    const user = request.user as any;

    if (customerId) {
      const recentRes = await db.query(
        `SELECT id FROM gdpr_erasure_requests
         WHERE location_id = $1 AND customer_id = $2 AND status = 'completed'
           AND completed_at > now() - interval '24 hours'
         LIMIT 1`,
        [locationId, customerId],
      );
      if (recentRes.rowCount > 0) {
        return reply.status(429).send({ error: 'A request for this customer was already completed in the last 24 hours' });
      }
    }

    if (phone && !customerId) {
      const custRes = await db.query(
        `SELECT id FROM customers WHERE location_id = $1 AND phone = $2 LIMIT 1`,
        [locationId, phone],
      );
      if (custRes.rowCount > 0) {
        const recentRes = await db.query(
          `SELECT id FROM gdpr_erasure_requests
           WHERE location_id = $1 AND customer_id = $2 AND status = 'completed'
             AND completed_at > now() - interval '24 hours'
           LIMIT 1`,
          [locationId, custRes.rows[0].id],
        );
        if (recentRes.rowCount > 0) {
          return reply.status(429).send({ error: 'A request for this customer was already completed in the last 24 hours' });
        }
      }
    }

    const insertRes = await db.query(
      `INSERT INTO gdpr_erasure_requests (location_id, customer_id, subject_phone, reason, requested_by_owner_id, status)
       VALUES ($1, $2, $3, $4, $5, 'pending')
       RETURNING id`,
      [locationId, customerId || null, phone || null, reason || null, user.userId],
    );

    const requestId = insertRes.rows[0].id;

    if (queue) {
      await queue.send('anonymizer.gdpr', { requestId });
    }

    return reply.status(201).send({ requestId, status: 'pending' });
  });

  // ─── GET List GDPR Requests ──────────────────────────────────────
  fastify.get('/:locationId/gdpr-requests', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: listQuerySchema,
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { status, limit, cursor } = request.query;

    const params: any[] = [locationId];
    let clauses = 'WHERE location_id = $1';

    if (status) {
      params.push(status);
      clauses += ` AND status = $${params.length}`;
    }

    if (cursor) {
      try {
        const decoded = JSON.parse(Buffer.from(cursor, 'base64url').toString());
        if (decoded.requestedAt) {
          params.push(decoded.requestedAt);
          clauses += ` AND requested_at < $${params.length}`;
        }
      } catch {
        // invalid cursor — ignore, will use no cursor filter
        console.debug('[gdpr] invalid cursor, ignoring');
      }
    }

    const limitIdx = params.length + 1;
    params.push(limit + 1);

    const res = await db.query(`
      SELECT id, location_id, customer_id, status, error_message,
             requested_at, completed_at
      FROM gdpr_erasure_requests
      ${clauses}
      ORDER BY requested_at DESC
      LIMIT $${limitIdx}
    `, params);

    const hasMore = res.rows.length > limit;
    const requests = (hasMore ? res.rows.slice(0, limit) : res.rows).map((row: any) => ({
      id: row.id,
      customerId: row.customer_id ? maskName(row.customer_id) : null,
      status: row.status,
      errorMessage: row.error_message,
      requestedAt: row.requested_at,
      completedAt: row.completed_at,
    }));

    const nextCursor = hasMore && requests.length > 0
      ? Buffer.from(JSON.stringify({ requestedAt: res.rows[limit - 1].requested_at })).toString('base64url')
      : null;

    return reply.send({ requests, nextCursor });
  });

  // ─── GET Single GDPR Request ─────────────────────────────────────
  fastify.get('/:locationId/gdpr-requests/:requestId', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), requestId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId, requestId } = request.params;

    const reqRes = await db.query(
      `SELECT id, location_id, customer_id, status, error_message, metadata,
              requested_at, completed_at
       FROM gdpr_erasure_requests
       WHERE id = $1 AND location_id = $2`,
      [requestId, locationId],
    );

    if (reqRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const row = reqRes.rows[0];

    let auditLogs: any[] = [];
    if (row.customer_id) {
      const auditRes = await db.query(
        `SELECT id, scope, subject_kind, subject_id, actor_kind, actor_id, metadata, created_at
         FROM anonymization_audit_log
         WHERE subject_id = $1 AND location_id = $2
         ORDER BY created_at DESC`,
        [row.customer_id, locationId],
      );
      auditLogs = auditRes.rows.map((a: any) => ({
        id: a.id,
        scope: a.scope,
        subjectKind: a.subject_kind,
        subjectId: maskName(a.subject_id),
        actorKind: a.actor_kind,
        actorId: a.actor_id ? maskName(a.actor_id) : null,
        metadata: a.metadata,
        createdAt: a.created_at,
      }));
    }

    return reply.send({
      id: row.id,
      customerId: row.customer_id ? maskName(row.customer_id) : null,
      status: row.status,
      errorMessage: row.error_message,
      metadata: row.metadata,
      requestedAt: row.requested_at,
      completedAt: row.completed_at,
      auditLogs,
    });
  });

  // ─── GET Retention Settings ──────────────────────────────────────
  fastify.get('/:locationId/settings/retention', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const res = await db.query(
      `SELECT retention_days FROM locations WHERE id = $1`,
      [locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    return reply.send({ retentionDays: res.rows[0].retention_days ?? 365 });
  });

  // ─── PUT Retention Settings ──────────────────────────────────────
  fastify.put('/:locationId/settings/retention', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: retentionBodySchema,
    },
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { retentionDays } = request.body;
    const res = await db.query(
      `UPDATE locations SET retention_days = $1 WHERE id = $2 RETURNING retention_days`,
      [retentionDays, locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    return reply.send({ retentionDays: res.rows[0].retention_days });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
