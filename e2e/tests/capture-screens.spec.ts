import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// UI/UX audit capture: screenshots key screens on the deployed app for FE/QA review.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
//   LOCAL_UI_PROOF=1 pnpm exec playwright test e2e/tests/capture-screens.spec.ts --project=desktop --reporter=line
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const DIR = '/tmp/screens';
test.skip(!process.env.CAPTURE_SCREENS, 'set CAPTURE_SCREENS=1 to capture');

// Mutating spec (seeds a telegram target + upserts the dev owner) — fail fast against prod/unknown.
test.beforeAll(() => requireStaging(BASE));

test('capture screens', async ({ context, request }) => {
  fs.mkdirSync(DIR, { recursive: true });
  const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  expect(auth.status(), 'mock-auth must return 200').toBe(200);
  const { access_token, activeLocationId, userId } = await auth.json();
  expectJwt(access_token, 'access_token');
  expectUuid(activeLocationId, 'activeLocationId');
  expectUuid(userId, 'userId');

  const seed = await request.post(`${BASE}/dev/seed-telegram-target`, {
    data: { locationId: activeLocationId, userId },
  });
  expect(seed.status(), 'seed-telegram-target must return 200').toBe(200);

  const shot = async (name: string, path: string, locale: string, auth = true) => {
    // Fresh page per shot so addInitScript injections do not accumulate and re-fire
    // on every subsequent navigation (one stray token/locale would poison later screens).
    const page = await context.newPage();
    try {
      await page.addInitScript(([tk, lc, doAuth]: any) => {
        if (doAuth) localStorage.setItem('dos_access_token', tk);
        localStorage.setItem('dos_locale', lc);
      }, [access_token, locale, auth]);
      const resp = await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded' });
      expect(resp?.status(), `${path} must return 200`).toBe(200);
      // Tolerated: networkidle can stall on pages with a persistent WS; the shell is already loaded.
      await page.waitForLoadState('networkidle').catch((e) => {
        void e;
      });
      // Liveness proof: the SPA actually mounted (a blank/crashed #root has no box → not visible).
      await expect(page.locator('#root')).toBeVisible();
      const file = `${DIR}/${name}.png`;
      await page.screenshot({ path: file, fullPage: true });
      expect(fs.statSync(file).size, `${name}.png must be a non-empty file`).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
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
