// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import argon2 from 'argon2';
import { requireLocationAccess } from '../../plugins/auth.js';

export default (async function ownerCourierInvitesRoutes(fastify, opts) {
  const { db } = opts as any;

  // Constants for argon2
  const hashOptions = {
    type: argon2.argon2id,
    memoryCost: 65536,
    timeCost: 3,
    parallelism: 4
  };

  fastify.addHook('preValidation', requireLocationAccess);

  // 1. Create Invite
  fastify.post('/api/owner/locations/:locationId/courier-invites', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({
        role: z.enum(['courier', 'dispatcher']),
        email: z.string().email().transform(e => e.toLowerCase().trim()),
        ttl_hours: z.number().int().min(1).max(168).optional().default(48)
      }).strict()
    },
    config: {
      rateLimit: {
        max: 10,
        timeWindow: '1 hour'
      }
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { role, email, ttl_hours } = request.body;
    const ownerId = request.user!.userId;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');

    // Generate random code
    const code = crypto.randomBytes(8).toString('hex'); // 16 chars
    const codeHash = await argon2.hash(code, hashOptions);
    const emailHash = crypto.createHash('sha256').update(email).digest('hex');

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const res = await client.query(
        `INSERT INTO courier_invites (location_id, created_by_owner_id, role, invited_email_hash, code_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + interval '1 hour' * $6) RETURNING id, expires_at`,
        [locationId, ownerId, role, emailHash, codeHash, ttl_hours]
      );

      const invite = res.rows[0];

      await client.query(
        `INSERT INTO courier_audit_log (location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
         VALUES ($1, 'invite.created', 'owner', $2, $3, $4)`,
        [locationId, ownerId, ipHash, uaHash]
      );

      await client.query('COMMIT');

      const deepLink = `https://dowiz.org/courier-invite/${invite.id}`;

      return reply.send({
        inviteId: invite.id,
        code, // Return code ONCE, it is never stored in plaintext
        deepLink,
        expiresAt: invite.expires_at
      });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

  // 2. List Active Invites
  fastify.get('/api/owner/locations/:locationId/courier-invites', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    
    const res = await db.query(
      `SELECT id, role, expires_at, created_at 
       FROM courier_invites 
       WHERE location_id = $1 AND used_at IS NULL AND revoked_at IS NULL AND expires_at > now()`,
      [locationId]
    );

    return reply.send({ invites: res.rows });
  });

  // 3. Revoke Invite
  fastify.delete('/api/owner/locations/:locationId/courier-invites/:inviteId', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), inviteId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId, inviteId } = request.params;
    const ownerId = request.user!.userId;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      const res = await client.query(
        `UPDATE courier_invites SET revoked_at = now() 
         WHERE id = $1 AND location_id = $2 AND used_at IS NULL AND revoked_at IS NULL`,
        [inviteId, locationId]
      );

      if (res.rowCount > 0) {
        await client.query(
          `INSERT INTO courier_audit_log (location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
           VALUES ($1, 'invite.revoked', 'owner', $2, $3, $4)`,
          [locationId, ownerId, ipHash, uaHash]
        );
      }

      await client.query('COMMIT');
      return reply.send({ success: true });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
