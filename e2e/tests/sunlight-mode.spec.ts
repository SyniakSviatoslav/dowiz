import { test, expect } from '@playwright/test';

// Sunlight Mode — high-contrast outdoor theme. Proves it flips dark surfaces to a light AAA
// palette and that the header toggle works. Runs against staging.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

function luminance(rgb: string): number {
  const m = (rgb.match(/\d+/g) || ['0', '0', '0']).map(Number);
  return 0.299 * m[0]! + 0.587 * m[1]! + 0.114 * m[2]!;
}

test.describe('Sunlight Mode', () => {
  test('persisted pref flips every surface to a light high-contrast theme', async ({ page, request }) => {
    const owner = (await (await request.post(`${BASE}/api/auth/local/login`, { data: { email: 'test@dowiz.com', password: 'test123456' } })).json()).access_token;
    const courier = (await (await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } })).json()).access_token;

    for (const [route, token] of [['/courier', courier], ['/admin', owner], ['/s/demo', null]] as const) {
      await page.addInitScript((t) => { try { localStorage.setItem('dowiz-sunlight', 'on'); if (t) localStorage.setItem('dos_access_token', t); } catch {} }, token);
      await page.goto(`${BASE}${route}`, { waitUntil: 'networkidle' });
      await page.waitForTimeout(800);
      await expect(page.locator('html')).toHaveAttribute('data-sunlight', 'on');
      const { bg, text } = await page.evaluate(() => ({ bg: getComputedStyle(document.body).backgroundColor, text: getComputedStyle(document.body).color }));
      expect(luminance(bg), `${route} background should be light in sunlight mode`).toBeGreaterThan(220);
      expect(luminance(text), `${route} text should be dark in sunlight mode`).toBeLessThan(60);
    }
  });

  test('header toggle turns Sunlight Mode on from a clean state', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await page.evaluate(() => { try { localStorage.removeItem('dowiz-sunlight'); } catch {} });
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForTimeout(800);

    const toggle = page.getByTestId('sunlight-toggle').first();
    await expect(toggle).toBeVisible({ timeout: 10000 });
    await toggle.click();
    await expect(page.locator('html')).toHaveAttribute('data-sunlight', 'on');
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(luminance(bg)).toBeGreaterThan(220);
  });
});
