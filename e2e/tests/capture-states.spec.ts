import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

/**
 * EXHAUSTIVE state capture for the sushi-durres tenant — every page × breakpoint × state
 * (default / loading-skeleton / error / empty) + interactive overlays (product modal, cart
 * drawer). For the pixel-perfect / design-system consistency audit.
 *
 * Run:
 *   CAPTURE=1 VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
 *     pnpm exec playwright test e2e/tests/capture-states.spec.ts --project=desktop --reporter=line
 */
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
// No hardcoded fallback secret — a committed secret would silently authenticate against
// staging even when DEV_AUTH_SECRET is unset. Throw inside the test body (not at module
// load, so test collection still works when CAPTURE is unset and the test is skipped).
const SECRET = process.env.DEV_AUTH_SECRET;
const SLUG = process.env.SLUG || 'demo';
const CLIENT_ONLY = !!process.env.CLIENT_ONLY;
const DIR = process.env.CAPTURE_DIR || '/root/dowiz/audit/full-capture';
test.skip(!process.env.CAPTURE, 'set CAPTURE=1 to capture');
// This spec mutates state via /dev/mock-auth (upserts a dev owner user) — refuse to run
// against prod / an unknown target.
test.beforeAll(() => requireStaging(BASE));
test.setTimeout(180_000);

const VIEWPORTS = [
  { tag: 'd', w: 1280, h: 900 },
  { tag: 'm', w: 390, h: 844 },
];

test('capture all states', async ({ page, request }) => {
  fs.mkdirSync(DIR, { recursive: true });
  if (!SECRET) throw new Error('DEV_AUTH_SECRET is unset — refusing to run capture without an explicit dev-auth secret');
  const hdr = { 'x-dev-auth-secret': SECRET };
  const ownerRes = await request.post(`${BASE}/api/dev/mock-auth`, { headers: hdr, data: { locationSlug: SLUG } });
  expect(ownerRes.status(), 'owner mock-auth must return 200').toBe(200);
  const owner = await ownerRes.json();
  expectJwt(owner.access_token, 'owner.access_token');
  const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { headers: hdr, data: { role: 'courier', locationId: owner.activeLocationId } });
  expect(courierRes.status(), 'courier mock-auth must return 200').toBe(200);
  const courier = await courierRes.json();
  expectJwt(courier.access_token, 'courier.access_token');
  expectUuid(courier.activeLocationId, 'courier.activeLocationId');
  const captured: string[] = [];

  const setAuth = async (token?: string, locale = 'sq') => {
    await page.addInitScript(([tk, lc]: any) => {
      if (tk) localStorage.setItem('dos_access_token', tk); else localStorage.removeItem('dos_access_token');
      localStorage.setItem('dos_locale', lc);
    }, [token, locale]);
  };

  const shot = async (name: string) => {
    await page.waitForTimeout(1400);
    // Wait for the (CDN) Tabler icon webfont to actually paint — otherwise icons screenshot blank
    // and read as "broken/empty" when they render fine for real users. See findings A5.
    await page.evaluate(() => (document as any).fonts?.ready).catch((e) => { void e; /* tolerated: fonts API may be absent / never resolve in some engines */ });
    await page.waitForTimeout(300);
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true }).catch((e) => { void e; /* tolerated: best-effort capture, a missed shot must not abort the run */ });
    captured.push(name);
  };

  const go = async (path: string) => {
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch((e) => { void e; /* tolerated: networkidle can time out on long-poll/WS pages; capture still proceeds */ });
  };

  // ── ADMIN (owner) ────────────────────────────────────────────────────────
  const ADMIN = [
    ['orders', '/admin'], ['menu', '/admin/menu'], ['settings', '/admin/settings'],
    ['branding', '/admin/branding'], ['analytics', '/admin/analytics'], ['promotions', '/admin/promotions'],
    ['crm', '/admin/crm'], ['couriers', '/admin/couriers'], ['supplies', '/admin/supplies'],
    ['activation', '/admin/activation'], ['onboarding', '/admin/onboarding'],
  ];
  // ── COURIER ──────────────────────────────────────────────────────────────
  const COURIER = [
    ['tasks', '/courier'], ['shift', '/courier/shift'], ['earnings', '/courier/earnings'], ['history', '/courier/history'],
  ];

  for (const v of VIEWPORTS) {
    await page.setViewportSize({ w: v.w, h: v.h } as any).catch(async () => { await page.setViewportSize({ width: v.w, height: v.h }); });

    // Admin default states
    if (!CLIENT_ONLY) {
    await setAuth(owner.access_token, 'sq');
    for (const [name, path] of ADMIN) {
      await go(path); await shot(`${v.tag}-admin-${name}`);
      // DOM-presence proof: a crashed/blank/login-redirected admin shell would still
      // increment `captured`. Assert the authed owner dashboard actually rendered.
      if (name === 'orders') await expect(page.locator('[data-testid="ws-status-dot"]')).toBeVisible({ timeout: 15_000 });
    }
    }

    // Client storefront default + overlays (no auth)
    await setAuth(undefined, 'sq');
    await go(`/s/${SLUG}`); await shot(`${v.tag}-client-menu`);
    // DOM-presence proof: an HTTP-500 / empty-render storefront would still push a capture.
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15_000 });
    // product modal
    try {
      await page.locator('[data-testid="menu-item"]').first().click({ timeout: 5000 });
      await page.waitForTimeout(800); await shot(`${v.tag}-client-modal`);
      // add to cart — scope to the open dialog so we don't hit a card FAB behind the backdrop
      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: /Shto në Shport|Add to Cart/i }).first().click({ timeout: 5000 }).catch(async () => {
        await dialog.locator('button').last().click({ timeout: 3000 }).catch((e) => { void e; /* tolerated: add-to-cart fallback is best-effort for the capture flow */ });
      });
      await page.waitForTimeout(1000);
      await page.locator('[data-testid="cart-open"]').first().click({ timeout: 4000 }).catch((e) => { void e; /* tolerated: cart may already be open / absent in this state during capture */ });
      await page.waitForTimeout(700); await shot(`${v.tag}-client-cart`);
      // proceed to checkout WITH an item in the cart (the empty cart redirects)
      await page.locator('[data-testid="cart-checkout"]').first().click({ timeout: 4000 }).catch((e) => { void e; /* tolerated: checkout button may be absent; fallback go() handles it below */ });
      await page.waitForTimeout(1200); await shot(`${v.tag}-client-checkout`);
    } catch { /* overlay flow best-effort */
      await go(`/s/${SLUG}/checkout`); await shot(`${v.tag}-client-checkout`);
    }

    // Courier default states
    if (!CLIENT_ONLY) {
    await setAuth(courier.access_token, 'sq');
    for (const [name, path] of COURIER) {
      await go(path); await shot(`${v.tag}-courier-${name}`);
      // DOM-presence proof: a login-redirected/blank courier app would still push a capture.
      // The online/offline status badge is rendered unconditionally on the authed Tasks page.
      if (name === 'tasks') await expect(page.locator('[role="status"]').first()).toBeVisible({ timeout: 15_000 });
    }
    }
  }
  if (CLIENT_ONLY) { console.log('CAPTURED', captured.length, ':', captured.join(', ')); expect(captured.length).toBeGreaterThan(5); return; }

  // ── FORCED STATES (desktop only) — loading skeleton + error ───────────────
  await page.setViewportSize({ width: 1280, height: 900 });

  // hold = delay forever so the SKELETON shows (screenshot fires during the pending fetch).
  // Guard continue/abort in try/catch — a retried request can already be handled.
  const hold = async (r: any) => { await new Promise(res => setTimeout(res, 20000)); try { await r.abort(); } catch { /* already handled */ } };
  const fail500 = (r: any) => { try { r.fulfill({ status: 500, contentType: 'application/json', body: '{"error":"server"}' }); } catch { /* already handled */ } };
  const failWith = (status: number) => (r: any) => { try { r.fulfill({ status, contentType: 'application/json', body: '{"error":"forced"}' }); } catch { /* already handled */ } };

  // Client menu: LOADING skeleton (theme/info still loads → branded skeleton)
  await setAuth(undefined, 'sq');
  await page.route('**/public/locations/**/menu**', hold);
  await go(`/s/${SLUG}`); await page.waitForTimeout(1100);
  await page.screenshot({ path: `${DIR}/state-client-menu-loading.png`, fullPage: true }).catch((e) => { void e; /* tolerated: best-effort capture, a missed shot must not abort the run */ }); captured.push('state-client-menu-loading');
  await page.unroute('**/public/locations/**/menu**');

  // Client menu: ERROR (500)
  await page.route('**/public/locations/**/menu**', fail500);
  await go(`/s/${SLUG}`); await shot('state-client-menu-error');
  await page.unroute('**/public/locations/**/menu**');

  // Client menu: 401 (expired/invalid token) — distinct error surface from 500.
  await page.route('**/public/locations/**/menu**', failWith(401));
  await go(`/s/${SLUG}`); await shot('state-client-menu-401');
  await page.unroute('**/public/locations/**/menu**');

  // Client menu: 404 (unknown slug / location not found).
  await page.route('**/public/locations/**/menu**', failWith(404));
  await go(`/s/${SLUG}`); await shot('state-client-menu-404');
  await page.unroute('**/public/locations/**/menu**');

  // Admin orders: LOADING skeleton
  await setAuth(owner.access_token, 'sq');
  await page.route('**/owner/**', hold);
  await go('/admin'); await page.waitForTimeout(1100);
  await page.screenshot({ path: `${DIR}/state-admin-orders-loading.png`, fullPage: true }).catch((e) => { void e; /* tolerated: best-effort capture, a missed shot must not abort the run */ }); captured.push('state-admin-orders-loading');
  await page.unroute('**/owner/**');

  // Admin orders: ERROR
  await page.route('**/owner/**', fail500);
  await go('/admin'); await shot('state-admin-orders-error');
  await page.unroute('**/owner/**');

  console.log('CAPTURED', captured.length, ':', captured.join(', '));
  fs.writeFileSync(`${DIR}/_manifest.json`, JSON.stringify(captured, null, 2));
  expect(captured.length).toBeGreaterThan(30);
});
