import { test, expect, type Page } from '@playwright/test';
import { requireStaging } from '../../helpers/staging-guard';

// Test-Integrity hardening (2026-06-27). Each product card renders exactly one
// `[role="switch"]` availability toggle (the only switches on this page — the
// kitchen-busy control is a plain button), so the switch count is a deterministic
// proxy for the rendered product count. We assert that count changes the way the
// feature claims, never `body.length > N`.
//
// needs_staging (NOT covered deterministically here — require a real staging run
// and/or a second real tenant; see StructuredOutput.needs_staging):
//  - F2 toggle PATCH persistence: mutates live data → 'toggling availability persists'
//    test below is guarded by requireStaging().
//  - F3 positive auth control: a real owner login (real creds) reaching the menu.
//  - F4 cross-tenant isolation: owner-A vs owner-B in separate contexts.
//  - F7 deterministic reorder proof: needs a seeded fixture with distinct prices.

const BASE = process.env.VITE_BASE_URL;

// Replace fixed waitForTimeout sleeps with a real load signal: the categories fetch.
async function gotoMenu(page: Page): Promise<void> {
  const categories = page.waitForResponse(
    (r) => r.url().includes('/owner/menu/categories') && r.request().method() === 'GET',
  );
  await page.goto('/admin/menu?dev=true', { waitUntil: 'domcontentloaded' });
  await categories;
}

test.describe('Menu Manager — Interactive CRUD', () => {

  test('menu page loads and renders category tabs', async ({ page }) => {
    await gotoMenu(page);

    // Category tabs should be visible (buttons in the scrollable tab bar)
    const tabBar = page.locator('.overflow-x-auto.hide-scrollbar').first();
    await expect(tabBar).toBeVisible({ timeout: 10000 });

    // Should have at least one category tab button
    const tabCount = await tabBar.locator('button').count();
    expect(tabCount).toBeGreaterThanOrEqual(1);
  });

  test('clicking category tab shows products', async ({ page }) => {
    await gotoMenu(page);

    const tabs = page.locator('.overflow-x-auto.hide-scrollbar').first().locator('button');
    await expect(tabs.first()).toBeVisible({ timeout: 10000 });
    const count = await tabs.count();
    expect(count).toBeGreaterThan(1); // 'All' + ≥1 real category

    const products = page.waitForResponse(
      (r) => r.url().includes('/owner/menu/products') && r.request().method() === 'GET',
    );
    await tabs.nth(1).click();
    await products;

    // After clicking a category, the products area should be present
    await expect(page.locator('.space-y-2').first()).toBeVisible({ timeout: 10000 });
  });

  test('search input filters the displayed product list (count shrinks)', async ({ page }) => {
    await gotoMenu(page);

    // Products auto-load (selectedCategory=null effect) → switches appear.
    await expect(page.locator('[role="switch"]').first()).toBeVisible({ timeout: 15000 });
    const before = await page.locator('[role="switch"]').count();
    expect(before).toBeGreaterThanOrEqual(1);

    const searchInput = page.locator('input:not([type="time"]):not([type="hidden"])').first();
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    // A term that matches nothing must empty the list — proves the input drives filtering
    // (vs. the old body.length>100 which passed even with zero filtering).
    await searchInput.fill('zzqx-no-such-product');
    await expect(page.locator('[role="switch"]')).toHaveCount(0, { timeout: 10000 });

    // Clearing restores the full list — filtering is reversible, not a crash.
    await searchInput.fill('');
    await expect(page.locator('[role="switch"]')).toHaveCount(before, { timeout: 10000 });
  });

  test('selecting a sort option applies and persists the sort', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.locator('[role="switch"]').first()).toBeVisible({ timeout: 15000 });

    const sortBtn = page.locator('button').filter({ has: page.locator('.ti-arrows-sort') }).first();
    await expect(sortBtn).toBeVisible({ timeout: 10000 });

    await sortBtn.click();
    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 5000 });

    // Default sort is name → the name option carries the active check, not price-desc.
    const descOption = page.locator('button').filter({ has: page.locator('.ti-sort-descending') }).first();
    await expect(descOption).toBeVisible({ timeout: 5000 });
    expect(await descOption.locator('.ti-check').count()).toBe(0);

    // Pick "price descending" and confirm the selection actually took (reopen → checked).
    await descOption.click();
    await expect(dropdown).toBeHidden();
    await sortBtn.click();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
    const descCheckedNow = page.locator('button')
      .filter({ has: page.locator('.ti-sort-descending') })
      .filter({ has: page.locator('.ti-check') });
    await expect(descCheckedNow).toBeVisible({ timeout: 5000 });
    // TODO(needs-staging): assert the rendered order is actually price-descending — needs a
    // seeded fixture with distinct, locale-stable prices (price text is currency-formatted).
  });

  test('availability filter changes the product list (partition invariant)', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.locator('[role="switch"]').first()).toBeVisible({ timeout: 15000 });
    const all = await page.locator('[role="switch"]').count();
    expect(all).toBeGreaterThanOrEqual(1);

    const filterBtn = page.locator('button').filter({ has: page.locator('.ti-filter') }).first();
    const dropdown = page.locator('[class*="shadow-elevation"]').first();

    // Filter → only stop-listed (unavailable) items.
    await filterBtn.click();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
    await page.locator('button').filter({ has: page.locator('.ti-circle-x') }).first().click();
    await expect(dropdown).toBeHidden();
    const unavailable = await page.locator('[role="switch"]').count();
    expect(unavailable).toBeLessThanOrEqual(all);

    // Filter → only available items.
    await filterBtn.click();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
    await page.locator('button').filter({ has: page.locator('.ti-circle-check') }).first().click();
    await expect(dropdown).toBeHidden();
    const available = await page.locator('[role="switch"]').count();

    // Every product is exactly one of available / unavailable → the two filtered lists
    // must partition the full list. This fails if the filter is a no-op (old test only
    // asserted the dropdown appeared).
    expect(available + unavailable).toBe(all);
  });

  test('products render availability toggles when loaded', async ({ page }) => {
    await gotoMenu(page);

    // At least one product toggle must render (old test asserted >= 0, which can never fail).
    const toggles = page.locator('[role="switch"]');
    await expect(toggles.first()).toBeVisible({ timeout: 15000 });
    expect(await toggles.count()).toBeGreaterThanOrEqual(1);

    // aria-checked must be a real boolean string, not absent.
    const checked = await toggles.first().getAttribute('aria-checked');
    expect(['true', 'false']).toContain(checked);
  });

  test('toggling availability persists via PATCH 200', async ({ page }) => {
    // needs_staging: this MUTATES live product availability — never run against prod.
    requireStaging(BASE);
    await gotoMenu(page);

    const toggle = page.locator('[role="switch"]').first();
    await expect(toggle).toBeVisible({ timeout: 15000 });
    const before = await toggle.getAttribute('aria-checked');

    const patch = page.waitForResponse(
      (r) => /\/owner\/menu\/products\/[^/]+$/.test(r.url()) && r.request().method() === 'PATCH',
    );
    await toggle.click();
    const res = await patch;
    expect(res.status()).toBe(200);
    expect(res.request().postDataJSON()).toHaveProperty('available');

    const flipped = before === 'true' ? 'false' : 'true';
    await expect(toggle).toHaveAttribute('aria-checked', flipped, { timeout: 10000 });

    // Restore original state so staging stays clean.
    const restore = page.waitForResponse(
      (r) => /\/owner\/menu\/products\/[^/]+$/.test(r.url()) && r.request().method() === 'PATCH',
    );
    await toggle.click();
    expect((await restore).status()).toBe(200);
  });

  test('shows an error state (not a blank/empty menu) when the menu fetch fails', async ({ page }) => {
    await page.route('**/owner/menu/categories', (route) =>
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: { code: 'INTERNAL', message: 'boom' } }),
      }),
    );

    await page.goto('/admin/menu?dev=true', { waitUntil: 'domcontentloaded' });

    // The load-error banner (with its Retry button) must show — NOT a silent empty menu.
    const retry = page.locator('button.underline.ml-3');
    await expect(retry).toBeVisible({ timeout: 15000 });
    // And no product cards may render off a failed fetch.
    await expect(page.locator('[role="switch"]')).toHaveCount(0);
  });

  test('admin menu requires auth — no dev bypass redirects to login', async ({ page }) => {
    // Negative auth control: without ?dev=true and with no token, the route must bounce
    // to /login. The other tests all use the dev bypass, so this is the only real gate check.
    await page.goto('/admin/menu', { waitUntil: 'domcontentloaded' });
    await expect(page).toHaveURL(/\/login/, { timeout: 15000 });
    // TODO(needs-staging): add the POSITIVE control — log in with real owner creds and
    // assert the menu renders (proves the gate isn't rejecting everyone).
  });

  test('no JS errors on page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await gotoMenu(page);
    await expect(page.locator('.overflow-x-auto.hide-scrollbar').first()).toBeVisible({ timeout: 10000 });

    const critical = errors.filter((e) =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver'),
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await gotoMenu(page);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
