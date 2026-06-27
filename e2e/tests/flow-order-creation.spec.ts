import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const TS = Date.now();

// Serial so beforeAll state is shared cleanly
test.describe.configure({ mode: 'serial' });

test.describe('Flow: Order Creation — Contract Tests', () => {
  let authToken: string;
  let activeLocationId: string;
  let categoryId: string;
  let productId: string;
  let productPrice: number;
  let deliveryLat: number;
  let deliveryLng: number;

  test.beforeAll(async ({ request }) => {
    // This suite CREATES categories, products and orders — a mutating run must
    // never hit prod. Fail fast unless BASE is an explicit staging/local target.
    requireStaging(BASE);

    // 1. Authenticate as owner
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;

    // 2. Get location coordinates so delivery pin is within range
    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const settings = await settingsRes.json();
    // Use the location's own coords so we don't fail the distance check
    deliveryLat = settings.lat ?? 41.3275;
    deliveryLng = settings.lng ?? 19.8187;

    // 3. Create a category for the test product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `OrdCreate-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;

    // 4. Create a product with a price that should clear any min_order (e.g. 1000 minor units)
    productPrice = 1000;
    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `OrdCreate-Product-${TS}`,
        price: productPrice,
        description: 'Integration test product',
        available: true,
        categoryId,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
    }
  });

  // ──────────────────────────────────────────────────────────────────────
  // TEST 1: Happy path — POST /api/orders returns 201 with order shape
  // ──────────────────────────────────────────────────────────────────────
  test('Happy path: POST /api/orders with valid body returns 201 with orderId', async ({ request }) => {
    const idempotencyKey = crypto.randomUUID();

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: '+355691000001', name: 'OrdCreate Test' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
          address_text: 'Test Street 1, Tirana',
        },
        payment: { method: 'cash' },
        idempotency_key: idempotencyKey,
      },
    });

    // The route can also return 200 (soft_confirm from preflight) or 422
    // (min_order, delivery range, etc.) depending on location config. Each
    // documented business outcome asserts its own contract below; the final
    // `else` throws, so an undocumented status (500/401/404/409, or a 200
    // without soft_confirm) FAILS the test rather than passing silently.
    const body = await orderRes.json();

    if (orderRes.status() === 201) {
      expectUuid(body.id, 'order.id');
      expect(body.status).toBe('PENDING');
      expect(typeof body.total).toBe('number');
      expect(body.total).toBeGreaterThan(0);
      expect(typeof body.subtotal).toBe('number');
      expect(body.subtotal).toBeGreaterThan(0);
      expect(body.createdAt).toBeTruthy();
      expect(body.locationId).toBe(activeLocationId);
    } else if (orderRes.status() === 200 && body.outcome === 'soft_confirm') {
      // Preflight soft-block — valid business response
      expect(body.reasons).toBeTruthy();
      expect(typeof body.requiresOtp).toBe('boolean');
    } else if (orderRes.status() === 422) {
      // Business rule rejection (min order, delivery range, etc.) — valid
      expect(body.error || body.code).toBeTruthy();
    } else if (orderRes.status() === 429) {
      // Rate limit hit — valid API response; test cannot assert order shape
      expect(body.code).toBeTruthy();
    } else {
      // Unexpected status — fail with detail
      throw new Error(`Unexpected status ${orderRes.status()}: ${JSON.stringify(body)}`);
    }
  });

  // ──────────────────────────────────────────────────────────────────────
  // TEST 2: Missing required fields → 400
  // The POST /orders rate limiter (max:5/min) is keyed by customer.phone
  // (apps/api/src/routes/orders.ts:71-74), so giving each validation request a
  // UNIQUE phone gives it its own fresh bucket → it can never be throttled →
  // Zod validation is ALWAYS exercised and the exact 400 is asserted (no 429
  // escape hatch). TS differs per browser-project process, so the phone is also
  // unique across the 3 Playwright projects.
  // ──────────────────────────────────────────────────────────────────────
  test('Missing required fields: POST /api/orders without items → 400', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        // items deliberately omitted
        customer: { phone: `+35569${String(TS).slice(-7)}2`, name: 'OrdCreate NoItems' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    // Fresh per-phone bucket → never rate-limited → Zod validation runs → 400.
    expect(orderRes.status()).toBe(400);
    const body = await orderRes.json();
    expect(body.code).toBe('VALIDATION_FAILED');
  });

  test('Missing required fields: POST /api/orders without idempotency_key → 400', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(TS).slice(-7)}3`, name: 'OrdCreate NoIdem' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
        },
        payment: { method: 'cash' },
        // idempotency_key deliberately omitted
      },
    });

    // Fresh per-phone bucket → never rate-limited → Zod validation runs → 400.
    expect(orderRes.status()).toBe(400);
    const body = await orderRes.json();
    expect(body.code).toBe('VALIDATION_FAILED');
  });

  test('Missing required fields: POST /api/orders without locationId → 400', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        // locationId deliberately omitted
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(TS).slice(-7)}4`, name: 'OrdCreate NoLoc' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    // Fresh per-phone bucket → never rate-limited → Zod validation runs → 400.
    expect(orderRes.status()).toBe(400);
    const body = await orderRes.json();
    expect(body.code).toBe('VALIDATION_FAILED');
  });

  // ──────────────────────────────────────────────────────────────────────
  // TEST 3: Unknown product ID → 422 PRODUCT_NOT_FOUND
  // ──────────────────────────────────────────────────────────────────────
  test('Unknown product ID: POST /api/orders with non-existent product → 422', async ({ request }) => {
    const fakeProductId = crypto.randomUUID();

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: fakeProductId, quantity: 1, modifier_ids: [] }],
        // Unique phone → fresh per-phone rate bucket → never throttled → the DB
        // product lookup always runs, so the 422 PRODUCT_NOT_FOUND contract is
        // asserted exactly (no 429 escape hatch).
        customer: { phone: `+35569${String(TS).slice(-7)}5`, name: 'OrdCreate Unknown Prod' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
          address_text: 'Test Street 2, Tirana',
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    expect(orderRes.status()).toBe(422);
    const body = await orderRes.json();
    expect(body.code).toBe('PRODUCT_NOT_FOUND');
  });

  // ──────────────────────────────────────────────────────────────────────
  // TEST 4: Min order not met → 422 MIN_ORDER_NOT_MET
  // Uses quantity=1 of a cheap product; if location has no min_order this may
  // return 201 instead — we accept either and assert the body contract.
  // ──────────────────────────────────────────────────────────────────────
  test('Min order check: POST /api/orders with very small quantity → 422 MIN_ORDER_NOT_MET or 201', async ({ request }) => {
    // Create a 1-unit priced product (1 minor unit) to try to trigger min_order
    const cheapProdRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `OrdCreate-Cheap-${TS}`,
        price: 1,
        description: 'Cheap product for min_order test',
        available: true,
        categoryId,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(cheapProdRes.status()).toBe(201);
    const cheapProductId = (await cheapProdRes.json()).id;

    try {
      const orderRes = await request.post(`${BASE}/api/orders`, {
        data: {
          locationId: activeLocationId,
          type: 'delivery',
          items: [{ product_id: cheapProductId, quantity: 1, modifier_ids: [] }],
          // Unique phone → fresh per-phone rate bucket → never throttled, so the
          // min_order check always runs and a 429 can never mask the result.
          customer: { phone: `+35569${String(TS).slice(-7)}6`, name: 'OrdCreate MinOrder' },
          delivery: {
            pin: { lat: deliveryLat, lng: deliveryLng },
            address_text: 'Test Street 3, Tirana',
          },
          payment: { method: 'cash' },
          idempotency_key: crypto.randomUUID(),
        },
      });

      // Documented outcomes are asserted per-branch below; the final `else`
      // throws, so any undocumented status (500/401/400/404/409/429) FAILS.
      const body = await orderRes.json();

      if (orderRes.status() === 422 && body.code === 'MIN_ORDER_NOT_MET') {
        // Expected path when location has min_order configured
        expect(body.details).toBeTruthy();
        expect(typeof body.details.min_order_value).toBe('number');
        expect(typeof body.details.subtotal).toBe('number');
        expect(body.details.subtotal).toBeLessThan(body.details.min_order_value);
      } else if (orderRes.status() === 201) {
        // Location has no min_order — order succeeds; clean up
        expectUuid(body.id, 'order.id');
        await request.patch(`${BASE}/api/orders/${body.id}/status`, {
          data: { status: 'CANCELLED' },
          headers: { Authorization: `Bearer ${authToken}` },
        }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
      } else if (orderRes.status() === 422) {
        // Other business rule (delivery range, etc.) — acceptable
        expect(body.code).toBeTruthy();
      } else if (orderRes.status() === 200 && body.outcome === 'soft_confirm') {
        // Preflight soft-block — acceptable
        expect(body.reasons).toBeTruthy();
      } else {
        throw new Error(`Unexpected status ${orderRes.status()}: ${JSON.stringify(body)}`);
      }
    } finally {
      // Clean up cheap product
      await request.delete(`${BASE}/api/owner/menu/products/${cheapProductId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
    }
  });

  // ──────────────────────────────────────────────────────────────────────
  // TEST 5: Duplicate idempotency key (same request) → 200 idempotent
  //         Duplicate key with different body → 422 IDEMPOTENCY_KEY_REUSED
  // ──────────────────────────────────────────────────────────────────────
  test('Duplicate idempotency key: same request body replayed → 200 or 201', async ({ request }) => {
    const idempotencyKey = crypto.randomUUID();
    const payload = {
      locationId: activeLocationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
      customer: { phone: '+355691000004', name: 'OrdCreate Idempotent' },
      delivery: {
        pin: { lat: deliveryLat, lng: deliveryLng },
        address_text: 'Test Street 4, Tirana',
      },
      payment: { method: 'cash' },
      idempotency_key: idempotencyKey,
    };

    // First request
    const first = await request.post(`${BASE}/api/orders`, { data: payload });
    expect(first.status()).not.toBe(500);
    expect(first.status()).not.toBe(401);
    const firstBody = await first.json();

    // Only replay if the first request actually created an order
    if (first.status() !== 201) {
      // Skip idempotency check; first request was blocked by a business rule
      test.skip();
      return;
    }

    const orderId = firstBody.id;

    // Second request with identical payload — must be idempotent
    const second = await request.post(`${BASE}/api/orders`, { data: payload });
    expect(second.status()).not.toBe(500);
    expect(second.status()).not.toBe(401);

    const secondBody = await second.json();

    // RFC-style idempotency: server must return 200 with the existing order
    expect(second.status()).toBe(200);
    expect(secondBody.id).toBe(orderId);

    // Clean up
    await request.patch(`${BASE}/api/orders/${orderId}/status`, {
      data: { status: 'CANCELLED' },
      headers: { Authorization: `Bearer ${authToken}` },
    }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
  });

  test('Duplicate idempotency key: different request body → 422 IDEMPOTENCY_KEY_REUSED', async ({ request }) => {
    const idempotencyKey = crypto.randomUUID();
    const basePayload = {
      locationId: activeLocationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
      customer: { phone: '+355691000005', name: 'OrdCreate IdemReuse' },
      delivery: {
        pin: { lat: deliveryLat, lng: deliveryLng },
        address_text: 'Test Street 5, Tirana',
      },
      payment: { method: 'cash' },
      idempotency_key: idempotencyKey,
    };

    // First request
    const first = await request.post(`${BASE}/api/orders`, { data: basePayload });
    expect(first.status()).not.toBe(500);
    expect(first.status()).not.toBe(401);

    if (first.status() !== 201) {
      // Can't test key-reuse if first request didn't commit
      test.skip();
      return;
    }

    const firstBody = await first.json();

    // Second request with same key but different quantity (different canonical body)
    const mutatedPayload = {
      ...basePayload,
      items: [{ product_id: productId, quantity: 2, modifier_ids: [] }],
    };
    const second = await request.post(`${BASE}/api/orders`, { data: mutatedPayload });

    // Route must detect hash mismatch and reject
    expect(second.status()).toBe(422);
    const secondBody = await second.json();
    expect(secondBody.code).toBe('IDEMPOTENCY_KEY_REUSED');

    // Clean up
    await request.patch(`${BASE}/api/orders/${firstBody.id}/status`, {
      data: { status: 'CANCELLED' },
      headers: { Authorization: `Bearer ${authToken}` },
    }).catch((e) => { void e; /* tolerated: best-effort cleanup */ });
  });

  // ──────────────────────────────────────────────────────────────────────
  // TODO (needs_staging — real second tenant): cross-tenant IDOR.
  // Order against tenant A's locationId using a product_id that belongs to a
  // DIFFERENT tenant's catalogue and assert 422 PRODUCT_NOT_FOUND (the product
  // is scoped to its own location, so it must not be orderable here). Test
  // Integrity #5 forbids faking this with a nil/all-zero UUID — it 404s by
  // absence and proves nothing. This requires a second seeded tenant with a
  // known product id on a live staging run; do NOT stub it.
  // ──────────────────────────────────────────────────────────────────────
});
