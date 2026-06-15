import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import argon2 from 'argon2';
import { verifyAuth, requireLocationAccess } from '../../plugins/auth.js';
import { decryptPII } from '../../lib/pii-cipher.js';
import { maskStr } from '../../lib/pii-mask.js';

export default (async function ownerCourierRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.addHook('preValidation', verifyAuth);
  fastify.addHook('preValidation', requireLocationAccess);

  // 1. List active members
  fastify.get('/api/owner/locations/:locationId/couriers', async (request: any, reply: any) => {
    const { locationId } = request.params as any;

    const client = await db.connect();
    try {
      // courier_locations has RLS; is_local=true requires an explicit BEGIN
      // so the setting persists beyond the set_config statement itself
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(
        `SELECT c.id, c.email_encrypted, c.phone_encrypted, c.full_name_encrypted, c.status, c.last_login_at, c.created_at, cl.role,
                (SELECT COUNT(*) FROM courier_assignments ca WHERE ca.courier_id = c.id AND ca.status = 'delivered') as deliveries_completed
         FROM couriers c
         JOIN courier_locations cl ON c.id = cl.courier_id
         WHERE cl.location_id = $1`,
        [locationId]
      );

      const couriers = res.rows.map((row: any) => {
        let emailPlain: string | null = null;
        let phonePlain: string | null = null;
        let fullName: string | null = null;
        try { emailPlain = decryptPII(row.email_encrypted); } catch {}
        try { phonePlain = row.phone_encrypted ? decryptPII(row.phone_encrypted) : null; } catch {}
        try { fullName = decryptPII(row.full_name_encrypted); } catch {}

        return {
          id: row.id,
          name: fullName || '',
          maskedPhone: phonePlain ? maskStr(phonePlain) : null,
          maskedEmail: emailPlain ? maskStr(emailPlain) : null,
          status: row.status,
          role: row.role,
          onlineStatus: null,
          ordersToday: parseInt(row.deliveries_completed) || 0,
          rating: 0,
          lastLoginAt: row.last_login_at ?? null,
          createdAt: row.created_at ?? new Date().toISOString(),
        };
      });

      await client.query('COMMIT');
      return reply.send({ couriers });
    } catch (e) {
      await client.query('ROLLBACK').catch(() => {});
      throw e;
    } finally {
      client.release();
    }
  });

  // 2. Update courier (status, role)
  fastify.patch('/api/owner/locations/:locationId/couriers/:courierId', async (request: any, reply: any) => {
    const { locationId, courierId } = request.params as any;
    const body = request.body as any;
    const status = body?.status;
    const role = body?.role;
    const ownerId = (request.user as any).userId;
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
  fastify.get('/api/owner/locations/:locationId/couriers/live', async (request: any, reply: any) => {
    const { locationId } = request.params as any;
    
    const client = await db.connect();
    try {
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

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

      const couriers = res.rows.map((row: any) => {
        const fullName = decryptPII(row.full_name_encrypted) || '';
        const phone = decryptPII(row.phone_encrypted) || '';
        
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

  // 5. Per-courier detail (shifts, earnings, history)
  fastify.get('/api/owner/locations/:locationId/couriers/:courierId/details', async (request: any, reply: any) => {
    const { locationId, courierId } = request.params as any;

    const [shiftsRes, earningsRes, historyRes] = await Promise.all([
      db.query(
        `SELECT id, status, started_at, ended_at, last_heartbeat_at
         FROM courier_shifts
         WHERE courier_id = $1 AND location_id = $2
         ORDER BY started_at DESC LIMIT 10`,
        [courierId, locationId]
      ),
      db.query(
        `SELECT
           COALESCE(SUM(CASE WHEN a.delivered_at >= CURRENT_DATE THEN a.cash_amount ELSE 0 END), 0) AS today,
           COALESCE(SUM(CASE WHEN a.delivered_at >= date_trunc('week', CURRENT_DATE) THEN a.cash_amount ELSE 0 END), 0) AS week,
           COALESCE(SUM(CASE WHEN a.delivered_at >= date_trunc('month', CURRENT_DATE) THEN a.cash_amount ELSE 0 END), 0) AS month,
           COUNT(CASE WHEN a.delivered_at >= CURRENT_DATE THEN 1 END) AS today_deliveries,
           COUNT(CASE WHEN a.delivered_at >= date_trunc('month', CURRENT_DATE) THEN 1 END) AS month_deliveries
         FROM courier_assignments a
         JOIN orders o ON o.id = a.order_id
         WHERE a.courier_id = $1 AND o.location_id = $2 AND a.status = 'delivered'`,
        [courierId, locationId]
      ),
      db.query(
        `SELECT a.id, a.order_id, a.status, a.assigned_at, a.accepted_at, a.picked_up_at, a.delivered_at, a.cash_amount,
                o.total, o.currency_code, o.delivery_address,
                c.name AS customer_name, c.phone AS customer_phone
         FROM courier_assignments a
         JOIN orders o ON o.id = a.order_id
         JOIN customers c ON c.id = o.customer_id
         WHERE a.courier_id = $1 AND o.location_id = $2
         ORDER BY a.created_at DESC LIMIT 20`,
        [courierId, locationId]
      )
    ]);

    return reply.send({
      shifts: shiftsRes.rows,
      earnings: earningsRes.rows[0] || { today: 0, week: 0, month: 0, today_deliveries: 0, month_deliveries: 0 },
      history: historyRes.rows
    });
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
