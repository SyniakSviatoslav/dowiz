import { z } from 'zod';
import crypto from 'node:crypto';
import { issueCustomerToken } from '@deliveryos/platform';

// POST /api/customer/track/exchange
//
// Pre-auth endpoint (registered in NO_AUTH_PATHS) that turns the opaque ?t= code
// from an order tracking link into a real customer JWT. Mirrors the OAuth/OTP
// opaque-code -> POST exchange -> reissue-existing-JWT handoff already used in
// auth.ts and otp.ts.
//
// The grant table holds only order_id + location_id, but issueCustomerToken needs
// phone + customerId too, so we JOIN grant -> orders -> customers to recover them.
// Runs on the operational pool with no tenant context and an explicit
// WHERE token_hash = $1 (RLS bypassed on this path, exactly like order creation).

const exchangeSchema = z
  .object({
    // base64url(32 bytes) — generateOpaqueToken() output.
    code: z.string().min(20).max(64),
  })
  .strict();

export default (async function customerTrackRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.post('/track/exchange', {
    schema: { body: exchangeSchema },
    config: {
      rateLimit: {
        max: 10,
        timeWindow: '1 minute',
        keyGenerator: (req: any) => req.ip,
      },
    },
  }, async (request: any, reply: any) => {
    const { code } = request.body as { code: string };

    // Never log the raw code. Hash it and look up the grant.
    const tokenHash = crypto.createHash('sha256').update(code).digest('hex');

    const grantRes = await db.query(
      `SELECT g.id            AS grant_id,
              g.order_id      AS order_id,
              g.location_id   AS location_id,
              o.customer_id   AS customer_id,
              c.phone         AS phone
         FROM customer_track_grants g
         JOIN orders    o ON o.id = g.order_id
         JOIN customers c ON c.id = o.customer_id
        WHERE g.token_hash = $1
          AND g.expires_at > now()`,
      [tokenHash],
    );

    if (grantRes.rowCount === 0) {
      // Unknown, expired, or order/customer gone — single 410 for all to avoid
      // leaking which case occurred.
      return reply.status(410).send({
        error: 'TRACK_LINK_EXPIRED',
        message: 'This tracking link is no longer valid. Please reopen the menu.',
      });
    }

    const grant = grantRes.rows[0];

    // Reusable until expiry (the customer may revisit the link); use_count is for
    // observability/abuse signal, not a single-use gate.
    await db.query(
      `UPDATE customer_track_grants SET use_count = use_count + 1 WHERE id = $1`,
      [grant.grant_id],
    );

    let token: string;
    try {
      token = await issueCustomerToken({
        orderId: grant.order_id,
        locationId: grant.location_id,
        phone: grant.phone,
        customerId: grant.customer_id,
      });
    } catch (err) {
      request.log.error({ err }, 'track/exchange: failed to issue customer token');
      return reply.status(500).send({ error: 'Internal server error' });
    }

    return reply.send({ token });
  });
} as any);
