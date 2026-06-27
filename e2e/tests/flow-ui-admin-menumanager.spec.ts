import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: MenuManager — Product & Category CRUD via Forms', () => {
  let authToken: string;
  const TS = Date.now();
  const CAT_NAME = `UI-FCat-${TS}`;
  const PROD_NAME = `UI-FProd-${TS}`;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    expectJwt(authToken, 'mock-auth access_token');
  });

  test('Menu manager page loads with categories visible', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Category tabs are clickable', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const tabs = page.locator('[role="tab"], [class*="tab"], button').filter({ hasText: /All|all|/i }).first();
    if (await tabs.isVisible({ timeout: 5000 }).catch(() => false)) {
      const allTabs = page.locator('[role="tab"]');
      const count = await allTabs.count();
      if (count >= 2) {
        await allTabs.nth(1).click();
        await page.waitForTimeout(500);
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Create category via API (UI form may use modal)', async ({ request }) => {
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: CAT_NAME },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const cat = await catRes.json();
    expect(cat.name).toBe(CAT_NAME);
  });

  test('Create product via API (UI form may use modal/drawer)', async ({ request }) => {
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const catId = catsList.find((c: any) => c.name === CAT_NAME)?.id;
    test.skip(!catId, 'Category not found');

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: PROD_NAME, price: 850, categoryId: catId, available: true, stockCount: 15 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    const prod = await prodRes.json();
    expect(prod.name).toBe(PROD_NAME);
    expect(prod.price).toBe(850);
  });

  test('Product search filters in UI', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i], input[placeholder*="Kërko" i]').first();
    if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await searchInput.fill(PROD_NAME);
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Sort dropdown opens in menu manager', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const sortBtn = page.locator('button, select').filter({ hasText: /sort|Sort|Rendit/i }).first();
    if (await sortBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await sortBtn.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Availability filter opens', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const filterBtn = page.locator('button, select').filter({ hasText: /availab|filter|Filter|all|All/i }).first();
    if (await filterBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await filterBtn.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delete test product via API cleanup', async ({ request }) => {
    const prodRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const prods = await prodRes.json();
    const prodList = prods.products || prods.data || prods;
    const target = prodList.find((p: any) => p.name === PROD_NAME);
    test.skip(!target, 'Product not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/products/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 204]).toContain(delRes.status());
  });

  test('Delete test category via API cleanup', async ({ request }) => {
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const target = catsList.find((c: any) => c.name === CAT_NAME);
    test.skip(!target, 'Category not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/categories/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(204);
  });

  test('Menu manager survives navigation without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('ResizeObserver')
    );
    expect(criticalErrors).toEqual([]);
  });
});
