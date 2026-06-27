import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const BOT_SECRET = process.env.TELEGRAM_BOT_SECRET;
const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN;
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
    // Guard: this spec MUTATES (creates locations/orders, transitions status) — never run it
    // against prod. And fail fast (not vacuously green) if the Telegram bot token is absent —
    // without it getUpdates hits /botundefined/ and the message assertions below have nothing
    // to match, so an empty result set would slip through as a false pass.
    requireStaging(BASE);
    expect(BOT_TOKEN ?? '', 'TELEGRAM_BOT_TOKEN must be a real bot token, not undefined').toMatch(/^\d+:[\w-]+$/);
    // Clear any pending updates to have clean slate
    const updates = await sendTelegramGetUpdates();
    if (updates.result && updates.result.length > 0) {
      lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
    }
  });

  test('should receive telegram notifications for order.created and order.delivered from real service events', async () => {
    // 0. Telegram connect / verify target active (reuse steps from telegram-full-flow)
    // Get owner token via mock-auth
    const authRes = await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' });
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
    ownerToken = authBody.access_token;
    expectJwt(ownerToken, 'ownerToken');
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
    expectUuid(locationId, 'locationId');

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
    expectUuid(connectToken, 'connectToken');

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
    // The /start webhook is processed asynchronously, so reading targets immediately races it.
    // Poll (≤15s) until our target is active. ponytail: bounded poll. ceiling: a worker slower
    // than the budget fails (not a silent pass on a missing target).
    // TODO(needs_staging): CHAT_ID is a fixed integer, so a target left 'active' by a PRIOR run
    // could satisfy this without the current webhook completing — a per-run fixture (unique chat
    // id linked via THIS run's connectToken) is required to make it fully non-vacuous.
    let ourTarget: any;
    const targetDeadline = Date.now() + 15000;
    do {
      const targetsRes = await fetch(`${BASE}/api/owner/locations/${locationId}/notifications/targets`, {
        headers: await authHeaders(ownerToken),
      });
      expect(targetsRes.status).toBe(200);
      const targetsBody = await targetsRes.json();
      const tgTargets = targetsBody.targets.filter((t: any) => t.channel === 'telegram');
      ourTarget = tgTargets.find((t: any) => t.address === String(CHAT_ID));
      if (ourTarget?.status === 'active') break;
      await new Promise(r => setTimeout(r, 2000));
    } while (Date.now() < targetDeadline);
    expect(ourTarget, 'telegram target for CHAT_ID must exist').toBeDefined();
    expect(ourTarget.status).toBe('active');

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
    expectUuid(productId, 'productId');

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
    expectUuid(orderId, 'orderId');
    // The Telegram template renders `#${orderId.substring(0,4).toUpperCase()}` (notifications/
    // workers/index.ts) — anchor matches to THIS order so an unrelated 'NEW ORDER' message can't
    // satisfy them (finding: text.includes('NEW ORDER') alone is forgeable by any stray message).
    const shortId = orderId.substring(0, 4).toUpperCase();

    // 5-6. Poll (≤20s) for the order.created message anchored to shortId. ponytail: bounded poll
    // replaces the fixed 3s sleep. ceiling: a worker slower than the budget fails — but a fast
    // broken pipeline can no longer pass vacuously on an empty result set (the toContain below
    // fails when no anchored message arrived).
    let orderCreatedMsg: string | null = null;
    const createdDeadline = Date.now() + 20000;
    do {
      const updates = await sendTelegramGetUpdates(lastUpdateId + 1);
      expect(updates.ok).toBe(true);
      for (const upd of updates.result) {
        if (upd.message?.chat?.id === CHAT_ID) {
          const text = upd.message.text;
          if (text && text.includes('NEW ORDER') && text.includes(`#${shortId}`)) {
            orderCreatedMsg = text;
          }
        }
      }
      if (updates.result.length > 0) {
        lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
      }
      if (orderCreatedMsg) break;
      await new Promise(r => setTimeout(r, 2000));
    } while (Date.now() < createdDeadline);
    expect(orderCreatedMsg, `order.created Telegram message for #${shortId} not found`).toContain(`NEW ORDER #${shortId}`);

    // 7. Authz controls on PATCH /orders/:id/status BEFORE the valid transition — neither a 401
    //    nor a 403 reaches the handler (preHandler [verifyAuth, requireRole(['owner'])]), so the
    //    order state is untouched.
    //    NEGATIVE — no token → 401 (verifyAuth).
    const noAuthPatch = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'DELIVERED' }),
    });
    expect(noAuthPatch.status).toBe(401);
    //    NEGATIVE — wrong role (courier) → 403 (requireRole(['owner'])).
    const courierAuth = await fetch(`${BASE}/api/dev/mock-auth`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'courier' }),
    });
    expect(courierAuth.status).toBe(200);
    const courierToken = (await courierAuth.json()).access_token;
    expectJwt(courierToken, 'courierToken');
    const wrongRolePatch = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      method: 'PATCH',
      headers: await authHeaders(courierToken),
      body: JSON.stringify({ status: 'DELIVERED' }),
    });
    expect(wrongRolePatch.status).toBe(403);
    // TODO(needs_staging): true cross-tenant authz — a SECOND tenant's owner PATCHing this order
    // must get 404 (orders.ts withTenant scopes the SELECT by user.userId → no rows). /dev/mock-auth
    // always mints the SAME dev owner, so it cannot produce a foreign owner; a real 2nd-tenant
    // fixture is required to assert the 404 without faking it.

    // 7b. POSITIVE — the order's own owner delivers it.
    const deliverRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      headers: await authHeaders(ownerToken),
      method: 'PATCH',
      body: JSON.stringify({ status: 'DELIVERED' }),
    });
    expect(deliverRes.status).toBe(200, await deliverRes.text());

    // 8-9. Poll (≤20s) for the order.delivered message anchored to this order's shortId.
    let orderDeliveredMsg: string | null = null;
    const deliveredDeadline = Date.now() + 20000;
    do {
      const updates = await sendTelegramGetUpdates(lastUpdateId + 1);
      expect(updates.ok).toBe(true);
      for (const upd of updates.result) {
        if (upd.message?.chat?.id === CHAT_ID) {
          const text = upd.message.text;
          if (text && text.includes('ORDER DELIVERED') && text.includes(`#${shortId}`)) {
            orderDeliveredMsg = text;
          }
        }
      }
      if (updates.result.length > 0) {
        lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
      }
      if (orderDeliveredMsg) break;
      await new Promise(r => setTimeout(r, 2000));
    } while (Date.now() < deliveredDeadline);
    expect(orderDeliveredMsg, `order.delivered Telegram message for #${shortId} not found`).toContain(`ORDER DELIVERED #${shortId}`);
    // TODO(needs_staging): opt-out negative scenario — disable the 'order.created' category for
    // this location, place another order, and assert NO matching Telegram message arrives within
    // the budget (the preference-centre control the happy path never exercises). Requires a live
    // staging run.
  });
});
