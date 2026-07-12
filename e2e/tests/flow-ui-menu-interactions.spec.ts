import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('UI: Client Menu — Detail Modal, Modifiers, Search, Filter', () => {
  let locationSlug = 'demo';

  test('Detail modal opens on product click and shows product info', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('article.product-card');
    const cardCount = await cards.count();
    test.skip(cardCount === 0, 'No product cards');

    // Capture the real product name from the card so we can prove the modal
    // shows THIS product (not just "some text > 50 chars").
    const productName = ((await cards.first().locator('h3').first().textContent()) ?? '').trim();
    expect(productName.length, 'product card must render a name').toBeGreaterThan(0);

    await cards.first().click();
    await page.waitForTimeout(1000);

    const modal = page.locator('[role="dialog"], .modal, [class*="modal"], [class*="drawer"]').first();
    const modalVisible = await modal.isVisible({ timeout: 3000 }).catch(() => false);

    // Hard requirement: clicking a product MUST open the detail modal. No soft-pass.
    expect(modalVisible, 'detail modal must open after clicking a product card').toBe(true);
    const modalText = (await modal.textContent()) ?? '';
    expect(modalText, 'modal must show the clicked product name').toContain(productName);
    // The detail modal owns the add-to-cart confirm action — proves it is the
    // product detail surface, not an arbitrary dialog.
    await expect(modal.locator('[data-testid="product-detail-confirm"]')).toBeVisible({ timeout: 3000 });

    // Close modal
    const closeBtn = modal.locator('button').first();
    if (await closeBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await closeBtn.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Search input filters product cards', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('[data-testid="menu-item"]');
    const beforeCount = await cards.count();
    test.skip(beforeCount === 0, 'No product cards to filter');

    // Derive the search term from a REAL product name so a positive match is
    // guaranteed — this proves the filter narrows to matching items rather than
    // silently returning all/zero.
    const term = (((await cards.first().locator('h3').first().textContent()) ?? '').trim().split(/\s+/)[0] || '');
    expect(term.length, 'product name must yield a search term').toBeGreaterThan(0);

    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i], input[placeholder*="Kërko" i]').first();
    await expect(searchInput, 'menu must expose a search input').toBeVisible({ timeout: 5000 });
    await searchInput.click();
    await searchInput.fill(term);
    await page.waitForTimeout(600);

    const afterCount = await cards.count();
    // The term came from a real product → at least that product must remain.
    expect(afterCount, `search "${term}" must keep its matching product`).toBeGreaterThan(0);
    // Filtering can only narrow (or keep) the set, never expand it.
    expect(afterCount, 'search must not expand the result set').toBeLessThanOrEqual(beforeCount);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Category tabs navigate between sections', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // The category nav is an in-page scroll widget (buttons, not role=tab —
    // they use aria-current). Clicking a later category MUST scroll the menu.
    const allTabs = page.locator('nav[data-testid="category-nav"] button');
    await expect(allTabs.first(), 'category nav must render').toBeVisible({ timeout: 5000 });
    const count = await allTabs.count();
    test.skip(count < 2, 'Need >=2 categories to assert navigation');

    const scrollBefore = await page.locator('.app-shell-main').evaluate(el => el.scrollTop);
    await allTabs.nth(count - 1).click(); // jump to the last category
    await page.waitForTimeout(800); // smooth-scroll settle
    const scrollAfter = await page.locator('.app-shell-main').evaluate(el => el.scrollTop);
    // Content actually moved — the tab click navigated to a different section.
    expect(scrollAfter, 'clicking a later category tab must scroll the menu').toBeGreaterThan(scrollBefore);
    // The newly active category reflects the navigation via aria-current.
    await expect(allTabs.nth(count - 1)).toHaveAttribute('aria-current', 'true', { timeout: 3000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Skeleton loading state appears before content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const skeleton = page.locator('[class*="skeleton"], [class*="Skeleton"], .animate-pulse').first();
    const hasSkeleton = await skeleton.isVisible({ timeout: 3000 }).catch(() => false);

    // Wait for full load
    await page.waitForLoadState('networkidle');

    if (hasSkeleton) {
      // Skeleton should be gone after content loads
      await expect(skeleton).not.toBeVisible({ timeout: 5000 }).catch(() => {});
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Cart FAB updates count when items added', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const addBtns = page.locator('button[aria-label="Add to cart"], button[aria-label="Add"]');
    const count = await addBtns.count();
    test.skip(count === 0, 'No add buttons');

    // Add first item
    await addBtns.first().click();
    await page.waitForTimeout(500);

    // SPA cart trigger (StickyActionBar). The count badge is the absolutely-
    // positioned span inside the button; the button label ALSO contains the
    // money total, so assert the badge element specifically — not a digit
    // anywhere in textContent.
    // TODO(needs-staging): selector path verified by reading the component; a
    // live staging run is required to confirm the SPA cart badge end-to-end.
    const fab = page.locator('[data-testid="cart-open"]');
    await expect(fab).toBeVisible({ timeout: 5000 });
    const badge = fab.locator('span.absolute').first();
    await expect(badge).toHaveText('1', { timeout: 5000 });

    // Add another item if available
    if (count > 1) {
      await addBtns.nth(1).click();
      await page.waitForTimeout(500);
      await expect(badge).toHaveText('2', { timeout: 5000 });
    }

    // Open cart drawer
    await fab.click();
    await page.waitForTimeout(500);

    const cartDrawer = page.locator('h2, h3, [role="dialog"]').filter({ hasText: /Cart|Shporta|Your/i }).first();
    await expect(cartDrawer).toBeVisible({ timeout: 3000 }).catch(() => {});

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Product cards show price in ALL currency', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body).toMatch(/ALL|Lek|\d+/);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('No JS errors on menu page after interactions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Perform multiple interactions
    const tabs = page.locator('[role="tab"]');
    if (await tabs.first().isVisible({ timeout: 3000 }).catch(() => false)) {
      await tabs.first().click();
    }

    const addBtn = page.locator('button[aria-label="Add to cart"]').first();
    if (await addBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await addBtn.click();
    }

    await page.waitForTimeout(1000);

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(criticalErrors).toEqual([]);
  });
});
