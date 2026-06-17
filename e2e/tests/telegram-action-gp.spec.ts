/**
 * Telegram Action E2E — Golden Path (GP) Tests
 *
 * GP-1  Accept:    new order → owner taps ✅ in Telegram → order CONFIRMED
 * GP-2  Reject:    new order → owner taps ❌ reason → order REJECTED
 * GP-3  Double-tap / stale: re-tap ✅ on already-CONFIRMED order → no-op (200, idempotent)
 *
 * All tests simulate Telegram callbacks via the live webhook endpoint (no real bot/userbot needed).
 * Setup runs once in beforeAll; individual tests share locationId / productId / notificationTargetId.
 */

import { test, expect } from '@playwright/test';

const BASE = 'https://dowiz.fly.dev';
const BOT_SECRET = 'Ihatenuclearwar';
const WEBHOOK_URL = `${BASE}/webhook/telegram/${BOT_SECRET}`;

const CHAT_ID = 977700000;
const TEST_SLUG = `gp-e2e-${Date.now()}`;

let authToken: string;
let locationId: string;
let productId: string;

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

async function authHeaders() {
  return {
    Authorization: `Bearer ${authToken}`,
    'Content-Type': 'application/json',
  };
}

async function postWebhook(data: Record<string, unknown>): Promise<Response> {
  return fetch(WEBHOOK_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-telegram-bot-api-secret-token': BOT_SECRET,
    },
    body: JSON.stringify(data),
  });
}

async function createOrder(productId: string, locationId: string): Promise<string> {
  const res = await fetch(`${BASE}/api/orders`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
      customer: { phone: '+355600000001', name: 'GP Tester' },
      delivery: { pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      idempotency_key: uuid(),
      acknowledged_codes: [],
    }),
  });
  expect(res.status).toBe(201);
  const body = await res.json() as { id: string };
  expect(body.id).toBeTruthy();
  return body.id;
}

async function getOrderStatus(orderId: string): Promise<string | null> {
  const res = await fetch(
    `${BASE}/api/owner/locations/${locationId}/dashboard/snapshot`,
    { headers: await authHeaders() },
  );
  if (!res.ok) return null;
  const body = await res.json() as { orders?: Array<{ orderId: string; status: string }> };
  const order = body.orders?.find(o => o.orderId === orderId);
  return order?.status ?? null;
}

// ════════════════════════════════════════════════════════════════════════════
// SETUP — runs once before all GP tests
// ════════════════════════════════════════════════════════════════════════════

test.describe('Telegram Action GP Tests — Live https://dowiz.fly.dev', () => {
  test.describe.configure({ mode: 'serial' });

  test('SETUP-1: mock-auth returns owner token', async ({ request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.access_token).toBeTruthy();
    authToken = body.access_token;
  });

  test('SETUP-2: create test location', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/start`, {
      headers: await authHeaders(),
      data: {
        name: 'GP Action Tests',
        phone: '+355600000099',
        slug: TEST_SLUG,
        currency_code: 'ALL',
        default_locale: 'sq',
        supported_locales: ['sq', 'en'],
      },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    locationId = body.locationId;
    expect(locationId).toBeTruthy();
  });

  test('SETUP-3: complete onboarding steps 1–3', async ({ request }) => {
    for (const step of [1, 2, 3]) {
      const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        headers: await authHeaders(),
        data: { step },
      });
      expect(res.status()).toBe(200);
    }
  });

  test('SETUP-4: complete/skip onboarding steps 4–8 in order', async ({ request }) => {
    // Skip step 4 (Branding)
    await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/4/skip`, { headers: await authHeaders() });
    // Skip step 5 (Delivery Settings)
    await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/5/skip`, { headers: await authHeaders() });
    // Complete step 6 (Publish & Share)
    const s6 = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 6 },
    });
    expect(s6.status()).toBe(200);
    // Skip step 7 (Telegram Alerts)
    await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/7/skip`, { headers: await authHeaders() });
    // Complete step 8 (Go Live)
    const s8 = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 8 },
    });
    expect(s8.status()).toBe(200);
    const body = await s8.json();
    expect(body.completed).toBe(true);
  });

  test('SETUP-6: create test product', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/locations/${locationId}/products`, {
      headers: await authHeaders(),
      data: { name: 'GP Test Burger', price: 1500, category_id: null, available: true },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    productId = body.id;
    expect(productId).toBeTruthy();
  });

  test('SETUP-7: connect Telegram via simulated /start', async ({ request }) => {
    const initRes = await request.post(
      `${BASE}/api/owner/locations/${locationId}/notifications/telegram/connect-init`,
      { headers: await authHeaders() },
    );
    expect(initRes.status()).toBe(200);
    const { token } = await initRes.json();
    expect(token).toBeTruthy();

    const webhookRes = await postWebhook({
      message: {
        text: `/start ${token}`,
        chat: { id: CHAT_ID, type: 'private' },
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
      },
    });
    expect(webhookRes.status).toBe(200);

    // Verify target is active
    const targetsRes = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(targetsRes.status()).toBe(200);
    const { targets } = await targetsRes.json();
    const tgTarget = targets.find((t: any) => t.address === String(CHAT_ID) && t.channel === 'telegram');
    expect(tgTarget).toBeTruthy();
    expect(tgTarget.status).toBe('active');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // GP-1  Accept: place order → tap ✅ → status = CONFIRMED
  // ══════════════════════════════════════════════════════════════════════════

  let orderId1: string;

  test('GP-1A: place order', async () => {
    orderId1 = await createOrder(productId, locationId);
    expect(orderId1).toBeTruthy();

    const status = await getOrderStatus(orderId1);
    expect(status).toBe('PENDING');
  });

  test('GP-1B: simulate Telegram ✅ tap (order.confirm callback)', async () => {
    const res = await postWebhook({
      callback_query: {
        id: 'gp1_confirm',
        data: `order.confirm:${orderId1}`,
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
        message: { chat: { id: CHAT_ID }, message_id: 1001, text: 'Order notification' },
      },
    });
    expect(res.status).toBe(200);
  });

  test('GP-1C: order status is now CONFIRMED', async () => {
    const status = await getOrderStatus(orderId1);
    expect(status).toBe('CONFIRMED');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // GP-2  Reject: place order → tap ❌ reason → status = REJECTED
  // ══════════════════════════════════════════════════════════════════════════

  let orderId2: string;

  test('GP-2A: place second order', async () => {
    orderId2 = await createOrder(productId, locationId);
    expect(orderId2).toBeTruthy();
    expect(orderId2).not.toBe(orderId1);

    const status = await getOrderStatus(orderId2);
    expect(status).toBe('PENDING');
  });

  test('GP-2B: simulate Telegram ❌ tap with reject reason (order.reject_reason_1)', async () => {
    const res = await postWebhook({
      callback_query: {
        id: 'gp2_reject',
        data: `order.reject_reason_1:${orderId2}`,
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
        message: { chat: { id: CHAT_ID }, message_id: 1002, text: 'Order notification' },
      },
    });
    expect(res.status).toBe(200);
  });

  test('GP-2C: order status is now REJECTED', async () => {
    const status = await getOrderStatus(orderId2);
    expect(status).toBe('REJECTED');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // GP-3  Double-tap / stale: re-tap ✅ on already-CONFIRMED order → no-op
  // ══════════════════════════════════════════════════════════════════════════

  test('GP-3A: re-tap ✅ on already-CONFIRMED order returns 200 (no-op)', async () => {
    // orderId1 is CONFIRMED from GP-1
    const res = await postWebhook({
      callback_query: {
        id: 'gp3_double_tap',
        data: `order.confirm:${orderId1}`,
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
        message: { chat: { id: CHAT_ID }, message_id: 1001, text: 'Order notification' },
      },
    });
    expect(res.status).toBe(200);
  });

  test('GP-3B: order stays CONFIRMED after double-tap', async () => {
    const status = await getOrderStatus(orderId1);
    expect(status).toBe('CONFIRMED');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // WEBHOOK EDGE CASES (deterministic, no state dependency)
  // ══════════════════════════════════════════════════════════════════════════

  test('EDGE-1: invalid secret token → 401', async () => {
    const res = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': 'not-the-right-secret',
      },
      body: JSON.stringify({ callback_query: { id: 'x', data: 'order.confirm:123', from: { id: CHAT_ID } } }),
    });
    expect(res.status).toBe(401);
  });

  test('EDGE-2: missing secret token → 401', async () => {
    const res = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ callback_query: { id: 'x', data: 'order.confirm:123', from: { id: CHAT_ID } } }),
    });
    expect(res.status).toBe(401);
  });

  test('EDGE-3: callback from unlinked chat_id → 200 (silently unauthorized)', async () => {
    const unknownChatId = 888800001;
    const res = await postWebhook({
      callback_query: {
        id: 'edge3_unlinked',
        data: `order.confirm:${orderId1}`,
        from: { id: unknownChatId, first_name: 'Hacker' },
        message: { chat: { id: unknownChatId }, message_id: 999 },
      },
    });
    expect(res.status).toBe(200);
    // Order must not change (should still be CONFIRMED, not something else)
    const status = await getOrderStatus(orderId1);
    expect(status).toBe('CONFIRMED');
  });

  test('EDGE-4: forged / non-existent orderId in callback_data → 200 (best-effort)', async () => {
    const fakeOrderId = uuid();
    const res = await postWebhook({
      callback_query: {
        id: 'edge4_fake',
        data: `order.confirm:${fakeOrderId}`,
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
        message: { chat: { id: CHAT_ID }, message_id: 1003 },
      },
    });
    expect(res.status).toBe(200);
  });

  test('EDGE-5: idempotent reject → second tap on REJECTED order returns 200 (no-op)', async () => {
    const res = await postWebhook({
      callback_query: {
        id: 'edge5_double_reject',
        data: `order.reject_reason_2:${orderId2}`,
        from: { id: CHAT_ID, first_name: 'GP', last_name: 'Tester' },
        message: { chat: { id: CHAT_ID }, message_id: 1002, text: 'Order notification' },
      },
    });
    expect(res.status).toBe(200);
    // Still rejected
    const status = await getOrderStatus(orderId2);
    expect(status).toBe('REJECTED');
  });
});
