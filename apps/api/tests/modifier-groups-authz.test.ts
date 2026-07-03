import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import type { ZodTypeAny } from 'zod';

// Site #11 (audit-fix-authz resolution.md §2): owner/modifier-groups.ts POST .../modifiers trusted
// the :groupId path param verbatim with no ownership predicate — an owner-A token could inject a
// modifier row into a competitor's (tenant-B's) modifier_groups row via a single-column FK. The
// fix folds group ownership into the INSERT (INSERT...SELECT ... FROM modifier_groups mg WHERE
// mg.id=$2 AND mg.location_id=$1), 0 rows => 404. Companion: the GET list's modifier_count join
// gains `AND m.location_id = mg.location_id` so a historically-injected foreign row can't inflate
// the count. DB-free: mirrors the repo's self-bootstrapping authz test pattern.

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
const GROUP_A = crypto.randomUUID(); // owned by LOC_A
const GROUP_B = crypto.randomUUID(); // owned by LOC_B — a DIFFERENT tenant

function makeDb() {
  const groups = new Map([[GROUP_A, LOC_A], [GROUP_B, LOC_B]]);
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql) || /set_config/i.test(sql)) return { rowCount: 0, rows: [] };

      // #11 POST modifiers — INSERT ... SELECT $1, mg.id, ... FROM modifier_groups mg WHERE mg.id=$2 AND mg.location_id=$1
      if (/INSERT\s+INTO\s+modifiers/i.test(sql)) {
        const [locationId, groupId] = params;
        return groups.get(groupId) === locationId
          ? { rowCount: 1, rows: [{ id: crypto.randomUUID(), group_id: groupId, name: 'x', price_delta: 0, available: true, sort_order: 0 }] }
          : { rowCount: 0, rows: [] };
      }

      // GET list — modifier_count join must be location-scoped too (companion fix).
      if (/SELECT\s+mg\.\*/i.test(sql)) {
        return { rowCount: 0, rows: [] };
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
  const { default: modifierGroupRoutes } = await import('../src/routes/owner/modifier-groups.js');
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
  await fastify.register(modifierGroupRoutes);
  return fastify;
}

test('#11 POST modifiers into a tenant-B group → 404, no row created', async () => {
  const { db, queries } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/modifier-groups/${GROUP_B}/modifiers`,
    payload: { name: 'Extra cheese', price_delta: 100 },
  });
  assert.equal(res.statusCode, 404, `expected 404, got ${res.statusCode}: ${res.body}`);
  const insert = queries.find((q) => /INSERT\s+INTO\s+modifiers/i.test(q.sql));
  assert.ok(insert, 'the INSERT is attempted (and correctly returns 0 rows for the foreign group)');
  await fastify.close();
});

test('#11 POST modifiers into own group → 201 (no regression)', async () => {
  const { db } = makeDb();
  const fastify = await buildApp(db);
  const res = await fastify.inject({
    method: 'POST',
    url: `/api/owner/locations/${LOC_A}/modifier-groups/${GROUP_A}/modifiers`,
    payload: { name: 'Extra cheese', price_delta: 100 },
  });
  assert.equal(res.statusCode, 201, `expected 201, got ${res.statusCode}: ${res.body}`);
  await fastify.close();
});
