import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// beforeAll mints a dev token via /api/dev/mock-auth which UPSERTs the dev owner row —
// that is a DB mutation, so refuse to run against prod/unknown targets.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('UI: Analytics + Supplies CRUD', () => {
  let authToken: string;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE);
    // Explicitly request the owner role — never let an empty body silently mint a
    // wrong-role/wrong-tenant token (mock-auth defaults to owner, but assert it).
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner' } });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    expectJwt(body.access_token, 'access_token');
    expectUuid(body.activeLocationId, 'activeLocationId');
    // Decode the minted JWT and prove the role claim is actually 'owner' — guards
    // against the endpoint handing back a token for the wrong role.
    const claims = JSON.parse(Buffer.from(body.access_token.split('.')[1], 'base64url').toString());
    expect(claims.role).toBe('owner');
    authToken = body.access_token;
  });

  test('Analytics page loads with KPI cards', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });

    // Real render proof: the KPI cards must actually be in the DOM (a spinner, a 500,
    // an error boundary or a login redirect all fail this).
    const kpiCards = page.locator('[data-testid=kpi-card]');
    await expect(kpiCards.first()).toBeVisible({ timeout: 15000 });
    expect(await kpiCards.count(), 'analytics renders the 4 KPI stat cards').toBeGreaterThanOrEqual(4);
    await expect(page.locator('[data-testid=kpi-value]').first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Analytics API returns revenue and chart data', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/analytics`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();

    // Assert the actual contract shape (revenue.today / orders.today are numbers, chart +
    // topProducts are arrays) — not an OR over `undefined !== undefined`.
    expect(typeof body.revenue?.today, 'revenue.today is a number').toBe('number');
    expect(typeof body.orders?.today, 'orders.today is a number').toBe('number');
    expect(Array.isArray(body.chart), 'chart is an array').toBe(true);
    expect(Array.isArray(body.topProducts), 'topProducts is an array').toBe(true);
  });

  test('Owner-only APIs reject unauthenticated and wrong-role callers', async ({ request }) => {
    // Negative controls: getOwnerContext/getLocationId return null (→ 401) for a missing
    // token AND for any non-owner role (apps/api/src/routes/spa-proxy.ts:59/114).
    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierRes.status()).toBe(200);
    const courierToken = (await courierRes.json()).access_token as string;
    expectJwt(courierToken, 'courier token');

    for (const path of ['/api/owner/analytics', '/api/owner/customers']) {
      const noAuth = await request.get(`${BASE}${path}`);
      expect(noAuth.status(), `${path} with no token must be 401`).toBe(401);

      const wrongRole = await request.get(`${BASE}${path}`, {
        headers: { Authorization: `Bearer ${courierToken}` },
      });
      expect(wrongRole.status(), `${path} with a courier token must be 401`).toBe(401);
    }
    // TODO(needs-staging): true cross-tenant isolation needs a REAL second owner+location
    // (mock-auth only ever mints the single dev owner). Probe /api/owner/analytics +
    // /api/owner/customers with tenant-B's token and assert tenant-A's rows never appear.
  });

  test('Analytics page survives navigation', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid=kpi-card]').first()).toBeVisible({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid=kpi-card]').first()).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies page loads with filter/search controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });

    // The supply list must actually render (seed data), and the search control be present.
    await expect(page.locator('[data-testid=supply-item]').first()).toBeVisible({ timeout: 15000 });
    await expect(page.locator('input[type="search"]').first()).toBeVisible({ timeout: 5000 });
    await expect(page.locator('button[aria-haspopup="menu"]').first()).toBeVisible({ timeout: 5000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies kind filter narrows the list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });

    const items = page.locator('[data-testid=supply-item]');
    await expect(items.first()).toBeVisible({ timeout: 15000 });
    const before = await items.count();
    expect(before, 'supplies list has more than one kind to filter').toBeGreaterThan(1);

    // The kind filter is the SegmentedControl (role=group, aria-pressed buttons). Clicking a
    // single non-"all" kind MUST reduce the list (each kind is a strict subset of all).
    const kindBtns = page.locator('[role=group] button[aria-pressed]');
    const kindCount = await kindBtns.count();
    expect(kindCount, 'kind filter renders multiple options').toBeGreaterThanOrEqual(2);
    await kindBtns.nth(kindCount - 1).click(); // last kind (utensil) — a small subset
    await expect.poll(() => items.count(), { timeout: 5000 }).toBeLessThan(before);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies search filters items', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });

    const items = page.locator('[data-testid=supply-item]');
    await expect(items.first()).toBeVisible({ timeout: 15000 });

    const searchInput = page.locator('input[type="search"]').first();
    await expect(searchInput).toBeVisible({ timeout: 5000 });
    // A term that matches no name/category MUST collapse the list to the empty state.
    await searchInput.fill('zzzznomatchqwerty');
    await expect(items).toHaveCount(0);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies sort menu opens and keeps items rendered', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });

    const items = page.locator('[data-testid=supply-item]');
    await expect(items.first()).toBeVisible({ timeout: 15000 });

    const sortBtn = page.locator('button[aria-haspopup="menu"]').first();
    await expect(sortBtn).toBeVisible({ timeout: 5000 });
    await sortBtn.click();
    await expect(page.getByRole('menu').first()).toBeVisible({ timeout: 5000 });

    // Choose the second sort option; the list must stay populated after re-sorting.
    await page.getByRole('menuitemradio').nth(1).click();
    await expect(items.first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Supplies page survives navigation', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid=supply-item]').first()).toBeVisible({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/supplies`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid=supply-item]').first()).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM page loads with customer list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/crm`, { waitUntil: 'networkidle' });

    // The CRM component's search control renders only in the loaded (non-spinner, non-crash)
    // state — proving the route mounted rather than a redirect/error boundary. And the
    // "could not load customers" error icon (ti-cloud-off) must be absent.
    await expect(page.locator('input[type="search"]').first()).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.ti-cloud-off')).toHaveCount(0);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('CRM API returns customers list', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/customers`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    // The contract is a bare array of customer rows (spa-proxy.ts:803).
    expect(Array.isArray(body), 'customers response is a bare array').toBe(true);
  });

  test('No cookies on any admin page', async ({ page }) => {
    for (const path of ['/admin/analytics', '/admin/supplies', '/admin/crm']) {
      await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
      await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' });
      const cookies = await page.context().cookies();
      expect(cookies, `${path} should have 0 cookies`).toEqual([]);
    }
  });
});
