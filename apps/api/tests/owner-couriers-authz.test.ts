import './_env-stub.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';

// Sites #13/#14 (audit-fix-authz resolution.md §2 / F3 / F4): owner/couriers.ts and
// owner/courier-invites.ts carried `verifyAuth` + `requireLocationAccess` but NOT
// `requireRole(['owner'])` — `requireLocationAccess` alone admits a customer whose JWT
// locationId==L (decrypted courier roster leak) and a courier whose activeLocationId==L
// (mutate a co-worker's status/role, or mint a fresh courier invite). The fix adds the
// missing role gate (mirrors gdpr.ts) plus, for invites, a body `role` allow-list so an
// invite can never mint an 'owner'. DB-free assertions use REAL signed JWTs (verifyAuth is
// imported directly in these two files, not a decorator, so it cannot be stubbed the way
// couriers-authz.test.ts stubs `routes/couriers.ts`) against a SQL-aware in-memory stub pool.

const OWNER_A = crypto.randomUUID();
const LOC_A = crypto.randomUUID();
const COURIER_A = crypto.randomUUID();

async function signOwner() {
  const { signAuthToken } = await import('@deliveryos/platform');
  return signAuthToken({ role: 'owner', userId: OWNER_A, activeLocationId: LOC_A } as any, '1h');
}
async function signCustomer() {
  const { signAuthToken } = await import('@deliveryos/platform');
  return signAuthToken({ role: 'customer', orderId: crypto.randomUUID(), locationId: LOC_A } as any, '1h');
}
async function signCourier() {
  const { signAuthToken } = await import('@deliveryos/platform');
  return signAuthToken({ role: 'courier', activeLocationId: LOC_A, jti: crypto.randomUUID(), sub: COURIER_A } as any, '1h');
}

function makeDb() {
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) return { rowCount: 0, rows: [] };
      // courierSessionValid lookup (verifyAuth's courier-session binding).
      if (/FROM\s+courier_sessions\s+s/i.test(sql)) {
        return { rowCount: 1, rows: [{ courier_id: COURIER_A, revoked_at: null, expires_at: null, has_location: true }] };
      }
      // requireLocationAccess's owner-membership check.
      if (/FROM\s+memberships\s+WHERE/i.test(sql)) {
        const [locationId, userId] = params;
        return locationId === LOC_A && userId === OWNER_A
          ? { rowCount: 1, rows: [{ '?column?': 1 }] }
          : { rowCount: 0, rows: [] };
      }
      // couriers.ts GET list (owner happy path) — empty roster is fine for a status check.
      if (/FROM\s+couriers\s+c/i.test(sql)) return { rowCount: 0, rows: [] };
      // courier-invites.ts POST (owner happy path).
      if (/INSERT\s+INTO\s+courier_invites/i.test(sql)) {
        return { rowCount: 1, rows: [{ id: crypto.randomUUID(), expires_at: new Date().toISOString() }] };
      }
      return { rowCount: 0, rows: [] };
    },
    release() {},
  };
  const db: any = { connect: async () => client, query: client.query };
  return { db, queries };
}

async function buildApp(routeModule: string, db: any) {
  const { default: routes } = await import(routeModule);
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  registerReplySendError(fastify);
  fastify.decorate('db', db);
  await fastify.register(routes, { db });
  return fastify;
}

// ─── F3 — owner/couriers.ts ─────────────────────────────────────────────────

test('F3 customer-token GET /couriers → 403 (was 200 with decrypted roster)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/couriers.js', db);
  const token = await signCustomer();
  const res = await fastify.inject({
    method: 'GET',
    url: `/api/owner/locations/${LOC_A}/couriers`,
    headers: { authorization: `Bearer ${token}` },
  });
  assert.equal(res.statusCode, 403, `expected 403, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('F3 courier-token PATCH /couriers/:id → 403 (co-worker mutation blocked)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/couriers.js', db);
  const token = await signCourier();
  const res = await fastify.inject({
    method: 'PATCH',
    url: `/api/owner/locations/${LOC_A}/couriers/${crypto.randomUUID()}`,
    headers: { authorization: `Bearer ${token}` },
    payload: { status: 'deactivated' },
  });
  assert.equal(res.statusCode, 403, `expected 403, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('F3 owner-token GET /couriers on own location → 200 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/couriers.js', db);
  const token = await signOwner();
  const res = await fastify.inject({
    method: 'GET',
    url: `/api/owner/locations/${LOC_A}/couriers`,
    headers: { authorization: `Bearer ${token}` },
  });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

// ─── F4 — owner/courier-invites.ts ──────────────────────────────────────────

test('F4 courier-token POST /courier-invites → 403 (non-owner cannot mint invites)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/courier-invites.js', db);
  const token = await signCourier();
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/courier-invites`,
    headers: { authorization: `Bearer ${token}` },
    payload: { role: 'courier', email: 'new-courier@example.com' },
  });
  assert.equal(res.statusCode, 403, `expected 403, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('F4 owner-token POST /courier-invites {role:"owner"} → 400 (allow-list rejects non-courier role)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/courier-invites.js', db);
  const token = await signOwner();
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/courier-invites`,
    headers: { authorization: `Bearer ${token}` },
    payload: { role: 'owner', email: 'shadow-owner@example.com' },
  });
  assert.equal(res.statusCode, 400, `expected 400, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('F4 owner-token POST /courier-invites {role:"courier"} → 200 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp('../src/routes/owner/courier-invites.js', db);
  const token = await signOwner();
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/courier-invites`,
    headers: { authorization: `Bearer ${token}` },
    payload: { role: 'courier', email: 'new-courier@example.com' },
  });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});
