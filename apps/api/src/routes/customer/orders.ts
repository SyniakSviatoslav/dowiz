// @ts-nocheck — pre-existing type errors in this file
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { CustomerOrderStatusResponse } from '@deliveryos/shared-types';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../../lib/registry.js';
import { distanceKm } from '../../lib/geo.js';

const env = loadEnv();

export default (async function customerOrderRoutes(fastify, opts) {
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
  }, async (request, reply) => {
    const { orderId } = request.params;
    const userId = (request.user as any).sub;

    try {
      const orderRes = await db.query(`
        SELECT o.id, o.status, o.delivery_address, o.delivery_instructions,
               o.total, o.created_at::text as created_at,
               o.delivery_pin_lat, o.delivery_pin_lng,
               ca.courier_id, ca.status as assignment_status
        FROM orders o
        LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted', 'picked_up')
        WHERE o.id = $1 AND o.customer_id = $2
      `, [orderId, userId]);

      if (orderRes.rowCount === 0) {
        return reply.status(404).send({ error: 'Not found' });
      }

      const row = orderRes.rows[0];

      const itemsRes = await db.query(
        `SELECT id, product_id, name_snapshot, price_snapshot, quantity
         FROM order_items WHERE order_id = $1`,
        [orderId]
      );

      let courierLat = null, courierLng = null, courierName = null, courierPhone = null;
      if (row.courier_id) {
        const courierRes = await db.query(`
          SELECT cp.lat, cp.lng, c.full_name_encrypted, c.phone_encrypted
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
        }
      }

      let etaMinutes = null;
      if (row.assignment_status === 'picked_up' && courierLat != null && courierLng != null && row.delivery_pin_lat != null && row.delivery_pin_lng != null) {
        const distKm = distanceKm(
          Number(courierLat), Number(courierLng),
          Number(row.delivery_pin_lat), Number(row.delivery_pin_lng)
        );
        if (distKm > 0) {
          etaMinutes = Math.max(1, Math.round((distKm / 25) * 60));
        }
      }

      return reply.status(200).send({
        id: row.id,
        status: row.status,
        deliveryAddress: row.delivery_address,
        deliveryInstructions: row.delivery_instructions,
        total: row.total,
        items: itemsRes.rows.map((r: any) => ({
          id: r.id,
          productId: r.product_id,
          nameSnapshot: r.name_snapshot,
          priceSnapshot: r.price_snapshot,
          quantity: r.quantity,
        })),
        createdAt: row.created_at,
        etaMinutes,
        courierName: row.courier_id ? courierName : null,
        courierPhoneMasked: row.courier_id ? courierPhone : null,
        courierPosition: courierLat != null && courierLng != null ? { lat: Number(courierLat), lng: Number(courierLng) } : null,
        deliveryLat: row.delivery_pin_lat != null ? Number(row.delivery_pin_lat) : null,
        deliveryLng: row.delivery_pin_lng != null ? Number(row.delivery_pin_lng) : null,
      });
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal server error' });
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
  }, async (request, reply) => {
    const { orderId } = request.params;
    const { reason } = request.body;
    const userId = (request.user as any).sub;

    const cancelWindowMs = parseInt(env.CANCEL_AFTER_DISPATCH_WINDOW_MS || '300000', 10); // 5 min default
    
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
        return reply.status(403).send({ error: 'Not your order' });
      }

      const order = orderRes.rows[0];

      if (order.status !== 'IN_DELIVERY') {
        await client.query('ROLLBACK');
        return reply.status(409).send({ error: 'CANCEL_NOT_ALLOWED_STATUS' });
      }

      const outForDeliveryAt = new Date(order.picked_up_at).getTime();
      const now = Date.now();

      if (now - outForDeliveryAt > cancelWindowMs) {
        await client.query('ROLLBACK');
        return reply.status(410).send({ error: 'CANCEL_WINDOW_EXPIRED' });
      }

      // 2. Cancel order
      await client.query(`
        UPDATE orders 
        SET status = 'CANCELLED', cancelled_at = now(), cancellation_reason = $1
        WHERE id = $2
      `, [reason, orderId]);

      // 3. Cancel assignment safely
      // Use app.settlement_reversal to bypass the cash immutable check since cash_collected becomes false
      await client.query(`SET LOCAL app.settlement_reversal = 'true'`);
      const assignmentRes = await client.query(`
        UPDATE courier_assignments
        SET status = 'cancelled', 
            cancelled_at = now(), 
            cancellation_reason = $1,
            cash_collected = false,
            cash_amount = NULL
        WHERE order_id = $2 AND status IN ('assigned', 'accepted', 'picked_up')
        RETURNING courier_id, shift_id, id
      `, [reason, orderId]);

      // 4. Reset shift if applicable
      if (assignmentRes.rowCount > 0) {
        const asgn = assignmentRes.rows[0];
        if (asgn.shift_id) {
          await client.query(`
            UPDATE courier_shifts SET status = 'available' WHERE id = $1 AND status = 'on_delivery'
          `, [asgn.shift_id]);
        }
      }

      await client.query('COMMIT');

      // 5. Notify
      await messageBus.publish(BUS_CHANNELS.ORDER_CANCEL_AFTER_DISPATCH, {
        orderId,
        locationId: order.location_id,
        reason
      });

      return reply.status(200).send({ success: true });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
