import { test } from 'node:test';
import assert from 'node:assert/strict';
import { insertOrderWithItems, type InsertOrderInput } from '../src/lib/order-persistence.js';

// Guardrail for the side-effectful persistence block. A stub client captures
// every (sql, params); the orders INSERT params array is asserted positionally
// because a swap (e.g. subtotal/total) would be a silent money bug. Also pins
// the dependent inserts + the two transactional enqueues.

interface Call {
  sql: string;
  params: unknown[];
}

function stubClient(calls: Call[]) {
  return {
    async query(sql: string, params: unknown[] = []) {
      calls.push({ sql, params });
      if (/INSERT INTO orders\b/.test(sql)) {
        return {
          rows: [{ id: 'order-1', status: 'PENDING', subtotal: 1500, total: 1700, created_at: 'now', timeout_at: 'later' }],
          rowCount: 1,
        };
      }
      if (/INSERT INTO order_items\b/.test(sql)) {
        return { rows: [{ id: 'oi-1' }], rowCount: 1 };
      }
      return { rows: [], rowCount: 0 };
    },
  };
}

function stubQueue(enqueues: Array<{ name: string; payload: any; opts: any }>) {
  return {
    enqueue: async (name: string, payload: any, opts: any) => {
      enqueues.push({ name, payload, opts });
      return 'job';
    },
    work: async () => {},
    start: async () => {},
    stop: async () => {},
  } as any;
}

const baseInput = (): InsertOrderInput => ({
  locationId: 'loc-1',
  resolvedCustomerId: 'cust-1',
  deliveryAddressText: 'Rruga X',
  pin: { lat: 41.32, lng: 19.45 },
  subtotal: 1500,
  deliveryFee: 200,
  taxTotal: 0,
  discountTotal: 0,
  total: 1700,
  cashPayWith: null,
  currencyCode: 'ALL',
  menuVersion: '7',
  clientMenuVersion: null,
  requestHash: 'hash-abc',
  timeoutAt: new Date('2026-06-27T12:00:00.000Z'),
  rawInstructions: 'leave at door',
  otpServerVerified: false,
  clientIpHash: 'iphash',
  preflightMeta: '{"outcome":"clean"}',
  type: 'delivery',
  messengerKind: null,
  messengerHandle: null,
  deliveryPhotoKey: null,
  tipAmount: undefined,
  orderItemRows: [
    { productId: 'p1', nameSnapshot: 'Pizza', priceSnapshot: 1500, quantity: 1, modifiers: [{ modifierId: 'm1', nameSnapshot: 'Cheese', priceDeltaSnapshot: 0 }] },
  ],
  idempotencyKey: 'idem-1',
  phoneHash: 'phash',
  custPhone: '+355600000',
});

test('insertOrderWithItems — orders INSERT receives the exact positional params (money safety)', async () => {
  const calls: Call[] = [];
  const enqueues: any[] = [];
  await insertOrderWithItems(stubClient(calls), stubQueue(enqueues), baseInput());

  const orderInsert = calls.find((c) => /INSERT INTO orders\b/.test(c.sql))!;
  assert.ok(orderInsert, 'orders INSERT issued');
  // Positional $1..$24 — see the VALUES mapping in order-persistence.ts.
  assert.deepEqual(orderInsert.params, [
    'loc-1', 'cust-1', 'Rruga X', 41.32, 19.45,
    1500, 200, 0, 0, 1700,
    null, 'ALL',
    '7', null, 'hash-abc', new Date('2026-06-27T12:00:00.000Z'),
    'leave at door',
    JSON.stringify({ otp_verified: false, client_ip_hash: 'iphash' }),
    '{"outcome":"clean"}',
    'delivery',
    null, null,
    null,
    0, // tipAmount undefined → 0
  ]);
});

test('insertOrderWithItems — writes dependent rows + returns order/trackCode', async () => {
  const calls: Call[] = [];
  const enqueues: any[] = [];
  const result = await insertOrderWithItems(stubClient(calls), stubQueue(enqueues), baseInput());

  const sqls = calls.map((c) => c.sql);
  assert.ok(sqls.some((s) => /INSERT INTO velocity_events/.test(s)), 'velocity event written');
  assert.ok(sqls.some((s) => /INSERT INTO order_items\b/.test(s)), 'order_items written');
  assert.ok(sqls.some((s) => /INSERT INTO order_item_modifiers/.test(s)), 'modifiers written');
  assert.ok(sqls.some((s) => /INSERT INTO idempotency_keys/.test(s)), 'idempotency key written');
  assert.ok(sqls.some((s) => /INSERT INTO customer_track_grants/.test(s)), 'track grant written');

  assert.equal(result.order.id, 'order-1');
  assert.equal(typeof result.trackCode, 'string'); // minted for a resolved customer
});

test('insertOrderWithItems — enqueues timeout + notify jobs transactionally', async () => {
  const calls: Call[] = [];
  const enqueues: Array<{ name: string; payload: any; opts: any }> = [];
  await insertOrderWithItems(stubClient(calls), stubQueue(enqueues), baseInput());

  assert.equal(enqueues.length, 2);
  const [timeout, notify] = enqueues;
  assert.equal(timeout.payload.orderId, 'order-1');
  assert.equal(timeout.opts.singletonKey, 'order-1');
  assert.equal(notify.payload.event, 'order.created');
  assert.equal(notify.payload.entity_id, 'order-1');
  assert.equal(notify.opts.singletonKey, 'order.created:order-1:loc-1');
  // both enqueues carry the tx db handle so they land inside the order transaction
  assert.ok(timeout.opts.db?.executeSql, 'timeout enqueue carries tx db');
  assert.ok(notify.opts.db?.executeSql, 'notify enqueue carries tx db');
});

test('insertOrderWithItems — no track grant / velocity for anonymous (no phone, no customer)', async () => {
  const calls: Call[] = [];
  const enqueues: any[] = [];
  const input = { ...baseInput(), resolvedCustomerId: null, custPhone: undefined, phoneHash: undefined, clientIpHash: undefined };
  const result = await insertOrderWithItems(stubClient(calls), stubQueue(enqueues), input);

  const sqls = calls.map((c) => c.sql);
  assert.ok(!sqls.some((s) => /INSERT INTO customer_track_grants/.test(s)), 'no track grant for anonymous');
  assert.ok(!sqls.some((s) => /INSERT INTO velocity_events/.test(s)), 'no velocity event without phone/ip hash');
  assert.equal(result.trackCode, undefined);
});
