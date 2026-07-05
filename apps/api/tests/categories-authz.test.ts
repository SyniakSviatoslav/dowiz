import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import type { ZodTypeAny } from 'zod';

// Site #12 (audit-fix-authz resolution.md §2): owner/categories.ts DELETE's "does this category
// have products" pre-check queried `products WHERE category_id=$1` with no location_id predicate —
// an owner-A token could distinguish "tenant-B category has products" (409) from "category doesn't
// exist" (404) purely from the status code, an existence oracle across tenants. The fix adds
// `AND location_id=$2` to the pre-check so a foreign category always reads as "no products" and
// falls through to the (already tenant-scoped) DELETE, which 404s. Covers both call sites: the
// :locationId path route and the JWT-derived menu-alias route. DB-free: mirrors the repo's
// self-bootstrapping authz test pattern.

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
const CATEGORY_B_WITH_PRODUCTS = crypto.randomUUID(); // owned by LOC_B, has products
const CATEGORY_B_EMPTY = crypto.randomUUID(); // owned by LOC_B, no products
const CATEGORY_A = crypto.randomUUID(); // owned by LOC_A, has products

function makeDb() {
  const categoriesWithProducts = new Set([CATEGORY_B_WITH_PRODUCTS, CATEGORY_A]);
  const categoryLocation = new Map([
    [CATEGORY_B_WITH_PRODUCTS, LOC_B],
    [CATEGORY_B_EMPTY, LOC_B],
    [CATEGORY_A, LOC_A],
  ]);
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) return { rowCount: 0, rows: [] };

      // The existence-oracle pre-check. Pre-fix this queried `category_id=$1` ALONE (1 param) —
      // ground truth "has products" leaks regardless of tenant (the oracle). Post-fix it also
      // binds `location_id=$2` (2 params) — a foreign category must read as "no products."
      if (/SELECT\s+id\s+FROM\s+products\s+WHERE\s+category_id/i.test(sql)) {
        const [categoryId, locationId] = params;
        const hasProductsAtAll = categoriesWithProducts.has(categoryId);
        const matches = params.length >= 2
          ? hasProductsAtAll && categoryLocation.get(categoryId) === locationId
          : hasProductsAtAll;
        return matches ? { rowCount: 1, rows: [{ id: 'p1' }] } : { rowCount: 0, rows: [] };
      }

      // The (already tenant-scoped) DELETE — both call sites bind (id, locationId), just in
      // different param order depending on which path built the query.
      if (/DELETE\s+FROM\s+categories/i.test(sql)) {
        const idParam = /WHERE\s+location_id\s*=\s*\$1\s+AND\s+id\s*=\s*\$2/i.test(sql) ? params[1] : params[0];
        const locParam = /WHERE\s+location_id\s*=\s*\$1\s+AND\s+id\s*=\s*\$2/i.test(sql) ? params[0] : params[1];
        return categoryLocation.get(idParam) === locParam
          ? { rowCount: 1, rows: [{ id: idParam }] }
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
  const { default: categoryRoutes } = await import('../src/routes/owner/categories.js');
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
  await fastify.register(categoryRoutes);
  return fastify;
}

test('#12 DELETE tenant-B category WITH products, via owner-A token → 404 (NOT 409 — oracle closed)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'DELETE',
    url: `/api/owner/locations/${LOC_A}/categories/${CATEGORY_B_WITH_PRODUCTS}`,
  });
  assert.equal(res.statusCode, 404,
    `expected 404 (same as a nonexistent category — oracle closed), got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('#12 DELETE tenant-B category WITHOUT products, via owner-A token → 404 (same status as WITH products)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'DELETE',
    url: `/api/owner/locations/${LOC_A}/categories/${CATEGORY_B_EMPTY}`,
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('#12 DELETE own category WITH products → 409 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'DELETE',
    url: `/api/owner/locations/${LOC_A}/categories/${CATEGORY_A}`,
  });
  assert.equal(res.statusCode, 409, `expected 409, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});
