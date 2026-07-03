import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import type { ZodTypeAny } from 'zod';

// Sites #5-#10 (audit-fix-authz resolution.md §2/§3.5): owner/products.ts's translation and
// modifier-groups subroutes trusted the child id (:id / group_id) verbatim with no ownership
// predicate — an owner-A token could read/overwrite/delete tenant-B's product_translations and
// wipe+rewrite tenant-B's product_modifier_groups links (a destructive cross-tenant DELETE).
// The fix folds product/group ownership into the statement itself (INSERT...SELECT / JOIN /
// DELETE...USING), 0 rows => 404 (or 400 for a foreign group_id in the modifier-groups sync).
// DB-free: mirrors the repo's self-bootstrapping authz test pattern (fastify.inject + a SQL-aware
// in-memory stub pool) exercised through the REAL route file.

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
const GROUP_A = crypto.randomUUID(); // owned by LOC_A
const GROUP_B = crypto.randomUUID(); // owned by LOC_B

function makeDb() {
  const products = new Map([[PRODUCT_A, LOC_A], [PRODUCT_B, LOC_B]]);
  const modifierGroups = new Map([[GROUP_A, LOC_A], [GROUP_B, LOC_B]]);
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) return { rowCount: 0, rows: [] };

      if (/supported_locales\s+FROM\s+locations/i.test(sql)) {
        return { rowCount: 1, rows: [{ supported_locales: ['en', 'sq'] }] };
      }

      // #5 translations PUT — INSERT ... SELECT p.id ... FROM products p WHERE p.id=$1 AND p.location_id=$5
      if (/INSERT\s+INTO\s+product_translations[\s\S]*SELECT\s+p\.id/i.test(sql)) {
        const [productId, locale, name, description, locationId] = params;
        return products.get(productId) === locationId
          ? { rowCount: 1, rows: [{ product_id: productId, locale, name, description }] }
          : { rowCount: 0, rows: [] };
      }

      // #6 translations GET — SELECT pt.* ... JOIN products p ON p.id=pt.product_id AND p.location_id=$2
      if (/SELECT\s+pt\.\*/i.test(sql)) {
        const [productId, locationId] = params;
        return products.get(productId) === locationId
          ? { rowCount: 1, rows: [{ product_id: productId, locale: 'en', name: 'Existing', description: null }] }
          : { rowCount: 0, rows: [] };
      }

      // #7 translations DELETE — DELETE FROM product_translations pt USING products p WHERE ...
      if (/DELETE\s+FROM\s+product_translations/i.test(sql)) {
        const [productId, locale, locationId] = params;
        return products.get(productId) === locationId
          ? { rowCount: 1, rows: [{ locale }] }
          : { rowCount: 0, rows: [] };
      }

      // #8 modifier-groups PUT — same-tx product-ownership pre-check
      if (/SELECT\s+1\s+FROM\s+products\s+WHERE\s+id\s*=\s*\$1\s+AND\s+location_id\s*=\s*\$2/i.test(sql)) {
        const [productId, locationId] = params;
        return products.get(productId) === locationId
          ? { rowCount: 1, rows: [{ '?column?': 1 }] }
          : { rowCount: 0, rows: [] };
      }

      // The destructive DELETE — must only ever be reached for an OWNED product (pre-check above
      // gates it). Recorded via `queries` so the "B's rows intact" proof can assert it never ran.
      if (/DELETE\s+FROM\s+product_modifier_groups/i.test(sql)) {
        return { rowCount: 1, rows: [] };
      }

      // #9 INSERT product_modifier_groups ... SELECT $1, mg.id, ... FROM modifier_groups mg WHERE mg.id=$2 AND mg.location_id=$4
      if (/INSERT\s+INTO\s+product_modifier_groups/i.test(sql)) {
        const [, groupId, , locationId] = params;
        return modifierGroups.get(groupId) === locationId
          ? { rowCount: 1, rows: [] }
          : { rowCount: 0, rows: [] };
      }

      // #10 GET product modifier-groups
      if (/SELECT\s+pmg\.sort_order/i.test(sql)) {
        const [productId, locationId] = params;
        return products.get(productId) === locationId
          ? { rowCount: 1, rows: [{ sort_order: 0, id: GROUP_A, location_id: locationId }] }
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
  const { default: productRoutes } = await import('../src/routes/owner/products.js');
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
  await fastify.register(productRoutes);
  return fastify;
}

// ─── #5/#6/#7 translations ──────────────────────────────────────────────────

test('#5 PUT translations on tenant-B product → 404 (no upsert)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'PUT',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_B}/translations/en`,
    payload: { name: 'Hacked name' },
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('#5 PUT translations on own product → 200 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'PUT',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_A}/translations/en`,
    payload: { name: 'My product' },
  });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

test('#6 GET translations on tenant-B product → empty (no cross-tenant read)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'GET',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_B}/translations`,
  });
  assert.equal(res.statusCode, 200);
  assert.deepEqual(JSON.parse(res.body).data, []);
  await fastify.close();
});

test('#7 DELETE translation on tenant-B product → 404', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'DELETE',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_B}/translations/en`,
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

// ─── #8/#9 modifier-groups PUT (the destructive-DELETE case) ────────────────

test('#8 PUT modifier-groups on tenant-B product → 404 AND the destructive DELETE never runs (B rows intact)', async () => {
  const { db, queries } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'PUT',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_B}/modifier-groups`,
    payload: [{ group_id: GROUP_B }],
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  const destructiveDelete = queries.find((q) => /DELETE\s+FROM\s+product_modifier_groups/i.test(q.sql));
  assert.equal(destructiveDelete, undefined,
    'the ownership pre-check must gate the DELETE — it must never fire for a foreign product (B rows stay intact)');
  const insert = queries.find((q) => /INSERT\s+INTO\s+product_modifier_groups/i.test(q.sql));
  assert.equal(insert, undefined, 'no new link row may be inserted for a foreign product');
  await fastify.close();
});

test('#9 PUT modifier-groups on OWN product with a foreign (tenant-B) group_id → 400, no row created', async () => {
  const { db, queries } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'PUT',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_A}/modifier-groups`,
    payload: [{ group_id: GROUP_B }],
  });
  assert.equal(res.statusCode, 400, `expected 400, got ${res.statusCode}: ${res.body}`);
  const insert = queries.find((q) => /INSERT\s+INTO\s+product_modifier_groups/i.test(q.sql));
  assert.ok(insert, 'the INSERT is attempted (and correctly returns 0 rows for the foreign group)');
  await fastify.close();
});

test('#8/#9 PUT modifier-groups on own product with own group_id → 200 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'PUT',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_A}/modifier-groups`,
    payload: [{ group_id: GROUP_A }],
  });
  assert.equal(res.statusCode, 200, `expected 200, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});

// ─── #10 GET modifier-groups ─────────────────────────────────────────────────

test('#10 GET modifier-groups on tenant-B product → empty (no cross-tenant read)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'GET',
    url: `/api/owner/locations/${LOC_A}/products/${PRODUCT_B}/modifier-groups`,
  });
  assert.equal(res.statusCode, 200);
  assert.deepEqual(JSON.parse(res.body).data, []);
  await fastify.close();
});
