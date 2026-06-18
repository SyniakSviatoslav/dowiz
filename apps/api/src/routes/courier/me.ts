import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import argon2 from 'argon2';
import { decryptPII } from '../../lib/pii-cipher.js';
import { maskStr } from '../../lib/pii-mask.js';

/**
 * Shape a courier delivery-history row for the client. The customer name is
 * MASKED here — couriers are a lower-trust role and must not receive plaintext
 * customer PII for past deliveries (consistent with every other courier-facing
 * surface). Exported so the masking is covered by a regression test.
 */
export function mapCourierHistoryRow(row: any) {
  return {
    id: row.id,
    orderId: row.order_id,
    date: row.delivered_at || row.created_at,
    restaurant: row.location_name,
    customerAddress: maskStr(row.customer_name),
    amount: parseInt(row.cash_amount) || parseInt(row.total) || 0,
    status: row.status === 'delivered' ? 'DELIVERED' : row.status === 'cancelled' ? 'CANCELLED' : row.status,
    rating: row.rating ?? null,
    feedback: row.feedback ?? null,
  };
}

export default (async function courierMeRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.addHook('preHandler', fastify.verifyAuth);
  fastify.addHook('preHandler', fastify.requireRole(['courier']));

  // 1. Get Profile
  fastify.get('/me', async (request: any, reply: any) => {
    const courierId = request.user!.sub;
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
    let emailPlain: string | null = null;
    let phonePlain: string | null = null;
    let fullName: string | null = null;
    try { emailPlain = decryptPII(row.email_encrypted); } catch (e) { request.log.error({ err: e }, 'decryptPII email failed'); }
    try { phonePlain = row.phone_encrypted ? decryptPII(row.phone_encrypted) : null; } catch (e) { request.log.error({ err: e }, 'decryptPII phone failed'); }
    try { fullName = decryptPII(row.full_name_encrypted); } catch (e) { request.log.error({ err: e }, 'decryptPII fullName failed'); }

    return reply.send({
      id: courierId,
      full_name: fullName ?? '(decryption failed)',
      masked_email: emailPlain ? maskStr(emailPlain) : '(decryption failed)',
      masked_phone: phonePlain ? maskStr(phonePlain) : null,
      last_login_at: row.last_login_at,
      active_location: { id: locationId, role: row.role }
    });
  });

  // 2. Audit Log
  fastify.get('/me/audit-log', async (request: any, reply: any) => {
    const courierId = request.user!.sub;

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
  fastify.patch('/me/password', async (request: any, reply: any) => {
    // Manual Zod parsing to avoid AJV compilation failures
    const bodySchema = z.object({
      current_password: z.string().min(1),
      new_password: z.string().min(12)
    }).strict();

    const result = bodySchema.safeParse(request.body);
    if (!result.success) {
      return reply.status(400).send({ error: 'Validation failed', details: result.error.format() });
    }

    const { current_password, new_password } = result.data;
    const courierId = request.user!.sub;
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

  // 4. Earnings Summary
  fastify.get('/me/earnings', async (request: any, reply: any) => {
    const courierId = request.user!.sub;
    const locationId = request.user!.activeLocationId;

    const locRes = await db.query(`SELECT currency_code FROM locations WHERE id = $1`, [locationId]);
    const locationCurrency = locRes.rows[0]?.currency_code || 'ALL';

    const today = await db.query(`
      SELECT COALESCE(SUM(cash_amount), 0) AS amount, COUNT(*)::int AS deliveries
      FROM courier_assignments
      WHERE courier_id = $1 AND status = 'delivered'
        AND delivered_at >= CURRENT_DATE
        AND location_id = $2
    `, [courierId, locationId]);

    const week = await db.query(`
      SELECT COALESCE(SUM(cash_amount), 0) AS amount, COUNT(*)::int AS deliveries
      FROM courier_assignments
      WHERE courier_id = $1 AND status = 'delivered'
        AND delivered_at >= date_trunc('week', CURRENT_DATE)
        AND location_id = $2
    `, [courierId, locationId]);

    const month = await db.query(`
      SELECT COALESCE(SUM(cash_amount), 0) AS amount, COUNT(*)::int AS deliveries
      FROM courier_assignments
      WHERE courier_id = $1 AND status = 'delivered'
        AND delivered_at >= date_trunc('month', CURRENT_DATE)
        AND location_id = $2
    `, [courierId, locationId]);

    const payouts = await db.query(`
      SELECT id, total_earned AS amount, status, period_start, period_end, created_at
      FROM courier_payouts
      WHERE courier_id = $1
      ORDER BY created_at DESC
      LIMIT 20
    `, [courierId]);

    return reply.send({
      summary: {
        today: parseInt(today.rows[0].amount),
        today_deliveries: today.rows[0].deliveries,
        week: parseInt(week.rows[0].amount),
        week_deliveries: week.rows[0].deliveries,
        month: parseInt(month.rows[0].amount),
        month_deliveries: month.rows[0].deliveries,
        currency: locationCurrency,
      },
      payouts: payouts.rows.map((p: any) => ({
        id: p.id,
        date: p.period_end || p.created_at,
        amount: parseInt(p.amount),
        status: p.status,
        reference: `Payout ${p.period_start?.slice(0, 10) || ''} - ${p.period_end?.slice(0, 10) || ''}`,
      })),
    });
  });

  // 5. Delivery History
  fastify.get('/me/history', async (request: any, reply: any) => {
    const courierId = request.user!.sub;

    const res = await db.query(`
      SELECT a.id, a.order_id, a.status, a.delivered_at, a.cash_amount,
             o.total, o.delivery_fee, o.created_at,
             c.name AS customer_name,
             l.name AS location_name
      FROM courier_assignments a
      JOIN orders o ON o.id = a.order_id
      JOIN customers c ON c.id = o.customer_id
      JOIN locations l ON l.id = o.location_id
      WHERE a.courier_id = $1 AND a.status IN ('delivered', 'cancelled')
      ORDER BY a.delivered_at DESC NULLS LAST, a.created_at DESC
      LIMIT 50
    `, [courierId]);

    // Attach ratings (separate query + try/catch so a missing order_ratings
    // table — before its migration — can't break the history list).
    let ratingsByOrder: Record<string, any> = {};
    try {
      const orderIds = res.rows.map((r: any) => r.order_id);
      if (orderIds.length) {
        const rr = await db.query(`SELECT order_id, rating, feedback FROM order_ratings WHERE order_id = ANY($1)`, [orderIds]);
        ratingsByOrder = Object.fromEntries(rr.rows.map((x: any) => [x.order_id, x]));
      }
    } catch { /* table not yet migrated */ }

    return reply.send(res.rows.map((r: any) => mapCourierHistoryRow({
      ...r,
      rating: ratingsByOrder[r.order_id]?.rating ?? null,
      feedback: ratingsByOrder[r.order_id]?.feedback ?? null,
    })));
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
