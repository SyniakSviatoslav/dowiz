import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';

// H7 (2026-07-03 reliability audit) — the `workers` aggregate on /health used to be
// hardcoded `status: 'ok'` regardless of what the ops_worker_heartbeat query actually
// returned: a fully-dead worker fleet, or a heartbeat query that itself failed, still
// reported the fleet healthy. This proves the aggregate is now DERIVED from the real
// heartbeat rows: dead/absent/query-failed → degraded, all-healthy → ok. Also covers
// the companion bug where `backup_restore` treated "never run a restore drill" as 'ok'.

async function buildApp(dbQuery: (sql: string) => Promise<any>) {
  const { default: healthRoutes } = await import('../src/routes/health.js');
  const fastify = Fastify();
  const db = { query: (sql: string) => dbQuery(sql) } as any;
  const messageBus = { checkHealth: async () => ({ ok: true }) } as any;
  await fastify.register(healthRoutes, { db, messageBus });
  return fastify;
}

// Baseline: every non-worker check returns empty rows (→ ok); postgres is alive.
function baseDb(workerRows: any[]) {
  return async (sql: string) => {
    if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
    if (/ops_worker_heartbeat/i.test(sql)) return { rows: workerRows };
    return { rows: [] };
  };
}

test('#H7 fully-dead worker fleet (zero live heartbeats) → workers degraded, NOT ok', async () => {
  const fastify = await buildApp(baseDb([])); // no rows within the last 60s = every worker dead/absent
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  const body = JSON.parse(res.body);
  assert.equal(body.checks.workers.status, 'degraded', 'a dead worker fleet must not report ok');
  assert.equal(body.status, 'degraded');
  assert.equal(res.statusCode, 200); // degraded, not down — don't 503 the whole app on stale heartbeats
  await fastify.close();
});

test('#H7 heartbeat query itself fails/times out → workers degraded, NOT ok', async () => {
  const fastify = await buildApp(async (sql: string) => {
    if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
    if (/ops_worker_heartbeat/i.test(sql)) throw new Error('relation "ops_worker_heartbeat" query failed');
    return { rows: [] };
  });
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  const body = JSON.parse(res.body);
  assert.equal(body.checks.workers.status, 'degraded', 'an unreadable heartbeat signal must not report ok');
  await fastify.close();
});

test('#H7 one stale/unhealthy worker among several → workers degraded', async () => {
  const fastify = await buildApp(
    baseDb([
      { worker_id: 'backup-cron', instance_id: 'a', job_name: null, status: 'healthy', stale_seconds: 5 },
      { worker_id: 'order-timeout-sweep', instance_id: 'a', job_name: null, status: 'stale', stale_seconds: 400 },
    ]),
  );
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  const body = JSON.parse(res.body);
  assert.equal(body.checks.workers.status, 'degraded');
  await fastify.close();
});

test('#H7 GREEN: all workers healthy & fresh → workers ok', async () => {
  const fastify = await buildApp(
    baseDb([
      { worker_id: 'backup-cron', instance_id: 'a', job_name: null, status: 'healthy', stale_seconds: 5 },
      { worker_id: 'order-timeout-sweep', instance_id: 'a', job_name: null, status: 'healthy', stale_seconds: 10 },
    ]),
  );
  const res = await fastify.inject({ method: 'GET', url: '/health' });
  const body = JSON.parse(res.body);
  assert.equal(body.checks.workers.status, 'ok');
  assert.equal(body.status, 'healthy');
  assert.equal(res.statusCode, 200);
  await fastify.close();
});

test('#H7 backup restore-test NEVER run → backup_restore degraded, NOT ok', async () => {
  const prevEnabled = process.env.BACKUP_ENABLED;
  process.env.BACKUP_ENABLED = 'true';
  try {
    const fastify = await buildApp(async (sql: string) => {
      if (/SELECT 1 AS alive/i.test(sql)) return { rows: [{ alive: 1 }] };
      if (/ops_worker_heartbeat/i.test(sql)) return { rows: [] };
      if (/backup_audit_log/i.test(sql)) {
        return { rows: [{ last_verified_at: null, result: null }] };
      }
      if (/backup_metadata/i.test(sql)) return { rows: [] };
      if (/R2_ENDPOINT|R2_BUCKET/i.test(sql)) return { rows: [] };
      return { rows: [] };
    });
    const res = await fastify.inject({ method: 'GET', url: '/health' });
    const body = JSON.parse(res.body);
    assert.equal(
      body.checks.backup_restore.status,
      'degraded',
      'a system that has never restore-tested must not report ok',
    );
    await fastify.close();
  } finally {
    if (prevEnabled === undefined) delete process.env.BACKUP_ENABLED;
    else process.env.BACKUP_ENABLED = prevEnabled;
  }
});
