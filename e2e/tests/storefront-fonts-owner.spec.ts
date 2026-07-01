/**
 * Per-tenant fonts — owner picker + dynamic loader E2E (staging-only, serial, mutating).
 * Uses the /api/dev/mock-auth backdoor; NEVER runs against prod (requireStaging).
 *
 *  A. Negative: PUT /owner/brand with a malformed font id → 400 VALIDATION_FAILED.
 *  B. PUT /owner/brand {headingFont:'bebas', bodyFont:'poppins'} → 200, echoes the ids.
 *  C. GET /owner/brand → ids persisted.
 *  D. GET /api/public/theme/{slug} → the storefront read returns the owner's font ids.
 *  E. /s/{slug} → the product title renders 'Bebas Neue' (a NON-BASE font ⇒ proves the dynamic
 *     Google-Fonts <link> loader injected + applied it), and a link[data-tenant-font] is present.
 *  F. afterAll: restore the owner's baseline fonts.
 */
import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

let ownerToken: string;
let baseline: { headingFont: string | null; bodyFont: string | null };
let slug: string;

test.describe.configure({ mode: 'serial' });

test.describe('Per-tenant fonts — owner picker + dynamic loader', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE);
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status(), 'mock-auth must mint an owner token').toBe(200);
    ownerToken = (await r.json()).access_token;
    expectJwt(ownerToken, 'access_token');
    const b = await (await request.get(`${BASE}/api/owner/brand`, { headers: { Authorization: `Bearer ${ownerToken}` } })).json();
    baseline = { headingFont: b.headingFont ?? null, bodyFont: b.bodyFont ?? null };
    const s = await (await request.get(`${BASE}/api/owner/settings`, { headers: { Authorization: `Bearer ${ownerToken}` } })).json();
    slug = s.slug;
  });

  test('A: PUT with a malformed font id is rejected 400 (bounded id charset)', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { headingFont: 'Bad Font!' },
    });
    expect(r.status(), 'malformed font id must be rejected').toBe(400);
    expect((await r.json()).code).toBe('VALIDATION_FAILED');
  });

  test('B: PUT persists a non-base heading font + body font (echoed)', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { headingFont: 'bebas', bodyFont: 'poppins' },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.headingFont, 'PUT echoes the persisted heading id').toBe('bebas');
    expect(body.bodyFont, 'PUT echoes the persisted body id').toBe('poppins');
  });

  test('C: GET confirms the font ids persisted', async ({ request }) => {
    const body = await (await request.get(`${BASE}/api/owner/brand`, { headers: { Authorization: `Bearer ${ownerToken}` } })).json();
    expect(body.headingFont).toBe('bebas');
    expect(body.bodyFont).toBe('poppins');
  });

  test('D: the public storefront theme read returns the owner font ids', async ({ request }) => {
    test.skip(!slug, 'no slug');
    const body = await (await request.get(`${BASE}/api/public/theme/${slug}`)).json();
    expect(body.headingFont).toBe('bebas');
    expect(body.bodyFont).toBe('poppins');
  });

  test('E: /s/{slug} renders the non-base font (dynamic loader injected + applied it)', async ({ page }) => {
    test.skip(!slug, 'no slug');
    await page.goto(`${BASE}/s/${slug}`);
    const title = page.getByTestId('menu-item').first().locator('h3').first();
    await title.waitFor({ state: 'visible', timeout: 20000 });
    // The tenant chose a NON-BASE font — proof it renders means ensureGoogleFonts injected its <link>.
    await expect.poll(async () => title.evaluate((el) => getComputedStyle(el).fontFamily), { timeout: 10000 })
      .toContain('Bebas Neue');
    const injected = await page.locator('link[data-tenant-font]').count();
    expect(injected, 'a tenant Google-Fonts <link> must have been injected').toBeGreaterThan(0);
  });

  test('F: selecting "Default" (explicit null) CLEARS the font, not a COALESCE no-op', async ({ request }) => {
    const put = await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { headingFont: null, bodyFont: null },
    });
    expect(put.status()).toBe(200);
    const body = await (await request.get(`${BASE}/api/owner/brand`, { headers: { Authorization: `Bearer ${ownerToken}` } })).json();
    expect(body.headingFont, 'explicit null must clear the heading font (reset to cuisine default)').toBeNull();
    expect(body.bodyFont, 'explicit null must clear the body font').toBeNull();
  });

  test.afterAll(async ({ request }) => {
    if (!ownerToken || !baseline) return;
    // Send explicit values; COALESCE keeps nulls as-is, so push empty-string→null explicitly via the
    // API contract (null clears). Baseline was likely null → restore to null by sending null.
    await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { headingFont: baseline.headingFont, bodyFont: baseline.bodyFont },
    });
  });
});
