import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';

export default (async function localAuthRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.post('/auth/local/login', {
    schema: {
      body: z.object({
        email: z.string().email(),
        password: z.string().min(6)
      })
    }
  }, async (request: any, reply: any) => {
    const { email, password } = request.body as any;

    const db = (opts as any)?.db || (fastify as any).db;
    if (!db) return reply.status(500).send({ error: 'DB not configured' });

    const client = await db.connect();
    try {
      const res = await client.query(
        `SELECT id, password_hash, display_name FROM users WHERE email = $1`,
        [email.toLowerCase()]
      );

      if (res.rowCount === 0) {
        return reply.status(401).send({ error: 'Invalid email or password' });
      }

      const user = res.rows[0];

      let valid = false;

      // Real password login only (ADR-0003). The dev-credential bypass that lived here
      // was removed — the single flag-gated dev-login path is the inline handler in
      // server.ts; no credential literal lives in this file anymore.
      if (user.password_hash) {
        try {
          const argon2 = await import('argon2');
          valid = await argon2.default.verify(user.password_hash, password) ||
                  await (argon2 as any).verify(user.password_hash, password);
        } catch (err: any) {
          console.warn('[auth] argon2 not available:', err?.message);
          if (!user.password_hash) {
            return reply.status(401).send({ error: 'Account uses another sign-in method' });
          }
        }
      } else {
        return reply.status(401).send({ error: 'Account uses another sign-in method' });
      }

       if (!valid) {
         return reply.status(401).send({ error: 'Invalid email or password' });
       }

       // Determine user role AND active location. A membership directly carries the
       // location, so prefer it (owner memberships first); fall back to an org the user
       // owns. Without activeLocationId the owner UI can't scope and shows an empty/
       // onboarding state — which is exactly why test@dowiz.com saw no data.
       let role = 'customer'; // default to customer
       let activeLocationId: string | undefined;
       try {
         const memRes = await client.query(
           `SELECT location_id, role FROM memberships
            WHERE user_id = $1 AND status = 'active'
            ORDER BY (role = 'owner') DESC LIMIT 1`,
           [user.id]
         );
         if (memRes.rowCount > 0) {
           role = memRes.rows[0].role;
           activeLocationId = memRes.rows[0].location_id;
         } else {
           // No membership — but the user may own an organization with a location.
           const orgRes = await client.query(
             `SELECT id FROM organizations WHERE owner_id = $1`,
             [user.id]
           );
           if (orgRes.rowCount > 0) {
             role = 'owner';
             const locRes = await client.query(
               `SELECT id FROM locations WHERE org_id = $1 AND status = 'active' ORDER BY created_at LIMIT 1`,
               [orgRes.rows[0].id]
             );
             if (locRes.rowCount > 0) activeLocationId = locRes.rows[0].id;
           }
           // If neither, keep default 'customer' role
         }
       } catch (err) {
         request.log.error(err);
         // Fallback to customer role on error
         role = 'customer';
       }

       const familyId = crypto.randomUUID();
      const tokenPayload: Record<string, unknown> = { role, userId: user.id, sub: user.id };
      if (activeLocationId) tokenPayload.activeLocationId = activeLocationId;
      const accessToken = await signAuthToken(tokenPayload as any, '15m');
      const refreshToken = crypto.randomBytes(32).toString('hex');
      const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');

      try {
        await client.query(
          `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
           VALUES ($1, $2, $3, now() + interval '7 days')`,
          [user.id, familyId, refreshTokenHash]
        );
      } catch (err: any) {
        console.debug('[auth] refresh token insert failed, table may not exist:', err?.message);
      }

      return { access_token: accessToken, refresh_token: refreshToken };
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
