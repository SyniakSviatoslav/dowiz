import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

async function authHeaders(token: string) {
  return { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
}

test.describe('Service event notifications pipeline', () => {
  test('should create order and advance through lifecycle', async () => {
    // 1. Get owner token
    const authRes = await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' });
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;

    // 2. Create location
    const startOnboarding = await fetch(`${BASE}/api/owner/onboarding/start`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
      body: JSON.stringify({
        name: `Test Loc ${uuid()}`,
        phone: '+355600000000',
        slug: `test-${uuid()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    expect(startOnboarding.status).toBe(201);
    const onboardingBody = await startOnboarding.json();
    const locationId = onboardingBody.locationId;

    // Complete onboarding
    for (let step = 1; step <= 8; step++) {
      if (step === 7) {
        const skipRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/7/skip`, {
          headers: await authHeaders(ownerToken),
          method: 'POST',
        });
        expect(skipRes.status).toBe(200);
      } else {
        const stepRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
          headers: await authHeaders(ownerToken),
          method: 'POST',
          body: JSON.stringify({ step }),
        });
        expect(stepRes.status).toBe(200);
      }
    }

    // 3. Create product
    const productRes = await fetch(`${BASE}/api/owner/locations/${locationId}/products`, {
      headers: await authHeaders(ownerToken),
      method: 'POST',
      body: JSON.stringify({
        name: 'Test Product',
        price: 1000,
        category_id: null,
        available: true,
      }),
    });
    expect(productRes.status).toBe(201);
    const productBody = await productRes.json();
    const productId = productBody.id;

    // 4. Create order - triggers order.created event
    const orderPayload = {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: '+355600000001', name: 'Test Customer' },
      delivery: { address_text: 'Test Street', pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      idempotency_key: uuid(),
    };
    const orderRes = await fetch(`${BASE}/api/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload),
    });
    const orderText = await orderRes.text();
    expect(orderRes.status).toBe(201, orderText);
    const orderBody = JSON.parse(orderText);
    const orderId = orderBody.id;
    expectUuid(orderId, 'orderId');

    // 5. Advance order through lifecycle (owner confirms → preparing → ready → in_delivery → delivered)
    const statuses = ['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'];
    for (const status of statuses) {
      const statusRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
        headers: await authHeaders(ownerToken),
        method: 'PATCH',
        body: JSON.stringify({ status }),
      });
      const statusText = await statusRes.text();
      expect(statusRes.status).toBe(200, `Failed to set status ${status}: ${statusText}`);
    }

    // Test passes if order lifecycle completes successfully
    // Each status change triggers order.status events → notification dispatch
  });
});