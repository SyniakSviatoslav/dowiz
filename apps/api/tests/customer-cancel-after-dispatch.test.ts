import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';

// LC3 proof — customer post-dispatch cancel (money C2 / arch F4).
//
// The audit found the ONLY coverage of POST /customer/orders/:id/cancel was an e2e that posts with
// an OWNER token and asserts 403 — a "403-only false-green" that NEVER executes the happy path. The
// pre-fix handler wrote a raw `UPDATE orders SET status='CANCELLED', cancelled_at=now(),
// cancellation_reason=$x …` — but `orders` has NO cancelled_at/cancellation_reason column (they live
// on courier_assignments), so Postgres raised 42703 and the route 500-rolled-back on EVERY call.
//
// This is the missing customer-token HAPPY-PATH proof: an IN_DELIVERY order the customer owns, within
// the cancel window → 200, routed through the sanctioned `updateOrderStatus` mutator (fingerprint:
// `UPDATE orders SET status=$1, timeout_at=NULL`), with NO phantom-column write to `orders`.
// RED on the pre-fix raw UPDATE (it 500s, and it issues `UPDATE orders … cancelled_at`); GREEN now.
// DB-free: scripted pg stub, same pattern as orders-status-patch-guards.test.ts.

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

const CUSTOMER_ID = crypto.randomUUID();
const LOCATION_ID = crypto.randomUUID();
const orderId = crypto.randomUUID();

interface Scenario {
  /** the ownership+picked_up read: null → 0 rows (not owned / not picked up) */
  cancelRow: { location_id: string; status: string; picked_up_at: string } | null;
}

function scriptedQuery(sc: Scenario, issued: Array<{ sql: string; params: unknown[] }>) {
  return async (sql: string, params: unknown[] = []) => {
    const s = String(sql);
    issued.push({ sql: s, params });
    if (/^(BEGIN|COMMIT|ROLLBACK)$/i.test(s.trim()) || /set_config|SAVEPOINT|RELEASE/i.test(s)) return { rows: [], rowCount: 0 };
    // The route's ownership + picked_up read (SELECT o.location_id, o.status, ca.picked_up_at ...).
    if (/FROM orders o\s+JOIN courier_assignments ca/i.test(s)) {
      return sc.cancelRow ? { rowCount: 1, rows: [sc.cancelRow] } : { rowCount: 0, rows: [] };
    }
    // updateOrderStatus internals:
    if (/SELECT id, status, location_id FROM orders WHERE id/i.test(s)) {
      return { rowCount: 1, rows: [{ id: orderId, status: 'IN_DELIVERY', location_id: LOCATION_ID }] };
    }
    if (/^UPDATE orders SET status/i.test(s.trim())) return { rowCount: 1, rows: [{ id: orderId }] };
    if (/UPDATE courier_assignments|UPDATE courier_shifts|WITH freed/i.test(s)) return { rowCount: 0, rows: [] };
    if (/INSERT INTO payment_events/i.test(s)) return { rowCount: 0, rows: [] };
    if (/INSERT INTO order_status_history/i.test(s)) return { rowCount: 1, rows: [] };
    return { rowCount: 0, rows: [] };
  };
}

async function buildApp(sc: Scenario, issued: Array<{ sql: string; params: unknown[] }>, principal: any = { role: 'customer', sub: CUSTOMER_ID }) {
  ensureEnv();
  const { default: customerOrderRoutes } = await import('../src/routes/customer/orders.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  // Mirror the app's Zod validator/serializer compilers (server.ts:106) so the module's
  // ZodTypeProvider route schemas build.
  fastify.setValidatorCompiler(({ schema }: any) => (data: any) => {
    const r = schema.safeParse(data);
    return r.success ? { value: r.data } : { error: r.error };
  });
  fastify.setSerializerCompiler(({ schema }: any) => (data: any) => {
    const r = schema.safeParse(data);
    if (!r.success) throw new Error(String(r.error));
    return JSON.stringify(r.data);
  });
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async (req: any) => { req.user = principal; });
  fastify.decorate('requireRole', () => async () => {});
  const q = scriptedQuery(sc, issued);
  const client = { query: (sql: string, params?: unknown[]) => q(sql, params), release() {} };
  const db = { connect: async () => client, query: (sql: string, params?: unknown[]) => q(sql, params) } as any;
  await fastify.register(customerOrderRoutes, {
    prefix: '/api/customer',
    db,
    messageBus: { publish: async () => {} } as any,
  });
  return fastify;
}

test('LC3: customer cancels an owned IN_DELIVERY order within window → 200, via updateOrderStatus, NO phantom orders columns', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({ cancelRow: { location_id: LOCATION_ID, status: 'IN_DELIVERY', picked_up_at: new Date().toISOString() } }, issued);
  const res = await app.inject({ method: 'POST', url: `/api/customer/orders/${orderId}/cancel`, payload: { reason: 'changed_mind' } });

  assert.equal(res.statusCode, 200, `pre-fix code 500'd on 42703 (phantom columns); body=${res.body}`);
  assert.deepEqual(res.json(), { success: true });

  // Routed through the sanctioned mutator — its fingerprint UPDATE, not a raw one.
  assert.ok(
    issued.some((x) => /^UPDATE orders SET status = \$1, timeout_at = NULL/i.test(x.sql.trim())),
    'must transition via updateOrderStatus (UPDATE orders SET status=$1, timeout_at=NULL), not a raw UPDATE',
  );
  // THE bug: no write to the phantom orders.cancelled_at / orders.cancellation_reason (42703 source).
  const phantomOrdersWrite = issued.find((x) => /UPDATE\s+orders\b[\s\S]*?(cancelled_at|cancellation_reason)/i.test(x.sql));
  assert.equal(phantomOrdersWrite, undefined, `LC3: the orders table has no cancelled_at/cancellation_reason — writing them is the 42703 bug. Offending SQL: ${phantomOrdersWrite?.sql}`);
  // The tenant GUC was armed with the ownership-verified location (DEP-1b), tx-scoped.
  assert.ok(
    issued.some((x) => /set_config\('app\.current_tenant'/i.test(x.sql) && (x.params as unknown[]).includes(LOCATION_ID)),
    'tenant context set from the ownership-verified location_id before the mutation',
  );
  await app.close();
});

test('LC3: a customer cancelling an order they do NOT own (read 0 rows) → 403, no transition', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({ cancelRow: null }, issued);
  const res = await app.inject({ method: 'POST', url: `/api/customer/orders/${orderId}/cancel`, payload: { reason: 'changed_mind' } });
  assert.equal(res.statusCode, 403);
  assert.ok(!issued.some((x) => /^UPDATE orders SET status/i.test(x.sql.trim())), 'no transition on a non-owned order');
  await app.close();
});

test('LC3: cancelling an order not IN_DELIVERY → 409 CANCEL_NOT_ALLOWED_STATUS', async () => {
  const issued: Array<{ sql: string; params: unknown[] }> = [];
  const app = await buildApp({ cancelRow: { location_id: LOCATION_ID, status: 'READY', picked_up_at: new Date().toISOString() } }, issued);
  const res = await app.inject({ method: 'POST', url: `/api/customer/orders/${orderId}/cancel`, payload: { reason: 'changed_mind' } });
  assert.equal(res.statusCode, 409);
  assert.equal(res.json().code, 'CANCEL_NOT_ALLOWED_STATUS');
  await app.close();
});
