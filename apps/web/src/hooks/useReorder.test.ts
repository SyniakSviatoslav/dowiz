import test from 'node:test';
import assert from 'node:assert/strict';
import { rehydrateOrderItems, type ReorderOrderItem } from './useReorder.js';
import type { ReconcileProduct } from '../lib/cartReconcile.js';

const menu: ReconcileProduct[] = [
  { id: 'p1', price: 800, available: true },
  { id: 'p2', price: 500, available: true }, // price drifted from the order snapshot (450 → 500)
  { id: 'p3', price: 300, available: false }, // now sold out
  // p4 deleted from the menu entirely
];

const order: ReorderOrderItem[] = [
  { productId: 'p1', nameSnapshot: 'Margherita', priceSnapshot: 800, quantity: 2 },
  { productId: 'p2', nameSnapshot: 'Cola', priceSnapshot: 450, quantity: 1 },
  { productId: 'p3', nameSnapshot: 'Tiramisu', priceSnapshot: 300, quantity: 1 },
  { productId: 'p4', nameSnapshot: 'Deleted dish', priceSnapshot: 999, quantity: 1 },
];

test('rehydrateOrderItems: keeps available lines, preserves quantity', () => {
  const r = rehydrateOrderItems(order, 7, menu);
  const p1 = r.items.find((i) => i.productId === 'p1');
  assert.ok(p1, 'available line survives');
  assert.equal(p1!.quantity, 2, 'quantity is preserved from the order');
  assert.equal(p1!.price, 800, 'unchanged price stays');
});

test('rehydrateOrderItems: skips unavailable (sold out) AND deleted items with names', () => {
  const r = rehydrateOrderItems(order, 7, menu);
  assert.ok(!r.items.some((i) => i.productId === 'p3'), 'sold-out line is dropped');
  assert.ok(!r.items.some((i) => i.productId === 'p4'), 'deleted line is dropped');
  assert.deepEqual([...r.skipped].sort(), ['Deleted dish', 'Tiramisu'], 'skipped names surfaced for the note');
});

test('rehydrateOrderItems: re-validates drifted price to the live menu (no hand-rolled pricing)', () => {
  const r = rehydrateOrderItems(order, 7, menu);
  const p2 = r.items.find((i) => i.productId === 'p2');
  assert.ok(p2, 'line survives');
  assert.equal(p2!.price, 500, 'price re-validated to current menu, not the stale 450 snapshot');
  assert.deepEqual(r.repriced, [{ name: 'Cola', from: 450, to: 500 }], 'reprice recorded for the note');
});

test('rehydrateOrderItems: empty when nothing is orderable anymore', () => {
  const soldOutMenu: ReconcileProduct[] = [{ id: 'p1', price: 800, available: false }];
  const r = rehydrateOrderItems([{ productId: 'p1', nameSnapshot: 'Margherita', priceSnapshot: 800, quantity: 1 }], 9, soldOutMenu);
  assert.equal(r.items.length, 0, 'no lines added');
  assert.deepEqual(r.skipped, ['Margherita'], 'the sole unavailable item is reported');
});

test('rehydrateOrderItems: drops zero/negative-quantity and productId-less lines', () => {
  const r = rehydrateOrderItems(
    [
      { productId: 'p1', nameSnapshot: 'Margherita', priceSnapshot: 800, quantity: 0 },
      { productId: '', nameSnapshot: 'Ghost', priceSnapshot: 100, quantity: 1 },
    ],
    7,
    menu,
  );
  assert.equal(r.items.length, 0, 'invalid candidate lines never reach the cart');
});
