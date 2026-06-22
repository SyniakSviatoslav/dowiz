import type { FastifyInstance } from 'fastify';
import { signAuthToken } from '@deliveryos/platform';
import crypto from 'node:crypto';
import { dashboardChannel } from '../../lib/registry.js';

export default async function mockAuthRoutes(fastify: FastifyInstance) {
  console.log('[API] Registering mockAuthRoutes: /dev/mock-auth');
  fastify.post('/dev/mock-auth', async (request: any, reply: any) => {
    const body = request.body as Record<string, unknown> || {};
    const role = body.role === 'courier' ? 'courier' : 'owner';

    if (role === 'courier') {
      const locationId = (body.locationId as string) || '1f609add-062a-4bb5-89bf-d695f963ede6';
      const courierId = crypto.randomUUID();

      const accessToken = await signAuthToken({
        role: 'courier',
        sub: courierId,
        activeLocationId: locationId,
      } as any, '1d');

      return reply.send({ access_token: accessToken, userId: courierId, activeLocationId: locationId });
    }

    // Owner role (default)
    const email = 'dev@deliveryos.com';
    const googleSub = 'mock-google-12345';
    const ownerName = 'Dev Owner';

    let userId: string;
    try {
      const res = await (fastify as any).db.query(
        `INSERT INTO users (email, google_sub, display_name) 
         VALUES ($1, $2, $3)
         ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
         RETURNING id`,
        [email, googleSub, ownerName]
      );
      userId = res.rows[0].id;
    } catch (e) {
      const updateRes = await (fastify as any).db.query(
        `UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3) WHERE email = $1 RETURNING id`,
        [email, googleSub, ownerName]
      );
      if (updateRes.rowCount === 0) {
        throw new Error('Failed to upsert dev user');
      }
      userId = updateRes.rows[0].id;
    }

    let activeLocationId: string | undefined;

    if (body.locationSlug) {
      const locRes = await (fastify as any).db.query(
        `SELECT id FROM locations WHERE slug = $1 AND status = 'active' LIMIT 1`,
        [body.locationSlug]
      );
      if (locRes.rowCount > 0) activeLocationId = locRes.rows[0].id;
    }

    if (!activeLocationId) {
      const memberRes = await (fastify as any).db.query(
        `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
        [userId]
      );
      if (memberRes.rowCount > 0) activeLocationId = memberRes.rows[0].location_id;
    }

    const tokenPayload: Record<string, unknown> = { role: 'owner', userId };
    if (activeLocationId) tokenPayload.activeLocationId = activeLocationId;

    const accessToken = await signAuthToken(tokenPayload as any, '1d');
    return reply.send({ access_token: accessToken, userId, activeLocationId });
  });

  // Test helper: create courier assignment for an order
  fastify.post('/dev/create-assignment', async (request: any, reply: any) => {
    const { orderId, courierId, locationId } = request.body as Record<string, string>;
    if (!orderId || !courierId || !locationId) {
      return reply.status(400).send({ error: 'orderId, courierId, locationId required' });
    }

    const ownerRes = await (fastify as any).db.query(
      `SELECT id FROM users WHERE email = 'dev@deliveryos.com' LIMIT 1`
    );
    const ownerId = ownerRes.rowCount > 0 ? ownerRes.rows[0].id : '00000000-0000-0000-0000-000000000000';

    const client = await (fastify as any).db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      await client.query(`SELECT set_config('app.user_id', $1, true)`, [ownerId]);

      const shiftRes = await client.query(
        `INSERT INTO courier_shifts (courier_id, location_id, status, started_at)
         VALUES ($1, $2, 'available', now())
         ON CONFLICT DO NOTHING
         RETURNING id`,
        [courierId, locationId]
      );
      const shiftId = shiftRes.rows[0]?.id || null;

      const asgnRes = await client.query(
        `INSERT INTO courier_assignments (order_id, courier_id, location_id, shift_id, status)
         VALUES ($1, $2, $3, $4, 'assigned')
         ON CONFLICT (order_id) DO UPDATE SET courier_id = EXCLUDED.courier_id, status = 'assigned'
         RETURNING id`,
        [orderId, courierId, locationId, shiftId]
      );

      await client.query('COMMIT');

      // Publish WS events for test verification
      const messageBus = (fastify as any).messageBus;
      if (messageBus) {
        await messageBus.publish(dashboardChannel(locationId), {
          type: 'assignment.created',
          orderId,
          courierId,
        });
        await messageBus.publish(`courier:${courierId}`, {
          type: 'task_assigned',
          payload: { id: orderId, orderId, status: 'assigned', courierId },
        });
      }

      return reply.send({ assignmentId: asgnRes.rows[0].id, shiftId });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // Test helper: seed an active telegram notification target for a location so the
  // category preference-centre (and /settings) has something to render against.
  fastify.post('/dev/seed-telegram-target', async (request: any, reply: any) => {
    const body = (request.body as Record<string, unknown>) || {};
    const locationId = body.locationId as string;
    const userId = (body.userId as string) || null;
    const address = (body.address as string) || `test-chat-${crypto.randomUUID().slice(0, 8)}`;
    if (!locationId) return reply.status(400).send({ error: 'locationId required' });

    const res = await (fastify as any).db.query(
      `INSERT INTO owner_notification_targets (location_id, channel, address, status, user_id)
       VALUES ($1, 'telegram', $2, 'active', $3)
       ON CONFLICT (location_id, channel, address) DO UPDATE SET status = 'active'
       RETURNING id`,
      [locationId, address, userId]
    );
    return reply.send({ targetId: res.rows[0].id, address });
  });

  // Test helper: report + repair the local-login fixture so test@dowiz.com owns the demo
  // location (the seed's membership link can be missing on a DB that was provisioned before
  // the demo location existed). Idempotent.
  fastify.post('/dev/repair-test-owner', async (request: any, reply: any) => {
    const body = (request.body as Record<string, unknown>) || {};
    const email = (body.email as string) || 'test@dowiz.com';
    const slug = (body.slug as string) || 'demo';
    const db = (fastify as any).db;

    const u = await db.query(`SELECT id FROM users WHERE email = $1`, [email]);
    if (u.rowCount === 0) return reply.status(404).send({ error: `user ${email} not found` });
    const userId = u.rows[0].id;
    // Resolve by explicit locationId if given, else by slug (NO status filter — the demo
    // location's status is not 'active' yet it is the live storefront).
    let locationId = (body.locationId as string) || null;
    let locName: string | null = null;
    if (locationId) {
      const l = await db.query(`SELECT name FROM locations WHERE id = $1`, [locationId]);
      locName = l.rowCount > 0 ? l.rows[0].name : null;
    } else {
      const loc = await db.query(`SELECT id, name FROM locations WHERE slug = $1 LIMIT 1`, [slug]);
      if (loc.rowCount === 0) return reply.status(404).send({ error: `location '${slug}' not found` });
      locationId = loc.rows[0].id; locName = loc.rows[0].name;
    }

    const before = await db.query(
      `SELECT location_id, role, status FROM memberships WHERE user_id = $1`, [userId]);
    const ownedOrgs = await db.query(`SELECT id FROM organizations WHERE owner_id = $1`, [userId]);

    await db.query(
      `INSERT INTO memberships (user_id, location_id, role, status)
       VALUES ($1, $2, 'owner', 'active')
       ON CONFLICT (user_id, location_id, role) DO UPDATE SET status = 'active'`,
      [userId, locationId]);

    const after = await db.query(
      `SELECT location_id, role, status FROM memberships WHERE user_id = $1`, [userId]);
    return reply.send({
      email, userId, slug, locationId, locationName: locName,
      ownedOrgs: ownedOrgs.rowCount, membershipsBefore: before.rows, membershipsAfter: after.rows,
    });
  });
}
