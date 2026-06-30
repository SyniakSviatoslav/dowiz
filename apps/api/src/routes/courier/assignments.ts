import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { acceptCourierAssignment } from '../../lib/courierAssignmentService';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { updateOrderStatus } from '../../lib/orderStatusService';
import { completeDelivery, CompletionError } from '../../lib/deliveryCompletion.js';
import { getImageUrl } from '../../lib/image-url.js';
import { distanceKm } from '../../lib/geo.js';
import { ETA_DEFAULTS, deliveryLegMinutes } from '../../lib/etaService.js';
import { releaseBindingAndReoffer } from '../../lib/bindingRelease.js';

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

       // deliver v2 §A: an 'offered' assignment is the handshake path — accept it (status-guarded, courier-
       // scoped) and advance the order to IN_DELIVERY (the courier takes the run). Used under
       // COURIER_OFFER_HANDSHAKE_ENABLED. The order WAS NOT advanced at offer time, so this is its first move.
       const offered = await client.query(
         `SELECT order_id, shift_id FROM courier_assignments WHERE id=$1 AND courier_id=$2 AND status='offered' FOR UPDATE`,
         [id, courierId],
       );
       if (offered.rowCount > 0) {
         const { order_id, shift_id } = offered.rows[0];
         await client.query(`UPDATE courier_assignments SET status='accepted', offered_expires_at=NULL WHERE id=$1`, [id]);
         if (shift_id) await client.query(`UPDATE courier_shifts SET status='on_delivery' WHERE id=$1`, [shift_id]);
         await updateOrderStatus(client, order_id, locationId, 'IN_DELIVERY', { messageBus });
         await client.query(`UPDATE orders SET courier_id=$1 WHERE id=$2`, [courierId, order_id]);
         await client.query('COMMIT');
         return reply.send({ success: true });
       }

       // Legacy (flag-off): the pre-handshake 'assigned'→'accepted' service path (scoped to the calling
       // courier — cross-courier IDOR fix; the service rejects another courier's assignment 404).
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
      // deliver v2: payment_outcome is the first-class completion signal (H-3); paid_partial/pending are
      // not in the enum → forbidden (H-2). cash_amount tightened to int/nonneg (M-2). cash_collected kept
      // for backward-compat (legacy courier app) — derives paid_full / refused_payment when no outcome sent.
      body: z.object({
        payment_outcome: z.enum(['paid_full', 'delivered_prepaid', 'refused_goods', 'refused_payment', 'customer_cancelled_on_door']).optional(),
        cash_collected: z.boolean().optional(),
        cash_amount: z.number().int().nonnegative().optional()
      }).strict()
    }
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    const { cash_collected, cash_amount } = request.body;
    // Resolve the outcome: explicit payment_outcome wins; else legacy cash_collected → paid_full/refused_payment.
    // For a PREPAID (crypto-paid) order the auto-resolve below overrides to 'delivered_prepaid' (no cash).
    let paymentOutcome: 'paid_full' | 'delivered_prepaid' | 'refused_goods' | 'refused_payment' | 'customer_cancelled_on_door' =
      request.body.payment_outcome ?? (cash_collected ? 'paid_full' : 'refused_payment');

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT ca.order_id, ca.shift_id, o.total, o.payment_status, o.payment_method,
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

      // C1 (ADR-0017): a crypto-prepaid order is settled before delivery — the courier collects NO cash, so
      // "mark delivered" auto-resolves to 'delivered_prepaid' (completeDelivery then skips the cash assert +
      // writes no till-hold; precondition payment_status='paid'). Overrides any cash-derived outcome.
      if (res.rows[0].payment_method === 'crypto' && res.rows[0].payment_status === 'paid') {
        paymentOutcome = 'delivered_prepaid';
      }

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

      // deliver v2 (R2-1): the SINGLE completion primitive — writes the assignment terminal + shift +
      // updateOrderStatus (DELIVERED for paid_full, CANCELLED for the no-cash tail) + delivery_trace crumb +
      // cash-as-proof 'hold'. The no-partial-handover rule is enforced server-side: paid_full requires
      // cash===total → CompletionError('CASH_AMOUNT_MISMATCH') → 422.
      let orderStatus: 'DELIVERED' | 'CANCELLED';
      try {
        ({ orderStatus } = await completeDelivery(client, {
          assignmentId: id, orderId: order_id, locationId, courierId, shiftId: shift_id, total,
          paymentOutcome, cashAmount: cash_amount,
          gpsLat: dRow.delivery_lat != null ? Number(dRow.delivery_lat) : null,
          gpsLng: dRow.delivery_lng != null ? Number(dRow.delivery_lng) : null,
          routeDistanceM, expectedDeliveryMin,
        }, { messageBus }));
      } catch (e) {
        if (e instanceof CompletionError) {
          await client.query('ROLLBACK');
          // PREPAID_NOT_PAID = 409 (the crypto order isn't confirmed paid yet); cash mismatch = 422.
          return reply.status(e.code === 'PREPAID_NOT_PAID' ? 409 : 422).send({ error: e.code, ...(e.meta ?? {}) });
        }
        throw e;
      }

      await client.query('COMMIT');

      // Lifecycle fan-out: ORDER_DELIVERED only for a real delivery (paid_full). The no-cash tail terminalized
      // the order to CANCELLED inside completeDelivery (which already broadcast the CANCELLED delta) — never
      // emit ORDER_DELIVERED for refused/returned food.
      if (orderStatus === 'DELIVERED') {
        await messageBus.publish(BUS_CHANNELS.ORDER_DELIVERED, {
          orderId: order_id,
          locationId,
          courierId,
          cashCollected: paymentOutcome === 'paid_full',
          cashAmount: paymentOutcome === 'paid_full' ? cash_amount : null,
        });
      }

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
        SELECT ca.order_id, ca.shift_id, ca.assigned_at, ca.status AS asg_status, o.status AS ord_status
        FROM courier_assignments ca JOIN orders o ON o.id = ca.order_id
        WHERE ca.id = $1 AND ca.courier_id = $2 AND ca.status IN ('accepted', 'picked_up') FOR UPDATE OF ca
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS', 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS');
      }

      const { order_id, shift_id, assigned_at, asg_status, ord_status } = res.rows[0];
      const elapsedMs = Date.now() - new Date(assigned_at).getTime();

      if (elapsedMs > cancelWindowMs) {
        await client.query('ROLLBACK');
        return reply.sendError(410, 'CANCEL_WINDOW_EXPIRED', 'CANCEL_WINDOW_EXPIRED');
      }

      // D1 (C-2 / R2-2 / R2-5): cancel takes the SAME exit rail as /abort — terminalize the binding, then
      // revert the order through updateOrderStatus ONLY when it is IN_DELIVERY (status-guarded; never forces an
      // illegal transition) and re-offer. The old code hand-published an unconditional ORDER_CANCELLED that
      // LIED for an order reverting to READY and left an owner-forced IN_DELIVERY order stranded (no revert);
      // the rail + the post-commit binding_changed broadcast now carry the truthful resulting state.
      const { reoffered } = await releaseBindingAndReoffer(
        client,
        { assignmentId: id, orderId: order_id, shiftId: shift_id, asgStatus: asg_status, ordStatus: ord_status, locationId, reason: `courier_cancelled: ${reason}` },
        { messageBus },
      );

      await client.query('COMMIT');

      await messageBus.publish(orderChannel(order_id), { type: 'binding_changed', orderId: order_id });
      await messageBus.publish(dashboardChannel(locationId), { type: 'assignment_aborted', orderId: order_id });

      return reply.send({ success: true, reoffered });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // deliver v2 (R2-2): en-route ABORT — the courier's "I can't complete this" exit, with NO time gate (the
  // 5-min window on /cancel is accept-regret only). No-trap red-line: abort ALWAYS frees the assignment; the
  // order-side action is CONDITIONAL on the order's LOCKED status (R3-2) so updateOrderStatus is invoked only
  // from IN_DELIVERY (the one state with a legal widened exit) and can never throw on a no-op transition.
  fastify.post('/assignments/:id/abort', {
    schema: {
      params: z.object({ id: z.string().uuid() }),
      body: z.object({ reason: z.string().max(300).optional() }).strict(),
    },
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    const reason = request.body?.reason ?? 'courier_aborted_en_route';

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      const res = await client.query(`
        SELECT ca.order_id, ca.shift_id, ca.status AS asg_status, o.status AS ord_status
        FROM courier_assignments ca JOIN orders o ON o.id = ca.order_id
        WHERE ca.id = $1 AND ca.courier_id = $2 AND ca.status IN ('accepted','picked_up') FOR UPDATE OF ca
      `, [id, courierId]);
      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS', 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS');
      }
      const { order_id, shift_id, asg_status, ord_status } = res.rows[0];

      // (1)+(2): terminalize the binding + take the order-side action via the SHARED rail (the same one
      // /cancel uses) — abort always frees the assignment; the transition is guarded on the locked order status.
      const { reoffered } = await releaseBindingAndReoffer(
        client,
        { assignmentId: id, orderId: order_id, shiftId: shift_id, asgStatus: asg_status, ordStatus: ord_status, locationId, reason },
        { messageBus },
      );

      await client.query('COMMIT');

      // R4-3: a binding-change broadcast (id-only, claim-check) so owner/customer realtime reconverges even
      // when the order status itself is unchanged (the flag-ON branch).
      await messageBus.publish(orderChannel(order_id), { type: 'binding_changed', orderId: order_id });
      await messageBus.publish(dashboardChannel(locationId), { type: 'assignment_aborted', orderId: order_id });
      return reply.send({ success: true, reoffered });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // deliver v2 §A: courier DECLINES an offered assignment → re-offer to the owner. 🔴 The customer order is
  // UNTOUCHED (only the binding rolls back) — never a trap-state for the customer. Re-enqueues like a reject.
  fastify.post('/assignments/:id/decline', {
    schema: { params: z.object({ id: z.string().uuid() }) },
  }, async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;
    const { id } = request.params;
    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      const res = await client.query(
        `SELECT order_id FROM courier_assignments WHERE id=$1 AND courier_id=$2 AND status='offered' FOR UPDATE`,
        [id, courierId],
      );
      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'ASSIGNMENT_NOT_FOUND_OR_NOT_OFFERED', 'ASSIGNMENT_NOT_FOUND_OR_NOT_OFFERED');
      }
      const { order_id } = res.rows[0];
      await client.query(
        `UPDATE courier_assignments SET status='offered_expired', cancelled_at=now(), cancellation_reason='courier_declined' WHERE id=$1`,
        [id],
      );
      await client.query(
        `INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now())
         ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`,
        [order_id, locationId],
      );
      await client.query('COMMIT');
      await messageBus.publish(dashboardChannel(locationId), { type: 'offer_declined', orderId: order_id });
      return reply.send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

});
