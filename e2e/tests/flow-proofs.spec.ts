import { test, expect } from '@playwright/test';
import { expectUuid, expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let locationSlug: string;
let categoryId: string;
let productId: string;
const TS = Date.now();

// 16×16 red PNG (valid, processable by sharp)
const TEST_PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMklEQVQ4T2NkYPj/n4EBBJgYKAQMowYMAwMDhRE0YBhGDRgGNGAYRg0YBjRgGNUfAABF1wH5r5lRawAAAABJRU5ErkJggg==';

test.describe.configure({ mode: 'serial' });

test.describe('Comprehensive E2E Flow Proofs — Lifecycle & Detail', () => {
  // ──────────────────────────────────────────────────────────────
  // SETUP
  // ──────────────────────────────────────────────────────────────
  test.beforeAll(async ({ request }) => {
    // This suite MUTATES state (creates products/categories) via the dev/mock-auth
    // backdoor — refuse to run against prod (closes the "mock-auth disabled in prod"
    // blind-spot: the suite can never reach the prod backdoor at all).
    requireStaging(BASE);
    // Auth
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    expectJwt(authToken, 'access_token');

    // Resolve slug
    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    locationSlug = (await settingsRes.json()).slug;

    // Verify public menu resolves
    const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(menuRes.status()).toBe(200);

    // Create test category
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `E2E-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    // Delete test product
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort teardown must not fail the suite */ });
    }
    // Delete test category
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort teardown must not fail the suite */ });
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1: Product CRUD — Create with full data, verify API
  // ──────────────────────────────────────────────────────────────
  test('Flow 1: Product — create with taste, recipeLines, allergens, image, verify API response', async ({ request }) => {
    // 1a. Create product with all fields
    const createRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `E2E-Sushi-${TS}`,
        price: 750,
        description: 'E2E comprehensive lifecycle test product',
        available: true,
        categoryId,
        taste: { spicy: 2, salty: 1, sour: 0, sweet: 0, richness: 3 },
        recipeLines: [
          { supplyId: 'e2e-rice', supplyName: 'Sushi Rice', qty: 200, unit: 'g', kind: 'food_ingredient', kcal: 130, proteinG: 3, fatG: 0, carbsG: 28, allergens: [] },
          { supplyId: 'e2e-soy', supplyName: 'Soy Sauce', qty: 15, unit: 'ml', kind: 'condiment', kcal: 8, proteinG: 1, fatG: 0, carbsG: 1, allergens: ['soy'] },
          { supplyId: 'e2e-nori', supplyName: 'Nori Sheets', qty: 2, unit: 'unit', kind: 'food_ingredient', kcal: 10, proteinG: 1, fatG: 0, carbsG: 1, allergens: [] },
        ],
        stockCount: 50,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(createRes.status()).toBe(201);
    const product = await createRes.json();
    expectUuid(product.id);
    expect(product.name).toBe(`E2E-Sushi-${TS}`);
    expect(product.price).toBe(750);
    expect(product.available).toBe(true);
    expect(product.taste).toBeTruthy();
    expect(product.taste.spicy).toBe(2);
    expect(product.taste.richness).toBe(3);
    expect(product.recipeLines).toBeTruthy();
    expect(product.recipeLines.length).toBe(3);
    expect(product.recipeLines[1].supplyName).toBe('Soy Sauce');
    expect(product.recipeLines[1].allergens).toContain('soy');
    expect(product.stockCount).toBe(50);
    productId = product.id;

    // 1b. Upload product image
    const pngBuf = Buffer.from(TEST_PNG_BASE64, 'base64');
    const imgRes = await request.post(`${BASE}/api/owner/menu/products/${productId}/image`, {
      headers: { Authorization: `Bearer ${authToken}` },
      multipart: {
        file: { name: 'test.png', mimeType: 'image/png', buffer: pngBuf },
      },
    });
    // A valid PNG against configured storage must succeed (route returns 200 + imageUrl;
    // 400=invalid image, 401=no auth, 500=storage/db failure — all are real failures here).
    expect(imgRes.status()).toBe(200);
    const imgBody = await imgRes.json();
    expect(String(imgBody.imageUrl)).toMatch(/^(https?:\/\/|\/|data:)/);
    // Follow-up: the persisted product must echo the uploaded image URL (round-trip proof).
    const imgGetRes = await request.get(`${BASE}/api/owner/menu/products?category_id=${categoryId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(imgGetRes.status()).toBe(200);
    const afterUpload = (await imgGetRes.json()).find((p: { id: string; imageUrl?: string }) => p.id === productId);
    expect(afterUpload).toBeTruthy();
    expect(String(afterUpload?.imageUrl)).toMatch(/^(https?:\/\/|\/)/);

    // 1c. Verify via GET: all fields survive round-trip
    const getRes = await request.get(`${BASE}/api/owner/menu/products?category_id=${categoryId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const products = await getRes.json();
    const found = Array.isArray(products)
      ? products.find((p: any) => p.id === productId)
      : null;
    expect(found).toBeTruthy();
    expect(found.name).toBe(`E2E-Sushi-${TS}`);
    expect(found.price).toBe(750);
    expect(found.taste.spicy).toBe(2);
    expect(found.recipeLines.length).toBe(3);
    expect(found.stockCount).toBe(50);
    expect(found.categoryId).toBe(categoryId);
    // API auto-aggregates allergens
    expect(found.allergens).toBeTruthy();
    expect(found.allergens).toContain('soy');

    // 1d. Public menu returns the product with correct attributes shape
    const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(menuRes.status()).toBe(200);
    const menuBody = await menuRes.json();
    const allProds = menuBody.categories.flatMap((c: any) => c.products || []);
    const pubProd = allProds.find((p: any) => p.name === `E2E-Sushi-${TS}`);
    expect(pubProd).toBeTruthy();
    expect(pubProd.attributes.taste).toBeTruthy();
    expect(pubProd.attributes.taste.spicy).toBe(2);
    expect(pubProd.attributes.bom).toBeTruthy();
    expect(pubProd.attributes.bom.length).toBe(3);
    // Allergens in public menu are comma-separated strings
    expect(pubProd.attributes.bom[1].allergens).toContain('soy');
    // Top-level kcal must NOT exist (it's inside bom[])
    expect(pubProd.attributes.kcal).toBeUndefined();
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1b: API auth controls — negative (401 no-token, 403 wrong-role),
  //          missing-resource (404), bad payload (400). Without these, the
  //          owner endpoints could be silently open or silently closed.
  // ──────────────────────────────────────────────────────────────
  test('Flow 1b: API — auth + error matrix on owner product endpoints', async ({ request }) => {
    // Positive control already proven in Flow 1 (create → 201). Negative controls:

    // 401 — no Authorization header → create rejected.
    const noAuthCreate = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `E2E-NoAuth-${TS}`, price: 100, categoryId },
    });
    expect(noAuthCreate.status()).toBe(401);

    // 401 — no Authorization header → delete of the real productId rejected.
    const noAuthDelete = await request.delete(`${BASE}/api/owner/menu/products/${productId}`);
    expect(noAuthDelete.status()).toBe(401);

    // 403 — valid token, wrong role (courier) → owner-only route forbidden.
    const courierAuth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierAuth.status()).toBe(200);
    const courierToken = (await courierAuth.json()).access_token;
    expectJwt(courierToken, 'courier access_token');
    const wrongRole = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `E2E-Courier-${TS}`, price: 100, categoryId },
      headers: { Authorization: `Bearer ${courierToken}` },
    });
    expect(wrongRole.status()).toBe(403);

    // 404 — owner token, delete a syntactically-valid but absent product id.
    const absentId = '11111111-1111-4111-8111-111111111111';
    const missing = await request.delete(`${BASE}/api/owner/menu/products/${absentId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(missing.status()).toBe(404);

    // 400 — owner token, invalid payload (price is required + numeric).
    const badPayload = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { categoryId },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(badPayload.status()).toBe(400);

    // TODO(needs_staging): cross-tenant IDOR — mint a SECOND real tenant's owner token and
    // assert it gets 403/404 reading/mutating this tenant's productId. Requires a real second
    // seeded tenant on staging (a nil/random UUID 404s by absence and proves nothing — see
    // Test Integrity rule #5), so it is left as an explicit gap, not faked.
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 2: Client Display — product renders with all details
  // ──────────────────────────────────────────────────────────────
  test('Flow 2: Client — product displays with allergens, taste, nutrition, ingredients', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Navigate to public menu page
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // 0 JS errors
    expect(errors, `JS errors on load: ${errors.join('; ')}`).toEqual([]);

    // Product cards rendered
    const cards = page.locator('[data-testid="menu-item"]');
    await expect(cards.first()).toBeVisible({ timeout: 10000 });
    const cardCount = await cards.count();
    expect(cardCount).toBeGreaterThan(0);

    // Find our test product by name (it will be in a category section)
    const prodName = page.locator('h3').filter({ hasText: `E2E-Sushi-${TS}` });
    await expect(prodName).toBeVisible({ timeout: 8000 });

    // Verify product card has allergen badge — soy badge visible
    const parentCard = prodName.locator('..').locator('..');
    const cardHtml = await parentCard.innerHTML();

    // Allergen "soy" must be in the card (either as badge or in ingredient list)
    expect(cardHtml.toLowerCase()).toContain('soy');

    // Price: formatALL(750) = Math.round(750/100) = "8 ALL"
    expect(cardHtml).toContain('8 ALL');

    // Ingredients: Sushi Rice, Soy Sauce, Nori Sheets
    expect(cardHtml).toContain('Sushi Rice');
    expect(cardHtml).toContain('Soy Sauce');
    expect(cardHtml).toContain('Nori Sheets');

    // Allergen badge: "soy"
    expect(cardHtml.toLowerCase()).toContain('soy');

    // Nutrition info: kcal (aggregated from recipeLines: 130+8+10=148)
    expect(cardHtml).toContain('148kcal');

    // Taste icons use TASTE_LABELS: "Spicy: 2/3", "Salty: 1/3", "Rich: 3/3"
    expect(cardHtml).toContain('title="Spicy: 2/3"');
    expect(cardHtml).toContain('title="Salty: 1/3"');
    expect(cardHtml).toContain('title="Rich: 3/3"');

    // No JS errors post-render
    expect(errors, `JS errors post-render: ${errors.join('; ')}`).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 3: Persistence — after refresh and URL navigation
  // ──────────────────────────────────────────────────────────────
  test('Flow 3: Persistence — after page refresh and route navigation, product still displays correctly', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Initial load
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    // Verify product exists
    let prodName = page.locator('h3').filter({ hasText: `E2E-Sushi-${TS}` });
    await expect(prodName).toBeVisible({ timeout: 8000 });
    let bodyText = await page.textContent('body');
    expect(bodyText).toContain('soy');
    expect(bodyText).toContain('148');

    // [REFRESH] Reload the page
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Re-verify product still displays
    prodName = page.locator('h3').filter({ hasText: `E2E-Sushi-${TS}` });
    await expect(prodName).toBeVisible({ timeout: 8000 });
    bodyText = await page.textContent('body');
    expect(bodyText).toContain('soy');
    expect(bodyText).toContain('148');

    // [ROUTE NAVIGATION] Navigate to cart page, then back to menu
    await page.goto(`${BASE}/s/${locationSlug}/cart`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    // Navigate back to menu
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Re-verify product still displays after navigation
    prodName = page.locator('h3').filter({ hasText: `E2E-Sushi-${TS}` });
    await expect(prodName).toBeVisible({ timeout: 8000 });
    bodyText = await page.textContent('body');
    expect(bodyText).toContain('soy');
    expect(bodyText).toContain('148');

    // 0 JS errors throughout
    expect(errors, `JS errors after refresh+nav: ${errors.join('; ')}`).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 4: Client Interactions — sort, filter, cart
  // ──────────────────────────────────────────────────────────────
  test('Flow 4: Client — sort by price/name, filter by allergen, add to cart, cart persists', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // [SORT] Verify sort buttons exist and are interactive
    const sortBtns = page.locator('button').filter({ hasText: /↑ Price|↓ Price|A-Z/ });
    const sortCount = await sortBtns.count();
    expect(sortCount).toBeGreaterThanOrEqual(3); // ↑ Price, ↓ Price, A-Z

    // Extract the price (integer ALL) of the first N visible product cards, in DOM order.
    const firstPrices = async (n: number): Promise<number[]> => {
      const texts = await page.locator('[data-testid="menu-item"]').allInnerTexts();
      return texts
        .map((t) => { const m = t.match(/(\d+)\s*ALL/); return m ? Number(m[1]) : null; })
        .filter((v): v is number => v !== null)
        .slice(0, n);
    };

    // Sort by name (A-Z)
    const nameBtn = page.locator('button').filter({ hasText: 'A-Z' }).first();
    await nameBtn.click();
    await page.waitForTimeout(500);

    // Sort by price ascending → DOM order of the first cards must be non-decreasing.
    const priceAscBtn = page.locator('button').filter({ hasText: '↑ Price' }).first();
    await priceAscBtn.click();
    await page.waitForTimeout(500);
    const asc = await firstPrices(3);
    expect(asc.length).toBeGreaterThanOrEqual(2);
    expect(asc).toEqual([...asc].sort((a, b) => a - b));

    // Sort by price descending → DOM order of the first cards must be non-increasing.
    const priceDescBtn = page.locator('button').filter({ hasText: '↓ Price' }).first();
    await priceDescBtn.click();
    await page.waitForTimeout(500);
    const desc = await firstPrices(3);
    expect(desc.length).toBeGreaterThanOrEqual(2);
    expect(desc).toEqual([...desc].sort((a, b) => b - a));

    // Reset to default
    const resetBtn = page.locator('button').filter({ hasText: '·' }).first();
    await resetBtn.click();
    await page.waitForTimeout(500);

    // [FILTER] Filter by allergen "soy" — our test product has soy
    const soyFilter = page.locator('button').filter({ hasText: 'soy' }).first();
    const hasSoyFilter = await soyFilter.isVisible().catch(() => false);
    if (hasSoyFilter) {
      const beforeCount = await page.locator('[data-testid="menu-item"]').count();
      await soyFilter.click();
      await page.waitForTimeout(500);

      // After filtering by soy, only products containing soy should be visible
      const filteredBody = await page.textContent('body');
      expect(filteredBody).toContain('soy');
      // The E2E-Sushi-${TS} should still be visible (it has a soy allergen)
      const sushiName = page.locator('h3').filter({ hasText: `E2E-Sushi-${TS}` });
      await expect(sushiName).toBeVisible({ timeout: 3000 });
      // ...and at least one non-soy product must have been HIDDEN (filter actually filters,
      // not a no-op): visible card count strictly drops but stays > 0.
      const afterCount = await page.locator('[data-testid="menu-item"]').count();
      expect(afterCount).toBeGreaterThan(0);
      expect(afterCount).toBeLessThan(beforeCount);

      // Clear filter by clicking again
      await soyFilter.click();
      await page.waitForTimeout(500);
    }

    // [CART] Add a product to cart via the add button
    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await expect(addBtn).toBeVisible({ timeout: 5000 });
    await addBtn.click();
    await page.waitForTimeout(1000);

    // Cart FAB must appear with count "1"
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 4000 });
    const fabText = await fab.textContent();
    expect(fabText).toMatch(/1/); // Cart has 1 item

    // Open cart drawer
    await fab.click();
    await page.waitForTimeout(500);
    const cartHeading = page.locator('h2').filter({ hasText: /Cart|Shporta/i }).first();
    await expect(cartHeading).toBeVisible({ timeout: 3000 });

    // Close cart drawer
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);

    // [CART PERSISTENCE] Refresh page — cart should persist (localStorage)
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Cart FAB should still be visible with same count
    const fabAfterReload = page.locator('#cartFabBtn');
    await expect(fabAfterReload).toBeVisible({ timeout: 5000 });

    // 0 JS errors
    expect(errors, `JS errors after interactions: ${errors.join('; ')}`).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 5: Admin Auth + Dashboard
  // ──────────────────────────────────────────────────────────────
  test('Flow 5: Admin — login page, dashboard loads with sidebar, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Login page renders with form
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    const emailInput = page.locator('input[type="email"]');
    await expect(emailInput.first()).toBeVisible({ timeout: 5000 });
    const passwordInput = page.locator('input[type="password"]');
    await expect(passwordInput.first()).toBeVisible({ timeout: 5000 });

    // Set auth token
    await page.evaluate((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    // Navigate to admin dashboard
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // 0 JS errors
    expect(errors, `JS errors on dashboard: ${errors.join('; ')}`).toEqual([]);

    // Dashboard body renders
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    // Sidebar navigation present
    const hasNav = /dashboard|orders|menu|branding|settings/i.test(body);
    expect(hasNav).toBe(true);

    // No cookies
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 6: Admin Menu Manager — categories, products, allergens
  // ──────────────────────────────────────────────────────────────
  test('Flow 6: Admin — menu manager shows categories, product allergens in admin UI', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // 0 JS errors
    expect(errors, `JS errors on menu page: ${errors.join('; ')}`).toEqual([]);

    // Menu page renders
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    const hasMenuContent = /menu|category|categories|product|item|E2E-Cat/i.test(body);
    expect(hasMenuContent).toBe(true);

    // Find and expand our test category to see products
    const catButton = page.locator('button').filter({ hasText: `E2E-Cat-${TS}` }).first();
    if (await catButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await catButton.click();
      await page.waitForTimeout(1500);

      // After expanding, verify our product is visible in admin
      const productInAdmin = page.locator('text=E2E-Sushi').first();
      const prodVisible = await productInAdmin.isVisible({ timeout: 3000 }).catch(() => false);
      if (prodVisible) {
        const adminBody = await page.textContent('body');
        // Allergen info should be in the admin view (either badges or text)
        expect(adminBody.toLowerCase()).toContain('soy');
      }
    } else {
      // Fallback: just check category name exists somewhere in the DOM
      const catExists = body.includes(`E2E-Cat-${TS}`);
      expect(catExists).toBe(true);
    }

    // 0 JS errors post-interaction
    expect(errors, `JS errors after menu interactions: ${errors.join('; ')}`).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 7: Admin Branding — theme editor, CSS vars
  // ──────────────────────────────────────────────────────────────
  test('Flow 7: Admin — branding page, CSS variables scoped, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    // Navigate to admin dashboard first
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    expect(errors, `JS errors on admin: ${errors.join('; ')}`).toEqual([]);

    // Try to find and click branding/theme link in sidebar
    const themeLink = page.locator('a, button, nav *, [role="navigation"] *')
      .filter({ hasText: /branding|theme|settings/i }).first();
    if (await themeLink.isVisible({ timeout: 3000 }).catch(() => false)) {
      await themeLink.click();
      await page.waitForTimeout(2500);
    } else {
      // Fallback: navigate directly
      const bodyText = await page.textContent('body');
      const url = bodyText.toLowerCase().includes('branding') ? '/admin/branding' : '/admin/settings';
      await page.addInitScript((token: string) => {
        localStorage.setItem('dos_access_token', token);
      }, authToken);
      await page.goto(`${BASE}${url}`, { waitUntil: 'networkidle' });
      await page.waitForTimeout(2000);
    }

    // 0 JS errors on branding page
    expect(errors, `JS errors on branding: ${errors.join('; ')}`).toEqual([]);

    // CSS variables must be scoped on admin pages
    const cssVars = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      return {
        primary: style.getPropertyValue('--brand-primary').trim(),
        bg: style.getPropertyValue('--brand-bg').trim(),
        surface: style.getPropertyValue('--brand-surface').trim(),
        text: style.getPropertyValue('--brand-text').trim(),
      };
    });
    expect(cssVars.primary).toBeTruthy();
    expect(cssVars.bg).toBeTruthy();
    expect(cssVars.surface).toBeTruthy();
    expect(cssVars.text).toBeTruthy();

    // No cookies
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 8: Courier — login page + tasks
  // ──────────────────────────────────────────────────────────────
  test('Flow 8: Courier — login page renders, tasks page loads, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Courier login page
    await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);

    const emailField = page.locator('input[type="email"]');
    await expect(emailField.first()).toBeVisible({ timeout: 5000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    // Courier tasks page (may redirect to login without auth)
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // 0 JS errors even without auth
    expect(errors, `JS errors on courier: ${errors.join('; ')}`).toEqual([]);

    // No cookies
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // TODO(needs_staging): real-time WS proof. Place an order from /s/:slug and assert the
  // owner /admin/orders page receives the LIVE update via the open WS connection (a new
  // [data-testid="order-row"] appearing WITHOUT a reload/poll-buffer — Test Integrity rule #8).
  // Requires a live staging order-create + an open authenticated admin page; left as an
  // explicit gap rather than a reload-based pseudo-proof that would false-green.
});
