import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Sunlight Mode — high-contrast outdoor theme. Proves it flips dark surfaces to a light AAA
// palette and that the header toggle works. Runs against staging.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const OWNER_EMAIL = process.env.TEST_OWNER_EMAIL || 'test@dowiz.com';
const OWNER_PASSWORD = process.env.TEST_OWNER_PASSWORD || 'test123456';

function luminance(rgb: string): number {
  const m = (rgb.match(/\d+/g) || ['0', '0', '0']).map(Number);
  return 0.299 * m[0]! + 0.587 * m[1]! + 0.114 * m[2]!;
}

// This suite hits the dev/mock-auth backdoor — refuse to run it against prod/unknown targets.
test.beforeAll(() => requireStaging(BASE));

test.describe('Sunlight Mode', () => {
  test('persisted pref flips every surface to a light high-contrast theme', async ({ page, request }) => {
    const loginRes = await request.post(`${BASE}/api/auth/local/login`, { data: { email: OWNER_EMAIL, password: OWNER_PASSWORD } });
    expect(loginRes.status(), 'owner login must succeed').toBe(200);
    const owner = (await loginRes.json()).access_token;
    expectJwt(owner, 'owner access_token');

    const mockRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(mockRes.status(), 'courier mock-auth must succeed').toBe(200);
    const { access_token: courier, activeLocationId: courierLocationId } = await mockRes.json();
    expectJwt(courier, 'courier access_token');

    for (const [route, token] of [['/courier', courier], ['/admin', owner], ['/s/demo', null]] as const) {
      // Reset state between iterations so init scripts / tokens don't stack across routes.
      await page.context().clearCookies();
      await page.goto(`${BASE}${route}`, { waitUntil: 'domcontentloaded' });
      await page.evaluate((t) => {
        try {
          localStorage.clear();
          localStorage.setItem('dowiz-sunlight', 'on');
          if (t) localStorage.setItem('dos_access_token', t);
        } catch { /* storage may be unavailable pre-origin; reload re-applies */ }
      }, token);
      await page.goto(`${BASE}${route}`, { waitUntil: 'networkidle' });
      // Deterministic signal — wait for the theme attribute, not a fixed sleep.
      await page.waitForFunction(() => document.documentElement.getAttribute('data-sunlight') === 'on');
      await expect(page.locator('html')).toHaveAttribute('data-sunlight', 'on');
      const { bg, text } = await page.evaluate(() => ({ bg: getComputedStyle(document.body).backgroundColor, text: getComputedStyle(document.body).color }));
      expect(luminance(bg), `${route} background should be light in sunlight mode`).toBeGreaterThan(220);
      expect(luminance(text), `${route} text should be dark in sunlight mode`).toBeLessThan(60);
    }

    // Cross-role isolation: the courier token must NOT reach an owner-only surface, even while
    // sunlight mode is active. requireRole(['owner']) returns 403 'Forbidden role' (apps/api/src/
    // plugins/auth.ts:111). Exercises the real auth path — proves the toggle is theme-only.
    // TODO(needs-staging): requires a live staging API + seeded courier location to return 403.
    const isoRes = await request.get(`${BASE}/api/owner/${courierLocationId}/dashboard/snapshot`, {
      headers: { authorization: `Bearer ${courier}` },
    });
    expect(isoRes.status(), 'courier token must be forbidden on owner dashboard').toBe(403);
  });

  test('header toggle turns Sunlight Mode on from a clean state', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await page.evaluate(() => { try { localStorage.removeItem('dowiz-sunlight'); } catch { /* noop */ } });
    await page.reload({ waitUntil: 'networkidle' });
    // Clean state: sunlight must be OFF before the toggle (deterministic signal, not a sleep).
    await page.waitForFunction(() => document.documentElement.getAttribute('data-sunlight') !== 'on');

    const toggle = page.getByTestId('sunlight-toggle').first();
    await expect(toggle).toBeVisible({ timeout: 10000 });

    // Baseline — capture the default-theme background BEFORE the toggle. This guards against the
    // luminance threshold passing on an unstyled browser-default white page: we assert the toggle
    // ACTUALLY changes the background, not just that it ends up light.
    const bgBefore = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);

    await toggle.click();
    await page.waitForFunction(() => document.documentElement.getAttribute('data-sunlight') === 'on');
    await expect(page.locator('html')).toHaveAttribute('data-sunlight', 'on');
    const bgAfter = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(bgAfter, 'sunlight toggle must change the background').not.toBe(bgBefore);
    expect(luminance(bgAfter)).toBeGreaterThan(220);
  });
});
