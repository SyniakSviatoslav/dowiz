/**
 * Branding settings lifecycle E2E — serial, UI-first
 *
 * 1. API: GET /owner/brand — read baseline, record for restore
 * 2. API: GET /owner/settings — get current slug (should be sushi-durres after migration)
 * 3. UI: /admin/branding loads, form visible, no JS errors
 * 4. API: PUT /owner/brand with new primary + bg colors
 * 5. API: GET /owner/brand — confirm colors persisted
 * 6. UI: Reload /admin/branding — form inputs show updated colors
 * 7. Public: /branding-preview/{slug} loads without JS errors
 * 8. Public: /s/{slug} loads and renders client menu (same page as branding-preview)
 * 9. Public: /api/public/theme/{slug} returns updated primary color
 * 10. API: Restore original branding
 */
import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

const NEW_PRIMARY = '#e11d48';
const NEW_BG = '#fdf2f8';

let ownerToken: string;
let baseline: Record<string, any>;
let locationSlug: string;

test.describe.configure({ mode: 'serial' });

test.describe('UI: Admin Branding — color/logo update cycle', () => {

  test.beforeAll(async ({ request }) => {
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status()).toBe(200);
    const body = await r.json();
    ownerToken = body.access_token;
    expect(ownerToken).toBeTruthy();
  });

  // ── STEP 1: Read baseline branding ────────────────────────────────────────────
  test('Step 1: API — GET /owner/brand returns 200 and valid shape', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status(), 'Brand endpoint must return 200').toBe(200);
    baseline = await r.json();
    expect(baseline).toHaveProperty('id');
    console.log('Baseline brand:', {
      primaryColor: baseline.primaryColor,
      bgColor: baseline.bgColor,
      textColor: baseline.textColor,
      logoUrl: baseline.logoUrl,
    });
  });

  // ── STEP 2: Get slug from settings ───────────────────────────────────────────
  test('Step 2: API — GET /owner/settings returns slug', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status()).toBe(200);
    const settings = await r.json();
    locationSlug = settings.slug;
    expect(locationSlug, 'Settings must include a slug').toBeTruthy();
    console.log('Location slug:', locationSlug);
  });

  // ── STEP 3: Admin branding page loads ─────────────────────────────────────────
  test('Step 3: UI — /admin/branding loads, form is visible, no JS errors', async ({ page }) => {
    const jsErrors: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));

    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    // The color form must be present
    const form = page.locator('form').first();
    await form.waitFor({ state: 'visible', timeout: 10000 });

    // ColorInput must have a text input for primary color
    const colorInputs = page.locator('input[type="text"]');
    const count = await colorInputs.count();
    expect(count, 'At least one color text input must be present').toBeGreaterThan(0);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
        && !e.includes('plausible')
    );
    expect(critical, `JS errors on branding page: ${critical.join('; ')}`).toEqual([]);
    console.log('Branding page loaded. Color inputs found:', count);
  });

  // ── STEP 4: API — update colors ───────────────────────────────────────────────
  test('Step 4: API — PUT /owner/brand with new primary + bg color', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { primaryColor: NEW_PRIMARY, bgColor: NEW_BG },
    });
    expect(r.status(), 'PUT /owner/brand must return 200').toBe(200);
    const body = await r.json();
    console.log('Brand PUT response:', body);
  });

  // ── STEP 5: API — verify colors persisted ────────────────────────────────────
  test('Step 5: API — GET /owner/brand confirms new colors saved', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();

    expect(body.primaryColor?.toLowerCase(), 'Primary color must match what was saved')
      .toBe(NEW_PRIMARY.toLowerCase());
    expect(body.bgColor?.toLowerCase(), 'Background color must match what was saved')
      .toBe(NEW_BG.toLowerCase());

    console.log('Confirmed colors — primary:', body.primaryColor, '| bg:', body.bgColor);
  });

  // ── STEP 6: UI — reload branding page, confirm color inputs updated ───────────
  test('Step 6: UI — reload /admin/branding shows updated colors in form', async ({ page }) => {
    const jsErrors: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));

    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    // Find color text inputs — at least one should contain the new primary color
    const textInputs = page.locator('input[type="text"]');
    const inputs = await textInputs.all();
    let foundPrimary = false;
    for (const input of inputs) {
      const val = await input.inputValue().catch(() => '');
      if (val.toLowerCase() === NEW_PRIMARY.toLowerCase()) { foundPrimary = true; break; }
    }
    expect(foundPrimary, `Primary color ${NEW_PRIMARY} must appear in a text input after reload`).toBe(true);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
        && !e.includes('plausible')
    );
    expect(critical, `JS errors after reload: ${critical.join('; ')}`).toEqual([]);
    console.log('UI shows updated primary color:', NEW_PRIMARY);
  });

  // ── STEP 7: Public — /branding-preview/{slug} loads without errors ────────────
  test('Step 7: Public — /branding-preview/{slug} loads without JS errors', async ({ page }) => {
    test.skip(!locationSlug, 'No slug available');
    const jsErrors: string[] = [];
    const cspViolations: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));
    page.on('console', msg => {
      const text = msg.text();
      if (text.includes('Content Security Policy') || text.includes('violated')) {
        cspViolations.push(text);
      }
    });

    await page.goto(`${BASE}/branding-preview/${locationSlug}`, {
      waitUntil: 'networkidle',
      timeout: 30000,
    });
    await page.waitForTimeout(3000);

    // Page must render content
    const bodyText = await page.textContent('body') || '';
    expect(bodyText.length, 'Branding-preview page must render content').toBeGreaterThan(100);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
        && !e.includes('plausible')
    );
    expect(critical, `JS errors on /branding-preview/${locationSlug}: ${critical.join('; ')}`).toEqual([]);

    if (cspViolations.length > 0) {
      console.warn('CSP violations detected:', cspViolations.join('; '));
    }
    expect(cspViolations, `CSP violations must not block page: ${cspViolations.join('; ')}`).toHaveLength(0);

    console.log(`/branding-preview/${locationSlug} loaded. Body length:`, bodyText.length);
  });

  // ── STEP 8: Public — /s/{slug} renders same menu as branding-preview ──────────
  test('Step 8: Public — /s/{slug} renders the client menu', async ({ page }) => {
    test.skip(!locationSlug, 'No slug available');
    const jsErrors: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, {
      waitUntil: 'networkidle',
      timeout: 30000,
    });
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body') || '';
    expect(bodyText.length, '/s/{slug} must render client content').toBeGreaterThan(100);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
        && !e.includes('plausible')
    );
    expect(critical, `JS errors on /s/${locationSlug}: ${critical.join('; ')}`).toEqual([]);

    console.log(`/s/${locationSlug} body length:`, bodyText.length);
  });

  // ── STEP 9: Public — /api/public/theme/{slug} returns updated primary color ────
  test('Step 9: API — public theme endpoint reflects updated primary color', async ({ request }) => {
    test.skip(!locationSlug, 'No slug available');
    const r = await request.get(`${BASE}/api/public/theme/${locationSlug}`);
    expect(r.status(), `GET /api/public/theme/${locationSlug} must return 200`).toBe(200);
    const body = await r.json();
    expect(body.primaryColor?.toLowerCase(), 'Public theme must return updated primary color')
      .toBe(NEW_PRIMARY.toLowerCase());
    console.log('Public theme primaryColor:', body.primaryColor);
  });

  // ── STEP 10: Restore original branding ───────────────────────────────────────
  test('Step 10: API — restore original branding', async ({ request }) => {
    test.skip(!baseline, 'No baseline to restore');
    const r = await request.put(`${BASE}/api/owner/brand`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        primaryColor: baseline.primaryColor ?? null,
        bgColor: baseline.bgColor ?? null,
        textColor: baseline.textColor ?? null,
        logoUrl: baseline.logoUrl ?? null,
      },
    });
    expect([200, 404]).toContain(r.status());
    console.log('Restored brand to baseline primary:', baseline.primaryColor);
  });
});
