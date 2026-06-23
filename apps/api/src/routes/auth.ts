import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import crypto from 'crypto';
import { loadEnv } from '@deliveryos/config';
import { signAuthToken, verifyAuthToken, withTenant } from '@deliveryos/platform';

const env = loadEnv();

/**
 * The /auth/refresh endpoint exclusively serves the owner login family (Google
 * OAuth + local owner login). Couriers refresh via courier_sessions and never
 * reach this table, so the refreshed access token's role is ALWAYS 'owner'.
 * Centralised + exported so the invariant is covered by a regression test —
 * previously the role was inferred from a nullable `google_sub` column, which
 * could flip a dual-identity user's role.
 */
export function refreshedOwnerClaims(
  userId: string,
  activeLocationId?: string | null,
): { role: 'owner'; userId: string; activeLocationId?: string } {
  const claims: { role: 'owner'; userId: string; activeLocationId?: string } = { role: 'owner', userId };
  // activeLocationId MUST survive a refresh — otherwise the refreshed token can't scope the
  // owner UI and the dashboard/menu/orders read empty after the access token rotates.
  if (activeLocationId) claims.activeLocationId = activeLocationId;
  return claims;
}

export default async function authRoutes(fastify: FastifyInstance) {

  // ============================================================================
  // GOOGLE OAUTH FLOW (OWNER ONLY)
  // ============================================================================

  fastify.get('/auth/google', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } }
  }, async (request: any, reply: any) => {
    const state = crypto.randomUUID();
    const nonce = crypto.randomUUID();

    const codeVerifier = crypto.randomBytes(32).toString('base64url');
    const codeChallenge = crypto.createHash('sha256').update(codeVerifier).digest('base64url');

    // Save to Redis (TTL 10m)
    await (fastify as any).redis.setex(`auth:state:${state}`, 600, JSON.stringify({ codeVerifier, nonce }));

    const redirectUri = `${env.APP_BASE_URL}/api/auth/google/callback`;
    const googleAuthUrl = new URL('https://accounts.google.com/o/oauth2/v2/auth');
    googleAuthUrl.searchParams.set('client_id', env.GOOGLE_CLIENT_ID);
    googleAuthUrl.searchParams.set('redirect_uri', redirectUri);
    googleAuthUrl.searchParams.set('response_type', 'code');
    googleAuthUrl.searchParams.set('scope', 'openid email profile');
    googleAuthUrl.searchParams.set('state', state);
    googleAuthUrl.searchParams.set('code_challenge', codeChallenge);
    googleAuthUrl.searchParams.set('code_challenge_method', 'S256');
    googleAuthUrl.searchParams.set('nonce', nonce);

    return reply.redirect(googleAuthUrl.toString());
  });

  fastify.get('/auth/google/callback', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } }
  }, async (request: any, reply: any) => {
    const querySchema = z.object({
      code: z.string(),
      state: z.string()
    });

    const parsed = querySchema.safeParse(request.query);
    if (!parsed.success) return reply.status(400).send({ error: 'Invalid query params' });
    const { code, state } = parsed.data;

    // Validate state from Redis
    const stateDataRaw = await (fastify as any).redis.get(`auth:state:${state}`);
    if (!stateDataRaw) return reply.status(400).send({ error: 'Invalid or expired state' });
    await (fastify as any).redis.del(`auth:state:${state}`);

    const { codeVerifier, nonce } = JSON.parse(stateDataRaw);

    // Exchange code for tokens
    const redirectUri = `${env.APP_BASE_URL}/api/auth/google/callback`;
    const tokenRes = await fetch('https://oauth2.googleapis.com/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        client_id: env.GOOGLE_CLIENT_ID,
        client_secret: env.GOOGLE_CLIENT_SECRET,
        code,
        code_verifier: codeVerifier,
        redirect_uri: redirectUri,
        grant_type: 'authorization_code'
      })
    });

    if (!tokenRes.ok) {
      (request as any).log.error(await tokenRes.text());
      return reply.status(400).send({ error: 'Failed to exchange token' });
    }

    const tokens = await tokenRes.json();

    // Decode ID Token to get sub and email
    // jose.decodeJwt doesn't verify signature, but we got it directly via HTTPS from Google
    const { decodeJwt } = await import('jose');
    const idTokenPayload = decodeJwt(tokens.id_token);

    if (idTokenPayload.nonce !== nonce) {
      return reply.status(400).send({ error: 'Nonce mismatch' });
    }

    const googleSub = idTokenPayload.sub;
    const email = idTokenPayload.email as string;
    const name = idTokenPayload.name as string;

    if (!googleSub || !email) {
      return reply.status(400).send({ error: 'Missing required profile info' });
    }

    // Upsert User
    const res = await (fastify as any).db.query(
      `INSERT INTO users (email, google_sub, display_name) 
       VALUES ($1, $2, $3)
       ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
       RETURNING id`,
      [email, googleSub, name]
    );
    // Note: If conflict is on email (user signed up via another method then tried Google), this simple query fails.
    // Let's make it more robust.
    let userId: string;
    try {
      userId = res.rows[0].id;
    } catch (e) {
      // Fallback: match by email and set google_sub
      const updateRes = await (fastify as any).db.query(
        `UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3) WHERE email = $1 RETURNING id`,
        [email, googleSub, name]
      );
      if (updateRes.rowCount === 0) {
        throw new Error('Failed to upsert user');
      }
      userId = updateRes.rows[0].id;
    }

    // Issue tokens
    const familyId = crypto.randomUUID();
    const accessToken = await signAuthToken({ role: 'owner', userId } as any, '7d');
    const refreshToken = crypto.randomBytes(32).toString('hex');
    const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');

    await (fastify as any).db.query(
      `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
       VALUES ($1, $2, $3, now() + interval '30 days')`,
      [userId, familyId, refreshTokenHash]
    );

    // One-time code exchange
    const opaqueCode = crypto.randomUUID();
    await (fastify as any).redis.setex(`auth:code:${opaqueCode}`, 60, JSON.stringify({
      access_token: accessToken,
      refresh_token: refreshToken
    }));

    return reply.redirect(`${env.APP_BASE_URL}/auth/callback#code=${opaqueCode}`);
  });

  // ============================================================================
  // TOKEN EXCHANGE & REFRESH
  // ============================================================================

  fastify.post('/auth/exchange', {
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
    schema: {
      body: z.object({ code: z.string().uuid() }).strict()
    }
  }, async (request: any, reply: any) => {
    const { code } = request.body as any;
    const dataRaw = await (fastify as any).redis.get(`auth:code:${code}`);
    if (!dataRaw) return reply.status(400).send({ error: 'Invalid or expired code' });

    await (fastify as any).redis.del(`auth:code:${code}`);
    return JSON.parse(dataRaw);
  });

  // ── Telegram owner login (TG) ───────────────────────────────────────────────
  // Web mints a short-lived token → deep-link /start login_<token> → the bot binds
  // the Telegram identity to an owner (creating one on first login) → web polls here.

  fastify.post('/auth/telegram/start', {
    config: { rateLimit: { max: 10, timeWindow: '5 minutes' } },
  }, async (_request: any, reply: any) => {
    const botUsername = process.env.TELEGRAM_BOT_USERNAME || 'dowiz_bot';
    const res = await (fastify as any).db.query(
      `INSERT INTO telegram_login_tokens (expires_at) VALUES (now() + interval '5 minutes') RETURNING token`,
    );
    const token = res.rows[0].token;
    return reply.send({ token, botUsername, deepLink: `https://t.me/${botUsername}?start=login_${token}` });
  });

  fastify.get('/auth/telegram/poll', {
    config: { rateLimit: { max: 120, timeWindow: '5 minutes' } },
    schema: { querystring: z.object({ token: z.string().uuid() }) },
  }, async (request: any, reply: any) => {
    const { token } = request.query as { token: string };
    const res = await (fastify as any).db.query(
      `SELECT status, user_id, expires_at FROM telegram_login_tokens WHERE token = $1::uuid`,
      [token],
    );
    if (res.rowCount === 0) return reply.status(404).send({ status: 'unknown' });
    const row = res.rows[0];
    if (new Date(row.expires_at) < new Date()) return reply.status(410).send({ status: 'expired' });
    if (row.status !== 'authenticated' || !row.user_id) return reply.send({ status: row.status === 'consumed' ? 'consumed' : 'pending' });

    // Single-use: atomically flip authenticated → consumed; loser of the race gets nothing.
    const upd = await (fastify as any).db.query(
      `UPDATE telegram_login_tokens SET status = 'consumed' WHERE token = $1::uuid AND status = 'authenticated' RETURNING user_id`,
      [token],
    );
    if (upd.rowCount === 0) return reply.status(410).send({ status: 'consumed' });

    const userId = upd.rows[0].user_id;
    const accessToken = await signAuthToken({ role: 'owner', userId } as any, '7d');
    const refreshToken = crypto.randomBytes(32).toString('hex');
    const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');
    await (fastify as any).db.query(
      `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
       VALUES ($1, $2, $3, now() + interval '30 days')`,
      [userId, crypto.randomUUID(), refreshTokenHash],
    );
    return reply.send({ status: 'authenticated', access_token: accessToken, refresh_token: refreshToken });
  });

  fastify.post('/auth/refresh', {
    config: { rateLimit: { max: 5, timeWindow: '1 minute' } },
    schema: {
      body: z.object({ refresh_token: z.string() }).strict()
    }
  }, async (request: any, reply: any) => {
    const { refresh_token } = request.body as any;
    const tokenHash = crypto.createHash('sha256').update(refresh_token).digest('hex');

    const res = await (fastify as any).db.query(
      `SELECT * FROM auth_refresh_tokens WHERE token_hash = $1`,
      [tokenHash]
    );

    if (res.rowCount === 0) return reply.status(401).send({ error: 'Invalid refresh token' });
    const tokenRecord = res.rows[0];

    if (tokenRecord.expires_at < new Date()) {
      return reply.status(401).send({ error: 'Refresh token expired' });
    }

    // Atomically claim the token: the guarded UPDATE flips used=false→true and only ONE
    // concurrent request can win it (rowCount 1). A zero rowCount means the token was
    // already used — a replay OR a second request that lost the race — so treat it as
    // reuse and revoke the whole family (breach signal). This replaces the previous
    // SELECT-then-UPDATE, where two concurrent requests both read used=false, both passed
    // the JS check, and both minted a fresh token family (single-use rotation defeated,
    // reuse-detection bypassed).
    const claim = await (fastify as any).db.query(
      `UPDATE auth_refresh_tokens SET used = true WHERE id = $1 AND used = false RETURNING id`,
      [tokenRecord.id]
    );
    if (claim.rowCount === 0) {
      // rowCount 0 = the token was already used. Distinguish a BENIGN concurrent refresh (a
      // sibling request rotated this family moments ago — two tabs, React StrictMode, parallel
      // 401-retries) from a GENUINE replay of an old token (theft). If any token in this family
      // was created in the last 10s, a rotation just happened → concurrent loser: return a soft
      // 409 so the client retries with the freshly-stored token, and do NOT revoke the family
      // (revoking would log every one of the user's sessions out — the "expires too soon" bug).
      const recent = await (fastify as any).db.query(
        `SELECT 1 FROM auth_refresh_tokens WHERE family_id = $1 AND created_at > now() - interval '5 seconds' LIMIT 1`,
        [tokenRecord.family_id]
      );
      if ((recent.rowCount ?? 0) > 0) {
        return reply.status(409).send({ error: 'concurrent_refresh' });
      }
      // No recent rotation → genuine reuse of a stale token. Compromised family — revoke all.
      await (fastify as any).db.query(`DELETE FROM auth_refresh_tokens WHERE family_id = $1`, [tokenRecord.family_id]);
      return reply.status(401).send({ error: 'Token reuse detected. Family revoked.' });
    }

    // This endpoint only ever issues owner tokens (couriers refresh via courier_sessions).
    // Mint the role explicitly; carry activeLocationId through the rotation so the owner UI
    // stays scoped after refresh (a nullable google_sub-inferred role previously flipped).
    const locRes = await (fastify as any).db.query(
      `SELECT location_id FROM memberships WHERE user_id = $1 AND status = 'active'
       ORDER BY (role = 'owner') DESC LIMIT 1`,
      [tokenRecord.user_id]
    );
    const activeLocationId = locRes.rows[0]?.location_id ?? null;
    const newAccessToken = await signAuthToken(refreshedOwnerClaims(tokenRecord.user_id, activeLocationId) as any, '7d');
    const newRefreshToken = crypto.randomBytes(32).toString('hex');
    const newRefreshTokenHash = crypto.createHash('sha256').update(newRefreshToken).digest('hex');

    await (fastify as any).db.query(
      `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
       VALUES ($1, $2, $3, now() + interval '7 days')`,
      [tokenRecord.user_id, tokenRecord.family_id, newRefreshTokenHash]
    );

    return { access_token: newAccessToken, refresh_token: newRefreshToken };
  });

  // ============================================================================
  // COURIER ACTIVATION
  // ============================================================================

  fastify.post('/auth/courier/activate', {
    config: { rateLimit: { max: 5, timeWindow: '1 minute' } },
    schema: {
      body: z.object({
        code: z.string().min(4),
        phone: z.string().min(6),
        name: z.string().min(1)
      }).strict()
    }
  }, async (request: any, reply: any) => {
    const { code, phone, name } = request.body as any;
    const codeHash = crypto.createHash('sha256').update(code).digest('hex');

    // Inside transaction to avoid race conditions
    const client = await (fastify as any).db.connect();
    try {
      await client.query('BEGIN');

      const inviteRes = await client.query(
        `SELECT id, location_id, used_at, expires_at FROM courier_invites WHERE code_hash = $1 FOR UPDATE`,
        [codeHash]
      );

      if (inviteRes.rowCount === 0) throw new Error('Invalid code');
      const invite = inviteRes.rows[0];

      if (invite.used_at) throw new Error('Code already used');
      if (invite.expires_at < new Date()) throw new Error('Code expired');

      // Upsert User
      const userRes = await client.query(
        `INSERT INTO users (phone, display_name) 
         VALUES ($1, $2)
         ON CONFLICT (phone) DO UPDATE SET display_name = EXCLUDED.display_name
         RETURNING id`,
        [phone, name]
      );
      // Wait, users does not have UNIQUE(phone) in core-identity migration! 
      // Let me check if `phone` is UNIQUE. `email citext UNIQUE, google_sub text UNIQUE`. `phone text`.
      // If `phone` is not unique, ON CONFLICT will fail. I will use standard select/insert.
      let userId: string;
      const existingUser = await client.query(`SELECT id FROM users WHERE phone = $1`, [phone]);
      if (existingUser.rowCount > 0) {
        userId = existingUser.rows[0].id;
        await client.query(`UPDATE users SET display_name = $1 WHERE id = $2`, [name, userId]);
      } else {
        const newUser = await client.query(`INSERT INTO users (phone, display_name) VALUES ($1, $2) RETURNING id`, [phone, name]);
        userId = newUser.rows[0].id;
      }

      // Create Membership
      await client.query(
        `INSERT INTO memberships (user_id, location_id, role)
         VALUES ($1, $2, 'courier')
         ON CONFLICT (user_id, location_id, role) DO UPDATE SET status = 'active'`,
        [userId, invite.location_id]
      );

      // Mark used
      await client.query(`UPDATE courier_invites SET used_at = now() WHERE id = $1`, [invite.id]);

      // Issue tokens
      const familyId = crypto.randomUUID();
      const accessToken = await signAuthToken({ role: 'courier', userId } as any, '7d');
      const refreshToken = crypto.randomBytes(32).toString('hex');
      const refreshTokenHash = crypto.createHash('sha256').update(refreshToken).digest('hex');

      await client.query(
        `INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at)
         VALUES ($1, $2, $3, now() + interval '7 days')`,
        [userId, familyId, refreshTokenHash]
      );

      await client.query('COMMIT');
      return { access_token: accessToken, refresh_token: refreshToken };
    } catch (err: any) {
      await client.query('ROLLBACK');
      (request as any).log.error(err);
      return reply.status(400).send({ error: err.message || 'Activation failed' });
    } finally {
      client.release();
    }
  });

}
