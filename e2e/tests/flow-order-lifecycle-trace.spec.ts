import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

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
 * transitions, (4) terminal states are final. API-level + MUTATING (creates orders,
 * uses the dev mock-auth backdoor), so it is hard-guarded to staging via
 * requireStaging() — it must NEVER run against the prod host.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

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
      customer: { phone: `+35560${String(Date.now()).slice(-6)}`, name: 'Erion Berisha' },
      delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Barrikadave, Tirana' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
    },
  });
  expect(res.status(), `create order: ${await res.text()}`).toBe(201);
  const id = (await res.json()).id;
  expectUuid(id, 'order id');
  return id;
}

/** Mint a dev mock-auth token for an arbitrary role/mode (privilege + isolation controls). */
async function mockAuth(request: any, data: Record<string, unknown>): Promise<string> {
  const res = await request.post(`${BASE}/api/dev/mock-auth`, { data });
  expect(res.status(), `mock-auth ${JSON.stringify(data)}: ${await res.text()}`).toBe(200);
  const token = (await res.json()).access_token;
  expectJwt(token, 'mock-auth token');
  return token;
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
    requireStaging(BASE); // MUTATING + dev-backdoor — fail fast unless target is staging/local

    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    expect(auth.status()).toBe(200);
    const body = await auth.json();
    authToken = body.access_token;
    locationId = body.activeLocationId;
    expectJwt(authToken, 'owner access_token');
    expectUuid(locationId, 'activeLocationId');

    const menu = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(menu.ok()).toBe(true);
    const cats = (await menu.json()).categories || [];
    const products = cats.flatMap((c: any) => c.products || []);
    expect(products.length, 'demo menu has products').toBeGreaterThan(0);
    productId = products[0].id;
    expectUuid(productId, 'productId');
  });

  test('happy path: PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED', async ({ request }) => {
    const id = await createOrder(request);
    expect(await readStatus(request, id)).toBe('PENDING');

    for (const next of ['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'] as const) {
      const res = await patchStatus(request, id, next);
      expect(res.status(), `${next}: ${await res.text()}`).toBe(200);
      expect((await res.json()).status).toBe(next);
      // Independent GET — the PATCH body could echo the requested status without persisting it.
      expect(await readStatus(request, id), `persisted ${next}`).toBe(next);
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

  test('cancel path: PENDING → CANCELLED is terminal (CANCELLED → CONFIRMED rejected 400)', async ({ request }) => {
    const id = await createOrder(request);
    const cancel = await patchStatus(request, id, 'CANCELLED');
    expect(cancel.status(), await cancel.text()).toBe(200);
    expect((await cancel.json()).status).toBe('CANCELLED');
    expect(await readStatus(request, id)).toBe('CANCELLED');
    // CANCELLED is terminal (order-machine.ts: TRANSITIONS.CANCELLED = [])
    const onward = await patchStatus(request, id, 'CONFIRMED');
    expect(onward.status()).toBe(400);
    expect(await readStatus(request, id)).toBe('CANCELLED');
  });

  test('authz: PATCH status with NO Authorization header is rejected (401)', async ({ request }) => {
    const id = await createOrder(request);
    const res = await request.patch(`${BASE}/api/orders/${id}/status`, { data: { status: 'CONFIRMED' } });
    expect(res.status(), await res.text()).toBe(401); // plugins/auth.ts verifyAuth: missing token → 401
    expect(await readStatus(request, id)).toBe('PENDING'); // order untouched
  });

  test('authz: PATCH status with a COURIER token is forbidden (403)', async ({ request }) => {
    const id = await createOrder(request);
    const courierToken = await mockAuth(request, { role: 'courier', locationSlug: 'demo' });
    const res = await request.patch(`${BASE}/api/orders/${id}/status`, {
      data: { status: 'CONFIRMED' },
      headers: { Authorization: `Bearer ${courierToken}` },
    });
    expect(res.status(), await res.text()).toBe(403); // requireRole(['owner']): wrong role → 403
    expect(await readStatus(request, id)).toBe('PENDING'); // order untouched
  });

  test('isolation: an owner with NO membership for the order cannot transition it (404)', async ({ request }) => {
    // A REAL, distinct authenticated owner principal (fresh:true mints a throwaway owner with no
    // location membership) — not a nil-UUID. RLS (withTenant) must hide the demo order → 404.
    // TODO(needs-staging): strengthen to a SECOND real tenant that owns its OWN location+order
    //   (mock-auth's owner path is pinned to the 'demo' tenant, so a true cross-location IDOR
    //   pair cannot be minted here). Requires a 2nd-tenant fixture on staging.
    const id = await createOrder(request);
    const otherOwner = await mockAuth(request, { fresh: true });
    const res = await request.patch(`${BASE}/api/orders/${id}/status`, {
      data: { status: 'CONFIRMED' },
      headers: { Authorization: `Bearer ${otherOwner}` },
    });
    expect(res.status(), await res.text()).toBe(404); // orders.ts: RLS-hidden row → 404 Order not found
    expect(await readStatus(request, id)).toBe('PENDING'); // demo owner still sees it untouched
  });
});
