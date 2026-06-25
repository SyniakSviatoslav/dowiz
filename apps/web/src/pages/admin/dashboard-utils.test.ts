import test from 'node:test';
import assert from 'node:assert/strict';
import { mergeDelta } from './dashboard-utils.js';

const order = (id: string, status: string, extra: any = {}): any => ({
  id, status, total: 1000, createdAt: '2026-06-22T10:00:00Z', items: [], itemCount: 1, ...extra,
});

test('mergeDelta: WS order-status frames', async (t) => {
  await t.test('forward transition applies (PENDING → CONFIRMED)', () => {
    const next = mergeDelta([order('o1', 'PENDING')], { orderId: 'o1', status: 'CONFIRMED' }, false);
    assert.equal(next[0]!.status, 'CONFIRMED');
  });

  await t.test('stale backward frame does NOT revert (CONFIRMED ⇸ PENDING)', () => {
    const next = mergeDelta([order('o1', 'CONFIRMED')], { orderId: 'o1', status: 'PENDING' }, false);
    assert.equal(next[0]!.status, 'CONFIRMED', 'a late PENDING frame must not revert a confirmed order');
  });

  await t.test('deeper backward frame blocked (IN_DELIVERY ⇸ CONFIRMED)', () => {
    const next = mergeDelta([order('o1', 'IN_DELIVERY')], { orderId: 'o1', status: 'CONFIRMED' }, false);
    assert.equal(next[0]!.status, 'IN_DELIVERY');
  });

  await t.test('terminal CANCELLED applies even from a forward state', () => {
    const next = mergeDelta([order('o1', 'CONFIRMED')], { orderId: 'o1', status: 'CANCELLED' }, false);
    assert.equal(next[0]!.status, 'CANCELLED');
  });

  await t.test('terminal DELIVERED applies (IN_DELIVERY → DELIVERED)', () => {
    const next = mergeDelta([order('o1', 'IN_DELIVERY')], { orderId: 'o1', status: 'DELIVERED' }, false);
    assert.equal(next[0]!.status, 'DELIVERED');
  });

  await t.test('non-status fields still merge even when status would regress', () => {
    const next = mergeDelta([order('o1', 'CONFIRMED', { courierName: null })], { orderId: 'o1', status: 'PENDING', courierName: 'Alex' }, false);
    assert.equal(next[0]!.status, 'CONFIRMED', 'status held');
    assert.equal(next[0]!.courierName, 'Alex', 'courier still updated');
  });

  await t.test('duplicate order.created (isNew) is a no-op when order exists', () => {
    const prev = [order('o1', 'CONFIRMED')];
    const next = mergeDelta(prev, { orderId: 'o1', status: 'PENDING' }, true);
    assert.equal(next, prev, 'returns the same array (no clobber)');
  });
});
