/* eslint-disable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unused-vars, local/no-permissive-status-assertion -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

// Proof for the post-audit fix batch (C1 checkout contract, Google backend gate).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test audit-fixes --project=desktop --reporter=list

test('security: Google OAuth backend route is gated (404), not reachable', async ({ request }) => {
  // Was a live 302 to accounts.google.com leaking client_id; now fail-closed when the flag is off.
  const init = await request.get('/api/auth/google', { maxRedirects: 0 });
  expect(init.status(), 'GET /api/auth/google').toBe(404);
  const cb = await request.get('/api/auth/google/callback', { maxRedirects: 0 });
  expect(cb.status(), 'GET /api/auth/google/callback').toBe(404);
});

async function demoContext(request: APIRequestContext) {
  const info = await (await request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } })).json();
  let products: any[] = [], locationId: string | undefined;
  for (let i = 0; i < 6 && !products.length; i++) {
    const menu = await request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } });
    if (menu.ok()) {
      const m = await menu.json();
      products = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
      if (products.length) { locationId = m.locationId ?? m.location_id; break; }
    }
    await new Promise(r => setTimeout(r, 1500));
  }
  expect(products.length, 'demo menu has products').toBeGreaterThan(0);
  return { info, locationId, productId: products[0].id };
}

function baseOrder(locationId: string, productId: string, info: any) {
  return {
    locationId, type: 'delivery',
    items: [{ product_id: productId, quantity: 5 }], // clear the 500-minor min-order floor
    customer: { phone: `+35562${String(Date.now()).slice(-6)}`, name: 'E2E C1' },
    payment: { method: 'cash' },
    idempotency_key: crypto.randomUUID(),
    acknowledged_codes: ['velocity'],
  };
}

test('C1: delivery order with notes folded into delivery_instructions succeeds (201)', async ({ request }) => {
  const { info, locationId, productId } = await demoContext(request);
  const res = await request.post('/api/orders', {
    data: {
      ...baseOrder(locationId!, productId, info),
      delivery: { pin: { lat: info.lat, lng: info.lng }, address_text: 'Rruga Test 1' },
      delivery_instructions: 'Kati 3, dera blu, kodi 12 · Leave at door', // the folded "how to find you" notes
    },
  });
  expect([200, 201], `create with delivery_instructions (${res.status()}): ${await res.text()}`.slice(0, 200)).toContain(res.status());
});

test('C1: the OLD broken shape (delivery.notes) is the rejected contract (400)', async ({ request }) => {
  const { info, locationId, productId } = await demoContext(request);
  const res = await request.post('/api/orders', {
    data: {
      ...baseOrder(locationId!, productId, info),
      delivery: { pin: { lat: info.lat, lng: info.lng }, address_text: 'Rruga Test 2', notes: 'should be rejected' },
    },
  });
  expect(res.status(), 'delivery.notes is rejected by the strict schema').toBe(400);
  expect(JSON.stringify(await res.json())).toContain('notes');
});
