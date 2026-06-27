import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

let authToken: string;
let locationId: string;
let orderId: string;
let anySendSucceeded = false;

test.describe.configure({ mode: 'serial' });

test.beforeAll(async ({ request }) => {
  requireStaging(BASE); // mutating spec (creates orders, posts messages, seeds) — never hit prod
  const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  expect(res.status()).toBe(200);
  const body = await res.json();
  authToken = body.access_token;
  locationId = body.activeLocationId;
  expectJwt(authToken, 'owner access_token');
  expectUuid(locationId, 'activeLocationId');

  const infoRes = await request.get(`${BASE}/public/locations/demo/info`);
  const locationSlug = infoRes.ok() ? (await infoRes.json()).slug : 'demo';
  const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
  expect(menuRes.ok()).toBe(true);
  const menu = await menuRes.json();
  const pid = (menu.products || menu.items || menu.data || [])[0]?.id;

  const orderRes = await request.post(`${BASE}/api/orders`, {
    data: {
      location_id: locationId,
      items: [{ product_id: pid, quantity: 1 }],
      delivery_type: 'delivery',
      customer_name: 'CR8E2E',
      customer_phone: '+355600000099',
      delivery_address: 'Rruga e Barrikadave, Tirana',
    },
  });
  if (orderRes.status() === 200 || orderRes.status() === 201) {
    orderId = (await orderRes.json()).id;
  } else {
    const existingRes = await request.get(`${BASE}/api/owner/orders`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    if (existingRes.ok()) {
      const data = await existingRes.json();
      const orders = data.orders || data.data || data;
      if (Array.isArray(orders) && orders[0]?.id) orderId = orders[0].id;
    }
  }
  // Finding 1 — fail fast: the suite is meaningless without a real order to message. No silent
  // fallback that leaves orderId undefined (would make every test below address `undefined`).
  expectUuid(orderId, 'orderId (beforeAll setup must produce a real order)');
});

test.describe('CR-8 Order Message Channel', () => {

  test.afterAll(() => {
    // Finding 2 — non-vacuity guard: at least one owner-message send must have reached the 201
    // path across tests 1/3/8. If every send 409'd, the send pipeline was never exercised yet
    // the suite went green — that is the false-green this asserts against.
    expect(anySendSucceeded, 'no owner-message send reached 201 — send path never exercised').toBe(true);
  });

  test('1 — send owner message', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_accepted_preparing', params: {} },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    if (res.status() === 409) {
      const err = await res.json();
      expect(err.error).toBeTruthy();
      return; // preset not allowed in current order status
    }
    expect(res.status()).toBe(201);
    const body = await res.json();
    expect(body.success).toBe(true);
    expect(body.message.sender).toBe('owner');
    expect(body.message.preset_key).toBe('ow_accepted_preparing');
    expect(body.message.body).toBeNull();
    anySendSucceeded = true;
  });

  test('2 — get message history', async ({ request }) => {
    const res = await request.get(`${BASE}/api/orders/${orderId}/messages`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.messages)).toBe(true);
    // Finding 3 — non-vacuous: test 1 (serial) must have produced ≥1 message, so assert the
    // shape unconditionally instead of skipping the whole block when the list is empty.
    // TODO(needs_staging): requires the seeded order in a send-allowed status so a message exists.
    expect(body.messages.length).toBeGreaterThan(0);
    const msg = body.messages[0];
    expectUuid(msg.id);
    expect(msg.preset_key).toBeTruthy();
    expect(msg.body).toBeNull();
    expect(msg.created_at).toBeTruthy();
  });

  test('3 — send delay with 15min param', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_delay', params: { minutes: 15 } },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    if (res.status() === 409) return; // preset not allowed in current order status
    expect(res.status()).toBe(201);
    expect((await res.json()).message.preset_key).toBe('ow_delay');
    anySendSucceeded = true;
  });

  test('4 — unknown preset returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'nonexistent_preset', params: {} },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(400);
  });

  test('5 — non-existent order returns 404', async ({ request }) => {
    const res = await request.get(`${BASE}/api/orders/00000000-0000-0000-0000-000000000000/messages`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(404);
  });

  test('6 — no auth returns 401', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_accepted_preparing', params: {} },
    });
    expect(res.status()).toBe(401);
  });

  test('6b — foreign-role token cannot send owner preset', async ({ request }) => {
    // Finding 6 — second negative control (beyond no-token in test 6). mock-auth mints only
    // owner|courier (no customer role), so we use a courier token: it is not a member of this
    // location, so order-messages.ts tenant-isolates the owner-preset send. Asserts the EXACT
    // status the route returns (404 NOT_FOUND), not an assumed 403 (no 403 path exists there).
    // TODO(needs_staging): to exercise the 409 sender-role-mismatch path specifically, seed a
    // SAME-tenant non-owner identity that owns this order; live staging + fixture required.
    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierRes.status()).toBe(200);
    const courierToken = (await courierRes.json()).access_token;
    expectJwt(courierToken, 'courier access_token');
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_accepted_preparing', params: {} },
      headers: { Authorization: `Bearer ${courierToken}` },
    });
    expect(res.status()).toBe(404);
  });

  test('7 — cross-tenant returns 404 (real 2nd tenant)', async ({ request }) => {
    // Finding 5 — real IDOR: seed a SECOND tenant (vis-open, owned by vis-owner ≠ the demo
    // owner) with its own order, then assert the demo owner token cannot read its messages.
    // Proves access-block, not absence (the old all-1s UUID just 404'd by not existing).
    // TODO(needs_staging): requires the gated dev seed endpoint live on staging (/api/dev).
    const seedRes = await request.post(`${BASE}/api/dev/seed-visual-state`, { data: {} });
    expect(seedRes.status()).toBe(200);
    const otherOrderId = (await seedRes.json()).orderId;
    expectUuid(otherOrderId, 'second-tenant orderId');
    const res = await request.get(`${BASE}/api/orders/${otherOrderId}/messages`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(404);
  });

  test('8 — message contract shape', async ({ request }) => {
    // Finding 4 — the only full contract assertion: must NOT be permanently skippable. Use a
    // distinct, broad-state owner preset (ow_high_load → PENDING/CONFIRMED/PREPARING) so the
    // 201 path is exercised, and drop the 409 early-return so the toMatchObject always runs.
    // TODO(needs_staging): if the seeded order is terminal/non-allowed this 409s — seed/reset
    // order state to guarantee 201 on live staging.
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_high_load', params: {} },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const { message: msg } = await res.json();
    expect(msg).toMatchObject({
      id: expect.any(String),
      order_id: orderId,
      sender: 'owner',
      preset_key: 'ow_high_load',
      params: expect.any(Object),
      created_at: expect.any(String),
    });
    expect(msg.body).toBeNull();
    expectUuid(msg.id);
    expectUuid(msg.location_id);
    anySendSucceeded = true;
  });

  test('9 — mark-read endpoint', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages/read`, {
      data: {},
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    expect((await res.json()).success).toBe(true);
  });
});
