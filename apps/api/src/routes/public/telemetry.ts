// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';

function hashIp(ip: string): string {
  return crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
}

export default (async function telemetryRoutes(fastify, opts) {
  const { db } = opts as any;

  const eventSchema = z.object({
    event: z.string().min(1).max(64),
    location_id: z.string().uuid().optional(),
    surface: z.enum(['storefront', 'embed', 'admin', 'courier', 'unknown']).optional(),
    lang: z.string().max(5).optional(),
    anon_id: z.string().max(64).optional(),
    session_id: z.string().max(64).optional(),
    version: z.string().max(20).optional(),
    props: z.record(z.any()).optional(),
  });

  const batchEventSchema = z.object({
    events: z.array(eventSchema).min(1).max(20),
    cwv: z.array(z.object({
      metric: z.enum(['LCP', 'CLS', 'INP', 'FID', 'TTFB']),
      value: z.number(),
      rating: z.enum(['good', 'needs-improvement', 'poor']).optional(),
      location_id: z.string().uuid().optional(),
      surface: z.string().optional(),
      anon_id: z.string().max(64).optional(),
      props: z.record(z.any()).optional(),
    })).max(10).optional(),
  });

  fastify.post('/api/telemetry', {
    config: { rateLimit: false }
  }, async (request, reply) => {
    try {
      const parsed = batchEventSchema.safeParse(request.body);
      if (!parsed.success) {
        return reply.status(202).send({ accepted: false, reason: 'validation' });
      }

      const { events, cwv } = parsed.data;
      const ipH = hashIp(request.ip);

      if (db && events.length > 0) {
        const client = await db.connect();
        try {
          for (const ev of events) {
            await client.query(
              `INSERT INTO analytics_events (event, location_id, surface, lang, anon_id, session_id, version, ip_hash, props)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
              [ev.event, ev.location_id || null, ev.surface || 'unknown', ev.lang || null,
               ev.anon_id || null, ev.session_id || null, ev.version || null, ipH,
               JSON.stringify(ev.props || {})]
            );
          }

          if (cwv && cwv.length > 0) {
            for (const m of cwv) {
              await client.query(
                `INSERT INTO analytics_cwv (metric, value, rating, location_id, surface, anon_id, props)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
                [m.metric, m.value, m.rating || null, m.location_id || null,
                 m.surface || null, m.anon_id || null, JSON.stringify(m.props || {})]
              );
            }
          }
        } finally {
          client.release();
        }
      }

      return reply.status(202).send({ accepted: true, count: events.length });
    } catch (err) {
      request.log.warn({ err }, 'Telemetry rejected');
      return reply.status(202).send({ accepted: false });
    }
  });

  fastify.post('/api/telemetry/abuse', {
    config: { rateLimit: false }
  }, async (request, reply) => {
    try {
      const body = request.body as any;
      const kind = body?.kind || 'unknown';
      const severity = body?.severity || 'low';
      const reason = body?.reason || '';

      if (db) {
        const client = await db.connect();
        try {
          await client.query(
            `INSERT INTO analytics_abuse_log (event, location_id, kind, severity, reason, ip_hash, anon_id, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
            ['abuse_signal', body?.location_id || null, kind, severity, reason,
             hashIp(request.ip), body?.anon_id || null, JSON.stringify(body?.metadata || {})]
          );
        } finally {
          client.release();
        }
      }

      return reply.status(202).send({ accepted: true });
    } catch (err) {
      request.log.warn({ err }, 'Abuse log rejected');
      return reply.status(202).send({ accepted: false });
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
