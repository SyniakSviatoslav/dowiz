import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SECRET = process.env.DEV_AUTH_SECRET || '';

// 1x1 PNGs (distinct bytes → distinct content hash → distinct image key).
const RED = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
const BLUE = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');

// Full product-image lifecycle against real R2: upload (add) → serve via the
// /images/* proxy → display on the client SPA → change (re-upload) yields a new
// cache-busted URL. Skips if no DEV_AUTH_SECRET (the dev upload gate).
test.describe('Product images — upload, serve, display, change', () => {
  test.skip(!SECRET, 'requires DEV_AUTH_SECRET for the dev owner token');

  test('image is added, served as webp via the proxy, displayed on the SPA, and changeable', async ({ request, page }) => {
    // owner token + a product in the demo location
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, {
      headers: { 'x-dev-auth-secret': SECRET, 'content-type': 'application/json' },
      data: { role: 'owner', locationSlug: 'demo' },
    });
    expect(auth.ok()).toBeTruthy();
    const token = (await auth.json()).access_token as string;
    expect(token).toBeTruthy();

    const menu = await (await request.get(`${BASE}/public/locations/demo/menu`)).json();
    const product = menu.categories.flatMap((c: any) => c.products || [])[0];
    expect(product?.id, 'demo has at least one product').toBeTruthy();

    // ── add: upload RED ──
    const up1 = await request.post(`${BASE}/api/owner/menu/products/${product.id}/image`, {
      headers: { authorization: `Bearer ${token}` },
      multipart: { file: { name: 'red.png', mimeType: 'image/png', buffer: RED } },
    });
    expect(up1.ok()).toBeTruthy();
    const r1 = await up1.json();
    // served through the app proxy, never a raw private-R2 URL
    expect(r1.imageUrl).toContain('/images/');
    expect(r1.imageUrl).not.toContain('r2.cloudflarestorage.com');

    // ── serve: the proxy returns a real webp ──
    const served = await request.get(r1.imageUrl);
    expect(served.status()).toBe(200);
    expect(served.headers()['content-type']).toContain('image/webp');
    expect((await served.body()).byteLength).toBeGreaterThan(0);

    // ── display: the image renders + decodes on the client SPA ──
    await page.goto(`${BASE}/s/demo`);
    const img = page.locator(`img[src*="${product.id}"]`).first();
    await expect(img).toBeVisible({ timeout: 15_000 });
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
    const served2 = await request.get(r2.imageUrl);
    expect(served2.status()).toBe(200);
    expect(served2.headers()['content-type']).toContain('image/webp');
  });
});
