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

// DB-free route-guard coverage for the POST /orders hotspot (orders.ts, CCN 140,
// previously zero unit tests). Stub the auth decorators and the pg pool so the
// early guards run without a real database.
async function buildApp(
  clientQuery: (sql: string, params?: any[]) => any,
  authOpts?: { verifyAuth?: () => Promise<void> },
) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const fastify = Fastify();
<<<<<<< Updated upstream
  fastify.decorate('verifyAuth', async () => {});
=======
  registerReplySendError(fastify); // A2: orders.ts uses reply.sendError — register it like server.ts
  fastify.decorate('verifyAuth', authOpts?.verifyAuth ?? (async () => {}));
>>>>>>> Stashed changes
  fastify.decorate('softVerifyAuth', async () => {});
  fastify.decorate('requireRole', () => async () => {});
  // params flow through so tests can assert tenant-scoping ($2 = location_id, etc.).
  const query = async (sql: string, params?: any[]) => clientQuery(sql, params);
  const client = { query, release() {} };
  // computeSignals() uses pool.query directly (not via connect) — route it to the same stub.
  const db = { connect: async () => client, query } as any;
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

// ─── Negative control: verifyAuth actually rejects (Finding 1) ──────────────
// The shared buildApp stubs verifyAuth as a no-op, so no positive test could
// catch a guard that silently lets everyone through. This builds an app where
// verifyAuth rejects with 401 and proves the PATCH preHandler chain runs it
// BEFORE the handler — i.e. an unauthenticated request is refused, not served.
test('PATCH /orders/:id/status — verifyAuth rejects unauthenticated request → 401', async () => {
  let handlerReached = false;
  const fastify = await buildApp(
    () => {
      handlerReached = true; // any DB touch means the guard let the request through
      return { rowCount: 0, rows: [] };
    },
    {
      verifyAuth: async () => {
        const e: any = new Error('Unauthorized');
        e.statusCode = 401;
        throw e;
      },
    },
  );
  const res = await fastify.inject({
    method: 'PATCH',
    url: `/api/orders/${crypto.randomUUID()}/status`,
    payload: { status: 'CONFIRMED' },
  });
  assert.equal(res.statusCode, 401);
  assert.equal(handlerReached, false); // guard short-circuited before the DB handler
  await fastify.close();
});

// ─── Velocity throttle 429 paths (Finding 3) ────────────────────────────────
// The phone-half throttle (orders.ts:244) fires before computeSignals/preflight.
test('POST /orders — phone over velocity throttle → 429 PHONE_THROTTLE', async () => {
  const pid = crypto.randomUUID();
  const fastify = await buildApp((sql: string) => {
    if (/FROM locations/i.test(sql)) return { rowCount: 1, rows: [{ lat: 0, lng: 0, published_at: '2026-01-01', slug: 'x', currency_code: 'ALL', currency_minor_unit: 0, require_phone_otp: false }] };
    if (/menu_versions/i.test(sql)) return { rowCount: 1, rows: [{ version: '1' }] };
    if (/velocity_events/i.test(sql) && /phone_hash/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 5 }] }; // >= THROTTLE_MAX_ORDERS
    return { rowCount: 0, rows: [] };
  });
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: { ...validPickup(), items: [{ product_id: pid, quantity: 1 }] } });
  assert.equal(res.statusCode, 429);
  assert.match(res.body, /PHONE_THROTTLE/);
  await fastify.close();
});

// The IP-half throttle (orders.ts:266) fires only after the phone check passes.
test('POST /orders — IP over velocity throttle (phone clean) → 429 IP_THROTTLE', async () => {
  const pid = crypto.randomUUID();
  const fastify = await buildApp((sql: string) => {
    if (/FROM locations/i.test(sql)) return { rowCount: 1, rows: [{ lat: 0, lng: 0, published_at: '2026-01-01', slug: 'x', currency_code: 'ALL', currency_minor_unit: 0, require_phone_otp: false }] };
    if (/menu_versions/i.test(sql)) return { rowCount: 1, rows: [{ version: '1' }] };
    if (/velocity_events/i.test(sql) && /client_ip_hash/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 20 }] }; // >= IP_THROTTLE_MAX_ORDERS
    if (/velocity_events/i.test(sql) && /phone_hash/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    return { rowCount: 0, rows: [] };
  });
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: { ...validPickup(), items: [{ product_id: pid, quantity: 1 }] } });
  assert.equal(res.statusCode, 429);
  assert.match(res.body, /IP_THROTTLE/);
  await fastify.close();
});

// Drives POST /orders through a CLEAN preflight (published location, available
// product, zero velocity) up to the tenant-scoped idempotency SELECT
// (orders.ts:363), which it defers to `onIdempotency(params)`. The post-idempotency
// product re-fetch (orders.ts:387) returns AVAILABLE by default.
function reachIdempotency(productId: string, onIdempotency: (params: any[]) => any) {
  return (sql: string, params?: any[]) => {
    if (/FROM locations/i.test(sql)) return { rowCount: 1, rows: [{ lat: 0, lng: 0, confirm_timeout_min: 10, busy_mode: false, phone: '', slug: 'x', published_at: '2026-01-01', currency_code: 'ALL', currency_minor_unit: 0, tax_rate: 0, price_includes_tax: false, min_order_value: null, free_delivery_threshold: null, delivery_fee_flat: null, require_phone_otp: false }] };
    if (/menu_versions/i.test(sql)) return { rowCount: 1, rows: [{ version: '1' }] };
    if (/SELECT id, is_available FROM products/i.test(sql)) return { rowCount: 1, rows: [{ id: productId, is_available: true }] }; // preflight (line 205)
    if (/velocity_events/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    if (/FROM idempotency_keys/i.test(sql)) return onIdempotency(params ?? []);
    if (/name, price, is_available/i.test(sql)) return { rowCount: 1, rows: [{ id: productId, name: 'X', price: 100, is_available: true }] }; // re-fetch (line 387)
    return { rowCount: 0, rows: [] };
  };
}

// ─── Idempotency conflict: same key, different request → 422 (Finding 4) ─────
test('POST /orders — idempotency key reused with a different request → 422 IDEMPOTENCY_KEY_REUSED', async () => {
  const pid = crypto.randomUUID();
  const fastify = await buildApp(
    reachIdempotency(pid, () => ({ rowCount: 1, rows: [{ order_id: crypto.randomUUID(), request_hash: 'a-different-hash' }] })),
  );
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: { ...validPickup(), items: [{ product_id: pid, quantity: 1 }] } });
  assert.equal(res.statusCode, 422);
  assert.match(res.body, /IDEMPOTENCY_KEY_REUSED/);
  await fastify.close();
});

// ─── Idempotency replay: same key + same hash → 200 cached order (Finding 4) ──
test('POST /orders — idempotency replay (same key+hash) → 200 cached order', async () => {
  const pid = crypto.randomUUID();
  const locationId = crypto.randomUUID();
  const items = [{ product_id: pid, quantity: 1 }];
  // Reproduce the exact server fingerprint (anonymous customer, menuVersion '1', ALL).
  const { buildRequestHash } = await import('../src/lib/order-canonical.js');
  const hash = buildRequestHash({ locationId, type: 'pickup', items, pin: null, addressText: null, cashPayWith: undefined, currencyCode: 'ALL', menuVersion: '1', customerId: 'anonymous' });
  const cachedId = crypto.randomUUID();
  const base = reachIdempotency(pid, () => ({ rowCount: 0, rows: [] }));
  const fastify = await buildApp((sql: string, params?: any[]) => {
    if (/FROM idempotency_keys/i.test(sql)) return { rowCount: 1, rows: [{ order_id: cachedId, request_hash: hash }] };
    if (/FROM orders WHERE id/i.test(sql)) return { rowCount: 1, rows: [{ id: cachedId, status: 'PENDING', subtotal: 100, total: 100, created_at: '2026-01-01T00:00:00Z', timeout_at: '2026-01-01T00:10:00Z' }] };
    return base(sql, params);
  });
  const res = await fastify.inject({ method: 'POST', url: '/api/orders', payload: { locationId, type: 'pickup', items, customer: { phone: '+15551234567', name: 'Test' }, payment: { method: 'cash' }, idempotency_key: crypto.randomUUID() } });
  assert.equal(res.statusCode, 200);
  assert.match(res.body, new RegExp(cachedId)); // returns the CACHED order, not a fresh one
  await fastify.close();
});

// ─── Cross-tenant idempotency isolation (Finding 2) ─────────────────────────
// The idempotency SELECT is scoped to (key, location_id). A key registered for
// tenant A must not replay against tenant B that submits the SAME key. Same
// stub, location-aware on the $2 param: returns a conflict row only for A.
test('POST /orders — idempotency is tenant-scoped (key for tenant A does not replay against tenant B)', async () => {
  const pid = crypto.randomUUID();
  const locationA = crypto.randomUUID();
  const locationB = crypto.randomUUID();
  const key = crypto.randomUUID();
  const mkStub = () => (sql: string, params?: any[]) => {
    if (/FROM locations/i.test(sql)) return { rowCount: 1, rows: [{ lat: 0, lng: 0, confirm_timeout_min: 10, busy_mode: false, phone: '', slug: 'x', published_at: '2026-01-01', currency_code: 'ALL', currency_minor_unit: 0, tax_rate: 0, price_includes_tax: false, min_order_value: null, free_delivery_threshold: null, delivery_fee_flat: null, require_phone_otp: false }] };
    if (/menu_versions/i.test(sql)) return { rowCount: 1, rows: [{ version: '1' }] };
    if (/SELECT id, is_available FROM products/i.test(sql)) return { rowCount: 1, rows: [{ id: pid, is_available: true }] };
    if (/velocity_events/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    if (/FROM idempotency_keys/i.test(sql)) {
      // Tenant A owns this key (different hash → conflict); tenant B has no such row.
      return (params ?? []).includes(locationA)
        ? { rowCount: 1, rows: [{ order_id: crypto.randomUUID(), request_hash: 'tenant-a-hash' }] }
        : { rowCount: 0, rows: [] };
    }
    // After idempotency passes for B, the re-fetch marks the product unavailable so the
    // request stops at a DOWNSTREAM 422 — proving it cleared the idempotency gate.
    if (/name, price, is_available/i.test(sql)) return { rowCount: 1, rows: [{ id: pid, name: 'X', price: 100, is_available: false }] };
    return { rowCount: 0, rows: [] };
  };
  const mkPayload = (locationId: string) => ({ locationId, type: 'pickup', items: [{ product_id: pid, quantity: 1 }], customer: { phone: '+15551234567', name: 'T' }, payment: { method: 'cash' }, idempotency_key: key });

  const appA = await buildApp(mkStub());
  const resA = await appA.inject({ method: 'POST', url: '/api/orders', payload: mkPayload(locationA) });
  assert.equal(resA.statusCode, 422);
  assert.match(resA.body, /IDEMPOTENCY_KEY_REUSED/); // key IS registered for tenant A
  await appA.close();

  const appB = await buildApp(mkStub());
  const resB = await appB.inject({ method: 'POST', url: '/api/orders', payload: mkPayload(locationB) });
  assert.equal(resB.statusCode, 422);
  assert.doesNotMatch(resB.body, /IDEMPOTENCY_KEY_REUSED/); // same key did NOT leak to tenant B
  assert.match(resB.body, /PRODUCT_UNAVAILABLE/);           // B cleared idempotency, rejected downstream
  await appB.close();
});
