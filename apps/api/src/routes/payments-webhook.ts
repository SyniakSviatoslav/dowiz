// Plisio crypto webhook — the money SOURCE OF TRUTH (ADR-0017 C3). HMAC-verified (fail-closed), idempotent
// (insert-wins), the ONLY writer of payment_status=paid/failed. DARK behind PAYMENTS_CRYPTO_ENABLED → 404.
// Tenancy: DEFINER resolver → set app.current_tenant (the dual RLS policy admits the GUC writer); becomes
// load-bearing once B3 removes BYPASSRLS, defense-in-depth until then.
import type { FastifyInstance } from 'fastify';
import { getPaymentProvider, isCryptoEnabled } from '../lib/payments/registry.js';

interface Opts { db: any }

export default async function paymentsWebhookRoutes(fastify: FastifyInstance, opts: Opts) {
  const db = opts.db;

  fastify.post('/webhook/payments/plisio', { config: { rawBody: true } }, async (request, reply) => {
    if (!isCryptoEnabled()) return reply.code(404).send({ error: 'not found' });

    const provider = getPaymentProvider();
    if (provider.name !== 'plisio') return reply.code(404).send({ error: 'not found' });

    const payload = (request.body ?? {}) as Record<string, unknown>;
    const rawBody = (request as any).rawBody as Buffer | undefined;

    // 1. Signature (fail-closed). A forged/garbled body is NOT 200'd (no silent swallow) — 401.
    if (!provider.verifyWebhook(rawBody ?? Buffer.from(''), payload)) {
      return reply.code(401).send({ error: 'invalid signature' });
    }
    const event = provider.parseEvent(payload);
    if (!event.providerPaymentId) return reply.code(400).send({ error: 'missing ref' });

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // 2. Tenant resolve WITHOUT a member (the C3 crux): DEFINER returns only the location_id.
      const locRes = await client.query(`SELECT payment_location_by_provider_ref($1, $2) AS loc`, ['plisio', event.providerPaymentId]);
      const locationId = locRes.rows[0]?.loc as string | null;
      if (!locationId) {
        // Unknown ref (charge never recorded / already pruned) — ACK to stop redelivery; nothing to write.
        await client.query('ROLLBACK');
        return reply.code(200).send({ ok: true });
      }
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);

      // 3. Idempotent ledger insert (insert-wins, NOT check-then-act). Composite unique admits the
      //    pending→completed progression while killing same-status replays (Plisio resends txn_id).
      const ins = await client.query(
        `INSERT INTO payment_events
           (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified, payload)
         SELECT p.id, p.location_id, 'plisio', $1, $2, $3, $4, true, $5::jsonb
           FROM payments p WHERE p.provider = 'plisio' AND p.provider_payment_id = $1
         ON CONFLICT (provider, provider_payment_id, type) DO NOTHING
         RETURNING id`,
        [event.providerPaymentId, event.type, event.amountMinor, event.currencyCode,
         JSON.stringify({ status: payload['status'] ?? null, confirmations: payload['confirmations'] ?? null })],
      );

      // 4. Guarded transition — ONLY on a genuinely new event (rowcount=1), status-guarded (monotonic).
      if (ins.rowCount === 1) {
        if (event.type === 'completed') {
          await client.query(
            `UPDATE payments SET status='paid', captured_amount_minor=amount_minor, updated_at=now()
             WHERE provider='plisio' AND provider_payment_id=$1 AND status NOT IN ('refunded','paid')`,
            [event.providerPaymentId],
          );
          // The only writer of orders.payment_status='paid'. Held order can now be offered to fulfillment.
          await client.query(
            `UPDATE orders SET payment_status='paid'
             WHERE id = (SELECT order_id FROM payments WHERE provider='plisio' AND provider_payment_id=$1)
               AND payment_status IN ('pending','authorized')`,
            [event.providerPaymentId],
          );
          // L-B (ADR-audit-fix-money §3.4 / LC6 pay-after-cancel): the paid flips above STAY even on a
          // terminal order (money truth: funds arrived) — but if the order is already CANCELLED/REJECTED
          // the obligation is recorded in the SAME tx, so `paid + refund_due(unmatched)` lands atomically
          // and the owner refunds queue (owner/refunds.ts) shows it. Idempotent: bare ON CONFLICT DO
          // NOTHING rides payment_events_idem_unique AND the refund_due-per-payment partial unique
          // (mig 086, N5); a race with the cancel-side writers (L-A/L-C) resolves to exactly one row.
          await client.query(
            `INSERT INTO payment_events
               (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
             SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
               FROM payments p JOIN orders o ON o.id = p.order_id
              WHERE p.provider = 'plisio' AND p.provider_payment_id = $1 AND p.status = 'paid'
                AND o.status IN ('CANCELLED','REJECTED')
             ON CONFLICT DO NOTHING`,
            [event.providerPaymentId],
          );
        } else if (event.type === 'failed') {
          await client.query(
            `UPDATE payments SET status='failed', updated_at=now()
             WHERE provider='plisio' AND provider_payment_id=$1 AND status='pending'`,
            [event.providerPaymentId],
          );
          await client.query(
            `UPDATE orders SET payment_status='failed'
             WHERE id = (SELECT order_id FROM payments WHERE provider='plisio' AND provider_payment_id=$1)
               AND payment_status='pending'`,
            [event.providerPaymentId],
          );
        }
        // 'mismatch' (under/over-payment) → event recorded, NO status flip → owner-review (C2 + crypto §).
      }

      await client.query('COMMIT');
      return reply.code(200).send({ ok: true });
    } catch (e) {
      await client.query('ROLLBACK').catch(() => {});
      request.log.error({ err: e }, 'plisio webhook failed');
      return reply.code(500).send({ error: 'internal' }); // real error → let Plisio retry
    } finally {
      client.release();
    }
  });
}
