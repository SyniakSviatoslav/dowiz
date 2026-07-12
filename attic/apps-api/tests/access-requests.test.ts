import { test, before, after, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import rateLimit from '@fastify/rate-limit';
import crypto from 'node:crypto';
import { Pool } from 'pg';

// Integration proof for the soft-access-gate route + workers (ADR-soft-access-gate).
// Requires a local Postgres `dowiz_sag` migrated through 1790000000042 (the test harness
// migrates it). Sets env BEFORE importing the route (loadEnv runs at module load).
const DB_URL = process.env.SAG_TEST_DB_URL
  || 'postgresql://postgres:postgres@127.0.0.1:5432/dowiz_sag?sslmode=disable';

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    APP_BASE_URL: 'http://localhost:3000',
    ***REDACTED***: DB_URL,
    ***REDACTED***: DB_URL,
    ***REDACTED***: DB_URL,
    REDIS_URL: 'redis://localhost:6379',
    ***REDACTED***: 'test-priv',
    ***REDACTED***: 'test-pub',
    JWT_KID: 'test',
    ***REDACTED***: 'test',
    ***REDACTED***: 'test',
    VAPID_PUBLIC_KEY: 'test',
    VAPID_PRIVATE_KEY: 'test',
    IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}
ensureEnv();

const PRIVACY_VERSION = '2026-06-20'; // matches config default PRIVACY_NOTICE_VERSION

let pool: Pool;
const sends: Array<{ name: string; data: any }> = [];

function hashIp(ip: string) {
  return crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
}

async function buildApp() {
  const { default: accessRequestRoutes } = await import('../src/routes/public/access-requests.js');
  const fastify = Fastify();
  await fastify.register(rateLimit, { max: 1000, timeWindow: '1 minute' });
  const fakeQueue = { boss: { send: async (name: string, data: any) => { sends.push({ name, data }); } } };
  await fastify.register(accessRequestRoutes, { db: pool, queue: fakeQueue });
  return fastify;
}

let app: any;
let ipCounter = 0;
function freshIp() { return `203.0.113.${++ipCounter % 250}`; }

// Each request gets a distinct Fly-Client-IP so the per-route 5/min limiter buckets
// independently (and so we exercise the Fly-Client-IP path, not request.ip).
function post(body: any, extraHeaders: Record<string, string> = {}) {
  return app.inject({
    method: 'POST',
    url: '/api/access-requests',
    headers: { 'content-type': 'application/json', 'fly-client-ip': freshIp(), ...extraHeaders },
    payload: JSON.stringify(body),
  });
}

before(async () => {
  pool = new Pool({ connectionString: DB_URL, max: 4 });
  app = await buildApp();
});
after(async () => { await app?.close(); await pool?.end(); });
beforeEach(async () => { sends.length = 0; await pool.query("DELETE FROM access_requests WHERE email LIKE '%@example.com'"); });

async function count(email?: string) {
  const r = email
    ? await pool.query('SELECT count(*)::int n FROM access_requests WHERE email=$1', [email])
    : await pool.query("SELECT count(*)::int n FROM access_requests WHERE email LIKE '%@example.com'");
  return r.rows[0].n as number;
}

test('consent:true → 200, row with consent_at + privacy_version + ip_hash, one enqueue', async () => {
  const fly = '198.51.100.7';
  const res = await app.inject({
    method: 'POST', url: '/api/access-requests',
    headers: { 'content-type': 'application/json', 'fly-client-ip': fly, 'x-forwarded-for': '1.2.3.4' },
    payload: JSON.stringify({ email: '  Alice@Example.COM ', consent: true, locale: 'en' }),
  });
  assert.equal(res.statusCode, 200);
  assert.deepEqual(res.json(), { ok: true });

  const r = await pool.query("SELECT email, consent_at, privacy_version, ip_hash, locale FROM access_requests WHERE email LIKE '%@example.com'");
  assert.equal(r.rowCount, 1);
  assert.equal(r.rows[0].email, 'alice@example.com', 'email trim+lowercased');
  assert.ok(r.rows[0].consent_at, 'consent_at set');
  assert.equal(r.rows[0].privacy_version, PRIVACY_VERSION);
  assert.equal(r.rows[0].locale, 'en');
  // B2: ip_hash derives from Fly-Client-IP, NEVER from the spoofed X-Forwarded-For.
  assert.equal(r.rows[0].ip_hash, hashIp(fly), 'ip_hash from Fly-Client-IP');
  assert.notEqual(r.rows[0].ip_hash, hashIp('1.2.3.4'), 'ip_hash NOT from X-Forwarded-For');

  assert.equal(sends.length, 1, 'one enqueue on a new row');
  assert.equal(sends[0].name, 'access-request.notify');
  assert.deepEqual(Object.keys(sends[0].data), ['requestId'], 'queue payload is {requestId} only — zero PII');
});

test('duplicate email → 200, NO second row, NO second enqueue (anti-enumeration + idempotent)', async () => {
  const first = await post({ email: 'dup@example.com', consent: true });
  const second = await post({ email: 'DUP@example.com', consent: true });
  assert.equal(first.statusCode, 200);
  assert.equal(second.statusCode, 200);
  assert.deepEqual(first.json(), second.json(), 'byte-identical body new vs duplicate');
  assert.equal(await count('dup@example.com'), 1, 'no second row');
  assert.equal(sends.length, 1, 'no second enqueue');
});

test('consent gate (R2-3/R2-8): missing/false/"true"/1 → 200 + NO row, byte-identical to a real 200', async () => {
  const real = await post({ email: 'ok@example.com', consent: true });
  const realBody = real.json();
  await pool.query("DELETE FROM access_requests WHERE email LIKE '%@example.com'"); sends.length = 0;

  for (const bad of [
    { email: 'a@example.com' },                    // missing
    { email: 'b@example.com', consent: false },    // false
    { email: 'c@example.com', consent: 'true' },   // truthy STRING — z.literal(true) must reject
    { email: 'd@example.com', consent: 1 },        // truthy number
  ]) {
    const res = await post(bad);
    assert.equal(res.statusCode, 200, `${JSON.stringify(bad)} → 200 (no 400)`);
    assert.deepEqual(res.json(), realBody, 'no-consent body byte-identical to real 200');
  }
  assert.equal(await count(), 0, 'no row written without literal consent:true');
  assert.equal(sends.length, 0, 'no enqueue without consent');
});

test('honeypot filled → 200 + NO row', async () => {
  const res = await post({ email: 'bot@example.com', consent: true, website: 'http://spam' });
  assert.equal(res.statusCode, 200);
  assert.equal(await count(), 0, 'honeypot → no insert');
  assert.equal(sends.length, 0);
});

test('malformed email (consent true) → 200 + NO row (never a 400)', async () => {
  const res = await post({ email: 'not-an-email', consent: true });
  assert.equal(res.statusCode, 200);
  assert.equal(await count(), 0);
});

test('prod with NO Fly-Client-IP → fails closed to a shared bucket (never trusts XFF)', async () => {
  // clientIp() is pure-ish; verify directly in a prod-like env.
  const mod = await import('../src/routes/public/access-requests.js');
  const prevEnv = process.env.NODE_ENV;
  process.env.NODE_ENV = 'production';
  try {
    const key = (mod as any).clientIp({ headers: { 'x-forwarded-for': '9.9.9.9' }, ip: '7.7.7.7', log: { warn() {} } });
    assert.equal(key, 'shared:no-fly-ip', 'no Fly-Client-IP in prod → shared bucket, NOT the XFF');
    const key2 = (mod as any).clientIp({ headers: { 'fly-client-ip': '5.5.5.5', 'x-forwarded-for': '9.9.9.9' }, ip: '7.7.7.7' });
    assert.equal(key2, '5.5.5.5', 'Fly-Client-IP used when present');
  } finally {
    process.env.NODE_ENV = prevEnv;
  }
});
