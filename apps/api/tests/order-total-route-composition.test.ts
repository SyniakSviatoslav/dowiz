import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import { ORDER_TOTAL_VECTORS } from './vectors/order-total-vectors.js';

// P3b (ADR-audit-fix-money §2.5.3 / breaker M5) — ROUTE-LEVEL server composition matrix.
// The server's total composition is INLINE in POST /orders (orders.ts §9), so unit tests of
// applyTax/estimateOrderTotal cannot prove it: this drives the real route handler end-to-end
// (DB stubbed per orders-guards.test.ts) and asserts the RESPONSE total against the hand-derived
// literal constants in tests/vectors/order-total-vectors.ts (zero-import vector file).
// Covers inclusive/exclusive × zero/boundary/round rates — ≥4 vectors, per the proof matrix.
// RED on pre-fix code: the old composition `subtotal + fee + taxTotal` returns 1650/1150 for the
// inclusive vectors where the constants pin 1450/1075.

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'test-priv',
    JWT_PUBLIC_KEY: 'test-pub',
    JWT_KID: 'test',
    GOOGLE_CLIENT_ID: 'test',
    GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test',
    VAPID_PRIVATE_KEY: 'test',
    IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}

interface VenueCfg {
  taxRate: number;
  priceIncludesTax: boolean;
  deliveryFeeFlat: number;
  productPrice: number;
}

// Scripted DB stub: every query the POST /orders path issues, answered by SQL-shape. The location
// row carries the vector's tax config; the single product's price IS the vector subtotal (qty 1).
function scriptedQuery(cfg: VenueCfg, productId: string) {
  return async (sql: string) => {
    const s = String(sql);
    if (/^(BEGIN|COMMIT|ROLLBACK)/i.test(s) || /^SET LOCAL/i.test(s) || /SAVEPOINT/i.test(s)) return { rows: [], rowCount: 0 };
    if (/FROM locations WHERE id/i.test(s) && /tax_rate/i.test(s)) {
      return {
        rowCount: 1,
        rows: [{
          lat: 41.3275, lng: 19.8187, confirm_timeout_min: 10, busy_mode: false, phone: null,
          slug: 'p3b', published_at: '2026-01-01T00:00:00Z',
          currency_code: 'ALL', currency_minor_unit: 0,
          tax_rate: cfg.taxRate, price_includes_tax: cfg.priceIncludesTax,
          min_order_value: null, free_delivery_threshold: null,
          delivery_fee_flat: cfg.deliveryFeeFlat, require_phone_otp: false,
        }],
      };
    }
    if (/SELECT require_phone_otp FROM locations/i.test(s)) return { rowCount: 1, rows: [{ require_phone_otp: false }] };
    if (/FROM menu_versions/i.test(s)) return { rowCount: 0, rows: [] };
    if (/SELECT id, is_available FROM products/i.test(s)) return { rowCount: 1, rows: [{ id: productId, is_available: true }] };
    if (/SELECT id, name, price, is_available/i.test(s)) {
      return { rowCount: 1, rows: [{ id: productId, name: 'Vector item', price: cfg.productPrice, is_available: true }] };
    }
    if (/FROM velocity_events/i.test(s)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    if (/FROM idempotency_keys/i.test(s)) return { rowCount: 0, rows: [] };
    if (/FROM modifier_groups/i.test(s) || /FROM modifiers/i.test(s)) return { rowCount: 0, rows: [] };
    if (/FROM delivery_tiers/i.test(s)) return { rowCount: 0, rows: [] }; // flat-fee venue → resolveDeliveryFee falls to delivery_fee_flat
    if (/FROM customers|FROM customer_signals/i.test(s)) return { rowCount: 0, rows: [] };
    if (/INSERT INTO customers/i.test(s)) return { rowCount: 1, rows: [{ id: crypto.randomUUID() }] };
    if (/INSERT INTO orders/i.test(s)) {
      return {
        rowCount: 1,
        rows: [{ id: crypto.randomUUID(), status: 'PENDING', subtotal: cfg.productPrice, total: 0, created_at: new Date().toISOString(), timeout_at: null }],
      };
    }
    if (/INSERT INTO order_items/i.test(s)) return { rowCount: 1, rows: [{ id: crypto.randomUUID() }] };
    if (/^INSERT INTO/i.test(s)) return { rowCount: 1, rows: [] }; // velocity_events / idempotency_keys / track grants
    return { rowCount: 0, rows: [] }; // computeSignals & friends — empty is a clean signal state
  };
}

async function buildApp(cfg: VenueCfg, productId: string) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async () => {});
  fastify.decorate('softVerifyAuth', async () => {});
  fastify.decorate('requireRole', () => async () => {});
  const q = scriptedQuery(cfg, productId);
  const client = { query: (sql: string) => q(sql), release() {} };
  const db = { connect: async () => client, query: (sql: string) => q(sql) } as any;
  await fastify.register(orderRoutes, {
    prefix: '/api',
    db,
    messageBus: { publish: async () => {} } as any,
    queue: { enqueue: async () => 'job', work: async () => {}, start: async () => {}, stop: async () => {} } as any,
  });
  return fastify;
}

test('P3b: POST /orders response total matches hand-derived constants (inclusive/exclusive × zero/boundary rates)', async (t) => {
  for (const v of ORDER_TOTAL_VECTORS) {
    await t.test(v.name, async () => {
      const productId = crypto.randomUUID();
      const fastify = await buildApp({
        taxRate: v.taxRate,
        priceIncludesTax: v.priceIncludesTax,
        deliveryFeeFlat: v.deliveryFeeFlat,
        productPrice: v.subtotal, // one line item, qty 1 → pricing subtotal === vector subtotal
      }, productId);
      const isPickup = v.deliveryFeeFlat === 0;
      const payload: Record<string, unknown> = {
        locationId: crypto.randomUUID(),
        type: isPickup ? 'pickup' : 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
        ...(isPickup ? {} : { delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga Test 1' } }),
      };
      const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload });
      assert.equal(res.statusCode, 201, `expected 201, got ${res.statusCode}: ${res.body}`);
      const body = res.json();
      assert.equal(body.subtotal, v.subtotal, `${v.name}: route subtotal`);
      assert.equal(body.total, v.expectedTotal, `${v.name}: route-composed total must equal the hand-derived constant`);
      await fastify.close();
    });
  }
});
