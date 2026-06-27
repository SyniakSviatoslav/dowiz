import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const TEST_PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMklEQVQ4T2NkYPj/n4EBBJgYKAQMowYMAwMDhRE0YBhGDRgGNGAYRg0YBjRgGNUfAABF1wH5r5lRawAAAABJRU5ErkJggg==';

let authToken: string;
let productId: string;
const TS = Date.now();

test.describe('UI: Image Upload — Product + Brand Logo', () => {
  test.beforeAll(async ({ request }) => {
    // This suite MUTATES tenant state (creates categories/products, uploads images).
    // Hard-guard against running it (and the dev mock-auth backdoor) at a prod target.
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    expectJwt(authToken, 'mock-auth access_token');

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `IMG-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `IMG-Prod-${TS}`, price: 500, available: true, categoryId },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
    expectUuid(productId, 'productId');
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort fixture cleanup in afterAll must not fail the suite */ });
    }
  });

  test('Flow 1: Upload product image returns an absolute image URL', async ({ request }) => {
    const pngBuf = Buffer.from(TEST_PNG_BASE64, 'base64');
    const imgRes = await request.post(`${BASE}/api/owner/menu/products/${productId}/image`, {
      headers: { Authorization: `Bearer ${authToken}` },
      multipart: { file: { name: 'test.png', mimeType: 'image/png', buffer: pngBuf } },
    });
    // The route returns reply.send({ imageUrl, imageKey }) → 200 on success
    // (spa-proxy.ts:237). Any 4xx/5xx is a real failure, not an acceptable outcome.
    expect(imgRes.status()).toBe(200);
    const body = await imgRes.json();
    // imageUrl is an absolute browser URL (R2 public host or <appBase>/images/<key>,
    // image-url.ts) — a relative path, '', 'null', or an error fragment must fail.
    expect(body.imageUrl).toMatch(/^https:\/\/\S+\.webp$/);
    expect(body.imageKey).toMatch(/\.webp$/);
  });

  test('Flow 2: Unauthenticated upload returns 401', async ({ request }) => {
    const pngBuf = Buffer.from(TEST_PNG_BASE64, 'base64');
    const imgRes = await request.post(`${BASE}/api/owner/menu/products/${productId}/image`, {
      multipart: { file: { name: 'test.png', mimeType: 'image/png', buffer: pngBuf } },
    });
    expect(imgRes.status()).toBe(401);
  });

  test('Flow 3: Get brand config returns theme settings', async ({ request }) => {
    const brandRes = await request.get(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(brandRes.status()).toBe(200);
    const body = await brandRes.json();
    // Assert the actual contract, not mere truthiness ({} is truthy). The response
    // always carries the tenant's locationId (a UUID) + the theme key set
    // (spa-proxy.ts:515-529); primaryColor may be null for an unthemed tenant.
    expectUuid(body.locationId, 'brand.locationId');
    expect(body).toHaveProperty('primaryColor');
    expect(body).toHaveProperty('logoUrl');
  });

  test('Flow 4: Update brand config — GET works, PUT may 500 (known bug)', async ({ request }) => {
    // GET always works
    const getRes = await request.get(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const before = await getRes.json();
    // Contract shape, not truthy-OR: the GET returns camelCase keys (spa-proxy.ts:515-529).
    expectUuid(before.locationId, 'brand.locationId');
    expect(before).toHaveProperty('primaryColor');

    // brandSchema is .strict() with camelCase keys (spa-proxy.ts:13-26); these snake_case
    // keys are unrecognized → ZodError → setErrorHandler returns 400 VALIDATION_FAILED
    // (server.ts:434-457), never a 500.
    const putRes = await request.put(`${BASE}/api/owner/brand`, {
      data: { primary_color: '#E53935', secondary_color: '#1E88E5' },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(putRes.status()).toBe(400);
  });

  test('Flow 5: Public theme CSS loads with CSS variables', async ({ request }) => {
    const themeRes = await request.get(`${BASE}/public/locations/demo/theme.css`);
    expect(themeRes.status()).toBe(200);
    const css = await themeRes.text();
    expect(css).toContain('--brand-primary');
    expect(css).toContain(':root');
  });

  test('Flow 6: Branding page UI loads without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle' });

    // Assert the real branding UI rendered (not a 500/redirect/loading skeleton):
    // the branding-page root + its heading must be visible. body.length>N would pass
    // on an error page or skeleton.
    await expect(page.getByTestId('branding-page')).toBeVisible({ timeout: 15000 });
    await expect(page.getByRole('heading', { level: 2 }).first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // TODO(needs_staging): cross-tenant IDOR coverage is MISSING and cannot be written
  // honestly here — the owner path of /api/dev/mock-auth always mints a token for the
  // single dev owner (dev@deliveryos.com, mock-auth.ts:71), so there is no real second
  // tenant to exercise isolation against. Faking it with a nil-UUID would 404 by absence
  // and prove nothing (AGENTS Test Integrity #5). To implement, provision a REAL second
  // owner+location on staging, then:
  //   - POST /api/owner/menu/products/${productId}/image with tenant-2's token → expect 403/404.
  //   - GET /api/owner/brand with tenant-2's token → expect tenant-2's locationId (not tenant-1's);
  //     PUT as tenant-2, then re-GET tenant-1's brand and assert it is unchanged.
});
