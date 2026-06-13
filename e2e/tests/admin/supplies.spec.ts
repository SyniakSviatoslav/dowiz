import { test, expect } from '@playwright/test';

test.describe('Supply Library — Interactive', () => {
  test('supplies page loads with content', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('kind filter buttons exist and are clickable', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Find the kind filter bar (overflow-x-auto with buttons)
    const filterBar = page.locator('.overflow-x-auto').last();
    const filterBtns = filterBar.locator('button');
    const count = await filterBtns.count();
    expect(count).toBeGreaterThanOrEqual(1);

    if (count > 1) {
      await filterBtns.nth(1).click();
      await page.waitForTimeout(300);
    }

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('search accepts text', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const searchInput = page.locator('input:not([type="time"]):not([type="hidden"])').first();
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    await searchInput.fill('Salmon');
    await page.waitForTimeout(500);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
  });

  test('sort icon button opens dropdown', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const sortBtn = page.locator('button').filter({ has: page.locator('.ti-arrows-sort') }).first();
    await expect(sortBtn).toBeVisible({ timeout: 5000 });
    await sortBtn.click();
    await page.waitForTimeout(500);

    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 3000 });
  });

  test('no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const critical = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
