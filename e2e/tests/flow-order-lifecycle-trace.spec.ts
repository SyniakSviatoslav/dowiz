import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

/**
 * Order lifecycle — authoritative end-to-end trace of the state machine in
 * packages/domain/src/order-machine.ts:
 *
 *   PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED
 *   PENDING → {REJECTED, CANCELLED}      CONFIRMED → IN_DELIVERY (skip prep)
 *   terminal: DELIVERED, REJECTED, CANCELLED
 *
 * Drives transitions via PATCH /api/orders/:id/status (owner), which runs every
 * change through assertTransition(). Proves: (1) the full happy path, (2) the
 * CONFIRMED→IN_DELIVERY skip-prep branch, (3) the machine's guards reject illegal
 * transitions, (4) terminal states are final. API-level, so it runs against prod
 * (default) or any VITE_BASE_URL.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

let authToken: string;
let locationId: string;
let productId: string;

test.describe.configure({ mode: 'serial' });

async function createOrder(request: any): Promise<string> {
  const res = await request.post(`${BASE}/api/orders`, {
    data: {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: `+35560${String(Date.now()).slice(-6)}`, name: 'Lifecycle Trace' },
      delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Barrikadave, Tirana' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
    },
  });
  expect(res.status(), `create order: ${await res.text()}`).toBe(201);
  return (await res.json()).id;
}

async function patchStatus(request: any, id: string, status: string) {
  return request.patch(`${BASE}/api/orders/${id}/status`, {
    data: { status },
    headers: { Authorization: `Bearer ${authToken}` },
  });
}

async function readStatus(request: any, id: string): Promise<string> {
  const res = await request.get(`${BASE}/api/owner/locations/${locationId}/orders/${id}/verify`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  expect(res.status()).toBe(200);
  const body = await res.json();
  return body.order?.status ?? body.status;
}

test.describe('Order lifecycle — state machine trace', () => {
  test.beforeAll(async ({ request }) => {
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    expect(auth.status()).toBe(200);
    const body = await auth.json();
    authToken = body.access_token;
    locationId = body.activeLocationId;

    const menu = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(menu.ok()).toBe(true);
    const cats = (await menu.json()).categories || [];
    const products = cats.flatMap((c: any) => c.products || []);
    expect(products.length, 'demo menu has products').toBeGreaterThan(0);
    productId = products[0].id;
  });

  test('happy path: PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED', async ({ request }) => {
    const id = await createOrder(request);
    expect(await readStatus(request, id)).toBe('PENDING');

    for (const next of ['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'] as const) {
      const res = await patchStatus(request, id, next);
      expect(res.status(), `${next}: ${await res.text()}`).toBe(200);
      expect((await res.json()).status).toBe(next);
    }

    expect(await readStatus(request, id)).toBe('DELIVERED');
  });

  test('skip-prep branch: CONFIRMED → IN_DELIVERY → DELIVERED', async ({ request }) => {
    const id = await createOrder(request);
    expect((await (await patchStatus(request, id, 'CONFIRMED')).json()).status).toBe('CONFIRMED');
    const inDel = await patchStatus(request, id, 'IN_DELIVERY');
    expect(inDel.status(), await inDel.text()).toBe(200);
    const delivered = await patchStatus(request, id, 'DELIVERED');
    expect(delivered.status()).toBe(200);
    expect(await readStatus(request, id)).toBe('DELIVERED');
  });

  test('guard: illegal transition PENDING → READY is rejected (400)', async ({ request }) => {
    const id = await createOrder(request);
    const res = await patchStatus(request, id, 'READY');
    expect(res.status()).toBe(400);
    // order is untouched
    expect(await readStatus(request, id)).toBe('PENDING');
  });

  test('guard: illegal transition PENDING → DELIVERED is rejected (400)', async ({ request }) => {
    const id = await createOrder(request);
    const res = await patchStatus(request, id, 'DELIVERED');
    expect(res.status()).toBe(400);
    expect(await readStatus(request, id)).toBe('PENDING');
  });

  test('guard: terminal REJECTED cannot transition onward (400)', async ({ request }) => {
    const id = await createOrder(request);
    expect((await patchStatus(request, id, 'REJECTED')).status()).toBe(200);
    expect(await readStatus(request, id)).toBe('REJECTED');
    const res = await patchStatus(request, id, 'CONFIRMED');
    expect(res.status()).toBe(400);
    expect(await readStatus(request, id)).toBe('REJECTED');
  });
});
