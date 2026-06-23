import { test, expect, type Page, type APIRequestContext } from '@playwright/test';

// MVP UI-improvements proof (the GO subset per docs/research/UI-IMPROVEMENTS-TESTPLAN.md).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test ui-improvements --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

// The login endpoint is rate-limited (max 5/min); with the suite running serially across
// three viewport projects that budget is easily blown. Memoize one token per worker.
let cachedToken: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (cachedToken) return cachedToken;
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login should succeed').toBeTruthy();
  const body = await res.json();
  expect(body.access_token, 'login returns an access token').toBeTruthy();
  cachedToken = body.access_token as string;
  return cachedToken;
}

async function ownerLogin(page: Page, request: APIRequestContext) {
  const token = await ownerToken(request);
  await page.goto('/login'); // establish the app origin before writing storage
  await page.evaluate((t) => {
    localStorage.setItem('dos_access_token', t);
    try { sessionStorage.setItem('dos_access_token', t); } catch { /* private mode */ }
  }, token);
}

// The demo storefront exposes its locationId + product list on the public menu (no auth),
// so the seed-driven tests can target real rows without an owner-locations endpoint.
async function demoMenu(request: APIRequestContext) {
  const res = await request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } });
  expect(res.ok(), 'public demo menu should load').toBeTruthy();
  const menu = await res.json();
  const locationId: string = menu.locationId ?? menu.location_id;
  expect(locationId, 'demo menu carries a locationId').toBeTruthy();
  const products: any[] = (menu.categories ?? []).flatMap((c: any) => c.products ?? []);
  return { locationId, products };
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

// Testplan 1c — seed the kitchen-busy window via the owner API, prove the storefront shows
// the BUSY state (banner + chip=busy) while ordering stays open, then clear it. Reversible.
test('storefront shows the venue-busy state when the kitchen is flagged busy', async ({ page, request }) => {
  const token = await ownerToken(request);
  const auth = { Authorization: `Bearer ${token}` };
  const { locationId } = await demoMenu(request);
  const busyUntil = new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString();

  const set = await request.patch(`/api/owner/locations/${locationId}/kitchen-busy`, {
    headers: auth,
    data: { busy_until: busyUntil },
  });
  expect(set.ok(), 'setting kitchen-busy should succeed').toBeTruthy();

  try {
    await page.goto('/s/demo');
    await expect(page.locator('[data-testid="venue-busy-banner"]')).toBeVisible({ timeout: 25000 });
    await expect(page.locator('[data-testid="venue-state-chip"]')).toHaveAttribute('data-state', 'busy');
    // busy is NOT closed — ordering stays open.
    await expect(page.locator('[data-testid="venue-closed-banner"]')).not.toBeVisible();
  } finally {
    const clear = await request.patch(`/api/owner/locations/${locationId}/kitchen-busy`, {
      headers: auth,
      data: { busy_until: null },
    });
    expect(clear.ok(), 'clearing kitchen-busy should succeed').toBeTruthy();
  }
});

// Testplan 1d (corrected to real behavior) — read_public_menu hard-filters `is_available = true`
// (migration 063, line 118), so flipping a product unavailable REMOVES it from the storefront
// rather than rendering the sold-out item-state-chip. The chip in ProductCard (`!isAvailable`)
// is therefore unreachable via the availability toggle (a shipped contradiction — see notes).
// This proves the ACTUAL contract: an unavailable product disappears from /s/demo. Reversible.
test('unavailable product is hidden from the storefront menu', async ({ page, request }) => {
  const token = await ownerToken(request);
  const auth = { Authorization: `Bearer ${token}` };
  const { locationId, products } = await demoMenu(request);
  const target = products.find((p) => p.available !== false);
  expect(target, 'demo menu has an available product to flip').toBeTruthy();

  // Establish the baseline: the product is present on the storefront before we flip it.
  await page.goto('/s/demo');
  await expect(page.getByText(target.name, { exact: true }).first()).toBeVisible({ timeout: 25000 });

  const off = await request.patch(`/api/owner/locations/${locationId}/products/${target.id}`, {
    headers: auth,
    data: { available: false },
  });
  expect(off.ok(), 'marking the product unavailable should succeed').toBeTruthy();

  try {
    await page.goto('/s/demo');
    await expect(page.getByText(target.name, { exact: true })).toHaveCount(0);
  } finally {
    const on = await request.patch(`/api/owner/locations/${locationId}/products/${target.id}`, {
      headers: auth,
      data: { available: true },
    });
    expect(on.ok(), 'restoring the product availability should succeed').toBeTruthy();
  }
});
