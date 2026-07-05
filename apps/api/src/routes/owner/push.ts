import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

const subscribePushSchema = z.object({
  subscription: z.object({
    endpoint: z.string().url(),
    keys: z.object({
      p256dh: z.string().min(1),
      auth: z.string().min(1),
    }),
  }),
}).strict();

export default (async function ownerPushRoutes(fastify: any, opts: any) {
  const { db } = opts;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // ─── Register owner push subscription ─────────────────────────────
  fastify.post('/:locationId/push/subscribe', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: subscribePushSchema,
    },
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params as any;
    const user = request.user as any;
    const { subscription } = request.body as any;

    const address = JSON.stringify(subscription);

    // Upsert: same address → reactivate; new → insert
    const existing = await db.query(
      `SELECT id, status FROM owner_notification_targets
       WHERE location_id = $1 AND channel = 'push' AND address = $2`,
      [locationId, address],
    );

    if (existing.rowCount > 0) {
      await db.query(
        `UPDATE owner_notification_targets
         SET status = 'active', last_error = NULL, disabled_at = NULL
         WHERE id = $1`,
        [existing.rows[0].id],
      );
    } else {
      // user_id is NOT NULL with an FK to users and a DEFAULT gen_random_uuid() (mig
      // 1790000000004) — omitting it inserts a random uuid that violates the FK, so this
      // route 500'd on EVERY new subscription. Pass the authenticated owner explicitly,
      // exactly like the telegram-webhook/telegram-poll insert sites do.
      await db.query(
        `INSERT INTO owner_notification_targets (location_id, channel, address, status, user_id)
         VALUES ($1, 'push', $2, 'active', $3)`,
        [locationId, address, user.userId],
      );
    }

    return reply.status(200).send({ ok: true });
  });

  // ─── Unsubscribe owner push ───────────────────────────────────────
  fastify.post('/:locationId/push/unsubscribe', {
    schema: { params: z.object({ locationId: z.string().uuid() }) },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params as any;

    await db.query(
      `UPDATE owner_notification_targets
       SET status = 'disabled', disabled_at = now()
       WHERE location_id = $1 AND channel = 'push' AND status = 'active'`,
      [locationId],
    );
    return reply.status(200).send({ ok: true });
  });

  // ─── Get push subscription state ──────────────────────────────────
  fastify.get('/:locationId/push/state', {
    schema: { params: z.object({ locationId: z.string().uuid() }) },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params as any;

    const res = await db.query(
      `SELECT id, status, last_error, created_at
       FROM owner_notification_targets
       WHERE location_id = $1 AND channel = 'push'
       ORDER BY created_at DESC LIMIT 1`,
      [locationId],
    );

    if (res.rowCount === 0) {
      return reply.send({ subscribed: false });
    }
    return reply.send({
      subscribed: res.rows[0].status === 'active',
      status: res.rows[0].status,
      lastError: res.rows[0].last_error,
      createdAt: res.rows[0].created_at,
    });
  });
});
