import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.DEV_AUTH_SECRET || '';
// A real, DISTINCT second tenant slug is required for the cross-tenant IDOR check.
const OTHER_SLUG = process.env.SECOND_LOCATION_SLUG || '';

// 1x1 PNGs (distinct bytes → distinct content hash → distinct image key).
const RED = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
const BLUE = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');

// The 12-hex content hash the route embeds in the key (`${loc}/${pid}-${hash}.webp`).
const hashOf = (key: string): string => key.match(/-([0-9a-f]{12})\.webp$/)?.[1] ?? '';

// Full product-image lifecycle against real R2: upload (add) → serve via the
// /images/* proxy → display on the client SPA → change (re-upload) yields a new
// cache-busted URL. Skips if no DEV_AUTH_SECRET (the dev upload gate).
test.describe('Product images — upload, serve, display, change', () => {
  test.skip(!SECRET, 'requires DEV_AUTH_SECRET for the dev owner token');
  // Mutating spec (writes images into a tenant) — refuse to run against prod.
  test.beforeAll(() => requireStaging(BASE));

  test('image is added, served as webp via the proxy, displayed on the SPA, and changeable', async ({ request, page }) => {
    // owner token + a product in the demo location
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, {
      headers: { 'x-dev-auth-secret': SECRET, 'content-type': 'application/json' },
      data: { role: 'owner', locationSlug: 'demo' },
    });
    expect(auth.ok()).toBeTruthy();
    const token = (await auth.json()).access_token as string;
    expectJwt(token);

    const menu = await (await request.get(`${BASE}/public/locations/demo/menu`)).json();
    const product = menu.categories.flatMap((c: any) => c.products || [])[0];
    expectUuid(product?.id, 'demo product id');

    // ── auth gate: an unauthenticated upload MUST be rejected (proves the gate holds) ──
    const noAuth = await request.post(`${BASE}/api/owner/menu/products/${product.id}/image`, {
      multipart: { file: { name: 'red.png', mimeType: 'image/png', buffer: RED } },
    });
    expect(noAuth.status()).toBe(401);

    // ── add: upload RED ──
    const up1 = await request.post(`${BASE}/api/owner/menu/products/${product.id}/image`, {
      headers: { authorization: `Bearer ${token}` },
      multipart: { file: { name: 'red.png', mimeType: 'image/png', buffer: RED } },
    });
    expect(up1.ok()).toBeTruthy();
    const r1 = await up1.json();
    // served through the app proxy at THIS origin, never a raw private-R2 URL
    expect(r1.imageUrl.startsWith(`${BASE}/images/`)).toBe(true);
    expect(r1.imageUrl).not.toContain('r2.cloudflarestorage.com');
    const hash1 = hashOf(r1.imageKey);
    expect(hash1).toMatch(/^[0-9a-f]{12}$/);

    // ── cross-tenant isolation: a DIFFERENT owner must not write into demo's product ──
    // TODO(needs_staging): requires SECOND_LOCATION_SLUG to be a real, distinct staging
    // tenant. Secure expectation is 404 (product not in the caller's location), mirroring
    // the owner/product-media.ts ensureProduct guard. NOTE: spa-proxy's upload handler
    // currently UPDATEs `WHERE location_id = $caller` (0 rows) yet still returns 200 — this
    // assertion will go RED until that handler rejects the cross-tenant write. Do not weaken.
    test.skip(!OTHER_SLUG, 'cross-tenant IDOR check requires SECOND_LOCATION_SLUG (a real 2nd tenant)');
    const auth2 = await request.post(`${BASE}/api/dev/mock-auth`, {
      headers: { 'x-dev-auth-secret': SECRET, 'content-type': 'application/json' },
      data: { role: 'owner', locationSlug: OTHER_SLUG },
    });
    expect(auth2.ok()).toBeTruthy();
    const token2 = (await auth2.json()).access_token as string;
    expectJwt(token2, 'second-tenant token');
    const crossUp = await request.post(`${BASE}/api/owner/menu/products/${product.id}/image`, {
      headers: { authorization: `Bearer ${token2}` },
      multipart: { file: { name: 'blue.png', mimeType: 'image/png', buffer: BLUE } },
    });
    expect(crossUp.status()).toBe(404);

    // ── serve: the proxy returns a real webp ──
    const served = await request.get(r1.imageUrl);
    expect(served.status()).toBe(200);
    expect(served.headers()['content-type']).toContain('image/webp');
    expect((await served.body()).byteLength).toBeGreaterThan(0);

    // ── display: the image renders + decodes on the client SPA ──
    // TODO(needs_staging): SPA render assertions require a live deployed target.
    await page.goto(`${BASE}/s/demo`);
    const img = page.locator(`img[src*="${product.id}"]`).first();
    await expect(img).toBeVisible({ timeout: 15_000 });
    // the served src must be THIS upload's hash, not a stale cached image from a prior run
    await expect.poll(() => img.getAttribute('src'), { timeout: 15_000 }).toContain(hash1);
    await img.scrollIntoViewIfNeeded(); // images are loading="lazy"
    await expect.poll(() => img.evaluate((e: HTMLImageElement) => e.naturalWidth), { timeout: 15_000 }).toBeGreaterThan(0);

    // ── change: re-upload BLUE → new content hash → new URL ──
    const up2 = await request.post(`${BASE}/api/owner/menu/products/${product.id}/image`, {
      headers: { authorization: `Bearer ${token}` },
      multipart: { file: { name: 'blue.png', mimeType: 'image/png', buffer: BLUE } },
    });
    expect(up2.ok()).toBeTruthy();
    const r2 = await up2.json();
    expect(r2.imageUrl).not.toBe(r1.imageUrl); // cache-busted: a changed image is a new URL
    const hash2 = hashOf(r2.imageKey);
    expect(hash2).toMatch(/^[0-9a-f]{12}$/);
    expect(hash2).not.toBe(hash1);
    const served2 = await request.get(r2.imageUrl);
    expect(served2.status()).toBe(200);
    expect(served2.headers()['content-type']).toContain('image/webp');

    // ── the SPA reflects the change: after reload, the rendered src is the NEW hash ──
    // TODO(needs_staging): live SPA reload; proves the old URL is superseded, not just that
    // the new URL serves 200.
    await page.reload();
    const img2 = page.locator(`img[src*="${product.id}"]`).first();
    await expect(img2).toBeVisible({ timeout: 15_000 });
    await expect.poll(() => img2.getAttribute('src'), { timeout: 15_000 }).toContain(hash2);
    expect(await img2.getAttribute('src')).not.toContain(hash1);
  });
});
