import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

const fallbackBodySchema = z.object({
  phone: z.string().max(50).optional(),
  showPhoneOnError: z.boolean(),
  showPhoneOnOffline: z.boolean(),
  wsRetryMax: z.number().int().min(1).max(30).optional(),
  wsRetryBaseMs: z.number().int().min(500).max(10000).optional(),
}).strict();

export default (async function ownerFallbackRoutes(fastify: any, opts: any) {
  const { db } = opts;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── GET Fallback Config ─────────────────────────────────────────
  fastify.get('/:locationId/settings/fallback', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const res = await db.query(
      `SELECT fallback_config FROM locations WHERE id = $1`,
      [locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    const config = res.rows[0]?.fallback_config || {};
    return reply.send({
      phone: config.phone || null,
      showPhoneOnError: config.show_phone_on_error !== false,
      showPhoneOnOffline: config.show_phone_on_offline !== false,
      wsRetryMax: config.ws_retry_max || 10,
      wsRetryBaseMs: config.ws_retry_base_ms || 2000,
    });
  });

  // ─── PUT Fallback Config ─────────────────────────────────────────
  fastify.put('/:locationId/settings/fallback', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: fallbackBodySchema,
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const { phone, showPhoneOnError, showPhoneOnOffline, wsRetryMax, wsRetryBaseMs } = request.body;

    const config: Record<string, any> = {};
    if (phone !== undefined) config.phone = phone;
    config.show_phone_on_error = showPhoneOnError;
    config.show_phone_on_offline = showPhoneOnOffline;
    if (wsRetryMax !== undefined) config.ws_retry_max = wsRetryMax;
    if (wsRetryBaseMs !== undefined) config.ws_retry_base_ms = wsRetryBaseMs;

    const res = await db.query(
      `UPDATE locations SET fallback_config = $1 WHERE id = $2 RETURNING id`,
      [JSON.stringify(config), locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    return reply.send({ success: true, config });
  });

  // ─── GET Degradation Status ──────────────────────────────────────
  fastify.get('/:locationId/degradation', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;

    const [fbRes, notifRes] = await Promise.all([
      db.query(
        `SELECT fallback_config FROM locations WHERE id = $1`,
        [locationId],
      ),
      db.query(
        `SELECT channel, status, last_error, created_at FROM owner_notification_targets
         WHERE location_id = $1 ORDER BY channel`,
        [locationId],
      ),
    ]);

    if (fbRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const config = fbRes.rows[0].fallback_config || {};
    const channels = notifRes.rows;
    const pushFailed = channels.filter((c: any) => c.channel === 'push' && c.last_error);
    const telegramFailed = channels.filter((c: any) => c.channel === 'telegram' && c.last_error);

    const deadChannels: string[] = [];
    if (pushFailed.length > 0) deadChannels.push('push');
    if (telegramFailed.length > 0) deadChannels.push('telegram');

    return reply.send({
      locationId,
      fallbackPhone: config.phone || null,
      showPhoneOnError: config.show_phone_on_error !== false,
      showPhoneOnOffline: config.show_phone_on_offline !== false,
      deadChannels,
      channels: channels.map((c: any) => ({
        channel: c.channel,
        status: c.status,
        lastError: c.last_error,
        createdAt: c.created_at,
      })),
    });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
