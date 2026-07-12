import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// BASE defaults to STAGING (never the prod host) — this suite drives the dev/mock-auth
// backdoor, so a run must never touch prod. requireStaging() in beforeAll fails fast.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;

// TODO(needs-staging): error-matrix coverage (401 expired / 403 wrong-role / 503 API down)
//   per page via page.route(...) intercept asserting the page's error UI — deferred: the
//   admin shells (dashboard/crm/couriers/supplies) have no error-boundary [data-testid] to
//   anchor on; needs a real staging run to confirm which error surfaces render.
// TODO(needs-staging): seed ≥1 known row per list in beforeAll and assert the list branch
//   ([data-testid=…-list]) is visible — distinguishes a healthy list from a permanently
//   broken one that always falls back to the empty-state placeholder. Needs a staging seed.

test.describe('UI: Empty States — All Lists', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // dev backdoor — refuse to run against prod/unknown target
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    // F4: fail fast if setup yielded no real JWT, instead of silently seeding every test
    // with `undefined` as the localStorage token.
    expectJwt(authToken, 'owner access_token');
  });

  test('Dashboard loads even with no orders (today)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });

    // F1: not bounced to /login, and the dashboard's own WS-status indicator is visible —
    // a login redirect / blank error page / crash boundary all fail this.
    await expect(page).toHaveURL(/\/admin(?:$|[/?])/);
    await expect(page.locator('[data-testid="ws-status-dot"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM page shows list or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/crm`, { waitUntil: 'networkidle' });

    await expect(page).toHaveURL(/\/admin\/crm/);
    await expect(page.locator('main.app-shell-main')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('main.app-shell-main h2').first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Couriers page shows list or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'networkidle' });

    await expect(page).toHaveURL(/\/admin\/couriers/);
    await expect(page.locator('main.app-shell-main')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('main.app-shell-main h2').first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Analytics page loads with KPIs', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });

    await expect(page).toHaveURL(/\/admin\/analytics/);
    // A real KPI card rendered — not just a non-empty <body>.
    await expect(page.locator('[data-testid="kpi-card"]').first()).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Client menu shows product cards or empty', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded', timeout: 15000 });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.waitForSelector('h3.product-name, [class*="product-card"]', { timeout: 8000 });

    const criticalErrors = errors.filter(e => !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver') && !e.includes("Unexpected token"));
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
  });

  test('Courier tasks page shows tasks or empty', async ({ page, request }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierRes.status()).toBe(200);
    const courierBody = await courierRes.json();
    const courierToken = courierBody.access_token;
    expectJwt(courierToken, 'courier access_token');

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });

    // F3: stays on the courier route (not redirected to /login or /admin) and renders the
    // COURIER shell — the owner /admin shell renders a sidebar <aside>; the courier one does
    // not. This proves an owner-level page was not served to a courier token.
    await expect(page).toHaveURL(/\/courier(?:$|[/?])/);
    await expect(page.locator('main.app-shell-main')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('aside')).toHaveCount(0);
    // TODO(needs-staging): assert the listed tasks belong to THIS courier (vs a 2nd
    //   courier's) — requires a real second-courier fixture on staging, not a nil id.

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Promotions page loads with list or empty', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });

    await expect(page).toHaveURL(/\/admin\/promotions/);
    // Either the populated list OR the empty-state — but NOT the error branch (which has no
    // testid), so a failed fetch fails the test instead of passing as "empty".
    await expect(
      page.locator('[data-testid="promotions-list"], [data-testid="empty-state"]').first(),
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies library page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });

    await expect(page).toHaveURL(/\/admin\/supplies/);
    await expect(page.locator('main.app-shell-main')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('main.app-shell-main h2').first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
