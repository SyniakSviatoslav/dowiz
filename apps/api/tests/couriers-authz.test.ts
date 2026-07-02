import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import { z, type ZodTypeAny } from 'zod';

// couriers.ts imports @deliveryos/platform (withTenant) — no loadEnv() at module load, but keep a
// dummy env for parity with the other route-unit suites.
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
const OWNER_A_LOCATION = crypto.randomUUID(); // owner-A's own location
const OWNER_B_LOCATION = crypto.randomUUID(); // a DIFFERENT tenant's location

// Fake pool/client that models the DB AFTER the explicit membership predicate is applied:
// the memberships row exists ONLY when (user_id, location_id) = (OWNER_A, OWNER_A_LOCATION).
// A pure RLS-only `SELECT 1 FROM locations WHERE id=$1` (the vulnerable form) would instead
// return a row for ANY locationId under the BYPASSRLS pool — that is what this test forbids.
function makeDb() {
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      // The authorization SELECT: only owner-A's own location authorizes.
      if (/SELECT[\s\S]*FROM\s+memberships/i.test(sql)) {
        const [userId, locationId] = params;
        if (userId === OWNER_A && locationId === OWNER_A_LOCATION) {
          return { rowCount: 1, rows: [{ '?column?': 1 }] };
        }
        return { rowCount: 0, rows: [] };
      }
      // RLS-only vulnerable pattern: if the handler still queried `locations` by id alone,
      // model the BYPASSRLS pool that returns the row cross-tenant → the test would then
      // observe a created invite for owner-B's location and FAIL.
      if (/SELECT[\s\S]*FROM\s+locations/i.test(sql)) {
        return { rowCount: 1, rows: [{ '?column?': 1 }] };
      }
      return { rowCount: 1, rows: [] };
    },
    release() {},
  };
  const db: any = { connect: async () => client, __client: client, __queries: queries };
  return db;
}

async function buildApp(db: any) {
  ensureEnv();
  const { default: courierRoutes } = await import('../src/routes/couriers.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  // Mirror server.ts's Zod validator compiler so `schema.body` (a ZodObject) is enforced.
  fastify.setValidatorCompiler(({ schema }) => (data) => {
    const result = (schema as ZodTypeAny).safeParse(data);
    return result.success ? { value: result.data } : { error: result.error as any };
  });
  registerReplySendError(fastify);
  fastify.decorate('db', db);
  // Authenticated as owner-A on every request.
  fastify.decorate('verifyAuth', async (request: any) => {
    request.user = { userId: OWNER_A, role: 'owner', activeLocationId: OWNER_A_LOCATION };
  });
  fastify.decorate('requireRole', () => async () => {});
  await fastify.register(courierRoutes);
  return fastify;
}

test('POST /couriers/invites — #7 cross-tenant invite is denied (no courier_invites row)', async () => {
  const db = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: '/couriers/invites',
    payload: { locationId: OWNER_B_LOCATION }, // owner-A attacks owner-B's location
  });
  assert.equal(res.statusCode, 404, 'cross-tenant locationId must be rejected 404');
  const inserted = db.__queries.find((q: any) => /INSERT\s+INTO\s+courier_invites/i.test(q.sql));
  assert.equal(inserted, undefined, 'no courier_invites row may be created for a cross-tenant location');
  await fastify.close();
});

test('POST /couriers/invites — authorization uses an explicit membership predicate (not RLS-only locations)', async () => {
  const db = makeDb();
  const fastify = await buildApp(db);
  await fastify.inject({
    method: 'POST',
    url: '/couriers/invites',
    payload: { locationId: OWNER_B_LOCATION },
  });
  const authSelect = db.__queries.find((q: any) => /SELECT[\s\S]*FROM\s+memberships/i.test(q.sql));
  assert.ok(authSelect, 'ownership must be checked against memberships, not locations alone');
  assert.match(authSelect.sql, /user_id\s*=\s*\$1/, 'predicate must bind the authenticated owner');
  assert.match(authSelect.sql, /location_id\s*=\s*\$2/, 'predicate must bind the body locationId');
  assert.match(authSelect.sql, /role\s*=\s*'owner'/, 'predicate must require the owner role');
  assert.match(authSelect.sql, /status\s*=\s*'active'/, 'predicate must require a live active membership');
  assert.deepEqual(authSelect.params, [OWNER_A, OWNER_B_LOCATION]);
  await fastify.close();
});

test('POST /couriers/invites — owner creating for their OWN location succeeds', async () => {
  const db = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: '/couriers/invites',
    payload: { locationId: OWNER_A_LOCATION },
  });
  assert.equal(res.statusCode, 200, 'own-location invite must succeed');
  const body = JSON.parse(res.body);
  assert.ok(typeof body.code === 'string' && body.code.length === 6, 'an invite code is returned');
  const inserted = db.__queries.find((q: any) => /INSERT\s+INTO\s+courier_invites/i.test(q.sql));
  assert.ok(inserted, 'a courier_invites row is created for the owner’s own location');
  assert.equal(inserted.params[0], OWNER_A_LOCATION, 'the invite is bound to the owner’s own location');
  await fastify.close();
});

// z is imported for its type only above; reference it so the import is not elided.
void z;
