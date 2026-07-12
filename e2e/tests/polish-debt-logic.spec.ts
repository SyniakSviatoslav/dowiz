/* eslint-disable @typescript-eslint/no-explicit-any, local/no-raw-any -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { reconcileCart } from '../../apps/web/src/lib/cartReconcile';
import { isOrderDetailsPending } from '../../packages/ui/src/components/admin/types';

// Deterministic proof for the polish-debt round's pure cores (no browser).
// Run: pnpm exec playwright test polish-debt-logic --project=desktop --reporter=list
const line = (over: Partial<{ id: string; productId: string; name: string; price: number; quantity: number; options: Record<string, string[]> }> = {}) => ({
  id: 'l1', productId: 'p1', name: 'Burger', price: 500, quantity: 1, options: {}, ...over,
}) as any;

test.describe('F9 reconcileCart — cart↔menu_version', () => {
  test('re-prices a modifier-free line whose stored price drifted', () => {
    const items = [line({ price: 500 })];
    const r = reconcileCart(items, 42, 43, [{ id: 'p1', price: 650, available: true }]);
    expect(r.summary).not.toBeNull();
    expect(r.summary!.repriced).toEqual([{ name: 'Burger', from: 500, to: 650 }]);
    expect(r.items[0]!.price).toBe(650);
    expect(r.pricedVersion).toBe(43);
  });

  test('drops an item no longer on the menu (sold-out/deleted)', () => {
    const items = [line({ id: 'l1', productId: 'gone', name: 'Special' }), line({ id: 'l2', productId: 'p1' })];
    const r = reconcileCart(items, 42, 43, [{ id: 'p1', price: 500, available: true }]);
    expect(r.summary!.removed).toEqual(['Special']);
    expect(r.items.map(i => i.productId)).toEqual(['p1']);
  });

  test('drops an item explicitly marked unavailable', () => {
    const r = reconcileCart([line()], 42, 43, [{ id: 'p1', price: 500, available: false }]);
    expect(r.summary!.removed).toEqual(['Burger']);
    expect(r.items).toHaveLength(0);
  });

  test('leaves a line with modifiers intact (server still guards those)', () => {
    const items = [line({ price: 800, options: { size: ['large'] } })];
    const r = reconcileCart(items, 42, 43, [{ id: 'p1', price: 650, available: true }]);
    expect(r.summary).toBeNull(); // not re-priced despite base-price drift
    expect(r.items[0]!.price).toBe(800);
    expect(r.pricedVersion).toBe(43); // but the version is still stamped forward
  });

  test('no-op fast path when already reconciled to this menu_version', () => {
    const items = [line({ price: 500 })];
    const r = reconcileCart(items, 43, 43, [{ id: 'p1', price: 650, available: true }]);
    expect(r.summary).toBeNull();
    expect(r.items).toBe(items); // same reference — no work done
  });

  test('empty cart just stamps the version', () => {
    const r = reconcileCart([], null, 43, []);
    expect(r.summary).toBeNull();
    expect(r.pricedVersion).toBe(43);
  });

  test('legacy cart (null pricedVersion) reconciles against current prices', () => {
    const r = reconcileCart([line({ price: 500 })], null, 43, [{ id: 'p1', price: 500, available: true }]);
    expect(r.summary).toBeNull(); // prices already match → no false alarm
    expect(r.pricedVersion).toBe(43);
  });
});

test.describe('F7 isOrderDetailsPending — owner hollow-card guard', () => {
  test('pending when itemCount says items exist but items not yet backfilled', () => {
    expect(isOrderDetailsPending({ itemCount: 3, items: [] })).toBe(true);
  });
  test('not pending once items have hydrated', () => {
    expect(isOrderDetailsPending({ itemCount: 3, items: [{ name: 'X', quantity: 1 }] })).toBe(false);
  });
  test('not pending for a genuinely empty order (no itemCount)', () => {
    expect(isOrderDetailsPending({ itemCount: 0, items: [] })).toBe(false);
    expect(isOrderDetailsPending({ items: [] })).toBe(false);
  });
});

test.describe('F14 single WebSocket client', () => {
  test('the reconnect-capped WsClient is gone (only the reconnect-forever hook remains)', () => {
    const capped = fileURLToPath(new URL('../../packages/ui/src/lib/websocket.ts', import.meta.url));
    const liveHook = fileURLToPath(new URL('../../apps/web/src/lib/useWebSocket.ts', import.meta.url));
    expect(existsSync(capped), 'dead capped WsClient must not be reintroduced').toBe(false);
    expect(existsSync(liveHook), 'the live reconnect-forever hook is the single client').toBe(true);
  });
});
