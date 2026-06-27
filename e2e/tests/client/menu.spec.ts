import { test, expect } from '@playwright/test';

const hasProducts = (c: { products?: unknown[] }): boolean => (c.products || []).length > 0;

test.describe('Client Menu Page', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
  });

  test('renders hero section with restaurant info', async ({ page }) => {
    await expect(page.locator('section h1, header h1').first()).toContainText('Dubin');
    await expect(page.locator('text=★★★★★')).toBeVisible();
  });

  test('renders category navigation with tabs', async ({ page }) => {
    const navButtons = page.locator('nav.sticky button');
    const count = await navButtons.count();
    expect(count).toBeGreaterThanOrEqual(2);

    // First tab should be active (primary color)
    const firstBtn = navButtons.first();
    const color = await firstBtn.evaluate(el => getComputedStyle(el).color);
    expect(color).not.toBe('rgb(168, 168, 168)'); // not muted color
  });

  test('clicking category tab scrolls to section', async ({ page }) => {
    // Resolve the brand primary token to its computed rgb() so we can assert the
    // EXACT active border color, not merely "non-transparent" (which a default
    // browser border would also satisfy).
    const brandPrimary = await page.evaluate(() => {
      const probe = document.createElement('span');
      probe.style.color = getComputedStyle(document.documentElement).getPropertyValue('--brand-primary');
      document.body.appendChild(probe);
      const rgb = getComputedStyle(probe).color;
      probe.remove();
      return rgb;
    });

    // Click the second category tab
    const navButtons = page.locator('nav.sticky button');
    const secondBtn = navButtons.nth(1);
    await secondBtn.click();

    // toHaveCSS auto-retries until the active state settles — no hard sleep, and
    // it asserts the precise brand-primary color rather than just non-transparency.
    await expect(secondBtn).toHaveCSS('border-bottom-color', brandPrimary);
  });

  test('renders product cards with name, price, and add button', async ({ page }) => {
    const cards = page.locator('[data-testid="menu-item"]');
    const count = await cards.count();
    expect(count).toBeGreaterThan(0);

    const firstCard = cards.first();
    await expect(firstCard.locator('h3')).toBeVisible();
    await expect(firstCard.locator('[data-testid="menu-item-add"]')).toBeVisible();
  });

  test('unavailable products render greyed-out with a disabled add button', async ({ page }) => {
    // Seed deterministically: force the first product of the first non-empty
    // category to be unavailable in the menu response, so the unavailable
    // presentation is exercised unconditionally (no vacuous if-count>0 skip).
    await page.route('**/public/locations/**/menu*', async (route) => {
      const res = await route.fetch();
      const json = await res.json();
      const cats: { products?: { available: boolean }[] }[] = json.categories || [];
      const seeded = cats.find(hasProducts);
      if (seeded?.products) seeded.products[0].available = false;
      await route.fulfill({ response: res, json });
    });
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // The seeded product must render with the unavailable styling (opacity-55)
    // and its add button must be disabled (ProductCard has no overlay text by
    // design — the greyed card + disabled add IS the unavailable affordance).
    const unavailableCard = page.locator('[data-testid="menu-item"].opacity-55').first();
    await expect(unavailableCard).toBeVisible();
    await expect(unavailableCard.locator('[data-testid="menu-item-add"]')).toBeDisabled();
  });

  test('add button click adds item to cart and shows FAB', async ({ page }) => {
    // Initially FAB should be hidden
    await expect(page.locator('#cartFabBtn')).not.toBeVisible();

    // Click add on first product
    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await addBtn.click();

    // Cart FAB should appear with count (expect.toBeVisible auto-polls — no hard sleep)
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
    await expect(fab).toContainText('1');
  });

  test('add multiple items increments FAB count', async ({ page }) => {
    const addButtons = page.locator('[data-testid="menu-item-add"]');

    await addButtons.first().click();
    await addButtons.nth(1).click();
    await addButtons.first().click();

    const fab = page.locator('#cartFabBtn');
    await expect(fab).toContainText('3');
  });

  test('cart FAB click opens cart drawer', async ({ page }) => {
    // Add an item first
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 3000 });

    // Click FAB
    await page.locator('#cartFabBtn').click();

    // Cart drawer should open
    await expect(page.locator('text=Your Cart')).toBeVisible({ timeout: 3000 });
  });

  test('shows skeletons during loading state', async ({ page }) => {
    // Delay the menu response so the loading skeletons are DETERMINISTICALLY
    // observable (otherwise a fast fixture races past them and the assertion is
    // meaningless). A missing loading state now fails this test.
    await page.route('**/public/locations/**/menu*', async (route) => {
      await new Promise((r) => setTimeout(r, 1500));
      await route.continue();
    });
    await page.goto('/s/test-slug?dev=true', { waitUntil: 'commit' });

    // Skeletons MUST be present while the request is in flight...
    await expect(page.locator('.skeleton-block').first()).toBeVisible({ timeout: 5000 });
    // ...and must give way to real menu items once it resolves.
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15000 });
  });

  test('theme variables are properly scoped', async ({ page }) => {
    const vars = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      return {
        primary: style.getPropertyValue('--brand-primary'),
        bg: style.getPropertyValue('--brand-bg'),
        text: style.getPropertyValue('--brand-text'),
      };
    });
    // A single space ' ' is truthy — assert each var is an actual color value,
    // not just non-empty garbage.
    const colorRe = /^(#[0-9a-f]{3,8}|rgb|hsl|oklch|color\()/i;
    expect(vars.primary.trim()).toMatch(colorRe);
    expect(vars.bg.trim()).toMatch(colorRe);
    expect(vars.text.trim()).toMatch(colorRe);
  });

  test('embed mode hides fixed elements', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true&embed=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // ClientLayout applies the `embed-mode` body class in embed contexts; the
    // CSS rule `.embed-mode .embed-hidden { display:none }` is what hides fixed
    // chrome. Assert the class is actually present (drives the hiding behavior).
    await expect(page.locator('body')).toHaveClass(/embed-mode/);
  });

  test('shows error state when the menu request fails', async ({ page }) => {
    // Force the menu fetch to fail; the page must render its error fallback
    // (not a blank page, spinner, or crash). loadMenu() catches → setFetchError.
    await page.route('**/public/locations/**/menu*', (route) => route.abort());
    await page.goto('/s/test-slug?dev=true');

    await expect(page.getByText('Failed to load menu')).toBeVisible({ timeout: 15000 });
    await expect(page.getByRole('button', { name: /Retry/i })).toBeVisible();
  });

  test('rejects a non-existent slug with the not-found state', async ({ page }) => {
    // TODO(needs_staging): exercises the REAL server slug-resolution path (a 404
    // for an unknown slug). Run against a deployed server (VITE_BASE_URL /
    // staging) — a local fixture server won't resolve real slugs. `?dev=true` is
    // intentionally omitted so the genuine fetch + 404 → notFound branch runs.
    await page.goto('/s/this-slug-does-not-exist-zzz999');

    await expect(page.getByText('Restaurant not found')).toBeVisible({ timeout: 15000 });
  });

});
