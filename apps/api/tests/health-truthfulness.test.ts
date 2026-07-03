import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';

// #3 — /health truthfulness + no driver-internal leak (DB-free).
// Proves: a Postgres failure/timeout maps to DOWN → HTTP 503 (so external uptime
// monitors page during an outage), and the unauthenticated public payload never
// serializes node-postgres result objects (data: rows incl. oid/dataTypeID) or raw
// driver `detail` text. Soft-check failures stay degraded/200.

async function buildApp(dbQuery: (sql: string) => Promise<any>) {
  const { default: healthRoutes } = await import('../src/routes/health.js');
  const fastify = Fastify();
  const db = { query: (sql: string) => dbQuery(sql) } as any;
  const messageBus = { checkHealth: async () => ({ ok: true }) } as any;
  await fastify.register(healthRoutes, { db, messageBus });
  return fastify;
}

// Everything healthy: SELECT 1 → alive, one fresh/healthy worker heartbeat (H7:
// zero heartbeat rows now honestly means the worker fleet is degraded, not ok —
// see health-worker-status.test.ts), all other checks return empty rows (→ ok).
const healthyDb = async (sql: string) => {
  if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
  if (/ops_worker_heartbeat/i.test(sql)) {
    return {
      rows: [
        { worker_id: 'backup-cron', instance_id: 'a', job_name: null, status: 'healthy', stale_seconds: 5 },
      ],
    };
  }
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
