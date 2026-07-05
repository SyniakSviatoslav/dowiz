import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import type { ZodTypeAny } from 'zod';

// Site #15 / R2-1 (audit-fix-authz resolution-r2.md §3.7): owner/menu-availability.ts POST
// .../menu-schedules trusted body product_id/category_id verbatim. menu_schedules' RLS WITH CHECK
// only validates the attacker's OWN location_id; the plain single-column FK to products/categories
// bypasses RLS, so an owner-A could schedule `available:false` against tenant-B's product and — via
// the unscoped read_public_menu availability scan — hide/rewrite B's LIVE storefront. The fix folds
// FK-ownership into the INSERT (INSERT...SELECT ... WHERE EXISTS(products/categories ... AND
// location_id=$1)), 0 rows => 404. DB-free: mirrors the repo's self-bootstrapping authz test pattern.
//
// The stub emulates Postgres faithfully by SQL SHAPE so this is a genuine red->green: the OLD
// `INSERT ... VALUES` always inserts (the foreign FK row exists) => 201 => these tests FAIL; the NEW
// `INSERT ... SELECT ... WHERE EXISTS` returns 0 rows for a foreign FK => 404 => they PASS.

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
const PRODUCT_A = crypto.randomUUID(); // owned by LOC_A
const PRODUCT_B = crypto.randomUUID(); // owned by LOC_B — a DIFFERENT tenant
const CATEGORY_B = crypto.randomUUID(); // owned by LOC_B

function makeDb() {
  const products = new Map([[PRODUCT_A, LOC_A], [PRODUCT_B, LOC_B]]);
  const categories = new Map([[CATEGORY_B, LOC_B]]);
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) return { rowCount: 0, rows: [] };

      if (/INSERT\s+INTO\s+menu_schedules/i.test(sql)) {
        const [locationId, productId, categoryId] = params;
        const guarded = /WHERE/i.test(sql) && /EXISTS/i.test(sql); // NEW fold-in vs OLD plain VALUES
        const ownsProduct = productId == null || products.get(productId) === locationId;
        const ownsCategory = categoryId == null || categories.get(categoryId) === locationId;
        const inserts = guarded ? (ownsProduct && ownsCategory) : true; // OLD SQL always inserts
        return inserts
          ? { rowCount: 1, rows: [{ id: crypto.randomUUID(), product_id: productId, category_id: categoryId, mode: 'daily', start_minute: null, end_minute: null, days_of_week: null, starts_at: null, ends_at: null, available: params[9] }] }
          : { rowCount: 0, rows: [] };
      }
      return { rowCount: 0, rows: [] };
    },
    release() {},
  };
  const db: any = { connect: async () => client };
  return { db, queries };
}

async function buildApp(db: any) {
  ensureEnv();
  const { default: menuAvailabilityRoutes } = await import('../src/routes/owner/menu-availability.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  fastify.setValidatorCompiler(({ schema }) => (data) => {
    const result = (schema as ZodTypeAny).safeParse(data);
    return result.success ? { value: result.data } : { error: result.error as any };
  });
  registerReplySendError(fastify);
  fastify.decorate('db', db);
  fastify.decorate('verifyAuth', async (request: any) => {
    request.user = { userId: OWNER_A, role: 'owner', activeLocationId: LOC_A };
  });
  fastify.decorate('requireRole', () => async () => {});
  fastify.decorate('requireLocationAccess', async () => {});
  await fastify.register(menuAvailabilityRoutes);
  return fastify;
}

test('#15 POST menu-schedule against a tenant-B product → 404, no row created', async () => {
  const { db, queries } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/menu-schedules`,
    payload: { product_id: PRODUCT_B, available: false },
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  const insert = queries.find((q) => /INSERT\s+INTO\s+menu_schedules/i.test(q.sql));
  assert.ok(insert, 'the INSERT is attempted and correctly returns 0 rows for the foreign product');
  await fastify.close();
});

test('#15 POST menu-schedule against a tenant-B category → 404, no row created', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/menu-schedules`,
    payload: { category_id: CATEGORY_B, available: false },
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('#15 POST menu-schedule against own product → 201 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/menu-schedules`,
    payload: { product_id: PRODUCT_A, available: false },
  });
  assert.equal(res.statusCode, 201, `expected 201, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});
