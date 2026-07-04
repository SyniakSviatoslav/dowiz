import type { FastifyInstance } from 'fastify';
import { CreateOrderInput, StatusUpdateInput } from '@deliveryos/shared-types';
import { assertTransition, type OrderStatus } from '@deliveryos/domain';
import { issueCustomerToken, withTenant } from '@deliveryos/platform';
import type { QueueProvider, MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { BUS_CHANNELS, orderChannel, dashboardChannel, courierChannel } from '../lib/registry.js';
import { updateOrderStatus } from '../lib/orderStatusService';
import { assertOwnerTargetAllowed } from '../lib/orderAuthz.js';
import { attemptHonestDispatch } from '../lib/dispatch.js';
import { getPaymentProvider, isPrepaidEnabled, isCryptoEnabled } from '../lib/payments/registry.js';
import { evaluatePreflight } from '../lib/preflight.js';
import { computeSignals } from '../lib/signals/compute.js';
import { courierReadVerdict } from '../lib/courier-room-authz.js';
import { clientIp } from '../lib/client-ip.js';
import type { Pool } from 'pg';
import crypto from 'crypto';

const env = loadEnv();
// OTP globally disabled until a real SMS gateway exists; per-location
// require_phone_otp only applies when OTP_ENABLED is 'true'. See packages/config.
const OTP_ENABLED = env.OTP_ENABLED === 'true';
import { applyTax, assertNonNegative } from '../lib/money.js';
import { computeOrderPricing, resolveDeliveryFee } from '../lib/order-pricing.js';
import { buildRequestHash, buildSignalState } from '../lib/order-canonical.js';
import { insertOrderWithItems } from '../lib/order-persistence.js';
import { isVenueOpen } from '../lib/venue-open.js';
import { channelFromHeader } from '../lib/channel.js';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
function isValidUUID(id: string): boolean {
  return UUID_RE.test(id);
}

interface OrderRouteOptions {
  db: Pool;
  messageBus: MessageBus;
  queue: QueueProvider;
}

function mapOrderRow(row: Record<string, unknown>) {
  return {
    id: row.id,
    locationId: row.location_id,
    customerId: row.customer_id,
    status: row.status,
    type: row.type,
    deliveryAddress: row.delivery_address,
    deliveryInstructions: row.delivery_instructions,
    subtotal: row.subtotal,
    total: row.total,
    paymentMethod: row.payment_method,
    paymentOutcome: row.payment_outcome,
    createdAt: row.created_at,
    timeoutAt: row.timeout_at,
  };
}

function mapItemRow(row: Record<string, unknown>) {
  return {
    id: row.id,
    productId: row.product_id,
    nameSnapshot: row.name_snapshot,
    priceSnapshot: row.price_snapshot,
    quantity: row.quantity,
  };
}

export default async function orderRoutes(fastify: FastifyInstance, opts: OrderRouteOptions) {
  const { db, messageBus, queue } = opts;

  // ─── POST /orders ──────────────────────────────────────────────────
  fastify.post('/orders', {
    config: {
      rateLimit: {
        max: 5,
        timeWindow: '1 minute',
        keyGenerator: (req: any) => {
          const phone = req.body?.customer?.phone || req.body?.phone;
          return phone ? `phone:${phone.replace(/\s/g, '').toLowerCase()}` : req.ip;
        },
        errorResponseBuilder: (_req: any, context: any) => ({
          statusCode: 429,
          error: 'Too many orders from this phone',
          code: 'PHONE_THROTTLE',
          message: `Rate limit exceeded, retry in ${context.after}`,
        }),
      },
    },
  }, async (request: any, reply: any) => {
    let input;
    try {
      input = CreateOrderInput.parse(request.body);
    } catch (err: any) {
      const issues = err?.issues?.map((i: any) => i.message).join('; ');
      return reply.sendError(400, 'VALIDATION_FAILED', issues || 'Validation error');
    }
    const { locationId, items, customer: cust, delivery, idempotency_key, cash_pay_with: cashPayWith, delivery_instructions: rawInstructions } = input;
    // Acquisition-channel attribution (QR/ATTRIBUTION) — travels as a header, not a
    // CreateOrderInput field (see lib/channel.ts for why). Write-only; never used below
    // for pricing/routing/authz — folded into orders.metadata via insertOrderWithItems.
    const channel = channelFromHeader(request.headers?.['x-channel']);
    // Pickup orders carry no delivery pin/address and incur no delivery fee.
    const isPickup = input.type === 'pickup';
    const pin = isPickup ? null : delivery!.pin;
    const deliveryAddressText = isPickup ? null : (delivery?.address_text ?? null);

    let client;
    try {
      client = await db.connect();
    } catch (err: unknown) {
      request.log.error({ err }, 'Failed to acquire DB connection');
      return reply.sendError(503, 'SERVICE_UNAVAILABLE', 'Service temporarily unavailable');
    }

    try {
      await client.query('BEGIN');
      // Bound the write-transaction hold so a wedged write self-aborts as a fast
      // 5xx instead of holding 1 of max:8 operational connections to exhaustion
      // (the live pool-wedge failure mode). 4.5s sits just inside
      // connectionTimeoutMillis=5000 (packages/db/src/index.ts) with margin; a
      // legitimate worst-case cart (~60 inserts × ~50ms ≈ 3s under a latency spike)
      // still completes inside it. This aborts a stuck write, not a slow-but-progressing one.
      await client.query("SET LOCAL statement_timeout = 4500");

      // 1. Location config (FOR UPDATE lock optionally, but standard read is fine if config updates are rare, wait we should lock locations? No need to lock config, just read it).
      const locRes = await client.query(
         `SELECT lat, lng, confirm_timeout_min, busy_mode, phone, slug, published_at,
                 currency_code, currency_minor_unit, tax_rate, price_includes_tax,
                 min_order_value, free_delivery_threshold, delivery_fee_flat,
                 require_phone_otp, hours_json, delivery_paused
          FROM locations WHERE id = $1`,
        [locationId]
      );

      if (locRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.sendError(404, 'NOT_FOUND', 'Location not found');
      }

      const location = locRes.rows[0];

      // Z7: a DRAFT storefront (never published) shows a preview but must NOT accept
      // real orders. Existing live locations were backfilled with published_at.
      if (location.published_at == null) {
        await client.query('ROLLBACK');
        return reply.sendError(409, 'NOT_PUBLISHED', 'Storefront is not published yet');
      }

      // Closed-venue gate (audit): the storefront computes open/closed from hours_json +
      // delivery_paused, but POST /orders accepted orders after hours. When ENFORCE_VENUE_HOURS
      // is on we mirror that EXACT computation server-side (lib/venue-open.ts ← public/menu.ts)
      // and refuse a closed venue BEFORE any write. Reversible/dark by default: flag OFF ⇒
      // unchanged (always accept), so a misfiring gate can be killed instantly without a deploy.
      if (
        process.env.ENFORCE_VENUE_HOURS === 'true' &&
        !isVenueOpen(location.hours_json, location.delivery_paused ?? false, new Date())
      ) {
        await client.query('ROLLBACK');
        return reply.sendError(409, 'VENUE_CLOSED', 'VENUE_CLOSED');
      }

      // OTP verification (if column exists)
      let otpVerified = false;
      let requireOtp = false;
      try {
        const otpCol = await client.query(
          `SELECT require_phone_otp FROM locations WHERE id = $1`, [locationId]
        );
        requireOtp = OTP_ENABLED && (otpCol.rows[0]?.require_phone_otp || false);
      } catch (err: any) {
        console.debug('[orders] require_phone_otp column check failed:', err?.message);
      }
      if (requireOtp) {
        const otpHeader = request.headers['x-otp-verified'] as string | undefined;
        if (otpHeader) {
          try {
            const { hashPhone } = await import('../lib/otp.js');
            const tokenHash = crypto.createHash('sha256').update(otpHeader).digest('hex');
            const otpSessionRes = await client.query(
              `SELECT id, phone_hash, order_intent_hash, expires_at, consumed_at
               FROM customer_otp_sessions
               WHERE token_hash = $1 AND purpose = 'otp_verified'
                 AND consumed_at IS NULL AND expires_at > now()`,
              [tokenHash],
            );
            if ((otpSessionRes.rowCount ?? 0) > 0) {
              otpVerified = true;
              // Mark the session as consumed (single-use)
              await client.query(
                `UPDATE customer_otp_sessions SET consumed_at = now() WHERE id = $1`,
                [otpSessionRes.rows[0].id],
              );
            }
          } catch (err: any) {
            console.debug('[orders] OTP verification failed, proceeding without:', err?.message);
          }
        }
      }

      // 2. Menu version
      const mvRes = await client.query(
        `SELECT version FROM menu_versions WHERE location_id = $1`,
        [locationId]
      );
      const menuVersion = mvRes.rowCount && mvRes.rows[0] ? mvRes.rows[0].version : '0';

      // 3. Request Hash (idempotency fingerprint) — pure (lib/order-canonical.ts).
      const requestHash = buildRequestHash({
        locationId,
        type: input.type,
        items,
        pin,
        addressText: deliveryAddressText,
        cashPayWith,
        currencyCode: location.currency_code,
        menuVersion,
        // #8 (security-hardening-2026-07): the customer token carries no `userId` — its
        // `sub` IS the customerId (jwt.ts issueCustomerToken). Reading `userId` yielded
        // undefined → the idempotency fingerprint silently degraded to phone/IP.
        customerId: request.user?.role === 'customer' ? request.user.sub : 'anonymous',
      });

      // 4. Preflight (E27) — check menu availability + signals + OTP before idempotency.
      // Statically imported (fail-loud): a missing module must break the build, never
      // silently degrade to "clean" — that previously disabled OTP + all risk signals.

      // 4a. Product/modifier availability for preflight
      let productIds = items.map(i => i.product_id);
      let allModifierIdsArr = [...new Set(items.flatMap(i => i.modifier_ids || []))];

      const prodAvailRes = await client.query(
        `SELECT id, is_available FROM products WHERE id = ANY($1::uuid[]) AND location_id = $2`,
        [productIds, locationId]
      );
      const prodAvail = new Map(prodAvailRes.rows.map((r: any) => [r.id, r.is_available]));

      const modAvailMap = new Map<string, boolean | null>();
      if (allModifierIdsArr.length > 0) {
        const modAvailRes = await client.query(
          `SELECT m.id, m.available, pmg.product_id
           FROM modifiers m
           JOIN product_modifier_groups pmg ON pmg.group_id = m.group_id
           WHERE m.id = ANY($1::uuid[]) AND m.location_id = $2`,
          [allModifierIdsArr, locationId]
        );
        for (const row of modAvailRes.rows) {
          modAvailMap.set(`${row.product_id}_${row.id}`, row.available);
        }
      }

      const lines = items.map(item => ({
        productId: item.product_id,
        quantity: item.quantity,
        modifierIds: item.modifier_ids || [],
        productAvailable: prodAvail.get(item.product_id) ?? null,
        modifierAvailability: Object.fromEntries(
          (item.modifier_ids || []).map(mid => [mid, modAvailMap.get(`${item.product_id}_${mid}`) ?? null])
        ) as Record<string, boolean | null>,
      }));

      // 4b. Per-phone order throttle (FX-4) — hard block, not advisory
      const phoneForSignals = cust?.phone || '';
      const phoneHash = phoneForSignals ? crypto.createHash('sha256').update(phoneForSignals.replace(/\D/g, '')).digest('hex') : undefined;
      // #9: key the per-IP order throttle on the REAL client IP (Fly-Client-IP via the shared
      // resolver), NOT request.ip (the Fly edge socket, which collapses every client onto one
      // hash → the throttle would either never fire or punish everyone together).
      const clientIpForThrottle = clientIp(request);
      const clientIpHash = clientIpForThrottle ? crypto.createHash('sha256').update(clientIpForThrottle).digest('hex') : undefined;
      // #8: same as the requestHash above — the customer identity is `sub`, not `userId`.
      const preflightCustomerId = request.user?.role === 'customer' ? request.user.sub : undefined;

      if (phoneHash) {
        const THROTTLE_WINDOW_SECONDS = 900; // 15 minutes
        const THROTTLE_MAX_ORDERS = 5;
        const throttleRes = await client.query(
          `SELECT COUNT(*)::int AS cnt FROM velocity_events
           WHERE location_id = $1 AND phone_hash = $2
             AND kind = 'order_placed'
             AND window_started_at > now() - ($3 || ' seconds')::interval`,
          [locationId, phoneHash, String(THROTTLE_WINDOW_SECONDS)],
        );
        const recentOrderCount = throttleRes.rows[0]?.cnt ?? 0;
        if (recentOrderCount >= THROTTLE_MAX_ORDERS) {
          await client.query('ROLLBACK');
          return reply.status(429).send({
            error: 'Too many orders from this phone number. Please try again later.',
            code: 'PHONE_THROTTLE',
            retryAfterSeconds: THROTTLE_WINDOW_SECONDS,
          });
        }
      }

      // §4 (Breaker M5): the IP half of the phone+IP velocity throttle. clientIpHash was computed
      // but never gated, so a phone-rotating attacker was bounded only per-phone. This caps orders
      // to ONE location from ONE IP over the window — generously (legit shared-NAT rarely nears it)
      // but enough to bound a phone-rotating flood. Same velocity_events table + ip index.
      if (clientIpHash) {
        const IP_THROTTLE_WINDOW_SECONDS = 900; // 15 minutes
        const IP_THROTTLE_MAX_ORDERS = 20;       // per (location, IP) — well above any real shared NAT
        const ipThrottleRes = await client.query(
          `SELECT COUNT(*)::int AS cnt FROM velocity_events
           WHERE location_id = $1 AND client_ip_hash = $2
             AND kind = 'order_placed'
             AND window_started_at > now() - ($3 || ' seconds')::interval`,
          [locationId, clientIpHash, String(IP_THROTTLE_WINDOW_SECONDS)],
        );
        const recentIpOrders = ipThrottleRes.rows[0]?.cnt ?? 0;
        if (recentIpOrders >= IP_THROTTLE_MAX_ORDERS) {
          await client.query('ROLLBACK');
          return reply.status(429).send({
            error: 'Too many orders from this network. Please try again later.',
            code: 'IP_THROTTLE',
            retryAfterSeconds: IP_THROTTLE_WINDOW_SECONDS,
          });
        }
      }

      const signals = await computeSignals(db, {
        locationId,
        phoneHash,
        clientIpHash,
        customerId: preflightCustomerId,
      });

      // 4c. Server-side OTP verification (authority — does NOT trust client input)
      let otpServerVerified = otpVerified; // from P26 header check
      if (OTP_ENABLED && location.require_phone_otp && input.otp_code && phoneForSignals) {
        try {
          const { verifyOtpCode } = await import('../lib/otp.js');
          const otpRes = await client.query(
            `SELECT id, code_hash, attempts FROM phone_otp
             WHERE location_id = $1 AND phone = $2
               AND consumed_at IS NULL AND expires_at > now()
             ORDER BY created_at DESC LIMIT 1`,
            [locationId, phoneForSignals]
          );
          if ((otpRes.rowCount ?? 0) > 0) {
            const otpRow = otpRes.rows[0];
            if (otpRow.attempts < 5) {
              const valid = await verifyOtpCode(input.otp_code, otpRow.code_hash);
              if (valid) {
                otpServerVerified = true;
                await client.query(`UPDATE phone_otp SET consumed_at = now(), attempts = COALESCE(attempts, 0) + 1 WHERE id = $1`, [otpRow.id]);
              } else {
                await client.query(`UPDATE phone_otp SET attempts = COALESCE(attempts, 0) + 1 WHERE id = $1`, [otpRow.id]);
              }
            }
          }
        } catch (err: any) {
          console.debug('[orders] OTP server check failed, proceeding without:', err?.message);
        }
      }

      // 4d. Build signal state for preflight — pure (lib/order-canonical.ts).
      const sigState = buildSignalState({
        signals,
        otpRequired: OTP_ENABLED && location.require_phone_otp,
        otpVerified: otpServerVerified,
      });

      // 4e. Evaluate preflight
      const preflight = evaluatePreflight({
        lines,
        signals: sigState,
        acknowledgedCodes: input.acknowledged_codes,
      });

      // 4f. Handle non-clean outcomes
      if (preflight.outcome === 'hard_block') {
        await client.query('ROLLBACK');
        return reply.status(422).send({ outcome: 'hard_block', reasons: preflight.reasons });
      }

      if (preflight.outcome === 'soft_confirm') {
        await client.query('ROLLBACK');
        return reply.status(200).send({
          outcome: 'soft_confirm',
          reasons: preflight.reasons,
          requiresOtp: preflight.requiresOtp,
          requiresConfirmation: true,
        });
      }

      // 4g. CLEAN — proceed to idempotency + creation
      // Store preflight result for owner visibility
      const preflightMeta = JSON.stringify({
        outcome: 'clean',
        reasons: preflight.reasons,
        confirmedReasons: preflight.confirmedReasons || [],
        computedAt: new Date().toISOString(),
      });

      // 5. Idempotency Check — tenant-scoped (FX-5)
      const existingKey = await client.query(
        `SELECT order_id, request_hash FROM idempotency_keys WHERE key = $1 AND location_id = $2`,
        [idempotency_key, locationId]
      );

      if (existingKey.rowCount && existingKey.rowCount > 0) {
        const row = existingKey.rows[0];
        if (row.request_hash !== requestHash) {
          await client.query('ROLLBACK');
          return reply.sendError(422, 'IDEMPOTENCY_KEY_REUSED', 'Idempotency key reused with different request');
        }
        const existingOrder = await client.query(`SELECT id, status, subtotal, total, created_at::text, timeout_at::text FROM orders WHERE id = $1`, [row.order_id]);
        if (existingOrder.rowCount && existingOrder.rowCount > 0) {
          await client.query('COMMIT');
          return reply.status(200).send(existingOrder.rows[0]);
        }
        await client.query(`DELETE FROM idempotency_keys WHERE key = $1 AND location_id = $2`, [idempotency_key, locationId]);
      }

      // 6. Verify Products. Price authority is the in-transaction MVCC snapshot
      // (NOT a row lock — there is no FOR UPDATE here): the price read and the
      // order INSERT share one snapshot, so the snapshotted price is coherent.
      // is_available is re-checked on the same snapshot below.
      productIds = items.map(i => i.product_id);
      const productsRes = await client.query(
        `SELECT id, name, price, is_available
         FROM products WHERE id = ANY($1::uuid[]) AND location_id = $2`,
        [productIds, locationId]
      );

      const productMap = new Map<string, any>();
      for (const row of productsRes.rows) {
        if (!row.is_available) {
          await client.query('ROLLBACK');
          return reply.sendError(422, 'PRODUCT_UNAVAILABLE', `Product ${row.name} is unavailable`);
        }
        productMap.set(row.id, row);
      }

      for (const item of items) {
        if (!productMap.has(item.product_id)) {
          await client.query('ROLLBACK');
          return reply.sendError(422, 'PRODUCT_NOT_FOUND', `Product ${item.product_id} not found`);
        }
      }

      // 6. Verify Modifiers
      const allModifierIds = new Set<string>();
      for (const item of items) {
        for (const mid of (item.modifier_ids || [])) {
          allModifierIds.add(mid);
        }
      }

      const modMap = new Map<string, any>();
      if (allModifierIds.size > 0) {
        const modRes = await client.query(
          `SELECT m.id, m.name, m.price_delta, m.available, pmg.product_id, pmg.group_id
           FROM modifiers m
           JOIN product_modifier_groups pmg ON pmg.group_id = m.group_id
           WHERE m.id = ANY($1::uuid[]) AND m.location_id = $2 AND m.available = true`,
          [Array.from(allModifierIds), locationId]
        );
        for (const row of modRes.rows) {
          modMap.set(`${row.product_id}_${row.id}`, row);
        }
      }

      // 6b. Batch-fetch modifier groups for ALL products in one set-based query
      // (was one round-trip per line-item — the only N-fan-out left inside the held
      // transaction). Same in-tx MVCC snapshot, so the price/availability coherence
      // is unchanged; this only cuts round-trips, flattening the connection-hold so
      // it no longer scales with cart size. Partitioned by product_id into a Map
      // that every line-item RE-READS (broadcast, never consumed-once) — so two
      // line-items of the same product are each validated against the full group set.
      const groupsByProduct = new Map<string, any[]>();
      {
        const groupRes = await client.query(
          `SELECT pmg.product_id, mg.id, mg.min_select, mg.max_select, mg.required
           FROM modifier_groups mg
           JOIN product_modifier_groups pmg ON pmg.group_id = mg.id
           WHERE pmg.product_id = ANY($1::uuid[])`,
          [productIds]
        );
        for (const row of groupRes.rows) {
          const list = groupsByProduct.get(row.product_id);
          if (list) list.push(row);
          else groupsByProduct.set(row.product_id, [row]);
        }
      }

      // 7. Calculate Pricing & Validate Modifier Groups.
      // Pure core (lib/order-pricing.ts) — consumes the in-tx MVCC snapshot Maps
      // fetched above, returns priced rows or the first validation failure. The
      // 422 codes/messages are unchanged; only ROLLBACK + reply stay in the handler.
      const pricing = computeOrderPricing({ items, productMap, modMap, groupsByProduct });
      if (!pricing.ok) {
        await client.query('ROLLBACK');
        return reply.sendError(422, pricing.error.code, pricing.error.message);
      }
      const { subtotal, orderItemRows } = pricing;

      // 8. Delivery Rules
      if (location.min_order_value !== null && subtotal < location.min_order_value) {
        await client.query('ROLLBACK');
        return reply.status(422).send({ 
          error: 'Minimum order value not met', 
          code: 'MIN_ORDER_NOT_MET',
          details: { min_order_value: location.min_order_value, subtotal }
        });
      }

      let deliveryFee = 0;
      if (!isPickup) {
      if (location.free_delivery_threshold !== null && subtotal >= location.free_delivery_threshold) {
        deliveryFee = 0;
      } else {
        const distRes = await client.query(
          `SELECT max_distance_km, fee FROM delivery_tiers WHERE location_id = $1 ORDER BY max_distance_km ASC`,
          [locationId]
        );
        // Pure tier resolution (lib/order-pricing.ts) — same NOT_DELIVERABLE /
        // DELIVERY_NOT_CONFIGURED 422 codes; ROLLBACK + reply stay here.
        const feeResult = resolveDeliveryFee({ location, pin, tiers: distRes.rows });
        if (!feeResult.ok) {
          await client.query('ROLLBACK');
          return reply.sendError(422, feeResult.error.code, feeResult.error.message);
        }
        deliveryFee = feeResult.deliveryFee;
      }
      } // end if (!isPickup) — pickup orders pay no delivery fee

      // 9. Taxes and Total
      // taxTotal is the VAT figure for display/records. When price_includes_tax, the tax is
      // ALREADY inside subtotal (applyTax EXTRACTS it), so adding it to total double-charges the
      // customer — the inclusive branch contributes 0 to the charge (ADR-audit-fix-money D1 / LC1).
      const taxTotal = applyTax(subtotal, Number(location.tax_rate), location.price_includes_tax, location.currency_minor_unit);
      const chargedTax = location.price_includes_tax ? 0 : taxTotal;
      const discountTotal = 0;
      const total = subtotal + deliveryFee + chargedTax - discountTotal;
      assertNonNegative(total);

      if (cashPayWith !== undefined && cashPayWith < total) {
        await client.query('ROLLBACK');
        return reply.sendError(422, 'CASH_AMOUNT_TOO_LOW', `Cash amount must be at least ${total}`);
      }

      // 10. Upsert Customer (if requested, otherwise anonymous logic)
      let resolvedCustomerId = null;
      if (cust && cust.phone) {
        const custRes = await client.query(
          `INSERT INTO customers (location_id, phone, name, messenger_kind, messenger_handle)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (location_id, phone) DO UPDATE SET
             name = COALESCE(EXCLUDED.name, customers.name),
             messenger_kind = COALESCE(EXCLUDED.messenger_kind, customers.messenger_kind),
             messenger_handle = COALESCE(EXCLUDED.messenger_handle, customers.messenger_handle)
           RETURNING id`,
          [locationId, cust.phone, cust.name || null, (cust as any).messenger_kind || null, (cust as any).messenger_handle || null]
        );
        resolvedCustomerId = custRes.rows[0].id;
      } else if (request.user?.role === 'customer') {
        // Find existing customer by phone if from token
        // Left unimplemented for brevity or fallback to anonymous if no customer record matched
      }

      const timeoutMinutes = location.busy_mode ? location.confirm_timeout_min * 2 : location.confirm_timeout_min;
      const timeoutAt = new Date(Date.now() + timeoutMinutes * 60 * 1000);

      // 11-14. Persist order + dependent rows + transactional enqueues
      // (lib/order-persistence.ts). Transaction control stays here.
      const { order, trackCode } = await insertOrderWithItems(client, queue, {
        locationId,
        resolvedCustomerId,
        deliveryAddressText,
        pin,
        subtotal,
        deliveryFee,
        taxTotal,
        discountTotal,
        total,
        cashPayWith,
        currencyCode: location.currency_code,
        menuVersion,
        clientMenuVersion: (input as any).client_menu_version ?? null,
        requestHash,
        timeoutAt,
        rawInstructions,
        otpServerVerified,
        clientIpHash,
        channel,
        preflightMeta,
        type: input.type,
        messengerKind: (cust as any)?.messenger_kind ?? null,
        messengerHandle: (cust as any)?.messenger_handle ?? null,
        // "Deliver to someone else" — receiver's own contact (null when delivering to the customer).
        // Pass-through (DB CHECK is the kind gate, like customer messenger_kind); only when fully present.
        receiverName: ((input as any).receiver?.name || '').trim() || null,
        receiverMessengerKind: (input as any).receiver?.messenger_kind ?? null,
        receiverHandle: ((input as any).receiver?.handle || '').trim() || null,
        deliveryPhotoKey: (input as any).delivery_photo_key ?? null,
        tipAmount: (input as any).tip_amount,
        orderItemRows,
        idempotencyKey: idempotency_key,
        phoneHash,
        custPhone: cust?.phone,
      });

      await client.query('COMMIT');

      // Post-commit MessageBus
      try {
        console.log('[ORDERS] Publishing order.created event for order:', order.id);
        await messageBus.publish(BUS_CHANNELS.ORDER_CREATED, {
          orderId: order.id,
          locationId,
          status: 'PENDING',
          total: total,
          currency: location.currency_code,
          timestamp: new Date().toISOString(),
        });
        console.log('[ORDERS] order.created event published successfully');
        await messageBus.publish(orderChannel(order.id), {
          type: 'order.status',
          orderId: order.id,
          status: 'PENDING',
          locationId,
          timestamp: new Date().toISOString(),
        });
        const shortId = '#' + order.id.substring(0, 4).toUpperCase();
        // P0-3 claim-check: the bus carries ZERO customer PII. Item names
        // (dietary/medical-adjacent), customer name and phone are NOT published \u2014
        // the dashboard pulls them from the authenticated, RLS-scoped /owner/orders
        // endpoint. Only non-PII status fields ride the bus (Upstash out of the PII
        // perimeter). location_id is included so subscribers can scope.
        await messageBus.publish(dashboardChannel(locationId), {
          type: 'order.created',
          data: {
            orderId: order.id,
            locationId,
            status: 'PENDING',
            total,
            currency: location.currency_code,
            createdAt: order.created_at,
            shortId,
            itemCount: items.length,
          },
        });
      } catch (err) {
        request.log.warn('Failed to publish order event');
      }

      let authToken: string | undefined;
      if (cust?.phone && resolvedCustomerId) {
        try {
          authToken = await issueCustomerToken({
            orderId: order.id,
            locationId,
            customerId: resolvedCustomerId,
          });
        } catch (err) {
          request.log.error({ err }, 'Failed to issue customer token');
        }
      }

      // Crypto prepaid fork (ADR-0017) — DARK behind PAYMENTS_PREPAID_ENABLED + PAYMENTS_CRYPTO_ENABLED. The
      // order is committed PENDING/unpaid above; here we create the money-SoT payment row + the hosted charge,
      // mark the order prepaid-pending (held: not offered to fulfillment until the webhook flips payment_status
      // to 'paid'), and hand back the Plisio invoice URL. Failures are swallowed — the order stays held and
      // times out / owner-reviews; a charge failure must NOT fail an already-committed order.
      let paymentRedirect: string | undefined;
      if ((input as any).payment?.method === 'crypto' && isPrepaidEnabled() && isCryptoEnabled()) {
        try {
          const provider = getPaymentProvider();
          const payRes = await client.query(
            `INSERT INTO payments (location_id, order_id, provider, status, amount_minor, currency_code)
             VALUES ($1, $2, $3, 'pending', $4, $5) RETURNING id`,
            [locationId, order.id, provider.name, total, location.currency_code],
          );
          const paymentId = payRes.rows[0].id as string;
          await client.query(`UPDATE orders SET payment_method = 'crypto', payment_status = 'pending' WHERE id = $1`, [order.id]);
          const charge = await provider.createCharge({
            paymentId,
            amountMinor: total,
            currencyCode: location.currency_code,
            minorUnit: location.currency_minor_unit ?? 2,
            idempotencyKey: idempotency_key,
            returnUrl: `${env.APP_BASE_URL}/s/${location.slug}/order/${order.id}`,
            orderName: '#' + order.id.substring(0, 4).toUpperCase(),
          });
          await client.query(`UPDATE payments SET provider_payment_id = $1 WHERE id = $2`, [charge.providerPaymentId, paymentId]);
          paymentRedirect = charge.redirectUrl;
        } catch (err) {
          request.log.error({ err }, 'crypto charge failed — order held unpaid');
        }
      }

      return reply.status(201).send({
        id: order.id,
        locationId,
        status: order.status,
        subtotal,
        total,
        deliveryInstructions: rawInstructions || null,
        createdAt: order.created_at,
        authToken,
        payment: paymentRedirect ? { method: 'crypto', redirectUrl: paymentRedirect } : undefined,
        trackUrl: trackCode
          ? `${env.APP_BASE_URL}/s/${location.slug}/order/${order.id}?t=${trackCode}`
          : undefined,
        preflight: {
          outcome: 'clean',
          reasons: preflight.reasons,
          confirmedReasons: preflight.confirmedReasons || [],
        },
      });

    } catch (err: unknown) {
      await client.query('ROLLBACK').catch(() => {});
      const error = err as Record<string, unknown>;
      request.log.error(error);

      if (error?.code === '23505') { // Unique violation
        return reply.sendError(409, 'IDEMPOTENCY_CONFLICT', 'Idempotency key conflict');
      }
      // Transient DB contention under load (serialization / deadlock / statement-timeout from the
      // bounded write-hold / connection drop) — the order is safe to retry, so surface a graceful
      // 503 the client can show as "try again" instead of a scary 500 (matches the db.connect()
      // 503 guard above and the login-503 hardening).
      const TRANSIENT_PG = new Set(['40001', '40P01', '57014', '53300', '08006', '08003', '08000']);
      if (typeof error?.code === 'string' && TRANSIENT_PG.has(error.code)) {
        return reply.sendError(503, 'SERVICE_UNAVAILABLE', 'Service temporarily unavailable, please try again');
      }
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    } finally {
      if (client) client.release();
    }
  });

  // ─── GET /orders/:id ───────────────────────────────────────────────
  fastify.get('/orders/:id', {
    preHandler: [(fastify as any).softVerifyAuth],
  }, async (request: any, reply: any) => {
    const { id } = request.params as { id: string };
    if (!isValidUUID(id)) {
      return reply.sendError(400, 'VALIDATION_FAILED', 'Invalid order ID format');
    }
    const user = request.user;
    let locationId: string | null = null;

    // P2-ANONORDER: this route uses softVerifyAuth (anonymous-permitting), but an
    // order is private. Require a recognized principal — a customer token (scoped
    // to its own orderId/locationId) or an owner/courier (tenant-scoped below via
    // withTenant). An unauthenticated/unknown caller gets 401; no bare
    // WHERE id=$1 fallback that would let anyone enumerate orders by UUID.
    if (user?.role === 'customer') {
      locationId = user.locationId;
      if (user.orderId !== id) {
        return reply.sendError(404, 'NOT_FOUND', 'Not found');
      }
    } else if (user?.role === 'owner' || user?.role === 'courier') {
      // Owner/courier — tenant isolation via withTenant
    } else {
      // Anonymous or unrecognized role — no access without a scoping credential.
      return reply.sendError(401, 'UNAUTHORIZED', 'Authentication required');
    }

    try {
      let result;
      if (user?.role === 'owner') {
        // #1 (security-hardening-2026-07 / ADR-0004): authorize the read by a LIVE active
        // owner membership, folded INTO the query as a JOIN — the JOIN is the tenant
        // boundary. A bare `WHERE id=$1` leaks cross-tenant under the BYPASSRLS pool
        // (RLS inert), and trusting the baked activeLocationId leaves an insider-removal
        // read window. The JOIN 404s an owner with no live membership at the order's
        // location AND resolves correctly for multi-location owners (any active membership).
        result = await withTenant(db, user.userId, async (client) => {
          const o = await client.query(
            `SELECT o.id, o.location_id, o.customer_id, o.status, o.type, o.delivery_address,
                    o.subtotal, o.total, o.payment_method, o.payment_outcome,
                    o.created_at::text, o.timeout_at::text
             FROM orders o
             JOIN memberships m ON m.location_id = o.location_id
             WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'`,
            [id, user.userId]
          );

          if (!o.rowCount || o.rowCount === 0) return null;

          const items = await client.query(
            `SELECT id, product_id, name_snapshot, price_snapshot, quantity
             FROM order_items WHERE order_id = $1`,
            [id]
          );

          return { ...mapOrderRow(o.rows[0]), items: items.rows.map(mapItemRow) };
        });
      } else if (user?.role === 'courier') {
        // #1 (security-hardening-2026-07 / ADR-0013): authorize by a LIVE courier_assignments
        // binding for THIS order via courierReadVerdict — the same liveness the WS fan-out
        // guard uses. Binding-scoping is strictly narrower than location-scoping, so this
        // closes cross-tenant, the insider-removal window (revoked binding → DENY), AND the
        // within-tenant cross-customer PII read (a courier sees only orders it is bound to).
        // The baked activeLocationId is never the authority — the live verdict is.
        const verdict = await courierReadVerdict(db, user.sub, user.activeLocationId, id);
        if (verdict === 'UNAVAILABLE') {
          // Retryable pool/DB blip — fail CLOSED but distinguishable (never fail-open).
          return reply.sendError(503, 'SERVICE_UNAVAILABLE', 'Service temporarily unavailable, please try again');
        }
        if (verdict !== 'ALLOW') {
          return reply.sendError(404, 'NOT_FOUND', 'Not found');
        }
        // Verdict ALLOW authorizes exactly this order id → the point-read is now scoped.
        const o = await db.query(
          `SELECT id, location_id, customer_id, status, type, delivery_address,
                  subtotal, total, payment_method, payment_outcome,
                  created_at::text, timeout_at::text
           FROM orders WHERE id = $1`,
          [id]
        );
        if (!o.rowCount || o.rowCount === 0) {
          return reply.sendError(404, 'NOT_FOUND', 'Not found');
        }
        const items = await db.query(
          `SELECT id, product_id, name_snapshot, price_snapshot, quantity
           FROM order_items WHERE order_id = $1`,
          [id]
        );
        result = { ...mapOrderRow(o.rows[0]), items: items.rows.map(mapItemRow) };
      } else {
        // Customer or anonymous — session-style read
        const query = locationId
          ? `SELECT id, location_id, customer_id, status, type, delivery_address,
                    subtotal, total, payment_method, payment_outcome,
                    created_at::text, timeout_at::text
             FROM orders WHERE id = $1 AND location_id = $2`
          : `SELECT id, location_id, customer_id, status, type, delivery_address,
                    subtotal, total, payment_method, payment_outcome,
                    created_at::text, timeout_at::text
             FROM orders WHERE id = $1`;

        const params = locationId ? [id, locationId] : [id];
        const o = await db.query(query, params);

        if (!o.rowCount || o.rowCount === 0) {
          return reply.sendError(404, 'NOT_FOUND', 'Not found');
        }

        const items = await db.query(
          `SELECT id, product_id, name_snapshot, price_snapshot, quantity
           FROM order_items WHERE order_id = $1`,
          [id]
        );

        result = { ...mapOrderRow(o.rows[0]), items: items.rows.map(mapItemRow) };
      }

      if (!result) {
        return reply.sendError(404, 'NOT_FOUND', 'Not found');
      }

      return reply.status(200).send(result);
    } catch (err) {
      request.log.error(err);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    }
  });

  // ─── PATCH /orders/:id/status ──────────────────────────────────────
  fastify.patch('/orders/:id/status', {
    preHandler: [(fastify as any).verifyAuth, (fastify as any).requireRole(['owner'])],
  }, async (request: any, reply: any) => {
    const { id } = request.params as { id: string };
    // safeParse → typed 400 (a bad `status` enum is client input, not a 500). A bare .parse()
    // threw a ZodError that the global handler didn't normalize → raw 500. Matches the create route.
    const parsed = StatusUpdateInput.safeParse(request.body);
    if (!parsed.success) {
      return reply.sendError(400, 'VALIDATION_FAILED', parsed.error.issues?.map((i: any) => i.message).join('; ') || 'Invalid status');
    }
    const { status: newStatus } = parsed.data;
    const user = request.user!;
    if (user.role !== 'owner') {
      return reply.sendError(403, 'FORBIDDEN', 'Forbidden');
    }

     // §5 / R2-1 honest dispatch: the resulting status + whether a courier was dispatched, carried out of the
     // tx (we cannot send the reply from inside withTenant). Defaults to the requested transition.
     let outcome: { status: string; dispatched?: boolean; reason?: string } = { status: newStatus };
     try {
       await withTenant(db, user.userId, async (client) => {
         // 1. Read current status — authorized by a LIVE active owner membership folded INTO the
         // query as a JOIN (LC2 / authz council; mirrors the GET sibling above and the dashboard
         // transition site). A bare `WHERE id=$1` leaks cross-tenant under the BYPASSRLS pool
         // (RLS inert) — an owner of location A could drive transitions on another tenant's order
         // by UUID. 0 rows → 404 BEFORE any transition logic; locationId below is taken from the
         // JOIN-verified row, never from client input or the baked token claim.
         const cur = await client.query(
           `SELECT o.id, o.status, o.location_id, o.type
              FROM orders o
              JOIN memberships m ON m.location_id = o.location_id
             WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'`,
           [id, user.userId]
         );

         if (!cur.rowCount || cur.rowCount === 0) {
           // Not found OR no live membership at the order's location — logged as a possible
           // cross-tenant attempt (indistinguishable by design: the 404 must not leak existence).
           request.log.warn(
             { orderId: id, userId: user.userId, targetStatus: newStatus },
             'PATCH /orders/:id/status membership-JOIN miss — order not found or cross-tenant attempt'
           );
           throw { statusCode: 404, error: 'Order not found' };
         }

          const locationId = cur.rows[0].location_id;

          // deliver v2 offer-sweep-cancel addendum: the widened CONFIRMED/PREPARING/READY→CANCELLED
          // edges are SYSTEM-only. An owner may not drive them via this request-supplied newStatus →
          // 403 CANCEL_NOT_PERMITTED. PENDING→CANCELLED and IN_DELIVERY→CANCELLED stay permitted.
          assertOwnerTargetAllowed(cur.rows[0].status, newStatus);

          // M6 / CC-1 (ADR-audit-fix-money §3.5, money-audit H1): DELIVERED/PICKED_UP must never be
          // reachable via PATCH when it would strand a courier binding or fabricate a delivery
          // without completeDelivery's cash-as-proof attestation. Two arms (both 409, was a silent
          // 200 + permanent strand):
          //  (a) an ACTIVE binding exists → ASSIGNMENT_ACTIVE — the owner completes via /deliver
          //      (owner-proxy exists: owner/dashboard.ts POST /deliver);
          //  (b) order is IN_DELIVERY with NO delivered assignment (binding drained by an
          //      offer-expiry/abort race or manual state) → USE_DELIVER_FLOW — otherwise the PATCH
          //      passes with zero attestation and the silent strand survives via the back door.
          // Never-dispatched orders (zero assignments, never IN_DELIVERY — phone/manual flow) stay
          // PATCH-able: nothing to strand, no courier cash in play. The sanctioned completion paths
          // (completeDelivery, owner-proxy /deliver) terminalize the assignment BEFORE calling
          // updateOrderStatus and do not pass through this route — untouched.
          if (newStatus === 'DELIVERED' || newStatus === 'PICKED_UP') {
            const activeBinding = await client.query(
              `SELECT 1 FROM courier_assignments
                WHERE order_id = $1 AND status IN ('offered','assigned','accepted','picked_up') LIMIT 1`,
              [id]
            );
            if (activeBinding.rowCount) {
              throw {
                statusCode: 409,
                error: 'Order has an active courier assignment — complete it via the deliver flow',
                code: 'ASSIGNMENT_ACTIVE',
              };
            }
            if (cur.rows[0].status === 'IN_DELIVERY') {
              const deliveredBinding = await client.query(
                `SELECT 1 FROM courier_assignments WHERE order_id = $1 AND status = 'delivered' LIMIT 1`,
                [id]
              );
              if (!deliveredBinding.rowCount) {
                throw {
                  statusCode: 409,
                  error: 'Order is in delivery without a delivered assignment — complete it via the deliver flow',
                  code: 'USE_DELIVER_FLOW',
                };
              }
            }
          }

          // §5 / R2-1 / R3-2 — HONEST DISPATCH. For an IN_DELIVERY target on a delivery order, find a courier
          // BEFORE advancing the status. The old code flipped to IN_DELIVERY first, then silently no-op'd when
          // no courier was free → an IN_DELIVERY order with NO courier and NO recovery affordance (the F1
          // orphan). Now: no courier → DO NOT advance (stay put), report {dispatched:false,reason:'no_courier'};
          // the owner re-taps when a courier comes on shift. An order already carrying an active binding (incl
          // 'offered' from the owner offer-handshake — in the mig-073 partial-uniques) is already being
          // dispatched → no conflicting insert. This holds regardless of any offer-handshake flag.
          if (newStatus === 'IN_DELIVERY' && cur.rows[0].type === 'delivery') {
            outcome = await attemptHonestDispatch(client, { orderId: id, locationId, currentStatus: cur.rows[0].status }, { messageBus });
            return;
          }

          // All other transitions (and pickup) — unchanged.
          await updateOrderStatus(client, id, locationId, newStatus, { messageBus });
          outcome = { status: newStatus };
        });

        return reply.status(200).send({ id, ...outcome });
     } catch (err: unknown) {
      const error = err as Record<string, unknown>;
      if (error.statusCode) {
        request.log.warn({ orderId: id, targetStatus: newStatus, error: error.error, code: error.code }, 'Status update rejected');
        return reply.status(error.statusCode as number).send({ error: error.error as string, code: error.code as string });
      }
      request.log.error(error);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    }
  });
}

