import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Default to STAGING, never the prod host: this spec MUTATES state (creates a real
// location + product, places an order) and uses the /api/dev/mock-auth backdoor.
// requireStaging() FAILS FAST if pointed at prod, so a regressed backdoor can never
// be exercised against the live system from here (Test Integrity #6).
const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
>>>>>>> Stashed changes

test.describe('Quick Order Test', () => {
  test.beforeAll(() => {
    requireStaging(BASE_URL);
  });

  test('should attempt to create an order and report error', async () => {
    test.setTimeout(15000); // 15 second timeout

    console.log('Starting quick order test');
    console.log(`Using BASE_URL: ${BASE_URL}`);

    // Get owner token
    console.log('Getting owner token...');
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, {
      method: 'POST',
      headers: { 'x-dev-auth-secret': process.env.DEV_AUTH_SECRET ?? '' },
    });
    console.log(`Auth response status: ${authRes.status}`);
    expect(authRes.status, 'mock-auth must succeed on staging').toBe(200);
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;
    expectJwt(ownerToken, 'owner access_token');
    console.log('Got owner token successfully');

    // Create a minimal location
    console.log('Creating location...');
    const startOnboarding = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: 'Quick Test Loc',
        phone: '+355600000000',
        slug: `quick-test-${Date.now()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    console.log(`Location creation response status: ${startOnboarding.status}`);
    expect(startOnboarding.status, 'onboarding/start must succeed').toBe(200);
    const onboardingBody = await startOnboarding.json();
    const locationId = onboardingBody.locationId;
    // Fail loudly if the API renamed the key — otherwise downstream URLs become
    // /api/owner/locations/undefined/products and silently 404 (finding #7).
    expectUuid(locationId, 'onboardingBody.locationId');
    console.log(`Created location: ${locationId}`);

    // Create a product
    console.log('Creating product...');
    const productRes = await fetch(`${BASE_URL}/api/owner/locations/${locationId}/products`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: `Quick Test Product`,
        price: 1000,
        category_id: null,
        available: true,
      }),
    });
    console.log(`Product creation response status: ${productRes.status}`);
    expect(productRes.status, 'product create must succeed').toBe(201);
    const productBody = await productRes.json();
    const productId = productBody.id;
    expectUuid(productId, 'productBody.id');
    console.log(`Created product: ${productId}`);

    // Place an order against our own published storefront (positive path).
    console.log('Placing order...');
    const orderPayload = {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: '+355600000001', name: 'Test Customer' },
      delivery: { address_text: 'Test Street', pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
    };

    console.log(`Order payload: ${JSON.stringify(orderPayload)}`);
<<<<<<< Updated upstream
    
    try {
      const orderRes = await fetch(`${BASE_URL}/api/orders`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(orderPayload),
      });
      console.log(`Order response status: ${orderRes.status}`);
      console.log(`Order response text: ${await orderRes.text()}`);
      
      if (!orderRes.ok) {
        throw new Error(`Failed to place order: ${await orderRes.text()}`);
      }
      
      const orderBody = await orderRes.json();
      console.log(`Order created successfully:`, orderBody);
      expect(orderBody.id).toBeTruthy();
    } catch (error) {
      console.error(`Error placing order:`, error);
      throw error;
    }
=======

    const orderRes = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload),
    });
    // Read the body exactly once — a second .text()/.json() returns '' and would
    // swallow the real error body (finding #2).
    const orderText = await orderRes.text();
    console.log(`Order response status: ${orderRes.status}`);
    console.log(`Order response body: ${orderText}`);

    expect(orderRes.status, `order create failed: ${orderText}`).toBe(201);
    const orderBody = JSON.parse(orderText);
    // A real order: UUID id, scoped to OUR location, with a non-empty status string
    // (truthy-on-id would pass for '', 0 or 'null' — finding #3).
    expectUuid(orderBody.id, 'orderBody.id');
    expect(orderBody.locationId, 'order must be scoped to our location').toBe(locationId);
    expect(typeof orderBody.status).toBe('string');
    expect(orderBody.status.length, 'order must have an initial status').toBeGreaterThan(0);
>>>>>>> Stashed changes
  });

  // NEGATIVE CONTROL — protected owner route must reject a missing token with EXACT 401
  // (server.ts:399-400). Proves the auth gate isn't silently open (Test Integrity #4).
  test('rejects unauthenticated onboarding with 401', async () => {
    const res = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'x', phone: '+355600000000', slug: `unauth-${Date.now()}` }),
    });
    expect(res.status, 'no-token must be 401').toBe(401);
  });

  // ERROR MATRIX — a delivery order with no delivery pin fails Zod superRefine and the
  // route returns EXACT 400 VALIDATION_FAILED (orders.ts:87-89). Validation runs before
  // any DB lookup, so this mutates nothing (finding #5).
  test('rejects malformed order (missing delivery pin) with 400', async () => {
    const res = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: crypto.randomUUID(),
        type: 'delivery',
        items: [{ product_id: crypto.randomUUID(), quantity: 1 }],
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
        // delivery omitted on purpose
      }),
    });
    const body = await res.text();
    expect(res.status, `expected 400 VALIDATION_FAILED, got: ${body}`).toBe(400);
    expect(body).toContain('VALIDATION_FAILED');
  });

  // TODO(needs-staging): cross-tenant isolation — read a SECOND real tenant's order id
  // via GET /api/orders/:id with our customer token and assert 403/404. Requires a real
  // second tenant + a placed order on it (a nil/all-zero UUID 404s by absence and proves
  // nothing — Test Integrity #5), and a 429 rate-limit probe (hammer onboarding/start past
  // its 3/min cap). Both need a live staging run with seeded fixtures.
  // TODO(needs-staging): teardown — no owner DELETE-location route exists; the created
  // location+product persist. Add cleanup via a staging dev endpoint or DB access.
});
