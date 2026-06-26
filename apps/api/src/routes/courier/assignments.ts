import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { acceptCourierAssignment } from '../../lib/courierAssignmentService';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { updateOrderStatus } from '../../lib/orderStatusService';
import { getImageUrl } from '../../lib/image-url.js';
import { distanceKm } from '../../lib/geo.js';
import { ETA_DEFAULTS, deliveryLegMinutes } from '../../lib/etaService.js';

const env = loadEnv();

export default (async function courierAssignmentsRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts as { db: any, messageBus: MessageBus };

  fastify.addHook('preValidation', fastify.verifyAuth);
  fastify.addHook('preValidation', fastify.requireRole(['courier']));

  function toTaskShape(row: any) {
    const cashAmt = row.cash_amount ? parseInt(row.cash_amount) : null;
    return {
      id: row.id,
      orderId: row.order_id,
      status: row.status,
      assignedAt: row.assigned_at ?? null,
      acceptedAt: row.accepted_at ?? null,
      pickedUpAt: row.picked_up_at ?? null,
      deliveredAt: row.delivered_at ?? null,
      cashCollected: row.cash_collected ?? false,
      cashAmount: cashAmt,
      total: parseInt(row.total) || 0,
      tipAmount: parseInt(row.tip_amount) || 0, // UX-4: informative; courier collects in cash
      eta: '~15 min',
      restaurant: {
        name: row.restaurant_name || '',
        address: row.restaurant_address || '',
        lat: row.restaurant_lat ? parseFloat(row.restaurant_lat) : null,
        lng: row.restaurant_lng ? parseFloat(row.restaurant_lng) : null,
      },
      customer: {
        address: row.delivery_address || '',
        phone: row.customer_phone || null,
        instructions: row.delivery_instructions || null,
        lat: row.delivery_lat ? parseFloat(row.delivery_lat) : null,
        lng: row.delivery_lng ? parseFloat(row.delivery_lng) : null,
        // UX-2: customer messenger, only while the task is active (parity with phone).
        messengerKind: ['assigned', 'accepted', 'picked_up'].includes(row.status) ? (row.customer_messenger_kind || null) : null,
        messengerHandle: ['assigned', 'accepted', 'picked_up'].includes(row.status) ? (row.customer_messenger_handle || null) : null,
        // UX-3: entry-anchor photo URL, only while the task is active.
        entryPhotoUrl: ['assigned', 'accepted', 'picked_up'].includes(row.status) ? getImageUrl(row.delivery_photo_key) : null,
      },
      cashPayWith: cashAmt,
    };
  }

  const ENRICHED_ASSIGNMENTS_QUERY = `
    SELECT ca.id, ca.order_id, ca.status, ca.assigned_at, ca.accepted_at,
           ca.picked_up_at, ca.delivered_at, ca.cash_collected, ca.cash_amount,
           o.total, o.delivery_address, o.delivery_lat, o.delivery_lng, o.delivery_instructions,
           l.name as restaurant_name, l.address as restaurant_address,
           l.lat as restaurant_lat, l.lng as restaurant_lng,
           c.phone as customer_phone,
           c.messenger_kind as customer_messenger_kind, c.messenger_handle as customer_messenger_handle,
           o.delivery_photo_key, o.tip_amount
    FROM courier_assignments ca
    JOIN orders o ON o.id = ca.order_id
    JOIN locations l ON l.id = o.location_id
    LEFT JOIN customers c ON c.id = o.customer_id
  `;

  // 1. Get assignments
  fastify.get('/me/assignments', async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(
        ENRICHED_ASSIGNMENTS_QUERY +
        `WHERE ca.courier_id = $1 AND ca.location_id = $2
           AND ca.status IN ('assigned', 'accepted', 'picked_up')
         ORDER BY ca.created_at DESC`,
        [courierId, locationId]
      );

      await client.query('COMMIT');
      return reply.send({ success: true, assignments: res.rows.map(toTaskShape) });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 1b. Get single assignment (used by DeliveryPage)
  fastify.get('/assignments/:id', {
    schema: { params: z.object({ id: z.string().uuid() }) }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;

    const client = await db.connect();
    try {
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      const res = await client.query(
        ENRICHED_ASSIGNMENTS_QUERY +
        `WHERE ca.id = $1 AND ca.courier_id = $2`,
        [id, courierId]
      );
      if (res.rowCount === 0) return reply.sendError(404, 'NOT_FOUND', 'Assignment not found');
      return reply.send(toTaskShape(res.rows[0]));
    } finally {
      client.release();
    }
  });

  // 2. Accept Assignment
  fastify.post('/assignments/:id/accept', {
    schema: {
      params: z.object({ id: z.string().uuid() })
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    
    const acceptWindowMs = parseInt((env as any).COURIER_ACCEPT_WINDOW_MS || '30000', 10);

     const client = await db.connect();
     try {
       await client.query('BEGIN');
       await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

       // Use the service to accept the assignment (scoped to the calling courier —
       // cross-courier IDOR fix; the service rejects another courier's assignment 404)
       const { orderId: orderIdForStatus } = await acceptCourierAssignment(client, id, locationId, courierId, { messageBus });

       // Advance order to CONFIRMED (idempotent — ignore if already at or past this state)
       try {
         await updateOrderStatus(client, orderIdForStatus, locationId, 'CONFIRMED', { messageBus });
       } catch { /* order may already be confirmed or in a further state */ }

       await client.query('COMMIT');
       return reply.send({ success: true });
     } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 3. Reject Assignment
  fastify.post('/assignments/:id/reject', {
    schema: {
      params: z.object({ id: z.string().uuid() })
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT order_id, shift_id FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status = 'assigned' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_NOT_ASSIGNED', 'ASSIGNMENT_NOT_FOUND_OR_NOT_ASSIGNED');
      }

      const { order_id, shift_id } = res.rows[0];

      await client.query(`
        UPDATE courier_assignments SET status = 'rejected', cancelled_at = now(), cancellation_reason = 'courier_rejected' WHERE id = $1
      `, [id]);

      await client.query(`
        UPDATE courier_shifts SET status = 'available' WHERE id = $1
      `, [shift_id]);

      // Re-enqueue for another courier
      await client.query(`
        INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at)
        VALUES ($1, $2, now())
        ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1
      `, [order_id, locationId]);

      await client.query(`
        INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id)
        VALUES ($1, $2, 'assignment.rejected', 'courier', $1)
      `, [courierId, locationId]);

      await client.query('COMMIT');

      // Kick off dispatch worker again
      await messageBus.publish(BUS_CHANNELS.ORDER_CONFIRMED, { orderId: order_id, locationId });

      return reply.send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 4. Picked up
  fastify.post('/assignments/:id/picked-up', {
    schema: {
      params: z.object({ id: z.string().uuid() })
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT order_id FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status = 'accepted' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_NOT_ACCEPTED', 'ASSIGNMENT_NOT_FOUND_OR_NOT_ACCEPTED');
      }

      const { order_id } = res.rows[0];

      await client.query(`
        UPDATE courier_assignments SET status = 'picked_up', picked_up_at = now() WHERE id = $1
      `, [id]);

      // Advance order to IN_DELIVERY (idempotent — ignore if already at or past this state)
      try {
        await updateOrderStatus(client, order_id, locationId, 'IN_DELIVERY', { messageBus });
      } catch { /* order may already be in delivery or in a further state */ }

      await client.query('COMMIT');

      await messageBus.publish(BUS_CHANNELS.ORDER_PICKED_UP, {
        orderId: order_id,
        locationId,
        courierId
      });

      return reply.send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 5. Delivered
  fastify.post('/assignments/:id/delivered', {
    schema: {
      params: z.object({ id: z.string().uuid() }),
      body: z.object({
        cash_collected: z.boolean(),
        cash_amount: z.number().optional()
      }).strict()
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    const { cash_collected, cash_amount } = request.body;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT ca.order_id, ca.shift_id, o.total,
               o.delivery_lat, o.delivery_lng, l.lat AS loc_lat, l.lng AS loc_lng
        FROM courier_assignments ca
        JOIN orders o ON ca.order_id = o.id
        JOIN locations l ON l.id = o.location_id
        WHERE ca.id = $1 AND ca.courier_id = $2 AND ca.status = 'picked_up' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP', 'ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP');
      }

      const { order_id, shift_id, total } = res.rows[0];

      // SENSOR-BUS §1.2: normalised delivery baseline — observed venue→customer road distance +
      // expected leg minutes, NO router (brief §1.2). Pure haversine × road-factor on coords in-hand;
      // null for pickup / missing pins. Written into the immutable delivery_trace audit below.
      const dRow = res.rows[0];
      const baselineKm = (dRow.delivery_lat != null && dRow.delivery_lng != null && dRow.loc_lat != null && dRow.loc_lng != null)
        ? distanceKm(Number(dRow.loc_lat), Number(dRow.loc_lng), Number(dRow.delivery_lat), Number(dRow.delivery_lng)) * ETA_DEFAULTS.roadFactor
        : null;
      const routeDistanceM = baselineKm != null && Number.isFinite(baselineKm) ? Math.round(baselineKm * 1000) : null;
      const legMin = deliveryLegMinutes(
        dRow.loc_lat != null ? Number(dRow.loc_lat) : null,
        dRow.loc_lng != null ? Number(dRow.loc_lng) : null,
        dRow.delivery_lat != null ? Number(dRow.delivery_lat) : null,
        dRow.delivery_lng != null ? Number(dRow.delivery_lng) : null,
      );
      const expectedDeliveryMin = legMin != null && Number.isFinite(legMin) ? Math.max(1, Math.round(legMin)) : null;

      if (cash_collected && cash_amount !== total) {
        await client.query('ROLLBACK');
        return reply.status(422).send({ error: 'CASH_AMOUNT_MISMATCH', expected: total });
      }

      await client.query(`
        UPDATE courier_assignments 
        SET status = 'delivered', delivered_at = now(), cash_collected = $1, cash_amount = $2 
        WHERE id = $3
      `, [cash_collected, cash_collected ? cash_amount : null, id]);

      await client.query(`
        UPDATE courier_shifts SET status = 'available' WHERE id = $1
      `, [shift_id]);

      // Canonical path: update orders status + publish WS events (customer + owner)
      await updateOrderStatus(client, order_id, locationId, 'DELIVERED', { messageBus });

      // Immutable delivery audit (one per order; idempotent). §1.2: carries the normalised
      // baseline (route_distance_m + expected_delivery_min); a re-fired DELIVERED is a no-op
      // (DO NOTHING) so the first observed baseline is the immutable record.
      await client.query(`
        INSERT INTO delivery_trace (order_id, location_id, courier_id, total, delivered_at, route_distance_m, expected_delivery_min)
        VALUES ($1, $2, $3, $4, now(), $5, $6)
        ON CONFLICT (order_id) DO NOTHING
      `, [order_id, locationId, courierId, total, routeDistanceM, expectedDeliveryMin]);

      // Cash-collected → append a 'hold' audit row (NOT the settlement source of
      // truth; settlement_items remains authoritative). Idempotent per (order_id,type).
      if (cash_collected) {
        await client.query(`
          INSERT INTO courier_cash_ledger (courier_id, location_id, order_id, type, amount)
          VALUES ($1, $2, $3, 'hold', $4)
          ON CONFLICT (order_id, type) DO NOTHING
        `, [courierId, locationId, order_id, cash_amount]);
      }

      await client.query('COMMIT');

      // Integrate with Phase 1 lifecycle
      await messageBus.publish(BUS_CHANNELS.ORDER_DELIVERED, {
        orderId: order_id,
        locationId,
        courierId,
        cashCollected: cash_collected,
        cashAmount: cash_collected ? cash_amount : null
      });

      // NOTE: a post-delivery feedback reminder was intended here, but registering a
      // new pg-boss queue is infeasible on this infra — pgboss.queue is owned by the
      // operational role (which lacks CREATE on the pgboss schema, revoked by
      // migration 009) while the migration role lacks REFERENCES on that table, so
      // neither can create_queue. Enqueue to an unregistered queue silently no-ops
      // (order.timeout has the same fate). Tracked as known-debt in the reliability
      // gate; revisit via an existing registered queue or a cron sweep.

      return reply.send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // 6. Cancel
  fastify.post('/assignments/:id/cancel', {
    schema: {
      params: z.object({ id: z.string().uuid() }),
      body: z.object({
        reason: z.string()
      }).strict()
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    const { reason } = request.body;
    
    const cancelWindowMs = parseInt((env as any).CANCEL_AFTER_DISPATCH_WINDOW_MS || '300000', 10);

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT order_id, shift_id, assigned_at FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status IN ('accepted', 'picked_up') FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS', 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS');
      }

      const { order_id, shift_id, assigned_at } = res.rows[0];
      const elapsedMs = Date.now() - new Date(assigned_at).getTime();

      if (elapsedMs > cancelWindowMs) {
        await client.query('ROLLBACK');
        return reply.sendError(410, 'CANCEL_WINDOW_EXPIRED', 'CANCEL_WINDOW_EXPIRED');
      }

      await client.query(`
        UPDATE courier_assignments 
        SET status = 'cancelled', cancelled_at = now(), cancellation_reason = $1 
        WHERE id = $2
      `, [reason, id]);

      await client.query(`
        UPDATE courier_shifts SET status = 'available' WHERE id = $1
      `, [shift_id]);

      await client.query('COMMIT');

      await messageBus.publish(BUS_CHANNELS.ORDER_CANCELLED, { 
        orderId: order_id, 
        locationId,
        reason: `courier_cancelled: ${reason}` 
      });

      return reply.send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

});
