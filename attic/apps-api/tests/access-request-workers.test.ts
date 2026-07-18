import { test, before, after, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import rateLimit from '@fastify/rate-limit';
import { Pool } from 'pg';

// Worker + gate + RLS proof for the soft-access-gate (ADR-soft-access-gate).
// Requires local Postgres `dowiz_sag` migrated through 1790000000042.
const DB_URL = process.env.SAG_TEST_DB_URL
  || 'postgresql://postgres:postgres@127.0.0.1:5432/dowiz_sag?sslmode=disable';

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test', APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: DB_URL, DATABASE_URL_SESSION: DB_URL, DATABASE_URL_MIGRATIONS: DB_URL,
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'test-priv', JWT_PUBLIC_KEY: 'test-pub', JWT_KID: 'test',
    GOOGLE_CLIENT_ID: 'test', GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test', VAPID_PRIVATE_KEY: 'test', IP_HASH_SALT: 'test',
    WAITLIST_NOTIFY_EMAIL: 'ops@example.com', // read at module load by the notify worker
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}
ensureEnv();

let pool: Pool;
const fakeBoss = { send: async () => {}, createQueue: async () => {}, work: async () => {}, schedule: async () => {} } as any;
const fakeBus = { publish: async () => {} } as any;

before(async () => { pool = new Pool({ connectionString: DB_URL, max: 4 }); });
after(async () => { await pool?.end(); });
beforeEach(async () => { await pool.query("DELETE FROM access_requests WHERE email LIKE '%@wkr.test'"); });

async function insertRow(email: string, createdAtSql = 'now()') {
  const r = await pool.query(
    `INSERT INTO access_requests (email, consent_at, privacy_version, created_at)
     VALUES ($1, now(), 'v1', ${createdAtSql}) RETURNING id`, [email]);
  return r.rows[0].id as string;
}

test('B8 claim-before-send: same job twice → exactly ONE send; second claim is a no-op', async () => {
  const { AccessRequestNotifyWorker } = await import('../src/workers/access-request-notify.js');
  let sends = 0;
  const countingAdapter = { sendOps: async () => { sends++; return { delivered: true }; } } as any;
  process.env.WAITLIST_NOTIFY_EMAIL = 'ops@example.com';
  const worker: any = new AccessRequestNotifyWorker(pool, fakeBoss, fakeBus, countingAdapter);

  const id = await insertRow('claim@wkr.test');
  await worker.handle(id);
  await worker.handle(id); // re-delivery (crash-after-send / two-worker race)

  assert.equal(sends, 1, 'at-most-one email even on double delivery');
  const r = await pool.query('SELECT notified_at FROM access_requests WHERE id=$1', [id]);
  assert.ok(r.rows[0].notified_at, 'notified_at set after successful send');
});

test('B8 erasure tolerance: notify on a missing row → ack, no throw', async () => {
  const { AccessRequestNotifyWorker } = await import('../src/workers/access-request-notify.js');
  const worker: any = new AccessRequestNotifyWorker(pool, fakeBoss, fakeBus,
    { sendOps: async () => ({ delivered: true }) } as any);
  await assert.doesNotReject(() => worker.handle('00000000-0000-0000-0000-000000000000'));
});

test('B8 send failure → rolls notified_at back to NULL, bumps notify_attempts, throws', async () => {
  const { AccessRequestNotifyWorker } = await import('../src/workers/access-request-notify.js');
  process.env.WAITLIST_NOTIFY_EMAIL = 'ops@example.com';
  const worker: any = new AccessRequestNotifyWorker(pool, fakeBoss, fakeBus,
    { sendOps: async () => ({ delivered: false, reason: 'HTTP_500' }) } as any);
  const id = await insertRow('fail@wkr.test');
  await assert.rejects(() => worker.handle(id), /send failed/);
  const r = await pool.query('SELECT notified_at, notify_attempts FROM access_requests WHERE id=$1', [id]);
  assert.equal(r.rows[0].notified_at, null, 'claim released on send failure');
  assert.equal(r.rows[0].notify_attempts, 1, 'attempt counter bumped (R2-9 bound)');
});

test('retention sweep: row > 12 months erased; row < 12 months survives', async () => {
  const { AccessRequestRetentionWorker } = await import('../src/workers/access-request-retention.js');
  const worker: any = new AccessRequestRetentionWorker(pool, fakeBoss, fakeBus);
  const oldId = await insertRow('old@wkr.test', "now() - interval '13 months'");
  const newId = await insertRow('new@wkr.test', "now() - interval '11 months'");
  await worker.runRetention();
  const old = await pool.query('SELECT 1 FROM access_requests WHERE id=$1', [oldId]);
  const recent = await pool.query('SELECT 1 FROM access_requests WHERE id=$1', [newId]);
  assert.equal(old.rowCount, 0, '13-month-old row erased');
  assert.equal(recent.rowCount, 1, '11-month-old row survives');
});

test('reconcile: re-enqueues un-notified rows past the grace window, within attempt cap', async () => {
  const { AccessRequestRetentionWorker } = await import('../src/workers/access-request-retention.js');
  const enq: string[] = [];
  const boss = { ...fakeBoss, send: async (_n: string, d: any) => { enq.push(d.requestId); } } as any;
  const worker: any = new AccessRequestRetentionWorker(pool, boss, fakeBus);
  const dueId = await insertRow('due@wkr.test', "now() - interval '1 hour'"); // past 5-min grace
  await insertRow('fresh@wkr.test', 'now()'); // within grace → not re-fed
  // a capped row: exhausted attempts → NOT re-fed
  const capped = await insertRow('capped@wkr.test', "now() - interval '1 hour'");
  await pool.query('UPDATE access_requests SET notify_attempts=10 WHERE id=$1', [capped]);

  await worker.runReconcile();
  assert.deepEqual(enq, [dueId], 'only the due, under-cap, un-notified row is re-enqueued');
});

test('R3-4 route-registration gate: POST 404 when flag off, 200 when on', async () => {
  const { default: accessRequestRoutes } = await import('../src/routes/public/access-requests.js');

  async function buildWithFlag(enabled: boolean) {
    const app = Fastify();
    await app.register(rateLimit, { max: 1000, timeWindow: '1 minute' });
    // mirror server.ts: register ONLY when the flag is on
    if (enabled) {
      await app.register(accessRequestRoutes, { db: pool, queue: { boss: { send: async () => {} } } });
    }
    app.setNotFoundHandler((_req, reply) => reply.status(404).send({ error: 'Not found' }));
    return app;
  }

  const off = await buildWithFlag(false);
  const offRes = await off.inject({ method: 'POST', url: '/api/access-requests',
    headers: { 'content-type': 'application/json', 'fly-client-ip': '1.1.1.1' },
    payload: JSON.stringify({ email: 'x@example.com', consent: true }) });
  assert.equal(offRes.statusCode, 404, 'flag off → route unmounted → 404 (not publicly POST-able pre-gating)');
  await off.close();

  const on = await buildWithFlag(true);
  const onRes = await on.inject({ method: 'POST', url: '/api/access-requests',
    headers: { 'content-type': 'application/json', 'fly-client-ip': '2.2.2.2' },
    payload: JSON.stringify({ email: 'y@example.com', consent: true }) });
  assert.equal(onRes.statusCode, 200, 'flag on → route mounted → 200');
  await on.close();
});

test('RLS: ENABLE+FORCE, single ops policy, and anon/authenticated/service_role hold zero privileges', async () => {
  const rls = await pool.query(`SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE relname='access_requests'`);
  assert.equal(rls.rows[0].relrowsecurity, true, 'RLS enabled');
  assert.equal(rls.rows[0].relforcerowsecurity, true, 'RLS forced');

  const pol = await pool.query(`SELECT polname FROM pg_policy WHERE polrelid='access_requests'::regclass`);
  assert.deepEqual(pol.rows.map((r: any) => r.polname), ['allow_ops_access_requests_all']);

  // Data API perimeter: the Supabase roles must hold NO grants on the table.
  const grants = await pool.query(
    `SELECT grantee, privilege_type FROM information_schema.role_table_grants
      WHERE table_name='access_requests' AND grantee IN ('anon','authenticated','service_role')`);
  assert.equal(grants.rowCount, 0, 'anon/authenticated/service_role revoked (GRANT-layer boundary)');
});
