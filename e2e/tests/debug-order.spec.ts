import { test, expect } from '@playwright/test';
import { placeOrder } from '../helpers/notifHelpers';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

test.describe('Debug Order Creation', () => {
  test('should create an order successfully', async () => {
    console.log('Starting debug order test');
    // First get a locationId by creating a minimal location
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
    // Mutating spec (creates location + product + order, hits dev/mock-auth) — fail fast
    // rather than write to prod or an unknown target.
    requireStaging(BASE_URL);
    console.log(`Using BASE_URL: ${BASE_URL}`);
    
    // Get owner token
    console.log('Getting owner token...');
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    console.log(`Auth response status: ${authRes.status}`);
    if (!authRes.ok) throw new Error(`Failed to get owner token: ${await authRes.text()}`);
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;
    // mock-auth must mint a real 3-segment JWT — '' / 'null' / an error string would pass a truthy check.
    expectJwt(ownerToken, 'ownerToken');
    console.log('Got owner token successfully');
    
    // Create a minimal location
    console.log('Creating location...');
    const startOnboarding = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: 'Debug Loc',
        phone: '+355600000000',
        slug: `debug-loc-${Date.now()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    console.log(`Location creation response status: ${startOnboarding.status}`);
    if (!startOnboarding.ok) throw new Error(`Failed to create location: ${await startOnboarding.text()}`);
    const onboardingBody = await startOnboarding.json();
    // Assert the contract shape so a renamed/missing field fails loudly here, not as an
    // opaque downstream network error.
    expect(onboardingBody, 'onboarding response must carry a locationId').toHaveProperty('locationId');
    const locationId = onboardingBody.locationId;
    expectUuid(locationId, 'locationId');

    console.log(`Created location: ${locationId}`);
    
    // Try to place an order
    console.log('Placing order...');
    try {
      // ESCALATE(#5): placeOrder() mints its OWN owner token (notifHelpers.ts:143), which may be a
      // different user than `ownerToken` above — a silent pass if product-create skips owner-scoping.
      // The fix (thread `ownerToken` through placeOrder) lives in notifHelpers.ts, a red-line file
      // (insecure-random `uuid()`) the gate forbids editing here; raised separately.
      const order = await placeOrder(locationId);
      console.log(`Order created successfully:`, order);
      expectUuid(order.id, 'order.id');
      // A non-empty string (e.g. an error-envelope id) is not enough — assert the real status.
      expect(order.status, 'a freshly created order must be PENDING').toBe('PENDING');
    } catch (error) {
      console.error(`Failed to create order:`, error);
      throw error;
    }

    // Negative control (error-matrix): an invalid payload must be rejected with an EXACT 400
    // VALIDATION_FAILED (apps/api/src/routes/orders.ts:89), never silently accepted. POST /orders
    // is a public endpoint (no auth preHandler, orders.ts:66), so no token is involved here.
    const badOrder = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ locationId }), // missing items/customer/delivery
    });
    expect(badOrder.status, 'invalid order payload must be 400 VALIDATION_FAILED').toBe(400);

    // TODO(needs-staging): cross-tenant IDOR — mint a SECOND real owner+location and assert
    // owner-A's token gets 403 POSTing a product to owner-B's locationId. Requires a real 2nd
    // tenant; must NOT be faked with a nil-UUID (it would 404 by absence, proving nothing). (#3)
    // NOTE(#2): POST /api/orders is intentionally public (no auth preHandler) — an
    // unauthenticated-401 control would assert behaviour the product does not have; escalated.
  });
});