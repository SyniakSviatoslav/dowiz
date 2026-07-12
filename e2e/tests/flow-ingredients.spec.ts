/* eslint-disable @typescript-eslint/no-explicit-any -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;
let activeLocationId: string;
let categoryId: string;
let productId: string;
let groupId: string;
let modifierId: string;
const TS = Date.now();

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Ingredients & Modifiers — Groups, Modifiers, price_delta, product attachment', () => {
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
    expect(authToken).toBeTruthy();
    expect(activeLocationId).toMatch(/^[0-9a-f-]{36}$/);

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `E2E-Ing-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `E2E-Ing-Prod-${TS}`, price: 500, available: true, categoryId,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    if (groupId) {
      await request
        .delete(`${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}`, {
          headers: { Authorization: `Bearer ${authToken}` },
        })
        .catch(() => {});
    }
    if (productId) {
      await request
        .delete(`${BASE}/api/owner/menu/products/${productId}`, {
          headers: { Authorization: `Bearer ${authToken}` },
        })
        .catch(() => {});
    }
    if (categoryId) {
      await request
        .delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
          headers: { Authorization: `Bearer ${authToken}` },
        })
        .catch(() => {});
    }
  });

  test('Flow 1: Owner — create modifier group with min/max select, verify contract', async ({ request }) => {
    const locBase = `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`;
    const authHeaders = { Authorization: `Bearer ${authToken}` };

    const createRes = await request.post(locBase, {
      data: { name: `E2E-Group-${TS}`, min_select: 1, max_select: 2, required: true },
      headers: authHeaders,
    });
    expect(createRes.status()).toBe(201);
    const body = await createRes.json();
    expect(body.id).toMatch(/^[0-9a-f-]{36}$/);
    expect(body.name).toBe(`E2E-Group-${TS}`);
    expect(body.minSelect).toBe(1);
    expect(body.maxSelect).toBe(2);
    expect(body.required).toBe(true);
    groupId = body.id;
  });

  test('Flow 2: Owner — list modifier groups returns array with created group', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` } },
    );
    expect(listRes.status()).toBe(200);
    const body = await listRes.json();
    expect(Array.isArray(body.data)).toBe(true);
    const found = body.data.find((g: any) => g.id === groupId);
    expect(found).toBeTruthy();
    expect(found.name).toBe(`E2E-Group-${TS}`);
    expect(found.minSelect).toBe(1);
    expect(found.maxSelect).toBe(2);
    expect(found.required).toBe(true);
  });

  test('Flow 3: Owner — update modifier group name + select constraints, verify round-trip', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const patchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}`,
      {
        data: { name: `E2E-Group-Updated-${TS}`, min_select: 0, max_select: 3, required: false },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(patchRes.status()).toBe(200);
    const patched = await patchRes.json();
    expect(patched.name).toBe(`E2E-Group-Updated-${TS}`);
    expect(patched.minSelect).toBe(0);
    expect(patched.maxSelect).toBe(3);
    expect(patched.required).toBe(false);
  });

  test('Flow 4: Owner — create modifier with price_delta (integer ALL), verify contract', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const createRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}/modifiers`,
      {
        data: { name: `Extra Cheese ${TS}`, price_delta: 100, available: true, sort_order: 1 },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(createRes.status()).toBe(201);
    const body = await createRes.json();
    expect(body.id).toMatch(/^[0-9a-f-]{36}$/);
    expect(body.name).toBe(`Extra Cheese ${TS}`);
    expect(body.priceDelta).toBe(100);
    expect(body.available).toBe(true);
    modifierId = body.id;
  });

  test('Flow 5: Owner — update modifier price_delta + availability, verify round-trip', async ({ request }) => {
    test.skip(!modifierId, 'No modifier created');
    const patchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}/modifiers/${modifierId}`,
      {
        data: { price_delta: 150, available: false },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(patchRes.status()).toBe(200);
    const patched = await patchRes.json();
    expect(patched.priceDelta).toBe(150);
    expect(patched.available).toBe(false);
  });

  test('Flow 6: Owner — attach modifier group to product, verify via GET', async ({ request }) => {
    test.skip(!groupId || !productId, 'No group or product');
    const attachRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/modifier-groups`,
      {
        data: [{ group_id: groupId, sort_order: 0 }],
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(attachRes.status()).toBe(200);

    const getRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` } },
    );
    expect(getRes.status()).toBe(200);
    const groups = await getRes.json();
    const attached = (groups.data || groups).find((g: any) => g.id === groupId);
    expect(attached).toBeTruthy();
  });

  test('Flow 7: Owner — delete modifier, verify gone from group', async ({ request }) => {
    test.skip(!modifierId, 'No modifier created');
    const delRes = await request.delete(
      `${BASE}/api/owner/locations/${activeLocationId}/modifiers/${modifierId}`,
      { headers: { Authorization: `Bearer ${authToken}` } },
    );
    expect(delRes.status()).toBe(204);

    // Verify deletion via group list (no standalone modifiers list endpoint exists)
    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` } },
    );
    expect(listRes.status()).toBe(200);
    const list = await listRes.json();
    const group = list.data.find((g: any) => g.id === groupId);
    expect(group?.modifierCount).toBe(0);
  });

  test('Flow 8: Owner — create modifier with zero price_delta and sort_order, verify contract', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const createRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}/modifiers`,
      {
        data: { name: `Free option ${TS}`, price_delta: 0, available: true, sort_order: 0 },
        headers: { Authorization: `Bearer ${authToken}` },
      },
    );
    expect(createRes.status()).toBe(201);
    const body = await createRes.json();
    expect(body.priceDelta).toBe(0);
    expect(body.sortOrder).toBe(0);
  });

  test('Flow 4: Unauthenticated — modifier group CRUD returns 401', async ({ request }) => {
    const routes = [
      {
        method: 'POST',
        url: `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
        data: { name: 'should-401' },
      },
      {
        method: 'GET',
        url: `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      },
    ];
    for (const r of routes) {
      let res;
      if (r.method === 'POST') {
        res = await request.post(r.url, { data: r.data });
      } else {
        res = await request.get(r.url);
      }
      expect(res.status(), `${r.method} ${r.url} should return 401`).toBe(401);
    }
  });
});
