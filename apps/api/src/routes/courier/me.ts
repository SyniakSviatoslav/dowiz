// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import argon2 from 'argon2';
import { decryptPII } from '../../lib/pii-cipher.js';
import { maskStr } from '../../lib/pii-mask.js';

export default (async function courierMeRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.addHook('preHandler', fastify.verifyAuth);
  fastify.addHook('preHandler', fastify.requireRole(['courier']));

  // 1. Get Profile
  fastify.get('/me', async (request, reply) => {
    const courierId = request.user!.userId;
    const locationId = request.user!.activeLocationId;

    const res = await db.query(
      `SELECT c.email_encrypted, c.phone_encrypted, c.full_name_encrypted, c.last_login_at, cl.role 
       FROM couriers c
       JOIN courier_locations cl ON c.id = cl.courier_id
       WHERE c.id = $1 AND cl.location_id = $2`,
      [courierId, locationId]
    );

    if (res.rowCount === 0) {
      return reply.status(404).send({ error: 'Not found' });
    }

    const row = res.rows[0];
    const emailPlain = decryptPII(row.email_encrypted);
    const phonePlain = row.phone_encrypted ? decryptPII(row.phone_encrypted) : null;
    const fullName = decryptPII(row.full_name_encrypted);

    return reply.send({
      id: courierId,
      full_name: fullName,
      masked_email: maskStr(emailPlain),
      masked_phone: phonePlain ? maskStr(phonePlain) : null,
      last_login_at: row.last_login_at,
      active_location: { id: locationId, role: row.role }
    });
  });

  // 2. Audit Log
  fastify.get('/me/audit-log', async (request, reply) => {
    const courierId = request.user!.userId;

    const res = await db.query(
      `SELECT action, actor_kind, created_at 
       FROM courier_audit_log 
       WHERE courier_id = $1 
       ORDER BY created_at DESC 
       LIMIT 50`,
      [courierId]
    );

    return reply.send({ logs: res.rows });
  });

  // 3. Change Password
  fastify.patch('/me/password', {
    schema: {
      body: z.object({
        current_password: z.string().min(1),
        new_password: z.string().min(12)
      }).strict()
    }
  }, async (request, reply) => {
    const { current_password, new_password } = request.body;
    const courierId = request.user!.userId;
    const locationId = request.user!.activeLocationId;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');

    const client = await db.connect();
    try {
      const courierRes = await client.query(`SELECT password_hash FROM couriers WHERE id = $1`, [courierId]);
      if (courierRes.rowCount === 0) {
        return reply.status(404).send({ error: 'Courier not found' });
      }

      const courier = courierRes.rows[0];
      const valid = await argon2.verify(courier.password_hash, current_password);
      if (!valid) {
        return reply.status(400).send({ error: 'Invalid current password' });
      }

      const hashOptions = {
        type: argon2.argon2id,
        memoryCost: 65536,
        timeCost: 3,
        parallelism: 4
      };

      const newHash = await argon2.hash(new_password, hashOptions);

      await client.query('BEGIN');

      await client.query(`UPDATE couriers SET password_hash = $1 WHERE id = $2`, [newHash, courierId]);

      await client.query(
        `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
         VALUES ($1, $2, 'password.changed', 'courier', $1, $3, $4)`,
        [courierId, locationId, ipHash, uaHash]
      );

      // Revoke all sessions to force re-login
      await client.query(
        `UPDATE courier_sessions SET revoked_at = now() WHERE courier_id = $1 AND revoked_at IS NULL`,
        [courierId]
      );

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
