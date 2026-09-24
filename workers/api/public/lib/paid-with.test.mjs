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
