import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

const sendOtpSchema = z.object({
  phone: z.string().regex(/^\+[1-9]\d{6,14}$/, 'Must be E.164 format'),
  order_intent: z.object({
    items: z.array(z.object({
      product_id: z.string().uuid(),
      quantity: z.number().int().positive(),
    })).min(1),
    total: z.number().positive(),
    currency: z.string().length(3),
  }),
}).strict();

const verifyOtpSchema = z.object({
  phone: z.string().regex(/^\+[1-9]\d{6,14}$/),
  code: z.string().length(6).regex(/^\d{6}$/),
  otp_token: z.string().min(1),
  order_intent_hash: z.string().min(1),
}).strict();

export default (async function customerOtpRoutes(fastify, opts) {
  const { db, messageBus } = opts as any;

  // ─── OTP Send ────────────────────────────────────────────────────
  fastify.post('/locations/:slug/otp/send', {
    schema: { body: sendOtpSchema },
    config: { rateLimit: { max: 3, timeWindow: '15 minutes', keyGenerator: (req: any) => req.body?.phone || req.ip } },
  }, async (request, reply) => {
    const { slug } = request.params as any;
    const { phone, order_intent } = request.body;

    // 1. Resolve location
    const locRes = await db.query(
      `SELECT id, require_phone_otp FROM locations WHERE slug = $1`,
      [slug],
    );
    if (locRes.rowCount === 0) return reply.status(404).send({ error: 'Location not found' });
    const location = locRes.rows[0];

    // 2. Check OTP toggle
    if (!location.require_phone_otp) {
      return reply.status(400).send({ error: 'OTP_NOT_REQUIRED', message: 'Phone verification is not required for this location' });
    }

    // 3. Rate-limit check (per 15min per phone per location)
    const sendRes = await db.query(
      `SELECT COUNT(*)::int AS cnt FROM phone_otp
       WHERE location_id = $1 AND phone = $2
         AND created_at > now() - interval '15 minutes'`,
      [location.id, phone],
    );
    if (sendRes.rows[0].cnt >= 3) {
      return reply.status(429).send({ error: 'OTP_RATE_LIMIT', message: 'Too many OTP requests. Try again later.' });
    }

    // 4. Generate code and hash
    const { generateOtpCode, hashOtpCode, generateOpaqueToken, hashPhone, hashOrderIntent } = await import('../lib/otp.js');
    const code = generateOtpCode();
    const codeHash = await hashOtpCode(code);
    const { token: otpToken, hash: tokenHash } = generateOpaqueToken();
    const phoneHash = hashPhone(phone);
    const intentHash = hashOrderIntent(order_intent.items);

    // 5. Store OTP + session
    const client = await db.connect();
    try {
      await client.query('BEGIN');

      await client.query(
        `INSERT INTO phone_otp (location_id, phone, code_hash, expires_at)
         VALUES ($1, $2, $3, now() + interval '5 minutes')`,
        [location.id, phone, codeHash],
      );

      await client.query(
        `INSERT INTO customer_otp_sessions (location_id, phone_hash, purpose, token_hash, order_intent_hash, expires_at)
         VALUES ($1, $2, 'otp_verified', $3, $4, now() + interval '5 minutes')`,
        [location.id, phoneHash, tokenHash, intentHash],
      );

      await client.query('COMMIT');
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }

    // 6. Log (SMS scaffold — P5+ real gateway)
    const { maskPhone } = await import('../lib/otp.js');
    console.log(`[OTP] Sending code to ${maskPhone(phone)} for location ${slug}`);

    // Publish event
    await messageBus.publish(`otp.sent`, { locationId: location.id, phoneHash });

    return reply.send({
      otp_token: otpToken,
      expires_in_ms: 5 * 60 * 1000,
    });
  });

  // ─── OTP Verify ──────────────────────────────────────────────────
  fastify.post('/locations/:slug/otp/verify', {
    schema: { body: verifyOtpSchema },
    config: { rateLimit: { max: 5, timeWindow: '15 minutes', keyGenerator: (req: any) => req.body?.phone || req.ip } },
  }, async (request, reply) => {
    const { slug } = request.params as any;
    const { phone, code, otp_token, order_intent_hash } = request.body;

    // 1. Resolve location
    const locRes = await db.query(
      `SELECT id, require_phone_otp FROM locations WHERE slug = $1`,
      [slug],
    );
    if (locRes.rowCount === 0) return reply.status(404).send({ error: 'Location not found' });
    const location = locRes.rows[0];

    // 2. Find OTP session
    const { hashPhone, verifyOtpCode, generateOpaqueToken, hashOrderIntent } = await import('../lib/otp.js');
    const phoneHash = hashPhone(phone);

    // 3. Find latest unspent OTP
    const otpRes = await db.query(
      `SELECT id, code_hash, attempts, consumed_at
       FROM phone_otp
       WHERE location_id = $1 AND phone = $2
         AND consumed_at IS NULL
         AND expires_at > now()
       ORDER BY created_at DESC LIMIT 1`,
      [location.id, phone],
    );
    if (otpRes.rowCount === 0) {
      return reply.status(410).send({ error: 'OTP_EXPIRED', message: 'No valid OTP found. Request a new one.' });
    }
    const otpRow = otpRes.rows[0];

    // 4. Check attempts
    if (otpRow.attempts >= 5) {
      return reply.status(429).send({ error: 'OTP_LOCKOUT', message: 'Too many failed attempts. Try again later.', retryAfterMs: 3600000 });
    }

    // 5. Find otp session token
    const tokenHash = require('crypto').createHash('sha256').update(otp_token).digest('hex');
    const sessionRes = await db.query(
      `SELECT id FROM customer_otp_sessions
       WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > now()`,
      [tokenHash],
    );
    if (sessionRes.rowCount === 0) {
      return reply.status(401).send({ error: 'INVALID_TOKEN', message: 'OTP session not found or expired.' });
    }

    // 6. Verify code
    const codeValid = await verifyOtpCode(code, otpRow.code_hash);
    if (!codeValid) {
      await db.query(`UPDATE phone_otp SET attempts = COALESCE(attempts, 0) + 1 WHERE id = $1`, [otpRow.id]);

      if (otpRow.attempts + 1 >= 5) {
        await db.query(`UPDATE phone_otp SET consumed_at = now() WHERE id = $1`, [otpRow.id]);
        return reply.status(429).send({ error: 'OTP_LOCKOUT', message: 'Too many failed attempts. OTP invalidated.', retryAfterMs: 3600000 });
      }

      return reply.status(401).send({ error: 'INVALID_CODE', message: 'Invalid verification code.', remainingAttempts: 5 - otpRow.attempts - 1 });
    }

    // 7. Mark consumed
    const client = await db.connect();
    try {
      await client.query('BEGIN');

      await client.query(`UPDATE phone_otp SET consumed_at = now(), attempts = COALESCE(attempts, 0) + 1 WHERE id = $1`, [otpRow.id]);
      await client.query(`UPDATE customer_otp_sessions SET consumed_at = now() WHERE id = $1`, [sessionRes.rows[0].id]);

      // Issue verified token
      const { token: verifiedToken, hash: verifiedHash } = generateOpaqueToken();
      const intentHash = hashOrderIntent(JSON.parse(Buffer.from(order_intent_hash, 'hex').toString() || '[]'));

      await client.query(
        `INSERT INTO customer_otp_sessions (location_id, phone_hash, purpose, token_hash, order_intent_hash, expires_at)
         VALUES ($1, $2, 'otp_verified', $3, $4, now() + interval '15 minutes')`,
        [location.id, phoneHash, verifiedHash, intentHash],
      );

      await client.query('COMMIT');

      await messageBus.publish(`otp.verified`, { locationId: location.id, phoneHash, success: true });

      return reply.send({
        verified_token: verifiedToken,
        expires_in_ms: 15 * 60 * 1000,
      });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
