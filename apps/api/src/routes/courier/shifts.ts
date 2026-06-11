// @ts-nocheck
import { z } from 'zod';
import { roundCoordinate, isWithinGeofence } from '../../lib/geo.js';
import { loadEnv } from '@deliveryos/config';
import { MessageBus } from '@deliveryos/platform';

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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

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
       await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

       // Use the service to open the shift
       const { shiftId, status, startedAt } = await openShift(client, courierId, locationId, { messageBus });

       if (lat !== undefined && lng !== undefined) {
        const rLat = roundCoordinate(lat);
        const rLng = roundCoordinate(lng);
        await client.query(`
          INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, source)
          VALUES ($1, $2, $3, $4, $5, 'gps')
        `, [courierId, locationId, shiftId, rLat, rLng]);
      }

      await client.query(`
        INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
        VALUES ($1, $2, 'shift.started', 'courier', $1)
      `, [courierId, locationId]);

      await client.query('COMMIT');

      await messageBus.publish(`location:${locationId}:couriers`, {
        type: 'courier.shift_updated',
        payload: { courierId, status: 'available' }
      });

      await messageBus.publish('shift.started', { shiftId, locationId, courierId, startedAt });

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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

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

      await messageBus.publish(`location:${locationId}:couriers`, {
        type: 'courier.shift_updated',
        payload: { courierId, status: 'offline' }
      });

      await messageBus.publish('shift.closed', { shiftId, locationId, courierId, endedAt: new Date().toISOString() });

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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

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
        await messageBus.publish(`location:${locationId}:couriers`, {
          type: 'courier.shift_updated',
          payload: { courierId, status: 'offline' }
        });

        await messageBus.publish('shift.closed', { shiftId, locationId, courierId, endedAt: new Date().toISOString() });

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

        // Record initial position
        const rLat = roundCoordinate(lat);
        const rLng = roundCoordinate(lng);

        await client.query(`
          INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, source)
          VALUES ($1, $2, $3, $4, $5, 'gps')
        `, [courierId, locationId, newShiftId, rLat, rLng]);

        await client.query(`
          INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
          VALUES ($1, $2, 'shift.transition_available', 'courier', $1)
        `, [courierId, locationId]);

        await client.query('COMMIT');

        // Broadcast to owner WS
        await messageBus.publish(`location:${locationId}:couriers`, {
          type: 'courier.shift_updated',
          payload: { courierId, status: 'available', position: { lat: rLat, lng: rLng } }
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
        timeWindow: 10000 // 1 ping per 10s
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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

      // Range check vs location pin
      const locRes = await client.query(`SELECT lat, lng FROM locations WHERE id = $1`, [locationId]);
      if (locRes.rowCount > 0 && locRes.rows[0].lat && locRes.rows[0].lng) {
        const center = { lat: locRes.rows[0].lat, lng: locRes.rows[0].lng };
        const maxDist = parseFloat(env.COURIER_GPS_MAX_DIST_KM || '50');
        if (!isWithinGeofence({ lat, lng }, center, maxDist)) {
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
      const rLat = roundCoordinate(lat);
      const rLng = roundCoordinate(lng);

      // Insert position
      await client.query(`
        INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, accuracy_meters, source)
        VALUES ($1, $2, $3, $4, $5, $6, 'gps')
      `, [courierId, locationId, shiftId, rLat, rLng, accuracy_meters || null]);

      // Update heartbeat
      await client.query(`
        UPDATE courier_shifts SET last_heartbeat_at = now() WHERE id = $1
      `, [shiftId]);

      await client.query('COMMIT');

      // Publish event (claim-check style)
      await messageBus.publish('courier.position_updated', {
        courierId,
        locationId,
        shiftId
      });

      return reply.send({ success: true });
    } catch (err) {
      try { await client.query('ROLLBACK'); } catch (_) {}
      throw err;
    } finally {
      client.release();
    }
  });
});