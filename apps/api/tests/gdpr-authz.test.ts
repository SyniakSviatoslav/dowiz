import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import type { ZodTypeAny } from 'zod';

// Site #1 (audit-fix-authz resolution.md §2): owner/gdpr.ts's direct-customerId branch used to
// trust a client-supplied customerId verbatim — a foreign (cross-tenant) customerId enqueued a
// real, irreversible erasure request against another tenant's customer (LC5). The fix adds a
// same-tenant proof (SELECT 1 FROM customers WHERE id=$1 AND location_id=$2) before the INSERT,
// classifies the gate-miss server-side (nonexistent vs cross-tenant) and security-logs a blocked
// cross-tenant attempt before returning 404. DB-free: mirrors the repo's existing self-bootstrapping
// authz pattern (couriers-authz.test.ts / orders-authz.test.ts) — a SQL-aware in-memory stub pool
// exercised through the REAL route via fastify.inject.

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'test-priv', JWT_PUBLIC_KEY: 'test-pub', JWT_KID: 'test',
    GOOGLE_CLIENT_ID: 'test', GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test', VAPID_PRIVATE_KEY: 'test', IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}

const OWNER_A = crypto.randomUUID();
const LOC_A = crypto.randomUUID();
const LOC_B = crypto.randomUUID();
const CUSTOMER_A = crypto.randomUUID(); // belongs to LOC_A
const CUSTOMER_B = crypto.randomUUID(); // belongs to LOC_B — a DIFFERENT tenant

interface CustomerRow { id: string; location_id: string }

function makeDb(customers: CustomerRow[]) {
  const byId = new Map(customers.map((c) => [c.id, c]));
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) {
        return { rowCount: 0, rows: [] };
      }
      // The NEW same-tenant proof (site #1 fix).
      if (/SELECT\s+1\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1\s+AND\s+location_id\s*=\s*\$2/i.test(sql)) {
        const [id, locationId] = params;
        const c = byId.get(id);
        return c && c.location_id === locationId
          ? { rowCount: 1, rows: [{ '?column?': 1 }] }
          : { rowCount: 0, rows: [] };
      }
      // The NEW gate-miss classifier (exists cross-tenant vs nonexistent).
      if (/SELECT\s+location_id\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1\s*$/i.test(sql.trim())) {
        const c = byId.get(params[0]);
        return c ? { rowCount: 1, rows: [{ location_id: c.location_id }] } : { rowCount: 0, rows: [] };
      }
      if (/FROM\s+customers\s+WHERE\s+location_id[\s\S]*phone/i.test(sql)) {
        return { rowCount: 0, rows: [] }; // no phone-based resolution exercised in these tests
      }
      if (/FROM\s+gdpr_erasure_requests[\s\S]*status\s+IN/i.test(sql)) {
        return { rowCount: 0, rows: [] }; // no pre-existing active request
      }
      if (/FROM\s+gdpr_erasure_requests[\s\S]*completed/i.test(sql)) {
        return { rowCount: 0, rows: [] }; // no recent cooldown request
      }
      if (/INSERT\s+INTO\s+gdpr_erasure_requests/i.test(sql)) {
        return { rowCount: 1, rows: [{ id: 'new-request-id' }] };
      }
      return { rowCount: 0, rows: [] };
    },
    release() {},
  };
  const db: any = { connect: async () => client };
  return { db, queries };
}

function makeLogger() {
  const warnCalls: any[] = [];
  const logger: any = {
    level: 'info',
    fatal() {}, error() {}, info() {}, debug() {}, trace() {},
    warn(...args: any[]) { warnCalls.push(args); },
    child() { return logger; },
  };
  return { logger, warnCalls };
}

async function buildApp(db: any, logger: any) {
  ensureEnv();
  const { default: gdprRoutes } = await import('../src/routes/owner/gdpr.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify({ loggerInstance: logger });
  fastify.setValidatorCompiler(({ schema }) => (data) => {
    const result = (schema as ZodTypeAny).safeParse(data);
    return result.success ? { value: result.data } : { error: result.error as any };
  });
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async (request: any) => {
    request.user = { userId: OWNER_A, role: 'owner', activeLocationId: LOC_A };
  });
  fastify.decorate('requireRole', () => async () => {});
  fastify.decorate('requireLocationAccess', async () => {});
  await fastify.register(gdprRoutes, {
    prefix: '/api/owner/locations',
    db,
    messageBus: { publish: async () => {} },
    queue: { send: async () => {} },
  });
  return fastify;
}

test('POST /gdpr-requests — cross-tenant customerId (LC5) → 404, no request row created, attempt logged', async () => {
  const { db, queries } = makeDb([
    { id: CUSTOMER_A, location_id: LOC_A },
    { id: CUSTOMER_B, location_id: LOC_B },
  ]);
  const { logger, warnCalls } = makeLogger();
  const fastify = await buildApp(db, logger);

  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/gdpr-requests`,
    payload: { customerId: CUSTOMER_B }, // owner-A attacks tenant-B's customer
  });

  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  const inserted = queries.find((q) => /INSERT\s+INTO\s+gdpr_erasure_requests/i.test(q.sql));
  assert.equal(inserted, undefined, 'no gdpr_erasure_requests row may be created for a cross-tenant customerId');

  const attemptLog = warnCalls.find(([payload]) => payload?.event === 'cross_tenant_attempt');
  assert.ok(attemptLog, 'a blocked cross-tenant erasure attempt must be security-logged');
  assert.equal(attemptLog[0].targetCustomerId, CUSTOMER_B);
  assert.equal(attemptLog[0].subjectLocationId, LOC_B);
  assert.equal(attemptLog[0].actorLocationId, LOC_A);
  await fastify.close();
});

test('POST /gdpr-requests — nonexistent customerId → 404, no attempt logged (not a cross-tenant signal)', async () => {
  const { db, queries } = makeDb([{ id: CUSTOMER_A, location_id: LOC_A }]);
  const { logger, warnCalls } = makeLogger();
  const fastify = await buildApp(db, logger);

  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/gdpr-requests`,
    payload: { customerId: crypto.randomUUID() }, // does not exist anywhere
  });

  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  const inserted = queries.find((q) => /INSERT\s+INTO\s+gdpr_erasure_requests/i.test(q.sql));
  assert.equal(inserted, undefined, 'no gdpr_erasure_requests row for a nonexistent customerId');
  const attemptLog = warnCalls.find(([payload]) => payload?.event === 'cross_tenant_attempt');
  assert.equal(attemptLog, undefined, 'a nonexistent id is not a cross-tenant attempt — must not be logged as one');
  await fastify.close();
});

test('POST /gdpr-requests — own-tenant customerId succeeds (no regression)', async () => {
  const { db, queries } = makeDb([{ id: CUSTOMER_A, location_id: LOC_A }]);
  const { logger } = makeLogger();
  const fastify = await buildApp(db, logger);

  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/gdpr-requests`,
    payload: { customerId: CUSTOMER_A },
  });

  assert.equal(res.statusCode, 201, `expected 201, got ${res.statusCode}: ${res.body}`);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'pending');
  const inserted = queries.find((q) => /INSERT\s+INTO\s+gdpr_erasure_requests/i.test(q.sql));
  assert.ok(inserted, 'a gdpr_erasure_requests row is created for the owner’s own-tenant customer');
  assert.equal(inserted!.params[0], LOC_A);
  assert.equal(inserted!.params[1], CUSTOMER_A);
  await fastify.close();
});
