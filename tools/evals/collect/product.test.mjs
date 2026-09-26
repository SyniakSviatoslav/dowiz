import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { stageSeconds, abandoned, countBy, orderIndicators, bookingIndicators, collect, WEEK_MS } from './product.mjs';
import * as O from './order.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const NOW = 100 * 86_400_000;
const orders = [
  { status: 'DELIVERED', channel: 'web', created_at_ms: NOW - 1000_000, at: { CONFIRMED: NOW - 940_000, DELIVERED: NOW - 100_000 }, eta: 30 },
  { status: 'DELIVERED', channel: 'web', created_at_ms: NOW - 2000_000, at: { CONFIRMED: NOW - 1880_000 } },
  { status: 'PREPARING', channel: 'phone', created_at_ms: NOW - 2 * 86_400_000, at: { CONFIRMED: NOW - 3 * 86_400_000 } },
  { status: 'SCHEDULED', channel: 'web', created_at_ms: NOW - 3 * 86_400_000 },
  { status: 'REJECTED', channel: 'web', created_at_ms: NOW - WEEK_MS - 1 },
];

test('stage seconds need both stamps in order', () => {
  assert.deepEqual(stageSeconds(orders, 'created', 'CONFIRMED'), [60, 120]);
  assert.deepEqual(stageSeconds(orders, 'CONFIRMED', 'DELIVERED'), [840]);
});

test('abandoned = not terminal, not scheduled, over a day old; countBy keys safely', () => {
  assert.equal(abandoned(orders, NOW), 1);
  assert.deepEqual(countBy([{ c: 'a-b' }, { c: 'a-b' }, { c: 'x' }], r => r.c), { a_b: 2, x: 1 });
});

test('order indicators are distributions and counts, never a row', () => {
  const r = byId(orderIndicators('v', orders, NOW));
  assert.equal(r['product.v.orders_7d'].value, 4);
  assert.equal(r['product.v.orders_7d'].note, '{"web":3,"phone":1}');
  assert.equal(r['product.v.created_to_confirmed.p50_s'].value, 60);
  assert.equal(r['product.v.created_to_confirmed.p90_s'].rule, 'plus25');
  assert.equal(r['product.v.confirmed_to_preparing.p90_s'].rule, 'trend');
  assert.equal(r['product.v.confirmed_to_preparing.p50_s'].value, null);
  assert.equal(r['product.v.abandoned'].value, 1);
  assert.equal(r['product.v.rejected_7d'].value, 0);
  assert.equal(r['product.v.eta_coverage_permille'].value, 500);
  assert.ok(r['product.v.eta_bias_s'].unverified);
  assert.equal(byId(orderIndicators('v', [], NOW))['product.v.eta_coverage_permille'].value, null);
  assert.doesNotMatch(JSON.stringify(r), /phone":\s*"\+|created_at_ms/);
});

test('booking conversion counts statuses', () => {
  const r = byId(bookingIndicators('v', [{ status: 'SEATED' }, { status: 'COMPLETED' }, { status: 'CANCELLED' }, { status: 'ARRIVED' }]));
  assert.equal(r['product.v.bookings_7d'].value, 4);
  assert.equal(r['product.v.booking_arrived_permille'].value, 750);
  assert.equal(byId(bookingIndicators('v', []))['product.v.booking_arrived_permille'].value, null);
});

const J = b => new Response(JSON.stringify(b));
test('the product collector reads orders and a seven-day reservation window', async () => {
  const seen = [];
  const f = async url => {
    const u = new URL(url);
    seen.push(u.pathname + u.search);
    if (u.pathname === '/api/auth/login') return J({ access_token: 'T' });
    if (u.pathname === '/api/owner/orders') return J({ orders });
    return J({ reservations: [{ status: 'SEATED' }] });
  };
  const creds = { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'p' };
  const r = byId(await collect({ hosts: ['https://v.dowiz.org'], creds, fetch: f, now: () => NOW }));
  assert.equal(r['product.v.orders_7d'].value, 4);
  assert.equal(r['product.v.bookings_7d'].value, 1);
  const m = NOW / 60000;
  assert.ok(seen.includes(`/api/owner/reservations?from=${m - 10080}&to=${m}`));
  const empty = byId(await collect({ hosts: ['https://v.dowiz.org'], creds, fetch: async u => (u.endsWith('login') ? J({ access_token: 'T' }) : J({})) }));
  assert.equal(empty['product.v.orders_7d'].value, 0);
  const refused = byId(await collect({ hosts: ['https://v.dowiz.org'], creds: {}, fetch: f }));
  assert.match(refused['product.v.orders_7d'].unverified, /no owner credentials/);
});

test('the order collector never places an order: no JSON means UNVERIFIED with the reason', async () => {
  const none = byId(await O.collect({ env: {} }));
  assert.equal(none['order.api_requests'].unverified, O.WHY_UNRUN);
  assert.equal(none['order.p95_ms'].rule, 'plus25');
  assert.equal(none['order.p95_ms'].unit, 'ms');
  assert.equal(none['order.cells_per_order'].unit, 'cells');
  const gone = byId(await O.collect({ env: { EVALS_TRAFFIC_JSON: '/no/such.json' } }));
  assert.match(gone['order.api_bytes'].unverified, /does not exist/);
  assert.ok(byId(await O.collect({}))['order.api_requests'].unverified);
});

test('a traffic report becomes the four budgets and the first paint', async () => {
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'evals-order-')), 'traffic-1.json');
  fs.writeFileSync(f, JSON.stringify({ at: '2026-09-26T00:00:00Z', requests: { total: 90, api: 60, byWho: { customer: 30 } },
    bytes: { api: 4000 }, latency: { p50: 80, p95: 400 }, image: { cellsPerOrder: 12 }, firstPaint: { fcp: 900 }, lifecycleMs: { delivered: 60000 } }));
  const r = byId(await O.collect({ env: { EVALS_TRAFFIC_JSON: f } }));
  assert.equal(r['order.api_requests'].value, 60);
  assert.equal(r['order.api_requests'].note, '{"customer":30}');
  assert.equal(r['order.cells_per_order'].value, 12);
  assert.equal(r['order.store_fcp_ms'].value, 900);
  const bare = byId(O.fromReport({}, 'f'));
  assert.equal(bare['order.api_requests'].value, null);
  assert.equal(bare['order.api_requests'].note, '{}');
  assert.equal(bare['order.placed_to_delivered_ms'].value, null);
  assert.equal(bare['order.requests_total'].value, null);
  assert.equal(bare['order.api_bytes'].value, null);
  assert.equal(bare['order.p50_ms'].value, null);
  assert.equal(bare['order.p95_ms'].value, null);
  assert.equal(bare['order.cells_per_order'].value, null);
  assert.equal(bare['order.store_fcp_ms'].value, null);
});
