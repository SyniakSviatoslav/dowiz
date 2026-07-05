import { test, expect } from '@playwright/test';

// NOTE on findings 6 (error/500 state) & 7 (cross-tenant isolation):
// SupplyLibraryPage is a CLIENT-SIDE page — supplies live in localStorage
// (loadSupplies/saveSupplies, STORAGE_KEY), seeded with defaults on first load.
// There is NO supplies HTTP route to page.route()-intercept for a 500, and the
// empty state is unreachable (loadSupplies reseeds whenever the store is empty).
// Likewise there is no server-side tenant scoping to cross — so a real
// second-tenant isolation assertion is architecturally inapplicable here.
// TODO(needs-staging): if/when supplies move to a tenant-scoped API, add a 500
// intercept + a real second-tenant isolation test against staging.

test.describe('Supply Library — Interactive', () => {
  test('supplies page loads with content', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });

    // Render proof: real supply rows must be visible — a spinner/error/redirect
    // shell renders zero [data-testid="supply-item"] and fails this.
    const items = page.getByTestId('supply-item');
    await expect(items.first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Salmon fillet')).toBeVisible();
    expect(await items.count()).toBeGreaterThan(1);
  });

  test('admin route is gated — no ?dev=true renders no supplies (negative control)', async ({ page }) => {
    // Without the dev flag and without a real token, the AdminRoutes guard
    // returns null + redirects to /login. Supply rows must NOT render to an
    // unauthenticated visitor — this is the control that proves ?dev=true
    // actually opens a gate rather than the route being open to everyone.
    await page.goto('/admin/supplies', { waitUntil: 'networkidle' });
    await expect(page.getByTestId('supply-item')).toHaveCount(0);
    await expect(page).toHaveURL(/\/login/);
  });

  test('kind filter narrows the list to the selected kind', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    const items = page.getByTestId('supply-item');
    await expect(items.first()).toBeVisible({ timeout: 10000 });
    const initialCount = await items.count();

    // Kind filter bar = the SegmentedControl (role=group, overflow-x-auto).
    const filterBar = page.locator('.overflow-x-auto').last();
    const filterBtns = filterBar.locator('button');
    expect(await filterBtns.count()).toBeGreaterThanOrEqual(2);

    // Second segment = "Ingredients" (food_ingredient). Selecting it must shrink
    // the list and drop condiment-only rows; an ingredient row stays.
    await filterBtns.nth(1).click();
    await expect(page.getByText('Spicy mayo')).toBeHidden();
    await expect(page.getByText('Salmon fillet')).toBeVisible();
    const filteredCount = await items.count();
    expect(filteredCount).toBeLessThan(initialCount);
    expect(filteredCount).toBeGreaterThan(0);
  });

  test('search filters the list to matching items', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    const items = page.getByTestId('supply-item');
    await expect(items.first()).toBeVisible({ timeout: 10000 });

    const searchInput = page.locator('input:not([type="time"]):not([type="hidden"])').first();
    await expect(searchInput).toBeVisible({ timeout: 5000 });
    await searchInput.fill('Salmon');

    // Matching row present, unrelated rows absent — proves the query filtered.
    await expect(page.getByText('Salmon fillet')).toBeVisible();
    await expect(page.getByText('Sushi rice')).toBeHidden();
    expect(await items.count()).toBeGreaterThan(0);
  });

  test('sort icon button opens dropdown', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await expect(page.getByTestId('supply-item').first()).toBeVisible({ timeout: 10000 });

    const sortBtn = page.locator('button').filter({ has: page.locator('.ti-arrows-sort') }).first();
    await expect(sortBtn).toBeVisible({ timeout: 5000 });
    await sortBtn.click();

    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 3000 });
  });

  test('no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await expect(page.getByTestId('supply-item').first()).toBeVisible({ timeout: 10000 });

    const critical = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await page.goto('/admin/supplies?dev=true', { waitUntil: 'networkidle' });
    await expect(page.getByTestId('supply-item').first()).toBeVisible({ timeout: 10000 });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
