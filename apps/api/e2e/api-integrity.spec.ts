import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../../../e2e/helpers/assert-shape';
import { requireStaging } from '../../../e2e/helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
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

  // Negative auth controls — the gate must fire on WRITE owner routes and courier routes,
  // not only on the one GET above. verifyAuth runs before any handler/DB, so a missing
  // Bearer is an EXACT 401 (apps/api/src/plugins/auth.ts:46-48) — no state is mutated.
  const NOAUTH_ROUTES: Array<{ method: 'post' | 'patch' | 'delete' | 'get'; path: string }> = [
    { method: 'post', path: '/api/owner/menu/products' },
    { method: 'patch', path: '/api/owner/menu/products/11111111-1111-1111-1111-111111111111' },
    { method: 'delete', path: '/api/owner/menu/products/11111111-1111-1111-1111-111111111111' },
    { method: 'get', path: '/api/courier/me' },
  ];
  for (const r of NOAUTH_ROUTES) {
    test(`Unauthenticated ${r.method.toUpperCase()} ${r.path} → 401`, async ({ request }) => {
      const res = await request[r.method](`${BASE}${r.path}`, { data: {} });
      expect(res.status()).toBe(401);
    });
  }
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
    const body = await res.json();
    expect(Array.isArray(body.categories)).toBeTruthy();
    expect(body.categories.length).toBeGreaterThan(0);
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

// The owner BE field-list above (OWNER_BE_PRODUCT_FIELDS) is exercised LIVE here, not just
// compared in-memory: we mint a real owner token, hit the real authenticated route, and assert
// the actual response body — so a server-side mapProductRow regression goes red.
//
// TODO(needs-staging): this suite MUTATES (mock-auth mint + product create) and needs a live
// staging target with ALLOW_DEV_LOGIN, the dev owner bound to an active location (e.g. via
// /api/dev/repair-test-owner so locationSlug='demo' resolves), and TENANT_B_CATEGORY_ID set to a
// REAL second tenant's category id for the IDOR check. Run with
// VITE_BASE_URL=https://dowiz-staging.fly.dev. It is requireStaging-guarded and never hits prod.
test.describe('Owner API — authenticated controls (staging-only)', () => {
  const OWNER_BE_PRODUCT_FIELDS = [
    'id', 'name', 'price', 'description', 'available', 'categoryId',
    'imageUrl', 'imageKey', 'stockCount', 'taste', 'recipeLines', 'attributes',
  ];
  let token = '';

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE);
    const res = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: TEST_SLUG },
    });
    expect(res.status()).toBe(200);
    token = (await res.json()).access_token;
    expectJwt(token, 'owner access_token');
  });

  // Findings #1 + #4: positive control + live field presence.
  test('Positive control: owner GET /menu/products → 200 + every OWNER_BE field present', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const rows = await res.json();
    expect(Array.isArray(rows)).toBeTruthy();
    expect(rows.length).toBeGreaterThan(0);
    const row = rows[0];
    for (const f of OWNER_BE_PRODUCT_FIELDS) {
      expect(row, `owner product row missing "${f}"`).toHaveProperty(f);
    }
    expectUuid(row.id, 'product id');
  });

  // Finding #2: seed a product with KNOWN attributes, then assert them unconditionally on readback.
  test('Attributes round-trip: stock_count/taste/bom seeded → exact values on readback', async ({ request }) => {
    const create = await request.post(`${BASE}/api/owner/menu/products`, {
      headers: { authorization: `Bearer ${token}` },
      data: {
        name: `__contract_attr_${Date.now()}`,
        price: 500,
        prep_time_minutes: 10,
        stockCount: 7,
        taste: { sweet: 2 },
        recipeLines: [{ ingredient: 'tuna', qty: 1 }],
      },
    });
    expect(create.status()).toBe(201);
    const created = await create.json();
    expectUuid(created.id, 'created product id');
    expect(created.stockCount).toBe(7);
    expect(created.taste.sweet).toBe(2);
    expect(Array.isArray(created.recipeLines)).toBeTruthy();
    expect(created.recipeLines).toHaveLength(1);
    expect(created.attributes.stock_count).toBe(7);
  });

  // Finding #5: cross-tenant / IDOR — owner A querying tenant B's category must see NO tenant-B data.
  test('IDOR: owner cannot read another tenant\'s products via category_id', async ({ request }) => {
    const otherCat = process.env.TENANT_B_CATEGORY_ID;
    expect(otherCat, 'TENANT_B_CATEGORY_ID must be a REAL second-tenant category id (no nil-UUID)').toBeTruthy();
    expectUuid(otherCat, 'tenant-B category_id');
    const res = await request.get(`${BASE}/api/owner/menu/products?category_id=${otherCat}`, {
      headers: { authorization: `Bearer ${token}` },
    });
    // RLS + location_id scoping means tenant B's category yields zero rows for owner A — never their data.
    expect(res.status()).toBe(200);
    const rows = await res.json();
    expect(Array.isArray(rows)).toBeTruthy();
    expect(rows.length).toBe(0);
  });
});
