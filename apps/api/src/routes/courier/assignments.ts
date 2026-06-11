// @ts-nocheck
import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { acceptCourierAssignment } from '../../lib/courierAssignmentService';
import { MessageBus } from '@deliveryos/platform';

const env = loadEnv();

export default (async function courierAssignmentsRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts as { db: any, messageBus: MessageBus };

  fastify.addHook('preValidation', fastify.verifyAuth);
  fastify.addHook('preValidation', fastify.requireRole(['courier']));

  // 1. Get assignments
  fastify.get('/me/assignments', async (request: any, reply: any) => {
    const courierId = request.user.sub;
    const locationId = request.user.activeLocationId;

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);
      
      const res = await client.query(`
        SELECT id, order_id, status, assigned_at, accepted_at, picked_up_at, delivered_at, cash_collected, cash_amount
        FROM courier_assignments
        WHERE courier_id = $1 AND location_id = $2
          AND (status IN ('assigned', 'accepted', 'picked_up') OR created_at > now() - interval '24 hours')
        ORDER BY created_at DESC
      `, [courierId, locationId]);

      await client.query('COMMIT');
      return reply.send({ success: true, assignments: res.rows });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
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
    
    const acceptWindowMs = parseInt(env.COURIER_ACCEPT_WINDOW_MS || '30000', 10);

     const client = await db.connect();
     try {
       await client.query('BEGIN');
       await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

       // Use the service to accept the assignment
       await acceptCourierAssignment(client, id, locationId, { messageBus });

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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

      const res = await client.query(`
        SELECT order_id, shift_id FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status = 'assigned' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'ASSIGNMENT_NOT_FOUND_OR_NOT_ASSIGNED' });
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
      await messageBus.publish('order.confirmed', { orderId: order_id, locationId });

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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

      const res = await client.query(`
        SELECT order_id FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status = 'accepted' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'ASSIGNMENT_NOT_FOUND_OR_NOT_ACCEPTED' });
      }

      const { order_id } = res.rows[0];

      await client.query(`
        UPDATE courier_assignments SET status = 'picked_up', picked_up_at = now() WHERE id = $1
      `, [id]);

      await client.query('COMMIT');

      await messageBus.publish('order.picked_up', { 
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
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

      const res = await client.query(`
        SELECT ca.order_id, ca.shift_id, o.total 
        FROM courier_assignments ca
        JOIN orders o ON ca.order_id = o.id
        WHERE ca.id = $1 AND ca.courier_id = $2 AND ca.status = 'picked_up' FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP' });
      }

      const { order_id, shift_id, total } = res.rows[0];

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

      await client.query('COMMIT');

      // Integrate with Phase 1 lifecycle
      await messageBus.publish('order.delivered', { 
        orderId: order_id, 
        locationId,
        courierId,
        cashCollected: cash_collected,
        cashAmount: cash_collected ? cash_amount : null
      });

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
    
    const cancelWindowMs = parseInt(env.CANCEL_AFTER_DISPATCH_WINDOW_MS || '300000', 10);

    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.current_tenant = '${locationId}'`);

      const res = await client.query(`
        SELECT order_id, shift_id, assigned_at FROM courier_assignments 
        WHERE id = $1 AND courier_id = $2 AND status IN ('accepted', 'picked_up') FOR UPDATE
      `, [id, courierId]);

      if (res.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS' });
      }

      const { order_id, shift_id, assigned_at } = res.rows[0];
      const elapsedMs = Date.now() - new Date(assigned_at).getTime();

      if (elapsedMs > cancelWindowMs) {
        await client.query('ROLLBACK');
        return reply.status(410).send({ error: 'CANCEL_WINDOW_EXPIRED' });
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

      await messageBus.publish('order.cancelled', { 
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
