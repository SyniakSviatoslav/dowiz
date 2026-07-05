import { test, expect } from '@playwright/test';
import { expectJwt } from '../../helpers/assert-shape';
import { requireStaging } from '../../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'http://localhost:3000';

test.describe('Admin Dashboard — Interactive', () => {
  let authToken: string;
  // Registered in beforeEach BEFORE goto so load-time errors are captured (finding #8).
  let jsErrors: string[] = [];

  test.beforeAll(async ({ request }) => {
    // mock-auth UPSERTs the dev user — treat this spec as mutating; never run it against prod.
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    expectJwt(authToken, 'mock-auth access_token');
  });

  test.beforeEach(async ({ page }) => {
    jsErrors = [];
    page.on('pageerror', err => jsErrors.push(err.message));
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
      sessionStorage.setItem('dos_dev', '1');
    }, authToken);
    await page.goto('/admin?dev=true', { waitUntil: 'networkidle' });
  });

  test('dashboard loads with content', async ({ page }) => {
    // Specific render proof: the owner WS-status indicator + view-mode tabs are owner-only chrome.
    await expect(page.locator('[data-testid="ws-status-dot"]')).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('tablist', { name: /view mode/i })).toBeVisible();
  });

  test('Live/History toggle buttons exist and are clickable', async ({ page }) => {
    await page.waitForTimeout(3000);

    // View-mode tablist MUST exist — absence is a hard failure, not a silent skip (finding #3).
    const tablist = page.getByRole('tablist', { name: /view mode/i });
    await expect(tablist).toBeVisible({ timeout: 10000 });

    const buttons = tablist.getByRole('tab');
    await expect(buttons).toHaveCount(2);

    // Toggle History then Live and assert the selected state actually tracks the click.
    await buttons.nth(1).click();
    await expect(buttons.nth(1)).toHaveAttribute('aria-selected', 'true');
    await buttons.first().click();
    await expect(buttons.first()).toHaveAttribute('aria-selected', 'true');
  });

  test('status filter buttons are clickable', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Scope to the order-status filter group by its aria-label (finding #4).
    const statusGroup = page.getByRole('group', { name: /status filter/i });
    await expect(statusGroup).toBeVisible({ timeout: 10000 });
    // It must contain the real status options, not just be any [role=group].
    await expect(statusGroup).toContainText(/all/i);
    await expect(statusGroup).toContainText(/pending/i);

    const buttons = statusGroup.locator('button');
    await expect(buttons.first()).toBeVisible();
    // Click a non-"all" status and assert the filter applies (button becomes pressed).
    const pending = statusGroup.getByRole('button', { name: /pending/i }).first();
    await pending.click();
    await expect(pending).toHaveAttribute('aria-pressed', 'true');
  });

  test('sort icon button opens dropdown', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Sort trigger by its listbox-popup semantics, not an icon class.
    const sortBtn = page.locator('button[aria-haspopup="listbox"]').first();
    await expect(sortBtn).toBeVisible({ timeout: 10000 });
    await expect(sortBtn).toHaveAttribute('aria-expanded', 'false');

    await sortBtn.click();

    // Assert the dropdown is open AND contains the real sort options (semantic content).
    await expect(sortBtn).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByText(/oldest first/i)).toBeVisible({ timeout: 5000 });
    await expect(page.getByText(/highest total/i)).toBeVisible();
  });

  test('search input accepts text', async ({ page }) => {
    await page.waitForTimeout(3000);

    // Target the orders search input by its accessible label (finding #6).
    const searchInput = page.getByLabel('Search orders by name or ID', { exact: true });
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    await searchInput.fill('zzz-no-such-order');
    await expect(searchInput).toHaveValue('zzz-no-such-order');
    // A query that matches nothing must filter the list down to the empty state,
    // proving the input is actually wired to the order list (not just an inert field).
    await expect(page.locator('[data-testid^="order-card-"]')).toHaveCount(0);
  });

  test('quick stats render in grid', async ({ page }) => {
    await page.waitForTimeout(3000);

    const stats = page.locator('.grid.grid-cols-3').first();
    await expect(stats).toBeVisible({ timeout: 10000 });

    const cards = stats.locator('> div');
    const count = await cards.count();
    expect(count).toBeGreaterThanOrEqual(3);
  });

  test('no JS errors', async ({ page }) => {
    // jsErrors is accumulated from beforeEach (listener registered before goto) — load-time
    // errors are now captured, not missed (finding #8).
    await page.waitForTimeout(3000);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await page.waitForTimeout(2000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});

// Auth controls — POSITIVE control above proves a valid owner sees the dashboard; these prove
// the gate actually rejects the unauthenticated and the wrong-role (findings #1 and #7). No owner
// token is injected here, so each test exercises the real auth path.
test.describe('Admin Dashboard — auth controls', () => {
  test('unauthenticated /admin redirects to login', async ({ page }) => {
    // TODO(needs_staging): requires a live deployed SPA to perform the auth redirect.
    await page.goto('/admin', { waitUntil: 'networkidle' });
    await expect(page).toHaveURL(/\/admin\/login/);
  });

  test('courier token cannot access the owner dashboard', async ({ page, request }) => {
    // TODO(needs_staging): mints a real courier token against a live API + 2nd role; needs staging.
    requireStaging(BASE);
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(res.status()).toBe(200);
    const courierToken = (await res.json()).access_token;
    expectJwt(courierToken, 'courier access_token');

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
      sessionStorage.setItem('dos_dev', '1');
    }, courierToken);
    await page.goto('/admin?dev=true', { waitUntil: 'networkidle' });

    // The owner-only dashboard chrome must NOT render for a courier role.
    await expect(page.locator('[data-testid="ws-status-dot"]')).not.toBeVisible();
  });
});
