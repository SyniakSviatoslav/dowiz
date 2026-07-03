import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';

// PATCH /orders/:id/status guards (ADR-audit-fix-money):
//  • LC2 (authz): the current-status read is authorized by a LIVE active owner membership JOIN —
//    0 rows → 404 BEFORE any transition logic; the issued SQL is pinned to contain the JOIN so a
//    regression to the bare `WHERE id=$1` read goes RED (pre-fix code fails this).
//  • M6 / CC-1 (§3.5, both arms): PATCH → DELIVERED/PICKED_UP is refused 409 when (a) an active
//    binding exists (ASSIGNMENT_ACTIVE) or (b) the order is IN_DELIVERY without a delivered
//    assignment (USE_DELIVER_FLOW). Pre-fix code returned 200 and silently stranded the binding
//    (money-audit H1) → the 409 assertions are RED on pre-fix code.
//  • Escape hatch preserved: never-dispatched orders (zero assignments) stay PATCH-able.
// DB-free: scripted pg stub per orders-guards.test.ts.

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

const OWNER_ID = crypto.randomUUID();
const LOCATION_ID = crypto.randomUUID();

interface Scenario {
  /** the order row PHYSICALLY present in `orders` (null → the order does not exist at all) */
  orderRow: { id: string; status: string; location_id: string; type: string } | null;
  /**
   * whether the authenticated owner holds a LIVE `active` owner membership at the order's location.
   * Defaults to true (present) when omitted so the existing M6 scenarios are unaffected. When false,
   * a correctly-authorized JOIN read hides the physically-present row (→ 404), whereas a regressed
   * bare `WHERE id=$1` read would still return it (the cross-tenant IDOR the LC2 JOIN closes).
   */
  ownerHasActiveMembership?: boolean;
  activeAssignmentExists: boolean;
  deliveredAssignmentExists: boolean;
}

function scriptedQuery(sc: Scenario, issued: Array<{ sql: string; params: unknown[] }>) {
  return async (sql: string, params: unknown[] = []) => {
    const s = String(sql);
    issued.push({ sql: s, params });
    if (/^(BEGIN|COMMIT|ROLLBACK)$/i.test(s.trim()) || /set_config/i.test(s) || /SAVEPOINT/i.test(s)) return { rows: [], rowCount: 0 };
    // LC2 current-status read. Modelled to the DB's TRUTH, not to a string shape, so that dropping
    // the JOIN behaviourally leaks (returns the row → the handler transitions) instead of merely
    // failing a grep. The row comes back only when BOTH the order physically exists AND the SQL
    // actually authorizes it via an active owner-membership JOIN keyed by the caller. A regressed bare
    // `WHERE id=$1` read (no JOIN) ignores membership → returns the row for a non-member owner (the IDOR).
    if (/SELECT o\.id, o\.status, o\.location_id, o\.type|SELECT id, status, location_id, type FROM orders/i.test(s)) {
      if (!sc.orderRow) return { rowCount: 0, rows: [] }; // order does not exist at all
      const authorizedRead = /JOIN memberships/i.test(s) && /m\.status = 'active'/i.test(s);
      const ownerIsMember = sc.ownerHasActiveMembership !== false; // default: member (M6 scenarios)
      // A properly-authorized read hides the row from a non-member owner; a bare read leaks it.
      if (authorizedRead && !ownerIsMember) return { rowCount: 0, rows: [] };
      return { rowCount: 1, rows: [sc.orderRow] };
    }
    if (/status IN \('offered','assigned','accepted','picked_up'\)/i.test(s) && /SELECT 1 FROM courier_assignments/i.test(s)) {
      return sc.activeAssignmentExists ? { rowCount: 1, rows: [{ '?column?': 1 }] } : { rowCount: 0, rows: [] };
    }
    if (/SELECT 1 FROM courier_assignments WHERE order_id = \$1 AND status = 'delivered'/i.test(s)) {
      return sc.deliveredAssignmentExists ? { rowCount: 1, rows: [{ '?column?': 1 }] } : { rowCount: 0, rows: [] };
    }
    // updateOrderStatus internals (escape-path 200s):
    if (/SELECT id, status, location_id FROM orders WHERE id/i.test(s)) {
      return sc.orderRow ? { rowCount: 1, rows: [sc.orderRow] } : { rowCount: 0, rows: [] };
    }
    if (/^UPDATE orders SET status/i.test(s.trim())) return { rowCount: 1, rows: [{ id: sc.orderRow?.id }] };
    if (/INSERT INTO order_status_history/i.test(s)) return { rowCount: 1, rows: [] };
    if (/INSERT INTO payment_events/i.test(s)) return { rowCount: 0, rows: [] };
    if (/item_count/i.test(s)) {
      return {
        rowCount: 1,
        rows: [{
          id: sc.orderRow?.id, status: 'DELIVERED', total: 1000, created_at: new Date().toISOString(),
          location_id: LOCATION_ID, currency_code: 'ALL', item_count: 1,
          confirmed_at: null, preparing_at: null, ready_at: null, in_delivery_at: null, delivered_at: null, picked_up_at: null,
        }],
      };
    }
    return { rowCount: 0, rows: [] };
  };
}

async function buildApp(sc: Scenario, issued: Array<{ sql: string; params: unknown[] }>) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async (req: any) => { req.user = { role: 'owner', userId: OWNER_ID }; });
  fastify.decorate('softVerifyAuth', async (req: any) => { req.user = { role: 'owner', userId: OWNER_ID }; });
  fastify.decorate('requireRole', () => async () => {});
  const q = scriptedQuery(sc, issued);
  const client = { query: (sql: string, params?: unknown[]) => q(sql, params), release() {} };
  const db = { connect: async () => client, query: (sql: string, params?: unknown[]) => q(sql, params) } as any;
  await fastify.register(orderRoutes, {
    prefix: '/api',
    db,
    messageBus: { publish: async () => {} } as any,
    queue: { enqueue: async () => 'job', work: async () => {}, start: async () => {}, stop: async () => {} } as any,
  });
  return fastify;
}

const orderId = crypto.randomUUID();

test('LC2: membership-JOIN miss → 404 before any transition logic, and the read SQL is pinned to the JOIN', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({ orderRow: null, activeAssignmentExists: false, deliveredAssignmentExists: false }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'CANCELLED' } });
  assert.equal(res.statusCode, 404);
  const read = issued.find((x) => /FROM orders o/i.test(x.sql) && /o\.status/.test(x.sql));
  assert.ok(read, 'current-status read was issued');
  assert.match(read!.sql, /JOIN memberships/i, 'LC2: the read MUST be authorized by the membership JOIN (bare WHERE id=$1 is the cross-tenant leak)');
  assert.ok(read!.params.includes(OWNER_ID), 'LC2: the JOIN must be keyed by the authenticated owner id');
  assert.match(read!.sql, /m\.status = 'active'/, 'LC2: only a LIVE membership authorizes');
  // no transition attempt happened after the miss
  assert.ok(!issued.some((x) => /^UPDATE orders SET status/i.test(x.sql.trim())), '404 must precede any transition write');
  await app.close();
});

test('LC2 behavioral: order EXISTS but owner has NO active membership at its location → JOIN read hides it (404, zero transition); a bare WHERE id=$1 read would have leaked it (200)', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  // The order is PHYSICALLY present and PATCH-eligible (CONFIRMED→PREPARING is a legal, non-money edge),
  // but the authenticated owner is a member of a DIFFERENT tenant — no active membership at LOCATION_ID.
  const sc: Scenario = {
    orderRow: { id: orderId, status: 'CONFIRMED', location_id: LOCATION_ID, type: 'delivery' },
    ownerHasActiveMembership: false,
    activeAssignmentExists: false,
    deliveredAssignmentExists: false,
  };
  const app = await buildApp(sc, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'PREPARING' } });

  // Real (JOIN-authorized) behaviour: the membership predicate hides the row the owner may not see →
  // 404, and NOT ONE transition write is attempted. This is the LEAK the LC2 JOIN prevents.
  assert.equal(res.statusCode, 404, `membership-miss on an existing order must 404, not leak; body=${res.body}`);
  assert.ok(!issued.some((x) => /^UPDATE orders SET status/i.test(x.sql.trim())), 'a non-member owner must drive ZERO transitions');

  // Behavioural discrimination (no product-code edit needed): feed the SAME scripted DB the pre-fix BARE
  // read (JOIN stripped). It returns the row → the handler WOULD have transitioned and leaked. So the 404
  // above is EARNED by the JOIN authorizing the read, not by an incidental string match. Removing
  // `JOIN memberships … m.status='active'` from the route flips this scenario 404 → 200.
  const readCall = issued.find((x) => /FROM orders o/i.test(x.sql) && /o\.status/.test(x.sql));
  assert.ok(readCall, 'the membership-JOIN current-status read was issued');
  const probe = scriptedQuery(sc, []);
  const bareRead = await probe(`SELECT o.id, o.status, o.location_id, o.type FROM orders o WHERE o.id = $1`, [orderId, OWNER_ID]);
  const joinRead = await probe(readCall!.sql, readCall!.params);
  assert.equal(bareRead.rowCount, 1, 'pre-fix bare WHERE id=$1 read LEAKS the row to a non-member owner (the IDOR)');
  assert.equal(joinRead.rowCount, 0, 'the membership-JOIN read correctly returns 0 rows for a non-member owner');
  await app.close();
});

test('M6 arm (a): DELIVERED with an ACTIVE binding → 409 ASSIGNMENT_ACTIVE, binding untouched', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({
    orderRow: { id: orderId, status: 'IN_DELIVERY', location_id: LOCATION_ID, type: 'delivery' },
    activeAssignmentExists: true,
    deliveredAssignmentExists: false,
  }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'DELIVERED' } });
  assert.equal(res.statusCode, 409, `pre-fix code 200s and strands the binding; body=${res.body}`);
  assert.equal(res.json().code, 'ASSIGNMENT_ACTIVE');
  assert.ok(!issued.some((x) => /^UPDATE/i.test(x.sql.trim())), 'no mutation on refusal');
  await app.close();
});

test('M6 arm (a) also guards PICKED_UP with an active binding', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({
    orderRow: { id: orderId, status: 'READY', location_id: LOCATION_ID, type: 'delivery' },
    activeAssignmentExists: true,
    deliveredAssignmentExists: false,
  }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'PICKED_UP' } });
  assert.equal(res.statusCode, 409);
  assert.equal(res.json().code, 'ASSIGNMENT_ACTIVE');
  await app.close();
});

test('M6 arm (b): IN_DELIVERY with a DRAINED binding (no delivered assignment) → 409 USE_DELIVER_FLOW', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({
    orderRow: { id: orderId, status: 'IN_DELIVERY', location_id: LOCATION_ID, type: 'delivery' },
    activeAssignmentExists: false,
    deliveredAssignmentExists: false,
  }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'DELIVERED' } });
  assert.equal(res.statusCode, 409, `pre-fix code 200s with zero attestation; body=${res.body}`);
  assert.equal(res.json().code, 'USE_DELIVER_FLOW');
  await app.close();
});

test('M6 negative: IN_DELIVERY with a delivered assignment passes through (sanctioned completion already attested)', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({
    orderRow: { id: orderId, status: 'IN_DELIVERY', location_id: LOCATION_ID, type: 'delivery' },
    activeAssignmentExists: false,
    deliveredAssignmentExists: true,
  }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'DELIVERED' } });
  assert.equal(res.statusCode, 200, `body=${res.body}`);
  await app.close();
});

test('M6 escape preserved: never-dispatched pickup order (zero assignments) stays PATCH-able to PICKED_UP', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({
    orderRow: { id: orderId, status: 'READY', location_id: LOCATION_ID, type: 'pickup' },
    activeAssignmentExists: false,
    deliveredAssignmentExists: false,
  }, issued);
  const res = await app.inject({ method: 'PATCH', url: `/api/orders/${orderId}/status`, payload: { status: 'PICKED_UP' } });
  assert.equal(res.statusCode, 200, `never-dispatched must keep the phone/manual flow; body=${res.body}`);
  await app.close();
});
