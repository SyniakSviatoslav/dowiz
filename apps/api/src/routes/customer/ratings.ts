import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import { QUEUE_NAMES } from '../../lib/registry.js';

const LOW_STAR_THRESHOLD = 2;
const EDIT_WINDOW_MS = 24 * 60 * 60 * 1000;

const ratingBodySchema = z.object({
  stars: z.number().int().min(1).max(5),
  comment: z.string().max(2000).nullable().optional(),
}).strict();

export default (async function customerRatingsRoutes(fastify: any, opts: any) {
  const { db, queue } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['customer']));

  // POST /api/customer/orders/:orderId/rating
  fastify.post('/orders/:orderId/rating', {
    schema: {
      params: z.object({ orderId: z.string().uuid() }),
      body: ratingBodySchema,
    },
  }, async (request: any, reply: any) => {
    const { orderId } = request.params;
    const { stars, comment } = request.body;
    const customerId = (request.user as any).sub;

    const orderRes = await db.query(
      `SELECT o.id, o.status, o.location_id,
              o.delivered_at,
              ca.courier_id
       FROM orders o
       LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted', 'picked_up', 'delivered')
       WHERE o.id = $1 AND o.customer_id = $2`,
      [orderId, customerId],
    );

    if (orderRes.rowCount === 0) {
      return reply.status(404).send({ error: 'Order not found' });
    }

    const order = orderRes.rows[0];

    if (order.status !== 'DELIVERED') {
      return reply.status(422).send({ error: 'Order must be DELIVERED to rate' });
    }

    const deliveredAt = order.delivered_at ? new Date(order.delivered_at) : null;
    if (deliveredAt && Date.now() - deliveredAt.getTime() > EDIT_WINDOW_MS) {
      return reply.status(422).send({ error: 'Rating window has closed' });
    }

    const now = new Date().toISOString();

    const upsertRes = await db.query(
      `INSERT INTO order_ratings (order_id, location_id, courier_id, stars, comment, created_at, updated_at)
       VALUES ($1, $2, $3, $4, $5, $6, $6)
       ON CONFLICT (order_id) DO UPDATE
         SET stars = EXCLUDED.stars,
             comment = EXCLUDED.comment,
             updated_at = EXCLUDED.updated_at
       RETURNING id, stars, comment, created_at, updated_at`,
      [orderId, order.location_id, order.courier_id || null, stars, comment ?? null, now],
    );

    const rating = upsertRes.rows[0];

    // Fire low-rating notification (analytics category, default OFF)
    if (stars <= LOW_STAR_THRESHOLD) {
      try {
        const dedupKey = `rating.low_received:${orderId}:${order.location_id}`;
        await queue.enqueue(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
          event: 'rating.low_received',
          entity_id: orderId,
          location_id: order.location_id,
          rating: stars,
        }, { singletonKey: dedupKey });
      } catch (err) {
        console.warn('[Ratings] Failed to enqueue low-rating notification:', err);
      }
    }

    const canEdit = deliveredAt
      ? Date.now() - deliveredAt.getTime() < EDIT_WINDOW_MS
      : true;

    return reply.status(201).send({
      id: rating.id,
      stars: rating.stars,
      comment: rating.comment,
      canEdit,
      updatedAt: rating.updated_at,
    });
  });

  // GET /api/customer/orders/:orderId/rating
  fastify.get('/orders/:orderId/rating', {
    schema: {
      params: z.object({ orderId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
    const { orderId } = request.params;
    const customerId = (request.user as any).sub;

    const orderRes = await db.query(
      `SELECT o.id, o.status, o.delivered_at
       FROM orders o
       WHERE o.id = $1 AND o.customer_id = $2`,
      [orderId, customerId],
    );

    if (orderRes.rowCount === 0) {
      return reply.status(404).send({ error: 'Order not found' });
    }

    const order = orderRes.rows[0];

    const ratingRes = await db.query(
      `SELECT id, stars, comment, created_at, updated_at FROM order_ratings WHERE order_id = $1`,
      [orderId],
    );

    if (ratingRes.rowCount === 0) {
      const deliveredAt = order.delivered_at ? new Date(order.delivered_at) : null;
      const canSubmit = order.status === 'DELIVERED' &&
        (!deliveredAt || Date.now() - deliveredAt.getTime() < EDIT_WINDOW_MS);
      return reply.status(200).send({ rating: null, canSubmit });
    }

    const rating = ratingRes.rows[0];
    const deliveredAt = order.delivered_at ? new Date(order.delivered_at) : null;
    const canEdit = deliveredAt
      ? Date.now() - deliveredAt.getTime() < EDIT_WINDOW_MS
      : true;

    return reply.status(200).send({
      rating: {
        id: rating.id,
        stars: rating.stars,
        comment: rating.comment,
        updatedAt: rating.updated_at,
      },
      canSubmit: false,
      canEdit,
    });
  });
}) satisfies FastifyPluginAsync;
