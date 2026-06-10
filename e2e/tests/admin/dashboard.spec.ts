import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;

test.describe('Admin Dashboard', () => {
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
  });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.getByText(/dashboard|orders/i).first()).toBeAttached({ timeout: 15000 });
  });

  test('dashboard page loads without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors).toEqual([]);

    const heading = page.locator('h1, h2, [class*="dashboard"]').first();
    await expect(heading).toBeAttached({ timeout: 10000 });
  });

  test('dashboard renders content', async ({ page }) => {
    const bodyText = await page.textContent('body');
    expect(bodyText).toBeTruthy();
    expect(bodyText!.length).toBeGreaterThan(50);
  });

  test('sidebar navigation is visible', async ({ page }) => {
    const navElements = page.locator('button, a, nav a').filter({ hasText: /orders|menu|branding|dashboard/i });
    const count = await navElements.count();
    expect(count).toBeGreaterThan(0);
  });

  test('no cookies are set on admin pages', async ({ page }) => {
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('theme variables are applied on admin', async ({ page }) => {
    const primary = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim()
    );
    expect(primary).toBeTruthy();
    expect(primary.startsWith('#')).toBe(true);
  });

});
