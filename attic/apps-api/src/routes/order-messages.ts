import { z } from 'zod';
import {
  PRESET_REGISTRY, MessagePresetKey, SendMessageRequest, MessageRecord, TERMINAL_STATUSES,
  validatePresetAllowed
} from '@deliveryos/shared-types';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';

export default async function orderMessageRoutes(fastify: any, opts: any) {
  const { db, messageBus } = opts;

  async function getOrder(request: any, orderId: string) {
    const { rows } = await db.query(
      `SELECT id, status, location_id, customer_id, delivery_instructions, payment_method, cash_pay_with
       FROM orders WHERE id = $1`,
      [orderId]
    );
    return rows[0] || null;
  }

  async function hasCourier(orderId: string) {
    const { rows } = await db.query(
      `SELECT 1 FROM courier_assignments
       WHERE order_id = $1 AND status IN ('assigned', 'accepted', 'picked_up') LIMIT 1`,
      [orderId]
    );
    return rows.length > 0;
  }

  // ─── Send message ──────────────────────────────────────────────────
  fastify.post('/api/orders/:orderId/messages', {
    preValidation: [fastify.verifyAuth]
  }, async (request: any, reply: any) => {
    const body = request.body || {};
    const orderId = request.params.orderId;

    const parsed = SendMessageRequest.safeParse({ ...body, order_id: orderId });
    if (!parsed.success) {
      return reply.status(400).send({ error: parsed.error.issues[0]?.message || 'Invalid request' });
    }

    const { preset_key, params } = parsed.data;

    // Look up preset
    const preset = PRESET_REGISTRY[preset_key];
    if (!preset) return reply.status(400).send({ error: `Unknown preset: ${preset_key}` });

    // Get order
    const order = await getOrder(request, orderId);
    if (!order) return reply.status(404).send({ error: 'Order not found' });

    // Check tenant isolation
    const role = request.user.role;
    const userId = request.user.sub;
    if (role === 'owner' || role === 'courier') {
      const memCheck = await db.query(
        `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND status = 'active'`,
        [userId, order.location_id]
      );
      if (memCheck.rows.length === 0) return reply.status(404).send({ error: 'Order not found' });
    } else if (role === 'customer') {
      if (order.customer_id !== userId) return reply.status(404).send({ error: 'Order not found' });
    }

    // Validate preset allowed for this role + status
    const allowed = validatePresetAllowed(preset, role, order.status);
    if (allowed) return reply.status(409).send({ error: allowed });

    // Courier check: cu_*/cc_* require courier assigned
    if (preset.requiresCourier !== false && (preset_key.startsWith('cu_') || preset_key.startsWith('cc_'))) {
      const courierOk = await hasCourier(orderId);
      if (!courierOk) return reply.status(409).send({ error: 'No courier assigned to this order' });
    }

    // Conditional checks
    if (preset.requiresDropoff) {
      const instructions = order.delivery_instructions || '';
      if (!instructions.toLowerCase().includes('leave')) {
        return reply.status(409).send({ error: 'Order does not have leave-at-door delivery' });
      }
    }

    if (preset.requiresCash) {
      if (order.payment_method !== 'cash' || !order.cash_pay_with) {
        return reply.status(409).send({ error: 'Order is not a cash payment' });
      }
    }

    // Validate params against preset schema
    const paramsCheck = preset.paramsSchema.safeParse(params || {});
    if (!paramsCheck.success) {
      return reply.status(400).send({ error: paramsCheck.error.issues[0]?.message || 'Invalid params' });
    }

    // Insert message
    const { rows } = await db.query(`
      INSERT INTO order_messages (order_id, location_id, sender, preset_key, params)
      VALUES ($1, $2, $3, $4, $5::jsonb)
      RETURNING id, order_id, location_id, sender, preset_key, params, body, read_at, created_at
    `, [orderId, order.location_id, role, preset_key, JSON.stringify(paramsCheck.data)]);

    const msg = rows[0];

    // Broadcast via MessageBus
    if (messageBus) {
      await messageBus.publish(orderChannel(orderId), {
        type: 'order.message',
        data: msg
      });
    }

    return reply.status(201).send({ success: true, message: msg });
  });

  // ─── Get message history ──────────────────────────────────────────
  fastify.get('/api/orders/:orderId/messages', {
    preValidation: [fastify.verifyAuth]
  }, async (request: any, reply: any) => {
    const orderId = request.params.orderId;
    const role = request.user.role;

    // Get order for tenant check
    const order = await getOrder(request, orderId);
    if (!order) return reply.status(404).send({ error: 'Order not found' });

    if (role === 'owner' || role === 'courier') {
      const memCheck = await db.query(
        `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND status = 'active'`,
        [request.user.sub, order.location_id]
      );
      if (memCheck.rows.length === 0) return reply.status(404).send({ error: 'Order not found' });
    } else if (role === 'customer') {
      if (order.customer_id !== request.user.sub) return reply.status(404).send({ error: 'Order not found' });
    }

    const { rows } = await db.query(`
      SELECT id, order_id, location_id, sender, preset_key, params, body, read_at,
             created_at::text as created_at
      FROM order_messages
      WHERE order_id = $1
      ORDER BY created_at ASC
    `, [orderId]);

    return reply.send({ success: true, messages: rows });
  });

  // ─── Mark as read ──────────────────────────────────────────────────
  fastify.post('/api/orders/:orderId/messages/read', {
    preValidation: [fastify.verifyAuth]
  }, async (request: any, reply: any) => {
    const orderId = request.params.orderId;
    const role = request.user.role;

    const order = await getOrder(request, orderId);
    if (!order) return reply.status(404).send({ error: 'Order not found' });

    if (role === 'owner' || role === 'courier') {
      const memCheck = await db.query(
        `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND status = 'active'`,
        [request.user.sub, order.location_id]
      );
      if (memCheck.rows.length === 0) return reply.status(404).send({ error: 'Order not found' });
    } else if (role === 'customer') {
      if (order.customer_id !== request.user.sub) return reply.status(404).send({ error: 'Order not found' });
    }

    await db.query(`
      UPDATE order_messages SET read_at = now()
      WHERE order_id = $1 AND read_at IS NULL
    `, [orderId]);

    return reply.send({ success: true });
  });
}
