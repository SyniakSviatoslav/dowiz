import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';

// #3 — /health truthfulness + no driver-internal leak (DB-free).
// Proves: a Postgres failure/timeout maps to DOWN → HTTP 503 (so external uptime
// monitors page during an outage), and the unauthenticated public payload never
// serializes node-postgres result objects (data: rows incl. oid/dataTypeID) or raw
// driver `detail` text. Soft-check failures stay degraded/200.

async function buildApp(
  dbQuery: (sql: string) => Promise<any>,
  busOverride?: { checkHealth: () => Promise<any> },
) {
  const { default: healthRoutes } = await import('../src/routes/health.js');
  const fastify = Fastify();
  const db = { query: (sql: string) => dbQuery(sql) } as any;
  const messageBus = (busOverride ?? { checkHealth: async () => ({ ok: true }) }) as any;
  await fastify.register(healthRoutes, { db, messageBus });
  return fastify;
}

// Everything healthy: SELECT 1 → alive, all other checks return empty rows (→ ok).
const healthyDb = async (sql: string) => {
  if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
  return { rows: [] };
};

test('#3 pg FAST-ERROR → 503 unhealthy, no driver internals leaked', async () => {
  const fastify = await buildApp(async (sql) => {
    if (/SELECT 1 AS alive/i.test(sql)) throw new Error('connection refused: host=10.0.0.1 user=secret');
    return { rows: [] };
  });
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  assert.equal(res.statusCode, 503);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'unhealthy');
  assert.equal(body.checks.postgres.status, 'down');
  // No leak: no raw row objects, no detail string, no driver error text.
  assert.equal(res.body.includes('"data"'), false, 'must not serialize data');
  assert.equal(res.body.includes('"detail"'), false, 'must not serialize detail');
  assert.equal(res.body.includes('oid'), false);
  assert.equal(res.body.includes('connection refused'), false, 'must not leak driver error text');
  assert.equal(res.body.includes('secret'), false);
  await fastify.close();
});

test('#3 pg TIMEOUT → 503 unhealthy (the real bug: timeout was degraded/200)', async () => {
  const fastify = await buildApp(async (sql) => {
    if (/SELECT 1 AS alive/i.test(sql)) return new Promise(() => {}); // never resolves → withTimeout fires
    return { rows: [] };
  });
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  assert.equal(res.statusCode, 503);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'unhealthy');
  assert.equal(body.checks.postgres.status, 'down');
  await fastify.close();
});

test('#3 all healthy → 200, public shape is status+latencyMs only', async () => {
  const fastify = await buildApp(healthyDb);
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  assert.equal(res.statusCode, 200);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'healthy');
  // Every check exposes only { status, latencyMs? } — nothing else.
  for (const [name, check] of Object.entries<any>(body.checks)) {
    const keys = Object.keys(check).sort();
    assert.ok(
      keys.every((k) => k === 'status' || k === 'latencyMs'),
      `check ${name} leaked keys: ${keys.join(',')}`,
    );
  }
  assert.equal(res.body.includes('"data"'), false);
  assert.equal(res.body.includes('"detail"'), false);
  assert.equal(res.body.includes('"entries"'), false, 'worker entries (instance/job ids) must not be public');
  await fastify.close();
});

// #1 — /livez liveness probe: it must answer 200 with { status:'ok', ISO timestamp }, and it
// must NOT touch Postgres (a machine restart can't fix a DB blip and would sever every socket).
test('#1 /livez → 200 ok with ISO timestamp', async () => {
  const fastify = await buildApp(healthyDb);
  const res = await fastify.inject({ method: 'GET', url: '/livez' });
  assert.equal(res.statusCode, 200);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'ok');
  assert.match(body.timestamp, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/);
  assert.equal(Number.isNaN(Date.parse(body.timestamp)), false);
  await fastify.close();
});

test('#1 /livez stays 200 even when Postgres throws (liveness must not query the DB)', async () => {
  const exploding = async () => {
    throw new Error('connection refused: host=10.0.0.1 user=secret');
  };
  const fastify = await buildApp(exploding);
  const res = await fastify.inject({ method: 'GET', url: '/livez' });
  assert.equal(res.statusCode, 200);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'ok');
  // It never queried pg, so no driver text can surface in the liveness response.
  assert.equal(res.body.includes('connection refused'), false);
  assert.equal(res.body.includes('secret'), false);
  await fastify.close();
});

// #2 — messageBus.checkHealth() failure path (previously always stubbed ok). The section is
// labelled "(Degraded)" but the call passes NO treatErrorAsDegraded flag, so a thrown bus error
// maps to 'down' → 503. Pin the ACTUAL contract (Test Integrity #3: assert the exact status).
// contract-flag: comment-intent (Degraded/200) vs behavior (down/503) mismatch — escalate, not weaken here.
test('#2 messageBus.checkHealth throws → 503 unhealthy, bus marked down, no secret leaked', async () => {
  const fastify = await buildApp(healthyDb, {
    checkHealth: async () => {
      throw new Error('redis AUTH failed pw=secret');
    },
  });
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  assert.equal(res.statusCode, 503);
  const body = JSON.parse(res.body);
  assert.equal(body.status, 'unhealthy');
  assert.equal(body.checks.messageBus.status, 'down');
  assert.equal(res.body.includes('secret'), false, 'must not leak bus error text');
  assert.equal(res.body.includes('AUTH'), false);
  await fastify.close();
});

// #3 — BACKUP_ENABLED=true path: the backup_restore check carries extra internal fields
// (last_verified_at/last_result/stale); these MUST be stripped from the unauthenticated public
// payload down to { status } only. Previously the BACKUP_ENABLED=true branch had zero coverage.
test('#3 BACKUP_ENABLED=true → backup_restore public shape is status-only (internal fields stripped)', async () => {
  const prev = process.env.BACKUP_ENABLED;
  process.env.BACKUP_ENABLED = 'true';
  try {
    const recent = new Date().toISOString();
    const dbQuery = async (sql: string) => {
      if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
      if (/backup_audit_log/i.test(sql)) return { rows: [{ last_verified_at: recent, result: true }] };
      return { rows: [] };
    };
    const fastify = await buildApp(dbQuery);
    const res = await fastify.inject({ method: 'GET', url: '/health' });
    const body = JSON.parse(res.body);
    const restore = body.checks.backup_restore;
    assert.ok(restore, 'backup_restore check must be present when BACKUP_ENABLED=true');
    assert.deepEqual(Object.keys(restore).sort(), ['status']);
    assert.equal(restore.status, 'ok');
    // The internal restore-drill fields are never serialized to the public payload.
    assert.equal(res.body.includes('last_verified_at'), false);
    assert.equal(res.body.includes('last_result'), false);
    assert.equal(res.body.includes('"stale"'), false);
    await fastify.close();
  } finally {
    if (prev === undefined) delete process.env.BACKUP_ENABLED;
    else process.env.BACKUP_ENABLED = prev;
  }
});
