// API integrity — health, headers, public menu/theme schema contracts, FE↔BE field parity.
// Relocated from apps/api/e2e/ (outside testDir → never ran) per proposal
// `deadsuite:api-integrity-relocate`. Integrity fixes vs the dead original:
//   - BASE was hardcoded to PROD; now VITE_BASE_URL + requireStaging (mock-auth writes).
//   - The in-memory FE↔BE array comparison (a tautology that never touched the API)
//     is replaced with a live GET /api/owner/menu/products field check.
//   - categories[0] access is guarded — an empty menu now FAILS instead of crashing
//     (and the per-product loops assert non-vacuity instead of passing on zero items).
import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const TEST_SLUG = 'demo';

test.beforeAll(() => {
  // The FE↔BE contract check below authenticates via /api/dev/mock-auth, which
  // UPSERTs the dev owner user — a write. Fail fast against prod/unknown targets.
  requireStaging(BASE);
});

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
    expect(headers['x-frame-options'] ?? '').toBe('');
  });

  test('Protected endpoints return structured 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/menu/products`);
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body).toHaveProperty('error');
  });
});

test.describe('SPA renders without errors', () => {
  test('SPA mounts at /s/:slug', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
    await page.goto(`${BASE}/s/${TEST_SLUG}`, { waitUntil: 'domcontentloaded' });
    expect(await page.title()).toBe('Dowiz');
    const root = await page.$('#root');
    expect(root).not.toBeNull();
    const critical = errors.filter(e => !e.includes('Failed to load resource') && !e.includes('404'));
    expect(critical).toEqual([]);
  });

  test('SPA loads JS + CSS assets', async ({ page }) => {
    await page.goto(`${BASE}/s/${TEST_SLUG}`, { waitUntil: 'networkidle' });
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
    expect(res.status()).toBe(200);
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
    expect(res.status()).toBe(200);
    const body = await res.json();
    // Guard: an empty menu is a seed/data failure, not a schema pass.
    expect(body.categories.length, `menu for '${TEST_SLUG}' must have categories to validate the schema against`).toBeGreaterThan(0);
    const cat = body.categories[0];
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
    expect(res.status()).toBe(200);
    const body = await res.json();
    // Non-vacuity guard: the loop below must actually run over at least one product.
    const products = body.categories.flatMap((c: any) => c.products);
    expect(products.length, 'menu must contain at least one product to validate').toBeGreaterThan(0);
    for (const p of products) {
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
  });

  test('Modifier group schema', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    // Shape check where present — a menu with zero modifier groups is legal,
    // but any group that exists must be fully formed.
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
    expect(res.status()).toBe(200);
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
  // Fields the FE Product form (MenuManagerPage) reads/writes and therefore
  // requires the owner API to supply. The old spec "validated" this by comparing
  // two hardcoded arrays in memory — a tautology that stayed green no matter what
  // the API returned. This version asserts the fields on the LIVE response of
  // GET /api/owner/menu/products (mapProductRow output).
  const FE_PRODUCT_FIELDS = [
    'id', 'name', 'price', 'description', 'available', 'categoryId',
    'imageUrl', 'stockCount', 'taste', 'recipeLines',
  ];

  test('Live owner products response carries every FE-required field', async ({ request }) => {
    // mock-auth is dev-gated (x-dev-auth-secret from playwright.config extraHTTPHeaders)
    // and UPSERTs the dev owner — requireStaging in beforeAll guards this write.
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status(), 'mock-auth must succeed (is DEV_AUTH_SECRET set?)').toBe(200);
    const { access_token } = await authRes.json();
    expect(typeof access_token).toBe('string');

    const res = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(res.status()).toBe(200);
    const products = await res.json();
    expect(Array.isArray(products)).toBeTruthy();
    // Non-vacuity guard: field parity can only be proven against real rows.
    expect(products.length, 'dev owner location must have at least one product').toBeGreaterThan(0);
    for (const p of products) {
      for (const field of FE_PRODUCT_FIELDS) {
        expect(p, `owner product ${p.id ?? '<no id>'} is missing FE-required field '${field}'`).toHaveProperty(field);
      }
    }
  });

  test('Public API supplies attributes sub-fields that map to FE extras', async ({ request }) => {
    // The public API returns `attributes` as a JSONB object.
    // When populated, attributes should contain stock_count, taste, bom
    // which are the BE equivalents of FE's stockCount, taste, recipeLines.
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    expect(res.status()).toBe(200);
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
});

test.describe('Menu content integrity', () => {
  test('Menu has at least one category with products', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    const catsWithProducts = body.categories.filter((c: any) => c.products.length > 0);
    expect(catsWithProducts.length).toBeGreaterThan(0);
  });

  test('All product prices are non-negative integers', async ({ request }) => {
    const res = await request.get(`${BASE}/public/locations/${TEST_SLUG}/menu`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    const products = body.categories.flatMap((c: any) => c.products);
    expect(products.length, 'price invariant needs at least one product to check').toBeGreaterThan(0);
    for (const p of products) {
      expect(p.price).toBeGreaterThanOrEqual(0);
      expect(Number.isInteger(p.price)).toBeTruthy();
    }
  });
});
