import { test, expect, type Page } from '@playwright/test';

// Test Integrity (AGENTS.md): every page-load must assert a SPECIFIC visible element +
// that the URL stayed on-route + that the login form is absent — so a /login redirect, a
// 404/catch-all redirect, an error boundary, or a blank spinner all turn the test RED.
// body.length / loose body-text regex (which pass on any of those) are banned.
//
// NEEDS-STAGING (do NOT fake here — require a real authed non-dev session + a real 2nd tenant):
//   TODO(needs_staging): error-matrix. The ?dev=true path monkey-patches window.fetch
//     (apps/web/src/api/devBootstrap.ts), so Playwright route.fulfill cannot inject 500/403/401
//     into mocked calls. A real error-matrix test must run WITHOUT ?dev=true against staging with a
//     real owner token, route.fulfill(500) on /owner/analytics & /owner/customers, and assert the
//     EmptyState error copy (AnalyticsPage:155 / CRMPage:198, data-testid="empty-state") renders.
//   TODO(needs_staging): cross-tenant / IDOR. Sign in as tenant A on staging and assert tenant B's
//     real records (by real id/name) are absent from CRM/orders/analytics — needs a real 2nd tenant.

const ADMIN_ROUTES = ['/admin', '/admin/menu', '/admin/branding', '/admin/couriers', '/admin/analytics', '/admin/crm', '/admin/settings', '/admin/onboarding'];

/**
 * Load an admin route via the dev bypass and prove the REAL page mounted.
 * Registers a pageerror collector and returns it for an end-of-test assertion.
 */
async function loadAdminPage(page: Page, route: string): Promise<string[]> {
  const errors: string[] = [];
  page.on('pageerror', err => errors.push(err.message));
  await page.goto(`${route}?dev=true`);
  await page.waitForLoadState('networkidle');
  await expect(page, `${route} must stay on-route (no auth/404 redirect)`).toHaveURL(new RegExp(`${route.replace(/\//g, '\\/')}(\\?|$)`));
  await expect(page.locator('#login-email'), `${route} must render the real page, not the login form`).toHaveCount(0);
  return errors;
}

test.describe('Admin Pages — Full Coverage (dev bypass)', () => {

  test('couriers page renders the couriers view', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/couriers');
    await expect(page.getByRole('heading', { level: 2 }).first(), 'couriers heading must be visible').toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    expect(await page.context().cookies(), 'dev bypass must set no cookies').toEqual([]);
  });

  test('analytics page renders KPI cards', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/analytics');
    await expect(page.getByTestId('kpi-card').first(), 'analytics KPI card must be visible').toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('crm page renders the customers view', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/crm');
    await expect(page.getByRole('heading', { level: 2 }).first(), 'crm heading must be visible').toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('settings page renders the settings form', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/settings');
    await expect(page.getByRole('heading', { level: 2 }).first(), 'settings heading must be visible').toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('onboarding page renders the wizard', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/onboarding');
    await expect(page.locator('input').first(), 'onboarding wizard must render at least one input').toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('onboarding step navigation fills form and advances', async ({ page }) => {
    const errors = await loadAdminPage(page, '/admin/onboarding');

    const inputs = page.locator('input');
    const inputCount = await inputs.count();
    expect(inputCount).toBeGreaterThanOrEqual(1);

    if (inputCount >= 3) {
      await inputs.nth(0).fill('Pizza Roma');
      await inputs.nth(1).fill('+355691234567');
    }

    const nextBtn = page.locator('button:has-text("Next")');
    const nextVisible = await nextBtn.count() > 0 && await nextBtn.isEnabled();
    expect(nextVisible).toBe(true);
    if (nextVisible) {
      await nextBtn.first().click();
      // The wizard must still be mounted after advancing — not crashed to a blank/error view.
      await expect(page.locator('input').first(), 'wizard must remain mounted after Next').toBeVisible();
      await expect(page, 'must stay within the onboarding wizard after Next').toHaveURL(/\/admin\/onboarding/);
    }
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('all admin pages set no cookies', async ({ page }) => {
    for (const p of ADMIN_ROUTES) {
      await page.goto(`${p}?dev=true`);
      const cookies = await page.context().cookies();
      expect(cookies, `${p} must set no cookies under dev bypass`).toEqual([]);
    }
  });

});

test.describe('Admin Pages — Real Auth Gate (no dev bypass)', () => {

  // The dev bypass (?dev=true) is exempt from the auth guard (AdminRoutes.tsx:62-65). Without it,
  // an unauthenticated visitor MUST be redirected to /login. This block proves the real gate works
  // — the "no cookies" assertion above only confirmed the bypass path, never the gate itself.
  for (const route of ADMIN_ROUTES) {
    test(`${route} without dev bypass redirects an unauthenticated user to /login`, async ({ page }) => {
      await page.goto(route);
      await page.waitForLoadState('networkidle');
      await expect(page, `${route} must redirect to /login when unauthenticated`).toHaveURL(/\/login(\?|\/|$)/);
      await expect(page.locator('#login-email'), 'login form must be visible after the auth redirect').toBeVisible();
    });
  }

});
