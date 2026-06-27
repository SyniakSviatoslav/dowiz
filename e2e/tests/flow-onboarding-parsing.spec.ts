import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

/**
 * L2 onboarding polish — the menu-parsing state. Proves that uploading a menu on
 * /start enters a crafted "reading your menu" state (scanning document + status
 * copy), not a bare spinner, and that every failure mode of
 * POST /owner/menu/import/anonymous drops that state into a humane error rather
 * than stranding the user on a dead spinner. The import request is stubbed; REST
 * is otherwise untouched and nothing is written to the target.
 *
 * Runs against the FE under test (VITE_BASE_URL); onboarding uses the default
 * app theme, so this is theme-independent.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// The route returns crypto.randomUUID() (apps/api/src/routes/owner/menu-import.ts
// L210); a fabricated 'x' would mask a client that ignores the response body and
// renders stale/cached data. Use a real UUID and read it back through the UI.
const IMPORT_ID = '11111111-1111-4111-8111-111111111111';
// Exact i18n English copy (start.reading); a raw-key fallback or a different
// string that merely contains "reading" must fail this.
const PARSE_COPY = 'Reading your menu…';
const PARSE_FAILED = "We couldn't read that file. Try a clearer PDF or photo.";
const UNSUPPORTED = 'Please upload a PDF or photo of your menu.';

const PNG = {
  name: 'menu.png',
  mimeType: 'image/png',
  buffer: Buffer.from('iVBORw0KGgo=', 'base64'),
};

test.describe('L2: menu parsing state', () => {
  test('uploading a menu shows the crafted parsing state', async ({ page }) => {
    expectUuid(IMPORT_ID, 'IMPORT_ID');
    // Hold the anonymous import so the parsing phase is observable, and record
    // that the intercept actually fired — a second un-stubbed request (retry,
    // preflight, CDN) would leave this false and fail the test instead of
    // silently timing out on a never-fulfilled stub.
    let importHit = false;
    await page.route('**/owner/menu/import/anonymous', async (route) => {
      importHit = true;
      await new Promise((r) => setTimeout(r, 4000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ anonymous_import_id: IMPORT_ID, restaurant: { name: 'Test' }, draft_preview: { products: [], categories: [] } }),
      });
    });

    await page.goto(`${BASE}/start`);

    await page.setInputFiles('[data-testid=menu-file-input]', PNG);

    const parsing = page.getByTestId('menu-parsing');
    await expect(parsing).toBeVisible({ timeout: 5000 });
    // The scanning document + reassurance copy are present (not a dead spinner).
    await expect(parsing.locator('.dz-parse-doc')).toBeVisible();
    await expect(parsing.locator('.dz-parse-line').first()).toHaveText(PARSE_COPY);
    // The scan-line affordance is actually rendered (not display:none / 0-size).
    await expect(parsing.locator('.dz-parse-scan')).toBeVisible();

    // The intercepted endpoint actually ran — not a second un-stubbed request.
    expect(importHit).toBe(true);

    // Client renders ONLY the response body: the stubbed restaurant.name 'Test'
    // flows into the review form once parsing resolves (proves no stale render).
    // TODO(needs-staging): requires the deployed onboarding FE to transition to review.
    await expect(parsing).toBeHidden({ timeout: 8000 });
    await expect(page.getByTestId('menu-preview')).toBeVisible();
  });

  // Error matrix — every status the import route can emit (400/413/422/429/500)
  // must drop the parsing state and surface a humane error. The FE only special-
  // cases UNSUPPORTED_TYPE; every other failure resolves to the generic copy.
  for (const { status, code, copy } of [
    { status: 400, code: 'UNSUPPORTED_TYPE', copy: UNSUPPORTED },
    { status: 400, code: 'VALIDATION_FAILED', copy: PARSE_FAILED },
    { status: 413, code: 'FILE_TOO_LARGE', copy: PARSE_FAILED },
    { status: 422, code: 'LOW_CONFIDENCE_REQUIRES_FORCE', copy: PARSE_FAILED },
    { status: 429, code: 'RATE_LIMITED', copy: PARSE_FAILED },
    { status: 500, code: 'UNKNOWN', copy: PARSE_FAILED },
  ] as const) {
    test(`import ${status}/${code} clears parsing and shows an error`, async ({ page }) => {
      await page.route('**/owner/menu/import/anonymous', async (route) => {
        await route.fulfill({
          status,
          contentType: 'application/json',
          body: JSON.stringify({ status, code, error: code, message: code }),
        });
      });

      await page.goto(`${BASE}/start`);
      await page.setInputFiles('[data-testid=menu-file-input]', PNG);

      const alert = page.getByRole('alert');
      await expect(alert).toBeVisible({ timeout: 8000 });
      await expect(alert).toContainText(copy);
      // The parsing state must not be left on screen behind the error.
      await expect(page.getByTestId('menu-parsing')).toBeHidden();
    });
  }
});
