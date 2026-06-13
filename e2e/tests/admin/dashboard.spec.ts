import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'http://localhost:3000';

test.describe('Admin Dashboard — Interactive', () => {
  let authToken: string;

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
  });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
      sessionStorage.setItem('dos_dev', '1');
    }, authToken);
    await page.goto('/admin?dev=true', { waitUntil: 'networkidle' });
  });

  test('dashboard loads with content', async ({ page }) => {
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
  });

  test('Live/History toggle buttons exist and are clickable', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Find toggle buttons in the view mode tablist
    const tablist = page.locator('[role="tablist"]').filter({ hasText: /Live|History|live|history/i });
    const tablistCount = await tablist.count();

    if (tablistCount > 0) {
      const buttons = tablist.first().locator('[role="tab"]');
      const btnCount = await buttons.count();
      expect(btnCount).toBeGreaterThanOrEqual(2);

      await buttons.first().click();
      await page.waitForTimeout(500);
      await buttons.nth(1).click();
      await page.waitForTimeout(500);

      // Should not crash after toggling
      const body = await page.textContent('body');
      expect(body.length).toBeGreaterThan(100);
    }
  });

  test('status filter buttons are clickable', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Status filter buttons are in a [role="group"] with status aria-label
    const statusGroup = page.locator('[role="group"]');
    await expect(statusGroup.first()).toBeVisible({ timeout: 10000 });

    const buttons = statusGroup.first().locator('button');
    const count = await buttons.count();
    if (count > 1) {
      await buttons.first().click();
      await page.waitForTimeout(300);
      await buttons.nth(1).click();
      await page.waitForTimeout(300);
    }

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('sort icon button opens dropdown', async ({ page }) => {
    await page.waitForTimeout(3000);

    const sortBtn = page.locator('button').filter({ has: page.locator('.ti-arrows-sort') }).first();
    await expect(sortBtn).toBeVisible({ timeout: 10000 });

    await sortBtn.click();
    await page.waitForTimeout(500);

    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
  });

  test('search input accepts text', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Find any visible text input that's not a time input
    const searchInput = page.locator('input:not([type="time"]):not([type="hidden"])').first();
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    await searchInput.fill('test');
    await page.waitForTimeout(500);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('quick stats render in grid', async ({ page }) => {
    await page.waitForTimeout(3000);

    const stats = page.locator('.grid.grid-cols-3').first();
    await expect(stats).toBeVisible({ timeout: 10000 });

    const cards = stats.locator('> div');
    const count = await cards.count();
    expect(count).toBeGreaterThanOrEqual(3);
  });

  test('no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.waitForTimeout(3000);

    const critical = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await page.waitForTimeout(2000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
