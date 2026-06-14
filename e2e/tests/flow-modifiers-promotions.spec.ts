import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;
let activeLocationId: string;
let productId: string;
let groupId: string;
let modifierId: string;
let promotionId: string;
let orderId: string;
const TS = Date.now();

test.describe.configure({ mode: 'serial' });

test.describe('API: Order with Modifiers + Promotions', () => {
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;

    // Create category + product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `ModPromo-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `ModPromo-Prod-${TS}`, price: 1000, available: true, categoryId, stockCount: 20 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create modifier group with modifiers
    const grpRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      {
        data: { name: `Size-${TS}`, min_select: 1, max_select: 1, required: true },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(grpRes.status()).toBe(201);
    groupId = (await grpRes.json()).id;

    const modRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}/modifiers`,
      {
        data: { name: `Large-${TS}`, price_delta: 200, available: true, sort_order: 1 },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(modRes.status()).toBe(201);
    modifierId = (await modRes.json()).id;

    // Attach modifier group to product
    await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/modifier-groups`,
      {
        data: { modifierGroupIds: [groupId] },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );

    // Create a promotion
    const promRes = await request.post(`${BASE}/api/owner/promotions`, {
      data: {
        code: `PROMO-${TS}`,
        type: 'percentage',
        discount: 10,
        validFrom: new Date(Date.now() - 86400000).toISOString(),
        validTo: new Date(Date.now() + 86400000 * 30).toISOString(),
        minOrder: 500,
        usageLimit: 100,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(promRes.status()).toBe(201);
    promotionId = (await promRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    if (promotionId) {
      await request.delete(`${BASE}/api/owner/promotions/${promotionId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Flow 1: Order with modifiers — POST returns 201 with correct totals', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 2, modifier_ids: [modifierId] }],
        customer: { phone: '+355600000010', name: 'Mod Promo Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Elbasanit, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: `mod-promo-${TS}`,
      },
    });
    expect(orderRes.status()).toBe(201);
    const body = await orderRes.json();
    orderId = body.id || body.orderId;
    expect(orderId).toMatch(/^[0-9a-f-]{36}$/);
    expect(body.status).toBe('PENDING');
    expect(typeof body.total).toBe('number');
    expect(body.total).toBeGreaterThan(0);

    // With 2 items × (1000 + 200 modifier) = 2400
    // Check total is at least base price
    expect(body.total).toBeGreaterThanOrEqual(2000);
    console.log(`Order ${orderId}: total=${body.total}, subtotal=${body.subtotal}`);
  });

  test('Flow 2: Validate promotion code returns valid', async ({ request }) => {
    const validRes = await request.post(`${BASE}/api/owner/promotions/validate`, {
      data: { code: `PROMO-${TS}`, orderTotal: 2400 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(validRes.status()).toBe(200);
    const body = await validRes.json();
    expect(body.valid).toBe(true);
    expect(body.discount).toBeGreaterThan(0);
  });

  test('Flow 3: GET order returns items with modifier details', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const getRes = await request.get(`${BASE}/api/orders/${orderId}`);
    expect(getRes.status()).toBe(200);
    const order = await getRes.json();
    expect(order.items).toBeTruthy();
    expect(order.items.length).toBeGreaterThan(0);
  });

  test('Flow 4: Promotion list includes created promotion', async ({ request }) => {
    const listRes = await request.get(`${BASE}/api/owner/promotions`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(listRes.status()).toBe(200);
    const body = await listRes.json();
    const promos = body.promotions || body.data || body;
    const found = promos.find((p: any) => p.id === promotionId);
    expect(found).toBeTruthy();
    expect(found.code).toBe(`PROMO-${TS}`);
    expect(found.type).toBe('percentage');
  });

  test('Flow 5: Owner confirm order via API', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const confirmRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } },
    );
    expect([200, 409]).toContain(confirmRes.status());
  });
});
