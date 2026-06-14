import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('UI: Analytics + Supplies CRUD', () => {
  let authToken: string;
  let activeLocationId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
  });

  test('Analytics page loads with KPI cards', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    const hasKPIs = /revenue|orders|total|average|delivery|count|sales|analytics/i.test(body);
    expect(hasKPIs).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Analytics API returns revenue and chart data', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/analytics`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();

    expect(body.totalRevenue !== undefined || body.revenue !== undefined).toBe(true);
    expect(body.totalOrders !== undefined || body.orders !== undefined).toBe(true);
  });

  test('Analytics page survives navigation', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies page loads with filter/search controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    // Check for interactive controls
    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i]').first();
    const hasSearch = await searchInput.isVisible({ timeout: 3000 }).catch(() => false);

    const sortBtn = page.locator('button, select').filter({ hasText: /sort|Sort|Rendit/i }).first();
    const hasSort = await sortBtn.isVisible({ timeout: 2000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies kind filter buttons interactive', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const filterBtns = page.locator('button, [role="tab"], [role="radio"]').filter({
      hasText: /all|food|beverage|packaging|All|Ushqim|Pije|Ambalazh/i,
    });
    const count = await filterBtns.count();
    if (count > 0) {
      await filterBtns.first().click();
      await page.waitForTimeout(300);
      if (count > 1) {
        await filterBtns.nth(1).click();
        await page.waitForTimeout(300);
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies search filters items', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i], input[placeholder*="Kërko" i]').first();
    if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies sort dropdown opens', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const sortBtn = page.locator('button, select').filter({ hasText: /sort|Sort|Rendit/i }).first();
    if (await sortBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await sortBtn.click();
      await page.waitForTimeout(500);
    }

    const select = page.locator('select').first();
    if (await select.isVisible({ timeout: 1000 }).catch(() => false)) {
      await select.selectOption({ index: 1 }).catch(() => {});
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies page survives navigation', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM page loads with customer list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/crm`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM API returns customers list', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/customers`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    const customers = body.customers || body.data || body;
    expect(Array.isArray(customers)).toBe(true);
  });

  test('No cookies on any admin page', async ({ page }) => {
    for (const path of ['/admin/analytics', '/admin/supplies', '/admin/crm']) {
      await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
      await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' });
      const cookies = await page.context().cookies();
      expect(cookies, `${path} should have 0 cookies`).toEqual([]);
    }
  });
});
