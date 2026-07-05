import { test, expect } from '@playwright/test';

/**
 * L2 onboarding polish — the menu-parsing state. Proves that uploading a menu on
 * /start enters a crafted "reading your menu" state (scanning document + status
 * copy), not a bare spinner. The import request is held in-flight so the state
 * stays on screen for assertion; REST is otherwise untouched.
 *
 * Runs against the FE under test (VITE_BASE_URL); onboarding uses the default
 * app theme, so this is theme-independent.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('L2: menu parsing state', () => {
  test('uploading a menu shows the crafted parsing state', async ({ page }) => {
    // Hold the anonymous import so the parsing phase is observable.
    await page.route('**/owner/menu/import/anonymous', async (route) => {
      await new Promise((r) => setTimeout(r, 4000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ anonymous_import_id: 'x', restaurant: { name: 'Test' }, draft_preview: { products: [], categories: [] } }),
      });
    });

    await page.goto(`${BASE}/start`);

    await page.setInputFiles('[data-testid=menu-file-input]', {
      name: 'menu.png',
      mimeType: 'image/png',
      buffer: Buffer.from('iVBORw0KGgo=', 'base64'),
    });

    const parsing = page.getByTestId('menu-parsing');
    await expect(parsing).toBeVisible({ timeout: 5000 });
    // The scanning document + reassurance copy are present (not a dead spinner).
    await expect(parsing.locator('.dz-parse-doc')).toBeVisible();
    await expect(parsing.locator('.dz-parse-line').first()).toHaveText(/reading your menu/i);
    // The scan-line element exists (the "reading" affordance, not a dead spinner).
    await expect(parsing.locator('.dz-parse-scan')).toHaveCount(1);
  });
});
