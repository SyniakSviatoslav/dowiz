import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'crypto';
import { signAuthToken, signDevToken } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { devLoginAllowed } from '../../plugins/dev-guard.js';

/** Constant-time string compare; false on any length mismatch (never throws). */
function timingSafeStrEqual(a: string, b: string): boolean {
  const ab = Buffer.from(String(a));
  const bb = Buffer.from(String(b));
  if (ab.length !== bb.length) return false;
  return crypto.timingSafeEqual(ab, bb);
}

/**
 * Local email+password login. Two paths, in order:
 *  1. Flag-gated DEV bypass (ADR-0003) — only when devLoginAllowed(env) (ALLOW_DEV_LOGIN
 *     ='true' AND DEV_AUTH_SECRET set; prod boot-guard D forbids both, so this is inert on
 *     prod). Mints a DEV-key token (signDevToken) that a prod verifier rejects — never a
 *     prod-key token, so it can never become a prod backdoor.
 *  2. Real argon2 verification against users.password_hash (the path that serves prod and
 *     every real owner, incl. test@dowiz.com once its hash is seeded). Issues a short-lived
 *     access token (1h) + a rotating refresh token (7d family) — the web app refreshes
 *     transparently on 401, so the session rolls forward without surprise logouts.
 *
 * Registered with prefix /api in server.ts → POST /api/auth/local/login. This replaces the
 * inline dev-only handler that used to shadow this route (which had no real-password path,
 * so DB-password login was dead code and test@dowiz.com could only ever 401 on prod).
 */
export default (async function localAuthRoutes(fastify: any, opts: any) {
  const env = loadEnv();

  fastify.post('/auth/local/login', {
    // Brute-force guard on the real password path (the global IP limiter is too loose for a
    // credential endpoint). Mirrors /auth/refresh.
    config: { rateLimit: { max: 5, timeWindow: '1 minute' } },
    schema: {
      body: z.object({
        email: z.string().email().max(200),
        password: z.string().min(1).max(200),
      }),
    },
  }, async (request: any, reply: any) => {
    const { email, password } = request.body as { email: string; password: string };
    const db = (opts as any)?.db || (fastify as any).db;
    if (!db) return reply.status(500).send({ error: 'DB not configured' });

    // ---- Path 1: flag-gated dev bypass (inert on prod) -------------------------------
    const devEmail = env.DEV_LOGIN_EMAIL;
    const devPassword = env.DEV_LOGIN_PASSWORD;
    if (
      devLoginAllowed(env) &&
      !!devEmail && !!devPassword &&
      timingSafeStrEqual(email.toLowerCase(), devEmail.toLowerCase()) &&
      timingSafeStrEqual(password, devPassword)
    ) {
      const ures = await db.query(`SELECT id FROM users WHERE email = $1`, [email.toLowerCase()]);
      if (ures.rowCount === 0) return reply.status(401).send({ error: 'Invalid credentials' });
      const uid = ures.rows[0].id;
      const memRes = await db.query(
        `SELECT location_id FROM memberships WHERE user_id = $1 AND status = 'active'
         ORDER BY (role = 'owner') DESC LIMIT 1`, [uid]);
      const devLoc = memRes.rows[0]?.location_id || null;
      const payload: Record<string, unknown> = { role: 'owner', userId: uid, sub: uid };
      if (devLoc) payload.activeLocationId = devLoc;
      const devToken = await signDevToken(payload as any, '1d');
      return reply.send({ access_token: devToken, userId: uid, activeLocationId: devLoc });
    }

    // ---- Path 2: real argon2 password login ------------------------------------------
    // Guard the pool checkout: under connection contention db.connect() rejects after
    // connectionTimeoutMillis — without this it surfaces as a 500 ("login failed") on the
    // first hit under load (the cold/load-correlated login 500). Return a graceful 503 the
    // UI can show as "try again", matching the order-create path.
    let client;
    try {
      client = await db.connect();
    } catch (err) {
      request.log.error({ err }, '[auth] failed to acquire DB connection');
      return reply.status(503).send({ error: 'Service temporarily unavailable, please try again' });
    }
    try {
      const res = await client.query(
        `SELECT id, password_hash, display_name FROM users WHERE email = $1`,
        [email.toLowerCase()]
      );
      if (res.rowCount === 0) {
        return reply.status(401).send({ error: 'Invalid email or password' });
      }
      const user = res.rows[0];

      if (!user.password_hash) {
        return reply.status(401).send({ error: 'Account uses another sign-in method' });
      }
      let valid = false;
      try {
        const argon2 = await import('argon2');
        valid = await argon2.default.verify(user.password_hash, password);
      } catch (err: any) {
        request.log.error({ err }, '[auth] argon2 verify failed');
        return reply.status(500).send({ error: 'Authentication unavailable' });
      }
      if (!valid) {
        return reply.status(401).send({ error: 'Invalid email or password' });
      }

      // Resolve role + active location. A membership carries the location directly (owner
      // memberships first); fall back to an org the user owns. Without activeLocationId the
      // owner UI can't scope and shows an empty/onboarding state.
      let role = 'customer';
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
          const orgRes = await client.query(`SELECT id FROM organizations WHERE owner_id = $1`, [user.id]);
          if (orgRes.rowCount > 0) {
            role = 'owner';
            const locRes = await client.query(
              `SELECT id FROM locations WHERE org_id = $1 AND status = 'active' ORDER BY created_at LIMIT 1`,
              [orgRes.rows[0].id]
            );
            if (locRes.rowCount > 0) activeLocationId = locRes.rows[0].id;
          }
        }
      } catch (err) {
        request.log.error(err);
        role = 'customer';
      }

      const familyId = crypto.randomUUID();
      const tokenPayload: Record<string, unknown> = { role, userId: user.id, sub: user.id };
      if (activeLocationId) tokenPayload.activeLocationId = activeLocationId;
      // Short-lived access (1h) + rotating refresh (7d): the web app refreshes on 401, so the
      // session rolls forward up to the refresh window without a surprise logout.
      const accessToken = await signAuthToken(tokenPayload as any, '1h');
      const refreshToken = crypto.randomBytes(32).toString('hex');
      const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');
      try {
        await client.query(
          `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
           VALUES ($1, $2, $3, now() + interval '7 days')`,
          [user.id, familyId, refreshTokenHash]
        );
      } catch (err: any) {
        request.log.warn({ err: err?.message }, '[auth] refresh token insert failed');
      }

      return { access_token: accessToken, refresh_token: refreshToken, userId: user.id, activeLocationId };
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
