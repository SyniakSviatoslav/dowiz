import test from 'node:test';
import assert from 'node:assert/strict';

// Simple mock for localStorage since we're in node environment for tests
const mockLocalStorage = new Map<string, string>();
(global as any).localStorage = {
  getItem: (key: string) => mockLocalStorage.get(key) || null,
  setItem: (key: string, value: string) => mockLocalStorage.set(key, value),
  removeItem: (key: string) => mockLocalStorage.delete(key),
};
(global as any).window = {
  dispatchEvent: () => {}
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
    saveCart('loc1', cart);

    const retrieved = getCart('loc1');
    assert.equal(retrieved.items.length, 1);
    assert.equal(retrieved.items[0].productId, 'p1');
    assert.equal(retrieved.items[0].quantity, 2);
  });

  await t.test('recovers from corruption by resetting', () => {
    mockLocalStorage.set('dowiz:cart:loc2', '{ corrupt json');
    // Fetch should catch the error and return empty cart
    const cart = getCart('loc2');
    assert.equal(cart.v, 1);
    assert.equal(cart.items.length, 0);
  });

  await t.test('clears cart', () => {
    const cart = getCart('loc3');
    cart.items.push({ productId: 'p3', quantity: 1, modifierIds: [] });
    saveCart('loc3', cart);
    
    clearCart('loc3');
    const cleared = getCart('loc3');
    assert.equal(cleared.items.length, 0);
  });
});
