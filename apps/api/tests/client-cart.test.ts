import test from 'node:test';
import assert from 'node:assert/strict';

// Simple mock for localStorage since we're in node environment for tests
const mockLocalStorage = new Map<string, string>();
(global as any).localStorage = {
  getItem: (key: string) => mockLocalStorage.get(key) || null,
  setItem: (key: string, value: string) => mockLocalStorage.set(key, value),
  removeItem: (key: string) => mockLocalStorage.delete(key),
};
const dispatchedEvents: Array<{ type: string; detail: unknown }> = [];
(global as any).window = {
  dispatchEvent: (ev: { type: string; detail: unknown }) => {
    dispatchedEvents.push({ type: ev.type, detail: ev.detail });
    return true;
  },
};

// Now import after setting up mocks
import { getCart, saveCart, clearCart } from '../src/client/cart/store.js';

test('Cart Store', async (t) => {
  mockLocalStorage.clear();

  await t.test('initializes empty cart', () => {
    const cart = getCart('loc1');
    assert.equal(cart.v, 1);
    assert.equal(cart.locationId, 'loc1');
    assert.equal(cart.items.length, 0);
  });

  await t.test('saves and retrieves items', () => {
    const cart = getCart('loc1');
    cart.items.push({ productId: 'p1', quantity: 2, modifierIds: [] });
    const before = Date.now();
    dispatchedEvents.length = 0;
    saveCart('loc1', cart);

    const retrieved = getCart('loc1');
    assert.equal(retrieved.items.length, 1);
    assert.equal(retrieved.items[0].productId, 'p1');
    assert.equal(retrieved.items[0].quantity, 2);
    // saveCart must advance updatedAt (in-place mutation persisted)
    assert.ok(retrieved.updatedAt >= before, 'updatedAt advanced on save');
    // saveCart must fire exactly one cart:updated event with the locationId
    assert.equal(dispatchedEvents.length, 1);
    assert.equal(dispatchedEvents[0].type, 'cart:updated');
    assert.equal(
      (dispatchedEvents[0].detail as { locationId: string }).locationId,
      'loc1',
    );
  });

  await t.test('recovers from corruption by resetting', () => {
    mockLocalStorage.set('dowiz:cart:loc2', '{ corrupt json');
    // Fetch should catch the error and return empty cart
    const cart = getCart('loc2');
    assert.equal(cart.v, 1);
    assert.equal(cart.items.length, 0);
  });

  await t.test('resets cart on version or locationId mismatch', () => {
    // Valid JSON but locationId belongs to a different tenant → distinct
    // business branch from corrupt JSON: must reset, never leak items cross-key.
    mockLocalStorage.set(
      'dowiz:cart:loc5',
      JSON.stringify({
        v: 1,
        locationId: 'loc1',
        menuVersion: '',
        items: [{ productId: 'leak', quantity: 9, modifierIds: [] }],
        createdAt: 1,
        updatedAt: 1,
      }),
    );
    const mismatched = getCart('loc5');
    assert.equal(mismatched.locationId, 'loc5');
    assert.equal(mismatched.items.length, 0);

    // Valid JSON but unknown schema version → reset.
    mockLocalStorage.set(
      'dowiz:cart:loc6',
      JSON.stringify({
        v: 2,
        locationId: 'loc6',
        menuVersion: '',
        items: [{ productId: 'stale', quantity: 3, modifierIds: [] }],
        createdAt: 1,
        updatedAt: 1,
      }),
    );
    const versioned = getCart('loc6');
    assert.equal(versioned.v, 1);
    assert.equal(versioned.items.length, 0);
  });

  await t.test('clears cart', () => {
    const cart = getCart('loc3');
    cart.items.push({ productId: 'p3', quantity: 1, modifierIds: [] });
    saveCart('loc3', cart);
    
    dispatchedEvents.length = 0;
    clearCart('loc3');
    const cleared = getCart('loc3');
    assert.equal(cleared.items.length, 0);
    // clearCart must fire one cart:updated event carrying the reset cart
    assert.equal(dispatchedEvents.length, 1);
    assert.equal(dispatchedEvents[0].type, 'cart:updated');
    assert.equal(
      (dispatchedEvents[0].detail as { locationId: string }).locationId,
      'loc3',
    );
  });
});
