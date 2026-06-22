import { test, expect } from '@playwright/test';
import fs from 'node:fs';

// UI/UX audit capture: screenshots key screens on the deployed app for FE/QA review.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
//   LOCAL_UI_PROOF=1 pnpm exec playwright test e2e/tests/capture-screens.spec.ts --project=desktop --reporter=line
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const DIR = '/tmp/screens';
test.skip(!process.env.CAPTURE_SCREENS, 'set CAPTURE_SCREENS=1 to capture');

test('capture screens', async ({ page, request }) => {
  fs.mkdirSync(DIR, { recursive: true });
  const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  const { access_token, activeLocationId, userId } = await auth.json();
  await request.post(`${BASE}/dev/seed-telegram-target`, { data: { locationId: activeLocationId, userId } });

  const shot = async (name: string, path: string, locale: string, auth = true) => {
    await page.addInitScript(([tk, lc, doAuth]: any) => {
      if (doAuth) localStorage.setItem('dos_access_token', tk);
      localStorage.setItem('dos_locale', lc);
    }, [access_token, locale, auth]);
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch(() => {});
    await page.waitForTimeout(1500);
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true });
  };

  // admin screens (default locale sq) + i18n spot-checks (en, uk) on settings
  await shot('admin-dashboard-sq', '/admin', 'sq');
  await shot('admin-settings-sq', '/admin/settings', 'sq');
  await shot('admin-settings-en', '/admin/settings', 'en');
  await shot('admin-settings-uk', '/admin/settings', 'uk');
  await shot('admin-menu-sq', '/admin/menu', 'sq');
  // public client storefront
  await shot('client-menu-sq', '/s/sushi-durres', 'sq', false);
  await shot('client-menu-en', '/s/sushi-durres', 'en', false);

  const files = fs.readdirSync(DIR);
  expect(files.length).toBeGreaterThan(5);
  console.log('CAPTURED:', files.join(', '));
});
