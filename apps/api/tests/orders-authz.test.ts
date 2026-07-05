import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';

// Behavioral red→green tests for the security-hardening-2026-07 batch:
//   #1 orders IDOR — GET /orders/:id must authorize owners by a LIVE active-membership
//      JOIN and couriers by a LIVE courier_assignments binding (courierReadVerdict),
//      never by a bare `WHERE id=$1` (which leaks cross-tenant under the BYPASSRLS pool)
//      nor by the baked activeLocationId (which leaves an insider-removal read window).
//   #8 customer identity — velocity/idempotency must key off the customer token `sub`
//      (which IS the customerId), not the undefined `userId`.
//
// DB-free: the pg pool is a SQL-aware stub over a tiny in-memory model, so the SAME
// stub yields RED against the vulnerable code (bare `FROM orders WHERE id=$1` returns
// the row → 200 leak) and GREEN against the fixed code (JOIN / binding → 0 rows → 404).

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

interface OrderRec {
  id: string;
  location_id: string;
  customer_id: string;
  status: string;
  type: string;
  delivery_address: string | null;
  subtotal: number;
  total: number;
  payment_method: string;
  payment_outcome: string | null;
  created_at: string;
  timeout_at: string | null;
}

interface Model {
  orders: Map<string, OrderRec>;
  ownerMemberships: { user_id: string; location_id: string }[]; // active owner memberships
  courierAssignments: { order_id: string; courier_id: string; status: string }[];
}

function orderRow(o: OrderRec) {
  return { ...o };
}

// Router shared by the GET /orders/:id authz tests.
function authzRouter(model: Model) {
  return (sql: string, params: any[] = []) => {
    if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config|SET LOCAL/i.test(sql)) {
      return { rowCount: 0, rows: [] };
    }
    // Owner authorizing read (fixed code): JOIN memberships.
    if (/JOIN\s+memberships/i.test(sql)) {
      const [id, userId] = params;
      const o = model.orders.get(id);
      const ok = o && model.ownerMemberships.some(m => m.user_id === userId && m.location_id === o.location_id);
      return ok ? { rowCount: 1, rows: [orderRow(o!)] } : { rowCount: 0, rows: [] };
    }
    // Courier binding verdict (fixed code).
    if (/FROM\s+courier_assignments/i.test(sql)) {
      const [orderId, courierId, statuses] = params;
      const list: string[] = Array.isArray(statuses) ? statuses : [];
      const hit = model.courierAssignments.some(a => a.order_id === orderId && a.courier_id === courierId && list.includes(a.status));
      return hit ? { rowCount: 1, rows: [{ '?column?': 1 }] } : { rowCount: 0, rows: [] };
    }
    if (/FROM\s+order_items/i.test(sql)) {
      return { rowCount: 0, rows: [] };
    }
    // Bare `FROM orders WHERE id=$1[ AND location_id=$2]` — the VULNERABLE read path
    // (old owner/courier branch) and the legitimate customer/courier post-verdict read.
    if (/FROM\s+orders/i.test(sql)) {
      const id = params[0];
      const loc = params[1];
      const o = model.orders.get(id);
      if (!o) return { rowCount: 0, rows: [] };
      if (loc && o.location_id !== loc) return { rowCount: 0, rows: [] };
      return { rowCount: 1, rows: [orderRow(o)] };
    }
    return { rowCount: 0, rows: [] };
  };
}

async function buildApp(
  route: (sql: string, params?: any[]) => any,
  opts: { user?: any; failConnect?: boolean } = {},
) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async () => {});
  fastify.decorate('softVerifyAuth', async () => {});
  fastify.decorate('requireRole', () => async () => {});
  // Inject the principal (POST /orders has no auth preHandler; GET uses softVerifyAuth
  // which is a no-op stub → set request.user directly, mirroring what the real hook does).
  fastify.addHook('onRequest', async (request: any) => {
    if (opts.user) request.user = opts.user;
  });
  const client = {
    query: async (sql: string, params?: any[]) => route(sql, params),
    release() {},
  };
  const db = {
    connect: async () => {
      if (opts.failConnect) throw new Error('pool exhausted');
      return client;
    },
    query: async (sql: string, params?: any[]) => route(sql, params),
  } as any;
  await fastify.register(orderRoutes, {
    prefix: '/api',
    db,
    messageBus: { publish: async () => {} } as any,
    queue: { enqueue: async () => 'job', work: async () => {}, start: async () => {}, stop: async () => {} } as any,
  });
  return fastify;
}

function makeOrder(id: string, location_id: string): OrderRec {
  return {
    id, location_id, customer_id: crypto.randomUUID(), status: 'PENDING', type: 'delivery',
    delivery_address: '1 Main St', subtotal: 1000, total: 1200, payment_method: 'cash',
    payment_outcome: null, created_at: new Date().toISOString(), timeout_at: null,
  };
}

// ─── #1 owner IDOR ────────────────────────────────────────────────────────────

test('#1 owner-A GET owner-B order → 404 (cross-tenant denied, no row leaked)', async () => {
  const locA = crypto.randomUUID(), locB = crypto.randomUUID();
  const ownerA = crypto.randomUUID();
  const orderB = makeOrder(crypto.randomUUID(), locB);
  const model: Model = {
    orders: new Map([[orderB.id, orderB]]),
    ownerMemberships: [{ user_id: ownerA, location_id: locA }], // owner-A only at loc-A
    courierAssignments: [],
  };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'owner', userId: ownerA, sub: ownerA, activeLocationId: locA },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${orderB.id}` });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  assert.doesNotMatch(res.body, new RegExp(orderB.delivery_address!), 'PII must not leak');
  await app.close();
});

test('#1 owner-A GET own order (multi-loc membership) → 200', async () => {
  const locA = crypto.randomUUID(), locB = crypto.randomUUID();
  const ownerA = crypto.randomUUID();
  const orderAtB = makeOrder(crypto.randomUUID(), locB); // owner-A owns loc-A AND loc-B
  const model: Model = {
    orders: new Map([[orderAtB.id, orderAtB]]),
    ownerMemberships: [
      { user_id: ownerA, location_id: locA },
      { user_id: ownerA, location_id: locB },
    ],
    courierAssignments: [],
  };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'owner', userId: ownerA, sub: ownerA, activeLocationId: locA },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${orderAtB.id}` });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  assert.equal(JSON.parse(res.body).id, orderAtB.id);
  await app.close();
});

// ─── #1 courier binding (+ insider-window) ──────────────────────────────────────

test('#1 courier NOT bound to order → 404 (denied)', async () => {
  const loc = crypto.randomUUID();
  const courier = crypto.randomUUID();
  const order = makeOrder(crypto.randomUUID(), loc);
  const model: Model = {
    orders: new Map([[order.id, order]]),
    ownerMemberships: [],
    courierAssignments: [], // no binding
  };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'courier', sub: courier, activeLocationId: loc },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${order.id}` });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await app.close();
});

test('#1 ex-courier (revoked binding, status not in read set) → 404 (insider-window closed)', async () => {
  const loc = crypto.randomUUID();
  const courier = crypto.randomUUID();
  const order = makeOrder(crypto.randomUUID(), loc);
  const model: Model = {
    orders: new Map([[order.id, order]]),
    ownerMemberships: [],
    // binding exists but was completed/cancelled → not a live READ binding.
    courierAssignments: [{ order_id: order.id, courier_id: courier, status: 'cancelled' }],
  };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'courier', sub: courier, activeLocationId: loc },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${order.id}` });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await app.close();
});

test('#1 courier WITH live binding → 200 (happy path preserved)', async () => {
  const loc = crypto.randomUUID();
  const courier = crypto.randomUUID();
  const order = makeOrder(crypto.randomUUID(), loc);
  const model: Model = {
    orders: new Map([[order.id, order]]),
    ownerMemberships: [],
    courierAssignments: [{ order_id: order.id, courier_id: courier, status: 'assigned' }],
  };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'courier', sub: courier, activeLocationId: loc },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${order.id}` });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  assert.equal(JSON.parse(res.body).id, order.id);
  await app.close();
});

test('#1 courier binding check UNAVAILABLE (pool blip) → 503 (fail-closed retryable, not open)', async () => {
  const loc = crypto.randomUUID();
  const courier = crypto.randomUUID();
  const order = makeOrder(crypto.randomUUID(), loc);
  const model: Model = { orders: new Map([[order.id, order]]), ownerMemberships: [], courierAssignments: [] };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'courier', sub: courier, activeLocationId: loc },
    failConnect: true, // courierReadVerdict's db.connect() throws → UNAVAILABLE
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${order.id}` });
  assert.equal(res.statusCode, 503, `expected 503, got ${res.statusCode}: ${res.body}`);
  await app.close();
});

// ─── customer read (no regression) ──────────────────────────────────────────────

test('#1 customer GET own order → 200 (scoped read preserved)', async () => {
  const loc = crypto.randomUUID();
  const order = makeOrder(crypto.randomUUID(), loc);
  const model: Model = { orders: new Map([[order.id, order]]), ownerMemberships: [], courierAssignments: [] };
  const app = await buildApp(authzRouter(model), {
    user: { role: 'customer', sub: order.customer_id, orderId: order.id, locationId: loc },
  });
  const res = await app.inject({ method: 'GET', url: `/api/orders/${order.id}` });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  await app.close();
});

// ─── #8 customer identity in velocity/idempotency ───────────────────────────────

test('#8 customer POST /orders keys velocity/idempotency off `sub` (the real customerId)', async () => {
  const loc = crypto.randomUUID();
  const productId = crypto.randomUUID();
  const customerId = crypto.randomUUID();
  const captured: { customersById: string | null } = { customersById: null };

  const createRouter = (sql: string, params: any[] = []) => {
    if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config|SET LOCAL/i.test(sql)) return { rowCount: 0, rows: [] };
    if (/FROM\s+locations\s+WHERE\s+id/i.test(sql)) {
      return { rowCount: 1, rows: [{
        lat: 0, lng: 0, confirm_timeout_min: 10, busy_mode: false, phone: '+1', slug: 's',
        published_at: new Date().toISOString(), currency_code: 'ALL', currency_minor_unit: 0,
        tax_rate: 0, price_includes_tax: false, min_order_value: 0, free_delivery_threshold: null,
        delivery_fee_flat: 0, require_phone_otp: false,
      }] };
    }
    if (/FROM\s+menu_versions/i.test(sql)) return { rowCount: 1, rows: [{ version: '1' }] };
    if (/FROM\s+products\s+WHERE/i.test(sql)) return { rowCount: 1, rows: [{ id: productId, is_available: true }] };
    if (/FROM\s+velocity_events/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    // computeSignals no-show lookup — this is the assertion target.
    if (/FROM\s+customers\s+WHERE\s+id\s*=\s*\$1/i.test(sql)) {
      captured.customersById = params[0] ?? null;
      return { rowCount: 0, rows: [] };
    }
    if (/app_velocity_(phone|ip)_count/i.test(sql)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    // Everything past computeSignals is irrelevant to this assertion; fail closed cheaply.
    return { rowCount: 0, rows: [] };
  };

  const app = await buildApp(createRouter, {
    user: { role: 'customer', sub: customerId, orderId: crypto.randomUUID(), locationId: loc },
  });
  const payload = {
    locationId: loc,
    type: 'pickup',
    items: [{ product_id: productId, quantity: 1 }],
    customer: { phone: '+15551234567', name: 'Test' },
    payment: { method: 'cash' },
    idempotency_key: crypto.randomUUID(),
  };
  await app.inject({ method: 'POST', url: '/api/orders', payload });
  // RED (pre-fix): request.user.userId is undefined → computeSignals gets customerId=undefined
  //   → the no-show `FROM customers WHERE id=$1` query never fires → captured stays null.
  // GREEN (post-fix): identity = sub → query fires with the real customerId.
  assert.equal(captured.customersById, customerId,
    `expected computeSignals to query customers by sub=${customerId}, got ${captured.customersById}`);
  await app.close();
});
