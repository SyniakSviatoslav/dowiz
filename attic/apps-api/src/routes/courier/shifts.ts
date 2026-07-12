import { z } from 'zod';
import { roundCoordinate, isWithinGeofence } from '../../lib/geo.js';
import { loadEnv } from '@deliveryos/config';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { openShift } from '../../lib/shiftService.js';
import { ACTIVE_DELIVERY_ASSIGNMENT_STATUSES } from '../../lib/courier-gps.js';

const env = loadEnv();

export default (async function courierShiftsRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts as { db: any, messageBus: MessageBus };

  // 0. Get current shift status
  fastify.get('/me/shift', {
    preValidation: [fastify.verifyAuth, fastify.requireRole(['courier'])]
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT id, status, started_at, ended_at, last_heartbeat_at
        FROM courier_shifts
        WHERE courier_id = $1 AND location_id = $2 AND status IN ('available', 'on_delivery')
        ORDER BY started_at DESC LIMIT 1
      `, [courierId, locationId]);

      await client.query('COMMIT');

      if (res.rowCount === 0) {
        return reply.send({ isActive: false, startedAt: null, elapsedSeconds: 0, stats: null });
      }

      const row = res.rows[0];
      const startedAt = row.started_at ? new Date(row.started_at).toISOString() : null;
      const elapsedSeconds = row.started_at ? Math.floor((Date.now() - new Date(row.started_at).getTime()) / 1000) : 0;

      return reply.send({
        isActive: row.status === 'available' || row.status === 'on_delivery',
        startedAt,
        elapsedSeconds,
        shiftId: row.id,
        status: row.status,
        stats: null
      });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 0b. Start shift (convenience route for frontend)
  fastify.post('/me/shift/start', {
    preValidation: [fastify.verifyAuth, fastify.requireRole(['courier'])]
  }, async (request: any, reply: any) => {
    const bodySchema = z.object({
      lat: z.number().optional(),
      lng: z.number().optional()
    });
    const result = bodySchema.safeParse(request.body || {});
    const lat = result.success ? result.data.lat : undefined;
    const lng = result.success ? result.data.lng : undefined;

    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
     try {
       await client.query('BEGIN');
       await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

       // Use the service to open the shift
       const { shiftId, status, startedAt } = await openShift(client, courierId, locationId, { messageBus });

       // P0-1: no position write at shift-open. A courier going on shift is idle, not
       // on a delivery — storing their location here is exactly the off-delivery
       // tracking we are removing (HD-1 privacy-max). GPS rows are written only by the
       // ping handler while on an active delivery.

      await client.query(`
        INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
        VALUES ($1, $2, 'shift.started', 'courier', $1)
      `, [courierId, locationId]);

      await client.query('COMMIT');

      await messageBus.publish(courierChannel(locationId), {
        type: 'courier.shift_updated',
        payload: { courierId, status: 'available' }
      });

      await messageBus.publish(BUS_CHANNELS.SHIFT_STARTED, { shiftId, locationId, courierId, startedAt });

      return reply.send({ success: true, status, shiftId, startedAt });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 0c. End shift (convenience route for frontend)
  fastify.post('/me/shift/end', {
    preValidation: [fastify.verifyAuth, fastify.requireRole(['courier'])]
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const shiftRes = await client.query(`
        SELECT id, status FROM courier_shifts
        WHERE courier_id = $1 AND location_id = $2 AND status IN ('available', 'on_delivery')
        FOR UPDATE
      `, [courierId, locationId]);

      if (shiftRes.rowCount === 0) {
        await client.query('COMMIT');
        return reply.send({ success: true, status: 'offline' });
      }

      const shiftId = shiftRes.rows[0].id;

      const activeRes = await client.query(`
        SELECT 1 FROM courier_assignments
        WHERE courier_id = $1 AND status IN ('assigned', 'accepted', 'picked_up')
      `, [courierId]);

      if (activeRes.rowCount > 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'ACTIVE_DELIVERY_EXISTS' });
      }

      await client.query(`
        UPDATE courier_shifts SET status = 'offline', ended_at = now() WHERE id = $1
      `, [shiftId]);

      await client.query(`
        INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
        VALUES ($1, $2, 'shift.ended', 'courier', $1)
      `, [courierId, locationId]);

      await client.query('COMMIT');

      await messageBus.publish(courierChannel(locationId), {
        type: 'courier.shift_updated',
        payload: { courierId, status: 'offline' }
      });

      await messageBus.publish(BUS_CHANNELS.SHIFT_CLOSED, { shiftId, locationId, courierId, endedAt: new Date().toISOString() });

      return reply.send({ success: true, status: 'offline' });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 1. Transition Shift Status
  fastify.post('/shifts/transition', {
    preValidation: [fastify.verifyAuth, fastify.requireRole(['courier'])]
  }, async (request: any, reply: any) => {
    const bodySchema = z.object({
      to: z.enum(['offline', 'available']),
      lat: z.number().optional(),
      lng: z.number().optional()
    }).strict();
    const bodyResult = bodySchema.safeParse((request as any).body);
    if (!bodyResult.success) {
      return reply.status(400).send({ error: 'Validation failed', details: bodyResult.error.format() });
    }
    const { to, lat, lng } = bodyResult.data;

    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      // Check existing shift
      const shiftRes = await client.query(`
        SELECT id, status FROM courier_shifts 
        WHERE courier_id = $1 AND location_id = $2 
        FOR UPDATE
      `, [courierId, locationId]);

      const currentStatus = shiftRes.rowCount > 0 ? shiftRes.rows[0].status : 'offline';
      const shiftId = shiftRes.rowCount > 0 ? shiftRes.rows[0].id : undefined;

      // Idempotency
      if (currentStatus === to) {
        await client.query('ROLLBACK');
        return reply.send({ success: true, status: to, shiftId });
      }

      if (to === 'offline') {
        if (currentStatus === 'on_delivery') {
          await client.query('ROLLBACK');
          return reply.status(409).send({ error: 'CANNOT_GO_OFFLINE_WITH_ACTIVE_ORDER' });
        }

        // Validate any active assignments (belt and braces)
        const activeRes = await client.query(`
          SELECT 1 FROM courier_assignments 
          WHERE courier_id = $1 AND status IN ('assigned', 'accepted', 'picked_up')
        `, [courierId]);

        if (activeRes.rowCount > 0) {
          await client.query('ROLLBACK');
          return reply.status(409).send({ error: 'ACTIVE_DELIVERY_EXISTS' });
        }

        // Transition to offline
        await client.query(`
          UPDATE courier_shifts SET status = 'offline', ended_at = now() WHERE id = $1
        `, [shiftId]);

        await client.query(`
          INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
          VALUES ($1, $2, 'shift.transition_offline', 'courier', $1)
        `, [courierId, locationId]);

        await client.query('COMMIT');

        // Broadcast to owner WS
        await messageBus.publish(courierChannel(locationId), {
          type: 'courier.shift_updated',
          payload: { courierId, status: 'offline' }
        });

        await messageBus.publish(BUS_CHANNELS.SHIFT_CLOSED, { shiftId, locationId, courierId, endedAt: new Date().toISOString() });

        return reply.send({ success: true, status: 'offline', shiftId });

      } else if (to === 'available') {
        if (currentStatus === 'on_delivery') {
          await client.query('ROLLBACK');
          return reply.status(409).send({ error: 'INVALID_TRANSITION' }); // Must be via delivered
        }

        if (lat === undefined || lng === undefined) {
          await client.query('ROLLBACK');
          return reply.status(400).send({ error: 'GPS_REQUIRED' });
        }

        let newShiftId = shiftId;
        if (!shiftId) {
          const insertRes = await client.query(`
            INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
            VALUES ($1, $2, 'available', now(), now())
            RETURNING id
          `, [courierId, locationId]);
          newShiftId = insertRes.rows[0].id;
        } else {
          await client.query(`
            UPDATE courier_shifts 
            SET status = 'available', ended_at = NULL, started_at = coalesce(started_at, now()), last_heartbeat_at = now() 
            WHERE id = $1
          `, [shiftId]);
        }

        // P0-1: no position write on transition-to-available — the courier is idle,
        // not on a delivery (HD-1 privacy-max). The owner map shows couriers only
        // while on an active delivery; an idle courier produces no position row.
        await client.query(`
          INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
          VALUES ($1, $2, 'shift.transition_available', 'courier', $1)
        `, [courierId, locationId]);

        await client.query('COMMIT');

        // Broadcast to owner WS (status only — no idle position, P0-1).
        await messageBus.publish(courierChannel(locationId), {
          type: 'courier.shift_updated',
          payload: { courierId, status: 'available' }
        });

        return reply.send({ success: true, status: 'available', shiftId: newShiftId });
      }

    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 2. Ping (GPS Update)
  fastify.post('/shifts/ping', {
    preValidation: [fastify.verifyAuth, fastify.requireRole(['courier'])],
    config: {
      rateLimit: {
        max: 1,
        timeWindow: 10000, // 1 ping per 10s, PER COURIER
        // Key by the courier's bearer token, not the default client IP. Multiple
        // couriers behind one NAT/carrier-grade-NAT share an IP, and IP-keyed
        // limiting made them throttle each other's GPS pings (only one courier's
        // position would update). The token is per-courier and available at
        // onRequest time (before auth runs), so each courier gets their own bucket.
        keyGenerator: (req: any) => req.headers?.authorization || req.ip,
      }
    }
  }, async (request: any, reply: any) => {
    const bodySchema = z.object({
      lat: z.number(),
      lng: z.number(),
      accuracy_meters: z.number().optional()
    }).strict();
    const bodyResult = bodySchema.safeParse((request as any).body);
    if (!bodyResult.success) {
      return reply.status(400).send({ error: 'Validation failed', details: bodyResult.error.format() });
    }
    const { lat, lng, accuracy_meters } = bodyResult.data;

    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    
    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      // Range check vs location pin
      const locRes = await client.query(`SELECT lat, lng FROM locations WHERE id = $1`, [locationId]);
      if (locRes.rowCount > 0 && locRes.rows[0].lat && locRes.rows[0].lng) {
        const center = { lat: locRes.rows[0].lat, lng: locRes.rows[0].lng };
        const maxDist = parseFloat((env as any).COURIER_GPS_MAX_DIST_KM || '50');
        if (!isWithinGeofence(lat, lng, center.lat, center.lng, maxDist)) {
          await client.query('ROLLBACK');
          return reply.status(400).send({ error: 'GPS_OUT_OF_RANGE' });
        }
      }

      const shiftRes = await client.query(`
        SELECT id FROM courier_shifts 
        WHERE courier_id = $1 AND location_id = $2 AND status IN ('available', 'on_delivery')
      `, [courierId, locationId]);

      if (shiftRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'NO_ACTIVE_SHIFT' });
      }

      const shiftId = shiftRes.rows[0].id;

      // P0-1 HARD GATE: store a GPS position ONLY while on an active delivery
      // (assignment accepted/picked_up — the courier's consent boundary). Tenant
      // is already scoped via set_config above, so this read is RLS-safe.
      const activeRes = await client.query(
        `SELECT 1 FROM courier_assignments
          WHERE courier_id = $1 AND status = ANY($2::text[]) LIMIT 1`,
        [courierId, ACTIVE_DELIVERY_ASSIGNMENT_STATUSES as unknown as string[]]
      );
      const onActiveDelivery = activeRes.rowCount > 0;

      if (onActiveDelivery) {
        const rLat = roundCoordinate(lat);
        const rLng = roundCoordinate(lng);
        await client.query(`
          INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, accuracy_meters, source)
          VALUES ($1, $2, $3, $4, $5, $6, 'gps')
        `, [courierId, locationId, shiftId, rLat, rLng, accuracy_meters || null]);
      }

      // Heartbeat ALWAYS updates while on shift — keeps the shift live (liveness) even
      // when GPS is withheld, so an idle on-shift courier is not marked stale.
      await client.query(`
        UPDATE courier_shifts SET last_heartbeat_at = now() WHERE id = $1
      `, [shiftId]);

      await client.query('COMMIT');

      // Publish (claim-check style) ONLY when a position was actually stored — an idle
      // courier emits no position event, so the owner map shows only active deliveries.
      if (onActiveDelivery) {
        await messageBus.publish(BUS_CHANNELS.COURIER_POSITION_UPDATED, {
          courierId,
          locationId,
          shiftId
        });
      }

      // gps_stored tells the courier app the server withheld the position (off-delivery),
      // so it can stop *collecting* GPS — but the heartbeat keeps flowing on its timer.
      return reply.send({ success: true, gps_stored: onActiveDelivery, reason: onActiveDelivery ? undefined : 'NOT_ON_ACTIVE_DELIVERY' });
    } catch (err) {
      try { await client.query('ROLLBACK'); } catch (_) {}
      throw err;
    } finally {
      client.release();
    }
  });
});