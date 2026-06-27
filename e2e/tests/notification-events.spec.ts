import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

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
  // Mutating spec (creates locations/orders, drives the lifecycle): refuse to run against
  // prod or an unknown target so a CI misconfig can never write to production.
  test.beforeAll(() => requireStaging(BASE));

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
    expectUuid(locationId, 'locationId');

    // Complete onboarding — assert the flow actually reaches a completed state on the
    // final step (a chain of bare 200s does not prove the location went live).
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
        const stepText = await stepRes.text();
        expect(stepRes.status).toBe(200, `onboarding step ${step}: ${stepText}`);
        if (step === 8) {
          expect(JSON.parse(stepText).completed, 'onboarding must mark completed on final step').toBe(true);
        }
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
    expectUuid(productId, 'productId');

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

    // 4b. Error matrix for PATCH /orders/:id/status — exercised BEFORE the happy path so
    // these rejected calls cannot mutate the order. Exact codes read from
    // apps/api/src/routes/orders.ts + lib/orderStatusService.ts.
    // (i) no token → 401 (verifyAuth preHandler, plugins/auth.ts:47).
    const noTokenRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'CONFIRMED' }),
    });
    expect(noTokenRes.status).toBe(401);
    // (ii) invalid status enum → 400 VALIDATION_FAILED (safeParse, orders.ts:757).
    const badEnumRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      headers: await authHeaders(ownerToken),
      method: 'PATCH',
      body: JSON.stringify({ status: 'INVALID' }),
    });
    expect(badEnumRes.status).toBe(400);
    // (iii) illegal transition (PENDING → DELIVERED) → 400 IllegalTransitionError
    // (assertTransition, orderStatusService.ts:80). Order is still PENDING here.
    const illegalRes = await fetch(`${BASE}/api/orders/${orderId}/status`, {
      headers: await authHeaders(ownerToken),
      method: 'PATCH',
      body: JSON.stringify({ status: 'DELIVERED' }),
    });
    expect(illegalRes.status).toBe(400);
    // (iv) nonexistent order id (valid UUID, absent / out-of-tenant) → 404
    // (RLS-scoped SELECT returns 0 rows, orders.ts:774).
    const missingRes = await fetch(`${BASE}/api/orders/${uuid()}/status`, {
      headers: await authHeaders(ownerToken),
      method: 'PATCH',
      body: JSON.stringify({ status: 'CONFIRMED' }),
    });
    expect(missingRes.status).toBe(404);

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

    // 6. Read the order back and assert the PATCH chain actually persisted the terminal
    // state (Test Integrity #9: verify a PATCH by reading the value, not just status 200).
    const readBack = await fetch(`${BASE}/api/orders/${orderId}`, {
      headers: await authHeaders(ownerToken),
    });
    const readText = await readBack.text();
    expect(readBack.status).toBe(200, readText);
    expect(String(JSON.parse(readText).status).toUpperCase(), 'order must persist DELIVERED').toBe('DELIVERED');

    // NOTE: this proves the lifecycle + event-emitting transitions, but NOT that a
    // notification was actually delivered to a channel. Asserting real dispatch requires
    // a live Telegram link (helpers/notifHelpers.linkTelegram + waitTelegramMessage) with
    // TELEGRAM_BOT_TOKEN/CHAT_ID set against staging.
    // TODO(needs-staging): add a Telegram/push dispatch assertion (see needs_staging).
    // TODO(needs-staging): cross-tenant IDOR — owner B advancing owner A's order must 404.
    // mock-auth always mints the SAME owner, so a REAL second tenant is required (never a nil UUID).
  });
});