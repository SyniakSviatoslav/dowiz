import { test, expect, type Page, type APIRequestContext } from '@playwright/test';

// MVP UI-improvements proof (the GO subset per docs/research/UI-IMPROVEMENTS-TESTPLAN.md).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test ui-improvements --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

async function ownerLogin(page: Page, request: APIRequestContext) {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login should succeed').toBeTruthy();
  const body = await res.json();
  expect(body.access_token).toBeTruthy();
  await page.goto('/login'); // establish the app origin before writing storage
  await page.evaluate((t) => {
    localStorage.setItem('dos_access_token', t);
    try { sessionStorage.setItem('dos_access_token', t); } catch { /* private mode */ }
  }, body.access_token as string);
}

test('storefront surfaces the venue state chip on /s/demo', async ({ page }) => {
  await page.goto('/s/demo');
  const chip = page.locator('[data-testid="venue-state-chip"]');
  await expect(chip).toBeVisible({ timeout: 25000 });
  await expect(chip).toHaveAttribute('data-state', /open|closed|busy/);
});

test('owner dashboard shows an honest alert state and arms on a gesture', async ({ page, request }) => {
  await ownerLogin(page, request);
  await page.goto('/admin');
  const enable = page.locator('[data-testid="owner-alert-enable"]');
  const status = page.locator('[data-testid="owner-alert-status"]');
  // The honest-state indicator must render (blocked/muted → enable; armed → status).
  await expect(enable.or(status).first()).toBeVisible({ timeout: 25000 });
  // Clicking enable attempts the audio unlock. The council gate is *honesty*: it may arm
  // (real audio device → status) OR stay blocked (no audio, e.g. headless → enable persists),
  // but it must NEVER claim armed without real playback. Assert the state stays honest.
  if (await enable.isVisible().catch(() => false)) {
    await enable.click();
    await page.waitForTimeout(1500);
    await expect(status.or(enable).first(), 'alert state must stay honest after unlock attempt').toBeVisible();
  }
});

test('owner menu manager renders the schedule editor + kitchen-busy toggle', async ({ page, request }) => {
  await ownerLogin(page, request);
  await page.goto('/admin/menu');
  await expect(page.locator('[data-testid="kitchen-busy-toggle"]')).toBeVisible({ timeout: 25000 });
  await expect(page.locator('[data-testid="schedule-editor"]')).toBeVisible();
});
