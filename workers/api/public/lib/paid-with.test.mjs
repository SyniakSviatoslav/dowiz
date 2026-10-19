// How an order was paid, called for real. `node --test workers/api/public/lib/paid-with.test.mjs`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { paidWith, paidIcon } from './paid-with.js';

const round = extra => ({ fulfilment: { kind: 'dine_in', table: '7' }, payment: 'cash', ...extra });

test('a round paid by card says card, whatever the placement stamped', () => {
  const o = round({ payments: [{ by: 'u1', amount: 1500, method: 'card', at: 1 }] });
  assert.deepEqual(paidWith(o), ['card']);
  assert.equal(paidIcon(paidWith(o)), 'credit-card');
});

test('a split round names every method once, in the order taken', () => {
  const o = round({ payments: [{ method: 'cash' }, { method: 'card' }, { method: 'cash' }] });
  assert.deepEqual(paidWith(o), ['cash', 'card']);
});

test('a round nobody has paid has no method yet', () => {
  assert.deepEqual(paidWith(round({})), []);
  assert.deepEqual(paidWith(round({ payments: [] })), []);
});

test('twin: a delivery keeps the method chosen at checkout', () => {
  assert.deepEqual(paidWith({ fulfilment: { kind: 'delivery' }, payment: 'crypto' }), ['crypto']);
  assert.equal(paidIcon(['crypto']), 'currency-bitcoin');
  assert.deepEqual(paidWith({ payment: 'cash' }), ['cash']);
  assert.deepEqual(paidWith(null), []);
});

// The course as `ebills/import/bill.rs` leaves it once the till's bill closed
// its table: born PICKED_UP at the table, no `payments[]`, the bill on it.
const tillCourse = bill => ({ id: 'ebills:00000000-0000-4000-8000-000000008602', status: 'PICKED_UP', channel: 'ebills',
  fulfilment: { kind: 'dine_in', table: '12', fee: 0 }, payment: 'cash', ...(bill ? { payment_status: 'paid', bill } : {}) });

test('a till table paid by its bill says how the bill was paid', () => {
  assert.deepEqual(paidWith(tillCourse({ uuid: 'b1', total: 500, payment: 'card', courses: 1 })), ['card']);
  assert.deepEqual(paidWith(tillCourse({ uuid: 'b1', total: 500, payment: 'cash', courses: 1 })), ['cash']);
});

test('twin: a till course whose bill has not come yet has no method', () => {
  assert.deepEqual(paidWith(tillCourse(null)), []);
});
