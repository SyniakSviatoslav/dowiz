import { test, expect } from '@playwright/test';

// Hardened against false-greens (AGENTS.md "Test Integrity"): every test asserts a
// specific product-domain [data-testid] is visible — never body.length / loose regex —
// and no assertion is wrapped in an if(count>0) conditional-skip. Render waits are
// web-first (expect-visible / poll), never fixed waitForTimeout sleeps.
const MENU_ITEM = '[data-testid="menu-item"]';
const RENDER_TIMEOUT = 20000;

test.describe('Client Menu — Interaction Tests', () => {
  test('menu page loads with nav and product cards', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

    // Sticky category nav + at least one real product card must render.
    await expect(page.locator('[data-testid="category-nav"]')).toBeVisible({ timeout: RENDER_TIMEOUT });
    await expect(page.locator(MENU_ITEM).first()).toBeVisible({ timeout: RENDER_TIMEOUT });
    await expect(page.locator('nav').first()).toBeVisible();
  });

  test('category tab click scrolls to its section', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

    const nav = page.locator('[data-testid="category-nav"]');
    await expect(nav).toBeVisible({ timeout: RENDER_TIMEOUT });

    const tabs = nav.locator('button');
    const tabCount = await tabs.count();
    expect(tabCount).toBeGreaterThan(0);

    // Sections render in the same order as their tabs (id={category.id}); clicking the
    // last tab must scroll its section into the viewport — a real DOM consequence, not a no-op.
    const sections = page.locator('section[id]');
    await expect(sections.last()).toBeAttached({ timeout: RENDER_TIMEOUT });
    await tabs.last().click();
    await expect(sections.last()).toBeInViewport({ timeout: 5000 });
  });

  test('product card renders with name and price', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

    const cards = page.locator(MENU_ITEM);
    await expect(cards.first()).toBeVisible({ timeout: RENDER_TIMEOUT });
    // A blank menu must FAIL the test — no body.length fallback.
    expect(await cards.count()).toBeGreaterThan(0);

    const firstCard = cards.first();
    await expect(firstCard.locator('h3')).toBeVisible(); // product name
    await expect(firstCard).toContainText(/\d/); // price (scoped to the card, not the body)
  });

  test('search input actually filters products', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

    const items = page.locator(MENU_ITEM);
    await expect(items.first()).toBeVisible({ timeout: RENDER_TIMEOUT });
    const baseline = await items.count();
    expect(baseline).toBeGreaterThan(0);

    const searchInput = page.locator('input[placeholder*="Search"]');
    await expect(searchInput).toHaveCount(1);

    // Derive a real query token from a rendered product so we don't depend on seed text.
    const firstName = (await items.first().locator('h3').textContent())?.trim() ?? '';
    const token = firstName.split(/\s+/)[0];
    expect(token.length).toBeGreaterThan(0);

    await searchInput.fill(token);
    // Filtering narrows (or holds) the set, and the source product stays matched.
    await expect.poll(async () => items.count()).toBeLessThanOrEqual(baseline);
    await expect(items.filter({ hasText: token }).first()).toBeVisible();

    // A query that cannot match anything must zero out the grid — proves the filter is live.
    await searchInput.fill('zzqqx-no-such-product');
    await expect.poll(async () => items.count()).toBe(0);
  });

  test('add to cart updates the cart FAB badge', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

<<<<<<< Updated upstream
    // Find plus buttons (add to cart) on product cards
    const addBtns = page.locator('button[aria-label="Add to cart"]');
    const count = await addBtns.count();
    if (count > 0) {
      // Click first add button
      await addBtns.first().click();
      await page.waitForTimeout(1000);
=======
    const addBtns = page.locator('[data-testid="menu-item-add"]');
    await expect(addBtns.first()).toBeVisible({ timeout: RENDER_TIMEOUT });
>>>>>>> Stashed changes

    // FAB renders only when the cart is non-empty; clicking add must make it appear with count "1".
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toHaveCount(0);
    await addBtns.first().click();
    await expect(fab).toBeVisible({ timeout: 10000 });
    await expect(fab).toContainText('1');
  });

  test('product cards show product info and add button', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });

<<<<<<< Updated upstream
    const productCards = page.locator('article.product-card').first();
    await expect(productCards).toBeVisible({ timeout: 15000 });

    const count = await page.locator('article.product-card').count();
    expect(count).toBeGreaterThanOrEqual(1);

    // Each card should have a price displayed
    const body = await page.textContent('body');
    expect(body).toMatch(/\d+/);
=======
    const cards = page.locator(MENU_ITEM);
    await expect(cards.first()).toBeVisible({ timeout: RENDER_TIMEOUT });
    expect(await cards.count()).toBeGreaterThanOrEqual(1);

    // Each card must carry a name, an add button, and a price digit.
    await expect(cards.first().locator('h3')).toBeVisible();
    await expect(cards.first().locator('[data-testid="menu-item-add"]')).toBeVisible();
    await expect(cards.first()).toContainText(/\d/);
>>>>>>> Stashed changes
  });

  test('no JS errors on client menu', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await expect(page.locator(MENU_ITEM).first()).toBeVisible({ timeout: RENDER_TIMEOUT });

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('no cookies set on client menu', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await expect(page.locator(MENU_ITEM).first()).toBeVisible({ timeout: RENDER_TIMEOUT });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
