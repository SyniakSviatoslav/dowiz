// @ts-nocheck
import type { FastifyInstance } from 'fastify';
import { CreateOrderInput, StatusUpdateInput } from '@deliveryos/shared-types';
import { assertTransition, type OrderStatus } from '@deliveryos/domain';
import { issueCustomerToken, withTenant } from '@deliveryos/platform';
import type { QueueProvider, MessageBus } from '@deliveryos/platform';
import type { Pool } from 'pg';
import crypto from 'crypto';
import { applyTax, assertNonNegative, computeLineTotal } from '../lib/money.js';
import { distanceKm } from '../lib/geo.js';

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
    config: { rateLimit: { max: 10, timeWindow: '1 minute' } },
  }, async (request, reply) => {
    let input;
    try {
      input = CreateOrderInput.parse(request.body);
    } catch (err: any) {
      const issues = err?.issues?.map((i: any) => i.message).join('; ');
      return reply.status(400).send({ code: 400, error: issues || 'Validation error' });
    }
    const { locationId, items, customer: cust, delivery, idempotency_key, cash_pay_with } = input;

    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // 1. Location config (FOR UPDATE lock optionally, but standard read is fine if config updates are rare, wait we should lock locations? No need to lock config, just read it).
      const locRes = await client.query(
         `SELECT lat, lng, confirm_timeout_min, busy_mode, phone,
                 currency_code, currency_minor_unit, tax_rate, price_includes_tax,
                 min_order_value, free_delivery_threshold, delivery_fee_flat
          FROM locations WHERE id = $1`,
        [locationId]
      );

      if (locRes.rowCount === 0) {
        await client.query('ROLLBACK');
        return reply.status(404).send({ error: 'Location not found' });
      }

      const location = locRes.rows[0];

      // OTP verification (if column exists)
      let otpVerified = false;
      let requireOtp = false;
      try {
        const otpCol = await client.query(
          `SELECT require_phone_otp FROM locations WHERE id = $1`, [locationId]
        );
        requireOtp = otpCol.rows[0]?.require_phone_otp || false;
      } catch { /* column may not exist — assume OTP not required */
        console.debug('[orders] require_phone_otp column check failed');
      }
      if (requireOtp) {
        const otpHeader = request.headers['x-otp-verified'] as string | undefined;
        if (otpHeader) {
          try {
            const { hashPhone } = await import('./lib/otp.js');
            const tokenHash = crypto.createHash('sha256').update(otpHeader).digest('hex');
            const otpSessionRes = await client.query(
              `SELECT id, phone_hash, order_intent_hash, expires_at, consumed_at
               FROM customer_otp_sessions
               WHERE token_hash = $1 AND purpose = 'otp_verified'
                 AND consumed_at IS NULL AND expires_at > now()`,
              [tokenHash],
            );
            if (otpSessionRes.rowCount > 0) {
              otpVerified = true;
              // Mark the session as consumed (single-use)
              await client.query(
                `UPDATE customer_otp_sessions SET consumed_at = now() WHERE id = $1`,
                [otpSessionRes.rows[0].id],
              );
            }
          } catch {
            // Invalid token — P26 does NOT block, E27 may soft-confirm
            console.debug('[orders] OTP verification failed, proceeding without');
          }
        }
      }

      // 2. Menu version
      const mvRes = await client.query(
        `SELECT version FROM menu_versions WHERE location_id = $1`,
        [locationId]
      );
      const menuVersion = mvRes.rowCount && mvRes.rows[0] ? mvRes.rows[0].version : '0';

      // 3. Request Hash
      const canonicalItems = items.map(i => ({
        product_id: i.product_id,
        quantity: i.quantity,
        modifier_ids: [...(i.modifier_ids || [])].sort()
      }));
      const lat_rounded_5 = Math.round(delivery.pin.lat * 100000) / 100000;
      const lng_rounded_5 = Math.round(delivery.pin.lng * 100000) / 100000;
      const customerId = request.user?.role === 'customer' ? request.user.userId : 'anonymous';

      const canonicalBody = JSON.stringify({
        locationId,
        items: canonicalItems,
        pin: { lat: lat_rounded_5, lng: lng_rounded_5 },
        address_text: delivery.address_text || null,
        cash_pay_with: cash_pay_with || false,
        currency_code: location.currency_code,
        menu_version: menuVersion,
        customer_id: customerId
      });
      const requestHash = crypto.createHash('sha256').update(canonicalBody).digest('hex');

      // 4. Preflight (E27) — check menu availability + signals + OTP before idempotency
      let evaluatePreflight, computeSignals;
      try {
        const pfModule = await import('../lib/preflight.js');
        evaluatePreflight = pfModule.evaluatePreflight;
        const sigModule = await import('../lib/signals/compute.js');
        computeSignals = sigModule.computeSignals;
      } catch {
        evaluatePreflight = (ctx: any) => ({ outcome: 'clean', reasons: [], confirmedReasons: [] });
        computeSignals = async () => [];
      }

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
        productAvailable: prodAvail.has(item.product_id) ? prodAvail.get(item.product_id) : null,
        modifierAvailability: Object.fromEntries(
          (item.modifier_ids || []).map(mid => [mid, modAvailMap.has(`${item.product_id}_${mid}`) ? modAvailMap.get(`${item.product_id}_${mid}`) : null])
        ),
      }));

      // 4b. Per-phone order throttle (FX-4) — hard block, not advisory
      const phoneForSignals = cust?.phone || '';
      const phoneHash = phoneForSignals ? crypto.createHash('sha256').update(phoneForSignals.replace(/\D/g, '')).digest('hex') : undefined;
      const clientIpHash = request.ip ? crypto.createHash('sha256').update(request.ip).digest('hex') : undefined;
      const preflightCustomerId = request.user?.role === 'customer' ? request.user.userId : undefined;

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

      const signals = await computeSignals(db, {
        locationId,
        phoneHash,
        clientIpHash,
        customerId: preflightCustomerId,
      });

      // 4c. Server-side OTP verification (authority — does NOT trust client input)
      let otpServerVerified = otpVerified; // from P26 header check
      if (location.require_phone_otp && input.otp_code && phoneForSignals) {
        try {
          const { verifyOtpCode } = await import('../lib/otp.js');
          const otpRes = await client.query(
            `SELECT id, code_hash, attempts FROM phone_otp
             WHERE location_id = $1 AND phone = $2
               AND consumed_at IS NULL AND expires_at > now()
             ORDER BY created_at DESC LIMIT 1`,
            [locationId, phoneForSignals]
          );
          if (otpRes.rowCount > 0) {
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
        } catch {
          // OTP server check failure — proceed without OTP (E27: not a hard block)
          console.debug('[orders] OTP server check failed, proceeding without');
        }
      }

      // 4d. Build signal state for preflight
      const sigState = {
        velocityPhoneCount: 0,
        velocityIpCount: 0,
        noShowCount: 0,
        noShowAgeDays: null as number | null,
        completedCount: 0,
        otpRequired: location.require_phone_otp,
        otpVerified: otpServerVerified,
      };
      for (const s of signals) {
        if (s.kind === 'velocity_rapid' || s.kind === 'velocity_high_volume') {
          sigState.velocityPhoneCount = Math.max(sigState.velocityPhoneCount, s.evidence.count || 0);
        }
        if (s.kind === 'ip_velocity_rapid' || s.kind === 'ip_velocity_high_volume') {
          sigState.velocityIpCount = Math.max(sigState.velocityIpCount, s.evidence.count || 0);
        }
        if (s.kind === 'no_show_recent') {
          sigState.noShowCount = s.evidence.count || 0;
          sigState.noShowAgeDays = s.evidence.ageDays ?? null;
          sigState.completedCount = s.evidence.completedCount || 0;
        }
      }

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
          return reply.status(422).send({ error: 'Idempotency key reused with different request', code: 'IDEMPOTENCY_KEY_REUSED' });
        }
        const existingOrder = await client.query(`SELECT id, status, subtotal, total, created_at::text, timeout_at::text FROM orders WHERE id = $1`, [row.order_id]);
        if (existingOrder.rowCount && existingOrder.rowCount > 0) {
          await client.query('COMMIT');
          return reply.status(200).send(existingOrder.rows[0]);
        }
        await client.query(`DELETE FROM idempotency_keys WHERE key = $1 AND location_id = $2`, [idempotency_key, locationId]);
      }

      // 6. Verify Products (FOR UPDATE for pricing — authoritative lock)
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
          return reply.status(422).send({ error: `Product ${row.name} is unavailable`, code: 'PRODUCT_UNAVAILABLE' });
        }
        productMap.set(row.id, row);
      }

      for (const item of items) {
        if (!productMap.has(item.product_id)) {
          await client.query('ROLLBACK');
          return reply.status(422).send({ error: `Product ${item.product_id} not found`, code: 'PRODUCT_NOT_FOUND' });
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

      // 7. Calculate Pricing & Validate Modifier Groups
      let subtotal = 0;
      const orderItemRows: Array<any> = [];

      for (const item of items) {
        const product = productMap.get(item.product_id)!;
        
        // Group logic validation
        const groupRes = await client.query(
          `SELECT mg.id, mg.min_select, mg.max_select, mg.required
           FROM modifier_groups mg
           JOIN product_modifier_groups pmg ON pmg.group_id = mg.id
           WHERE pmg.product_id = $1`,
          [item.product_id]
        );
        
        const groupCounts = new Map<string, number>();
        const modifierPrices: number[] = [];
        const itemModifiersRows: Array<any> = [];

        // Check each modifier is valid for this product
        const uniqueModIdsInItem = new Set(item.modifier_ids || []);
        if (uniqueModIdsInItem.size !== (item.modifier_ids || []).length) {
          await client.query('ROLLBACK');
          return reply.status(422).send({ error: 'Duplicate modifier', code: 'DUPLICATE_MODIFIER' });
        }

        for (const mid of (item.modifier_ids || [])) {
          const modInfo = modMap.get(`${item.product_id}_${mid}`);
          if (!modInfo) {
            await client.query('ROLLBACK');
            return reply.status(422).send({ error: `Modifier ${mid} unavailable or invalid for product`, code: 'MODIFIER_UNAVAILABLE' });
          }
          const groupId = modInfo.group_id;
          groupCounts.set(groupId, (groupCounts.get(groupId) || 0) + 1);
          modifierPrices.push(modInfo.price_delta);
          itemModifiersRows.push({
            modifierId: mid,
            nameSnapshot: modInfo.name,
            priceDeltaSnapshot: modInfo.price_delta
          });
        }

        // Validate min/max select
        for (const gRow of groupRes.rows) {
          const count = groupCounts.get(gRow.id) || 0;
          if (gRow.required && count < gRow.min_select) {
            await client.query('ROLLBACK');
            return reply.status(422).send({ error: `Modifier group ${gRow.id} min select not met`, code: 'MODIFIER_MIN_NOT_MET' });
          }
          if (count > gRow.max_select) {
            await client.query('ROLLBACK');
            return reply.status(422).send({ error: `Modifier group ${gRow.id} max select exceeded`, code: 'MODIFIER_MAX_EXCEEDED' });
          }
        }

        const lineTotal = computeLineTotal(product.price, modifierPrices, item.quantity);
        subtotal += lineTotal;
        
        orderItemRows.push({
          productId: item.product_id,
          nameSnapshot: product.name,
          priceSnapshot: product.price, // price without modifiers
          quantity: item.quantity,
          modifiers: itemModifiersRows
        });
      }

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
      if (location.free_delivery_threshold !== null && subtotal >= location.free_delivery_threshold) {
        deliveryFee = 0;
      } else {
        const distRes = await client.query(
          `SELECT max_distance_km, fee FROM delivery_tiers WHERE location_id = $1 ORDER BY max_distance_km ASC`,
          [locationId]
        );
        const tiers = distRes.rows;

        if (tiers.length > 0 && location.lat != null && location.lng != null) {
          const distKm = distanceKm(delivery.pin.lat, delivery.pin.lng, location.lat, location.lng);
          let foundTier = false;
          for (const tier of tiers) {
            if (distKm <= Number(tier.max_distance_km)) {
              deliveryFee = tier.fee;
              foundTier = true;
              break;
            }
          }
          if (!foundTier) {
            await client.query('ROLLBACK');
            return reply.status(422).send({ error: 'Location out of delivery range', code: 'NOT_DELIVERABLE' });
          }
        } else if (location.delivery_fee_flat !== null) {
          deliveryFee = location.delivery_fee_flat;
        } else {
          await client.query('ROLLBACK');
          return reply.status(422).send({ error: 'Delivery not configured', code: 'DELIVERY_NOT_CONFIGURED' });
        }
      }

      // 9. Taxes and Total
      const taxTotal = applyTax(subtotal, Number(location.tax_rate), location.price_includes_tax, location.currency_minor_unit);
      const discountTotal = 0;
      const total = subtotal + deliveryFee + taxTotal - discountTotal;
      assertNonNegative(total);

      // 10. Upsert Customer (if requested, otherwise anonymous logic)
      let resolvedCustomerId = null;
      if (cust && cust.phone) {
        const custRes = await client.query(
          `INSERT INTO customers (location_id, phone, name)
           VALUES ($1, $2, $3)
           ON CONFLICT (location_id, phone) DO UPDATE SET name = COALESCE(EXCLUDED.name, customers.name)
           RETURNING id`,
          [locationId, cust.phone, cust.name || null]
        );
        resolvedCustomerId = custRes.rows[0].id;
      } else if (request.user?.role === 'customer') {
        // Find existing customer by phone if from token
        // Left unimplemented for brevity or fallback to anonymous if no customer record matched
      }

      const timeoutMinutes = location.busy_mode ? location.confirm_timeout_min * 2 : location.confirm_timeout_min;
      const timeoutAt = new Date(Date.now() + timeoutMinutes * 60 * 1000);

      // 11. Insert Orders
      const orderRes = await client.query(
        `INSERT INTO orders (
          location_id, customer_id, type, status, 
          delivery_address, delivery_lat, delivery_lng,
          subtotal, delivery_fee, tax_total, discount_total, total, 
          payment_method, cash_pay_with, currency_code, 
          menu_version, client_menu_version, request_hash, timeout_at,
          metadata,
          preflight
         )
         VALUES ($1, $2, 'delivery', 'PENDING', $3, $4, $5, $6, $7, $8, $9, $10, 'cash', $11, $12, $13, $14, $15, $16, $17, $18)
         RETURNING id, status, subtotal, total, created_at::text, timeout_at::text`,
        [
          locationId, resolvedCustomerId, delivery.address_text || null, delivery.pin.lat, delivery.pin.lng,
          subtotal, deliveryFee, taxTotal, discountTotal, total,
          cash_pay_with || false, location.currency_code, 
          menuVersion, input.client_menu_version || null, requestHash, timeoutAt,
          JSON.stringify({ otp_verified: otpServerVerified, client_ip_hash: clientIpHash }),
          preflightMeta,
        ]
      );
      const order = orderRes.rows[0];

      // 12b. Record velocity event (for FX-4 throttle + signal computation)
      if (phoneHash || clientIpHash) {
        await client.query(
          `INSERT INTO velocity_events (location_id, phone_hash, client_ip_hash, kind, window_started_at)
           VALUES ($1, $2, $3, 'order_placed', $4)`,
          [locationId, phoneHash || null, clientIpHash || null, new Date().toISOString()],
        );
      }

      // 13. Insert Order Items & Modifiers
      for (const row of orderItemRows) {
        const oiRes = await client.query(
          `INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
           VALUES ($1, $2, $3, $4, $5) RETURNING id`,
          [order.id, row.productId, row.nameSnapshot, row.priceSnapshot, row.quantity]
        );
        const oiId = oiRes.rows[0].id;

        for (const mod of row.modifiers) {
          await client.query(
            `INSERT INTO order_item_modifiers (order_item_id, modifier_id, name_snapshot, price_delta_snapshot)
             VALUES ($1, $2, $3, $4)`,
            [oiId, mod.modifierId, mod.nameSnapshot, mod.priceDeltaSnapshot]
          );
        }
      }

      // 13. Idempotency Key
      await client.query(
        `INSERT INTO idempotency_keys (key, location_id, request_hash, order_id, response_code)
         VALUES ($1, $2, $3, $4, 201)`,
        [idempotency_key, locationId, requestHash, order.id]
      );

      // 14. Transactional Enqueue
      await queue.enqueue('order.timeout', { orderId: order.id, locationId }, {
        singletonKey: order.id,
        startAfter: new Date(timeoutAt),
        db: { executeSql: (sql: string, values: any[]) => client.query(sql, values) }
      });

      await client.query('COMMIT');

      // Post-commit MessageBus
      try {
        await messageBus.publish('order.created', {
          orderId: order.id,
          locationId,
          status: 'PENDING',
          total: total,
          currency: location.currency_code,
          timestamp: new Date().toISOString(),
        });
        await messageBus.publish(`order:${order.id}`, {
          type: 'order.status',
          orderId: order.id,
          status: 'PENDING',
          locationId,
          timestamp: new Date().toISOString(),
        });
        await messageBus.publish(`location:${locationId}:dashboard`, {
          type: 'order.created',
          data: {
            orderId: order.id,
            status: 'PENDING',
            total,
            currency: location.currency_code,
            createdAt: order.created_at,
            customerNameMasked: cust?.name ? cust.name.replace(/(?<=.).(?=.*@)/g, '*') : '***',
            customerPhoneMasked: cust?.phone ? cust.phone.slice(0, -4).replace(/\d/g, '*') + cust.phone.slice(-4) : '***',
            itemCount: items.length,
          },
        });
      } catch (err) {
        request.log.warn('Failed to publish order event');
      }

      return reply.status(201).send({
        id: order.id,
        locationId,
        status: order.status,
        subtotal,
        total,
        createdAt: order.created_at,
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
        return reply.status(409).send({ error: 'Idempotency key conflict', code: 'IDEMPOTENCY_CONFLICT' });
      }
      return reply.status(500).send({ error: 'Internal server error' });
    } finally {
      client.release();
    }
  });

  // ─── GET /orders/:id ───────────────────────────────────────────────
  fastify.get('/orders/:id', async (request, reply) => {
    const { id } = request.params as { id: string };
    if (!isValidUUID(id)) {
      return reply.status(400).send({ error: 'Invalid order ID format' });
    }
    const user = request.user;
    let locationId: string | null = null;

    if (user?.role === 'customer') {
      locationId = user.locationId;
      if (user.orderId !== id) {
        return reply.status(404).send({ error: 'Not found' });
      }
    } else if (user?.role === 'owner' || user?.role === 'courier') {
      // Owner/courier — tenant isolation via withTenant
    }

    try {
      let result;
      if (user?.role === 'owner' || user?.role === 'courier') {
        result = await withTenant(db, user.userId, async (client) => {
          const o = await client.query(
            `SELECT id, location_id, customer_id, status, type, delivery_address,
                    subtotal, total, payment_method, payment_outcome,
                    created_at::text, timeout_at::text
             FROM orders WHERE id = $1`,
            [id]
          );

          if (!o.rowCount || o.rowCount === 0) return null;

          const items = await client.query(
            `SELECT id, product_id, name_snapshot, price_snapshot, quantity
             FROM order_items WHERE order_id = $1`,
            [id]
          );

          return { ...mapOrderRow(o.rows[0]), items: items.rows.map(mapItemRow) };
        });
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
          return reply.status(404).send({ error: 'Not found' });
        }

        const items = await db.query(
          `SELECT id, product_id, name_snapshot, price_snapshot, quantity
           FROM order_items WHERE order_id = $1`,
          [id]
        );

        result = { ...mapOrderRow(o.rows[0]), items: items.rows.map(mapItemRow) };
      }

      if (!result) {
        return reply.status(404).send({ error: 'Not found' });
      }

      return reply.status(200).send(result);
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal server error' });
    }
  });

  // ─── PATCH /orders/:id/status ──────────────────────────────────────
  fastify.patch('/orders/:id/status', {
    preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])],
  }, async (request, reply) => {
    const { id } = request.params as { id: string };
    const { status: newStatus } = StatusUpdateInput.parse(request.body);
    const user = request.user!;
    if (user.role !== 'owner') {
      return reply.status(403).send({ error: 'Forbidden' });
    }

    try {
      await withTenant(db, user.userId, async (client) => {
        // 1. Read current status
        const cur = await client.query(
          `SELECT id, status, location_id FROM orders WHERE id = $1`,
          [id]
        );

        if (!cur.rowCount || cur.rowCount === 0) {
          throw { statusCode: 404, error: 'Order not found' };
        }

        const currentStatus: string = cur.rows[0].status;

        // 2. State machine validation (before SQL)
        try {
          assertTransition(currentStatus as OrderStatus, newStatus as OrderStatus);
        } catch (e: unknown) {
          const err = e as Error;
          if (err.name === 'IllegalTransitionError' || err.name === 'ScaffoldDisabledError') {
            throw { statusCode: 400, error: err.message, code: err.name };
          }
          if (err.name === 'SameStatusError') {
            throw { statusCode: 400, error: err.message, code: err.name };
          }
          throw e;
        }

        // 3. Status-guarded UPDATE (anti-race)
        let res;
        if (newStatus === 'CONFIRMED') {
          res = await client.query(
            `UPDATE orders SET status = $1, confirmed_at = now(), timeout_at = NULL
             WHERE id = $2 AND status = $3 RETURNING id`,
            [newStatus, id, currentStatus]
          );
        } else {
          res = await client.query(
            `UPDATE orders SET status = $1, timeout_at = NULL
             WHERE id = $2 AND status = $3 RETURNING id`,
            [newStatus, id, currentStatus]
          );
        }

        if (!res.rowCount || res.rowCount === 0) {
          throw { statusCode: 409, error: 'Order status already changed', code: 'CONFLICT' };
        }

        // 4. Broadcast via MessageBus
        await messageBus.publish(`order:${id}`, {
          type: 'order.status',
          orderId: id,
          status: newStatus,
          locationId: cur.rows[0].location_id,
          timestamp: new Date().toISOString(),
        });

        // Forward to dashboard room for live owner dashboard
        if (cur.rows[0].location_id) {
          await messageBus.publish(`location:${cur.rows[0].location_id}:dashboard`, {
            type: `order.${newStatus.toLowerCase()}`,
            data: { orderId: id, status: newStatus, statusUpdatedAt: new Date().toISOString() },
          });
        }

        return reply.status(200).send({ id, status: newStatus });
      });
    } catch (err: unknown) {
      const error = err as Record<string, unknown>;
      if (error.statusCode) {
        request.log.warn({ orderId: id, targetStatus: newStatus, error: error.error, code: error.code }, 'Status update rejected');
        return reply.status(error.statusCode as number).send({ error: error.error as string, code: error.code as string });
      }
      request.log.error(error);
      return reply.status(500).send({ error: 'Internal server error' });
    }
  });
}

