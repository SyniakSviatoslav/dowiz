import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import argon2 from 'argon2';
import crypto from 'node:crypto';
import { encryptPII } from '../../lib/pii-cipher.js';
import { signAuthToken } from '@deliveryos/platform';

export default (async function courierAuthRoutes(fastify, opts) {
  const { db } = opts as any;

  // Constants for argon2
  const hashOptions = {
    type: argon2.argon2id,
    memoryCost: 65536,
    timeCost: 3,
    parallelism: 4
  };

  // 1. Redeem Invite
  fastify.post('/api/courier/invites/:inviteId/redeem', {
    schema: {
      params: z.object({ inviteId: z.string().uuid() }),
      body: z.object({
        email: z.string().email().transform(e => e.toLowerCase().trim()),
        code: z.string().min(1),
        password: z.string().min(12),
        full_name: z.string().min(1),
        phone: z.string().optional()
      }).strict()
    },
    config: {
      rateLimit: {
        max: 5,
        timeWindow: '15 minutes'
      }
    }
  }, async (request, reply) => {
    const { inviteId } = request.params;
    const { email, code, password, full_name, phone } = request.body;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');
    
    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const inviteRes = await client.query(
        `SELECT * FROM courier_invites WHERE id = $1 AND expires_at > now() AND used_at IS NULL AND revoked_at IS NULL FOR UPDATE`,
        [inviteId]
      );

      if (inviteRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(410).send({ error: 'Invite invalid, expired, or already used', code: 'INVITE_INVALID' });
      }

      const invite = inviteRes.rows[0];

      // Verify code
      const validCode = await argon2.verify(invite.code_hash, code);
      if (!validCode) {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Invalid code', code: 'INVALID_CODE' });
      }

      // Hash password
      const passwordHash = await argon2.hash(password, hashOptions);

      // Encrypt PII
      const emailHash = crypto.createHash('sha256').update(email).digest('hex');
      const emailEncrypted = encryptPII(email);
      const fullNameEncrypted = encryptPII(full_name);
      let phoneEncrypted = null;
      let phoneHash = null;
      if (phone) {
        phoneEncrypted = encryptPII(phone);
        phoneHash = crypto.createHash('sha256').update(phone).digest('hex');
      }

      // Create Courier
      const courierRes = await client.query(
        `INSERT INTO couriers (email_encrypted, email_hash, phone_encrypted, phone_hash, full_name_encrypted, password_hash)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (email_hash) DO UPDATE SET password_hash = EXCLUDED.password_hash RETURNING id`,
        [emailEncrypted, emailHash, phoneEncrypted, phoneHash, fullNameEncrypted, passwordHash]
      );
      const courierId = courierRes.rows[0].id;

      // Add location membership
      await client.query(
        `INSERT INTO courier_locations (courier_id, location_id, role, added_by_owner_id)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (courier_id, location_id) DO NOTHING`,
        [courierId, invite.location_id, invite.role, invite.created_by_owner_id]
      );

      // Mark invite used
      await client.query(
        `UPDATE courier_invites SET used_at = now(), used_by_courier_id = $1 WHERE id = $2`,
        [courierId, inviteId]
      );

      // Audit Log
      await client.query(
        `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
         VALUES ($1, $2, 'invite.accepted', 'courier', $1, $3, $4)`,
        [courierId, invite.location_id, ipHash, uaHash]
      );

      // Issue JWT and Session
      const familyId = crypto.randomUUID();
      const tokenPlain = crypto.randomBytes(32).toString('hex');
      const tokenHash = await argon2.hash(tokenPlain, hashOptions);

      const sessionRes = await client.query(
        `INSERT INTO courier_sessions (courier_id, family_id, token_hash, active_location_id, expires_at, ip_hash, user_agent_hash)
         VALUES ($1, $2, $3, $4, now() + interval '30 days', $5, $6) RETURNING id`,
        [courierId, familyId, tokenHash, invite.location_id, ipHash, uaHash]
      );
      const sessionId = sessionRes.rows[0].id;

      // Create Session Token (JWT)
      const tokenStr = await signAuthToken({
        sub: courierId,
        role: 'courier',
        sessionId,
        familyId,
        activeLocationId: invite.location_id
      } as any, '14d');

      await client.query('COMMIT');

      // Do NOT send plaintext email or phone back in API response body (must be masked)
      const maskStr = (str: string) => str.length > 4 ? str.substring(0, 2) + '***' + str.substring(str.length - 2) : '***';
      
      return reply.send({
        jwt,
        refreshToken: `${sessionId}.${tokenPlain}`,
        courier: {
          id: courierId,
          masked_email: maskStr(email),
          full_name: full_name, // allowed in response
          locations: [{ id: invite.location_id, role: invite.role }]
        }
      });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

  // 2. Login
  fastify.post('/api/courier/login', {
    schema: {
      body: z.object({
        email: z.string().email().transform(e => e.toLowerCase().trim()),
        password: z.string().min(1),
        location_id: z.string().uuid()
      }).strict()
    },
    config: {
      rateLimit: {
        max: 5,
        timeWindow: '15 minutes'
        // Ideally keyed by IP + email_hash, but we rely on default IP rate limit here + application logic if needed
      }
    }
  }, async (request, reply) => {
    const { email, password, location_id } = request.body;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');
    const emailHash = crypto.createHash('sha256').update(email).digest('hex');

    const client = await db.connect();
    try {
      const courierRes = await client.query(
        `SELECT id, password_hash, status FROM couriers WHERE email_hash = $1`,
        [emailHash]
      );

      if (courierRes.rowCount === 0) {
        // Dummy verify to prevent timing attacks
        await argon2.verify(await argon2.hash('dummy', hashOptions), password);
        return reply.status(401).send({ error: 'Invalid credentials', code: 'INVALID_CREDENTIALS' });
      }

      const courier = courierRes.rows[0];

      const validPassword = await argon2.verify(courier.password_hash, password);
      if (!validPassword) {
        // Log failed attempt
        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
           VALUES ($1, $2, 'login.failed', 'courier', $1, $3, $4)`,
          [courier.id, location_id, ipHash, uaHash]
        );
        return reply.status(401).send({ error: 'Invalid credentials', code: 'INVALID_CREDENTIALS' });
      }

      if (courier.status === 'deactivated') {
        return reply.status(403).send({ error: 'Courier deactivated', code: 'COURIER_DEACTIVATED' });
      }

      // Check membership
      const membershipRes = await client.query(
        `SELECT role FROM courier_locations WHERE courier_id = $1 AND location_id = $2`,
        [courier.id, location_id]
      );
      if (membershipRes.rowCount === 0) {
        return reply.status(403).send({ error: 'Not authorized for this location', code: 'NOT_AUTHORIZED_FOR_LOCATION' });
      }

      await client.query('BEGIN');

      // Update last login
      await client.query(`UPDATE couriers SET last_login_at = now() WHERE id = $1`, [courier.id]);

      // Audit Log
      await client.query(
        `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
         VALUES ($1, $2, 'login.success', 'courier', $1, $3, $4)`,
        [courier.id, location_id, ipHash, uaHash]
      );

      // Issue JWT and Session
      const familyId = crypto.randomUUID();
      const tokenPlain = crypto.randomBytes(32).toString('hex');
      const tokenHash = await argon2.hash(tokenPlain, hashOptions);

      const sessionRes = await client.query(
        `INSERT INTO courier_sessions (courier_id, family_id, token_hash, active_location_id, expires_at, ip_hash, user_agent_hash)
         VALUES ($1, $2, $3, $4, now() + interval '30 days', $5, $6) RETURNING id`,
        [courier.id, familyId, tokenHash, location_id, ipHash, uaHash]
      );
      const sessionId = sessionRes.rows[0].id;

      const jwt = await signAuthToken({
        role: 'courier',
        userId: courier.id,
        activeLocationId: location_id,
        jti: sessionId
      }, '24h');

      await client.query('COMMIT');

      return reply.send({ jwt, refreshToken: `${sessionId}.${tokenPlain}` });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

  // 3. Refresh
  fastify.post('/api/courier/refresh', {
    schema: {
      body: z.object({
        refresh_token: z.string().min(1)
      }).strict()
    },
    config: {
      rateLimit: {
        max: 10,
        timeWindow: '1 minute'
      }
    }
  }, async (request, reply) => {
    const { refresh_token } = request.body;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // We need to find the session that matches this token. 
      // But we hashed the token using argon2, so we can't look it up directly.
      // Wait. The refresh rotation pattern says we need to look up the session by ID or family, OR we just do a table scan? No.
      // P1-3 used argon2id for token_hash. But how do we find the session?
      // Usually, refresh tokens are formatted as `sessionId.tokenPlain` so we can look up by ID.
      // Let's assume the refresh_token is just the plain string, wait. In `login` we issued `refreshToken: tokenPlain`.
      // If we used argon2, we can't query by it without a sequential scan!
      // This means we must change the format to include the session ID! `jti: sessionId` is in the JWT.
      // If the client sends only the refresh token, they must send the JWT too? Or the `refresh_token` should contain the ID.
      // Ah! In P1-3 we probably returned `refreshToken: sessionId + '.' + tokenPlain`. Let's fix that.
      
      const parts = refresh_token.split('.');
      if (parts.length !== 2) {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Invalid refresh token format', code: 'INVALID_REFRESH_TOKEN' });
      }

      const sessionId = parts[0];
      const tokenPlain = parts[1];

      const sessionRes = await client.query(
        `SELECT * FROM courier_sessions WHERE id = $1 FOR UPDATE NOWAIT`,
        [sessionId]
      );

      if (sessionRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Session not found', code: 'SESSION_NOT_FOUND' });
      }

      const session = sessionRes.rows[0];

      const validToken = await argon2.verify(session.token_hash, tokenPlain);
      if (!validToken) {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Invalid refresh token', code: 'INVALID_REFRESH_TOKEN' });
      }

      if (session.revoked_at) {
        // Reuse detected -> Revoke family
        await client.query(`UPDATE courier_sessions SET revoked_at = now() WHERE family_id = $1`, [session.family_id]);
        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
           VALUES ($1, $2, 'session.revoked.family_compromised', 'courier', $1, $3, $4)`,
          [session.courier_id, session.active_location_id, ipHash, uaHash]
        );
        await client.query('COMMIT');
        return reply.status(401).send({ error: 'Refresh token reused', code: 'REFRESH_REUSED' });
      }

      if (new Date() > new Date(session.expires_at)) {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Refresh token expired', code: 'REFRESH_EXPIRED' });
      }

      // Check courier status
      const courierRes = await client.query(`SELECT status FROM couriers WHERE id = $1`, [session.courier_id]);
      if (courierRes.rows[0].status !== 'active') {
        await client.query('ROLLBACK');
        return reply.status(401).send({ error: 'Courier deactivated', code: 'COURIER_DEACTIVATED' });
      }

      // Rotate session
      await client.query(`UPDATE courier_sessions SET revoked_at = now(), last_used_at = now() WHERE id = $1`, [session.id]);

      const newTokenPlain = crypto.randomBytes(32).toString('hex');
      const newTokenHash = await argon2.hash(newTokenPlain, hashOptions);

      const newSessionRes = await client.query(
        `INSERT INTO courier_sessions (courier_id, family_id, token_hash, active_location_id, expires_at, ip_hash, user_agent_hash, replaced_by)
         VALUES ($1, $2, $3, $4, now() + interval '30 days', $5, $6, $7) RETURNING id`,
        [session.courier_id, session.family_id, newTokenHash, session.active_location_id, ipHash, uaHash, session.id]
      );
      const newSessionId = newSessionRes.rows[0].id;

      await client.query(
        `UPDATE courier_sessions SET replaced_by = $1 WHERE id = $2`,
        [newSessionId, session.id]
      );

      const jwt = await signAuthToken({
        role: 'courier',
        userId: session.courier_id,
        activeLocationId: session.active_location_id,
        jti: newSessionId
      }, '24h');

      await client.query('COMMIT');

      return reply.send({ jwt, refreshToken: `${newSessionId}.${newTokenPlain}` });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

  // 4. Logout
  fastify.post('/api/courier/logout', {
    schema: {
      body: z.object({
        refresh_token: z.string().min(1)
      }).strict()
    }
  }, async (request, reply) => {
    const { refresh_token } = request.body;
    const parts = refresh_token.split('.');
    if (parts.length !== 2) {
      return reply.send({ success: true }); // Ignore invalid
    }
    const sessionId = parts[0];

    const client = await db.connect();
    try {
      await client.query(
        `UPDATE courier_sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL`,
        [sessionId]
      );
      return reply.send({ success: true });
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
