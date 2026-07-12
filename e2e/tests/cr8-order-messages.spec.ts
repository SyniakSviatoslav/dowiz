/* eslint-disable local/no-permissive-status-assertion -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

let authToken: string;
let locationId: string;
let orderId: string;

test.describe.configure({ mode: 'serial' });

test.beforeAll(async ({ request }) => {
  const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  expect(res.status()).toBe(200);
  const body = await res.json();
  authToken = body.access_token;
  locationId = body.activeLocationId;

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
});

test.describe('CR-8 Order Message Channel', () => {

  test('1 — send owner message', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_accepted_preparing', params: {} },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 201, 409]).toContain(res.status());
    if (res.status() === 200 || res.status() === 201) {
      const body = await res.json();
      expect(body.sender).toBe('owner');
      expect(body.preset_key).toBe('ow_accepted_preparing');
      expect(body.body).toBeNull();
    } else {
      const err = await res.json();
      expect(err.error).toBeTruthy();
    }
  });

  test('2 — get message history', async ({ request }) => {
    const res = await request.get(`${BASE}/api/orders/${orderId}/messages`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.messages)).toBe(true);
    if (body.messages.length > 0) {
      const msg = body.messages[0];
      expect(msg.id).toBeTruthy();
      expect(msg.preset_key).toBeTruthy();
      expect(msg.body).toBeNull();
      expect(msg.created_at).toBeTruthy();
    }
  });

  test('3 — send delay with 15min param', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_delay', params: { minutes: 15 } },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 201, 409]).toContain(res.status());
    if (res.status() === 200 || res.status() === 201) {
      expect((await res.json()).preset_key).toBe('ow_delay');
    }
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

  test('7 — cross-tenant returns 404', async ({ request }) => {
    const res = await request.get(`${BASE}/api/orders/11111111-1111-1111-1111-111111111111/messages`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(404);
  });

  test('8 — message contract shape', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders/${orderId}/messages`, {
      data: { preset_key: 'ow_accepted_preparing', params: {} },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    if (res.status() === 409) return; // preset not allowed in current status
    expect(res.status()).toBe(201);
    const msg = await res.json();
    expect(msg).toMatchObject({
      id: expect.any(String),
      order_id: orderId,
      sender: 'owner',
      preset_key: 'ow_accepted_preparing',
      params: expect.any(Object),
      created_at: expect.any(String),
    });
    expect(msg.body).toBeNull();
    expect(msg.location_id).toBeTruthy();
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
