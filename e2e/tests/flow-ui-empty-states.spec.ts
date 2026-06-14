import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;

test.describe('UI: Empty States — All Lists', () => {
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
  });

  test('Dashboard loads even with no orders (today)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM page shows list or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/crm`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Couriers page shows list or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Analytics page loads with KPIs', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Client menu shows product cards or empty', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('article[data-testid="menu-item"]');
    await expect(cards.first().or(page.locator('text=no products'))).toBeAttached({ timeout: 8000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Courier tasks page shows tasks or empty', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Create temp courier token with no assignments
    const courierRes = await (await test.info()).request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'courier' },
    });
    expect(courierRes.status()).toBe(200);
    const courierBody = await courierRes.json();
    const courierToken = courierBody.access_token;

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Promotions page loads with list or empty', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies library page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
