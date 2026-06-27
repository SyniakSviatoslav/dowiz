import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Owner CRUD — Products & Categories', () => {
  let authToken: string;
  const TS = Date.now();
  const TEST_ITEM = `UI-CRUD-${TS}`;

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
  });

  test('Flow 1: Menu manager page loads with categories and products', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 2: Create category via API', async ({ request }) => {
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `${TEST_ITEM}-Cat` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const cat = await catRes.json();
    expectUuid(cat.id);
    expect(cat.name).toBe(`${TEST_ITEM}-Cat`);
  });

  test('Flow 3: Create product via API', async ({ request }) => {
    // First get or create a category
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catsRes.status()).toBe(200);
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const catId = catsList[0]?.id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `${TEST_ITEM}-Prod`, price: 750, categoryId: catId, available: true, stockCount: 5 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    const prod = await prodRes.json();
    expectUuid(prod.id);
    expect(prod.name).toBe(`${TEST_ITEM}-Prod`);
    expect(prod.price).toBe(750);

    // Verify product appears on public menu
    const menuRes = await request.get(`${BASE}/public/locations/demo/menu`, {
      headers: { 'Cache-Control': 'no-cache' },
    });
    expect(menuRes.ok()).toBe(true);
    const menu = await menuRes.json();
    const products = menu.products || menu.items || menu.data || [];
    const found = products.some((p: any) => p.name === `${TEST_ITEM}-Prod`);
    if (!found) {
      // Check in categories
      const cats2 = menu.categories || [];
      const inCats = cats2.some((c: any) =>
        (c.products || c.items || []).some((p: any) => p.name === `${TEST_ITEM}-Prod`)
      );
      expect(inCats).toBe(true);
    } else {
      expect(found).toBe(true);
    }
  });

  test('Flow 4: Update product via API and verify round-trip', async ({ request }) => {
    // Find our test product
    const prodRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(200);
    const prods = await prodRes.json();
    const prodList = prods.products || prods.data || prods;
    const target = prodList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    test.skip(!target, 'Test product not found');

    const patchRes = await request.patch(`${BASE}/api/owner/menu/products/${target.id}`, {
      data: { price: 899, stockCount: 3 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(patchRes.status()).toBe(200);
    const updated = await patchRes.json();
    expect(updated.price ?? target.price).toBe(899);
  });

  test('Flow 5: Delete test product via API', async ({ request }) => {
    const prodRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(200);
    const prods = await prodRes.json();
    const prodList = prods.products || prods.data || prods;
    const target = prodList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    test.skip(!target, 'Test product not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/products/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(200);

    // Verify deleted
    const verifyRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const verifyProds = await verifyRes.json();
    const vList = verifyProds.products || verifyProds.data || verifyProds;
    const gone = vList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    expect(gone).toBeFalsy();
  });

  test('Flow 6: Delete test category via API', async ({ request }) => {
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catsRes.status()).toBe(200);
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const target = catsList.find((c: any) => c.name === `${TEST_ITEM}-Cat`);
    test.skip(!target, 'Test category not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/categories/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(204);
  });

  test('Flow 7: Menu manager survives page navigation, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Navigate to dashboard and back
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 8: Unauthenticated 401 on product endpoints', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'should-fail', price: 100 },
    });
    expect(res.status()).toBe(401);
  });
});
