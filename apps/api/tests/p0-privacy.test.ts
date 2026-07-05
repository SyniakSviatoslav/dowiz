import { test, before, after, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import rateLimit from '@fastify/rate-limit';
import { Pool } from 'pg';

// P0 privacy-hardening proof (ADR-p0-privacy-hardening). Requires local Postgres
// `dowiz_sag` migrated through 1790000000043 AND seeded (packages/db/scripts/seed.ts) —
// the seed provides an on-shift courier + demo location + orders we attach assignments to.
const DB_URL = process.env.SAG_TEST_DB_URL
  || 'postgresql://postgres:postgres@127.0.0.1:5432/dowiz_sag?sslmode=disable';

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test', APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: DB_URL, DATABASE_URL_SESSION: DB_URL, DATABASE_URL_MIGRATIONS: DB_URL,
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'x', JWT_PUBLIC_KEY: 'x', JWT_KID: 'x',
    GOOGLE_CLIENT_ID: 'x', GOOGLE_CLIENT_SECRET: 'x',
    VAPID_PUBLIC_KEY: 'x', VAPID_PRIVATE_KEY: 'x', IP_HASH_SALT: 'x',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}
ensureEnv();

let pool: Pool;
let app: any;
let courierId: string, locationId: string, shiftId: string, orderId: string;
let lat: number, lng: number;
let pingN = 0;

async function buildApp() {
  const { default: courierShiftsRoutes } = await import('../src/routes/courier/shifts.js');
  const f = Fastify();
  await f.register(rateLimit, { max: 10000, timeWindow: '1 minute' });
  f.decorate('verifyAuth', async (request: any) => {
    request.user = { sub: courierId, activeLocationId: locationId, role: 'courier' };
  });
  f.decorate('requireRole', () => async () => {});
  await f.register(courierShiftsRoutes, { db: pool, messageBus: { publish: async () => {} } });
  return f;
}

async function ping() {
  // Distinct Authorization per call → its own per-route rate-limit bucket.
  return app.inject({
    method: 'POST', url: '/shifts/ping',
    headers: { 'content-type': 'application/json', authorization: `Bearer test-${++pingN}` },
    payload: JSON.stringify({ lat, lng }),
  });
}
async function positionCount() {
  const r = await pool.query('SELECT count(*)::int n FROM courier_positions WHERE shift_id=$1', [shiftId]);
  return r.rows[0].n as number;
}

before(async () => {
  pool = new Pool({ connectionString: DB_URL, max: 4 });
  const c = await pool.query(
    `SELECT id, location_id FROM courier_shifts WHERE status IN ('available','on_delivery') ORDER BY started_at DESC LIMIT 1`);
  assert.ok(c.rowCount > 0, 'seed must provide an on-shift courier');
  shiftId = c.rows[0].id; locationId = c.rows[0].location_id;
  const sh = await pool.query('SELECT courier_id FROM courier_shifts WHERE id=$1', [shiftId]);
  courierId = sh.rows[0].courier_id;
  // Minimal order fixture at the shift's location (assignment.order_id is a NOT NULL FK).
  const o = await pool.query(
    `INSERT INTO orders (location_id, subtotal, total, request_hash)
     VALUES ($1, 1000, 1000, 'p0-test-' || gen_random_uuid()::text) RETURNING id`, [locationId]);
  orderId = o.rows[0].id;
  const loc = await pool.query('SELECT lat, lng FROM locations WHERE id=$1', [locationId]);
  lat = Number(loc.rows[0].lat ?? 41.32); lng = Number(loc.rows[0].lng ?? 19.82);
  app = await buildApp();
});
after(async () => {
  await app?.close();
  if (pool) {
    await pool.query('DELETE FROM courier_assignments WHERE order_id=$1', [orderId]).catch((e) => { void e; /* tolerated: best-effort teardown cleanup */ });
    await pool.query('DELETE FROM courier_positions WHERE shift_id=$1', [shiftId]).catch((e) => { void e; /* tolerated: best-effort teardown cleanup */ });
    await pool.query('DELETE FROM orders WHERE id=$1', [orderId]).catch((e) => { void e; /* tolerated: best-effort teardown cleanup */ });
    await pool.end();
  }
});
beforeEach(async () => { await pool.query('DELETE FROM courier_positions WHERE shift_id=$1', [shiftId]); await pool.query('DELETE FROM courier_assignments WHERE order_id=$1', [orderId]); });

async function addAssignment(status: string) {
  await pool.query(
    `INSERT INTO courier_assignments (id, order_id, location_id, courier_id, shift_id, status, assigned_at, cash_collected)
     VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, now(), false)`,
    [orderId, locationId, courierId, shiftId, status]);
}

test('P0-1: ping with NO active assignment → gps_stored:false, ZERO position rows', async () => {
  const res = await ping();
  assert.equal(res.statusCode, 200);
  assert.equal(res.json().gps_stored, false, 'no active delivery → GPS withheld');
  assert.equal(res.json().reason, 'NOT_ON_ACTIVE_DELIVERY');
  assert.equal(await positionCount(), 0, 'no courier_positions row written off-delivery');
});

test('P0-1: ping WITH accepted assignment → gps_stored:true, one row', async () => {
  await addAssignment('accepted');
  const res = await ping();
  assert.equal(res.statusCode, 200);
  assert.equal(res.json().gps_stored, true, 'active delivery (accepted) → GPS stored');
  assert.equal(await positionCount(), 1, 'exactly one position row');
});

test('P0-1: picked_up is active; delivered/assigned are NOT (consent boundary)', async () => {
  await addAssignment('picked_up');
  assert.equal((await ping()).json().gps_stored, true, 'picked_up → stored');
  await pool.query('DELETE FROM courier_positions WHERE shift_id=$1', [shiftId]);

  await pool.query('DELETE FROM courier_assignments WHERE order_id=$1', [orderId]);
  await addAssignment('assigned'); // pre-consent → must NOT track (DEV-3)
  assert.equal((await ping()).json().gps_stored, false, "'assigned' (pre-accept) → NOT tracked");

  await pool.query('DELETE FROM courier_assignments WHERE order_id=$1', [orderId]);
  await addAssignment('delivered'); // finished → must NOT track
  assert.equal((await ping()).json().gps_stored, false, "'delivered' → NOT tracked");
});

test('P0-1: heartbeat updates even when GPS withheld (liveness preserved)', async () => {
  const before = await pool.query('SELECT last_heartbeat_at FROM courier_shifts WHERE id=$1', [shiftId]);
  await new Promise(r => setTimeout(r, 10));
  await ping(); // no assignment → gps withheld
  const after = await pool.query('SELECT last_heartbeat_at FROM courier_shifts WHERE id=$1', [shiftId]);
  assert.ok(new Date(after.rows[0].last_heartbeat_at) >= new Date(before.rows[0].last_heartbeat_at),
    'heartbeat advanced despite GPS being withheld');
});

test('P0-1: GPS purge cron deletes positions older than the retention window', async () => {
  const { CourierCronWorker } = await import('../src/workers/courier-cron.js');
  await addAssignment('accepted');
  await ping(); // writes a fresh row
  // inject an old row directly
  await pool.query(
    `INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, source, recorded_at)
     VALUES ($1,$2,$3,$4,$5,'gps', now() - interval '25 hours')`,
    [courierId, locationId, shiftId, lat, lng]);
  assert.equal(await positionCount(), 2, 'fresh + old');
  const worker: any = new CourierCronWorker(pool, { } as any, { publish: async () => {} } as any);
  await worker.handleGpsPurge();
  assert.equal(await positionCount(), 1, 'old (>24h) purged, fresh kept');
});

test('P0-2: NOT VALID constraint rejects a NEW whatsapp target, accepts telegram', async () => {
  const u = await pool.query('SELECT id FROM users LIMIT 1');
  const userId = u.rows[0].id;
  await assert.rejects(
    () => pool.query(
      `INSERT INTO owner_notification_targets (id, location_id, channel, address, status, prefs, locale, user_id)
       VALUES (gen_random_uuid(), $1, 'whatsapp', '+355690000000', 'active', '{}'::jsonb, 'sq', $2)`, [locationId, userId]),
    /not_whatsapp|check constraint/i,
    'new whatsapp target rejected by owner_notification_targets_not_whatsapp');

  const ok = await pool.query(
    `INSERT INTO owner_notification_targets (id, location_id, channel, address, status, prefs, locale, user_id)
     VALUES (gen_random_uuid(), $1, 'telegram', '999000111', 'active', '{}'::jsonb, 'sq', $2) RETURNING id`, [locationId, userId]);
  assert.equal(ok.rowCount, 1, 'telegram target still allowed');
  await pool.query('DELETE FROM owner_notification_targets WHERE id=$1', [ok.rows[0].id]);
});
