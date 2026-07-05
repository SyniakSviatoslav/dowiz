import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';

const subscribeSchema = z.object({
  subscription: z.object({
    endpoint: z.string().url(),
    keys: z.object({
      p256dh: z.string().min(1),
      auth: z.string().min(1),
    }),
  }),
  opted_in: z.boolean().default(true),
}).strict();

export default (async function customerPushRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  // ─── Register/update push subscription ────────────────────────────
  fastify.post('/push/subscribe', {
    schema: { body: subscribeSchema },
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const user = request.user as any;
    if (!user || user.role !== 'customer') {
      return reply.sendError(401, 'UNAUTHORIZED', 'Customer authentication required');
    }

    const { subscription, opted_in } = request.body;
    const fingerprint = crypto.createHash('sha256').update(subscription.endpoint).digest('hex');

    const client = await db.connect();
    try {
      await client.query("SELECT set_config('app.user_id', $1, true)", [user.userId]);
      const existing = await client.query(
        `SELECT id FROM customer_devices WHERE customer_id = $1 AND fingerprint = $2`,
        [user.userId, fingerprint],
      );

      if (existing.rowCount > 0) {
        await client.query(
          `UPDATE customer_devices
           SET push_subscription = $1, vapid_endpoint = $2, keys_p256dh = $3, keys_auth = $4,
               opted_in = $5, platform = 'webpush', last_seen_at = now()
           WHERE id = $6`,
          [JSON.stringify(subscription), subscription.endpoint, subscription.keys.p256dh, subscription.keys.auth, opted_in, existing.rows[0].id],
        );
      } else {
        await client.query(
          `INSERT INTO customer_devices (customer_id, platform, token_encrypted, fingerprint, opted_in, push_subscription, vapid_endpoint, keys_p256dh, keys_auth)
           VALUES ($1, 'webpush', $2, $3, $4, $5, $6, $7, $8)`,
          [user.userId, subscription.endpoint, fingerprint, opted_in, JSON.stringify(subscription), subscription.endpoint, subscription.keys.p256dh, subscription.keys.auth],
        );
      }

      return reply.status(200).send({ ok: true });
    } finally {
      client.release();
    }
  });

  // ─── Unsubscribe ──────────────────────────────────────────────────
  fastify.post('/push/unsubscribe', {
    config: { rateLimit: { max: 5, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const user = request.user as any;
    if (!user || user.role !== 'customer') return reply.sendError(401, 'UNAUTHORIZED', 'Unauthorized');

    const client = await db.connect();
    try {
      await client.query("SELECT set_config('app.user_id', $1, true)", [user.userId]);
      await client.query(
        `UPDATE customer_devices SET opted_in = false WHERE customer_id = $1 AND platform = 'webpush'`,
        [user.userId],
      );
      return reply.status(200).send({ ok: true });
    } finally {
      client.release();
    }
  });

});

