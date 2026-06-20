import { test, expect } from '@playwright/test';

// Proof for the 2026-06-19 onboarding QA fixes (docs/audit/2026-06-19/onboarding-qa-report.md):
//   O1 — a fresh owner (valid token, no location yet) must land on the onboarding
//        wizard, NOT be bounced to /login with "session expired".
//   O2 — the stepper's courier label is "Couriers", not the leaked "Courier:".
//   O4 — the final step reads as a customer-facing "Test order", not "Order Flow Test".
//   O3 — the menu step subhead no longer promises a "PDF" import.
//
// Requires a deployment carrying these changes + DEV_AUTH_SECRET (injected by
// playwright.config.ts). Run: pnpm exec playwright test onboarding-copy-qa --reporter=list

async function freshOwnerToken(request: any): Promise<string | null> {
  const res = await request.post('/api/dev/mock-auth', { data: { fresh: true } });
  if (!res.ok()) return null; // 404 ⇒ no DEV_AUTH_SECRET / not a dev-enabled target
  const body = await res.json();
  return body.access_token ?? null;
}

test.describe('Onboarding QA fixes', () => {
  test('O1: a fresh owner lands on the wizard, not bounced to login', async ({ page, request }) => {
    const token = await freshOwnerToken(request);
    test.skip(!token, 'mock-auth fresh owner unavailable (no DEV_AUTH_SECRET on target)');

    await page.addInitScript((t) => {
      localStorage.setItem('dos_access_token', t);
      localStorage.setItem('dos_locale', 'en'); // assert against English copy deterministically
    }, token);
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');

    // Must NOT be ejected to the login screen with a session-expired banner.
    expect(page.url()).not.toContain('/login');
    await expect(page.getByText(/session has expired/i)).toHaveCount(0);
    // Must be on the onboarding wizard.
    await expect(page).toHaveURL(/\/admin\/onboarding/);
  });

  test('O2 + O4: stepper labels are "Couriers" and "Test order", never "Courier:"', async ({ page, request }) => {
    const token = await freshOwnerToken(request);
    test.skip(!token, 'mock-auth fresh owner unavailable (no DEV_AUTH_SECRET on target)');

    await page.addInitScript((t) => {
      localStorage.setItem('dos_access_token', t);
      localStorage.setItem('dos_locale', 'en'); // assert against English copy deterministically
    }, token);
    await page.goto('/admin/onboarding');
    await page.waitForLoadState('networkidle');

    // Scope to the wizard body — "Couriers" also appears in the admin bottom-nav.
    const main = page.getByRole('main');
    await expect(main.getByText('Couriers', { exact: true })).toBeVisible();       // O2
    await expect(main.getByText('Test order', { exact: true })).toBeVisible();      // O4
    await expect(page.getByText('Courier:', { exact: true })).toHaveCount(0);       // O2 (no leaked colon)
  });

  test('O3: menu step subhead does not promise a PDF import', async ({ page, request }) => {
    const token = await freshOwnerToken(request);
    test.skip(!token, 'mock-auth fresh owner unavailable (no DEV_AUTH_SECRET on target)');

    await page.addInitScript((t) => {
      localStorage.setItem('dos_access_token', t);
      localStorage.setItem('dos_locale', 'en'); // assert against English copy deterministically
    }, token);
    await page.goto('/admin/onboarding');
    await page.waitForLoadState('networkidle');

    // Advance from step 0 (Restaurant) to step 1 (Menu).
    await page.getByPlaceholder('e.g. Pizza Roma').fill('QA Diner');
    await page.getByPlaceholder('+355 69 XXX XXXX').first().fill('+355691234567');
    await page.getByPlaceholder('sushi-durres').fill('qa-diner-copy');
    await page.getByRole('button', { name: /Next/ }).click();

    await expect(page.getByText('Your Menu')).toBeVisible();
    await expect(page.getByText(/PDF/)).toHaveCount(0);
  });
});
