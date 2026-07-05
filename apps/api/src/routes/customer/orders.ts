import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { CustomerOrderStatusResponse } from '@deliveryos/shared-types';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { distanceKm } from '../../lib/geo.js';
import { loadRoute } from '../../lib/routing.js';
import { gatherOrderEtaRange } from '../../lib/etaGather.js';
import { updateOrderStatus } from '../../lib/orderStatusService.js';

const env = loadEnv();

export default (async function customerOrderRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts as any;

  // Ensure this is only accessible to authenticated customers
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['customer']));

  fastify.get('/orders/:orderId/status', {
    schema: {
      params: z.object({
        orderId: z.string().uuid()
      })
    }
  }, async (request: any, reply: any) => {
    const { orderId } = request.params;
    const userId = (request.user as any).sub;

    try {
      const orderRes = await db.query(`
         SELECT o.id, o.status, o.type, o.delivery_address, o.delivery_instructions,
                o.payment_outcome,
                o.total, o.tip_amount, o.created_at::text as created_at,
                o.delivery_lat, o.delivery_lng,
                o.promised_window_lo_min, o.promised_window_hi_min,
                o.live_eta_lo_min, o.live_eta_hi_min,
                o.confirmed_at::text   as confirmed_at,
                o.preparing_at::text   as preparing_at,
                o.ready_at::text       as ready_at,
                o.in_delivery_at::text as in_delivery_at,
                o.delivered_at::text   as delivered_at,
                o.picked_up_at::text   as picked_up_at,
                o.location_id, l.lat AS loc_lat, l.lng AS loc_lng,
               ca.courier_id, ca.status as assignment_status
        FROM orders o
        JOIN locations l ON l.id = o.location_id
        LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted', 'picked_up')
        WHERE o.id = $1 AND o.customer_id = $2
      `, [orderId, userId]);

      if (orderRes.rowCount === 0) {
        return reply.sendError(404, 'NOT_FOUND', 'Not found');
      }

      const row = orderRes.rows[0];

      const itemsRes = await db.query(
        `SELECT id, product_id, name_snapshot, price_snapshot, quantity
         FROM order_items WHERE order_id = $1`,
        [orderId]
      );

      let courierLat = null, courierLng = null, courierName = null, courierPhone = null;
      let courierMsgKind: string | null = null, courierMsgHandle: string | null = null;
      if (row.courier_id) {
        const courierRes = await db.query(`
          SELECT cp.lat, cp.lng, c.full_name_encrypted, c.phone_encrypted, c.messenger_kind, c.messenger_handle
          FROM courier_positions cp
          JOIN couriers c ON c.id = cp.courier_id
          WHERE cp.courier_id = $1
          ORDER BY cp.recorded_at DESC LIMIT 1
        `, [row.courier_id]);

        if (courierRes.rowCount > 0) {
          courierLat = courierRes.rows[0].lat;
          courierLng = courierRes.rows[0].lng;
          const enc = courierRes.rows[0].full_name_encrypted;
          courierName = enc ? String(enc).charAt(0) + '***' : null;
          const phoneEnc = courierRes.rows[0].phone_encrypted;
          courierPhone = phoneEnc ? '+*** *** ' + String(phoneEnc).substring(String(phoneEnc).length - 4) : null;
          courierMsgKind = courierRes.rows[0].messenger_kind ?? null;
          courierMsgHandle = courierRes.rows[0].messenger_handle ?? null;
        }
      }
      // UX-2: expose the courier's messenger only within an active order (parity
      // with phone; hidden once the order is terminal).
      const courierActive = !['DELIVERED', 'REJECTED', 'CANCELLED'].includes(row.status);

      let etaMinutes = null;
      if (row.assignment_status === 'picked_up' && courierLat != null && courierLng != null && row.delivery_lat != null && row.delivery_lng != null) {
        const distKm = distanceKm(
          Number(courierLat), Number(courierLng),
          Number(row.delivery_lat), Number(row.delivery_lng)
        );
        if (distKm > 0) {
          etaMinutes = Math.max(1, Math.round((distKm / 25) * 60));
        }
      }

      // Existing rating, if the customer already rated. order_ratings may not
      // exist until its migration is applied — fail soft so the page still loads.
      let rating: number | null = null, feedback: string | null = null;
      try {
        const rr = await db.query(`SELECT rating, feedback FROM order_ratings WHERE order_id = $1`, [orderId]);
        if (rr.rowCount > 0) { rating = rr.rows[0].rating; feedback = rr.rows[0].feedback; }
      } catch { /* table not yet migrated */ }

      // Stored road route (advisory) for reconnecting clients — the worker pushes it
      // live to order:{id} once at picked_up; this serves a client that joined late.
      // Redis is the fast path; order_routes is the durable fallback once the Redis
      // entry has expired (2h TTL).
      let storedRoute = await loadRoute(orderId);
      if (!storedRoute) {
        try {
          const dr = await db.query(
            `SELECT polyline, distance_meters, duration_seconds FROM order_routes WHERE order_id = $1`,
            [orderId],
          );
          if (dr.rowCount > 0) {
            // polyline is stored as JSON text; provider is metrics-only and not persisted.
            storedRoute = { polyline: JSON.parse(dr.rows[0].polyline), distance_m: dr.rows[0].distance_meters, duration_s: dr.rows[0].duration_seconds, provider: 'self' };
          }
        } catch { /* order_routes may not be migrated yet — advisory, fail soft */ }
      }

      // ETA range (v1) — compute-on-read; honest [low,high], never a single number / 0.
      // Fails soft: a bad ETA must never break the order-status page.
      let etaRange = null;
      try {
        etaRange = await gatherOrderEtaRange(db, {
          orderId: row.id,
          status: row.status,
          locationId: row.location_id,
          createdAt: row.created_at,
          preparingAt: row.preparing_at,
          deliveryLat: row.delivery_lat,
          deliveryLng: row.delivery_lng,
          locationLat: row.loc_lat,
          locationLng: row.loc_lng,
          courierId: row.courier_id ?? null,
          assignmentStatus: row.assignment_status ?? null,
          courierLat,
          courierLng,
        });
      } catch (e) {
        request.log.error({ e }, 'etaRange compute failed (soft)');
      }

      return reply.status(200).send({
        id: row.id,
        status: row.status,
        // deliver v2 (Q4): surface the customer's OWN recorded outcome so a customer recorded as a refuser
        // (refused_goods/refused_payment/customer_cancelled_on_door) can SEE and contest it — the accused sees
        // the accusation (the inversion-of-C2 fix). Code only; the FE i18n-maps it humanely. paid_full/pending
        // need no callout.
        outcome: row.payment_outcome && row.payment_outcome !== 'pending' && row.payment_outcome !== 'paid_full'
          ? { code: row.payment_outcome }
          : null,
        type: row.type,
        rating,
        feedback,
        route: storedRoute
          ? { polyline: storedRoute.polyline, durationSeconds: storedRoute.duration_s, distanceMeters: storedRoute.distance_m }
          : null,
        canRate: row.status === 'DELIVERED' && rating == null,
        deliveryAddress: row.delivery_address,
        deliveryInstructions: row.delivery_instructions,
        total: row.total,
        tipAmount: row.tip_amount || 0,
        items: itemsRes.rows.map((r: any) => ({
          id: r.id,
          productId: r.product_id,
          nameSnapshot: r.name_snapshot,
          priceSnapshot: r.price_snapshot,
          quantity: r.quantity,
        })),
        createdAt: row.created_at,
        // ORDER-TRACKING: per-transition timestamps (nullable until reached).
        // The stepper lights up filled steps from these; falls back to
        // status-only when null.
        confirmedAt: row.confirmed_at,
        preparingAt: row.preparing_at,
        readyAt: row.ready_at,
        inDeliveryAt: row.in_delivery_at,
        deliveredAt: row.delivered_at,
        pickedUpAt: row.picked_up_at,
        etaMinutes,
        etaRange, // { lowMin, highMin, phase, overdue } | null — the v1 honest range
        // SENSOR-BUS §1.1 (ESTOP-1): the frozen first promise (measurement) vs the live customer
        // truth channel, recomputed per stage with the width-floor + absolute cap. Both bounds only
        // (range-never-point); null until the order is confirmed.
        promisedWindow: row.promised_window_lo_min != null && row.promised_window_hi_min != null
          ? { loMin: Number(row.promised_window_lo_min), hiMin: Number(row.promised_window_hi_min) }
          : null,
        liveEta: row.live_eta_lo_min != null && row.live_eta_hi_min != null
          ? { loMin: Number(row.live_eta_lo_min), hiMin: Number(row.live_eta_hi_min) }
          : null,

        courierName: row.courier_id ? courierName : null,
        courierPhoneMasked: row.courier_id ? courierPhone : null,
        courierMessenger: row.courier_id && courierActive && courierMsgKind && courierMsgHandle
          ? { kind: courierMsgKind, handle: courierMsgHandle }
          : null,
        courierPosition: courierLat != null && courierLng != null ? { lat: Number(courierLat), lng: Number(courierLng) } : null,
        deliveryLat: row.delivery_lat != null ? Number(row.delivery_lat) : null,
        deliveryLng: row.delivery_lng != null ? Number(row.delivery_lng) : null,
      });
    } catch (err) {
      request.log.error(err);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    }
  });

  // Customer leaves a 1–5 star rating + optional feedback on a DELIVERED order.
  // Exactly-once (UPSERT on order_id), editable within a 24h window, ownership
  // enforced by customer_id = token.sub.
  fastify.post('/orders/:orderId/rating', {
    schema: {
      params: z.object({ orderId: z.string().uuid() }),
      body: z.object({
        rating: z.number().int().min(1).max(5),
        feedback: z.string().max(1000).optional(),
      }),
    },
  }, async (request: any, reply: any) => {
    const { orderId } = request.params;
    const { rating, feedback } = request.body;
    const userId = (request.user as any).sub;
    try {
      const o = await db.query(`
        SELECT o.location_id, o.status, o.delivered_at, ca.courier_id
        FROM orders o
        LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status = 'delivered'
        WHERE o.id = $1 AND o.customer_id = $2
      `, [orderId, userId]);
      if (o.rowCount === 0) return reply.sendError(404, 'NOT_FOUND', 'Not found');
      const row = o.rows[0];
      if (row.status !== 'DELIVERED') {
        return reply.sendError(409, 'NOT_DELIVERED', 'Order not delivered yet');
      }
      if (row.delivered_at && Date.now() - new Date(row.delivered_at).getTime() > 24 * 60 * 60 * 1000) {
        return reply.sendError(409, 'RATING_WINDOW_CLOSED', 'Rating window has closed');
      }
      await db.query(`
        INSERT INTO order_ratings (order_id, location_id, courier_id, customer_id, rating, feedback)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (order_id) DO UPDATE
          SET rating = EXCLUDED.rating, feedback = EXCLUDED.feedback, updated_at = now()
      `, [orderId, row.location_id, row.courier_id, userId, rating, feedback ?? null]);
      return reply.status(200).send({ success: true, rating, feedback: feedback ?? null });
    } catch (err) {
      request.log.error(err);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    }
  });

  fastify.post('/orders/:orderId/cancel', {
    schema: {
      params: z.object({
        orderId: z.string().uuid()
      }),
      body: z.object({
        reason: z.string().min(5).max(500)
      })
    }
  }, async (request: any, reply: any) => {
    const { orderId } = request.params;
    const { reason } = request.body;
    const userId = (request.user as any).sub;

    const cancelWindowMs = parseInt((env as any).CANCEL_AFTER_DISPATCH_WINDOW_MS || '300000', 10); // 5 min default
    
    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // 1. Fetch order and assignment to verify ownership, status, and time
      const orderRes = await client.query(`
        SELECT o.location_id, o.status, ca.picked_up_at
        FROM orders o
        JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status = 'picked_up'
        WHERE o.id = $1 AND o.customer_id = $2
        FOR UPDATE OF o
      `, [orderId, userId]);

      if (orderRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(403, 'FORBIDDEN', 'Not your order');
      }

      const order = orderRes.rows[0];

      if (order.status !== 'IN_DELIVERY') {
        await client.query('ROLLBACK');
        return reply.sendError(409, 'CANCEL_NOT_ALLOWED_STATUS', 'CANCEL_NOT_ALLOWED_STATUS');
      }

      const outForDeliveryAt = new Date(order.picked_up_at).getTime();
      const now = Date.now();

      if (now - outForDeliveryAt > cancelWindowMs) {
        await client.query('ROLLBACK');
        return reply.sendError(410, 'CANCEL_WINDOW_EXPIRED', 'CANCEL_WINDOW_EXPIRED');
      }

      // 2. Cancel through the sanctioned mutator (LC3 fix, ADR-audit-fix-money §3.3.2 / DEP-1).
      // The old raw UPDATE wrote orders.cancelled_at/cancellation_reason — columns that exist in
      // NO migration (they live on courier_assignments only) → Postgres 42703 → this route
      // 500-rolled-back on EVERY call. Routing through updateOrderStatus:
      //   • sets only real columns (status/timeout_at), status-guarded against races;
      //   • terminalizes the active assignment + frees the shift in the SAME tx (R2-3 fold);
      //   • writes the order_status_history audit row + live WS deltas (owner dashboard/customer);
      //   • records the 'refund_due' obligation for a crypto-PAID order via the L-A fold — the
      //     customer cancel is exactly the case that most needs the refund (P4b).
      //
      // DEP-1(b) tenant context + N7 minimal-GUC-window: location_id comes from the ownership-
      // verified read above (WHERE o.customer_id = $2 — the customer can only ever set the tenant
      // of an order they own). The GUC is set IMMEDIATELY before the mutation and ONLY
      // ownership-scoped statements run while it is set (tx-scoped: dies at COMMIT). Exact
      // precedent: payments-webhook.ts:41 (DEFINER resolver → GUC → dual-policy GUC arm) — this is
      // what makes the fold's payment_events insert pass FORCE RLS pre- and post-B3.
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [order.location_id]);
      await updateOrderStatus(client, orderId, order.location_id, 'CANCELLED', {
        messageBus,
        comment: reason,
      });

      await client.query('COMMIT');

      // 5. Notify
      await messageBus.publish(BUS_CHANNELS.ORDER_CANCEL_AFTER_DISPATCH, {
        orderId,
        locationId: order.location_id,
        reason
      });

      return reply.status(200).send({ success: true });
    } catch (err: any) {
      await client.query('ROLLBACK').catch(() => {});
      // updateOrderStatus throws typed {statusCode, error, code} objects (409 CONFLICT on a lost
      // race, 400 on an illegal transition, 500 REFUND_DUE_RECORD_FAILED on a fold failure) —
      // surface them as the error envelope instead of a shapeless rethrown 500.
      if (err && typeof err.statusCode === 'number') {
        return reply.sendError(err.statusCode, err.code || 'ERROR', err.error || 'Request failed');
      }
      request.log.error(err);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
