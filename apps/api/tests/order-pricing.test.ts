import { test } from 'node:test';
import assert from 'node:assert/strict';
import { computeOrderPricing } from '../src/lib/order-pricing.js';

// Guardrail for the pure pricing/validation core extracted from POST /orders.
// Money is integer minor units throughout (RED LINE). Each failure case asserts
// the exact code the inline handler emitted as a 422.

const prod = (id: string, price: number, name = id) => [id, { name, price }] as const;
const mod = (productId: string, mid: string, priceDelta: number, groupId: string, name = mid) =>
  [`${productId}_${mid}`, { name, price_delta: priceDelta, group_id: groupId }] as const;

test('computeOrderPricing — happy path sums line totals (product + modifiers) × qty', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 2, modifier_ids: ['m1'] }],
    productMap: new Map([prod('p1', 500, 'Pizza')]),
    modMap: new Map([mod('p1', 'm1', 150, 'g1', 'Extra cheese')]),
    groupsByProduct: new Map(),
  });
  assert.equal(res.ok, true);
  if (!res.ok) return;
  // (500 + 150) * 2 = 1300
  assert.equal(res.subtotal, 1300);
  assert.equal(res.orderItemRows.length, 1);
  assert.equal(res.orderItemRows[0].priceSnapshot, 500);
  assert.equal(res.orderItemRows[0].modifiers[0].priceDeltaSnapshot, 150);
});

test('computeOrderPricing — multiple items accumulate subtotal', () => {
  const res = computeOrderPricing({
    items: [
      { product_id: 'p1', quantity: 1 },
      { product_id: 'p2', quantity: 3 },
    ],
    productMap: new Map([prod('p1', 500), prod('p2', 200)]),
    modMap: new Map(),
    groupsByProduct: new Map(),
  });
  assert.equal(res.ok, true);
  if (!res.ok) return;
  assert.equal(res.subtotal, 500 + 600);
});

test('computeOrderPricing — duplicate modifier id → DUPLICATE_MODIFIER', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 1, modifier_ids: ['m1', 'm1'] }],
    productMap: new Map([prod('p1', 500)]),
    modMap: new Map([mod('p1', 'm1', 100, 'g1')]),
    groupsByProduct: new Map(),
  });
  assert.equal(res.ok, false);
  if (res.ok) return;
  assert.equal(res.error.code, 'DUPLICATE_MODIFIER');
});

test('computeOrderPricing — unknown/unavailable modifier → MODIFIER_UNAVAILABLE', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 1, modifier_ids: ['ghost'] }],
    productMap: new Map([prod('p1', 500)]),
    modMap: new Map(),
    groupsByProduct: new Map(),
  });
  assert.equal(res.ok, false);
  if (res.ok) return;
  assert.equal(res.error.code, 'MODIFIER_UNAVAILABLE');
});

test('computeOrderPricing — required group below min → MODIFIER_MIN_NOT_MET', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 1, modifier_ids: [] }],
    productMap: new Map([prod('p1', 500)]),
    modMap: new Map(),
    groupsByProduct: new Map([['p1', [{ id: 'g1', min_select: 1, max_select: 2, required: true }]]]),
  });
  assert.equal(res.ok, false);
  if (res.ok) return;
  assert.equal(res.error.code, 'MODIFIER_MIN_NOT_MET');
});

test('computeOrderPricing — group above max → MODIFIER_MAX_EXCEEDED', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 1, modifier_ids: ['m1', 'm2'] }],
    productMap: new Map([prod('p1', 500)]),
    modMap: new Map([mod('p1', 'm1', 100, 'g1'), mod('p1', 'm2', 100, 'g1')]),
    groupsByProduct: new Map([['p1', [{ id: 'g1', min_select: 0, max_select: 1, required: false }]]]),
  });
  assert.equal(res.ok, false);
  if (res.ok) return;
  assert.equal(res.error.code, 'MODIFIER_MAX_EXCEEDED');
});

test('computeOrderPricing — optional group at zero passes (not required)', () => {
  const res = computeOrderPricing({
    items: [{ product_id: 'p1', quantity: 1, modifier_ids: [] }],
    productMap: new Map([prod('p1', 500)]),
    modMap: new Map(),
    groupsByProduct: new Map([['p1', [{ id: 'g1', min_select: 1, max_select: 2, required: false }]]]),
  });
  assert.equal(res.ok, true);
});
