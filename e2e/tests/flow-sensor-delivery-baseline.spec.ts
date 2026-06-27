import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

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
test.beforeAll(() => requireStaging(BASE)); // mutating spec (creates orders, drives lifecycle) — never hit prod

async function demoTarget(request: APIRequestContext) {
  const [info, menu] = await Promise.all([
    request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } }),
    request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } }),
  ]);
  expect(info.status(), `demo info: ${await info.text()}`).toBe(200);
  expect(menu.status(), `demo menu: ${await menu.text()}`).toBe(200);
  const loc = await info.json();
  const m = await menu.json();
  expect(typeof loc.lat, 'venue lat numeric').toBe('number');
  expect(typeof loc.lng, 'venue lng numeric').toBe('number');
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
  expect(mock.status(), `mock-auth: ${await mock.text()}`).toBe(200);
  const { access_token: courierTok, userId: courierId } = await mock.json();
  expectJwt(courierTok, 'courier access_token');
  expectUuid(courierId, 'courier userId');
  const authHeader = { Authorization: `Bearer ${courierTok}` };

  const asg = await request.post('/api/dev/create-assignment', { data: { orderId, courierId, locationId: t.locationId }, headers: devHeaders });
  expect(asg.status(), `create-assignment: ${await asg.text()}`).toBe(200);
  const assignmentId = (await asg.json()).assignmentId as string;
  expectUuid(assignmentId, 'assignment id');

  // accept (→ accepted, order CONFIRMED) → picked-up → delivered (no cash).
  expect((await request.post(`/api/courier/assignments/${assignmentId}/accept`, { headers: authHeader })).status(), 'accept').toBe(200);
  expect((await request.post(`/api/courier/assignments/${assignmentId}/picked-up`, { headers: authHeader })).status(), 'picked-up').toBe(200);
  const delivered = await request.post(`/api/courier/assignments/${assignmentId}/delivered`, {
    data: { cash_collected: false },
    headers: authHeader,
  });
  expect(delivered.status(), `delivered: ${await delivered.text()}`).toBe(200);

  // The §1.2 baseline write rides this DELIVERED txn (route_distance_m + expected_delivery_min into
  // delivery_trace). The body proves the transition COMMITTED (success:true is sent only after the
  // delivery_trace INSERT + COMMIT); DB-state value proven out-of-band via psql (recorded in commit).
  expect((await delivered.json()).success, 'DELIVERED txn (§1.2 baseline write) committed').toBe(true);
});

test('lifecycle rejects out-of-order transitions, replays, and a foreign courier', async ({ request }) => {
  const t = await demoTarget(request);

  const created = await request.post('/api/orders', {
    data: {
      locationId: t.locationId,
      type: 'delivery',
      items: [{ product_id: t.productId, quantity: 1 }],
      customer: { phone: uniquePhone(), name: 'E2E Matrix' },
      delivery: { pin: { lat: t.lat + 0.018, lng: t.lng }, address_text: 'Demo HQ +2km' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
      acknowledged_codes: ['velocity'],
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const orderId = (await created.json()).id as string;
  expectUuid(orderId, 'matrix order id');

  const mock = await request.post('/api/dev/mock-auth', { data: { role: 'courier', locationId: t.locationId }, headers: devHeaders });
  expect(mock.status(), `mock-auth: ${await mock.text()}`).toBe(200);
  const { access_token: courierTok, userId: courierId } = await mock.json();
  expectJwt(courierTok, 'courier access_token');
  expectUuid(courierId, 'courier userId');
  const authHeader = { Authorization: `Bearer ${courierTok}` };

  const asg = await request.post('/api/dev/create-assignment', { data: { orderId, courierId, locationId: t.locationId }, headers: devHeaders });
  expect(asg.status(), `create-assignment: ${await asg.text()}`).toBe(200);
  const assignmentId = (await asg.json()).assignmentId as string;
  expectUuid(assignmentId, 'assignment id');

  // picked-up before accept (status 'assigned') → 404 NOT_ACCEPTED (assignments.ts:241).
  expect((await request.post(`/api/courier/assignments/${assignmentId}/picked-up`, { headers: authHeader })).status(), 'picked-up before accept').toBe(404);
  // delivered before pickup (status 'assigned') → 404 NOT_PICKED_UP (assignments.ts:303).
  expect((await request.post(`/api/courier/assignments/${assignmentId}/delivered`, { data: { cash_collected: false }, headers: authHeader })).status(), 'delivered before accept').toBe(404);

  // Cross-courier IDOR control: a DIFFERENT courier (mock-auth mints a fresh random courierId)
  // on the SAME tenant cannot accept this assignment → 404 (acceptCourierAssignment scopes by
  // courier_id). TODO(needs_staging): a true cross-TENANT control needs a 2nd real tenant's
  // assignment id — a random/nil locationId only 404s by absence, proving nothing.
  const foreign = await request.post('/api/dev/mock-auth', { data: { role: 'courier', locationId: t.locationId }, headers: devHeaders });
  expect(foreign.status(), `foreign mock-auth: ${await foreign.text()}`).toBe(200);
  const foreignTok = (await foreign.json()).access_token as string;
  expectJwt(foreignTok, 'foreign courier token');
  expect((await request.post(`/api/courier/assignments/${assignmentId}/accept`, { headers: { Authorization: `Bearer ${foreignTok}` } })).status(), 'foreign courier accept').toBe(404);

  // Happy path with mid-sequence violations interleaved.
  expect((await request.post(`/api/courier/assignments/${assignmentId}/accept`, { headers: authHeader })).status(), 'accept').toBe(200);
  // delivered before pickup (now 'accepted') → still 404.
  expect((await request.post(`/api/courier/assignments/${assignmentId}/delivered`, { data: { cash_collected: false }, headers: authHeader })).status(), 'delivered before pickup').toBe(404);
  expect((await request.post(`/api/courier/assignments/${assignmentId}/picked-up`, { headers: authHeader })).status(), 'picked-up').toBe(200);

  const delivered = await request.post(`/api/courier/assignments/${assignmentId}/delivered`, { data: { cash_collected: false }, headers: authHeader });
  expect(delivered.status(), `delivered: ${await delivered.text()}`).toBe(200);
  expect((await delivered.json()).success, 'delivered committed').toBe(true);
  // Replay delivered (status now 'delivered') → 404, NOT a second success (assignments.ts:303).
  expect((await request.post(`/api/courier/assignments/${assignmentId}/delivered`, { data: { cash_collected: false }, headers: authHeader })).status(), 'replay delivered').toBe(404);
});
