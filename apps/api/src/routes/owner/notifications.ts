import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';

export default (async function ownerNotificationRoutes(fastify, opts) {
  const { db, queue } = opts as any;

  // Auth: verify JWT
  fastify.addHook('onRequest', fastify.verifyAuth);

  // List targets
  fastify.get('/api/owner/locations/:locationId/notifications/targets', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params as { locationId: string };
    const client = await db.connect();
    try {
      const res = await client.query(`SELECT * FROM owner_notification_targets WHERE location_id = $1`, [locationId]);
      return reply.send({ targets: res.rows });
    } finally {
      client.release();
    }
  });

  // Notification status (lightweight readiness check)
  fastify.get('/api/owner/locations/:locationId/notifications/status', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params as { locationId: string };
    const client = await db.connect();
    try {
      const res = await client.query(
        `SELECT channel, status FROM owner_notification_targets WHERE location_id = $1`,
        [locationId]
      );
      const channels = res.rows;
      const anyActive = channels.some((r: any) => r.status === 'active');
      const telegramConnected = channels.some((r: any) => r.channel === 'telegram' && r.status === 'active');
      return reply.send({ channels, anyActive, telegramConnected });
    } finally {
      client.release();
    }
  });

  // Telegram connect-init
  fastify.post('/api/owner/locations/:locationId/notifications/telegram/connect-init', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params as { locationId: string };
    const ownerId = (request as any).user.sub;
    const token = crypto.randomUUID();
    
    const client = await db.connect();
    try {
      await client.query(
        `INSERT INTO telegram_connect_tokens (token, location_id, user_id, expires_at)
         VALUES ($1, $2, $3, now() + interval '10 minutes')`,
        [token, locationId, ownerId]
      );
      
      const botUsername = process.env.TELEGRAM_BOT_USERNAME || 'dowiz_bot';
      const deepLink = `https://t.me/${botUsername}?startapp=${token}`;
      
      return reply.send({ deepLink, token });
    } finally {
      client.release();
    }
  });

  // Send test
  fastify.post('/api/owner/locations/:locationId/notifications/test', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({ targetId: z.string().uuid().optional() }).strict()
    }
  }, async (request, reply) => {
    const { locationId } = request.params as { locationId: string };
    const { targetId } = request.body as { targetId?: string };
    
    const client = await db.connect();
    try {
      let query = `SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active'`;
      const params: any[] = [locationId];
      if (targetId) {
        query += ` AND id = $2`;
        params.push(targetId);
      }
      
      const targetsRes = await client.query(query, params);
      
      for (const target of targetsRes.rows) {
        await queue.boss.send('notify.dispatch', {
          targetId: target.id,
          eventType: 'test',
          locationId: locationId,
          attempt: 0,
          testMessage: 'This is a test notification.'
        });
      }
      
      return reply.send({ enqueued: targetsRes.rows.length });
    } finally {
      client.release();
    }
  });

  // Target management (disable/reconnect/prefs)
  fastify.put('/api/owner/locations/:locationId/notifications/targets/:targetId', {
    schema: {
      params: z.object({
        locationId: z.string().uuid(),
        targetId: z.string().uuid()
      }),
      body: z.object({
        status: z.enum(['active', 'disabled', 'disconnected']).optional(),
        prefs: z.record(z.boolean()).optional(),
        locale: z.enum(['sq', 'en', 'uk']).optional()
      }).strict()
    }
  }, async (request, reply) => {
    const { locationId, targetId } = request.params as { locationId: string; targetId: string };
    const { status, prefs, locale } = request.body as { status?: 'active' | 'disabled' | 'disconnected'; prefs?: Record<string, boolean>; locale?: string };
    
    const client = await db.connect();
    try {
      const currentRes = await client.query(`SELECT prefs FROM owner_notification_targets WHERE id = $1 AND location_id = $2`, [targetId, locationId]);
      if (currentRes.rows.length === 0) return reply.status(404).send({ error: 'Target not found' });
      
      let currentPrefs = currentRes.rows[0].prefs || {};
      if (prefs) {
        currentPrefs = { ...currentPrefs, ...prefs };
      }
      
      let setQuery = [];
      let params = [targetId, locationId];
      let pIdx = 3;
      
      if (status) {
        setQuery.push(`status = $${pIdx++}`);
        params.push(status);
        if (status === 'active') {
          setQuery.push(`last_error = NULL`);
          setQuery.push(`disabled_at = NULL`);
        }
      }
      
      if (prefs) {
        setQuery.push(`prefs = $${pIdx++}`);
        params.push(currentPrefs);
      }
      
      if (locale) {
        setQuery.push(`locale = $${pIdx++}`);
        params.push(locale);
      }
      
      if (setQuery.length > 0) {
        await client.query(
          `UPDATE owner_notification_targets SET ${setQuery.join(', ')} WHERE id = $1 AND location_id = $2`,
          params
        );
      }
      
      return reply.send({ success: true });
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
