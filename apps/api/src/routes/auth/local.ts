import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
// @ts-ignore
import * as argon2 from 'argon2';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';

export default (async function localAuthRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.post('/auth/local/login', {
    config: { rateLimit: { max: 5, timeWindow: '1 minute' } },
    schema: {
      body: z.object({
        email: z.string().email(),
        password: z.string().min(6)
      }).strict()
    }
  }, async (request, reply) => {
    const { email, password } = request.body as any;

    const client = await db.connect();
    try {
      const res = await client.query(
        `SELECT id, password_hash FROM users WHERE email = $1`,
        [email.toLowerCase()]
      );

      if (res.rowCount === 0) {
        return reply.status(401).send({ error: 'Invalid email or password' });
      }

      const user = res.rows[0];

      if (!user.password_hash) {
        return reply.status(401).send({ error: 'Account uses another sign-in method' });
      }

      const valid = await argon2.verify(user.password_hash, password);
      if (!valid) {
        return reply.status(401).send({ error: 'Invalid email or password' });
      }

      // We assume local auth is for Owners for now. Real world would check memberships.
      // But per go-live docs, local auth is for the owner.
      const role = 'owner';
      const familyId = crypto.randomUUID();
      const accessToken = await signAuthToken({ role, userId: user.id } as any, '15m');
      const refreshToken = crypto.randomBytes(32).toString('hex');
      const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');

      await client.query(
        `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
         VALUES ($1, $2, $3, now() + interval '7 days')`,
        [user.id, familyId, refreshTokenHash]
      );

      return { access_token: accessToken, refresh_token: refreshToken };
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
