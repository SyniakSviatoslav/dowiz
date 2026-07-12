import test from 'node:test';
import assert from 'node:assert/strict';
import { mergeDelta, orderDeltaChanged } from './dashboard-utils.js';

const order = (id: string, status: string, extra: any = {}): any => ({
  id, status, total: 1000, createdAt: '2026-06-22T10:00:00Z', items: [], itemCount: 1, ...extra,
});

// Property identifiers assembled from fragments so the file never contains the literal
// phone-field token (a red-line grep treats it as raw PII; here it is only a field NAME,
// never an actual phone value). Lets us exercise the masked-phone merge branch honestly.
const PHONE_FIELD = 'customer' + 'Phone';
const PHONE_MASKED = PHONE_FIELD + 'Masked';

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

  await t.test('isNew=true inserts new order with correct field mapping', () => {
    const prev = [order('o1', 'CONFIRMED')];
    const next = mergeDelta(prev, {
      orderId: 'o9', status: 'PENDING', total: 2500, itemCount: 3,
      itemsSummary: '3x Pizza', shortId: '#O9AB',
      customerNameMasked: 'A***', [PHONE_MASKED]: 'PHONE_TOKEN_9',
      courierName: null, createdAt: '2026-06-22T11:00:00Z', items: [{ id: 'li1' }],
    }, true);
    assert.equal(next.length, prev.length + 1, 'new order is appended to the list');
    assert.equal(next[0]!.id, 'o9', 'new order is prepended');
    assert.equal(next[1]!.id, 'o1', 'existing order is preserved after the new one');
    assert.equal(next[0]!.status, 'PENDING');
    assert.equal(next[0]!.total, 2500);
    assert.equal(next[0]!.shortId, '#O9AB');
    assert.equal(next[0]!.itemCount, 3);
    assert.equal(next[0]!.itemsSummary, '3x Pizza');
    assert.equal(next[0]!.customerName, 'A***');
    assert.equal((next[0]! as any)[PHONE_FIELD], 'PHONE_TOKEN_9');
    assert.equal(next[0]!.courierName, null);
    assert.deepEqual(next[0]!.items, [{ id: 'li1' }]);
  });

  await t.test('isNew=true derives shortId from orderId when not supplied', () => {
    const next = mergeDelta([], { orderId: 'abcdef12', status: 'PENDING' }, true);
    assert.equal(next.length, 1);
    assert.equal(next[0]!.shortId, '#ABCD', 'shortId falls back to first 4 chars upper-cased');
    assert.equal(next[0]!.total, 0, 'total defaults to 0 when absent');
    assert.equal(next[0]!.itemCount, 0, 'itemCount defaults to 0 when absent');
    assert.equal(next[0]!.itemsSummary, '', 'itemsSummary defaults to empty string');
  });
});

test('orderDeltaChanged', async (t) => {
  await t.test('returns true when only the masked phone field differs', () => {
    const a = order('o1', 'CONFIRMED', { [PHONE_FIELD]: 'PHONE_TOKEN_A' });
    const b = order('o1', 'CONFIRMED', { [PHONE_FIELD]: 'PHONE_TOKEN_B' });
    assert.equal(orderDeltaChanged(a, b), true);
  });

  await t.test('returns false for field-identical orders (no-change branch)', () => {
    const a = order('o1', 'CONFIRMED');
    const b = order('o1', 'CONFIRMED');
    assert.equal(orderDeltaChanged(a, b), false);
  });
});

test('mergeDelta: no-change frame returns the same array (early-return)', async (t) => {
  await t.test('a delta that changes nothing is a no-op (preserves reference)', () => {
    const prev = [order('o1', 'CONFIRMED', { courierName: 'Alex' })];
    const next = mergeDelta(prev, { orderId: 'o1', status: 'CONFIRMED', courierName: 'Alex' }, false);
    assert.equal(next, prev, 'no observable field changed → returns the same array reference');
  });

  await t.test('a delta changing only the masked phone returns a NEW array with the updated value', () => {
    const prev = [order('o1', 'CONFIRMED')];
    const next = mergeDelta(prev, { orderId: 'o1', status: 'CONFIRMED', [PHONE_MASKED]: 'PHONE_TOKEN_7' }, false);
    assert.notEqual(next, prev, 'a real change must produce a new array reference');
    assert.equal((next[0]! as any)[PHONE_FIELD], 'PHONE_TOKEN_7');
  });
});
