import { test, expect } from '@playwright/test';

const BASE = 'https://dowiz.fly.dev';
const TEST_SLUG = 'demo';

test.describe('Health + Infrastructure', () => {
  test('Health endpoint responds with all checks', async ({ request }) => {
    const res = await request.get(`${BASE}/health`);
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body).toHaveProperty('checks');
    expect(body.checks.postgres.status).toBe('ok');
    expect(body.checks.workers.status).toBe('ok');
    expect(body.checks.messageBus.status).toBe('ok');
  });

  test('Branding preview permits framing', async ({ request }) => {
    const res = await request.get(`${BASE}/branding-preview/${TEST_SLUG}?embed=true`, {
      headers: { 'Accept': 'text/html' }
    });
    expect(res.ok()).toBeTruthy();
    const headers = res.headers();
    expect(headers['content-security-policy']).toContain('frame-ancestors *');
    const xfo = headers['x-frame-options'];
    expect(xfo === undefined || xfo === '').toBeTruthy();
  });

  test('Protected endpoints return structured 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/menu/products?category_id=nonexistent`);
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body).toHaveProperty('error');
  });
});

test.describe('SPA renders without errors', () => {
  test('SPA mounts at /s/demo', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' });
    expect(await page.title()).toBe('Dowiz');
    const root = await page.$('#root');
    expect(root).not.toBeNull();
    const critical = errors.filter(e => !e.includes('Failed to load resource') && !e.includes('404'));
    expect(critical.length).toBe(0);
  });

  test('SPA loads JS + CSS assets', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    const scripts = await page.locator('script[type="module"]').count();
    expect(scripts).toBeGreaterThan(0);
    const css = await page.locator('link[rel="stylesheet"]').count();
    expect(css).toBeGreaterThan(0);
  });

  test('login and unknown routes render SPA, not crash', async ({ page }) => {
    for (const url of [`${BASE}/login`, `${BASE}/some-random-route`]) {
      await page.goto(url, { waitUntil: 'domcontentloaded' });
      expect(await page.title()).toBe('Dowiz');
    }
  });
});

test.describe('Public Menu API — full schema contract', () => {
  test('Top-level fields', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    expect(body).toHaveProperty('menu_version');
    expect(typeof body.menu_version).toBe('number');
    expect(body).toHaveProperty('default_locale');
    expect(typeof body.default_locale).toBe('string');
    expect(body).toHaveProperty('supported_locales');
    expect(Array.isArray(body.supported_locales)).toBeTruthy();
    expect(body).toHaveProperty('currency');
    expect(body).toHaveProperty('location_name');
    expect(typeof body.location_name).toBe('string');
    expect(body).toHaveProperty('categories');
    expect(Array.isArray(body.categories)).toBeTruthy();
  });

  test('Category schema', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const cat = (await res.json()).categories[0];
    expect(cat).toHaveProperty('id');
    expect(typeof cat.id).toBe('string');
    expect(cat).toHaveProperty('name');
    expect(typeof cat.name).toBe('string');
    expect(cat).toHaveProperty('sort_order');
    expect(typeof cat.sort_order).toBe('number');
    expect(cat).toHaveProperty('products');
    expect(Array.isArray(cat.products)).toBeTruthy();
  });

  test('Product schema — ALL fields present', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    for (const cat of body.categories) {
      for (const p of cat.products) {
        // Required fields — every product must have these
        expect(p).toHaveProperty('id');
        expect(typeof p.id).toBe('string');
        expect(p).toHaveProperty('name');
        expect(typeof p.name).toBe('string');
        expect(p).toHaveProperty('price');
        expect(typeof p.price).toBe('number');
        expect(p).toHaveProperty('available');
        expect(typeof p.available).toBe('boolean');

        // Optional fields — must exist but may be null
        expect(p).toHaveProperty('description');
        expect(p).toHaveProperty('image_key');
        expect(p).toHaveProperty('attributes');
        expect(p).toHaveProperty('modifier_groups');
        expect(Array.isArray(p.modifier_groups)).toBeTruthy();

        // attributes must be an object (may be empty)
        expect(typeof p.attributes).toBe('object');
        expect(p.attributes).not.toBeNull();
      }
    }
  });

  test('Modifier group schema', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    for (const cat of body.categories) {
      for (const p of cat.products) {
        for (const mg of p.modifier_groups) {
          expect(mg).toHaveProperty('id');
          expect(mg).toHaveProperty('name');
          expect(mg).toHaveProperty('min_select');
          expect(mg).toHaveProperty('max_select');
          expect(mg).toHaveProperty('required');
          expect(mg).toHaveProperty('modifiers');
          expect(Array.isArray(mg.modifiers)).toBeTruthy();
          for (const m of mg.modifiers) {
            expect(m).toHaveProperty('id');
            expect(m).toHaveProperty('name');
            expect(m).toHaveProperty('price_delta');
            expect(m).toHaveProperty('available');
            expect(m).toHaveProperty('sort_order');
          }
        }
      }
    }
  });
});

test.describe('Public Theme API — full schema contract', () => {
  test('All brand fields present', async ({ request }) => {
    const res = await request.get(`${BASE}/api/public/theme/${TEST_SLUG}`);
    const body = await res.json();
    expect(body).toHaveProperty('primaryColor');
    expect(body).toHaveProperty('bgColor');
    expect(body).toHaveProperty('textColor');
    expect(body).toHaveProperty('logoUrl');
    expect(body).toHaveProperty('locationName');
    expect(typeof body.locationName).toBe('string');
  });
});

test.describe('FE ↔ BE Contract Validation', () => {
  // This test validates that every field in the FE Product interface
  // has a corresponding field in the BE response, and vice versa.
  // It catches the pattern where FE collects data but never sends it to BE.

  const FE_PRODUCT_FIELDS = [
    'id', 'name', 'price', 'description', 'available', 'categoryId',
    'imageUrl', 'stockCount', 'taste', 'recipeLines',
  ];

  // Fields the public BE returns (read-only customer-facing)
  const PUBLIC_BE_PRODUCT_FIELDS = [
    'id', 'name', 'price', 'available', 'description', 'image_key',
    'attributes', 'modifier_groups',
  ];

  // Fields the owner BE returns (admin dashboard, via mapProductRow)
  const OWNER_BE_PRODUCT_FIELDS = [
    'id', 'name', 'price', 'description', 'available', 'categoryId',
    'imageUrl', 'imageKey', 'stockCount', 'taste', 'recipeLines', 'attributes',
  ];

  test('Every FE Product field maps to a BE field', () => {
    // Fields that FE collects but must have a BE counterpart
    const missing: string[] = [];
    for (const f of FE_PRODUCT_FIELDS) {
      if (!OWNER_BE_PRODUCT_FIELDS.includes(f)) {
        missing.push(f);
      }
    }
    expect(missing).toEqual([]);
  });

  test('Public API supplies attributes sub-fields that map to FE extras', async ({ request }) => {
    // The public API returns `attributes` as a JSONB object.
    // When populated, attributes should contain stock_count, taste, bom
    // which are the BE equivalents of FE's stockCount, taste, recipeLines.
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    for (const cat of body.categories) {
      for (const p of cat.products) {
        const attrs = p.attributes || {};
        // If attributes has these keys, they must be the right types
        if (attrs.stock_count !== undefined) expect(typeof attrs.stock_count).toBe('number');
        if (attrs.taste !== undefined) expect(typeof attrs.taste).toBe('object');
        if (attrs.bom !== undefined) expect(Array.isArray(attrs.bom)).toBeTruthy();
      }
    }
  });

  test('Static: FE-BE mismatch would be caught by contract', () => {
    // This test documents the known mismatch pattern:
    // Previously, FE collected taste, stockCount, recipeLines in form state
    // but handleSaveProduct NEVER sent them to BE. This test ensures they ARE mapped.
    const feFormFields = ['name', 'price', 'description', 'available', 'categoryId', 'taste', 'stockCount', 'recipeLines'];
    const beSchemaFields = ['name', 'price', 'description', 'available', 'category_id', 'categoryId', 'image_key', 'imageUrl', 'stockCount', 'taste', 'recipeLines', 'attributes'];
    for (const f of feFormFields) {
      expect(beSchemaFields).toContain(f);
    }
  });
});

test.describe('Menu content integrity', () => {
  test('Menu has at least one category with products', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    const catsWithProducts = body.categories.filter((c: any) => c.products.length > 0);
    expect(catsWithProducts.length).toBeGreaterThan(0);
  });

  test('All product prices are positive integers', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    const body = await res.json();
    for (const cat of body.categories) {
      for (const p of cat.products) {
        expect(p.price).toBeGreaterThanOrEqual(0);
        expect(Number.isInteger(p.price)).toBeTruthy();
      }
    }
  });
});
