import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  courierCanAccessRoom,
  courierCanReadOrder,
  courierCanSendOrder,
  courierReadVerdict,
  courierRoomVerdict,
  BINDING_READ_STATUSES,
  BINDING_SEND_STATUSES,
} from '../src/lib/courier-room-authz.js';

// ADR-0013 guardrail. Mock Pool: records every query, returns a configurable rowCount for the
// binding SELECT. No live DB — this pins the AUTHZ LOGIC (the E2E pins the live RLS path).
function mockPool(opts: { rows?: number; throwOn?: 'connect' | 'begin' | 'select' } = {}) {
  const q: { sql: string; params?: any[] }[] = [];
  let released = false;
  const client = {
    async query(sql: string, params?: any[]) {
      q.push({ sql, params });
      if (opts.throwOn === 'begin' && /BEGIN/.test(sql)) throw new Error('begin boom');
      if (opts.throwOn === 'select' && /courier_assignments/.test(sql)) throw new Error('select boom');
      if (/courier_assignments/.test(sql)) return { rowCount: opts.rows ?? 0 };
      return { rowCount: 0 };
    },
    release() { released = true; },
  };
  const pool: any = {
    connected: false,
    async connect() {
      if (opts.throwOn === 'connect') throw new Error('connect boom');
      pool.connected = true;
      return client;
    },
  };
  return { pool, queries: q, wasReleased: () => released };
}

const SUB = 'courier-A';
const LOC = 'loc-1';

test('location:* is denied for couriers with ZERO db access (owner dashboard feed)', async () => {
  const { pool, queries } = mockPool({ rows: 99 });
  assert.equal(await courierCanAccessRoom(pool, SUB, LOC, 'location:other-tenant'), false);
  assert.equal(await courierCanAccessRoom(pool, SUB, LOC, 'location:loc-1'), false);
  assert.equal(pool.connected, false, 'must not touch the DB to deny a location room');
  assert.equal(queries.length, 0);
});

test('malformed / non-order rooms are denied with no db access', async () => {
  const { pool } = mockPool({ rows: 1 });
  for (const room of ['shift:x', 'order:', 'garbage', 'courier:other']) {
    assert.equal(await courierCanAccessRoom(pool, SUB, LOC, room), false, room);
  }
  assert.equal(pool.connected, false);
});

test('order room: bound courier is ALLOWED and the query runs under tenant context (NOBYPASSRLS-sound)', async () => {
  const { pool, queries, wasReleased } = mockPool({ rows: 1 });
  assert.equal(await courierCanAccessRoom(pool, SUB, LOC, 'order:ord-9'), true);
  const sqls = queries.map((x) => x.sql);
  assert.ok(sqls.some((s) => /BEGIN/.test(s)), 'must open a tx');
  const setCfg = queries.find((x) => /set_config/.test(x.sql));
  assert.ok(setCfg && /app\.current_tenant/.test(setCfg.sql), 'must set app.current_tenant');
  assert.equal(setCfg!.params?.[0], LOC, 'tenant = activeLocationId');
  assert.ok(sqls.some((s) => /COMMIT/.test(s)), 'must commit');
  assert.ok(wasReleased(), 'must release the client');
});

test('order room: unbound courier is DENIED', async () => {
  const { pool } = mockPool({ rows: 0 });
  assert.equal(await courierCanAccessRoom(pool, SUB, LOC, 'order:ord-9'), false);
});

test('missing activeLocationId → denied, no db access (cannot scope tenant)', async () => {
  const { pool } = mockPool({ rows: 1 });
  assert.equal(await courierCanReadOrder(pool, SUB, undefined, 'ord-9'), false);
  assert.equal(pool.connected, false);
});

test('fail CLOSED on db errors, and the client is still released', async () => {
  for (const throwOn of ['connect', 'begin', 'select'] as const) {
    const { pool, wasReleased } = mockPool({ rows: 1, throwOn });
    assert.equal(await courierCanReadOrder(pool, SUB, LOC, 'ord-9'), false, `throwOn=${throwOn}`);
    if (throwOn !== 'connect') assert.ok(wasReleased(), `released after ${throwOn}`);
  }
});

test('read includes offered (offer-handshake); send excludes offered (read-but-not-speak)', async () => {
  assert.ok(BINDING_READ_STATUSES.includes('offered'));
  assert.ok(!BINDING_SEND_STATUSES.includes('offered'));
  assert.ok(BINDING_SEND_STATUSES.includes('assigned'));
  // send path uses the stricter status set
  const { pool, queries } = mockPool({ rows: 1 });
  await courierCanSendOrder(pool, SUB, LOC, 'ord-9');
  const sel = queries.find((x) => /courier_assignments/.test(x.sql));
  assert.deepEqual(sel!.params?.[2], BINDING_SEND_STATUSES);
});

// ── Tri-state (ADR-0013 Breaker H1/NEW-A): a DB blip must read UNAVAILABLE (retryable), NOT DENY.
// This distinction is load-bearing for the fan-out relay (DENY→evict vs UNAVAILABLE→withhold+ceiling)
// and the subscribe path (UNAVAILABLE→retryable soft error, never a fleet-denying ws.close).
test('verdict: bound courier → ALLOW', async () => {
  const { pool } = mockPool({ rows: 1 });
  assert.equal(await courierReadVerdict(pool, SUB, LOC, 'ord-9'), 'ALLOW');
  assert.equal(await courierRoomVerdict(pool, SUB, LOC, 'order:ord-9'), 'ALLOW');
});

test('verdict: clean 0-row (real negative) → DENY, not UNAVAILABLE', async () => {
  const { pool } = mockPool({ rows: 0 });
  assert.equal(await courierReadVerdict(pool, SUB, LOC, 'ord-9'), 'DENY');
});

test('verdict: location:* and malformed rooms → DENY with ZERO db access', async () => {
  const { pool } = mockPool({ rows: 1 });
  for (const room of ['location:other', 'location:loc-1', 'garbage', 'order:']) {
    assert.equal(await courierRoomVerdict(pool, SUB, LOC, room), 'DENY', room);
  }
  assert.equal(pool.connected, false);
});

test('verdict: missing activeLocationId → DENY, no db access (cannot scope tenant)', async () => {
  const { pool } = mockPool({ rows: 1 });
  assert.equal(await courierReadVerdict(pool, SUB, undefined, 'ord-9'), 'DENY');
  assert.equal(pool.connected, false);
});

test('verdict: connect/begin/select failure → UNAVAILABLE (retryable), never throws', async () => {
  for (const throwOn of ['connect', 'begin', 'select'] as const) {
    const { pool } = mockPool({ rows: 1, throwOn });
    assert.equal(await courierReadVerdict(pool, SUB, LOC, 'ord-9'), 'UNAVAILABLE', `throwOn=${throwOn}`);
  }
});
