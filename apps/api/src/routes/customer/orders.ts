// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

export default (async function customerOrderRoutes(fastify, opts) {
  const { db, messageBus } = opts as any;

  // Ensure this is only accessible to authenticated customers
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['customer']));

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
      await messageBus.publish('order.cancelled.customer_after_dispatch', {
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
