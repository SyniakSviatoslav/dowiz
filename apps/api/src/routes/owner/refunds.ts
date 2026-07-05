// Owner crypto-refund review (ADR-0017 C2). Crypto is irreversible → refunds are manual: completeDelivery
// records a 'refund_due' obligation when a paid prepaid order is refused/cancelled; the owner sends the crypto
// back out-of-band and records it here → payment_status='refunded'. DARK (returns empty / 404 when prepaid off).
import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import { withTenant } from '@deliveryos/platform';
import { isPrepaidEnabled } from '../../lib/payments/registry.js';

export default (async function ownerRefundsRoutes(fastify: any, opts: any) {
  const { db } = opts;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // Pending refund obligations: a 'refund_due' with no matching 'refund_sent' yet.
  fastify.get('/:locationId/refunds', {
    schema: { params: z.object({ locationId: z.string().uuid() }) },
  }, async (request: any, reply: any) => {
    if (!isPrepaidEnabled()) return reply.send({ refunds: [] });
    const { locationId } = request.params;
    const userId = (request.user as any).userId;
    const res: any = await withTenant(db, userId, (client: any) =>
      client.query(
        `SELECT pe.payment_id, p.order_id, pe.amount_minor, pe.currency_code, pe.created_at,
                p.provider, p.provider_payment_id
           FROM payment_events pe JOIN payments p ON p.id = pe.payment_id
          WHERE pe.type = 'refund_due' AND p.location_id = $1
            AND NOT EXISTS (SELECT 1 FROM payment_events s WHERE s.payment_id = pe.payment_id AND s.type = 'refund_sent')
          ORDER BY pe.created_at DESC LIMIT 100`,
        [locationId],
      ),
    );
    return reply.send({
      refunds: res.rows.map((r: any) => ({
        paymentId: r.payment_id, orderId: r.order_id, amountMinor: r.amount_minor,
        currencyCode: r.currency_code, createdAt: r.created_at, provider: r.provider, providerRef: r.provider_payment_id,
      })),
    });
  });

  // Owner records that they sent the crypto back → refunded. Idempotent (insert-wins on refund_sent).
  fastify.post('/:locationId/refunds/:paymentId/sent', {
    schema: {
      params: z.object({ locationId: z.string().uuid(), paymentId: z.string().uuid() }),
      body: z.object({ txRef: z.string().max(200).optional() }).optional(),
    },
  }, async (request: any, reply: any) => {
    if (!isPrepaidEnabled()) return reply.sendError(404, 'NOT_FOUND', 'Not found');
    const { locationId, paymentId } = request.params;
    const txRef = (request.body as any)?.txRef;
    const userId = (request.user as any).userId;
    const orderId = await withTenant(db, userId, async (client: any) => {
      const p = await client.query(
        `SELECT id, order_id, provider, provider_payment_id, amount_minor, currency_code, location_id
           FROM payments WHERE id = $1 AND location_id = $2`,
        [paymentId, locationId],
      );
      if (p.rowCount === 0) return null;
      const pr = p.rows[0];
      await client.query(
        `INSERT INTO payment_events
           (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified, payload)
         VALUES ($1, $2, $3, $4, 'refund_sent', $5, $6, true, $7::jsonb)
         ON CONFLICT (provider, provider_payment_id, type) DO NOTHING`,
        [pr.id, pr.location_id, pr.provider, pr.provider_payment_id, pr.amount_minor, pr.currency_code, JSON.stringify({ txRef: txRef || null })],
      );
      // residual-guard holds: refunded(amount) <= captured(amount, set by the webhook) <= amount.
      await client.query(`UPDATE payments SET status = 'refunded', refunded_amount_minor = amount_minor, updated_at = now() WHERE id = $1`, [pr.id]);
      await client.query(`UPDATE orders SET payment_status = 'refunded' WHERE id = $1`, [pr.order_id]);
      return pr.order_id as string;
    });
    if (!orderId) return reply.sendError(404, 'NOT_FOUND', 'Payment not found');
    return reply.send({ ok: true, orderId });
  });
} as FastifyPluginAsync);
