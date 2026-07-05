import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import crypto from 'node:crypto';
import { isVenueOpen } from '../src/lib/venue-open.js';

// Closed-venue order gate (audit remediation).
//
//  • Helper (lib/venue-open.ts) is the PURE server-side mirror of the storefront's open/closed
//    computation (public/menu.ts:335-358). These cases hand-pick an INDEPENDENT clock + hours
//    window (never derived from the code under test) so a semantic drift from menu.ts goes RED.
//  • Gate (POST /orders) refuses a closed venue with 409 VENUE_CLOSED ONLY when the reversible
//    flag ENFORCE_VENUE_HOURS='true'; flag OFF ⇒ unchanged (order proceeds past the gate).
//    DB-free: scripted pg stub, same harness as orders-status-patch-guards.test.ts.

// A concrete Monday (2026-07-06). new Date(y, monthIndex, d, h, m) is ALWAYS local time, so
// getDay()/getHours()/getMinutes() are self-consistent regardless of the runner's timezone —
// matching the helper's (and menu.ts's) server-local semantics.
const MON = (h: number, m: number) => new Date(2026, 6, 6, h, m, 0); // monthIndex 6 = July; the 6th is a Monday
const OPEN_9_TO_22 = { monday: { isOpen: true, open: '09:00', close: '22:00' } };
const MON_CLOSED = { monday: { isOpen: false } };
// Every day closed → the venue is closed whatever new Date().getDay() returns (clock-independent).
const ALWAYS_CLOSED = Object.fromEntries(
  ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'].map((d) => [d, { isOpen: false }]),
);

// ─────────────────────────── Helper: pure parity ───────────────────────────

test('helper: a day flagged isOpen:false is CLOSED, even mid-afternoon', () => {
  assert.equal(isVenueOpen(MON_CLOSED, false, MON(14, 30)), false);
});

test('helper: within the open..close window is OPEN', () => {
  // 14:30 sits strictly inside 09:00..22:00 → open (independent expected value).
  assert.equal(isVenueOpen(OPEN_9_TO_22, false, MON(14, 30)), true);
});

test('helper: before opening is CLOSED', () => {
  // 08:00 < 09:00 open.
  assert.equal(isVenueOpen(OPEN_9_TO_22, false, MON(8, 0)), false);
});

test('helper: after closing is CLOSED (and exactly-at-close is CLOSED — window is [open, close))', () => {
  assert.equal(isVenueOpen(OPEN_9_TO_22, false, MON(22, 30)), false);
  assert.equal(isVenueOpen(OPEN_9_TO_22, false, MON(22, 0)), false); // nowMins < closeMins is strict
});

test('helper: delivery_paused forces CLOSED even inside the open window', () => {
  assert.equal(isVenueOpen(OPEN_9_TO_22, true, MON(14, 30)), false);
});

test('helper: missing/malformed hours_json ⇒ OPEN unless delivery_paused (mirrors menu.ts catch/guard)', () => {
  assert.equal(isVenueOpen(null, false, MON(14, 30)), true);          // no hours at all
  assert.equal(isVenueOpen(null, true, MON(14, 30)), false);         // …but paused still wins
  assert.equal(isVenueOpen('garbage-not-an-object', false, MON(3, 0)), true); // truthy non-object → day undefined → open
  assert.equal(isVenueOpen({}, false, MON(3, 0)), true);              // object, no day key → open
  assert.equal(isVenueOpen({ monday: 'nope' }, false, MON(14, 30)), true); // day is not an object → open
});

test('helper: a BUSY venue (kitchen_busy_until in the future) is still OPEN for ordering (busy ≠ closed)', () => {
  // kitchen_busy_until is NOT an input to the open/closed gate — an open-by-hours venue stays
  // orderable while the kitchen is merely busy. Same as menu.ts, where `busy` is a distinct
  // status layered ON TOP of isOpen=true.
  assert.equal(isVenueOpen(OPEN_9_TO_22, false, MON(14, 30)), true);
});

// ─────────────────────────── Gate: POST /orders ───────────────────────────

const OWNER_ID = crypto.randomUUID();
const LOCATION_ID = crypto.randomUUID();
const PRODUCT_ID = crypto.randomUUID();

function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test',
    APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
    DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
    REDIS_URL: 'redis://localhost:6379',
    JWT_PRIVATE_KEY: 'test-priv',
    JWT_PUBLIC_KEY: 'test-pub',
    JWT_KID: 'test',
    GOOGLE_CLIENT_ID: 'test',
    GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test',
    VAPID_PRIVATE_KEY: 'test',
    IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}

// A published, CLOSED location row (all days closed → closed regardless of wall-clock).
const CLOSED_LOCATION = {
  lat: 41.3, lng: 19.8, confirm_timeout_min: 10, busy_mode: false, phone: null, slug: 'demo',
  published_at: new Date('2026-01-01T00:00:00Z').toISOString(),
  currency_code: 'ALL', currency_minor_unit: 2, tax_rate: 0, price_includes_tax: true,
  min_order_value: 0, free_delivery_threshold: null, delivery_fee_flat: 0,
  require_phone_otp: false, hours_json: ALWAYS_CLOSED, delivery_paused: false,
};

function scriptedQuery(issued: Array<{ sql: string; params: unknown[] }>) {
  return async (sql: string, params: unknown[] = []) => {
    const s = String(sql);
    issued.push({ sql: s, params });
    if (/^(BEGIN|COMMIT|ROLLBACK)$/i.test(s.trim()) || /SET LOCAL|set_config|SAVEPOINT|RELEASE/i.test(s)) {
      return { rows: [], rowCount: 0 };
    }
    // The create handler's location config read (the SELECT that carries hours_json/delivery_paused).
    if (/hours_json/i.test(s) && /FROM locations/i.test(s)) {
      return { rowCount: 1, rows: [CLOSED_LOCATION] };
    }
    if (/SELECT require_phone_otp FROM locations/i.test(s)) {
      return { rowCount: 1, rows: [{ require_phone_otp: false }] };
    }
    // First DB read PAST the gate — its presence proves the gate did not short-circuit.
    if (/FROM menu_versions/i.test(s)) {
      return { rowCount: 1, rows: [{ version: '1' }] };
    }
    if (/FROM products/i.test(s)) return { rowCount: 0, rows: [] };
    if (/FROM velocity_events/i.test(s)) return { rowCount: 1, rows: [{ cnt: 0 }] };
    return { rowCount: 0, rows: [] };
  };
}

async function buildApp(issued: Array<{ sql: string; params: unknown[] }>) {
  ensureEnv();
  const { default: orderRoutes } = await import('../src/routes/orders.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const fastify = Fastify();
  registerReplySendError(fastify);
  fastify.decorate('verifyAuth', async (req: any) => { req.user = { role: 'owner', userId: OWNER_ID }; });
  fastify.decorate('softVerifyAuth', async (_req: any) => {});
  fastify.decorate('requireRole', () => async () => {});
  const q = scriptedQuery(issued);
  const client = { query: (sql: string, params?: unknown[]) => q(sql, params), release() {} };
  const db = { connect: async () => client, query: (sql: string, params?: unknown[]) => q(sql, params) } as any;
  await fastify.register(orderRoutes, {
    prefix: '/api',
    db,
    messageBus: { publish: async () => {} } as any,
    queue: { enqueue: async () => 'job', work: async () => {}, start: async () => {}, stop: async () => {} } as any,
  });
  return fastify;
}

function payload() {
  return {
    locationId: LOCATION_ID,
    type: 'pickup',
    items: [{ product_id: PRODUCT_ID, quantity: 1 }],
    payment: { method: 'cash' },
    idempotency_key: crypto.randomUUID(),
  };
}

test('gate ON + closed venue → 409 VENUE_CLOSED, rolled back, no work past the gate', async () => {
  const prev = process.env.ENFORCE_VENUE_HOURS;
  process.env.ENFORCE_VENUE_HOURS = 'true';
  try {
    const issued: Array<{ sql: string; params: unknown[] }> = [];
    const app = await buildApp(issued);
    const res = await app.inject({ method: 'POST', url: '/api/orders', payload: payload() });
    assert.equal(res.statusCode, 409, `expected VENUE_CLOSED; body=${res.body}`);
    assert.equal(res.json().code, 'VENUE_CLOSED');
    assert.ok(issued.some((x) => /^ROLLBACK$/i.test(x.sql.trim())), 'the write tx must be rolled back on refusal');
    assert.ok(!issued.some((x) => /FROM menu_versions/i.test(x.sql)), 'the gate must return BEFORE any downstream read/write');
    await app.close();
  } finally {
    if (prev === undefined) delete process.env.ENFORCE_VENUE_HOURS; else process.env.ENFORCE_VENUE_HOURS = prev;
  }
});

test('gate OFF (flag unset) + closed venue → order proceeds PAST the gate (unchanged behavior)', async () => {
  const prev = process.env.ENFORCE_VENUE_HOURS;
  delete process.env.ENFORCE_VENUE_HOURS;
  try {
    const issued: Array<{ sql: string; params: unknown[] }> = [];
    const app = await buildApp(issued);
    const res = await app.inject({ method: 'POST', url: '/api/orders', payload: payload() });
    assert.notEqual(res.json()?.code, 'VENUE_CLOSED', `flag OFF must never emit VENUE_CLOSED; body=${res.body}`);
    assert.ok(
      issued.some((x) => /FROM menu_versions/i.test(x.sql)),
      'flag OFF must fall through the gate into the normal create flow (menu_versions read)',
    );
    await app.close();
  } finally {
    if (prev === undefined) delete process.env.ENFORCE_VENUE_HOURS; else process.env.ENFORCE_VENUE_HOURS = prev;
  }
});
