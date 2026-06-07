// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import argon2 from 'argon2';
import { requireLocationAccess } from '../../plugins/auth.js';
import { decryptPII } from '../../lib/pii-cipher.js';
import { maskStr } from '../../lib/pii-mask.js';

export default (async function ownerCourierRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.addHook('preValidation', requireLocationAccess);

  // 1. List active members
  fastify.get('/api/owner/locations/:locationId/couriers', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    
    // We get couriers for this location
    const res = await db.query(
      `SELECT c.id, c.email_encrypted, c.phone_encrypted, c.full_name_encrypted, c.status, c.last_login_at, cl.role 
       FROM couriers c
       JOIN courier_locations cl ON c.id = cl.courier_id
       WHERE cl.location_id = $1`,
      [locationId]
    );

    const couriers = res.rows.map((row: any) => {
      const emailPlain = decryptPII(row.email_encrypted);
      const phonePlain = row.phone_encrypted ? decryptPII(row.phone_encrypted) : null;
      const fullName = decryptPII(row.full_name_encrypted);

      return {
        id: row.id,
        status: row.status,
        role: row.role,
        last_login_at: row.last_login_at,
        full_name: fullName,
        masked_email: maskStr(emailPlain),
        masked_phone: phonePlain ? maskStr(phonePlain) : null
      };
    });

    return reply.send({ couriers });
  });

  // 2. Update courier (status, role)
  fastify.patch('/api/owner/locations/:locationId/couriers/:courierId', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), courierId: z.string().uuid() }),
      body: z.object({
        status: z.enum(['active', 'suspended', 'deactivated']).optional(),
        role: z.enum(['courier', 'dispatcher']).optional()
      }).strict()
    }
  }, async (request, reply) => {
    const { locationId, courierId } = request.params;
    const { status, role } = request.body;
    const ownerId = request.user!.userId;
    const ipHash = crypto.createHash('sha256').update(request.ip).digest('hex');
    const uaHash = crypto.createHash('sha256').update(request.headers['user-agent'] || '').digest('hex');

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // Verify membership
      const memberRes = await client.query(
        `SELECT role FROM courier_locations WHERE courier_id = $1 AND location_id = $2`,
        [courierId, locationId]
      );
      if (memberRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'Courier not found in location' });
      }

      if (status) {
        await client.query(
          `UPDATE couriers SET status = $1, deactivated_at = CASE WHEN $1 = 'deactivated' THEN now() ELSE null END, deactivated_by_owner_id = CASE WHEN $1 = 'deactivated' THEN $2::uuid ELSE null END WHERE id = $3`,
          [status, ownerId, courierId]
        );
        
        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash)
           VALUES ($1, $2, $3, 'owner', $4, $5, $6)`,
          [courierId, locationId, `courier.${status}`, ownerId, ipHash, uaHash]
        );

        if (status === 'deactivated' || status === 'suspended') {
          // Revoke all sessions
          await client.query(
            `UPDATE courier_sessions SET revoked_at = now() WHERE courier_id = $1 AND revoked_at IS NULL`,
            [courierId]
          );
        }
      }

      if (role) {
        await client.query(
          `UPDATE courier_locations SET role = $1 WHERE courier_id = $2 AND location_id = $3`,
          [role, courierId, locationId]
        );

        await client.query(
          `INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash, metadata)
           VALUES ($1, $2, 'courier.role_changed', 'owner', $3, $4, $5, $6)`,
          [courierId, locationId, ownerId, ipHash, uaHash, JSON.stringify({ new_role: role })]
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

  // 4. Live Map Data
  fastify.get('/api/owner/locations/:locationId/couriers/live', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    
    const client = await db.connect();
    try {
      await client.query(`SET LOCAL app.current_tenant = $1`, [locationId]);

      const res = await client.query(`
        SELECT 
          c.id as courier_id, 
          c.full_name_encrypted, 
          c.phone_encrypted,
          cs.id as shift_id,
          cs.status as shift_status,
          cs.last_heartbeat_at,
          cp.lat,
          cp.lng,
          ca.id as assignment_id,
          ca.status as assignment_status,
          ca.order_id
        FROM courier_shifts cs
        JOIN couriers c ON cs.courier_id = c.id
        LEFT JOIN courier_positions cp ON cp.shift_id = cs.id AND cp.recorded_at = (
          SELECT MAX(recorded_at) FROM courier_positions WHERE shift_id = cs.id
        )
        LEFT JOIN courier_assignments ca ON ca.shift_id = cs.id AND ca.status IN ('assigned', 'accepted', 'picked_up')
        WHERE cs.location_id = $1 AND cs.status IN ('available', 'on_delivery')
      `, [locationId]);

      const couriers = res.rows.map(row => {
        const fullName = decryptPII(row.full_name_encrypted);
        const phone = decryptPII(row.phone_encrypted);
        
        return {
          courierId: row.courier_id,
          nameMasked: fullName.substring(0, 1) + '***',
          phoneMasked: maskStr(phone),
          status: row.shift_status,
          position: row.lat !== null && row.lng !== null ? { lat: Number(row.lat), lng: Number(row.lng) } : null,
          lastHeartbeatAt: row.last_heartbeat_at,
          currentAssignment: row.assignment_id ? {
            id: row.assignment_id,
            status: row.assignment_status,
            orderId: row.order_id
          } : null
        };
      });

      return reply.send({ success: true, couriers });
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
