import { test, expect } from '@playwright/test';

const BASE = 'https://dowiz.fly.dev';
const BOT_SECRET = 'Ihatenuclearwar';
const BOT_TOKEN = '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';
const CHAT_ID = 999999;

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

async function authHeaders(token: string) {
  return { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
}

async function sendTelegramGetUpdates(offset?: number) {
  const url = `https://api.telegram.org/bot${BOT_TOKEN}/getUpdates${offset ? `?offset=${offset}&limit=100&timeout=0` : ''}`;
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`Telegram getUpdates failed: ${resp.status}`);
  return await resp.json();
}

test.describe('Real service event notifications → Telegram', () => {
  let ownerToken: string;
  let locationId: string;
  let productId: string;
  let lastUpdateId: number = 0;

  test.beforeAll(async () => {
    // Clear any pending updates to have clean slate
    const updates = await sendTelegramGetUpdates();
    if (updates.result && updates.result.length > 0) {
      lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
    }
  });

  test('should receive telegram notifications for order.created and order.delivered from real service events', async ({}) => {
    // 0. Telegram connect / verify target active (reuse steps from telegram-full-flow)
    // Get owner token via mock-auth
    const authRes = await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' });
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
    ownerToken = authBody.access_token;
    const ownerId = authBody.userId;

    // Create a location via onboarding (skip to active)
    const startOnboarding = await fetch(`${BASE}/api/owner/onboarding/start`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
      body: JSON.stringify({
        name: 'Test Loc for Real Notifs',
        phone: '+355600000000',
        slug: `test-real-notif-${uuid()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    expect(startOnboarding.status).toBe(201);
    const onboardingBody = await startOnboarding.json();
    locationId = onboardingBody.locationId;

    // Complete onboarding steps 1-6
    for (let step = 1; step <= 6; step++) {
      const stepRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        headers: await authHeaders(ownerToken),
        method: 'POST',
        body: JSON.stringify({ step }),
      });
      expect(stepRes.status).toBe(200);
    }
    // Skip step 7 (Telegram Alerts) – we will handle telegram connect manually
    const skip7 = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/7/skip`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
    });
    expect(skip7.status).toBe(200);
    // Complete step 8 (Publish & Go Live)
    const finish = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
      body: JSON.stringify({ step: 8 }),
    });
    expect(finish.status).toBe(200);
    const finishBody = await finish.json();
    expect(finishBody.completed).toBe(true);

    // 1. Generate Telegram connect token
    const connectInitRes = await fetch(`${BASE}/api/owner/locations/${locationId}/notifications/telegram/connect-init`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
    });
    expect(connectInitRes.status).toBe(200);
    const connectInitBody = await connectInitRes.json();
    const connectToken = connectInitBody.token;
    expect(connectToken).toBeTruthy();

    // 2. Simulate /start <token> from Telegram
    const startStatus = await fetch(`https://api.telegram.org/bot${BOT_TOKEN}/sendMessage`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        chat_id: CHAT_ID,
        text: `/start ${connectToken}`,
      }),
    });
    // Note: We don't need to check status; the webhook will process it.
    // Instead, verify target becomes active via API.
    const targetsRes = await fetch(`${BASE}/api/owner/locations/${locationId}/notifications/targets`, {
      headers: await authHeaders(ownerToken),
    });
    expect(targetsRes.status).toBe(200);
    const targetsBody = await targetsRes.json();
    const tgTargets = targetsBody.targets.filter((t: any) => t.channel === 'telegram');
    expect(tgTargets.length).toBeGreaterThanOrEqual(1);
    const ourTarget = tgTargets.find((t: any) => t.address === String(CHAT_ID));
    expect(ourTarget).toBeTruthy();
    expect(ourTarget.status).toBe('active');
    // (Optional) update lastUpdateId after the /start webhook? We'll just get updates after we send order.

    // 3. Create a product
    const productRes = await fetch(`${BASE}/api/owner/locations/${locationId}/products`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
      body: JSON.stringify({
        name: 'Real Notification Test Product',
        price: 1500,
        category_id: null,
        available: true,
      }),
    });
    expect(productRes.status).toBe(201);
    const productBody = await productRes.json();
    productId = productBody.id;

    // 4. Create an order via customer endpoint (no auth)
    const orderPayload = {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: '+355600000002', name: 'Real Test Customer' },
      delivery: { address_text: '123 Real St', pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      // No idempotency key to avoid conflict
    };
    const orderRes = await fetch(`${BASE}/api/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload),
    });
    expect(orderRes.status).toBe(201, await orderRes.text());
    const orderBody = await orderRes.json();
    const orderId = orderBody.id;
    expect(orderId).toBeTruthy();

    // 5. Wait a moment for event processing
    await new Promise(r => setTimeout(r, 3000));

    // 6. Check for order.created telegram message
    const updatesAfterOrder = await sendTelegramGetUpdates(lastUpdateId + 1);
    expect(updatesAfterOrder.ok).toBeTruthy();
    const result = updatesAfterOrder.result;
    let orderCreatedMsg = null;
    for (const upd of result) {
      if (upd.message && upd.message.chat?.id === CHAT_ID) {
        const text = upd.message.text;
        if (text && text.includes('NEW ORDER')) {
          orderCreatedMsg = text;
          break;
        }
      }
    }
    expect(orderCreatedMsg).toBeTruthy(`Expected order.created telegram message not found. Updates: ${JSON.stringify(result)}`);

    // Update lastUpdateId to latest processed
    if (updatesAfterOrder.result.length > 0) {
      lastUpdateId = updatesAfterOrder.result[updatesAfterOrder.result.length - 1].update_id + 1;
    }

    // 7. Deliver the order via owner PATCH /api/orders/:id/status
    const deliverRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      headers: await authHeaders(ownerToken),
      method: 'PATCH',
      body: JSON.stringify({ status: 'DELIVERED' }),
    });
    expect(deliverRes.status).toBe(200, await deliverRes.text());

    // 8. Wait for event processing
    await new Promise(r => setTimeout(r, 3000));

    // 9. Check for order.delivered telegram message
    const updatesAfterDeliver = await sendTelegramGetUpdates(lastUpdateId + 1);
    expect(updatesAfterDeliver.ok).toBeTruthy();
    const result2 = updatesAfterDeliver.result;
    let orderDeliveredMsg = null;
    for (const upd of result2) {
      if (upd.message && upd.message.chat?.id === CHAT_ID) {
        const text = upd.message.text;
        if (text && text.includes('ORDER DELIVERED')) {
          orderDeliveredMsg = text;
          break;
        }
      }
    }
    expect(orderDeliveredMsg).toBeTruthy(`Expected order.delivered telegram message not found. Updates: ${JSON.stringify(result2)}`);
  });
});
