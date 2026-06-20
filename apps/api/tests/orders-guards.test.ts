import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';

// orders.ts calls loadEnv() at module load, so set a dummy env BEFORE importing it
// (ESM evaluates static imports first → orderRoutes is imported dynamically below).
function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    APP_BASE_URL: 'http://localhost:3000',
    ***REDACTED***: 'postgres://u:p@localhost:5432/db',
    ***REDACTED***: 'postgres://u:p@localhost:5432/db',
    ***REDACTED***: 'postgres://u:p@localhost:5432/db',
    REDIS_URL: 'redis://localhost:6379',
    ***REDACTED***: 'test-priv',
    ***REDACTED***: 'test-pub',
    JWT_KID: 'test',
    ***REDACTED***: 'test',
    ***REDACTED***: 'test',
    VAPID_PUBLIC_KEY: 'test',
    VAPID_PRIVATE_KEY: 'test',
    IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}

// DB-free route-guard coverage for the POST /orders hotspot (orders.ts, CCN 140,
// previously zero unit tests). Stub the auth decorators and the pg pool so the
// early guards run without a real database.
async function buildApp(clientQuery: (sql: string) => any) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const fastify = Fastify();
  fastify.decorate('verifyAuth', async () => {});
  fastify.decorate('softVerifyAuth', async () => {});
  fastify.decorate('requireRole', () => async () => {});
  const client = { query: async (sql: string) => clientQuery(sql), release() {} };
  const db = { connect: async () => client } as any;
  await fastify.register(orderRoutes, {
    prefix: '/api',
    db,
    messageBus: { publish: async () => {} } as any,
    queue: { enqueue: async () => 'job', work: async () => {}, start: async () => {}, stop: async () => {} } as any,
  });
  return fastify;
}

const validPickup = () => ({
  locationId: crypto.randomUUID(),
  type: 'pickup',
  items: [{ product_id: crypto.randomUUID(), quantity: 1 }],
  customer: { phone: '+15551234567', name: 'Test' },
  payment: { method: 'cash' },
  idempotency_key: crypto.randomUUID(),
});

test('POST /orders — DRAFT storefront (published_at null) → 409 NOT_PUBLISHED', async () => {
  const fastify = await buildApp((sql) => {
    if (/FROM locations/i.test(sql)) {
      return { rowCount: 1, rows: [{ lat: 0, lng: 0, published_at: null, slug: 'x', currency_code: 'ALL', currency_minor_unit: 0, require_phone_otp: false }] };
    }
    return { rowCount: 0, rows: [] };
  });
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: validPickup() });
  assert.equal(res.statusCode, 409);
  assert.match(res.body, /NOT_PUBLISHED/);
  await fastify.close();
});

test('POST /orders — unknown location → 404', async () => {
  const fastify = await buildApp(() => ({ rowCount: 0, rows: [] }));
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: validPickup() });
  assert.equal(res.statusCode, 404);
  assert.match(res.body, /Location not found/);
  await fastify.close();
});

test('POST /orders — invalid body → 400 validation', async () => {
  const fastify = await buildApp(() => ({ rowCount: 0, rows: [] }));
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: { type: 'pickup' } });
  assert.equal(res.statusCode, 400);
  await fastify.close();
});

test('POST /orders — delivery order without pin → 400 (schema refine)', async () => {
  const fastify = await buildApp(() => ({ rowCount: 0, rows: [] }));
  const bad = { ...validPickup(), type: 'delivery' }; // delivery requires a pin
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: bad });
  assert.equal(res.statusCode, 400);
  await fastify.close();
});
