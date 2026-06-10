import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;
let locationId: string;
let locationSlug: string;

test.describe.configure({ mode: 'serial' });

test.describe('Deploy Validation — Live Session Proofs', () => {

  // ── 0. Auth: get owner token via mock-auth ─────────────────────────
  test('0.1 — mock-auth returns valid owner token', async ({ request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: {},
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.access_token).toBeTruthy();
    expect(body.userId).toBeTruthy();
    authToken = body.access_token;
    if (body.activeLocationId) {
      locationId = body.activeLocationId;
    }
  });

  // ── 1. Auth 401 fixes ───────────────────────────────────────────────
  test('1.1 — unauthenticated GET /api/owner/locations returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/locations`);
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.error).toBe('Unauthorized');
  });

  test('1.2 — unauthenticated GET /api/courier/assignments returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/courier/me/assignments`);
    expect(res.status()).toBe(401);
  });

  test('1.3 — unauthenticated GET /api/customer/orders returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/customer/orders`);
    expect(res.status()).toBe(401);
  });

  // ── 2. Order validation fixes ──────────────────────────────────────
  test('2.1 — GET /orders/invalid-uuid returns 400, not 500', async ({ request }) => {
    const res = await request.get(`${BASE}/api/orders/not-a-valid-uuid`);
    expect(res.status()).toBe(400);
    const body = await res.json();
    expect(body.error).toContain('Invalid');
  });

  test('2.2 — POST /orders with empty body returns 400, not 500', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders`, {
      data: {},
      headers: { 'Content-Type': 'application/json' },
    });
    expect(res.status()).toBe(400);
    const body = await res.json();
    expect(body.error).toBeTruthy();
  });

  // ── 3. Slug contract: settings API slug resolves to live endpoints ──
  test('3.1 — settings API returns slug that resolves on public endpoints', async ({ request }) => {
    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const settings = await settingsRes.json();
    expect(settings.slug).toBeTruthy();
    locationSlug = settings.slug;

    const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(menuRes.status()).toBe(200);

    const themeRes = await request.get(`${BASE}/api/public/theme/${locationSlug}`);
    expect(themeRes.status()).toBe(200);
    const theme = await themeRes.json();
    expect(theme.primaryColor).toBeTruthy();
  });

  // ── 4. Owner menu — product creation with taste + recipeLines ──────
  let createdCategoryId: string;
  let createdProductId: string;

  test('4.1 — create category via owner API', async ({ request }) => {
    const catName = `Test-Cat-${Date.now()}`;
    const res = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: catName },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    expect(body.id).toBeTruthy();
    expect(body.name).toBe(catName);
    createdCategoryId = body.id;
  });

  test('4.2 — create product with taste + recipeLines via owner API', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: 'Pita Test Sushi',
        price: 950,
        description: 'RecipeLines validation test',
        available: true,
        categoryId: createdCategoryId,
        taste: { spicy: 2, salty: 1, sour: 0, sweet: 0, richness: 3 },
        recipeLines: [
          { supplyId: 's-rice', supplyName: 'Sushi Rice', qty: 100, unit: 'g', kind: 'food_ingredient', kcal: 130, proteinG: 3, fatG: 0, carbsG: 28, allergens: [] },
          { supplyId: 's-wasabi', supplyName: 'Wasabi', qty: 5, unit: 'g', kind: 'condiment', kcal: 15, proteinG: 0, fatG: 0, carbsG: 3, allergens: [] },
        ],
        stockCount: 42,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    expect(body.id).toBeTruthy();
    expect(body.name).toBe('Pita Test Sushi');
    expect(body.taste).toBeTruthy();
    expect(body.taste.spicy).toBe(2);
    expect(body.recipeLines).toBeTruthy();
    expect(body.recipeLines.length).toBe(2);
    expect(body.recipeLines[0].supplyName).toBe('Sushi Rice');
    expect(body.stockCount).toBe(42);
    createdProductId = body.id;
  });

  test('4.3 — PATCH product preserves recipeLines on edit', async ({ request }) => {
    const res = await request.patch(`${BASE}/api/owner/menu/products/${createdProductId}`, {
      data: {
        name: 'Pita Test Sushi Updated',
        price: 1050,
        taste: { spicy: 3, salty: 2 },
        recipeLines: [
          { supplyId: 's-rice', supplyName: 'Sushi Rice', qty: 150, unit: 'g', kind: 'food_ingredient', kcal: 195, proteinG: 4, fatG: 0, carbsG: 42, allergens: [] },
          { supplyId: 's-nori', supplyName: 'Nori Sheets', qty: 2, unit: 'unit', kind: 'food_ingredient', kcal: 10, proteinG: 1, fatG: 0, carbsG: 1, allergens: [] },
          { supplyId: 's-soy', supplyName: 'Soy Sauce', qty: 15, unit: 'ml', kind: 'condiment', kcal: 8, proteinG: 1, fatG: 0, carbsG: 1, allergens: ['soy'] },
        ],
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.name).toBe('Pita Test Sushi Updated');
    expect(body.price).toBe(1050);
    expect(body.taste.spicy).toBe(3);
    expect(body.recipeLines.length).toBe(3);
    expect(body.recipeLines[2].supplyName).toBe('Soy Sauce');
    expect(body.recipeLines[2].allergens).toContain('soy');
  });

  // ── 5. Public menu API: attributes shape contract ───────────────────
  test('5.1 — public menu returns attributes with taste+bom, NOT top-level kcal', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.categories).toBeTruthy();
    expect(body.categories.length).toBeGreaterThan(0);
    const allProducts = body.categories.flatMap((c: any) => c.products || []);
    const pitaProduct = allProducts.find((p: any) => p.name && p.name.includes('Pita Test Sushi Updated'));
    expect(pitaProduct).toBeTruthy();

    expect(pitaProduct.attributes).toBeTruthy();
    expect(pitaProduct.attributes.taste).toBeTruthy();
    expect(pitaProduct.attributes.taste.spicy).toBe(3);

    expect(pitaProduct.attributes.bom).toBeTruthy();
    expect(pitaProduct.attributes.bom.length).toBe(3);
    expect(pitaProduct.attributes.bom[2].allergens).toContain('soy');

    expect(pitaProduct.attributes.kcal).toBeUndefined();
    expect(pitaProduct.attributes.protein).toBeUndefined();
    expect(pitaProduct.attributes.fat).toBeUndefined();
  });

  // ── 6. Image upload — strict assertion (no 500 accepted) ──────────────
  test('6.1 — image upload without auth returns 401', async ({ request }) => {
    const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
    const res = await request.post(`${BASE}/api/owner/menu/products/${createdProductId}/image`, {
      headers: { Authorization: '' },
      multipart: {
        file: { name: 'test.png', mimeType: 'image/png', buffer: fakePng },
      },
    });
    expect(res.status()).toBe(401);
  });

  test('6.2 — image upload with auth returns 200 or 400 for invalid image (STRICT: 500 is a failure)', async ({ request }) => {
    const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFklEQVQYV2P8z8BQz0BFwMgwakChAAB0VQx8W6je5QAAAABJRU5ErkJggg==', 'base64');
    const res = await request.post(`${BASE}/api/owner/menu/products/${createdProductId}/image`, {
      headers: { Authorization: `Bearer ${authToken}` },
      multipart: {
        file: { name: 'test.png', mimeType: 'image/png', buffer: fakePng },
      },
    });
    expect(res.status()).not.toBe(500);
    expect(res.status()).not.toBe(401);
    if (res.status() === 400) {
      const body = await res.json();
      expect(body.error).toMatch(/invalid|No file|image/i);
    }
    if (res.status() === 200) {
      const body = await res.json();
      expect(body.imageUrl || body.imageKey).toBeTruthy();
    }
  });

  // ── 7. Menu import AI — LLM adapter detection (strict: no 500) ──────
  test('7.1 — menu import endpoint handles LLM unavailability gracefully (no 500 crash)', async ({ request }) => {
    const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
    const res = await request.post(`${BASE}/api/owner/menu/import/preview`, {
      headers: { Authorization: `Bearer ${authToken}` },
      multipart: {
        file: { name: 'menu.png', mimeType: 'image/png', buffer: fakePng },
        mode: 'add_only',
      },
    });
    expect(res.status()).not.toBe(401);
    expect(res.status()).not.toBe(404);
    expect(res.status()).not.toBe(500);
    const contentType = res.headers()['content-type'] || '';
    if (contentType.includes('application/json')) {
      const body = await res.json();
      if (body.issues && body.issues.length > 0) {
        const llmIssue = body.issues.find((i: any) => i.code === 'PARSE_ERROR');
        if (llmIssue) {
          console.log('LLM issue (expected if no LLM service):', llmIssue.message);
        }
      }
    }
  });

  // ── 8. Settlements health check fix ────────────────────────────────
  test('8.1 — health check shows settlement as OK (not BROKEN)', async ({ request }) => {
    const res = await request.get(`${BASE}/health`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.checks.settlement.status).toBe('ok');
  });

  // ── 9. SSR X-Menu-Version header ──────────────────────────────────
  test('9.1 — SSR page includes X-Menu-Version header', async ({ request }) => {
    const res = await request.get(`${BASE}/s/${locationSlug}`);
    expect(res.status()).toBe(200);
    expect(res.headers()['x-menu-version']).toBeTruthy();
  });

  // ── 10. SPA /dashboard route fix ────────────────────────────────
  test('10.1 — /dashboard returns 200 (SPA fallback)', async ({ request }) => {
    const res = await request.get(`${BASE}/dashboard`);
    expect(res.status()).toBe(200);
    const text = await res.text();
    expect(text).toContain('root');
  });

  // ── 11. Product data round-trip: admin → client ────────────────────
  test('11.1 — admin product list includes taste + recipeLines', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/menu/products?category_id=${createdCategoryId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const products = await res.json();
    const found = Array.isArray(products) ? products.find((p: any) => p.id === createdProductId) : null;
    expect(found).toBeTruthy();
    expect(found.taste).toBeTruthy();
    expect(found.taste.spicy).toBe(3);
    expect(found.recipeLines.length).toBe(3);
    expect(found.stockCount).toBe(42);
  });

  // ── 12. Theme endpoint resolves with slug from settings ─────────────
  test('12.1 — theme endpoint returns valid data for settings slug', async ({ request }) => {
    expect(locationSlug).toBeTruthy();
    const themeRes = await request.get(`${BASE}/api/public/theme/${locationSlug}`);
    expect(themeRes.status()).toBe(200);
    const theme = await themeRes.json();
    expect(theme.primaryColor).toBeTruthy();
  });

  // ── 13. Cleanup: delete test product ────────────────────────────────
  test('13.1 — delete test product', async ({ request }) => {
    const res = await request.delete(`${BASE}/api/owner/menu/products/${createdProductId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 204]).toContain(res.status());
  });

  test('13.2 — delete test category', async ({ request }) => {
    const res = await request.delete(`${BASE}/api/owner/menu/categories/${createdCategoryId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 204]).toContain(res.status());
  });
});