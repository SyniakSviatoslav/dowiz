import { test, expect } from '@playwright/test';

// Menu-first onboarding (Thread A): the public /start front door parses a menu
// anonymously, pre-fills the storefront identity, and claims it with Telegram —
// then POST /owner/onboarding/start seeds the new location with the parsed menu.
//
// This spec mocks the API at the network boundary so it proves the FRONTEND flow
// deterministically (no backend/DB needed): upload → prefilled review → claim →
// onboarding/start fired WITH the anonymous_import_id. The seeding itself is
// proven separately against a real Postgres schema (seedMenuFromDraft).

const IMPORT_ID = '11111111-1111-1111-1111-111111111111';

test('menu-first: upload → prefilled review → Telegram claim seeds via onboarding/start', async ({ page, context }) => {
  let onboardingBody: any = null;

  // Catch-all FIRST so the specific mocks below take precedence (Playwright
  // matches routes in reverse registration order). Every endpoint THIS flow
  // exercises is mocked explicitly below; a GET that lands here is at worst a
  // harmless SPA boot/config probe (empty body is fine), but any unmocked
  // MUTATION is an unintended request and must fail loudly rather than be
  // silently swallowed by a blanket 200 (Test Integrity §2/§3).
  await page.route('**/api/**', async (route) => {
    const req = route.request();
    if (req.method() !== 'GET') {
      await route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'unexpected request', method: req.method(), url: req.url() }) });
      return;
    }
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });

  await page.route('**/api/owner/menu/import/anonymous', async (route) => {
    await route.fulfill({
      status: 200, contentType: 'application/json',
      body: JSON.stringify({
        anonymous_import_id: IMPORT_ID,
        expires_at: new Date(Date.now() + 1800000).toISOString(),
        summary: { valid: 3 }, issues: [],
        draft_preview: {
          categories: [{ externalKey: 'c1', name: 'Pizza' }, { externalKey: 'c2', name: 'Drinks' }],
          products: [
            { externalKey: 'p1', categoryKey: 'c1', name: 'Margherita', price: 500, available: true },
            { externalKey: 'p2', categoryKey: 'c1', name: 'Pepperoni', price: 650, available: true },
            { externalKey: 'p3', categoryKey: 'c2', name: 'Cola', price: 150, available: true },
          ],
        },
        restaurant: { name: 'Pizza Roma', phone: '+355691234567', address: 'Rruga Myslym Shyri' },
      }),
    });
  });
  await page.route('**/api/auth/telegram/start', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ token: 'tok-1', deepLink: 'https://t.me/dowiz_bot?start=login_tok-1', botUsername: 'dowiz_bot' }) });
  });
  await page.route('**/api/auth/telegram/poll**', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ status: 'authenticated', access_token: 'jwt-abc', refresh_token: 'refr-xyz' }) });
  });
  await page.route('**/api/owner/onboarding/start', async (route) => {
    onboardingBody = route.request().postDataJSON();
    await route.fulfill({ status: 201, contentType: 'application/json', body: JSON.stringify({ locationId: 'loc-1', slug: onboardingBody?.slug, onboardingState: {}, currentStep: 1, seeded: { categories: 2, products: 3 } }) });
  });
  // The Telegram deep link opens in a popup — stub t.me so it doesn't hit the network.
  await context.route(/t\.me/, async (route) => { await route.fulfill({ status: 200, body: '' }); });

  // 1) Public front door shows the upload CTA
  await page.goto('/start');
  await expect(page.getByTestId('upload-menu-cta')).toBeVisible();

  // 2) Upload a menu file → triggers anonymous parse
  await page.setInputFiles('[data-testid=menu-file-input]', { name: 'menu.pdf', mimeType: 'application/pdf', buffer: Buffer.from('%PDF-1.4 fake menu') });

  // 3) Review screen: identity pre-filled from the parsed menu + item preview
  await expect(page.getByTestId('menu-preview')).toBeVisible();
  // Assert the exact item SET surfaced (3 named products), not a bare substring
  // '3' which any digit-bearing token would satisfy. Locale-independent: product
  // names render verbatim regardless of the active i18n catalog.
  await expect(page.getByTestId('menu-preview')).toContainText('Margherita · Pepperoni · Cola');
  await expect(page.locator('input[placeholder="e.g. Pizza Roma"]')).toHaveValue('Pizza Roma');
  await expect(page.locator('input[placeholder^="+355"]')).toHaveValue('+355691234567');

  // 4) Claim with Telegram → poll authenticates → onboarding/start fires WITH the import id
  await page.getByTestId('claim-cta').click();
  await expect.poll(() => onboardingBody?.anonymous_import_id, { timeout: 20000 }).toBe(IMPORT_ID);
  expect(onboardingBody.slug).toBe('pizza-roma');
  expect(onboardingBody.name).toBe('Pizza Roma');
  // The pre-filled phone must reach the backend payload (not just sit in the
  // input) — proves the parsed identity is carried through to onboarding/start.
  // (No address field exists in this form, so address is intentionally absent.)
  expect(onboardingBody.phone).toBe('+355691234567');

  // The poll firing onboarding/start is necessary but not sufficient: assert the
  // FE actually HANDLED the 201 by completing the flow — it redirects to the
  // activation screen on success. A swallowed error keeps us on /start.
  await expect(page).toHaveURL(/\/admin\/activation/);
});
