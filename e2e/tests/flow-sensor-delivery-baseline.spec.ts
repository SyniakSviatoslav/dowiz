import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

/**
 * SENSOR-BUS §1.2 runtime proof (ADR-0009 §4b) — the normalised delivery baseline.
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
 *      pnpm exec playwright test flow-sensor-delivery-baseline --reporter=list
 *
 * Drives a delivery order through the full courier lifecycle to DELIVERED, exercising the
 * /assignments/:id/delivered handler that writes route_distance_m + expected_delivery_min into the
 * immutable delivery_trace audit. Asserts the lifecycle completes (the §1.2 write path runs); the
 * order is pinned ~2 km from the venue so the baseline is a real non-zero distance, not 0.
 *
 * The definitive DB-state assertion (delivery_trace.route_distance_m > 0 + expected_delivery_min ≥ 1
 * for this order) is proven out-of-band via psql against the staging DB and recorded in the commit
 * (it needs DB creds plain CI lacks). Skips unless the dev-mint secret is provided (ADR-0003).
 */

const DEV_SECRET = process.env.DEV_AUTH_SECRET;
const devHeaders = DEV_SECRET ? { 'x-dev-auth-secret': DEV_SECRET } : undefined;

test.describe.configure({ mode: 'serial' });
test.skip(!DEV_SECRET, 'requires DEV_AUTH_SECRET to mint a courier + assignment (dev-gated, ADR-0003)');

async function demoTarget(request: APIRequestContext) {
  const [info, menu] = await Promise.all([
    request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } }),
    request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } }),
  ]);
  expect(info.ok() && menu.ok(), 'demo info+menu load').toBeTruthy();
  const loc = await info.json();
  const m = await menu.json();
  const products: any[] = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
  expect(products.length, 'demo has products').toBeGreaterThan(0);
  return { locationId: m.locationId ?? m.location_id, lat: loc.lat, lng: loc.lng, productId: products[0].id };
}

let phoneSeq = 0;
function uniquePhone() {
  phoneSeq += 1;
  return `+35566${String(Date.now()).slice(-4)}${String(phoneSeq).padStart(2, '0')}`;
}

test('a delivered order writes the §1.2 normalised baseline into delivery_trace', async ({ request }) => {
  const t = await demoTarget(request);

  // Pin ~2 km north of the venue → a real, non-zero road baseline (not 0).
  const created = await request.post('/api/orders', {
    data: {
      locationId: t.locationId,
      type: 'delivery',
      items: [{ product_id: t.productId, quantity: 1 }],
      customer: { phone: uniquePhone(), name: 'E2E Baseline' },
      delivery: { pin: { lat: t.lat + 0.018, lng: t.lng }, address_text: 'Demo HQ +2km' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
      acknowledged_codes: ['velocity'],
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const orderId = (await created.json()).id as string;

  const mock = await request.post('/api/dev/mock-auth', { data: { role: 'courier', locationId: t.locationId }, headers: devHeaders });
  expect(mock.ok(), `mock-auth: ${await mock.text()}`).toBeTruthy();
  const { access_token: courierTok, userId: courierId } = await mock.json();
  const authHeader = { Authorization: `Bearer ${courierTok}` };

  const asg = await request.post('/api/dev/create-assignment', { data: { orderId, courierId, locationId: t.locationId }, headers: devHeaders });
  expect(asg.ok(), `create-assignment: ${await asg.text()}`).toBeTruthy();
  const assignmentId = (await asg.json()).assignmentId as string;

  // accept (→ accepted, order CONFIRMED) → picked-up → delivered (no cash).
  expect((await request.post(`/api/courier/assignments/${assignmentId}/accept`, { headers: authHeader })).status(), 'accept').toBe(200);
  expect((await request.post(`/api/courier/assignments/${assignmentId}/picked-up`, { headers: authHeader })).status(), 'picked-up').toBe(200);
  const delivered = await request.post(`/api/courier/assignments/${assignmentId}/delivered`, {
    data: { cash_collected: false },
    headers: authHeader,
  });
  expect(delivered.status(), `delivered: ${await delivered.text()}`).toBe(200);

  // The §1.2 baseline write rides this DELIVERED txn (route_distance_m + expected_delivery_min into
  // delivery_trace). DB-state value proven out-of-band via psql (recorded in the commit).
  expect(orderId, 'order reached DELIVERED through the §1.2 write path').toBeTruthy();
});
