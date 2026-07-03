/**
 * E2E proof for the LC9/S3 "fake data as real" fixes (audit-frontend-2026-07-03.md
 * CRITICAL #1, HIGH #9/#10/#20; AUDIT-SYNTHESIS-2026-07-03.md LC9) plus the adjacent
 * S4 (silent mutation failure) and S5 (no-location explicit state) fixes in the same
 * lane: CRMPage, AnalyticsPage, SettingsPage, courier DeliveryPage.
 *
 * AUTHORED-ONLY at commit time (per Ship Discipline / Mandatory Proof Rule): this spec
 * is written against the staging-guard conventions used elsewhere in this suite but has
 * NOT been executed against any deployed environment yet — that happens at converge,
 * after commit + staging deploy, via:
 *   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test e2e/tests/audit-fix-data-integrity.spec.ts --reporter=list
 *
 * Strategy: intercept the relevant API calls with page.route() to deterministically
 * force the failure/empty condition (500 / 404) rather than depending on real backend
 * fixture state — the fabricated-fallback bug ONLY reproduces on a failed/empty response.
 */
import { test, expect } from '@playwright/test';
import { isProdTarget } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const isProd = isProdTarget(BASE);

test.describe('Audit fix: fake-data-as-real (LC9/S3) + S4/S5 — admin + courier', () => {
  let authToken: string;

  test.beforeAll(async ({ request }) => {
    // /api/dev/mock-auth is dev-only and closed on prod — every test below skips there.
    test.skip(isProd, 'dev/mock-auth is closed on prod — staging only');
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
  });

  test.beforeEach(() => {
    test.skip(isProd, 'mutating / auth-gated — staging only');
  });

  test('CRMPage: a failed customer-analytics fetch shows an error state, never the old fabricated order/LTV/heatmap', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);

    // Force the analytics-detail fetch to fail.
    await page.route('**/api/owner/customers/*/analytics', route => route.fulfill({ status: 500, body: '{}' }));

    await page.goto(`${BASE}/admin/crm`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const firstRow = page.locator('[class*="cursor-pointer"]').first();
    if (await firstRow.isVisible().catch(() => false)) {
      await firstRow.click();

      // The error state (EmptyState + Retry) must render …
      await expect(page.getByText(/couldn.?t load history|nuk u ngarkua/i)).toBeVisible({ timeout: 10000 });
      await expect(page.getByRole('button', { name: /retry|provo/i })).toBeVisible();

      // … and the OLD fabricated literals must never appear anywhere on the page.
      const body = await page.textContent('body');
      expect(body).not.toContain('Rruga e Durres');
      expect(body).not.toContain('750,000');
      expect(body).not.toContain('750000');
    }
  });

  test('AnalyticsPage: ingredient consumption is an honest "not available" state with no export button', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // The old fake sushi dataset must be gone everywhere on the page.
    const body = await page.textContent('body');
    expect(body).not.toMatch(/Salmon fillet|Sushi rice|Nori sheets/);

    // The consumption panel itself must show the "not available" copy …
    const consumptionPanel = page
      .locator('div')
      .filter({ hasText: /ingredient consumption/i })
      .last();
    await expect(consumptionPanel.getByText(/not available yet|ende e padisponueshme/i)).toBeVisible({ timeout: 10000 });

    // … and must NOT offer a CSV/JSON export for it (the export of fabricated data was
    // the worst part of the original bug — exporting must be impossible in this state).
    await expect(consumptionPanel.getByRole('button', { name: /export csv|export json/i })).toHaveCount(0);
  });

  test('SettingsPage: a 404/empty load renders a BLANK setup form, never a savable fake identity', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.route('**/api/owner/settings', route => {
      if (route.request().method() === 'GET') return route.fulfill({ status: 404, body: '{}' });
      return route.continue();
    });

    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await expect(page.locator('#settings-locationName')).toBeVisible({ timeout: 15000 });

    // Must be blank — NOT the old "Downtown Tirana" / "+35542345678" mock identity.
    await expect(page.locator('#settings-locationName')).toHaveValue('');
    await expect(page.locator('#settings-phone')).toHaveValue('');
    await expect(page.locator('#settings-address')).toHaveValue('');
  });

  test('SettingsPage: a save that 404s shows a real error, never the fake "Settings saved" success', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.route('**/api/owner/settings', route => {
      if (route.request().method() === 'PUT') return route.fulfill({ status: 404, body: '{}' });
      return route.continue();
    });

    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await page.locator('#settings-locationName').fill('Test Venue');
    await page.locator('#settings-phone').fill('+355691234567');
    await page.locator('#settings-address').fill('Test Address 1');
    await page.locator('button[type="submit"]').first().click();

    // Must show a real failure — catalog: admin.settings_save_error = "Failed to
    // save settings" / "Ruajtja e cilësimeve dështoi".
    await expect(page.getByText(/failed to save settings|dështoi/i)).toBeVisible({ timeout: 10000 });
    // Must NOT show the success toast — catalog: common.saved = "Saved!" / "U ruajt!"
    // (note: exact string incl. "!" — "nuk u ruajt" (failed) also contains "u ruajt").
    await expect(page.getByText(/^saved!$|^u ruajt!$/i)).not.toBeVisible();
  });

  test('Courier DeliveryPage: missing customer coords → explicit no-location banner, never a Tirana/Durrës pin', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.route('**/api/courier/assignments/*', route => {
      if (route.request().method() !== 'GET') return route.continue();
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'e2e-test-task',
          orderId: 'e2e-test-order',
          status: 'accepted',
          restaurant: { name: 'Test Kitchen', address: 'Somewhere' }, // no lat/lng
          customer: { address: 'Unknown street, no pin' }, // no lat/lng — the exact bug shape
          total: 100000,
          eta: '15 min',
        }),
      });
    });

    await page.goto(`${BASE}/courier/delivery/e2e-test-task`, { waitUntil: 'networkidle' });
    await expect(page.getByTestId('courier-no-location')).toBeVisible({ timeout: 15000 });
  });
});
